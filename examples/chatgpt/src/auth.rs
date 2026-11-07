//! Sign in with ChatGPT: the OAuth 2.0 authorization-code flow with PKCE that
//! the Codex CLI uses, a loopback redirect on port 1455, and token refresh.
//!
//! Everything here is blocking and runs on a background thread.

use std::{
    io::{BufRead as _, BufReader, Write as _},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context as _, Result, anyhow, bail};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore as _;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

const ISSUER: &str = "https://auth.openai.com";
/// The public OAuth client registered for the Codex CLI. Its redirect
/// allow-list holds the loopback URI below, so the port is fixed.
const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const PORT: u16 = 1455;
const REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const SCOPE: &str = "openid profile email offline_access";
pub const ORIGINATOR: &str = "codex_cli_rs";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);

/// Tokens for a ChatGPT account, persisted between launches.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Credentials {
    access_token: String,
    refresh_token: String,
    id_token: String,
    account_id: Option<String>,
    email: Option<String>,
    /// Unix seconds at which the access token expires.
    expires_at: u64,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
}

impl Credentials {
    pub fn access_token(&self) -> &str {
        &self.access_token
    }

    pub fn account_id(&self) -> Option<&str> {
        self.account_id.as_deref()
    }

    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    fn path() -> Option<PathBuf> {
        Some(
            dirs::config_dir()?
                .join("gpui-kit-chatgpt")
                .join("auth.json"),
        )
    }

    /// Load saved credentials, falling back to a Codex CLI sign-in.
    pub fn load() -> Option<Self> {
        if let Some(credentials) = Self::path()
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        {
            return Some(credentials);
        }
        Self::load_codex()
    }

    /// Reuse `~/.codex/auth.json` written by `codex login`, if present.
    fn load_codex() -> Option<Self> {
        #[derive(Deserialize)]
        struct CodexAuth {
            tokens: Option<CodexTokens>,
        }
        #[derive(Deserialize)]
        struct CodexTokens {
            id_token: String,
            access_token: String,
            refresh_token: String,
        }

        let path = dirs::home_dir()?.join(".codex").join("auth.json");
        let auth: CodexAuth = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
        let tokens = auth.tokens?;
        Self::from_tokens(tokens.access_token, tokens.refresh_token, tokens.id_token).ok()
    }

    fn from_tokens(access_token: String, refresh_token: String, id_token: String) -> Result<Self> {
        let id_claims = jwt_claims(&id_token)?;
        let auth_claims = &id_claims["https://api.openai.com/auth"];
        let expires_at = jwt_claims(&access_token)
            .ok()
            .and_then(|claims| claims["exp"].as_u64())
            .unwrap_or_else(|| now() + 3600);

        Ok(Self {
            account_id: auth_claims["chatgpt_account_id"]
                .as_str()
                .map(str::to_string),
            email: id_claims["email"].as_str().map(str::to_string),
            access_token,
            refresh_token,
            id_token,
            expires_at,
        })
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path().context("no config directory")?;
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, serde_json::to_vec_pretty(self)?)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }

    pub fn forget() {
        if let Some(path) = Self::path() {
            _ = std::fs::remove_file(path);
        }
    }

    /// Whether the access token expires within the next minute.
    pub fn is_expiring(&self) -> bool {
        self.expires_at <= now() + 60
    }

    /// Exchange the refresh token for a new access token.
    pub fn refresh(&self) -> Result<Self> {
        let response: TokenResponse = reqwest::blocking::Client::new()
            .post(format!("{ISSUER}/oauth/token"))
            .json(&serde_json::json!({
                "client_id": CLIENT_ID,
                "grant_type": "refresh_token",
                "refresh_token": self.refresh_token,
                "scope": "openid profile email",
            }))
            .send()?
            .error_for_status()
            .context("ChatGPT sign-in expired")?
            .json()?;

        let credentials = Self::from_tokens(
            response.access_token,
            response
                .refresh_token
                .unwrap_or_else(|| self.refresh_token.clone()),
            response.id_token.unwrap_or_else(|| self.id_token.clone()),
        )?;
        credentials.save()?;
        Ok(credentials)
    }
}

/// A sign-in waiting for the browser to redirect back.
pub struct Login {
    url: String,
    verifier: String,
    state: String,
    listener: TcpListener,
}

impl Login {
    /// Bind the loopback listener and build the authorization URL.
    pub fn start() -> Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", PORT)).with_context(|| {
            format!(
                "port {PORT} is in use; quit any other sign-in (such as `codex login`) and retry"
            )
        })?;
        let verifier = random_token(64);
        let state = random_token(32);
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let url = Url::parse_with_params(
            &format!("{ISSUER}/oauth/authorize"),
            &[
                ("response_type", "code"),
                ("client_id", CLIENT_ID),
                ("redirect_uri", REDIRECT_URI),
                ("scope", SCOPE),
                ("code_challenge", &challenge),
                ("code_challenge_method", "S256"),
                ("id_token_add_organizations", "true"),
                ("codex_cli_simplified_flow", "true"),
                ("state", &state),
                ("originator", ORIGINATOR),
            ],
        )?
        .to_string();

        Ok(Self {
            url,
            verifier,
            state,
            listener,
        })
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    /// Wait for the redirect, then exchange the code for tokens.
    pub fn finish(self) -> Result<Credentials> {
        self.listener.set_nonblocking(true)?;
        let deadline = SystemTime::now() + LOGIN_TIMEOUT;
        loop {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    stream.set_nonblocking(false)?;
                    if let Some(code) = self.handle_redirect(stream)? {
                        let credentials = exchange_code(&code, &self.verifier)?;
                        credentials.save()?;
                        return Ok(credentials);
                    }
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    if SystemTime::now() > deadline {
                        bail!("sign-in timed out");
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(err) => return Err(err.into()),
            }
        }
    }

    /// Answer one HTTP request; returns the code once the callback arrives.
    fn handle_redirect(&self, mut stream: TcpStream) -> Result<Option<String>> {
        let mut request_line = String::new();
        BufReader::new(&stream).read_line(&mut request_line)?;
        let target = request_line.split_whitespace().nth(1).unwrap_or("/");
        let url = Url::parse(&format!("http://localhost{target}"))?;

        if url.path() != "/auth/callback" {
            write_response(&mut stream, "404 Not Found", "Not found")?;
            return Ok(None);
        }

        let param = |name: &str| {
            url.query_pairs()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.into_owned())
        };
        if let Some(error) = param("error") {
            write_response(
                &mut stream,
                "400 Bad Request",
                "Sign-in was cancelled. You can close this tab.",
            )?;
            bail!("sign-in failed: {error}");
        }
        if param("state").as_deref() != Some(self.state.as_str()) {
            write_response(
                &mut stream,
                "400 Bad Request",
                "Sign-in state did not match.",
            )?;
            bail!("sign-in state did not match");
        }
        let code = param("code").ok_or_else(|| anyhow!("sign-in returned no code"))?;
        write_response(
            &mut stream,
            "200 OK",
            "Signed in to ChatGPT. You can close this tab and return to the app.",
        )?;
        Ok(Some(code))
    }
}

fn exchange_code(code: &str, verifier: &str) -> Result<Credentials> {
    let response: TokenResponse = reqwest::blocking::Client::new()
        .post(format!("{ISSUER}/oauth/token"))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", REDIRECT_URI),
            ("client_id", CLIENT_ID),
            ("code_verifier", verifier),
        ])
        .send()?
        .error_for_status()
        .context("token exchange failed")?
        .json()?;

    Credentials::from_tokens(
        response.access_token,
        response
            .refresh_token
            .context("no refresh token returned")?,
        response.id_token.context("no ID token returned")?,
    )
}

fn write_response(stream: &mut TcpStream, status: &str, message: &str) -> Result<()> {
    let body = format!(
        "<!doctype html><meta charset=utf-8><title>ChatGPT</title>\
         <body style=\"font:16px system-ui;margin:4rem\">{message}</body>"
    );
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    Ok(())
}

/// Decode a JWT's payload without verifying it; the token came straight
/// from the issuer over TLS and is only read for display and routing.
fn jwt_claims(token: &str) -> Result<serde_json::Value> {
    let payload = token.split('.').nth(1).context("malformed token")?;
    Ok(serde_json::from_slice(
        &URL_SAFE_NO_PAD.decode(payload.trim_end_matches('='))?,
    )?)
}

fn random_token(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_account_and_expiry_from_tokens() {
        let encode = |claims: serde_json::Value| {
            format!("e30.{}.sig", URL_SAFE_NO_PAD.encode(claims.to_string()))
        };
        let id_token = encode(serde_json::json!({
            "email": "me@example.com",
            "https://api.openai.com/auth": { "chatgpt_account_id": "acct_1" },
        }));
        let access_token = encode(serde_json::json!({ "exp": 4_000_000_000u64 }));

        let credentials =
            Credentials::from_tokens(access_token, "refresh".into(), id_token).unwrap();
        assert_eq!(credentials.account_id(), Some("acct_1"));
        assert_eq!(credentials.email(), Some("me@example.com"));
        assert!(!credentials.is_expiring());
    }

    #[test]
    fn pkce_challenge_is_unpadded_base64url() {
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(b"verifier"));
        assert!(!challenge.contains('='));
        assert_eq!(challenge.len(), 43);
    }
}

//! What a script application is allowed to do.
//!
//! The default set is empty: a script gets no file, process, network, storage or
//! clipboard access until the host grants it. Grants come from a plugin manifest
//! (§18.1) or directly from the embedding application.

use std::path::{Path, PathBuf};

use cap_std::{ambient_authority, fs::Dir};

/// Whether a script may hand `url` to the system URL opener.
///
/// One rule for every route out: `Link.href`, `cx.open_url`, and a link in
/// `TextView` content. The scheme check is the part that matters. Without it
/// any of them becomes a way to hand an arbitrary URI to whatever handler the
/// desktop has registered for its scheme, which is a considerably larger thing
/// than opening a page.
pub(crate) fn is_openable_url(url: &str) -> bool {
    reqwest::Url::parse(url).is_ok_and(|parsed| {
        matches!(parsed.scheme(), "http" | "https") && parsed.host_str().is_some()
    })
}

/// A capability grant. Every field is private so adding a capability later is
/// not a breaking change for embedders.
#[derive(Clone, Debug, Default)]
pub struct Capabilities {
    read_roots: Vec<PathBuf>,
    write_roots: Vec<PathBuf>,
    execute: ExecuteGrant,
    network_hosts: Vec<String>,
    http_requests: Vec<HttpRequestGrant>,
    storage: bool,
    clipboard_read: bool,
    clipboard_write: bool,
    exit: bool,
}

/// A scheme-, effective-port-, method-, and path-scoped HTTP grant for one host.
///
/// This is deliberately separate from [`Capabilities::network_hosts`]: a
/// plugin may read one REST resource without gaining TCP or WebSocket access to
/// the same host, or permission to POST to another path on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpRequestGrant {
    scheme: String,
    host: String,
    port: Option<u16>,
    methods: Vec<String>,
    paths: Vec<String>,
    path_prefixes: Vec<String>,
}

impl HttpRequestGrant {
    pub fn new<H, M, P, Q>(host: impl Into<String>, methods: H, paths: M, path_prefixes: P) -> Self
    where
        H: IntoIterator<Item = Q>,
        M: IntoIterator,
        M::Item: Into<String>,
        P: IntoIterator,
        P::Item: Into<String>,
        Q: Into<String>,
    {
        Self {
            scheme: "https".to_owned(),
            host: host.into().to_ascii_lowercase(),
            port: None,
            methods: methods
                .into_iter()
                .map(|method| method.into().to_ascii_uppercase())
                .collect(),
            paths: paths.into_iter().map(Into::into).collect(),
            path_prefixes: path_prefixes.into_iter().map(Into::into).collect(),
        }
    }

    /// Overrides the default HTTPS scheme for this grant.
    pub fn scheme(mut self, scheme: impl Into<String>) -> Self {
        self.scheme = scheme.into().to_ascii_lowercase();
        self
    }

    /// Restricts this grant to a non-default effective port.
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    fn allows(
        &self,
        scheme: &str,
        host: &str,
        port: Option<u16>,
        method: &str,
        path: &str,
    ) -> bool {
        self.scheme.eq_ignore_ascii_case(scheme)
            && self.host.eq_ignore_ascii_case(host)
            && effective_port(&self.scheme, self.port) == effective_port(scheme, port)
            && self
                .methods
                .iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(method))
            && (self.paths.iter().any(|allowed| allowed == path)
                || self.path_prefixes.iter().any(|allowed| {
                    path == allowed
                        || path
                            .strip_prefix(allowed)
                            .is_some_and(|suffix| allowed.ends_with('/') || suffix.starts_with('/'))
                }))
    }
}

fn effective_port(scheme: &str, port: Option<u16>) -> Option<u16> {
    port.or_else(|| match scheme {
        "http" => Some(80),
        "https" => Some(443),
        _ => None,
    })
}

/// Which external commands a script may run.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum ExecuteGrant {
    /// `os.execute` is unavailable.
    #[default]
    Denied,
    /// Only these command names may run.
    Allowed(Vec<String>),
    /// Any command may run. Shown to the user at the highest severity.
    Unrestricted,
}

impl Capabilities {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read_roots(mut self, roots: impl IntoIterator<Item = PathBuf>) -> Self {
        self.read_roots = roots.into_iter().collect();
        self
    }

    pub fn write_roots(mut self, roots: impl IntoIterator<Item = PathBuf>) -> Self {
        self.write_roots = roots.into_iter().collect();
        self
    }

    pub fn execute(mut self, grant: ExecuteGrant) -> Self {
        self.execute = grant;
        self
    }

    pub fn network_hosts(mut self, hosts: impl IntoIterator<Item = String>) -> Self {
        self.network_hosts = hosts
            .into_iter()
            .map(|host| host.to_ascii_lowercase())
            .collect();
        self
    }

    pub fn http_requests(mut self, requests: impl IntoIterator<Item = HttpRequestGrant>) -> Self {
        self.http_requests = requests.into_iter().collect();
        self
    }

    pub fn storage(mut self, allowed: bool) -> Self {
        self.storage = allowed;
        self
    }

    pub fn clipboard_read(mut self, allowed: bool) -> Self {
        self.clipboard_read = allowed;
        self
    }

    pub fn clipboard_write(mut self, allowed: bool) -> Self {
        self.clipboard_write = allowed;
        self
    }

    /// Whether script may ask its host to exit.
    pub fn exit(mut self, allowed: bool) -> Self {
        self.exit = allowed;
        self
    }

    pub fn may_exit(&self) -> bool {
        self.exit
    }

    pub fn has_storage(&self) -> bool {
        self.storage
    }

    pub fn is_clipboard_readable(&self) -> bool {
        self.clipboard_read
    }

    pub fn is_clipboard_writable(&self) -> bool {
        self.clipboard_write
    }

    pub fn execute_grant(&self) -> &ExecuteGrant {
        &self.execute
    }

    /// Whether any filesystem write is permitted at all. `os.remove` and
    /// friends need this before their path is even resolved.
    pub fn has_write_access(&self) -> bool {
        !self.write_roots.is_empty()
    }

    pub fn has_read_access(&self) -> bool {
        !self.read_roots.is_empty()
    }

    pub fn may_run(&self, command: &str) -> bool {
        match &self.execute {
            ExecuteGrant::Denied => false,
            ExecuteGrant::Unrestricted => true,
            ExecuteGrant::Allowed(names) => names.iter().any(|name| name == command),
        }
    }

    pub fn may_reach(&self, host: &str) -> bool {
        self.network_hosts
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(host))
    }

    /// Whether an HTTP request is permitted by either a legacy unrestricted
    /// host grant or a scoped HTTP grant.
    pub fn may_request(
        &self,
        scheme: &str,
        host: &str,
        port: Option<u16>,
        method: &str,
        path: &str,
    ) -> bool {
        self.may_reach(host)
            || self
                .http_requests
                .iter()
                .any(|grant| grant.allows(scheme, host, port, method, path))
    }

    /// Opens the granted directory a path belongs to, and the path within it.
    ///
    /// The caller gets a **capability**, not a string: a directory handle that
    /// refuses to resolve outside itself, plus the path to use against it. Every
    /// operation then goes through that handle, so no name is resolved twice and
    /// there is no window between deciding a path is allowed and using it.
    ///
    /// That is the whole point, and the reason this replaced a resolver that
    /// returned a `PathBuf`. The old one compared strings; the one before *that*
    /// compared strings and then canonicalized, which caught a link that was
    /// already there and not one swapped in afterwards. `std` cannot express the
    /// difference — `cap-std` can, because a `Dir` carries the authority instead
    /// of describing it.
    ///
    /// The same resolver serves the `fs` modules, the capability-gated `os.*`
    /// functions and the asset source, so there is no second path policy to keep
    /// in sync.
    pub(crate) fn open(&self, path: &Path, access: Access) -> Result<Grant, CapabilityError> {
        let roots = match access {
            Access::Read => &self.read_roots,
            Access::Write => &self.write_roots,
        };
        if roots.is_empty() {
            return Err(CapabilityError::NotGranted(access));
        }

        for root in roots {
            // Resolved the same way a path is, because a granted directory need
            // not exist yet and a root reached through a link — `/var` on macOS
            // — must agree with a path that was not.
            let Some(resolved) = resolved_root(root) else {
                continue;
            };

            // An absolute path is allowed if it is already inside a root, so it
            // becomes relative before `Dir` sees it; `Dir` refuses an absolute
            // path outright, and rightly.
            let relative = if path.is_absolute() {
                // Resolved the same way the root was, or the two disagree about
                // a directory neither of them left: on macOS a temporary path
                // reaches `/private/var` through `/var`.
                let normalized = resolved_root(path).unwrap_or_else(|| normalize(path));
                match normalized.strip_prefix(&resolved) {
                    Ok(relative) => relative.to_path_buf(),
                    Err(_) => continue,
                }
            } else {
                // Lexical first, so the ordinary `../..` is refused with a
                // sentence about the grant rather than an `errno` from below.
                let normalized = normalize(&resolved.join(path));
                match normalized.strip_prefix(&resolved) {
                    Ok(relative) => relative.to_path_buf(),
                    Err(_) => continue,
                }
            };

            // A granted directory need not exist yet — an application's data
            // directory on first run is the ordinary case — and a write grant is
            // the host saying that directory is the application's. Making it is
            // honouring the grant, not exceeding it. A *read* grant on a missing
            // directory has nothing to offer and stays a denial.
            if access == Access::Write && !resolved.exists() {
                let _ = std::fs::create_dir_all(&resolved);
            }

            let Ok(dir) = Dir::open_ambient_dir(&resolved, ambient_authority()) else {
                continue;
            };

            // `.` and the root itself normalize to nothing, which is a name no
            // directory has. The root is a legitimate target — listing it is how
            // a script finds out what it was given — so it gets the name that
            // means it.
            let relative = if relative.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                relative
            };

            return Ok(Grant { dir, relative });
        }

        Err(CapabilityError::OutsideRoots {
            path: path.to_path_buf(),
            access,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Access {
    Read,
    Write,
}

impl Access {
    fn as_str(self) -> &'static str {
        match self {
            Access::Read => "read",
            Access::Write => "write",
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum CapabilityError {
    NotGranted(Access),
    OutsideRoots { path: PathBuf, access: Access },
    ExecuteDenied(String),
    StorageDenied,
}

impl std::fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CapabilityError::NotGranted(access) => write!(
                f,
                "filesystem {} is not granted; declare capabilities.fs.{} in the manifest",
                access.as_str(),
                access.as_str()
            ),
            CapabilityError::OutsideRoots { path, access } => write!(
                f,
                "`{}` is outside every granted {} root",
                path.display(),
                access.as_str()
            ),
            CapabilityError::ExecuteDenied(command) => write!(
                f,
                "running `{command}` is not granted; add it to capabilities.fs.execute in the manifest"
            ),
            CapabilityError::StorageDenied => {
                f.write_str("storage is not granted; set capabilities.storage to true")
            }
        }
    }
}

impl std::error::Error for CapabilityError {}

// ---------------------------------------------------------------------------
// The installed grant
// ---------------------------------------------------------------------------

/// Grants the application that is about to be loaded.
///
/// The grant lives on the default [`Policy`](crate::policy), which is what a
/// call inherits when nothing narrower is in force. A plugin host that runs
/// several applications in one runtime builds a policy for each instead of
/// calling this, so that two plugins can hold two different grants at the same
/// time.
///
/// This is deliberately not a knob on the engine. It used to be one, and an
/// engine constructed any other way — in a test, in a second embedding — would
/// then answer the security question differently from the one the host had
/// configured. A grant is a decision about the *application*, not about the
/// interpreter, and there is now nowhere for an engine to answer it
/// differently.
///
/// The default is [`Capabilities::default`], which allows nothing.
pub(crate) fn install(capabilities: Capabilities) {
    crate::policy::update_default(|policy| policy.with_capabilities(capabilities));
}

/// A granted directory, and the path to use against it.
///
/// Holding the directory open is what makes this a capability rather than a
/// claim: the handle cannot be made to name something outside itself, so an
/// operation on it cannot leave the grant however the filesystem changes
/// underneath.
#[derive(Debug)]
pub(crate) struct Grant {
    dir: Dir,
    relative: PathBuf,
}

impl Grant {
    /// The directory every operation goes through.
    // Only the symlink tests read the handle apart from the path, and those
    // are Unix-only — so on Windows this has no caller and `-D warnings` says
    // so.
    #[cfg(all(test, unix))]
    pub(crate) fn dir(&self) -> &Dir {
        &self.dir
    }

    /// The path within it. Never absolute, never `..`.
    #[cfg(test)]
    pub(crate) fn path(&self) -> &Path {
        &self.relative
    }

    /// What to call the target in a message. Not a path anything opens — the
    /// handle is — but the words a script needs to recognize what it asked for.
    pub fn describe(&self) -> String {
        self.relative.display().to_string()
    }

    /// Splits into the two halves, for work that moves to another thread.
    pub fn into_parts(self) -> (Dir, PathBuf) {
        (self.dir, self.relative)
    }
}

/// Resolves a root by asking the filesystem, not by trusting the string.
///
/// A granted directory need not exist yet, so this cannot simply canonicalize:
/// it resolves as far as the filesystem goes and keeps the rest verbatim. The
/// deepest existing ancestor is canonicalized, following every link on the way.
/// Whatever lies below it does not exist, so none of those components can yet
/// be a link. A dangling symlink at the boundary is refused rather than guessed
/// at, because its destination cannot be proven.
fn resolved_root(root: &Path) -> Option<PathBuf> {
    let (resolved, tail) = deepest_resolvable(root)?;
    let mut out = resolved;
    out.extend(tail.iter());
    Some(out)
}

/// Splits a path into its deepest ancestor that resolves, canonicalized, and
/// the components below it in order.
///
/// `canonicalize` fails for a path that does not exist and for one that dangles,
/// which is exactly the set that has to be walked past — and it follows every
/// link on the part that does resolve, which is the point.
fn deepest_resolvable(path: &Path) -> Option<(PathBuf, Vec<std::ffi::OsString>)> {
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    let mut current = path.to_path_buf();

    let resolved = loop {
        if let Ok(real) = current.canonicalize() {
            break real;
        }
        let name = current.file_name()?.to_os_string();
        if !current.pop() {
            return None;
        }
        tail.push(name);
    };

    tail.reverse();
    Some((resolved, tail))
}

/// Lexical normalization, which is the cheap half of the check. [`contain`] is
/// the half that holds when a component is a symlink.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real directory, because a grant is now an open handle rather than a
    /// string: there is nothing to hold onto a path that does not exist.
    struct Root(PathBuf);

    impl Root {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("gpui-shell-grant-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("a root");
            Self(path.canonicalize().expect("a canonical root"))
        }

        fn granted(&self) -> Capabilities {
            Capabilities::new()
                .read_roots([self.0.clone()])
                .write_roots([self.0.clone()])
        }
    }

    impl Drop for Root {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn network_host_grants_are_dns_case_insensitive() {
        let capabilities = Capabilities::new().network_hosts(["API.Example.COM".to_owned()]);
        assert!(capabilities.may_reach("api.example.com"));
        assert!(capabilities.may_reach("Api.Example.Com"));
    }

    #[test]
    fn traversal_out_of_a_root_is_rejected() {
        let root = Root::new("traversal");
        let error = root
            .granted()
            .open(Path::new("../../etc/passwd"), Access::Read)
            .unwrap_err();
        assert!(matches!(error, CapabilityError::OutsideRoots { .. }));
    }

    #[test]
    fn a_relative_path_resolves_inside_its_root() {
        let root = Root::new("relative");
        let grant = root
            .granted()
            .open(Path::new("items.json"), Access::Write)
            .expect("a path inside the root");
        assert_eq!(grant.path(), Path::new("items.json"));
    }

    #[test]
    fn an_absolute_path_inside_a_root_becomes_relative_to_it() {
        let root = Root::new("absolute");
        let grant = root
            .granted()
            .open(&root.0.join("nested/items.json"), Access::Write)
            .expect("a path inside the root");
        assert_eq!(grant.path(), Path::new("nested/items.json"));
    }

    /// An application's data directory does not exist before its first run, and
    /// a write grant is the host saying that directory is the application's.
    #[test]
    fn a_write_grant_materializes_a_root_that_is_not_there_yet() {
        let root = Root::new("missing");
        let inner = root.0.join("not-yet");
        let capabilities = Capabilities::new().write_roots([inner.clone()]);

        let grant = capabilities
            .open(Path::new("settings.json"), Access::Write)
            .expect("a write grant should make its own root");
        assert_eq!(grant.path(), Path::new("settings.json"));
        assert!(inner.is_dir());

        // A read grant has nothing to offer a directory that is not there.
        let reading = Capabilities::new().read_roots([root.0.join("still-not-there")]);
        assert!(reading.open(Path::new("x"), Access::Read).is_err());
    }

    #[test]
    fn nothing_resolves_without_a_grant() {
        let error = Capabilities::new()
            .open(Path::new("items.json"), Access::Read)
            .unwrap_err();
        assert_eq!(error, CapabilityError::NotGranted(Access::Read));
    }

    #[test]
    fn execute_is_denied_by_default_and_allowlisted_when_granted() {
        assert!(!Capabilities::new().may_run("git"));
        let capabilities = Capabilities::new().execute(ExecuteGrant::Allowed(vec!["git".into()]));
        assert!(capabilities.may_run("git"));
        assert!(!capabilities.may_run("curl"));
    }
}

/// Unix-only, and the tests say why rather than the module: creating a symlink
/// on Windows needs Developer Mode or an elevated process, which CI has neither
/// of. A test that quietly does nothing when the link cannot be made would pass
/// while testing nothing, which is worse than not running.
#[cfg(all(test, unix))]
mod symlink_tests {
    use super::*;

    /// A grant is a promise about a *directory*, and a symlink is the oldest
    /// way to make a path that is lexically inside one point somewhere else.
    /// These use real files, because the whole question is what the filesystem
    /// does rather than what the string looks like.
    struct Sandbox {
        root: PathBuf,
    }

    impl Sandbox {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir()
                .join(format!("gpui-shell-escape-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(root.join("inside")).expect("a sandbox");
            std::fs::write(root.join("inside/ours.txt"), "ours").expect("a file");
            Self {
                root: root.canonicalize().expect("a canonical sandbox"),
            }
        }

        /// Somewhere outside the grant, standing in for `/etc`.
        fn outside(&self) -> PathBuf {
            let outside = self.root.join("outside");
            std::fs::create_dir_all(&outside).expect("somewhere outside");
            std::fs::write(outside.join("secret.txt"), "secret").expect("a secret");
            outside
        }

        fn granted(&self) -> Capabilities {
            Capabilities::new()
                .read_roots([self.root.join("inside")])
                .write_roots([self.root.join("inside")])
        }

        fn link(&self, name: &str, target: &Path) {
            std::os::unix::fs::symlink(target, self.root.join("inside").join(name))
                .expect("a symlink");
        }
    }

    impl Drop for Sandbox {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn a_symlink_out_of_a_root_cannot_be_read_through() {
        let sandbox = Sandbox::new("read");
        let outside = sandbox.outside();
        sandbox.link("escape", &outside);
        let capabilities = sandbox.granted();

        // The refusal is at the operation, not at the resolution: the grant
        // hands back a directory that cannot be made to name anything outside
        // itself, and that is what a read runs against. Testing the resolution
        // would be testing the old design.
        let grant = capabilities
            .open(Path::new("escape/secret.txt"), Access::Read)
            .expect("lexically inside the root");
        assert!(
            grant.dir().read_to_string(grant.path()).is_err(),
            "reading through a symlink left the granted root"
        );

        // And an ordinary path still works, which is the point of the grant.
        let ours = capabilities
            .open(Path::new("ours.txt"), Access::Read)
            .expect("a path inside the root");
        assert_eq!(
            ours.dir()
                .read_to_string(ours.path())
                .expect("our own file"),
            "ours"
        );
    }

    #[test]
    fn a_symlink_out_of_a_root_cannot_be_written_through() {
        let sandbox = Sandbox::new("write");
        let outside = sandbox.outside();
        sandbox.link("escape", &outside);
        let capabilities = sandbox.granted();

        let planted = capabilities
            .open(Path::new("escape/planted.txt"), Access::Write)
            .expect("lexically inside the root");
        assert!(
            planted.dir().write(planted.path(), b"x").is_err(),
            "writing through a symlink left the granted root"
        );
        assert!(!outside.join("planted.txt").exists());

        let made = capabilities
            .open(Path::new("escape/planted"), Access::Write)
            .expect("lexically inside the root");
        assert!(made.dir().create_dir_all(made.path()).is_err());
        assert!(!outside.join("planted").exists());

        // A new file directly in the root is still allowed.
        let ours = capabilities
            .open(Path::new("new.txt"), Access::Write)
            .expect("a path inside the root");
        ours.dir()
            .write(ours.path(), b"ours")
            .expect("our own file");
    }

    /// The case that check-then-use could never cover: the link appears *after*
    /// the path was judged. A handle cannot be talked out of its directory, so
    /// the timing stops mattering.
    #[test]
    fn a_symlink_planted_after_the_check_is_still_refused() {
        let sandbox = Sandbox::new("toctou");
        let outside = sandbox.outside();
        let capabilities = sandbox.granted();

        // Judged while `escape` is an ordinary missing name.
        let grant = capabilities
            .open(Path::new("escape/secret.txt"), Access::Read)
            .expect("lexically inside the root");

        // Now somebody else makes it a link. Under the old resolver this is the
        // window: the check had already passed and the syscall would resolve the
        // name a second time.
        sandbox.link("escape", &outside);

        assert!(
            grant.dir().read_to_string(grant.path()).is_err(),
            "a link planted after the check was followed out of the root"
        );
    }

    #[test]
    fn a_dangling_symlink_is_not_followed() {
        let sandbox = Sandbox::new("dangling");
        let outside = sandbox.outside();
        sandbox.link("dangling", &outside.join("not-there.txt"));
        let capabilities = sandbox.granted();

        let grant = capabilities
            .open(Path::new("dangling"), Access::Write)
            .expect("lexically inside the root");
        assert!(
            grant.dir().write(grant.path(), b"x").is_err(),
            "a write followed a dangling symlink out of the root"
        );
        assert!(!outside.join("not-there.txt").exists());
    }
}

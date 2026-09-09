use std::{
    collections::BTreeMap,
    fs::File,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use anyhow::{Context as _, Result, bail};
use fs2::FileExt as _;
use sha2::{Digest as _, Sha256};
use wait_timeout::ChildExt as _;

use crate::plugin::{GitDependency, PluginManifest};

/// Materializes Git-backed JavaScript packages in gpui-shell's user cache.
pub(crate) struct GitDependencyStore {
    root: PathBuf,
}

#[derive(Clone, Debug)]
pub(crate) struct MaterializedDependency {
    pub(crate) root: PathBuf,
    pub(crate) entry: PathBuf,
}

impl GitDependencyStore {
    pub(crate) fn for_user() -> Result<Self> {
        Self::for_user_with_environment(|variable| std::env::var_os(variable))
    }

    fn for_user_with_environment(
        environment: impl Fn(&str) -> Option<std::ffi::OsString>,
    ) -> Result<Self> {
        let Some((variable, home)) = ["HOME", "USERPROFILE"].into_iter().find_map(|variable| {
            environment(variable)
                .filter(|value| !value.is_empty())
                .map(|value| (variable, PathBuf::from(value)))
        }) else {
            bail!(
                "cannot locate the Git dependency cache: HOME or USERPROFILE must name an absolute user directory"
            );
        };
        if !home.is_absolute() {
            bail!(
                "cannot locate the Git dependency cache: {variable} must be an absolute path, got `{}`",
                home.display()
            );
        }
        Ok(Self::new(dependency_cache_root(&home)))
    }

    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub(crate) fn materialize(
        &self,
        name: &str,
        dependency: &GitDependency,
    ) -> Result<MaterializedDependency> {
        std::fs::create_dir_all(&self.root)
            .with_context(|| format!("creating Git dependency cache {}", self.root.display()))?;
        let remote_key = digest(&[("git", dependency.git())]);
        let locks = self.root.join("locks");
        let mirrors = self.root.join("mirrors");
        let checkouts = self.root.join("checkouts").join(&remote_key);
        std::fs::create_dir_all(&locks)?;
        std::fs::create_dir_all(&mirrors)?;
        std::fs::create_dir_all(&checkouts)?;
        let _lock = CacheLock::acquire(&locks.join(format!("{remote_key}.lock")), name)?;

        let mirror = mirrors.join(format!("{remote_key}.git"));
        if !mirror.is_dir() {
            let temporary = temporary_path(&mirrors, &remote_key);
            let mut command = git_command();
            command
                .args(["clone", "--mirror", "--"])
                .arg(dependency.git())
                .arg(&temporary);
            if let Err(error) = run_command(name, "clone", command) {
                let _ = std::fs::remove_dir_all(&temporary);
                return Err(error);
            }
            match std::fs::rename(&temporary, &mirror) {
                Ok(()) => {}
                Err(error) if mirror.is_dir() => {
                    let _ = std::fs::remove_dir_all(&temporary);
                    tracing::debug!("another process published {}: {error}", mirror.display());
                }
                Err(error) => return Err(error).context("publishing Git dependency mirror"),
            }
        }

        let configured = configured_origin(name, &mirror)?;
        if configured != dependency.git() {
            bail!(
                "Git dependency `{name}` cache origin is `{}`, expected `{}`; remove {} and retry",
                configured,
                dependency.git(),
                mirror.display()
            );
        }

        let reference = match (
            dependency.uses_package_entry(),
            dependency.reference(),
            dependency.branch(),
            dependency.tag(),
        ) {
            (true, Some(reference), None, None) => reference.to_owned(),
            (true, None, None, None) => "HEAD".to_owned(),
            (false, None, Some(branch), None) => format!("refs/heads/{branch}"),
            (false, None, None, Some(tag)) => format!("refs/tags/{tag}"),
            _ => unreachable!("manifest validation requires one supported Git selector"),
        };
        let mut fetch = git_command();
        fetch.args(["fetch", "--force", "--depth", "1", "origin", &reference]);
        fetch.current_dir(&mirror);
        run_command(name, "fetch", fetch)?;

        let mut rev_parse = git_command();
        rev_parse.args(["rev-parse", "FETCH_HEAD"]);
        rev_parse.current_dir(&mirror);
        let commit = output_text(name, "resolve fetched commit", rev_parse)?;
        let commit = commit.trim();
        if !(commit.len() == 40 || commit.len() == 64)
            || !commit.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            bail!("Git dependency `{name}` resolved an invalid commit id `{commit}`");
        }

        let checkout = checkouts.join(commit);
        if !checkout.join(".git").is_dir() {
            let temporary = temporary_path(&checkouts, commit);
            let mut clone = git_command();
            clone
                .args(["clone", "--no-checkout", "--"])
                .arg(&mirror)
                .arg(&temporary);
            if let Err(error) = run_command(name, "create immutable checkout", clone) {
                let _ = std::fs::remove_dir_all(&temporary);
                return Err(error);
            }
            let mut checkout_commit = git_command();
            checkout_commit
                .args(["checkout", "--force", "--detach", commit])
                .current_dir(&temporary);
            if let Err(error) = run_command(name, "checkout fetched commit", checkout_commit) {
                let _ = std::fs::remove_dir_all(&temporary);
                return Err(error);
            }
            match std::fs::rename(&temporary, &checkout) {
                Ok(()) => {}
                Err(error) if checkout.join(".git").is_dir() => {
                    let _ = std::fs::remove_dir_all(&temporary);
                    tracing::debug!("another process published {}: {error}", checkout.display());
                }
                Err(error) => return Err(error).context("publishing Git dependency checkout"),
            }
        }

        let root = checkout
            .canonicalize()
            .with_context(|| format!("resolving dependency checkout {}", checkout.display()))?;
        let entry_name = dependency_entry_name(name, dependency, &root)?;
        let entry = root
            .join(&entry_name)
            .canonicalize()
            .with_context(|| format!("Git dependency `{name}` has no entry `{}`", entry_name))?;
        if !entry.starts_with(&root) || !entry.is_file() {
            bail!(
                "Git dependency `{name}` entry `{}` is not a file inside its checkout",
                entry_name
            );
        }

        Ok(MaterializedDependency { root, entry })
    }

    /// Materializes every dependency a manifest declares, in manifest order.
    pub(crate) fn materialize_all(
        &self,
        manifest: &PluginManifest,
    ) -> Result<BTreeMap<String, MaterializedDependency>> {
        manifest
            .dependencies()
            .iter()
            .map(|(name, dependency)| {
                self.materialize(name, dependency)
                    .map(|materialized| (name.clone(), materialized))
            })
            .collect()
    }

    /// Points an editor at the packages this application will import.
    ///
    /// The runtime answers `import { style } from "omarchy-ui"` from the
    /// manifest. An editor answers it by walking `node_modules` up from the
    /// importing file, so nothing in the application directory tells it where
    /// the package is: a correct import is underlined as a missing module, and
    /// every name behind it loses its type, its parameters and its
    /// documentation.
    ///
    /// Linking the checkout under the name the manifest gave it closes that gap
    /// without writing a second description of the package. The editor reads
    /// the same files the runtime is about to execute, so the signatures and
    /// JSDoc it shows are the package's own and cannot drift from what runs.
    ///
    /// The links live in `node_modules` — already ignored by every JavaScript
    /// project — rather than in `gpui-kit.d.ts` or a `tsconfig.json`, because they
    /// name a machine-specific cache path and both of those files are committed.
    ///
    /// Only entries this store owns are ever replaced or removed: a symlink
    /// into its own cache, or a directory carrying the marker file it writes.
    /// Anything else in `node_modules` belongs to whoever installed it.
    /// Returns the links that were created or repointed.
    pub(crate) fn link_for_editor(
        &self,
        app_root: &Path,
        dependencies: &BTreeMap<String, MaterializedDependency>,
    ) -> Result<Vec<PathBuf>> {
        let modules = app_root.join(EDITOR_MODULE_DIRECTORY);
        if dependencies.is_empty() && !modules.is_dir() {
            return Ok(Vec::new());
        }
        std::fs::create_dir_all(&modules)
            .with_context(|| format!("creating {}", modules.display()))?;

        let mut linked = Vec::new();
        let mut declared = Vec::new();
        for (name, dependency) in dependencies {
            let link = modules.join(name);
            declared.push(link.clone());
            if let Some(parent) = link.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating {}", parent.display()))?;
            }
            if self.relink(&link, name, dependency)? {
                linked.push(link);
            }
        }
        self.prune(&modules, &declared);
        Ok(linked)
    }

    /// Makes `link` point at `dependency`, and reports whether it had to.
    fn relink(&self, link: &Path, name: &str, dependency: &MaterializedDependency) -> Result<bool> {
        match std::fs::symlink_metadata(link) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                let current = std::fs::read_link(link)
                    .with_context(|| format!("reading {}", link.display()))?;
                if current == dependency.root {
                    return Ok(false);
                }
                if !current.starts_with(&self.root) {
                    tracing::debug!(
                        "leaving {} alone: it already points outside the dependency cache",
                        link.display()
                    );
                    return Ok(false);
                }
                remove_directory_link(link)
                    .with_context(|| format!("replacing {}", link.display()))?;
            }
            Ok(_) if is_editor_link_stub(link) => {
                std::fs::remove_dir_all(link)
                    .with_context(|| format!("replacing {}", link.display()))?;
            }
            Ok(_) => {
                tracing::debug!(
                    "leaving {} alone: an installed package already claims that name",
                    link.display()
                );
                return Ok(false);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| format!("inspecting {}", link.display()));
            }
        }

        match symlink_directory(&dependency.root, link) {
            Ok(()) => Ok(true),
            // Windows refuses a symlink to an unprivileged process unless
            // developer mode is on. A package that re-exports the checkout by
            // absolute path types the same way for a bare import; only a
            // package-subpath import is left unresolved.
            Err(error) => {
                tracing::debug!(
                    "linking dependency `{name}` failed ({error}); writing a re-export instead"
                );
                write_editor_link_stub(link, name, dependency)?;
                Ok(true)
            }
        }
    }

    /// Removes the links of dependencies the manifest no longer declares.
    ///
    /// Bounded at the depth a scoped name needs, and confined to entries this
    /// store wrote, so a stale link cannot outlive its manifest entry and an
    /// installed package cannot be mistaken for one.
    fn prune(&self, modules: &Path, declared: &[PathBuf]) {
        let mut pending = vec![(modules.to_path_buf(), 0usize)];
        while let Some((directory, depth)) = pending.pop() {
            for entry in std::fs::read_dir(&directory)
                .into_iter()
                .flatten()
                .flatten()
            {
                let path = entry.path();
                if declared.contains(&path) {
                    continue;
                }
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };
                if file_type.is_symlink() {
                    if std::fs::read_link(&path).is_ok_and(|target| target.starts_with(&self.root))
                    {
                        let _ = remove_directory_link(&path);
                    }
                } else if file_type.is_dir() {
                    if is_editor_link_stub(&path) {
                        let _ = std::fs::remove_dir_all(&path);
                    } else if depth < 1 {
                        pending.push((path, depth + 1));
                    }
                }
            }
        }
    }
}

/// Where an editor looks for a bare module specifier.
const EDITOR_MODULE_DIRECTORY: &str = "node_modules";

/// Names the file that marks a directory as one [`GitDependencyStore`] wrote.
///
/// A marker rather than a naming convention: it is what lets pruning tell its
/// own re-export package from an installed one that happens to sit under the
/// same name.
const EDITOR_LINK_MARKER: &str = ".gpui-shell-link";

fn is_editor_link_stub(link: &Path) -> bool {
    link.join(EDITOR_LINK_MARKER).is_file()
}

/// Writes the package that stands in for a symlink the platform refused.
fn write_editor_link_stub(
    link: &Path,
    name: &str,
    dependency: &MaterializedDependency,
) -> Result<()> {
    std::fs::create_dir_all(link).with_context(|| format!("creating {}", link.display()))?;
    let manifest = serde_json::json!({
        "name": name,
        "private": true,
        "type": "module",
        "main": "index.js",
    });
    let entry = module_specifier(&dependency.entry);
    std::fs::write(
        link.join(EDITOR_LINK_MARKER),
        format!("{}\n", dependency.root.display()),
    )?;
    std::fs::write(
        link.join("package.json"),
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    std::fs::write(
        link.join("index.js"),
        format!(
            "// Written by gpui-shell so an editor can resolve `{name}`.\n             // The runtime resolves the manifest entry directly and never reads this file.\n             export * from {};\n",
            serde_json::to_string(&entry)?
        ),
    )?;
    Ok(())
}

/// A path as TypeScript wants to read it: rooted, with forward slashes.
fn module_specifier(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(unix)]
fn symlink_directory(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn symlink_directory(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(target, link)
}

/// Removes a symlink to a directory, which the two platforms spell differently.
#[cfg(unix)]
fn remove_directory_link(link: &Path) -> std::io::Result<()> {
    std::fs::remove_file(link)
}

#[cfg(windows)]
fn remove_directory_link(link: &Path) -> std::io::Result<()> {
    std::fs::remove_dir(link)
}

fn dependency_entry_name(name: &str, dependency: &GitDependency, root: &Path) -> Result<String> {
    if !dependency.uses_package_entry() {
        return Ok(dependency.entry().to_owned());
    }

    let manifest = root.join("package.json");
    let manifest = match manifest.symlink_metadata() {
        Ok(_) => {
            let manifest = manifest
                .canonicalize()
                .with_context(|| format!("resolving package.json for Git dependency `{name}`"))?;
            if !manifest.starts_with(root) || !manifest.is_file() {
                bail!("Git dependency `{name}` package.json is not a file inside its checkout");
            }
            Some(manifest)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(error)
                .with_context(|| format!("inspecting package.json for Git dependency `{name}`"));
        }
    };
    let Some(manifest) = manifest else {
        return Ok(default_package_entry());
    };
    let source = std::fs::read_to_string(&manifest)
        .with_context(|| format!("reading package.json for Git dependency `{name}`"))?;
    let value: serde_json::Value = serde_json::from_str(&source).map_err(|error| {
        anyhow::anyhow!("Git dependency `{name}` package.json must contain valid JSON: {error}")
    })?;
    let object = value.as_object().ok_or_else(|| {
        anyhow::anyhow!("Git dependency `{name}` package.json must contain a JSON object")
    })?;
    let entry = match object.get("main") {
        None => default_package_entry(),
        Some(serde_json::Value::String(entry)) => entry.clone(),
        Some(_) => {
            bail!("Git dependency `{name}` package.json `main` must be a string");
        }
    };
    let path = Path::new(&entry);
    if entry.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
        || entry.contains(['\\', ':'])
    {
        bail!(
            "Git dependency `{name}` package.json `main` `{entry}` must be a path inside its checkout"
        );
    }
    Ok(entry)
}

fn default_package_entry() -> String {
    "index.js".to_owned()
}

fn dependency_cache_root(home: &Path) -> PathBuf {
    home.join(".gpui-shell").join("cache").join("dependencies")
}

const GIT_TIMEOUT: Duration = Duration::from_secs(30);
const LOCK_TIMEOUT: Duration = Duration::from_secs(2 * 60);

fn git_command() -> Command {
    let mut command = Command::new("git");
    command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "Never")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

fn run_command(name: &str, operation: &str, mut command: Command) -> Result<Output> {
    let mut child = command
        .spawn()
        .with_context(|| format!("starting git to {operation} dependency `{name}`"))?;
    let status = child
        .wait_timeout(GIT_TIMEOUT)
        .with_context(|| format!("waiting for git to {operation} dependency `{name}`"))?;
    if status.is_none() {
        let _ = child.kill();
        let _ = child.wait();
        bail!(
            "git timed out after {} seconds while trying to {operation} dependency `{name}`",
            GIT_TIMEOUT.as_secs()
        );
    }
    let output = child.wait_with_output()?;
    if output.status.success() {
        return Ok(output);
    }
    let detail = String::from_utf8_lossy(&output.stderr);
    bail!(
        "could not {operation} Git dependency `{name}`: {}",
        detail.trim()
    )
}

fn output_text(name: &str, operation: &str, command: Command) -> Result<String> {
    let output = run_command(name, operation, command)?;
    String::from_utf8(output.stdout).with_context(|| {
        format!("git returned non-UTF-8 output while trying to {operation} `{name}`")
    })
}

fn configured_origin(name: &str, mirror: &Path) -> Result<String> {
    let mut command = git_command();
    command
        .args(["config", "--null", "--get-all", "remote.origin.url"])
        .current_dir(mirror);
    let output = run_command(name, "inspect cached origin", command)?;
    let Some(origin) = output.stdout.strip_suffix(&[0]) else {
        bail!(
            "Git dependency `{name}` cache origin config is malformed; remove {} and retry",
            mirror.display()
        );
    };
    if origin.is_empty() || origin.contains(&0) {
        bail!(
            "Git dependency `{name}` cache origin config must contain exactly one non-empty URL; remove {} and retry",
            mirror.display()
        );
    }
    String::from_utf8(origin.to_vec()).with_context(|| {
        format!("git returned a non-UTF-8 cached origin URL while inspecting `{name}`")
    })
}

fn digest(fields: &[(&str, &str)]) -> String {
    let mut digest = Sha256::new();
    for (kind, value) in fields {
        digest.update(kind.len().to_le_bytes());
        digest.update(kind.as_bytes());
        digest.update(value.len().to_le_bytes());
        digest.update(value.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn temporary_path(parent: &Path, label: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    parent.join(format!(
        ".{label}.tmp-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

struct CacheLock(File);

impl CacheLock {
    fn acquire(path: &Path, name: &str) -> Result<Self> {
        let file = File::options()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)
            .with_context(|| format!("opening Git dependency cache lock {}", path.display()))?;
        let started = Instant::now();
        loop {
            match file.try_lock_exclusive() {
                Ok(()) => return Ok(Self(file)),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if started.elapsed() >= LOCK_TIMEOUT {
                        bail!(
                            "timed out waiting for another process to finish Git dependency `{name}`"
                        );
                    }
                    std::thread::sleep(Duration::from_millis(25));
                }
                Err(error) => return Err(error).context("locking Git dependency cache"),
            }
        }
    }
}

impl Drop for CacheLock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EDITOR_LINK_MARKER, EDITOR_MODULE_DIRECTORY, GitDependencyStore, MaterializedDependency,
        dependency_cache_root, digest,
    };
    use crate::plugin::PluginManifest;
    use std::{
        ffi::OsString,
        path::{Path, PathBuf},
        process::Command,
        sync::{
            Arc, Barrier,
            atomic::{AtomicU64, Ordering},
        },
    };

    static NEXT: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn a_user_dependency_cache_lives_in_the_shell_cache() {
        assert_eq!(
            dependency_cache_root(Path::new("/home/example")),
            PathBuf::from("/home/example/.gpui-shell/cache/dependencies")
        );
    }

    #[test]
    fn for_user_wires_home_to_the_shell_cache_root() {
        let store = GitDependencyStore::for_user_with_environment(|variable| match variable {
            "HOME" => Some(OsString::from("/home/example")),
            _ => None,
        })
        .expect("an absolute HOME should select a private cache root");

        assert_eq!(
            store.root,
            PathBuf::from("/home/example/.gpui-shell/cache/dependencies")
        );
    }

    #[test]
    fn for_user_uses_userprofile_when_home_is_missing() {
        let store = GitDependencyStore::for_user_with_environment(|variable| match variable {
            "USERPROFILE" => Some(OsString::from("/profiles/example")),
            _ => None,
        })
        .expect("an absolute USERPROFILE should select a private cache root");

        assert_eq!(
            store.root,
            PathBuf::from("/profiles/example/.gpui-shell/cache/dependencies")
        );
    }

    #[test]
    fn for_user_ignores_an_empty_home_before_userprofile() {
        let store = GitDependencyStore::for_user_with_environment(|variable| match variable {
            "HOME" => Some(OsString::new()),
            "USERPROFILE" => Some(OsString::from("/profiles/example")),
            _ => None,
        })
        .expect("an empty HOME should allow an absolute USERPROFILE");

        assert_eq!(
            store.root,
            PathBuf::from("/profiles/example/.gpui-shell/cache/dependencies")
        );
    }

    #[test]
    fn for_user_rejects_missing_or_empty_home_variables() {
        for (home, userprofile) in [(None, None), (Some(OsString::new()), Some(OsString::new()))] {
            let result = GitDependencyStore::for_user_with_environment(|variable| match variable {
                "HOME" => home.clone(),
                "USERPROFILE" => userprofile.clone(),
                _ => None,
            });
            let error = result.err().expect("a private home directory is required");

            assert!(
                error.to_string().contains("HOME or USERPROFILE"),
                "{error:#}"
            );
        }
    }

    #[test]
    fn for_user_rejects_a_relative_selected_home() {
        let result = GitDependencyStore::for_user_with_environment(|variable| match variable {
            "HOME" => Some(OsString::from("relative/home")),
            "USERPROFILE" => Some(OsString::from("/profiles/example")),
            _ => None,
        });
        let error = result
            .err()
            .expect("a relative HOME must not select a shared working-directory cache");

        assert!(error.to_string().contains("HOME"), "{error:#}");
        assert!(error.to_string().contains("absolute"), "{error:#}");
        assert!(error.to_string().contains("relative/home"), "{error:#}");
    }

    struct GitFixture {
        root: PathBuf,
        remote: PathBuf,
        cache: PathBuf,
    }

    impl GitFixture {
        fn new() -> Self {
            let unique = NEXT.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "gpui-shell-git-dependency-{}-{unique}",
                std::process::id()
            ));
            let remote = root.join("remote");
            let cache = root.join("cache");
            std::fs::create_dir_all(&remote).expect("fixture directory");
            git(&remote, &["init", "--initial-branch=main"]);
            git(&remote, &["config", "user.name", "gpui-shell test"]);
            git(
                &remote,
                &["config", "user.email", "gpui-shell@example.invalid"],
            );
            Self {
                root,
                remote,
                cache,
            }
        }

        fn commit(&self, source: &str, message: &str) {
            std::fs::write(self.remote.join("index.js"), source).expect("dependency source");
            git(&self.remote, &["add", "index.js"]);
            git(&self.remote, &["commit", "-m", message]);
        }

        fn dependency(&self, selector: &str) -> crate::plugin::GitDependency {
            self.dependency_at(&self.remote.to_string_lossy(), selector)
        }

        fn dependency_at(&self, git_url: &str, selector: &str) -> crate::plugin::GitDependency {
            let manifest = format!(
                r#"{{
                    "id": "com.example.fixture",
                    "name": "Fixture",
                    "entry": "main.js",
                    "dependencies": {{
                        "omarchy-ui": {{
                            "git": {},
                            {selector}
                        }}
                    }}
                }}"#,
                serde_json::to_string(git_url).expect("remote URL as JSON")
            );
            PluginManifest::parse(&manifest)
                .expect("fixture manifest")
                .dependencies()["omarchy-ui"]
                .clone()
        }

        fn package_dependency(&self, reference: Option<&str>) -> crate::plugin::GitDependency {
            let remote = format!("file://{}", self.remote.display());
            let source = match reference {
                Some(reference) => format!("{remote}#{reference}"),
                None => remote,
            };
            let manifest = format!(
                r#"{{
                    "id": "com.example.fixture",
                    "name": "Fixture",
                    "entry": "main.js",
                    "dependencies": {{ "omarchy-ui": {} }}
                }}"#,
                serde_json::to_string(&source).expect("dependency as JSON")
            );
            PluginManifest::parse(&manifest)
                .expect("fixture package dependency")
                .dependencies()["omarchy-ui"]
                .clone()
        }

        fn write(&self, path: &str, source: &str) {
            let path = self.remote.join(path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("dependency source parent");
            }
            std::fs::write(path, source).expect("dependency source");
        }

        fn commit_all(&self, message: &str) {
            git(&self.remote, &["add", "."]);
            git(&self.remote, &["commit", "-m", message]);
        }

        fn head(&self) -> String {
            git_output(&self.remote, &["rev-parse", "HEAD"])
        }
    }

    impl Drop for GitFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn git(directory: &Path, arguments: &[&str]) {
        let output = Command::new("git")
            .args(arguments)
            .current_dir(directory)
            .output()
            .expect("git must be installed for the test");
        assert!(
            output.status.success(),
            "git {} failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn git_output(directory: &Path, arguments: &[&str]) -> String {
        let output = Command::new("git")
            .args(arguments)
            .current_dir(directory)
            .output()
            .expect("git must be installed for the test");
        assert!(
            output.status.success(),
            "git {} failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .expect("git fixture output should be UTF-8")
            .trim()
            .to_owned()
    }

    #[test]
    fn package_dependencies_resolve_branch_tag_commit_ish_and_remote_head() {
        let fixture = GitFixture::new();
        fixture.commit("export const version = 1;", "tagged");
        let tagged_commit = fixture.head();
        git(&fixture.remote, &["tag", "v1"]);
        fixture.commit("export const version = 2;", "current");
        let store = GitDependencyStore::new(fixture.cache.clone());

        for (reference, expected) in [
            (Some("main"), "export const version = 2;"),
            (Some("v1"), "export const version = 1;"),
            (Some(tagged_commit.as_str()), "export const version = 1;"),
            (None, "export const version = 2;"),
        ] {
            let dependency = fixture.package_dependency(reference);
            let package = store
                .materialize("omarchy-ui", &dependency)
                .expect("the Git reference should resolve");
            assert_eq!(std::fs::read_to_string(package.entry).unwrap(), expected);
        }
    }

    #[test]
    fn package_dependencies_read_package_main_or_default_to_index_js() {
        let custom = GitFixture::new();
        custom.write("dist/public.js", "export const entry = 'package main';");
        custom.write("package.json", r#"{ "main": "dist/public.js" }"#);
        custom.commit_all("custom package entry");
        let package = GitDependencyStore::new(custom.cache.clone())
            .materialize("omarchy-ui", &custom.package_dependency(Some("main")))
            .expect("package.json main should select the entry");
        assert_eq!(
            std::fs::read_to_string(package.entry).unwrap(),
            "export const entry = 'package main';"
        );

        let defaulted = GitFixture::new();
        defaulted.commit(
            "export const entry = 'index default';",
            "default package entry",
        );
        let package = GitDependencyStore::new(defaulted.cache.clone())
            .materialize("omarchy-ui", &defaulted.package_dependency(Some("main")))
            .expect("a missing package.json should default to index.js");
        assert_eq!(
            std::fs::read_to_string(package.entry).unwrap(),
            "export const entry = 'index default';"
        );
    }

    #[test]
    fn malformed_or_non_string_package_main_is_rejected() {
        for (package_json, expected) in [
            ("{ not JSON", "valid JSON"),
            (r#"{ "main": 42 }"#, "string"),
            (r#"{ "main": null }"#, "string"),
        ] {
            let fixture = GitFixture::new();
            fixture.write("index.js", "export const executed = true;");
            fixture.write("package.json", package_json);
            fixture.commit_all("invalid package manifest");

            let error = GitDependencyStore::new(fixture.cache.clone())
                .materialize("omarchy-ui", &fixture.package_dependency(Some("main")))
                .expect_err("an invalid package manifest must fail before execution");
            assert!(error.to_string().contains("package.json"), "{error:#}");
            assert!(error.to_string().contains(expected), "{error:#}");
        }
    }

    #[test]
    fn package_main_must_name_a_file_confined_to_the_checkout() {
        for (main, expected) in [("../private.js", "inside"), ("dist", "file")] {
            let fixture = GitFixture::new();
            fixture.write("dist/nested.js", "export const nested = true;");
            fixture.write(
                "package.json",
                &serde_json::json!({ "main": main }).to_string(),
            );
            fixture.commit_all("unsafe package entry");

            let error = GitDependencyStore::new(fixture.cache.clone())
                .materialize("omarchy-ui", &fixture.package_dependency(Some("main")))
                .expect_err("package main must resolve to an in-checkout file");
            assert!(error.to_string().contains(expected), "{error:#}");
        }
    }

    #[test]
    fn a_branch_dependency_refreshes_to_the_remote_head_on_each_materialization() {
        let fixture = GitFixture::new();
        fixture.commit("export const version = 1;", "first");
        let dependency = fixture.dependency(r#""branch": "main""#);
        let store = GitDependencyStore::new(fixture.cache.clone());

        let first = store
            .materialize("omarchy-ui", &dependency)
            .expect("first checkout");
        assert_eq!(
            std::fs::read_to_string(&first.entry).unwrap(),
            "export const version = 1;"
        );

        fixture.commit("export const version = 2;", "second");
        let second = store
            .materialize("omarchy-ui", &dependency)
            .expect("updated checkout");
        assert_eq!(
            std::fs::read_to_string(&second.entry).unwrap(),
            "export const version = 2;"
        );
        assert_eq!(
            std::fs::read_to_string(&first.entry).unwrap(),
            "export const version = 1;",
            "a refresh must not mutate a checkout retained by an older module generation"
        );
    }

    #[test]
    fn a_tag_dependency_stays_at_the_tagged_commit() {
        let fixture = GitFixture::new();
        fixture.commit("export const version = 1;", "tagged");
        git(&fixture.remote, &["tag", "v1"]);
        fixture.commit("export const version = 2;", "later");
        let dependency = fixture.dependency(r#""tag": "v1""#);
        let store = GitDependencyStore::new(fixture.cache.clone());

        let package = store
            .materialize("omarchy-ui", &dependency)
            .expect("tag checkout");
        assert_eq!(
            std::fs::read_to_string(&package.entry).unwrap(),
            "export const version = 1;"
        );
    }

    #[test]
    fn concurrent_materializations_publish_one_valid_checkout() {
        let fixture = GitFixture::new();
        fixture.commit("export const version = 1;", "first");
        let dependency = fixture.dependency(r#""branch": "main""#);
        let barrier = Arc::new(Barrier::new(2));

        let workers: Vec<_> = (0..2)
            .map(|_| {
                let cache = fixture.cache.clone();
                let dependency = dependency.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    GitDependencyStore::new(cache)
                        .materialize("omarchy-ui", &dependency)
                        .expect("concurrent checkout")
                })
            })
            .collect();
        let packages: Vec<_> = workers
            .into_iter()
            .map(|worker| worker.join().expect("materialization worker"))
            .collect();

        assert_eq!(packages[0].root, packages[1].root);
        assert_eq!(
            std::fs::read_to_string(&packages[0].entry).unwrap(),
            "export const version = 1;"
        );
    }

    #[test]
    fn a_cached_mirror_with_the_wrong_origin_is_refused() {
        let fixture = GitFixture::new();
        fixture.commit("export const version = 1;", "first");
        let dependency = fixture.dependency(r#""branch": "main""#);
        let store = GitDependencyStore::new(fixture.cache.clone());
        store
            .materialize("omarchy-ui", &dependency)
            .expect("initial checkout");
        let mirror = std::fs::read_dir(fixture.cache.join("mirrors"))
            .expect("mirror directory")
            .next()
            .expect("one mirror")
            .expect("mirror entry")
            .path();
        git(&mirror, &["remote", "set-url", "origin", "/wrong/remote"]);

        let error = store
            .materialize("omarchy-ui", &dependency)
            .expect_err("a cache may not silently change repository identity");
        assert!(error.to_string().contains("cache origin"), "{error:#}");
    }

    #[test]
    fn a_cached_mirror_accepts_its_raw_origin_when_git_rewrites_the_effective_url() {
        let fixture = GitFixture::new();
        fixture.commit("export const version = 1;", "first");
        let raw_origin = "https://github.com/huacnlee/omarchy-ui";
        let dependency = fixture.dependency_at(raw_origin, r#""branch": "main""#);
        let store = GitDependencyStore::new(fixture.cache.clone());
        let remote_key = digest(&[("git", raw_origin)]);
        let mirror = fixture
            .cache
            .join("mirrors")
            .join(format!("{remote_key}.git"));
        std::fs::create_dir_all(mirror.parent().expect("mirror parent")).expect("mirror parent");
        git(
            mirror.parent().expect("mirror parent"),
            &[
                "clone",
                "--mirror",
                "--",
                fixture.remote.to_str().expect("UTF-8 fixture remote"),
                mirror.to_str().expect("UTF-8 mirror path"),
            ],
        );
        git(&mirror, &["remote", "set-url", "origin", raw_origin]);
        let rewrite_key = format!("url.{}.insteadOf", fixture.remote.display());
        git(&mirror, &["config", &rewrite_key, raw_origin]);

        let package = store
            .materialize("omarchy-ui", &dependency)
            .expect("a URL rewrite must not change the configured origin identity");
        assert_eq!(
            std::fs::read_to_string(package.entry).unwrap(),
            "export const version = 1;"
        );
    }

    /// A store, an application, and checkouts under the store's own cache —
    /// everything linking needs, and nothing Git does.
    struct LinkFixture {
        root: PathBuf,
        store: GitDependencyStore,
        app: PathBuf,
        cache: PathBuf,
    }

    impl LinkFixture {
        fn new() -> Self {
            let unique = NEXT.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "gpui-shell-dependency-link-{}-{unique}",
                std::process::id()
            ));
            let cache = root.join("cache");
            let app = root.join("app");
            std::fs::create_dir_all(&app).expect("application directory");
            std::fs::create_dir_all(&cache).expect("cache directory");
            Self {
                store: GitDependencyStore::new(cache.clone()),
                root,
                app,
                cache,
            }
        }

        /// A checkout of `name` at `commit`, holding one entry file.
        fn checkout(&self, name: &str, commit: &str) -> MaterializedDependency {
            let root = self.cache.join("checkouts").join(name).join(commit);
            std::fs::create_dir_all(&root).expect("checkout directory");
            let entry = root.join("index.js");
            std::fs::write(&entry, "export const marker = 1;\n").expect("checkout entry");
            MaterializedDependency { root, entry }
        }

        fn link(&self, name: &str) -> PathBuf {
            self.app.join(EDITOR_MODULE_DIRECTORY).join(name)
        }

        fn link_all(&self, dependencies: &[(&str, &MaterializedDependency)]) -> Vec<PathBuf> {
            let map = dependencies
                .iter()
                .map(|(name, dependency)| ((*name).to_owned(), (*dependency).clone()))
                .collect();
            self.store
                .link_for_editor(&self.app, &map)
                .expect("linking is writable")
        }
    }

    impl Drop for LinkFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn linking_points_an_editor_at_the_checkout_the_runtime_will_load() {
        let fixture = LinkFixture::new();
        let dependency = fixture.checkout("omarchy-ui", "aaaa");

        let written = fixture.link_all(&[("omarchy-ui", &dependency)]);

        let link = fixture.link("omarchy-ui");
        assert_eq!(written, vec![link.clone()]);
        assert_eq!(
            std::fs::canonicalize(link.join("index.js")).expect("the link resolves"),
            std::fs::canonicalize(&dependency.entry).expect("the entry exists")
        );
    }

    #[test]
    fn linking_rewrites_nothing_when_the_checkout_has_not_moved() {
        let fixture = LinkFixture::new();
        let dependency = fixture.checkout("omarchy-ui", "aaaa");

        assert_eq!(fixture.link_all(&[("omarchy-ui", &dependency)]).len(), 1);
        assert!(fixture.link_all(&[("omarchy-ui", &dependency)]).is_empty());
    }

    #[test]
    fn linking_repoints_when_the_selector_resolves_to_a_new_commit() {
        let fixture = LinkFixture::new();
        let before = fixture.checkout("omarchy-ui", "aaaa");
        let after = fixture.checkout("omarchy-ui", "bbbb");

        fixture.link_all(&[("omarchy-ui", &before)]);
        assert_eq!(fixture.link_all(&[("omarchy-ui", &after)]).len(), 1);

        assert_eq!(
            std::fs::read_link(fixture.link("omarchy-ui")).expect("a link"),
            after.root
        );
    }

    #[test]
    fn linking_removes_a_dependency_the_manifest_no_longer_declares() {
        let fixture = LinkFixture::new();
        let kept = fixture.checkout("omarchy-ui", "aaaa");
        let dropped = fixture.checkout("charts", "bbbb");

        fixture.link_all(&[("omarchy-ui", &kept), ("charts", &dropped)]);
        fixture.link_all(&[("omarchy-ui", &kept)]);

        assert!(std::fs::symlink_metadata(fixture.link("omarchy-ui")).is_ok());
        assert!(std::fs::symlink_metadata(fixture.link("charts")).is_err());
    }

    #[test]
    fn linking_leaves_an_installed_package_of_the_same_name_alone() {
        let fixture = LinkFixture::new();
        let dependency = fixture.checkout("omarchy-ui", "aaaa");
        let installed = fixture.link("omarchy-ui");
        std::fs::create_dir_all(&installed).expect("an installed package");
        std::fs::write(installed.join("package.json"), "{}").expect("its manifest");

        assert!(fixture.link_all(&[("omarchy-ui", &dependency)]).is_empty());

        assert!(
            std::fs::symlink_metadata(&installed)
                .expect("the installed package survives")
                .is_dir()
        );
        assert!(!installed.join(EDITOR_LINK_MARKER).exists());
    }

    #[test]
    fn a_scoped_dependency_name_becomes_a_nested_link() {
        let fixture = LinkFixture::new();
        let dependency = fixture.checkout("ui", "aaaa");

        fixture.link_all(&[("@omarchy/ui", &dependency)]);

        assert_eq!(
            std::fs::read_link(fixture.link("@omarchy/ui")).expect("a nested link"),
            dependency.root
        );
    }
}

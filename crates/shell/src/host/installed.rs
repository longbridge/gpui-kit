//! Where an application comes from when the command line does not give a path.
//!
//! Two places, both single-application:
//!
//! - **Installed**: `gpui-shell run clock` runs `~/.gpui-shell/apps/clock`. A
//!   dot directory in the home directory rather than a platform configuration
//!   directory, so `curl`, Homebrew and mise installs, and every platform,
//!   agree on one path a user can type.
//! - **Bundled**: with no argument at all, the application packaged beside the
//!   executable. A macOS `.app` or a Linux package is then this binary plus a
//!   directory of scripts, and needs no launcher script to pass a path —
//!   which an `Info.plist` or a desktop file could not do portably anyway.

use std::path::{Path, PathBuf};

use super::{ENTRY, HostBrand};
use crate::plugin::{MANIFEST_FILE, PluginManifest};

/// The directory installed applications live in, one directory each.
///
/// Named after the command, so an embedding application's own branded command
/// keeps applications written against its catalog apart from ones `gpui-shell`
/// cannot run. Its siblings, such as the dependency cache, are not
/// applications.
///
/// `GPUI_SHELL_HOME`, when set, replaces the whole `~/.<command>` directory —
/// the dependency cache honours it too — so a portable or system-wide install
/// can keep everything somewhere else.
pub(super) fn applications_directory(brand: HostBrand) -> PathBuf {
    shell_home(brand, |variable| std::env::var_os(variable)).join("apps")
}

fn shell_home(
    brand: HostBrand,
    environment: impl Fn(&str) -> Option<std::ffi::OsString>,
) -> PathBuf {
    match environment(crate::dependencies::SHELL_HOME_VARIABLE).filter(|value| !value.is_empty()) {
        Some(directory) => std::path::absolute(&directory).unwrap_or_else(|_| directory.into()),
        None => home().join(format!(".{}", brand.name())),
    }
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

/// Whether an argument names an installed application rather than a path.
///
/// A name is one plain path segment: no separator, and not `.` or `..`, which
/// both mean a directory relative to where the command runs.
pub(super) fn is_application_name(argument: &str) -> bool {
    !argument.is_empty()
        && argument != "."
        && argument != ".."
        && !argument.contains(['/', '\\'])
        && !argument.starts_with('~')
}

/// The directory of an installed application.
pub(super) fn resolve(name: &str, brand: HostBrand) -> Result<PathBuf, String> {
    let directory = applications_directory(brand).join(name);
    if is_application(&directory) {
        return Ok(directory);
    }
    Err(format!(
        "no application `{name}` is installed: {} has no {ENTRY} or {MANIFEST_FILE}. \
         `{cmd} list` shows what is installed; to run a directory, use `{cmd} run -d {name}`.",
        directory.display(),
        cmd = brand.name()
    ))
}

fn is_application(directory: &Path) -> bool {
    directory.join(MANIFEST_FILE).is_file() || directory.join(ENTRY).is_file()
}

/// Where a packaged application is looked for, relative to the executable, in
/// the words the help prints.
pub(super) const BUNDLED_LOCATIONS: &str =
    "`app` beside the executable, or `../Resources/app` inside a macOS bundle";

/// The application packaged beside the running executable, if there is one.
pub(super) fn bundled() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let executable = executable.canonicalize().unwrap_or(executable);
    bundled_beside(executable.parent()?)
}

fn bundled_beside(executable_directory: &Path) -> Option<PathBuf> {
    [
        // `Foo.app/Contents/MacOS/gpui-shell` with `Foo.app/Contents/Resources/app`.
        executable_directory
            .join("..")
            .join("Resources")
            .join("app"),
        // A Linux or Windows package, or an unpacked archive.
        executable_directory.join("app"),
    ]
    .into_iter()
    .find(|candidate| is_application(candidate))
}

/// `list`: every installed application and what it asks for, executing none.
pub(super) fn list(brand: HostBrand) -> ! {
    let directory = applications_directory(brand);
    let mut names: Vec<PathBuf> = std::fs::read_dir(&directory)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| is_application(path))
        .collect();
    names.sort();

    if names.is_empty() {
        eprintln!(
            "no application is installed in {}. Each application is a directory there \
             holding {ENTRY} and, usually, {MANIFEST_FILE}.",
            directory.display()
        );
        std::process::exit(1);
    }

    for root in names {
        let name = root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !root.join(MANIFEST_FILE).is_file() {
            println!("{name}\n    no {MANIFEST_FILE}: local defaults (storage only)");
            continue;
        }
        match PluginManifest::read(&root) {
            Ok(manifest) => {
                println!(
                    "{name}  {} {} ({})",
                    manifest.name(),
                    manifest.version(),
                    manifest.id()
                );
                let data = crate::runtime::app_data_dir(manifest.id()).unwrap_or_default();
                // What `run` will actually grant: the manifest's request plus the
                // application's own directory and storage.
                let grants = super::local_capabilities(&root, &data, Some(&manifest)).summary();
                if grants.is_empty() {
                    println!("    no capabilities");
                }
                for grant in grants {
                    println!("    {grant}");
                }
            }
            Err(error) => println!("{name}\n    invalid: {error}"),
        }
    }
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_name_is_installed_and_anything_path_like_is_a_path() {
        assert!(is_application_name("clock"));
        assert!(is_application_name("my-app.v2"));
        for path in [
            ".",
            "..",
            "./clock",
            "examples/js_todolist",
            "/abs",
            "~/clock",
            "a\\b",
            "",
        ] {
            assert!(!is_application_name(path), "`{path}` reads as a path");
        }
    }

    #[test]
    fn installed_applications_live_under_the_command_name() {
        let directory = applications_directory(HostBrand::new("gpui-shell", "0.0.0"));
        assert!(directory.ends_with(Path::new(".gpui-shell").join("apps")));

        let branded = applications_directory(HostBrand::new("market-shell", "0.0.0"));
        assert!(branded.ends_with(Path::new(".market-shell").join("apps")));
    }

    #[test]
    fn the_shell_home_variable_replaces_the_dot_directory() {
        let brand = HostBrand::new("gpui-shell", "0.0.0");
        let moved = shell_home(brand, |variable| {
            (variable == crate::dependencies::SHELL_HOME_VARIABLE).then(|| "/opt/shell".into())
        });
        assert_eq!(moved, PathBuf::from("/opt/shell"));

        let unset = shell_home(brand, |_| None);
        assert!(unset.ends_with(".gpui-shell"));
    }

    #[test]
    fn a_bundled_application_is_found_in_a_macos_bundle_or_beside_the_binary() {
        let root = std::env::temp_dir().join(format!("gpui-shell-bundle-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let executables = root.join("Contents").join("MacOS");
        std::fs::create_dir_all(&executables).expect("bundle");
        assert_eq!(
            bundled_beside(&executables),
            None,
            "nothing is packaged yet"
        );

        let resources = root.join("Contents").join("Resources").join("app");
        std::fs::create_dir_all(&resources).expect("resources");
        std::fs::write(resources.join(ENTRY), "").expect("entry");
        assert_eq!(
            bundled_beside(&executables),
            Some(executables.join("..").join("Resources").join("app"))
        );

        let beside = executables.join("app");
        std::fs::create_dir_all(&beside).expect("app beside");
        std::fs::write(beside.join(ENTRY), "").expect("entry");
        assert!(bundled_beside(&executables).is_some());

        let _ = std::fs::remove_dir_all(&root);
    }
}

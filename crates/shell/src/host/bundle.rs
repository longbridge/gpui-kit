//! `bundle`: an application as a desktop application.
//!
//! On macOS and Linux a bundle holds the application's directory and the
//! scripts that start it, not `gpui-shell`: the first launch — or, on Linux,
//! the installer — uses a `gpui-shell` already on the computer, or downloads
//! the release this command belongs to. Any platform's bundle is made on any
//! platform. One platform per invocation:
//!
//! - **macOS**: `Name.app`, unsigned, whose executable is a launcher script.
//!   Signing and notarization need the author's identity, so they stay the
//!   author's step.
//! - **Linux**: a `.tar.gz` holding `app/` and the launcher, which runs where
//!   it is unpacked, and beside it an `install.sh` that installs the archive
//!   for the current user with a desktop entry — from the file beside it, or
//!   downloaded, so `curl … | sh` works.
//! - **Windows**: a portable `.zip`: `Name.exe`, which is the release's
//!   `gpui-shell.exe` marked as a GUI program, with `app\` beside it.
//!
//! The scripts run the application with `run --dir <app> --packaged`, and the
//! Windows executable finds `app\` beside itself, so either way it keeps its
//! data under its `id` wherever it is.

use std::{
    collections::BTreeSet,
    fs,
    io::{self, Write as _},
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result, bail};
use image::{ImageFormat, RgbaImage, imageops::FilterType};

use super::{ENTRY, HostBrand, agents::AGENTS_FILE};
use crate::{
    plugin::{MANIFEST_FILE, PluginManifest},
    typings,
};

/// The functions every POSIX launcher shares to find or download the runtime.
const RUNTIME_FUNCTIONS: &str = include_str!("bundle/runtime.sh");
const LINUX_LAUNCHER: &str = include_str!("bundle/linux-launcher.sh");
const LINUX_INSTALLER: &str = include_str!("bundle/linux-install.sh");
const MACOS_LAUNCHER: &str = include_str!("bundle/macos-launcher.sh");

/// The Windows release `gpui-shell.exe` is copied from.
const WINDOWS_TARGET: &str = "x86_64-pc-windows-msvc";

/// The operating system a bundle is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Platform {
    MacOS,
    Linux,
    Windows,
}

impl Platform {
    /// The platform this command runs on, the default.
    fn host() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOS
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Linux
        }
    }

    pub(super) fn parse(value: &str) -> Option<Self> {
        match value {
            "macos" => Some(Self::MacOS),
            "linux" => Some(Self::Linux),
            "windows" => Some(Self::Windows),
            _ => None,
        }
    }

    /// The `--platform` spelling, also used in archive names.
    fn as_str(self) -> &'static str {
        match self {
            Self::MacOS => "macos",
            Self::Linux => "linux",
            Self::Windows => "windows",
        }
    }
}

/// What `bundle` was asked for, beyond the application.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct BundleOptions {
    pub(super) platform: Option<Platform>,
    pub(super) icon: Option<PathBuf>,
    pub(super) out: Option<PathBuf>,
    /// Where the Linux archive will be published, so `install.sh` can be run
    /// on its own (`curl …/install.sh | sh`) and download it.
    pub(super) download_url: Option<String>,
}

/// Bundles the application at `directory`, returning the lines to print.
pub(super) fn create(
    directory: &Path,
    options: &BundleOptions,
    brand: HostBrand,
) -> Result<String> {
    // The scripts download `gpui-shell` releases. An embedding command's
    // catalog is not in them, so its applications would start without it.
    if brand.name() != "gpui-shell" {
        bail!(
            "`bundle` installs gpui-shell releases on the user's computer, which do not \
             carry {}'s components",
            brand.name()
        );
    }
    let root = crate::resolve_app_root(directory, ENTRY)?;
    if !root.join(MANIFEST_FILE).is_file() {
        bail!(
            "{} has no {MANIFEST_FILE}. A bundle needs one: its `id` keeps the user's data \
             when the application is moved or updated, and its `name` and `version` name the bundle.",
            root.display()
        );
    }
    let manifest = PluginManifest::read(&root)?;

    let platform = options.platform.unwrap_or_else(Platform::host);
    if options.download_url.is_some() && platform != Platform::Linux {
        bail!("`--download-url` is for the Linux bundle, whose install.sh downloads the archive");
    }
    let icon = options.icon.as_deref().map(read_icon).transpose()?;
    let out = match &options.out {
        Some(out) => out.clone(),
        None => std::env::current_dir()?.join("dist"),
    };
    fs::create_dir_all(&out).with_context(|| format!("cannot create {}", out.display()))?;
    // The output directory is often inside the application (`dist/` beside
    // `main.js`), and a bundle must not contain the previous bundle.
    let exclude = out.canonicalize()?;

    let names = Names::new(&manifest, platform);
    let mut values = ScriptValues {
        id: manifest.id(),
        name: manifest.name(),
        command: &names.command,
        app_version: manifest.version(),
        runtime_version: brand.version(),
        icon_size: String::new(),
        archive: String::new(),
        archive_sha256: String::new(),
        download_url: options
            .download_url
            .as_deref()
            .unwrap_or_default()
            .trim_end_matches('/'),
        stem: &names.archive_stem,
    };
    let runtime = format!(
        "It uses gpui-shell {} or later when one is installed, and downloads v{} otherwise.",
        brand.version(),
        brand.version()
    );
    let mut tree = Tree::default();
    let report = match platform {
        Platform::MacOS => {
            let contents = format!("{}/Contents", names.app_bundle);
            tree.bytes(
                format!("{contents}/Info.plist"),
                info_plist(&manifest, &names, icon.is_some()).into_bytes(),
                false,
            );
            tree.bytes(format!("{contents}/PkgInfo"), b"APPL????".to_vec(), false);
            tree.bytes(
                format!("{contents}/MacOS/{}", names.display),
                shell_script(MACOS_LAUNCHER, &values).into_bytes(),
                true,
            );
            if let Some(icon) = &icon {
                tree.bytes(
                    format!("{contents}/Resources/{}.icns", names.display),
                    icns(icon)?,
                    false,
                );
            }
            add_application(
                &mut tree,
                &root,
                &format!("{contents}/Resources/app"),
                &exclude,
            )?;
            let bundle = out.join(&names.app_bundle);
            replace_directory(&tree, &out, &bundle)?;
            format!(
                "bundled {}\n{runtime} The first launch downloads it when needed.\n\
                 The bundle is not signed. Sign and notarize it before giving it to anyone; \
                 see https://gpui-kit.com/shell/packaging#signing-on-macos",
                bundle.display()
            )
        }
        Platform::Linux => {
            let top = &names.archive_stem;
            if let Some(icon) = &icon {
                let size = [512, 256, 128, 64, 48, 32, 16]
                    .into_iter()
                    .find(|size| *size <= icon.width())
                    .unwrap_or(icon.width());
                tree.bytes(
                    format!("{top}/{}.png", names.command),
                    png(icon, size)?,
                    false,
                );
                values.icon_size = size.to_string();
            }
            tree.bytes(
                format!("{top}/{}", names.command),
                shell_script(LINUX_LAUNCHER, &values).into_bytes(),
                true,
            );
            add_application(&mut tree, &root, &format!("{top}/app"), &exclude)?;
            let archive = out.join(format!("{top}.tar.gz"));
            replace_file(&archive, |file| write_tar_gz(&tree, file))?;

            // Outside the archive, and holding its checksum: it installs the
            // archive beside it, or downloads it, and trusts neither unchecked.
            values.archive = format!("{top}.tar.gz");
            values.archive_sha256 = sha256_hex(
                &fs::read(&archive)
                    .with_context(|| format!("cannot read {}", archive.display()))?,
            );
            let installer = out.join("install.sh");
            let script = shell_script(LINUX_INSTALLER, &values);
            replace_file(&installer, |mut file| file.write_all(script.as_bytes()))?;
            set_executable(&installer)?;
            let remote = match options.download_url.as_deref() {
                Some(url) => format!(
                    "\nPublish both at {url}; `curl -fsSL {}/install.sh | sh` then installs it.",
                    url.trim_end_matches('/')
                ),
                None => String::new(),
            };
            format!(
                "bundled {}\nbundled {}\n\
                 Unpack the archive and run the launcher in it, or run `sh install.sh` beside the \
                 archive to install it for the current user; `sh install.sh --uninstall` removes \
                 it.{remote}\n{runtime}",
                archive.display(),
                installer.display()
            )
        }
        Platform::Windows => {
            // The one bundle with a runtime in it: a portable folder has no
            // installer to fetch one.
            let mut executable = windows_runtime(brand, download_verified)?;
            mark_as_gui_program(&mut executable)?;
            let top = format!("{}-x86_64", names.archive_stem);
            tree.bytes(format!("{top}/{}.exe", names.display), executable, true);
            add_application(&mut tree, &root, &format!("{top}/app"), &exclude)?;
            let archive = out.join(format!("{top}.zip"));
            replace_file(&archive, |file| write_zip(&tree, file))?;
            let mut report = format!(
                "bundled {}\nUnzip it anywhere and open {}.exe; it needs no installation. It \
                 carries gpui-shell {}.",
                archive.display(),
                names.display,
                brand.version()
            );
            if icon.is_some() {
                report.push_str("\nThe icon is not embedded: the executable keeps gpui-shell's.");
            }
            report
        }
    };
    Ok(report)
}

// ---------------------------------------------------------------------------
// Names
// ---------------------------------------------------------------------------

/// The names one bundle uses, all derived from the manifest.
struct Names {
    /// The manifest's `name`, safe as a file name: `Folio Desk`.
    display: String,
    /// A command name: `folio-desk`.
    command: String,
    /// `Folio Desk.app`.
    app_bundle: String,
    /// `folio-desk-0.1.0-linux`, the archive's name and top directory. No
    /// architecture: the bundle holds no binary.
    archive_stem: String,
}

impl Names {
    fn new(manifest: &PluginManifest, platform: Platform) -> Self {
        let command = command_name(manifest);
        let display = file_name(manifest.name()).unwrap_or_else(|| command.clone());
        Self {
            app_bundle: format!("{display}.app"),
            archive_stem: format!("{command}-{}-{}", manifest.version(), platform.as_str()),
            display,
            command,
        }
    }
}

/// The manifest's `name` as a command: lowercase ASCII words joined by `-`.
/// A name with no ASCII letters or digits falls back to the last part of the
/// `id`, which the manifest already restricts to that alphabet.
fn command_name(manifest: &PluginManifest) -> String {
    let slug = |value: &str| {
        value
            .split(|character: char| !character.is_ascii_alphanumeric())
            .filter(|word| !word.is_empty())
            .map(str::to_ascii_lowercase)
            .collect::<Vec<_>>()
            .join("-")
    };
    [
        slug(manifest.name()),
        slug(manifest.id().rsplit('.').next().unwrap_or_default()),
    ]
    .into_iter()
    .find(|name| !name.is_empty())
    .unwrap_or_else(|| "app".to_owned())
}

/// `name` without the characters a file name cannot hold on any of the three
/// platforms, or `None` when nothing is left.
fn file_name(name: &str) -> Option<String> {
    let cleaned: String = name
        .chars()
        .map(|character| match character {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            character if character.is_control() => '-',
            character => character,
        })
        .collect();
    let cleaned = cleaned.trim().trim_end_matches('.').trim();
    (!cleaned.is_empty()).then(|| cleaned.to_owned())
}

// ---------------------------------------------------------------------------
// Scripts
// ---------------------------------------------------------------------------

/// What a script template's `@NAME@` placeholders stand for.
struct ScriptValues<'a> {
    id: &'a str,
    name: &'a str,
    command: &'a str,
    app_version: &'a str,
    /// The release the scripts download, and the oldest one they accept.
    runtime_version: &'a str,
    /// The Linux icon's size in pixels, empty without an icon.
    icon_size: String,
    /// The Linux archive's file name and SHA-256, for `install.sh`.
    archive: String,
    archive_sha256: String,
    /// Where the Linux archive is published, empty when nowhere.
    download_url: &'a str,
    /// The archive's top directory.
    stem: &'a str,
}

impl ScriptValues<'_> {
    /// The raw value of one placeholder, or `None` for one this does not know.
    fn get(&self, placeholder: &str) -> Option<&str> {
        Some(match placeholder {
            "ID" => self.id,
            "NAME" => self.name,
            "COMMAND" => self.command,
            "APP_VERSION" => self.app_version,
            "RUNTIME_VERSION" => self.runtime_version,
            "ICON_SIZE" => &self.icon_size,
            "ARCHIVE" => &self.archive,
            "ARCHIVE_SHA256" => &self.archive_sha256,
            "DOWNLOAD_URL" => self.download_url,
            "STEM" => self.stem,
            _ => return None,
        })
    }
}

/// A POSIX shell template with every value as one single-quoted word. A line
/// holding only `@RUNTIME_FUNCTIONS@` becomes the shared runtime functions.
fn shell_script(template: &str, values: &ScriptValues) -> String {
    substitute(template, |placeholder| match placeholder {
        "RUNTIME_FUNCTIONS" => Some(shell_script(RUNTIME_FUNCTIONS, values)),
        placeholder => values.get(placeholder).map(shell_quote),
    })
}

/// Replaces each `@NAME@` — uppercase letters, digits and underscores — that
/// `replacement` knows, in one pass, so a value containing `@ID@` is never
/// expanded again. Anything else between `@`s, such as an address in a
/// comment, is left as written.
fn substitute(template: &str, replacement: impl Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find('@') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let name_length = after
            .find(|character: char| {
                !(character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_')
            })
            .unwrap_or(after.len());
        let name = &after[..name_length];
        match after[name_length..]
            .starts_with('@')
            .then(|| replacement(name))
            .flatten()
        {
            Some(value) => {
                out.push_str(&value);
                rest = &after[name_length + 1..];
            }
            None => {
                out.push('@');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// One POSIX shell word whose value is `value`, whatever it contains.
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

fn info_plist(manifest: &PluginManifest, names: &Names, has_icon: bool) -> String {
    // `CFBundleVersion` takes up to three integers, so a pre-release or build
    // suffix stays in the short version only. The manifest checked it is semver.
    let build = semver::Version::parse(manifest.version())
        .map(|version| format!("{}.{}.{}", version.major, version.minor, version.patch))
        .unwrap_or_else(|_| manifest.version().to_owned());
    let mut entries = vec![
        ("CFBundleDevelopmentRegion", "en".to_owned()),
        ("CFBundleDisplayName", manifest.name().to_owned()),
        ("CFBundleExecutable", names.display.clone()),
        ("CFBundleIdentifier", manifest.id().to_owned()),
        ("CFBundleInfoDictionaryVersion", "6.0".to_owned()),
        ("CFBundleName", manifest.name().to_owned()),
        ("CFBundlePackageType", "APPL".to_owned()),
        ("CFBundleShortVersionString", manifest.version().to_owned()),
        ("CFBundleVersion", build),
        ("LSMinimumSystemVersion", "11.0".to_owned()),
    ];
    if has_icon {
        entries.push(("CFBundleIconFile", names.display.clone()));
    }
    entries.sort_by_key(|(key, _)| *key);

    let mut plist = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
         \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n<dict>\n",
    );
    for (key, value) in entries {
        plist.push_str(&format!(
            "  <key>{key}</key>\n  <string>{}</string>\n",
            xml_escape(&value)
        ));
    }
    plist.push_str("  <key>NSHighResolutionCapable</key>\n  <true/>\n</dict>\n</plist>\n");
    plist
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ---------------------------------------------------------------------------
// Icons
// ---------------------------------------------------------------------------

fn read_icon(path: &Path) -> Result<RgbaImage> {
    let bytes = fs::read(path).with_context(|| format!("cannot read {}", path.display()))?;
    let icon = image::load_from_memory_with_format(&bytes, ImageFormat::Png)
        .with_context(|| format!("{} is not a PNG image", path.display()))?
        .into_rgba8();
    if icon.width() != icon.height() || icon.width() < 16 {
        bail!(
            "{} is {}×{}; the icon must be a square PNG, ideally 1024×1024",
            path.display(),
            icon.width(),
            icon.height()
        );
    }
    Ok(icon)
}

/// `icon` scaled to `size`, as PNG bytes.
fn png(icon: &RgbaImage, size: u32) -> Result<Vec<u8>> {
    let scaled;
    let image = if icon.width() == size {
        icon
    } else {
        scaled = image::imageops::resize(icon, size, size, FilterType::Lanczos3);
        &scaled
    };
    let mut bytes = io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, ImageFormat::Png)?;
    Ok(bytes.into_inner())
}

/// An `.icns` file holding every standard size up to the source's, each as PNG.
fn icns(icon: &RgbaImage) -> Result<Vec<u8>> {
    const SIZES: [(u32, &[u8; 4]); 7] = [
        (16, b"icp4"),
        (32, b"icp5"),
        (64, b"icp6"),
        (128, b"ic07"),
        (256, b"ic08"),
        (512, b"ic09"),
        (1024, b"ic10"),
    ];
    let mut body = Vec::new();
    for (size, kind) in SIZES.iter().filter(|(size, _)| *size <= icon.width()) {
        let image = png(icon, *size)?;
        body.extend_from_slice(*kind);
        body.extend_from_slice(&(image.len() as u32 + 8).to_be_bytes());
        body.extend_from_slice(&image);
    }
    let mut file = b"icns".to_vec();
    file.extend_from_slice(&(body.len() as u32 + 8).to_be_bytes());
    file.extend_from_slice(&body);
    Ok(file)
}

// ---------------------------------------------------------------------------
// The tree
// ---------------------------------------------------------------------------

/// The files of one bundle, by their `/`-separated path inside it, written
/// out as a directory or an archive.
#[derive(Default)]
struct Tree {
    entries: Vec<Entry>,
    directories: BTreeSet<String>,
}

struct Entry {
    path: String,
    contents: Contents,
    executable: bool,
}

enum Contents {
    Directory,
    File(PathBuf),
    Bytes(Vec<u8>),
}

impl Tree {
    fn directory(&mut self, path: &str) {
        if path.is_empty() || self.directories.contains(path) {
            return;
        }
        if let Some((parent, _)) = path.rsplit_once('/') {
            self.directory(parent);
        }
        self.directories.insert(path.to_owned());
        self.entries.push(Entry {
            path: path.to_owned(),
            contents: Contents::Directory,
            executable: true,
        });
    }

    fn add(&mut self, path: String, contents: Contents, executable: bool) {
        if let Some((parent, _)) = path.rsplit_once('/') {
            self.directory(parent);
        }
        self.entries.push(Entry {
            path,
            contents,
            executable,
        });
    }

    fn bytes(&mut self, path: String, bytes: Vec<u8>, executable: bool) {
        self.add(path, Contents::Bytes(bytes), executable);
    }
}

impl Entry {
    fn read(&self) -> io::Result<Vec<u8>> {
        match &self.contents {
            Contents::Directory => Ok(Vec::new()),
            Contents::File(path) => fs::read(path),
            Contents::Bytes(bytes) => Ok(bytes.clone()),
        }
    }

    fn mode(&self) -> u32 {
        if self.executable { 0o755 } else { 0o644 }
    }
}

/// Adds the application's files under `prefix`, leaving out what only
/// development needs: hidden entries, `node_modules`, and the editor files
/// `gpui-shell` writes (`gpui-kit.d.ts`, `jsconfig.json`, `AGENTS.md`). A
/// bundled application does not write them back; see `set_editor_files`.
fn add_application(tree: &mut Tree, root: &Path, prefix: &str, exclude: &Path) -> Result<()> {
    tree.directory(prefix);
    let mut visited = BTreeSet::new();
    add_directory(tree, root, prefix, exclude, true, &mut visited)
}

fn add_directory(
    tree: &mut Tree,
    directory: &Path,
    prefix: &str,
    exclude: &Path,
    is_root: bool,
    visited: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    // A symbolic link back up the tree would otherwise copy forever.
    if !visited.insert(directory.canonicalize()?) {
        return Ok(());
    }
    let mut entries = fs::read_dir(directory)
        .with_context(|| format!("cannot read {}", directory.display()))?
        .collect::<io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            bail!(
                "{} is not a UTF-8 file name, which an archive cannot hold portably",
                entry.path().display()
            );
        };
        let left_out = name.starts_with('.')
            || name == "node_modules"
            || name == typings::FILE_NAME
            || (is_root && (name == typings::CONFIG_FILE_NAME || name == AGENTS_FILE));
        let path = entry.path();
        if left_out || path.canonicalize().is_ok_and(|path| path == exclude) {
            continue;
        }
        // Follows symbolic links: the bundle holds what they point at.
        let metadata =
            fs::metadata(&path).with_context(|| format!("cannot read {}", path.display()))?;
        let inner = format!("{prefix}/{name}");
        if metadata.is_dir() {
            tree.directory(&inner);
            add_directory(tree, &path, &inner, exclude, false, visited)?;
        } else if metadata.is_file() {
            tree.add(inner, Contents::File(path), is_executable(&metadata));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn is_executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_: &fs::Metadata) -> bool {
    false
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

/// Writes `tree` into `out` and moves its single top-level directory to
/// `destination`, replacing an earlier bundle there and nothing else.
fn replace_directory(tree: &Tree, out: &Path, destination: &Path) -> Result<()> {
    if destination.exists() && !destination.join("Contents").join("Info.plist").is_file() {
        bail!(
            "{} exists and is not an application bundle; move it away first",
            destination.display()
        );
    }
    let staging = out.join(format!(".bundle-{}", std::process::id()));
    let _ = fs::remove_dir_all(&staging);
    let written = write_directory(tree, &staging).and_then(|()| {
        let top = fs::read_dir(&staging)?
            .next()
            .ok_or_else(|| io::Error::other("the bundle is empty"))??
            .path();
        if destination.exists() {
            fs::remove_dir_all(destination)?;
        }
        fs::rename(top, destination)
    });
    let _ = fs::remove_dir_all(&staging);
    written.with_context(|| format!("cannot write {}", destination.display()))
}

/// Writes an archive beside `destination` and renames it into place, so an
/// interrupted run never leaves half an archive under the final name.
fn replace_file(destination: &Path, write: impl FnOnce(fs::File) -> io::Result<()>) -> Result<()> {
    let mut partial = destination.as_os_str().to_owned();
    partial.push(".partial");
    let partial = PathBuf::from(partial);
    let written = fs::File::create(&partial)
        .and_then(write)
        .and_then(|()| fs::rename(&partial, destination));
    if written.is_err() {
        let _ = fs::remove_file(&partial);
    }
    written.with_context(|| format!("cannot write {}", destination.display()))
}

fn write_directory(tree: &Tree, destination: &Path) -> io::Result<()> {
    for entry in &tree.entries {
        let path = destination.join(&entry.path);
        match &entry.contents {
            Contents::Directory => fs::create_dir_all(&path)?,
            Contents::File(source) => {
                fs::copy(source, &path)?;
            }
            Contents::Bytes(bytes) => fs::write(&path, bytes)?,
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            fs::set_permissions(&path, fs::Permissions::from_mode(entry.mode()))?;
        }
    }
    Ok(())
}

fn write_tar_gz(tree: &Tree, out: impl io::Write) -> io::Result<()> {
    let encoder =
        flate2::write::GzEncoder::new(io::BufWriter::new(out), flate2::Compression::default());
    let mut archive = tar::Builder::new(encoder);
    let modified = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |time| time.as_secs());
    for entry in &tree.entries {
        let mut header = tar::Header::new_gnu();
        header.set_mode(entry.mode());
        header.set_mtime(modified);
        match &entry.contents {
            Contents::Directory => {
                header.set_entry_type(tar::EntryType::Directory);
                header.set_size(0);
                archive.append_data(&mut header, format!("{}/", entry.path), io::empty())?;
            }
            _ => {
                let bytes = entry.read()?;
                header.set_entry_type(tar::EntryType::Regular);
                header.set_size(bytes.len() as u64);
                archive.append_data(&mut header, &entry.path, bytes.as_slice())?;
            }
        }
    }
    archive.into_inner()?.finish()?.flush()
}

// ---------------------------------------------------------------------------
// Downloads
// ---------------------------------------------------------------------------

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Downloads `url` and checks it against the `.sha256` the release carries
/// beside it, in either the `<hex>  <name>` or the `<hex> *<name>` form.
#[cfg(feature = "quickjs")]
fn download_verified(url: &str) -> Result<Vec<u8>> {
    let get = |url: &str| -> Result<Vec<u8>> {
        let response = reqwest::blocking::get(url)?.error_for_status()?;
        Ok(response.bytes()?.to_vec())
    };
    let bytes = get(url)?;
    let checksum = String::from_utf8(get(&format!("{url}.sha256"))?)?;
    let Some(expected) = checksum.split_whitespace().next() else {
        bail!("{url}.sha256 is empty");
    };
    if !expected.eq_ignore_ascii_case(&sha256_hex(&bytes)) {
        bail!("{url} does not match its checksum");
    }
    Ok(bytes)
}

#[cfg(not(feature = "quickjs"))]
fn download_verified(_: &str) -> Result<Vec<u8>> {
    bail!("this build of gpui-shell cannot download; it was built without the `quickjs` feature")
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .with_context(|| format!("cannot make {} executable", path.display()))
}

#[cfg(not(unix))]
fn set_executable(_: &Path) -> Result<()> {
    Ok(())
}

// ---------------------------------------------------------------------------
// The Windows executable
// ---------------------------------------------------------------------------

/// The `gpui-shell.exe` a Windows bundle carries, of this command's version.
///
/// On Windows that is the running executable. Elsewhere it is the release's,
/// downloaded once into the cache beside the dependency cache — the shell home,
/// which `GPUI_SHELL_HOME` moves.
fn windows_runtime(
    brand: HostBrand,
    download: impl FnOnce(&str) -> Result<Vec<u8>>,
) -> Result<Vec<u8>> {
    if cfg!(target_os = "windows") {
        let executable = std::env::current_exe().context("cannot locate the running executable")?;
        return fs::read(&executable)
            .with_context(|| format!("cannot read {}", executable.display()));
    }

    let version = brand.version();
    let applications = super::installed::applications_directory(brand);
    let cache = applications
        .parent()
        .unwrap_or(&applications)
        .join("cache")
        .join("runtime")
        .join(format!("v{version}"))
        .join(WINDOWS_TARGET)
        .join("gpui-shell.exe");
    windows_runtime_from(&cache, version, download)
}

/// The cached copy at `cache`, else the release `v<version>` unpacked into it.
fn windows_runtime_from(
    cache: &Path,
    version: &str,
    download: impl FnOnce(&str) -> Result<Vec<u8>>,
) -> Result<Vec<u8>> {
    if cache.is_file() {
        let executable =
            fs::read(cache).with_context(|| format!("cannot read {}", cache.display()))?;
        pe_header(&executable)
            .ok_or_else(|| anyhow::anyhow!("{} is not a Windows executable", cache.display()))?;
        return Ok(executable);
    }

    let url = format!(
        "https://github.com/longbridge/gpui-kit/releases/download/v{version}/gpui-shell-{WINDOWS_TARGET}.zip"
    );
    let archive = download(&url).with_context(|| {
        format!(
            "cannot download gpui-shell {version} for Windows from {url}; a Windows bundle \
             carries it. Offline, put that release's gpui-shell.exe at {}",
            cache.display()
        )
    })?;
    let executable = zip_entry(
        &archive,
        &format!("gpui-shell-{WINDOWS_TARGET}/gpui-shell.exe"),
    )
    .with_context(|| url.clone())?;
    pe_header(&executable).ok_or_else(|| anyhow::anyhow!("{url} holds no Windows executable"))?;
    if let Some(directory) = cache.parent() {
        fs::create_dir_all(directory)
            .with_context(|| format!("cannot create {}", directory.display()))?;
    }
    replace_file(cache, |mut file| file.write_all(&executable))?;
    Ok(executable)
}

/// The contents of the file `name` in a zip archive, stored or deflated.
fn zip_entry(archive: &[u8], name: &str) -> Result<Vec<u8>> {
    use std::io::Read as _;

    let u16_at = |at: usize| -> Result<usize> {
        let bytes = archive
            .get(at..at + 2)
            .ok_or_else(|| anyhow::anyhow!("the zip archive is truncated"))?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]) as usize)
    };
    let u32_at = |at: usize| -> Result<usize> {
        let bytes = archive
            .get(at..at + 4)
            .ok_or_else(|| anyhow::anyhow!("the zip archive is truncated"))?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize)
    };

    // The end record is the last thing in the file, before a comment of up
    // to 64 KiB.
    let end = (0..archive.len().saturating_sub(21))
        .rev()
        .take(65_557)
        .find(|at| archive[*at..].starts_with(&0x0605_4b50u32.to_le_bytes()))
        .ok_or_else(|| anyhow::anyhow!("not a zip archive"))?;
    let count = u16_at(end + 10)?;
    let mut at = u32_at(end + 16)?;
    for _ in 0..count {
        if u32_at(at)? != 0x0201_4b50 {
            bail!("the zip archive's directory is damaged");
        }
        let method = u16_at(at + 10)?;
        let compressed = u32_at(at + 20)?;
        let size = u32_at(at + 24)?;
        let name_length = u16_at(at + 28)?;
        let skip = name_length + u16_at(at + 30)? + u16_at(at + 32)?;
        let local = u32_at(at + 42)?;
        let entry = archive
            .get(at + 46..at + 46 + name_length)
            .ok_or_else(|| anyhow::anyhow!("the zip archive is truncated"))?;
        if entry == name.as_bytes() {
            if u32_at(local)? != 0x0403_4b50 {
                bail!("the zip archive's entry for {name} is damaged");
            }
            let data = local + 30 + u16_at(local + 26)? + u16_at(local + 28)?;
            let stored = archive
                .get(data..data + compressed)
                .ok_or_else(|| anyhow::anyhow!("the zip archive is truncated"))?;
            let mut contents = Vec::with_capacity(size);
            match method {
                0 => contents.extend_from_slice(stored),
                8 => {
                    flate2::read::DeflateDecoder::new(stored).read_to_end(&mut contents)?;
                }
                other => bail!("{name} is compressed with method {other}, which is not supported"),
            }
            if contents.len() != size {
                bail!("{name} in the zip archive is damaged");
            }
            return Ok(contents);
        }
        at += 46 + skip;
    }
    bail!("the zip archive holds no {name}")
}

/// Where a PE file's `PE\0\0` signature is.
fn pe_header(bytes: &[u8]) -> Option<usize> {
    if !bytes.starts_with(b"MZ") {
        return None;
    }
    let at = u32::from_le_bytes(bytes.get(0x3c..0x40)?.try_into().ok()?) as usize;
    (bytes.get(at..at + 4)? == b"PE\0\0").then_some(at)
}

/// Marks a Windows executable as a GUI program rather than a console one.
///
/// `gpui-shell.exe` is a console program, so its commands can print; started
/// from Explorer, a console program opens a console window beside its own. A
/// bundle is started from Explorer, so its copy is switched to the GUI
/// subsystem — one field of the optional header, the same one the linker's
/// `/SUBSYSTEM:WINDOWS` sets. Done before anyone signs it, so it breaks no
/// signature.
fn mark_as_gui_program(bytes: &mut [u8]) -> Result<()> {
    const IMAGE_SUBSYSTEM_WINDOWS_GUI: u16 = 2;
    let optional = pe_header(bytes).ok_or_else(|| anyhow::anyhow!("not a PE executable"))? + 24;
    let magic = bytes
        .get(optional..optional + 2)
        .map(|magic| u16::from_le_bytes([magic[0], magic[1]]));
    // PE32 and PE32+ keep the subsystem at the same offset.
    if !matches!(magic, Some(0x10b | 0x20b)) {
        bail!("the PE executable has no optional header");
    }
    let subsystem = optional + 68;
    let field = bytes
        .get_mut(subsystem..subsystem + 2)
        .ok_or_else(|| anyhow::anyhow!("the PE optional header is truncated"))?;
    field.copy_from_slice(&IMAGE_SUBSYSTEM_WINDOWS_GUI.to_le_bytes());
    Ok(())
}

/// A zip archive, deflated, with Unix modes so an executable stays one when
/// it is unpacked on macOS or Linux. No Zip64: a bundle is far below 4 GiB.
fn write_zip(tree: &Tree, file: fs::File) -> io::Result<()> {
    let too_large = || io::Error::other("the bundle is too large for a zip archive");
    let mut out = io::BufWriter::new(file);
    let (time, date) = dos_time(chrono::Local::now().naive_local());
    let mut central = Vec::new();
    let mut offset = 0u32;

    for entry in &tree.entries {
        let is_directory = matches!(entry.contents, Contents::Directory);
        let name = if is_directory {
            format!("{}/", entry.path)
        } else {
            entry.path.clone()
        };
        let data = entry.read()?;
        let mut crc = flate2::Crc::new();
        crc.update(&data);
        let (method, stored) = if is_directory {
            (0u16, Vec::new())
        } else {
            let mut encoder =
                flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            encoder.write_all(&data)?;
            (8u16, encoder.finish()?)
        };
        let compressed = u32::try_from(stored.len()).map_err(|_| too_large())?;
        let size = u32::try_from(data.len()).map_err(|_| too_large())?;
        let name_length = u16::try_from(name.len()).map_err(|_| too_large())?;
        // The bit for UTF-8 names.
        let flags = 0x0800u16;

        let mut local = Vec::with_capacity(30 + name.len());
        local.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        local.extend_from_slice(&20u16.to_le_bytes());
        local.extend_from_slice(&flags.to_le_bytes());
        local.extend_from_slice(&method.to_le_bytes());
        local.extend_from_slice(&time.to_le_bytes());
        local.extend_from_slice(&date.to_le_bytes());
        local.extend_from_slice(&crc.sum().to_le_bytes());
        local.extend_from_slice(&compressed.to_le_bytes());
        local.extend_from_slice(&size.to_le_bytes());
        local.extend_from_slice(&name_length.to_le_bytes());
        local.extend_from_slice(&0u16.to_le_bytes());
        local.extend_from_slice(name.as_bytes());
        out.write_all(&local)?;
        out.write_all(&stored)?;

        let file_type = if is_directory { 0o040000 } else { 0o100000 };
        let attributes = ((file_type | entry.mode()) << 16) | if is_directory { 0x10 } else { 0 };
        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        // Made by Unix (3), so readers honour the mode in the attributes.
        central.extend_from_slice(&((3u16 << 8) | 20).to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&flags.to_le_bytes());
        central.extend_from_slice(&method.to_le_bytes());
        central.extend_from_slice(&time.to_le_bytes());
        central.extend_from_slice(&date.to_le_bytes());
        central.extend_from_slice(&crc.sum().to_le_bytes());
        central.extend_from_slice(&compressed.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&name_length.to_le_bytes());
        // Extra field, comment, disk number, internal attributes.
        central.extend_from_slice(&[0; 8]);
        central.extend_from_slice(&attributes.to_le_bytes());
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());

        offset = u32::try_from(local.len() + stored.len())
            .ok()
            .and_then(|length| offset.checked_add(length))
            .ok_or_else(too_large)?;
    }

    let count = u16::try_from(tree.entries.len()).map_err(|_| too_large())?;
    let central_size = u32::try_from(central.len()).map_err(|_| too_large())?;
    out.write_all(&central)?;
    let mut end = Vec::with_capacity(22);
    end.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    end.extend_from_slice(&[0; 4]);
    end.extend_from_slice(&count.to_le_bytes());
    end.extend_from_slice(&count.to_le_bytes());
    end.extend_from_slice(&central_size.to_le_bytes());
    end.extend_from_slice(&offset.to_le_bytes());
    end.extend_from_slice(&0u16.to_le_bytes());
    out.write_all(&end)?;
    out.flush()
}

/// A zip entry's MS-DOS time and date, which cannot say anything before 1980.
fn dos_time(time: chrono::NaiveDateTime) -> (u16, u16) {
    use chrono::{Datelike as _, Timelike as _};
    if time.year() < 1980 {
        return (0, (1 << 5) | 1);
    }
    (
        ((time.hour() << 11) | (time.minute() << 5) | (time.second() / 2)) as u16,
        ((((time.year() - 1980) as u32) << 9) | (time.month() << 5) | time.day()) as u16,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(name: &str, id: &str) -> PluginManifest {
        PluginManifest::parse(&format!(
            r#"{{ "id": "{id}", "name": "{name}", "version": "1.2.3-beta.1", "entry": "main.js" }}"#
        ))
        .expect("manifest")
    }

    #[test]
    fn names_come_from_the_manifest() {
        let names = Names::new(&manifest("Folio Desk", "com.folio.desk"), Platform::Linux);
        assert_eq!(names.display, "Folio Desk");
        assert_eq!(names.command, "folio-desk");
        assert_eq!(names.app_bundle, "Folio Desk.app");
        assert_eq!(names.archive_stem, "folio-desk-1.2.3-beta.1-linux");

        // A name with nothing ASCII in it takes its command from the id.
        let names = Names::new(&manifest("时钟", "com.example.clock"), Platform::MacOS);
        assert_eq!(names.command, "clock");
        assert_eq!(names.display, "时钟");

        assert_eq!(file_name("A/B: C?").as_deref(), Some("A-B- C-"));
        assert_eq!(file_name(" .. "), None);
    }

    #[test]
    fn the_info_plist_names_the_executable_and_escapes_values() {
        let manifest = manifest("Tom & Jerry", "com.example.tom");
        let names = Names::new(&manifest, Platform::MacOS);
        let plist = info_plist(&manifest, &names, true);
        assert!(
            plist.contains("<key>CFBundleExecutable</key>\n  <string>Tom &amp; Jerry</string>")
        );
        assert!(
            plist.contains("<key>CFBundleIdentifier</key>\n  <string>com.example.tom</string>")
        );
        assert!(plist.contains("<key>CFBundleVersion</key>\n  <string>1.2.3</string>"));
        assert!(plist.contains("<string>1.2.3-beta.1</string>"));
        assert!(plist.contains("<key>CFBundleIconFile</key>"));
    }

    fn values<'a>(name: &'a str) -> ScriptValues<'a> {
        ScriptValues {
            id: "com.example.its",
            name,
            command: "its",
            app_version: "1.0.0",
            runtime_version: "0.8.0",
            icon_size: "256".to_owned(),
            archive: "its-1.0.0-linux.tar.gz".to_owned(),
            archive_sha256: "ab".repeat(32),
            download_url: "",
            stem: "its-1.0.0-linux",
        }
    }

    #[test]
    fn shell_templates_quote_every_value_once() {
        let script = shell_script(
            "id=@ID@\nname=@NAME@\nsize=@ICON_SIZE@\nmail=me@example.com @UNKNOWN@\n",
            &values("It's @ID@"),
        );
        assert_eq!(
            script,
            "id='com.example.its'\nname='It'\\''s @ID@'\nsize='256'\n\
             mail=me@example.com @UNKNOWN@\n"
        );

        // Every placeholder the real templates use is one this fills.
        for template in [LINUX_LAUNCHER, LINUX_INSTALLER, MACOS_LAUNCHER] {
            let script = shell_script(template, &values("A"));
            // Independent of `substitute`, so a name it fails to read is caught.
            let left = script.split('@').skip(1).step_by(2).find(|name| {
                !name.is_empty()
                    && name
                        .chars()
                        .all(|character| character.is_ascii_alphanumeric() || character == '_')
            });
            assert_eq!(left, None, "a placeholder is left unfilled");
        }

        // The runtime functions come in whole, with their own placeholders filled.
        let launcher = shell_script("#!/bin/sh\n@RUNTIME_FUNCTIONS@\nexec x\n", &values("A"));
        assert!(launcher.starts_with("#!/bin/sh\n"));
        assert!(launcher.ends_with("\nexec x\n"));
        assert!(!launcher.contains("@RUNTIME_FUNCTIONS@"));
    }

    #[test]
    fn an_icns_file_holds_every_size_up_to_the_source() {
        let icns = icns(&RgbaImage::new(64, 64)).expect("icns");
        assert_eq!(&icns[..4], b"icns");
        assert_eq!(
            u32::from_be_bytes(icns[4..8].try_into().unwrap()) as usize,
            icns.len()
        );
        let mut kinds = Vec::new();
        let mut at = 8;
        while at < icns.len() {
            kinds.push(String::from_utf8_lossy(&icns[at..at + 4]).into_owned());
            at += u32::from_be_bytes(icns[at + 4..at + 8].try_into().unwrap()) as usize;
        }
        assert_eq!(kinds, ["icp4", "icp5", "icp6"]);
    }

    #[test]
    fn the_application_leaves_out_development_files_and_the_output() {
        let root =
            std::env::temp_dir().join(format!("gpui-shell-bundle-tree-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for directory in ["lib", "node_modules/x", ".git", "dist"] {
            fs::create_dir_all(root.join(directory)).expect("directory");
        }
        for file in [
            "main.js",
            "gpui-shell.json",
            "gpui-kit.d.ts",
            "jsconfig.json",
            "AGENTS.md",
            "lib/util.js",
            "lib/gpui-kit.d.ts",
            "lib/AGENTS.md",
            "node_modules/x/index.js",
            ".git/HEAD",
            "dist/old.tar.gz",
        ] {
            fs::write(root.join(file), "").expect("file");
        }

        let mut tree = Tree::default();
        let exclude = root.join("dist").canonicalize().expect("dist");
        add_application(&mut tree, &root, "top/app", &exclude).expect("tree");
        let paths: Vec<_> = tree
            .entries
            .iter()
            .map(|entry| entry.path.as_str())
            .collect();
        assert_eq!(
            paths,
            [
                "top",
                "top/app",
                "top/app/gpui-shell.json",
                "top/app/lib",
                "top/app/lib/AGENTS.md",
                "top/app/lib/util.js",
                "top/app/main.js",
            ]
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn archives_round_trip_with_their_modes() {
        let directory =
            std::env::temp_dir().join(format!("gpui-shell-bundle-archive-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("directory");
        let mut tree = Tree::default();
        tree.bytes("top/run".to_owned(), b"#!/bin/sh\n".to_vec(), true);
        tree.bytes(
            "top/app/main.js".to_owned(),
            b"export {}".repeat(100),
            false,
        );

        let archive = directory.join("bundle.tar.gz");
        replace_file(&archive, |file| write_tar_gz(&tree, file)).expect("tar.gz");
        let mut read = tar::Archive::new(flate2::read::GzDecoder::new(
            fs::File::open(&archive).expect("open"),
        ));
        let entries: Vec<_> = read
            .entries()
            .expect("entries")
            .map(|entry| {
                let entry = entry.expect("entry");
                (
                    entry.path().expect("path").display().to_string(),
                    entry.header().mode().expect("mode"),
                )
            })
            .collect();
        assert!(entries.contains(&("top/run".to_owned(), 0o755)));
        assert!(entries.contains(&("top/app/main.js".to_owned(), 0o644)));

        let zip = directory.join("bundle.zip");
        replace_file(&zip, |file| write_zip(&tree, file)).expect("zip");
        let bytes = fs::read(&zip).expect("zip bytes");
        let end = bytes.len() - 22;
        assert_eq!(&bytes[end..end + 4], &0x0605_4b50u32.to_le_bytes());
        assert_eq!(u16::from_le_bytes([bytes[end + 10], bytes[end + 11]]), 4);
        assert!(!directory.join("bundle.zip.partial").exists());

        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_windows_executable_comes_from_the_cache_or_the_release_zip() {
        let directory =
            std::env::temp_dir().join(format!("gpui-shell-bundle-runtime-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("directory");

        let mut executable = vec![0u8; 0x200];
        executable[..2].copy_from_slice(b"MZ");
        executable[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        executable[0x80..0x84].copy_from_slice(b"PE\0\0");
        executable[0x98..0x9a].copy_from_slice(&0x20bu16.to_le_bytes());
        // The console subsystem, as `gpui-shell.exe` is linked.
        executable[0x98 + 68] = 3;

        // The release zip, as written by the same zip writer the bundle uses.
        let mut tree = Tree::default();
        tree.bytes(
            format!("gpui-shell-{WINDOWS_TARGET}/README.md"),
            b"readme".to_vec(),
            false,
        );
        tree.bytes(
            format!("gpui-shell-{WINDOWS_TARGET}/gpui-shell.exe"),
            executable.clone(),
            true,
        );
        let release = directory.join("release.zip");
        replace_file(&release, |file| write_zip(&tree, file)).expect("zip");
        let release = fs::read(&release).expect("zip bytes");

        let cache = directory.join("cache").join("gpui-shell.exe");
        let downloaded = windows_runtime_from(&cache, "0.8.0", |url| {
            assert!(url.ends_with(&format!("/v0.8.0/gpui-shell-{WINDOWS_TARGET}.zip")));
            Ok(release.clone())
        })
        .expect("downloaded");
        assert_eq!(downloaded, executable);
        // The second bundle reads the cache and downloads nothing.
        let cached =
            windows_runtime_from(&cache, "0.8.0", |_| panic!("no download")).expect("cached");
        assert_eq!(cached, executable);

        let mut marked = cached;
        mark_as_gui_program(&mut marked).expect("a PE32+ header");
        assert_eq!(marked[0x98 + 68], 2);
        assert!(mark_as_gui_program(&mut b"#!/bin/sh\n".to_vec()).is_err());
        assert!(zip_entry(&release, "missing.exe").is_err());

        let _ = fs::remove_dir_all(&directory);
    }
}

//! Who a plugin is, what it may do, and where its data lives.
//!
//! A host that runs one application from a directory needs none of this: the
//! command line names the directory, the grant is decided by the act of typing
//! the command, and storage is keyed by the path. A host that runs *several*
//! applications cannot do any of that, because the three questions become
//! per plugin — identity, permission, storage — and all three have to be
//! answerable **before** the plugin's code runs.
//!
//! That is the whole reason a manifest exists, and it is why the manifest has
//! seven recognized fields: `id`, `name`, `version`, `shell-version`, `entry`,
//! `dependencies`, and `capabilities`. `version`, `shell-version`,
//! `dependencies`, and `capabilities` are
//! optional. Commands, panels, keybindings, settings and themes are
//! registered from script instead of being declared here a second time —
//! *capabilities are permission, contributions are behavior*. A permission has
//! to be shown to a user and approved before any code runs, so it belongs in
//! data; a contribution is code, so it belongs in code. Declaring contributions
//! in both places would create a class of bug (manifest and script disagreeing)
//! while producing no information the script did not already carry.
//!
//! Two consequences run through this module:
//!
//! - **Discovery executes nothing.** [`PluginManager::discover`] reads
//!   manifests and stops. A host with thirty installed plugins lists thirty
//!   names, versions and permission sets without starting thirty programs.
//!   Only [`PluginManager::load`] evaluates script.
//! - **Compatibility is checked before execution when declared.**
//!   `shell-version` names the oldest compatible gpui-shell release. Omitting
//!   it accepts the current runtime; an explicit incompatible version is
//!   rejected before the entry module can run.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    rc::Rc,
};

use anyhow::{Context as _, Result, bail};
use gpui::{App, AppContext as _, Context, Entity, IntoElement, Render, Window};
use schemars::JsonSchema;
use semver::Version;
use serde::Deserialize;

use crate::{
    capability::{Capabilities, ExecuteGrant, HttpRequestGrant},
    engine::ShellRuntime,
    policy::Policy,
    root::ShellRoot,
    runtime::failure_surface,
    scope::{self, ScopePhase},
    view::ScriptView,
};

/// The gpui-shell release that parses and executes this manifest.
pub const SHELL_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The file a plugin directory is recognized by.
///
/// §18.1 shows the manifest's content but never names the file.
/// `gpui-shell.json` makes the owning runtime explicit.
pub const MANIFEST_FILE: &str = "gpui-shell.json";

/// The JSON Schema for a manifest, for editor validation.
///
/// §18.1 keeps the schema worth generating but small enough to read: six
/// fields, one nested permission object. `crates/component/src/theme/schema.rs` is the
/// precedent for generating rather than hand-writing it — the schema and the
/// parser then cannot disagree, because both come from the same type.
pub fn manifest_schema() -> serde_json::Value {
    schemars::schema_for!(ManifestFile).to_value()
}

/// A plugin's identity and the permissions it asks for.
///
/// Cloneable and inert: holding one runs nothing, which is what lets a host
/// list, sort and display installed plugins — or show a permission sheet — with
/// no plugin code loaded.
#[derive(Clone, Debug, PartialEq)]
pub struct PluginManifest {
    id: String,
    name: String,
    version: String,
    shell_version: String,
    entry: String,
    dependencies: BTreeMap<String, GitDependency>,
    capabilities: CapabilitiesFile,
}

/// One JavaScript package fetched from Git before an application starts.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema)]
#[serde(try_from = "GitDependencyFile")]
#[schemars(with = "GitDependencyFile")]
pub struct GitDependency {
    git: String,
    branch: Option<String>,
    tag: Option<String>,
    entry: String,
    reference: Option<String>,
    package_entry: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(untagged)]
enum GitDependencyFile {
    Shorthand(String),
    Object(GitDependencyObject),
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct GitDependencyObject {
    git: String,
    #[serde(default)]
    branch: Option<String>,
    #[serde(default)]
    tag: Option<String>,
    #[serde(default = "default_dependency_entry")]
    entry: String,
}

fn default_dependency_entry() -> String {
    "index.js".to_owned()
}

impl TryFrom<GitDependencyFile> for GitDependency {
    type Error = String;

    fn try_from(value: GitDependencyFile) -> Result<Self, Self::Error> {
        match value {
            GitDependencyFile::Object(object) => Ok(Self {
                git: object.git,
                branch: object.branch,
                tag: object.tag,
                entry: object.entry,
                reference: None,
                package_entry: false,
            }),
            GitDependencyFile::Shorthand(source) => {
                let (git, reference) = parse_git_dependency_string(&source)?;
                Ok(Self {
                    git,
                    branch: None,
                    tag: None,
                    entry: default_dependency_entry(),
                    reference,
                    package_entry: true,
                })
            }
        }
    }
}

fn parse_git_dependency_string(source: &str) -> Result<(String, Option<String>), String> {
    if source.trim() != source || source.is_empty() || source.matches('#').count() > 1 {
        return Err(
            "a string dependency must be a Git URL or GitHub owner/repository with one optional #Git ref"
                .to_owned(),
        );
    }
    let (remote, fragment) = match source.split_once('#') {
        Some((_remote, "")) => {
            return Err("a string dependency #Git ref must not be empty".to_owned());
        }
        Some((remote, reference)) => (remote, Some(reference)),
        None => (source, None),
    };
    if let Some(reference) = fragment
        && !valid_git_ref_name(reference)
    {
        return Err(format!(
            "string dependency selector `{reference}` is not a valid Git ref"
        ));
    }

    if remote.contains("://") || looks_like_scp_git_url(remote) {
        if remote.chars().any(char::is_whitespace)
            || remote.ends_with("://")
            || remote.starts_with("://")
        {
            return Err("a string dependency must contain a valid Git URL".to_owned());
        }
        return Ok((remote.to_owned(), fragment.map(str::to_owned)));
    }

    let mut components = remote.split('/');
    let owner = components.next().unwrap_or_default();
    let repository = components.next().unwrap_or_default();
    if components.next().is_some()
        || !valid_github_component(owner)
        || !valid_github_component(repository)
    {
        return Err(
            "GitHub shorthand must contain exactly owner/repository plus an optional #Git ref"
                .to_owned(),
        );
    }
    Ok((
        format!("https://github.com/{owner}/{repository}"),
        Some(fragment.unwrap_or("main").to_owned()),
    ))
}

fn looks_like_scp_git_url(remote: &str) -> bool {
    let Some((authority, path)) = remote.split_once(':') else {
        return false;
    };
    authority.contains('@') && !authority.contains('/') && path.contains('/') && !path.is_empty()
}

fn valid_github_component(component: &str) -> bool {
    !component.is_empty()
        && component != "."
        && component != ".."
        && component
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

impl GitDependency {
    pub fn git(&self) -> &str {
        &self.git
    }

    pub fn branch(&self) -> Option<&str> {
        self.branch.as_deref()
    }

    pub fn tag(&self) -> Option<&str> {
        self.tag.as_deref()
    }

    pub fn entry(&self) -> &str {
        &self.entry
    }

    pub(crate) fn reference(&self) -> Option<&str> {
        self.reference.as_deref()
    }

    pub(crate) fn uses_package_entry(&self) -> bool {
        self.package_entry
    }
}

impl PluginManifest {
    /// Parses manifest source.
    ///
    /// Every failure names the field and says what was expected, because this
    /// is the first thing an author of a plugin meets and it is usually the
    /// only diagnostic they get: nothing has run yet, so there is no stack
    /// trace to fall back on.
    pub fn parse(source: &str) -> Result<Self, ManifestError> {
        Self::parse_inner(source).map_err(ManifestError::from)
    }

    /// Reads `<directory>/gpui-shell.json`.
    ///
    /// The path is carried into the error, because a host reads many manifests
    /// in one pass and "missing field `id`" is not actionable without it.
    pub fn read(directory: &Path) -> Result<Self, ManifestError> {
        let path = directory.join(MANIFEST_FILE);
        let source = std::fs::read_to_string(&path).map_err(|error| ManifestError {
            path: Some(path.clone()),
            problem: ManifestProblem::Unreadable(error.to_string()),
        })?;

        Self::parse_inner(&source).map_err(|problem| ManifestError {
            path: Some(path),
            problem,
        })
    }

    fn parse_inner(source: &str) -> Result<Self, ManifestProblem> {
        let value: serde_json::Value = serde_json::from_str(source)
            .map_err(|error| ManifestProblem::NotJson(error.to_string()))?;

        let serde_json::Value::Object(fields) = value else {
            return Err(ManifestProblem::NotAnObject(json_type_of(&value)));
        };

        // Unknown fields are rejected before missing ones, so that a typo
        // reports itself rather than reporting the field it was meant to be.
        // This is the case the design is most exposed to: `"capabilites"` is
        // optional-looking, and accepting it would hand the plugin an empty
        // grant while its author believes it was granted everything listed.
        for field in fields.keys() {
            if !FIELDS.contains(&field.as_str()) {
                return Err(ManifestProblem::UnknownField {
                    field: field.clone(),
                    suggestion: nearest_field(field),
                });
            }
        }

        let id = string_field(&fields, "id")?;
        validate_id(&id)?;
        let name = string_field(&fields, "name")?;
        let version = match fields.get("version") {
            None | Some(serde_json::Value::Null) => "unknown".to_owned(),
            Some(_) => {
                let version = string_field(&fields, "version")?;
                validate_version(&version)?;
                version
            }
        };
        let shell_version = match fields.get("shell-version") {
            None | Some(serde_json::Value::Null) => SHELL_VERSION.to_owned(),
            Some(_) => {
                let shell_version = string_field(&fields, "shell-version")?;
                validate_shell_version(&shell_version)?;
                shell_version
            }
        };
        let entry = string_field(&fields, "entry")?;
        validate_entry(&entry)?;
        let dependencies = match fields.get("dependencies") {
            None | Some(serde_json::Value::Null) => BTreeMap::new(),
            Some(value) => serde_json::from_value::<BTreeMap<String, GitDependency>>(value.clone())
                .map_err(|error| ManifestProblem::Dependencies(error.to_string()))?,
        };
        validate_dependencies(&dependencies)?;

        // An omitted `capabilities` has the one permission default that cannot
        // be an accident: absent means the empty grant
        // (§5.7), which is also what an explicit `{}` means. Requiring the key
        // would add a line that says "nothing" to every plugin that wants
        // nothing.
        let capabilities = match fields.get("capabilities") {
            None | Some(serde_json::Value::Null) => CapabilitiesFile::default(),
            Some(value) => serde_json::from_value(value.clone())
                .map_err(|error| ManifestProblem::Capabilities(error.to_string()))?,
        };
        capabilities.validate_placeholders()?;
        capabilities.validate_network()?;

        Ok(Self {
            id,
            name,
            version,
            shell_version,
            entry,
            dependencies,
            capabilities,
        })
    }

    /// The namespace this plugin owns.
    ///
    /// It is the panel-name prefix (`script:<id>/<panel>`, §15.4), the storage
    /// key, the log field and the identity a capability approval is recorded
    /// against — which is why the manifest parser validates ids strictly.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The human-readable name, for menus and permission sheets.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The plugin's own version, or `unknown` when the manifest omits it.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// The oldest gpui-shell release this application requires.
    pub fn shell_version(&self) -> &str {
        &self.shell_version
    }

    /// The module evaluated at load, relative to the plugin directory.
    pub fn entry(&self) -> &str {
        &self.entry
    }

    /// Git-backed packages available to JavaScript as bare module imports.
    pub fn dependencies(&self) -> &BTreeMap<String, GitDependency> {
        &self.dependencies
    }

    /// The grant this manifest asks for, resolved against the two directories
    /// only the host knows.
    ///
    /// The manifest writes `${pluginDir}` and `${dataDir}` (§18.1) rather than
    /// real paths, for the same reason a plugin cannot name its own storage
    /// location: a path chosen by the plugin is a path the plugin can point
    /// anywhere. So the *shape* of the grant comes from the manifest and
    /// nowhere else, while the two directories it is anchored to come from the
    /// host and nowhere else. A relative path is anchored to the plugin
    /// directory; an absolute path is taken as written, and is the case a host
    /// policy or an approval prompt (§19.2) exists to gate.
    pub fn capabilities(&self, plugin_dir: &Path, data_dir: &Path) -> Capabilities {
        self.capabilities.grant(plugin_dir, data_dir)
    }
}

const FIELDS: [&str; 7] = [
    "id",
    "name",
    "version",
    "shell-version",
    "entry",
    "dependencies",
    "capabilities",
];

fn validate_dependencies(
    dependencies: &BTreeMap<String, GitDependency>,
) -> Result<(), ManifestProblem> {
    for (name, dependency) in dependencies {
        if name.is_empty()
            || name.starts_with(['.', '/'])
            || name.contains(['\\', ':'])
            || name.split('/').any(|part| part.is_empty() || part == "..")
        {
            return Err(ManifestProblem::Dependencies(format!(
                "`{name}` is not a valid bare module name"
            )));
        }
        if crate::host_modules::RESERVED_SPECIFIERS.contains(&name.as_str()) {
            return Err(ManifestProblem::Dependencies(format!(
                "`{name}` is reserved by gpui-shell and cannot name a Git dependency"
            )));
        }
        if dependency.git.trim().is_empty() {
            return Err(ManifestProblem::Dependencies(format!(
                "`{name}.git` must not be empty"
            )));
        }
        if dependency.uses_package_entry() {
            if let Some(reference) = dependency.reference()
                && !valid_git_ref_name(reference)
            {
                return Err(ManifestProblem::Dependencies(format!(
                    "`{name}` selector `{reference}` is not a valid Git ref"
                )));
            }
            continue;
        }
        let reference = match (&dependency.branch, &dependency.tag) {
            (Some(branch), None) if !branch.trim().is_empty() => branch,
            (None, Some(tag)) if !tag.trim().is_empty() => tag,
            (Some(_), Some(_)) => {
                return Err(ManifestProblem::Dependencies(format!(
                    "`{name}` must select either `branch` or `tag`, not both"
                )));
            }
            _ => {
                return Err(ManifestProblem::Dependencies(format!(
                    "`{name}` must select one non-empty `branch` or `tag`"
                )));
            }
        };
        if !valid_git_ref_name(reference) {
            return Err(ManifestProblem::Dependencies(format!(
                "`{name}` selector `{reference}` is not a valid Git ref name"
            )));
        }
        validate_entry(&dependency.entry).map_err(|_| {
            ManifestProblem::Dependencies(format!(
                "`{name}.entry` must be a path inside the Git repository"
            ))
        })?;
    }
    Ok(())
}

fn valid_git_ref_name(reference: &str) -> bool {
    reference != "@"
        && !reference.starts_with(['.', '/'])
        && !reference.ends_with(['.', '/'])
        && !reference.contains("..")
        && !reference.contains("@{")
        && !reference.contains("//")
        && !reference.chars().any(|character| {
            character.is_control()
                || character.is_whitespace()
                || matches!(character, '~' | '^' | ':' | '?' | '*' | '[' | '\\')
        })
        && reference.split('/').all(|component| {
            !component.is_empty() && !component.starts_with('.') && !component.ends_with(".lock")
        })
}

fn string_field(
    fields: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<String, ManifestProblem> {
    match fields.get(field) {
        None | Some(serde_json::Value::Null) => Err(ManifestProblem::MissingField(field)),
        Some(serde_json::Value::String(value)) if value.trim().is_empty() => {
            Err(ManifestProblem::EmptyField(field))
        }
        Some(serde_json::Value::String(value)) => Ok(value.clone()),
        Some(other) => Err(ManifestProblem::WrongType {
            field,
            found: json_type_of(other),
        }),
    }
}

/// An `id` is used verbatim as a directory name, a panel-name prefix and a log
/// field, so the characters it may contain are decided by the strictest of
/// those three uses rather than by taste.
///
/// The two rules that are security and not style: no path separators and no
/// `..`, because `data_dir/<id>` must stay inside the data directory; and no
/// uppercase, because two ids differing only in case would be one directory on
/// a case-insensitive filesystem and two everywhere else.
fn validate_id(id: &str) -> Result<(), ManifestProblem> {
    if let Some(character) = id
        .chars()
        .find(|character| !matches!(character, 'a'..='z' | '0'..='9' | '.' | '-' | '_'))
    {
        return Err(ManifestProblem::InvalidId {
            id: id.to_owned(),
            reason: format!("`{character}` is not allowed"),
        });
    }

    let bounded_by_separator =
        |value: &str| value.starts_with(['.', '-', '_']) || value.ends_with(['.', '-', '_']);
    if bounded_by_separator(id) || id.contains("..") {
        return Err(ManifestProblem::InvalidId {
            id: id.to_owned(),
            reason: "it must begin and end with a letter or a digit".to_owned(),
        });
    }

    Ok(())
}

/// When supplied, a plugin version is compared across an upgrade (§19.4: an
/// update that adds capabilities asks again), and comparison needs an agreed
/// shape. Semver's is the one §23 already uses.
fn validate_version(version: &str) -> Result<(), ManifestProblem> {
    Version::parse(version)
        .map(|_| ())
        .map_err(|_| ManifestProblem::InvalidVersion(version.to_owned()))
}

fn validate_shell_version(version: &str) -> Result<(), ManifestProblem> {
    let required = Version::parse(version)
        .map_err(|_| ManifestProblem::InvalidShellVersion(version.to_owned()))?;
    let runtime = Version::parse(SHELL_VERSION).expect("the crate version is semantic");

    if runtime >= required {
        Ok(())
    } else {
        Err(ManifestProblem::IncompatibleShellVersion {
            required: version.to_owned(),
            runtime: SHELL_VERSION.to_owned(),
        })
    }
}

/// The entry is resolved inside the plugin directory, so it must be a path that
/// cannot leave it. This is the same rule the module resolver applies to every
/// `import` (§19.1); applying it here means a manifest cannot ask for a file
/// the resolver would refuse anyway.
fn validate_entry(entry: &str) -> Result<(), ManifestProblem> {
    let path = Path::new(entry);
    let escapes = path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
        || entry.starts_with('\\')
        || entry.contains(':');

    if escapes {
        Err(ManifestProblem::InvalidEntry(entry.to_owned()))
    } else {
        Ok(())
    }
}

fn json_type_of(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "a boolean",
        serde_json::Value::Number(_) => "a number",
        serde_json::Value::String(_) => "a string",
        serde_json::Value::Array(_) => "an array",
        serde_json::Value::Object(_) => "an object",
    }
}

/// The closest known field name, so a typo is answered with the word the author
/// meant rather than with a list they have to scan.
fn nearest_field(unknown: &str) -> Option<&'static str> {
    FIELDS
        .iter()
        .map(|field| (edit_distance(field, unknown), *field))
        .filter(|(distance, _)| *distance <= 3)
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, field)| field)
}

fn edit_distance(left: &str, right: &str) -> usize {
    let right: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    let mut current = vec![0; right.len() + 1];

    for (row, left_character) in left.chars().enumerate() {
        current[0] = row + 1;
        for (column, right_character) in right.iter().enumerate() {
            let substitution = usize::from(left_character != *right_character);
            current[column + 1] = (previous[column] + substitution)
                .min(previous[column + 1] + 1)
                .min(current[column] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[right.len()]
}

// -----------------------------------------------------------------------------
// The file form of the manifest
// -----------------------------------------------------------------------------

/// The serde and schemars view of a manifest.
///
/// It exists only so `schemars` can describe the file; parsing goes through
/// [`PluginManifest::parse`], which reports one field at a time. Keeping the
/// two in one module is what stops the schema from describing a file the
/// parser would reject.
// The fields are never read through this type — parsing goes field by field so
// each failure can carry its own explanation — but they are what `schemars`
// walks, and they are read by the test that keeps this type and the parser
// agreeing.
#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ManifestFile {
    /// Reverse-DNS identity, e.g. `com.example.inbox`. Also the namespace for
    /// panels, storage and capability records.
    id: String,
    /// Human-readable name, shown in menus and in the permission prompt.
    name: String,
    /// Optional plugin semantic version, e.g. `1.2.0`.
    #[serde(default)]
    version: Option<String>,
    /// Optional oldest compatible gpui-shell release. Checked before `entry` executes.
    #[serde(default, rename = "shell-version")]
    shell_version: Option<String>,
    /// The module evaluated at load, relative to the plugin directory.
    entry: String,
    /// Git-backed packages imported by their map key from JavaScript.
    #[serde(default)]
    dependencies: BTreeMap<String, GitDependency>,
    /// What the plugin is allowed to do. Absent means nothing but its own
    /// storage — see [`CapabilitiesFile::storage`].
    #[serde(default)]
    capabilities: CapabilitiesFile,
}

/// The `capabilities` block, before it is anchored to real directories.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CapabilitiesFile {
    /// Filesystem and subprocess access.
    #[serde(default)]
    fs: Option<FsGrantFile>,
    /// Outbound network access, by host.
    #[serde(default)]
    network: Option<NetworkGrantFile>,
    /// Whether `localStorage` is available.
    ///
    /// The one grant that defaults to *given*, and it follows the web: a
    /// browser hands every origin a `localStorage` unconditionally, because
    /// what it reaches is that origin's own data and nothing else. The same
    /// holds here — storage is keyed by bundle id, cannot name its own file,
    /// and is bounded — so there is nothing for an author to ask permission
    /// for. Defaulting it to `false` also had a trap: an application that
    /// added a manifest to declare a network host silently lost its settings.
    ///
    /// A host that runs code it does not trust can still say `false`, which is
    /// why this stays a capability rather than becoming ambient like
    /// `sessionStorage`.
    #[serde(default = "granted")]
    storage: bool,
    /// Clipboard access.
    #[serde(default)]
    clipboard: Option<ClipboardGrantFile>,
    /// Process-level host requests, separate from filesystem execution.
    #[serde(default)]
    process: Option<ProcessGrantFile>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct FsGrantFile {
    /// Directories that may be read. `${pluginDir}` and `${dataDir}` expand to
    /// the plugin's own directory and its storage directory.
    #[serde(default)]
    read: Vec<String>,
    /// Directories that may be written.
    #[serde(default)]
    write: Vec<String>,
    /// Commands `process.run` may start.
    #[serde(default)]
    execute: Option<ExecuteFile>,
}

/// Either an allowlist of command names, or the string `"*"`.
///
/// Unrestricted execution has to be spellable — a host that cannot express it
/// pushes its users to grant a wildcard read root instead, which is worse — but
/// it is spelled differently from an allowlist so that a permission sheet can
/// show it at the severity it deserves (§19.2).
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema)]
#[serde(untagged)]
enum ExecuteFile {
    Allowed(Vec<String>),
    Unrestricted(String),
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct NetworkGrantFile {
    /// Hosts that may be reached, e.g. `api.example.com`.
    #[serde(default)]
    hosts: Vec<String>,
    /// HTTP requests constrained by host, method and URL path.
    #[serde(default)]
    http: Vec<HttpRequestGrantFile>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct HttpRequestGrantFile {
    #[serde(default = "default_https_scheme")]
    scheme: String,
    host: String,
    #[serde(default)]
    port: Option<u16>,
    methods: Vec<String>,
    #[serde(default)]
    paths: Vec<String>,
    #[serde(default)]
    path_prefixes: Vec<String>,
}

fn default_https_scheme() -> String {
    "https".to_owned()
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ClipboardGrantFile {
    #[serde(default)]
    read: bool,
    #[serde(default)]
    write: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ProcessGrantFile {
    #[serde(default)]
    exit: bool,
}

const PLUGIN_DIR_PLACEHOLDER: &str = "${pluginDir}";
const DATA_DIR_PLACEHOLDER: &str = "${dataDir}";

fn granted() -> bool {
    true
}

/// Written out rather than derived, because a derived `Default` would ignore
/// the `serde` field defaults above — and then an absent `capabilities` block
/// and a block that merely omits `storage` would disagree about storage. They
/// have to be the same answer, or "does declaring a network host cost me my
/// settings?" depends on whether the block exists at all.
impl Default for CapabilitiesFile {
    fn default() -> Self {
        Self {
            fs: None,
            network: None,
            storage: granted(),
            clipboard: None,
            process: None,
        }
    }
}

impl CapabilitiesFile {
    fn grant(&self, plugin_dir: &Path, data_dir: &Path) -> Capabilities {
        let fs = self.fs.clone().unwrap_or_default();
        let clipboard = self.clipboard.clone().unwrap_or_default();
        let process = self.process.clone().unwrap_or_default();
        let execute = match fs.execute.clone() {
            None => ExecuteGrant::Denied,
            Some(ExecuteFile::Unrestricted(_)) => ExecuteGrant::Unrestricted,
            Some(ExecuteFile::Allowed(commands)) => ExecuteGrant::Allowed(commands),
        };

        let network = self.network.clone().unwrap_or_default();
        Capabilities::new()
            .read_roots(expand_all(&fs.read, plugin_dir, data_dir))
            .write_roots(expand_all(&fs.write, plugin_dir, data_dir))
            .execute(execute)
            .network_hosts(network.hosts.into_iter().map(|host| host.to_lowercase()))
            .http_requests(network.http.into_iter().map(|request| {
                let mut grant = HttpRequestGrant::new(
                    request.host,
                    request.methods,
                    request.paths,
                    request.path_prefixes,
                )
                .scheme(request.scheme);
                if let Some(port) = request.port {
                    grant = grant.port(port);
                }
                grant
            }))
            .storage(self.storage)
            .clipboard_read(clipboard.read)
            .clipboard_write(clipboard.write)
            .exit(process.exit)
    }

    /// A placeholder the host does not expand would otherwise reach
    /// [`Capabilities`] as the literal directory name `${dataDir}`, and grant
    /// access to a directory that does not exist. Catching it at parse time
    /// makes it a manifest error, which is where an author can see it.
    fn validate_placeholders(&self) -> Result<(), ManifestProblem> {
        let Some(fs) = &self.fs else {
            return Ok(());
        };

        for (field, paths) in [("read", &fs.read), ("write", &fs.write)] {
            for path in paths {
                if let Some(placeholder) = unknown_placeholder(path) {
                    return Err(ManifestProblem::UnknownPlaceholder {
                        field: format!("capabilities.fs.{field}"),
                        placeholder,
                    });
                }
            }
        }

        Ok(())
    }

    fn validate_network(&self) -> Result<(), ManifestProblem> {
        let Some(network) = &self.network else {
            return Ok(());
        };
        for host in network
            .hosts
            .iter()
            .chain(network.http.iter().map(|rule| &rule.host))
        {
            if host.is_empty() || host.contains("://") || host.contains('/') {
                return Err(ManifestProblem::Capabilities(format!(
                    "network host `{host}` must be a hostname without a scheme or path"
                )));
            }
        }
        for (index, rule) in network.http.iter().enumerate() {
            if !matches!(rule.scheme.as_str(), "http" | "https") {
                return Err(ManifestProblem::Capabilities(format!(
                    "network.http[{index}].scheme must be `http` or `https`"
                )));
            }
            if rule.methods.is_empty() {
                return Err(ManifestProblem::Capabilities(format!(
                    "network.http[{index}].methods must contain at least one HTTP method"
                )));
            }
            for method in &rule.methods {
                if !matches!(method.as_str(), "GET" | "POST") {
                    return Err(ManifestProblem::Capabilities(format!(
                        "network.http[{index}].methods contains invalid HTTP method `{method}`"
                    )));
                }
            }
            for (field, paths) in [
                ("paths", &rule.paths),
                ("path_prefixes", &rule.path_prefixes),
            ] {
                for path in paths {
                    if !path.starts_with('/') {
                        return Err(ManifestProblem::Capabilities(format!(
                            "network.http[{index}].{field} entry `{path}` must start with `/`"
                        )));
                    }
                }
            }
        }
        Ok(())
    }
}

fn unknown_placeholder(value: &str) -> Option<String> {
    let mut rest = value;
    while let Some(start) = rest.find("${") {
        let tail = &rest[start..];
        let end = tail.find('}')? + 1;
        let placeholder = &tail[..end];
        if placeholder != PLUGIN_DIR_PLACEHOLDER && placeholder != DATA_DIR_PLACEHOLDER {
            return Some(placeholder.to_owned());
        }
        rest = &tail[end..];
    }
    None
}

fn expand_all(paths: &[String], plugin_dir: &Path, data_dir: &Path) -> Vec<PathBuf> {
    paths
        .iter()
        .map(|path| expand(path, plugin_dir, data_dir))
        .collect()
}

fn expand(raw: &str, plugin_dir: &Path, data_dir: &Path) -> PathBuf {
    let expanded = raw
        .replace(
            PLUGIN_DIR_PLACEHOLDER,
            plugin_dir.to_string_lossy().as_ref(),
        )
        .replace(DATA_DIR_PLACEHOLDER, data_dir.to_string_lossy().as_ref());

    let path = PathBuf::from(expanded);
    if path.is_absolute() {
        path
    } else {
        plugin_dir.join(path)
    }
}

// -----------------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------------

/// Why a manifest could not be used, and — when it was read from disk — which
/// file it was.
///
/// The path is a separate field rather than a variant so that every problem
/// gains it without the enum doubling in size, and so a caller can match on the
/// problem without unwrapping a location first.
#[derive(Debug, PartialEq)]
pub struct ManifestError {
    path: Option<PathBuf>,
    problem: ManifestProblem,
}

impl ManifestError {
    /// The manifest file, when the manifest came from one.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn problem(&self) -> &ManifestProblem {
        &self.problem
    }
}

impl From<ManifestProblem> for ManifestError {
    fn from(problem: ManifestProblem) -> Self {
        Self {
            path: None,
            problem,
        }
    }
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.path {
            Some(path) => write!(f, "{}: {}", path.display(), self.problem),
            None => self.problem.fmt(f),
        }
    }
}

impl std::error::Error for ManifestError {}

/// What was wrong with a manifest.
///
/// Wording follows `CapabilityError`: say what is wrong, then say what to
/// write instead. A plugin author reading one of these has no other diagnostic
/// available — nothing has run yet.
#[derive(Debug, PartialEq)]
pub enum ManifestProblem {
    Unreadable(String),
    NotJson(String),
    NotAnObject(&'static str),
    MissingField(&'static str),
    EmptyField(&'static str),
    WrongType {
        field: &'static str,
        found: &'static str,
    },
    UnknownField {
        field: String,
        suggestion: Option<&'static str>,
    },
    InvalidId {
        id: String,
        reason: String,
    },
    DuplicateId {
        id: String,
        first: PathBuf,
    },
    InvalidVersion(String),
    InvalidShellVersion(String),
    IncompatibleShellVersion {
        required: String,
        runtime: String,
    },
    InvalidEntry(String),
    UnknownPlaceholder {
        field: String,
        placeholder: String,
    },
    Capabilities(String),
    Dependencies(String),
}

/// What each field is for, in one line, appended to the error that reports it
/// missing. A field name alone tells an author which key to add but not what to
/// put in it.
fn field_expectation(field: &str) -> &'static str {
    match field {
        "id" => {
            "a reverse-DNS identifier such as \"com.example.inbox\"; it is also the plugin's namespace for panels, storage and permissions"
        }
        "name" => {
            "a human-readable name such as \"Inbox\", shown in menus and in the permission prompt"
        }
        "version" => "the plugin's own semantic version such as \"1.2.0\"",
        "shell-version" => "the oldest compatible gpui-shell semantic version, such as \"0.1.0\"",
        "entry" => {
            "the module to evaluate at load, such as \"main.js\", relative to the plugin directory"
        }
        _ => "a value",
    }
}

impl std::fmt::Display for ManifestProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestProblem::Unreadable(error) => {
                write!(f, "cannot read the manifest: {error}")
            }
            ManifestProblem::NotJson(error) => {
                write!(f, "the manifest is not valid JSON: {error}")
            }
            ManifestProblem::NotAnObject(found) => write!(
                f,
                "the manifest must be a JSON object with the fields {}, found {found}",
                FIELDS.join(", ")
            ),
            ManifestProblem::MissingField(field) => write!(
                f,
                "missing field `{field}`: expected {}",
                field_expectation(field)
            ),
            ManifestProblem::EmptyField(field) => write!(
                f,
                "field `{field}` is empty: expected {}",
                field_expectation(field)
            ),
            ManifestProblem::WrongType { field, found } => write!(
                f,
                "field `{field}` must be a string, found {found}: expected {}",
                field_expectation(field)
            ),
            ManifestProblem::UnknownField { field, suggestion } => {
                write!(f, "unknown field `{field}`")?;
                if let Some(suggestion) = suggestion {
                    write!(f, "; did you mean `{suggestion}`?")?;
                }
                write!(
                    f,
                    " A manifest has exactly the fields {}. Commands, panels, keybindings, settings and themes are registered in script, not declared here.",
                    FIELDS.join(", ")
                )
            }
            ManifestProblem::InvalidId { id, reason } => write!(
                f,
                "invalid `id` \"{id}\": {reason}; an id may contain lowercase letters, digits, `.`, `-` and `_`, because it is used verbatim as a panel name prefix (script:<id>/<panel>) and as a directory name under the user's data directory"
            ),
            ManifestProblem::DuplicateId { id, first } => write!(
                f,
                "`{id}` is already provided by {}; a plugin id is a namespace, and two plugins sharing one would share their storage, their panel names and their permissions",
                first.display()
            ),
            ManifestProblem::InvalidVersion(version) => write!(
                f,
                "invalid `version` \"{version}\": expected a semantic version such as \"1.2.0\""
            ),
            ManifestProblem::InvalidShellVersion(version) => write!(
                f,
                "invalid `shell-version` \"{version}\": expected a semantic version such as \"0.1.0\""
            ),
            ManifestProblem::IncompatibleShellVersion { required, runtime } => write!(
                f,
                "this application requires gpui-shell {required}, but this runtime is {runtime} and is not compatible"
            ),
            ManifestProblem::InvalidEntry(entry) => write!(
                f,
                "invalid `entry` \"{entry}\": expected a path inside the plugin directory, such as \"main.js\"; an absolute path or one containing `..` is refused for the same reason an `import` of one is"
            ),
            ManifestProblem::UnknownPlaceholder { field, placeholder } => write!(
                f,
                "unknown placeholder `{placeholder}` in {field}: the manifest may use {PLUGIN_DIR_PLACEHOLDER} and {DATA_DIR_PLACEHOLDER}"
            ),
            ManifestProblem::Capabilities(error) => write!(
                f,
                "invalid `capabilities`: {error}. The block accepts fs (read, write, execute), network (hosts, http), storage, clipboard (read, write) and process (exit)"
            ),
            ManifestProblem::Dependencies(error) => write!(
                f,
                "invalid `dependencies`: {error}. Use a GitHub owner/repository shorthand or full Git URL with an optional #ref, or an object with a Git URL, exactly one non-empty `branch` or `tag`, and an optional repository-relative `entry`"
            ),
        }
    }
}

// -----------------------------------------------------------------------------
// Loaded plugins
// -----------------------------------------------------------------------------

/// One installed plugin: its manifest, where it lives, and — once loaded — what
/// it turned into.
pub struct Plugin {
    manifest: PluginManifest,
    root: PathBuf,
    data_dir: PathBuf,
    store_path: PathBuf,
    /// The authority this plugin's code runs under, built once at load from its
    /// manifest and then never swapped. Callbacks it registers capture it, so a
    /// timer firing inside plugin A cannot run with plugin B's grant.
    policy: Rc<Policy>,
    runtime: Rc<ShellRuntime>,
    application: Option<Rc<crate::runtime::ApplicationGeneration>>,
    view: Entity<ScriptView>,
}

impl Plugin {
    pub fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    pub fn id(&self) -> &str {
        self.manifest.id()
    }

    /// The directory the manifest was read from.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Where this plugin's data lives, keyed by `id` so it survives an upgrade
    /// that moves the plugin directory.
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// The file behind `localStorage`.
    pub fn store_path(&self) -> &Path {
        &self.store_path
    }

    /// The grant in force while this plugin runs. It comes from the manifest
    /// and nowhere else.
    pub fn capabilities(&self) -> &Capabilities {
        self.policy.capabilities()
    }

    /// The view the entry module default-exported.
    pub fn view(&self) -> &Entity<ScriptView> {
        &self.view
    }

    fn shutdown(mut self) {
        if let Some(application) = self.application.take() {
            self.runtime
                .release_application_generation_without_context(&application);
        }
        crate::engine::quickjs::cancel_policy_tasks(&self.policy);
    }
}

fn load_plugin(
    runtime: &Rc<ShellRuntime>,
    manifest: PluginManifest,
    root: PathBuf,
    data_dir: PathBuf,
    window: &mut Window,
    cx: &mut App,
) -> Result<Plugin> {
    let id = manifest.id().to_owned();
    let store_path = data_dir.join("store.json");
    if let Err(error) = std::fs::create_dir_all(&data_dir) {
        tracing::warn!(
            "storage is unavailable for `{id}`: cannot create {}: {error}",
            data_dir.display()
        );
    }

    let policy = Rc::new(
        Policy::default()
            // The manifest id is already unique among loaded plugins, which is
            // exactly what a dock layout needs to keep two plugins' panels of
            // the same name apart.
            .with_application(&id)
            .with_capabilities(manifest.capabilities(&root, &data_dir))
            .with_storage_path(store_path.clone()),
    );

    let view = load_view_with_policy(runtime, &root, manifest.entry(), policy.clone(), window, cx)
        .with_context(|| format!("loading application `{id}`"))?;

    let application = view.read(cx).application_generation();
    Ok(Plugin {
        manifest,
        root,
        data_dir,
        store_path,
        policy,
        runtime: runtime.clone(),
        application,
        view,
    })
}

fn load_view_with_policy(
    runtime: &Rc<ShellRuntime>,
    root: &Path,
    entry: &str,
    policy: Rc<Policy>,
    window: &mut Window,
    cx: &mut App,
) -> Result<Entity<ScriptView>> {
    let loaded = {
        let (_scope, _) =
            scope::enter_with_runtime(runtime, window, cx, ScopePhase::Task, None, policy.clone());
        runtime.load_app(root, entry).and_then(|view_type| {
            runtime.instantiate_view_with_policy(&view_type, policy.clone(), window, cx)
        })
    };

    if loaded.is_err() {
        crate::engine::quickjs::cancel_policy_tasks(&policy);
    }
    loaded
}

struct ApplicationLoadFailure {
    message: String,
}

impl Render for ApplicationLoadFailure {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        failure_surface(
            "This application could not be loaded",
            &self.message,
            "Fix the application and start it again.",
            window,
            cx,
        )
    }
}

impl ShellRuntime {
    /// Loads one application as this window's root view.
    ///
    /// The common single-application host needs no plugin discovery or id
    /// lookup. A manifest selects identity and entry metadata, but its
    /// capability block is only a request: this method always uses the policy
    /// the host installed. A load error becomes the normal selectable failure
    /// surface so a bad script cannot crash the host while constructing its
    /// window.
    pub fn load(
        self: &Rc<Self>,
        root: impl AsRef<Path>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<ShellRoot> {
        match self.try_load(root, window, cx) {
            Ok(root) => root,
            Err(error) => load_failure_root(error.to_string(), window, cx),
        }
    }

    /// Loads one application and preserves a structured error for hosts that
    /// do not want the convenience failure surface returned by [`Self::load`].
    pub fn try_load(
        self: &Rc<Self>,
        root: impl AsRef<Path>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Entity<ShellRoot>> {
        let root = root.as_ref().to_path_buf();
        let manifest_path = root.join(MANIFEST_FILE);
        let entry = if manifest_path.is_file() {
            let manifest = PluginManifest::read(&root)?;
            manifest.entry().to_owned()
        } else {
            "main.js".to_owned()
        };
        let root = crate::runtime::resolve_app_root(&root, &entry)?;
        let policy = Rc::new(crate::policy::default().duplicate());
        let view = load_view_with_policy(self, &root, &entry, policy.clone(), window, cx)?;
        Ok(cx.new(|cx| ShellRoot::with_application(view, root, entry, policy, window, cx)))
    }

    /// Loads and renders an application once, returning its description.
    ///
    /// This is the headless host path used by source checks. It keeps engine
    /// handles and the `ScriptView` construction protocol inside the shell
    /// facade while preserving structured load and render errors.
    pub fn check(
        self: &Rc<Self>,
        root: impl AsRef<Path>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<String> {
        let root = self.try_load(root, window, cx)?;
        let view = root
            .read(cx)
            .application()
            .expect("try_load always mounts an application")
            .view
            .clone();
        let object = view.read(cx).object().clone();
        self.render_to_spec(&object, Some(view), window, cx)
    }
}

fn load_failure_root(message: String, window: &mut Window, cx: &mut App) -> Entity<ShellRoot> {
    tracing::error!("{message}");
    let failure = cx.new(|_| ApplicationLoadFailure { message });
    cx.new(|cx| ShellRoot::new(failure.into(), window, cx))
}

/// Discovers, loads and unloads plugins.
///
/// Every loaded plugin holds its own [`Policy`] — its grant, its storage, its
/// host modules — and every call into its code runs under that policy because
/// the policy travels on the call frame rather than in a process-wide slot. Two
/// plugins loaded at once hold two different grants at the same time, and
/// neither can see the other's files.
///
/// This used to be one slot with a guard around each call, and the guard could
/// not be made correct: a plugin that `await`s hands control back before its
/// guard drops, so the grant in force during the continuation was whichever
/// plugin happened to be running when the promise resolved. Time is what the
/// swap could not account for. Authority now belongs to the code rather than to
/// the moment.
pub struct PluginManager {
    directories: Vec<PathBuf>,
    data_home: PathBuf,
    catalog: Vec<CatalogEntry>,
    discovered: bool,
    loaded: BTreeMap<String, Plugin>,
}

struct CatalogEntry {
    manifest: PluginManifest,
    root: PathBuf,
}

impl PluginManager {
    /// `directories` are searched in order; an earlier directory wins a
    /// duplicate `id`, which is what lets a user's own copy shadow a bundled
    /// one.
    pub fn new(directories: Vec<PathBuf>) -> Self {
        Self {
            directories,
            data_home: default_data_home(),
            catalog: Vec::new(),
            discovered: false,
            loaded: BTreeMap::new(),
        }
    }

    /// Overrides where plugin data lives. A host that keeps a portable profile
    /// needs this, and so does a test that must not touch the real one.
    pub fn with_data_home(mut self, path: PathBuf) -> Self {
        self.data_home = path;
        self
    }

    pub fn directories(&self) -> &[PathBuf] {
        &self.directories
    }

    /// Reads every manifest found under the configured directories.
    ///
    /// **Nothing is executed.** This is the whole point of the manifest: a host
    /// can show thirty plugins, their versions and the permissions each asks
    /// for, having started none of them.
    ///
    /// Both outcomes are returned in one list rather than the whole pass
    /// failing on the first bad manifest — one broken plugin must not hide the
    /// twenty-nine working ones, and the broken one still has to be reportable.
    /// Results are ordered by path so a listing does not reshuffle itself
    /// between runs.
    pub fn discover(&mut self) -> Vec<Result<PluginManifest, ManifestError>> {
        self.catalog.clear();
        self.discovered = true;

        let mut results = Vec::new();
        let mut seen: BTreeMap<String, PathBuf> = BTreeMap::new();

        for directory in self.directories.clone() {
            for root in plugin_roots(&directory) {
                let manifest = match PluginManifest::read(&root) {
                    Ok(manifest) => manifest,
                    Err(error) => {
                        results.push(Err(error));
                        continue;
                    }
                };

                // First directory wins, and the loser is reported rather
                // than dropped: a shadowed plugin that simply never appears is
                // the hardest kind of install problem to diagnose.
                if let Some(first) = seen.get(manifest.id()) {
                    results.push(Err(ManifestError {
                        path: Some(root.join(MANIFEST_FILE)),
                        problem: ManifestProblem::DuplicateId {
                            id: manifest.id().to_owned(),
                            first: first.clone(),
                        },
                    }));
                    continue;
                }

                seen.insert(manifest.id().to_owned(), root.clone());
                self.catalog.push(CatalogEntry {
                    manifest: manifest.clone(),
                    root,
                });
                results.push(Ok(manifest));
            }
        }

        results
    }

    /// The manifests found by the last [`discover`](Self::discover).
    pub fn available(&self) -> impl Iterator<Item = &PluginManifest> {
        self.catalog.iter().map(|entry| &entry.manifest)
    }

    /// Evaluates a plugin's entry module and constructs its view.
    ///
    /// This is the only method that runs script. `authorize` is called with the
    /// inert manifest before its requested capabilities become a policy; a
    /// denial executes nothing. The whole approved load then runs inside that
    /// policy, because the entry module may use capabilities while registering.
    /// Call [`Self::discover`] first and handle every returned result; `load`
    /// never hides a malformed manifest behind a generic missing-id error.
    pub fn load(
        &mut self,
        runtime: &Rc<ShellRuntime>,
        id: &str,
        authorize: impl FnOnce(&PluginManifest) -> bool,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<()> {
        if !self.discovered {
            bail!(
                "plugin discovery has not run; call PluginManager::discover() and handle every result before loading `{id}`"
            );
        }

        if self.loaded.contains_key(id) {
            bail!("plugin `{id}` is already loaded; unload it first to load it again");
        }

        let Some(entry) = self.catalog.iter().find(|entry| entry.manifest.id() == id) else {
            bail!("no plugin `{id}`{}", self.known_ids_hint());
        };
        let manifest = entry.manifest.clone();
        if !authorize(&manifest) {
            bail!("capabilities for plugin `{id}` were not approved");
        }
        let root = entry.root.clone();
        let data_dir = self.data_dir(id);
        let plugin = load_plugin(runtime, manifest, root, data_dir, window, cx)?;
        self.loaded.insert(id.to_owned(), plugin);

        Ok(())
    }

    /// Cancels work created under a plugin's policy, then drops its view and
    /// policy. Returns whether the plugin was loaded.
    ///
    /// There is no script `deactivate()` hook: host-owned task cancellation is
    /// deterministic and prevents owner-less work from retaining the unloaded
    /// plugin's authority.
    pub fn unload(&mut self, id: &str) -> bool {
        let Some(plugin) = self.loaded.remove(id) else {
            return false;
        };
        plugin.shutdown();
        true
    }

    pub fn loaded(&self) -> impl Iterator<Item = &Plugin> {
        self.loaded.values()
    }

    pub fn plugin(&self, id: &str) -> Option<&Plugin> {
        self.loaded.get(id)
    }

    /// Where a plugin's data lives.
    ///
    /// Keyed by `id`, not by path: an upgrade that replaces the plugin
    /// directory must not lose the user's data, and two checkouts of the same
    /// plugin are the same installation — which is the opposite of the rule for
    /// a directory run from the command line, where the path *is* the identity.
    pub fn data_dir(&self, id: &str) -> PathBuf {
        self.data_home.join("gpui-shell").join("plugins").join(id)
    }

    fn known_ids_hint(&self) -> String {
        let ids: Vec<&str> = self
            .catalog
            .iter()
            .map(|entry| entry.manifest.id())
            .collect();
        if ids.is_empty() {
            format!(
                "; no plugin was found in {}",
                self.directories
                    .iter()
                    .map(|directory| directory.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        } else {
            format!("; found {}", ids.join(", "))
        }
    }
}

impl Drop for PluginManager {
    fn drop(&mut self) {
        for (_, plugin) in std::mem::take(&mut self.loaded) {
            plugin.shutdown();
        }
    }
}

/// A configured directory is either one plugin or a directory of them.
///
/// Both are worth supporting and they are distinguishable without guessing: a
/// directory holding a manifest is a plugin, anything else is a container. That
/// is what makes `--plugin ~/dev/my-plugin` work alongside a user's installed
/// plugin folder, with no second flag to say which kind it is.
fn plugin_roots(directory: &Path) -> Vec<PathBuf> {
    if directory.join(MANIFEST_FILE).is_file() {
        return vec![directory.to_path_buf()];
    }

    let mut roots: Vec<PathBuf> = std::fs::read_dir(directory)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join(MANIFEST_FILE).is_file())
        .collect();

    // Directory order is filesystem-dependent; a plugin list that reorders
    // itself between runs is a bug report waiting to happen.
    roots.sort();
    roots
}

/// The platform's per-user data directory.
///
/// Duplicated from `src/bin/gpui-shell.rs` on purpose rather than shared: this
/// module owns no file but itself. It belongs in `runtime.rs` so the binary and
/// the plugin manager cannot disagree about where a user's data is — see the
/// report accompanying this module.
fn default_data_home() -> PathBuf {
    if let Some(explicit) = std::env::var_os("XDG_DATA_HOME").filter(|it| !it.is_empty()) {
        return PathBuf::from(explicit);
    }

    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);

    if cfg!(target_os = "macos") {
        home.join("Library").join("Application Support")
    } else if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("AppData").join("Roaming"))
    } else {
        home.join(".local").join("share")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{TestAppContext, VisualTestContext};
    use std::ops::Deref as _;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// A complete manifest, requiring the runtime this test binary is. The
    /// crate version moves with every release and a `0.x` line accepts only
    /// its own minor, so a fixture that spelled a version out would expire at
    /// the next bump.
    const VALID: &str = concat!(
        r#"{
        "id": "com.example.inbox",
        "name": "Inbox",
        "version": "1.2.0",
        "shell-version": ""#,
        env!("CARGO_PKG_VERSION"),
        r#"",
        "entry": "main.js",
        "capabilities": {
            "fs": {
                "read": ["${pluginDir}", "${dataDir}"],
                "write": ["${dataDir}"],
                "execute": ["git"]
            },
            "network": {
                "hosts": ["api.example.com"],
                "http": [{
                    "host": "readonly.example.com",
                    "methods": ["GET"],
                    "paths": ["/v1/account"],
                    "path_prefixes": ["/v1/quotes/"]
                }]
            },
            "storage": true,
            "clipboard": { "write": true },
            "process": { "exit": true }
        }
    }"#
    );

    /// A directory that removes itself. `tempfile` is not a dependency of this
    /// crate and one test module is not a reason to add one.
    struct TempTree(PathBuf);

    impl TempTree {
        fn new(label: &str) -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "gpui-shell-plugin-{label}-{}-{unique}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("cannot create a temporary directory");
            Self(path)
        }

        fn plugin(&self, directory: &str, manifest: &str) -> PathBuf {
            let root = self.0.join(directory);
            std::fs::create_dir_all(&root).expect("cannot create a plugin directory");
            std::fs::write(root.join(MANIFEST_FILE), manifest).expect("cannot write a manifest");
            root
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[gpui::test]
    fn default_runtime_global_does_not_retain_the_runtime(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let runtime = cx
            .update(ShellRuntime::new)
            .expect("default application runtime");
        let weak = Rc::downgrade(&runtime);

        drop(runtime);

        assert!(
            weak.upgrade().is_none(),
            "the App global must not keep an otherwise unowned runtime alive"
        );
        assert!(cx.update(|cx| ShellRuntime::global(cx)).is_none());

        let replacement = cx
            .update(ShellRuntime::new)
            .expect("a dead default runtime can be replaced");
        let installed = cx.update(|cx| ShellRuntime::global(cx).expect("replacement runtime"));
        assert!(Rc::ptr_eq(&replacement, &installed));
    }

    #[gpui::test]
    fn runtime_load_builds_the_window_root_without_plugin_manager_ceremony(
        cx: &mut TestAppContext,
    ) {
        cx.update(crate::init);
        let application = TempTree::new("direct-load");
        std::fs::write(
            application.path().join("main.js"),
            r#"
                import { div, View } from "gpui-kit";
                export default class App extends View {
                  render(cx) { return "loaded"; }
                }
            "#,
        )
        .expect("application source");

        let runtime = cx
            .update(ShellRuntime::new)
            .expect("default application runtime");
        let installed = cx.update(|cx| ShellRuntime::global(cx).expect("installed runtime"));
        assert!(Rc::ptr_eq(&runtime, &installed));
        let duplicate = cx.update(ShellRuntime::new);
        assert!(
            duplicate
                .as_ref()
                .is_err_and(|error| error.to_string().contains("new_isolated")),
            "constructing a second default runtime must not silently replace the first"
        );

        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);
        let root = context.update(|window, cx| runtime.load(application.path(), window, cx));
        context.update(|_, cx| {
            assert_eq!(root.read(cx).dialog_count(), 0);
            assert!(
                root.read(cx)
                    .content()
                    .clone()
                    .downcast::<ScriptView>()
                    .is_ok()
            );
        });
        let watch = context
            .update(|window, cx| runtime.watch(&root, window, cx))
            .expect("loaded root retains its source metadata");
        drop(watch);
        context
            .update(|_, cx| runtime.refresh(&root, cx))
            .expect("loaded root can be refreshed without exposing ScriptView");

        let other_runtime = ShellRuntime::new_isolated().expect("other runtime");
        let error = context
            .update(|_, cx| other_runtime.refresh(&root, cx))
            .expect_err("a runtime must not refresh another runtime's application");
        assert!(error.to_string().contains("different gpui-shell runtime"));
    }

    #[gpui::test]
    fn runtime_try_load_preserves_the_structured_script_error(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let application = TempTree::new("direct-try-load-error");
        std::fs::write(application.path().join("main.js"), "this is not javascript")
            .expect("broken source");

        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);
        let error = context
            .update(|window, cx| runtime.try_load(application.path(), window, cx))
            .expect_err("try_load must not replace a structured error with a view");
        assert!(
            error.to_string().contains("main.js"),
            "the source path must survive: {error:#}"
        );
    }

    #[gpui::test]
    fn runtime_check_returns_the_rendered_description(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let application = TempTree::new("direct-check");
        std::fs::write(
            application.path().join("main.js"),
            r#"
                import { div, View } from "gpui-kit";
                export default class App extends View {
                  render(cx) { return "checked through the facade"; }
                }
            "#,
        )
        .expect("application source");

        let runtime = ShellRuntime::new_isolated().expect("runtime");
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);
        let description = context
            .update(|window, cx| runtime.check(application.path(), window, cx))
            .expect("check application through the high-level facade");

        assert!(description.contains("checked through the facade"));
    }

    #[gpui::test]
    fn runtime_load_uses_the_manifest_entry_without_plugin_manager_ceremony(
        cx: &mut TestAppContext,
    ) {
        cx.update(crate::init);
        let application = TempTree::new("direct-manifest-load");
        std::fs::write(
            application.path().join(MANIFEST_FILE),
            r#"{
                "id": "com.example.direct-load",
                "name": "Direct load",
                "entry": "application.js",
                "capabilities": {
                    "network": { "hosts": ["api.example.com"] },
                    "process": { "exit": true }
                }
            }"#,
        )
        .expect("application manifest");
        std::fs::write(
            application.path().join("application.js"),
            r#"
                import { div, View } from "gpui-kit";
                export default class App extends View {
                  render() { return "manifest entry loaded"; }
                }
            "#,
        )
        .expect("application source");

        let runtime = cx
            .update(ShellRuntime::new)
            .expect("default application runtime");
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);
        let root = context.update(|window, cx| runtime.load(application.path(), window, cx));
        let policy = context.update(|_, cx| {
            let view = root
                .read(cx)
                .content()
                .clone()
                .downcast::<ScriptView>()
                .expect("manifest entry view");
            view.read(cx).policy()
        });
        assert!(
            !Rc::ptr_eq(&policy, &crate::policy::default()),
            "each loaded application needs its own task-cancellation identity"
        );
        assert!(
            !policy.capabilities().may_exit(),
            "a manifest requests capabilities; runtime.load must not approve them"
        );
        assert!(
            !policy.capabilities().may_reach("api.example.com"),
            "the host's default policy remains the permission ceiling"
        );
    }

    #[gpui::test]
    fn runtime_load_cancels_tasks_when_initialization_fails(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let application = TempTree::new("failed-load-tasks");
        std::fs::write(
            application.path().join("main.js"),
            r#"
                import { div, View } from "gpui-kit";
                export default class Broken extends View {
                  init(_props, cx) {
                    cx.timer.every(1_000, () => {});
                    throw new Error("initialization failed");
                  }
                  render(cx) { throw new Error("unreachable"); }
                }
            "#,
        )
        .expect("application source");

        let runtime = cx
            .update(ShellRuntime::new)
            .expect("default application runtime");
        let tasks_before = crate::engine::quickjs::task_count();
        let window = cx.add_window(|_, _| gpui::Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);
        let root = context.update(|window, cx| runtime.load(application.path(), window, cx));

        context.update(|_, cx| {
            assert!(
                root.read(cx)
                    .content()
                    .clone()
                    .downcast::<ScriptView>()
                    .is_err(),
                "a failed application must render the failure surface"
            );
        });
        assert_eq!(
            crate::engine::quickjs::task_count(),
            tasks_before,
            "tasks started before initialization failed must be cancelled"
        );
    }

    #[gpui::test]
    fn async_init_runs_under_the_plugin_policy_and_notifies_its_view(cx: &mut TestAppContext) {
        let plugins = TempTree::new("async-init");
        let data = TempTree::new("async-init-data");
        let manifest = r#"{
            "id": "com.example.async-init",
            "name": "Async Init",
            "version": "1.0.0",
            "entry": "main.js",
            "capabilities": {
                "fs": { "read": ["${pluginDir}"] }
            }
        }"#;
        let root = plugins.plugin("async-init", manifest);
        std::fs::write(root.join("message.txt"), "loaded through plugin policy")
            .expect("write fixture");
        std::fs::write(
            root.join("main.js"),
            r#"
                import { div, View } from "gpui-kit";
                import { v_flex } from "gpui-base";
                import * as fs from "fs/promises";
                export default class Panel extends View {
                  init(_props, cx) {
                    this.message = "pending";
                    cx.spawn(async (cx) => {
                      this.message = await fs.readFile("message.txt", "utf8");
                      cx.notify();
                    });
                  }
                  render(cx) { return v_flex().child(this.message); }
                }
            "#,
        )
        .expect("write script");

        cx.update(crate::init);
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        cx.update(|cx| runtime.set_global(cx));
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);
        let mut manager = PluginManager::new(vec![plugins.path().to_path_buf()])
            .with_data_home(data.path().to_path_buf());
        let undiscovered = context
            .update(|window, cx| {
                manager.load(&runtime, "com.example.async-init", |_| true, window, cx)
            })
            .expect_err("load must not hide discovery errors");
        assert!(
            undiscovered.to_string().contains("discover"),
            "{undiscovered:#}"
        );
        let discovered = manager.discover();
        assert!(
            discovered.iter().all(Result::is_ok),
            "test plugin must discover cleanly: {discovered:?}"
        );

        let denied = context
            .update(|window, cx| {
                manager.load(&runtime, "com.example.async-init", |_| false, window, cx)
            })
            .expect_err("an unapproved manifest must not execute");
        assert!(denied.to_string().contains("not approved"), "{denied:#}");
        assert_eq!(manager.loaded().count(), 0);

        context
            .update(|window, cx| {
                manager.load(&runtime, "com.example.async-init", |_| true, window, cx)
            })
            .expect("load plugin");
        let view = manager
            .plugin("com.example.async-init")
            .map(Plugin::view)
            .expect("plugin view")
            .clone();

        draw(&mut context, &view);
        assert!(snapshot_text(&mut context, &view).contains("pending"));
        context.run_until_parked();
        draw(&mut context, &view);

        let settled = snapshot_text(&mut context, &view);
        assert!(
            settled.contains("loaded through plugin policy"),
            "async init did not invalidate its own view: {settled}"
        );
    }

    #[gpui::test]
    fn unloading_a_plugin_releases_its_entities_and_subscriptions(cx: &mut TestAppContext) {
        let plugins = TempTree::new("unload-entities");
        let data = TempTree::new("unload-entities-data");
        let manifest = r#"{
            "id": "com.example.retained-input",
            "name": "Retained input",
            "entry": "main.js"
        }"#;
        let root = plugins.plugin("retained-input", manifest);
        std::fs::write(
            root.join("main.js"),
            r#"
                import { div, View } from "gpui-kit";
                import { Input, InputState } from "gpui-base";
                export default class Panel extends View {
                  init() {
                    this.field = InputState.new({ value: "owned by plugin" });
                    this.field.on("change", () => {});
                  }
                  render(cx) { return Input.new(this.field); }
                }
            "#,
        )
        .expect("write script");

        cx.update(crate::init);
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        cx.update(|cx| runtime.set_global(cx));
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);
        let mut manager = PluginManager::new(vec![plugins.path().to_path_buf()])
            .with_data_home(data.path().to_path_buf());
        assert!(manager.discover().iter().all(Result::is_ok));

        context
            .update(|window, cx| {
                manager.load(&runtime, "com.example.retained-input", |_| true, window, cx)
            })
            .expect("load plugin");
        assert_eq!(runtime.entities().len(), 1);

        assert!(manager.unload("com.example.retained-input"));
        assert!(
            runtime.entities().is_empty(),
            "unload must drop the input record and its GPUI subscriptions"
        );
    }

    #[gpui::test]
    fn dropping_plugin_manager_shuts_down_loaded_plugins(cx: &mut TestAppContext) {
        let plugins = TempTree::new("drop-manager");
        let data = TempTree::new("drop-manager-data");
        let manifest = r#"{
            "id": "com.example.drop-manager",
            "name": "Drop manager",
            "entry": "main.js"
        }"#;
        let root = plugins.plugin("drop-manager", manifest);
        std::fs::write(
            root.join("main.js"),
            r#"
                import { div, View } from "gpui-kit";
                import { Input, InputState } from "gpui-base";
                export default class Panel extends View {
                  init(_props, cx) {
                    this.field = InputState.new({ value: "owned by plugin" });
                    cx.timer.every(60_000, () => {});
                  }
                  render(cx) { return Input.new(this.field); }
                }
            "#,
        )
        .expect("write script");

        cx.update(crate::init);
        let runtime = ShellRuntime::new_isolated().expect("runtime");
        cx.update(|cx| runtime.set_global(cx));
        let tasks_before = crate::engine::quickjs::task_count();
        let window = cx.add_window(|_, _| Empty);
        let mut context = VisualTestContext::from_window(*window.deref(), cx);
        let mut manager = PluginManager::new(vec![plugins.path().to_path_buf()])
            .with_data_home(data.path().to_path_buf());
        assert!(manager.discover().iter().all(Result::is_ok));

        context
            .update(|window, cx| {
                manager.load(&runtime, "com.example.drop-manager", |_| true, window, cx)
            })
            .expect("load plugin");
        let retained_view = manager
            .plugin("com.example.drop-manager")
            .expect("loaded plugin")
            .view()
            .clone();
        assert_eq!(runtime.entities().len(), 1);
        assert_eq!(crate::engine::quickjs::task_count(), tasks_before + 1);

        drop(manager);

        assert!(
            runtime.entities().is_empty(),
            "dropping the manager must release application-owned entities even while GPUI retains the view"
        );
        assert_eq!(
            crate::engine::quickjs::task_count(),
            tasks_before,
            "dropping the manager must cancel application-owned tasks"
        );
        drop(retained_view);
    }

    fn draw(context: &mut VisualTestContext, view: &Entity<ScriptView>) {
        let view = view.clone();
        context.draw(
            gpui::Point::default(),
            gpui::size(gpui::px(400.), gpui::px(300.)),
            move |_, _| view.into_any_element(),
        );
    }

    fn snapshot_text(context: &mut VisualTestContext, view: &Entity<ScriptView>) -> String {
        context.update(|_, cx| {
            view.read(cx)
                .snapshot()
                .map(crate::RenderSnapshot::debug_tree)
                .unwrap_or_default()
        })
    }

    struct Empty;

    impl gpui::Render for Empty {
        fn render(
            &mut self,
            _: &mut gpui::Window,
            _: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            gpui::div()
        }
    }

    #[test]
    fn a_valid_manifest_reads_back_what_was_written() {
        let manifest = PluginManifest::parse(VALID).expect("the manifest should parse");

        assert_eq!(manifest.id(), "com.example.inbox");
        assert_eq!(manifest.name(), "Inbox");
        assert_eq!(manifest.version(), "1.2.0");
        assert_eq!(manifest.shell_version(), SHELL_VERSION);
        assert_eq!(manifest.entry(), "main.js");

        let capabilities =
            manifest.capabilities(Path::new("/plugins/inbox"), Path::new("/data/inbox"));
        assert!(capabilities.has_storage());
        assert!(capabilities.is_clipboard_writable());
        assert!(!capabilities.is_clipboard_readable());
        assert!(capabilities.may_exit());
    }

    #[test]
    fn a_manifest_accepts_the_omarchy_ui_https_remote_without_dot_git() {
        let source = r#"{
            "id": "com.example.third-party-ui",
            "name": "Third-party UI",
            "entry": "main.js",
            "dependencies": {
                "omarchy-ui": {
                    "git": "https://github.com/huacnlee/omarchy-ui",
                    "branch": "main"
                }
            }
        }"#;

        let manifest =
            PluginManifest::parse(source).expect("a branch-pinned Git dependency should parse");
        let dependency = &manifest.dependencies()["omarchy-ui"];
        assert_eq!(dependency.git(), "https://github.com/huacnlee/omarchy-ui");
        assert_eq!(dependency.branch(), Some("main"));
        assert_eq!(dependency.tag(), None);
        assert_eq!(dependency.entry(), "index.js");
    }

    #[test]
    fn a_manifest_accepts_git_dependency_strings() {
        for (source, expected_git, expected_reference) in [
            (
                "https://github.com/huacnlee/omarchy-ui#main",
                "https://github.com/huacnlee/omarchy-ui",
                Some("main"),
            ),
            (
                "https://github.com/huacnlee/omarchy-ui#v1.2.0",
                "https://github.com/huacnlee/omarchy-ui",
                Some("v1.2.0"),
            ),
            (
                "https://github.com/huacnlee/omarchy-ui#0123456789abcdef0123456789abcdef01234567",
                "https://github.com/huacnlee/omarchy-ui",
                Some("0123456789abcdef0123456789abcdef01234567"),
            ),
            (
                "https://github.com/huacnlee/omarchy-ui",
                "https://github.com/huacnlee/omarchy-ui",
                None,
            ),
        ] {
            let manifest = PluginManifest::parse(&format!(
                r#"{{
                    "id": "com.example.third-party-ui",
                    "name": "Third-party UI",
                    "entry": "main.js",
                    "dependencies": {{ "omarchy-ui": {} }}
                }}"#,
                serde_json::to_string(source).expect("dependency as JSON")
            ))
            .expect("a Git dependency string should parse");
            let dependency = &manifest.dependencies()["omarchy-ui"];

            assert_eq!(dependency.git(), expected_git);
            assert_eq!(dependency.reference(), expected_reference);
            assert!(dependency.uses_package_entry());
        }
    }

    #[test]
    fn github_shorthand_expands_to_an_https_remote_and_defaults_to_main() {
        for (source, expected_reference) in [
            ("huacnlee/omarchy-ui", "main"),
            ("huacnlee/omarchy-ui#stable", "stable"),
        ] {
            let manifest = PluginManifest::parse(&format!(
                r#"{{
                    "id": "com.example.third-party-ui",
                    "name": "Third-party UI",
                    "entry": "main.js",
                    "dependencies": {{ "omarchy-ui": {} }}
                }}"#,
                serde_json::to_string(source).expect("dependency as JSON")
            ))
            .expect("strict GitHub shorthand should parse");
            let dependency = &manifest.dependencies()["omarchy-ui"];

            assert_eq!(dependency.git(), "https://github.com/huacnlee/omarchy-ui");
            assert_eq!(dependency.reference(), Some(expected_reference));
            assert!(dependency.uses_package_entry());
        }
    }

    #[test]
    fn malformed_or_ambiguous_git_dependency_strings_are_rejected() {
        for source in [
            "huacnlee",
            "/omarchy-ui",
            "huacnlee/",
            "huacnlee/omarchy-ui/extra",
            "huacnlee//omarchy-ui",
            "huacnlee/omarchy-ui#",
            "https://github.com/huacnlee/omarchy-ui#",
            "not a git dependency",
        ] {
            let error = PluginManifest::parse(&format!(
                r#"{{
                    "id": "com.example.third-party-ui",
                    "name": "Third-party UI",
                    "entry": "main.js",
                    "dependencies": {{ "omarchy-ui": {} }}
                }}"#,
                serde_json::to_string(source).expect("dependency as JSON")
            ))
            .expect_err("malformed string syntax must not select an unintended repository");

            assert!(
                error.to_string().contains("Git URL")
                    || error.to_string().contains("owner/repository")
                    || error.to_string().contains("Git ref"),
                "`{source}` produced an unclear error: {error:#}"
            );
        }
    }

    #[test]
    fn object_git_dependencies_keep_their_explicit_selector_and_entry_contract() {
        let source = r#"{
            "id": "com.example.third-party-ui",
            "name": "Third-party UI",
            "entry": "main.js",
            "dependencies": {
                "omarchy-ui": {
                    "git": "https://github.com/huacnlee/omarchy-ui",
                    "tag": "v1.2.0",
                    "entry": "src/public.js"
                }
            }
        }"#;

        let manifest = PluginManifest::parse(source).expect("the legacy object form should parse");
        let dependency = &manifest.dependencies()["omarchy-ui"];

        assert_eq!(dependency.branch(), None);
        assert_eq!(dependency.tag(), Some("v1.2.0"));
        assert_eq!(dependency.entry(), "src/public.js");
        assert!(!dependency.uses_package_entry());
    }

    #[test]
    fn a_manifest_accepts_a_git_dependency_pinned_to_a_tag() {
        let source = r#"{
            "id": "com.example.third-party-ui",
            "name": "Third-party UI",
            "entry": "main.js",
            "dependencies": {
                "omarchy-ui": {
                    "git": "ssh://git@example.com/omarchy-ui.git",
                    "tag": "v1.2.0",
                    "entry": "src/public.js"
                }
            }
        }"#;

        let manifest =
            PluginManifest::parse(source).expect("a tag-pinned Git dependency should parse");
        let dependency = &manifest.dependencies()["omarchy-ui"];
        assert_eq!(dependency.branch(), None);
        assert_eq!(dependency.tag(), Some("v1.2.0"));
        assert_eq!(dependency.entry(), "src/public.js");
    }

    #[test]
    fn a_git_dependency_cannot_select_a_branch_and_a_tag() {
        let source = r#"{
            "id": "com.example.third-party-ui",
            "name": "Third-party UI",
            "entry": "main.js",
            "dependencies": {
                "omarchy-ui": {
                    "git": "https://github.com/example/omarchy-ui.git",
                    "branch": "main",
                    "tag": "v1.2.0"
                }
            }
        }"#;

        let error = PluginManifest::parse(source).expect_err("the ref is ambiguous");
        assert!(error.to_string().contains("either `branch` or `tag`"));
    }

    #[test]
    fn a_git_dependency_entry_cannot_escape_its_checkout() {
        let source = r#"{
            "id": "com.example.third-party-ui",
            "name": "Third-party UI",
            "entry": "main.js",
            "dependencies": {
                "omarchy-ui": {
                    "git": "https://github.com/example/omarchy-ui.git",
                    "tag": "v1.2.0",
                    "entry": "../private.js"
                }
            }
        }"#;

        let error = PluginManifest::parse(source).expect_err("the entry escapes the checkout");
        assert!(error.to_string().contains("path inside the Git repository"));
    }

    #[test]
    fn a_git_dependency_cannot_use_a_runtime_module_name() {
        let source = r#"{
            "id": "com.example.shadow",
            "name": "Shadow",
            "entry": "main.js",
            "dependencies": {
                "gpui-kit": {
                    "git": "https://github.com/example/not-gpui.git",
                    "branch": "main"
                }
            }
        }"#;

        let error = PluginManifest::parse(source).expect_err("the dependency is unreachable");
        assert!(error.to_string().contains("reserved by gpui-shell"));
    }

    #[test]
    fn a_git_dependency_rejects_refspec_syntax_in_a_branch() {
        let source = r#"{
            "id": "com.example.refspec",
            "name": "Refspec",
            "entry": "main.js",
            "dependencies": {
                "omarchy-ui": {
                    "git": "https://github.com/example/omarchy-ui.git",
                    "branch": "main:refs/heads/injected"
                }
            }
        }"#;

        let error = PluginManifest::parse(source).expect_err("a selector is not a refspec");
        assert!(error.to_string().contains("valid Git ref name"));
    }

    #[test]
    fn shell_version_is_declared_in_the_manifest_before_script_runs() {
        let manifest = PluginManifest::parse(VALID)
            .expect("the runtime version belongs in inert manifest metadata");
        assert_eq!(manifest.shell_version(), SHELL_VERSION);
    }

    #[test]
    fn an_omitted_shell_version_accepts_the_current_runtime() {
        let source = VALID.replace(
            &format!("        \"shell-version\": \"{SHELL_VERSION}\",\n"),
            "",
        );
        let manifest = PluginManifest::parse(&source)
            .expect("omitting shell-version supports the runtime loading the application");
        assert_eq!(manifest.shell_version(), SHELL_VERSION);
    }

    #[test]
    fn an_omitted_application_version_is_reported_as_unknown() {
        let source = VALID.replace("        \"version\": \"1.2.0\",\n", "");
        let manifest = PluginManifest::parse(&source).expect("version metadata is optional");
        assert_eq!(manifest.version(), "unknown");
    }

    #[test]
    fn capabilities_become_a_real_grant() {
        let manifest = PluginManifest::parse(VALID).expect("the manifest should parse");
        // Real directories, because a grant is an open handle now: the
        // placeholders still resolve to paths, but a path is only half of one.
        let base = std::env::temp_dir().join(format!("gpui-shell-grant-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let plugin_dir = base.join("plugins/inbox");
        let data_dir = base.join("data/inbox");
        std::fs::create_dir_all(&plugin_dir).expect("a plugin directory");
        std::fs::create_dir_all(&data_dir).expect("a data directory");
        let capabilities = manifest.capabilities(&plugin_dir, &data_dir);

        assert_eq!(
            capabilities.execute_grant(),
            &ExecuteGrant::Allowed(vec!["git".to_owned()])
        );
        assert!(capabilities.may_run("git"));
        assert!(!capabilities.may_run("curl"));
        assert!(capabilities.may_reach("api.example.com"));
        assert!(!capabilities.may_reach("evil.example.com"));
        assert!(capabilities.may_request(
            "https",
            "readonly.example.com",
            None,
            "GET",
            "/v1/account"
        ));
        assert!(capabilities.may_request(
            "https",
            "readonly.example.com",
            None,
            "GET",
            "/v1/quotes/AAPL.US"
        ));
        assert!(!capabilities.may_request(
            "https",
            "readonly.example.com",
            None,
            "POST",
            "/v1/account"
        ));

        // The placeholders are the only way a manifest can name a directory it
        // does not know the path of.
        assert_eq!(
            capabilities
                .open(Path::new("main.js"), crate::capability::Access::Read)
                .expect("the plugin directory should be readable")
                .path(),
            Path::new("main.js")
        );
        assert_eq!(
            capabilities
                .open(
                    &data_dir.join("items.json"),
                    crate::capability::Access::Write
                )
                .expect("the data directory should be writable")
                .path(),
            Path::new("items.json")
        );
        assert!(
            capabilities
                .open(Path::new("/etc/passwd"), crate::capability::Access::Read)
                .is_err()
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Everything that reaches outside the application is denied; its own
    /// storage is not, on the same reasoning a browser gives every origin a
    /// `localStorage` without asking.
    #[test]
    fn an_absent_capabilities_block_grants_nothing_but_storage() {
        let manifest = PluginManifest::parse(
            r#"{"id": "a.b", "name": "B", "version": "0.1.0", "entry": "main.js"}"#,
        )
        .expect("capabilities may be omitted");

        let capabilities = manifest.capabilities(Path::new("/plugins/b"), Path::new("/data/b"));
        assert!(capabilities.has_storage());
        assert!(!capabilities.has_read_access());
        assert!(!capabilities.has_write_access());
        assert_eq!(capabilities.execute_grant(), &ExecuteGrant::Denied);
    }

    /// The point of keeping storage a capability rather than making it ambient
    /// like `sessionStorage`: a host running code it does not trust can still
    /// refuse, and refusing has to actually work.
    #[test]
    fn a_manifest_may_still_refuse_storage() {
        let manifest = PluginManifest::parse(
            r#"{"id": "a.b", "name": "B", "version": "0.1.0",
                "entry": "main.js", "capabilities": {"storage": false}}"#,
        )
        .expect("storage may be declined");

        assert!(
            !manifest
                .capabilities(Path::new("/plugins/b"), Path::new("/data/b"))
                .has_storage()
        );
    }

    /// The trap this default exists to close: declaring one unrelated grant
    /// must not silently cost an application its settings.
    #[test]
    fn declaring_another_grant_does_not_cost_an_application_its_storage() {
        let manifest = PluginManifest::parse(
            r#"{"id": "a.b", "name": "B", "version": "0.1.0",
                "entry": "main.js", "capabilities": {"network": {"hosts": ["example.com"]}}}"#,
        )
        .expect("a network grant is legal on its own");

        assert!(
            manifest
                .capabilities(Path::new("/plugins/b"), Path::new("/data/b"))
                .has_storage()
        );
    }

    #[test]
    fn each_missing_field_names_itself() {
        for field in ["id", "name", "entry"] {
            let mut fields: serde_json::Map<String, serde_json::Value> =
                serde_json::from_str(VALID).expect("the fixture should be an object");
            fields.remove(field);
            let source = serde_json::to_string(&fields).expect("re-encoding cannot fail");

            let error = PluginManifest::parse(&source).expect_err("the field is required");
            assert_eq!(error.problem(), &ManifestProblem::MissingField(field));

            let message = error.to_string();
            assert!(
                message.contains(&format!("`{field}`")),
                "the message must name the field: {message}"
            );
            assert!(
                message.contains("expected"),
                "the message must say what was expected: {message}"
            );
        }
    }

    #[test]
    fn a_field_of_the_wrong_type_says_which_and_what() {
        let error = PluginManifest::parse(
            r#"{"id": "a.b", "name": 7, "version": "0.1.0", "entry": "main.js"}"#,
        )
        .expect_err("a numeric name is not a name");

        assert_eq!(
            error.problem(),
            &ManifestProblem::WrongType {
                field: "name",
                found: "a number"
            }
        );
        assert!(error.to_string().contains("must be a string"));
    }

    #[test]
    fn a_typo_in_capabilities_is_refused_rather_than_granting_nothing() {
        let source = VALID.replacen("\"capabilities\"", "\"capabilites\"", 1);
        let error = PluginManifest::parse(&source).expect_err("an unknown field is not accepted");

        assert_eq!(
            error.problem(),
            &ManifestProblem::UnknownField {
                field: "capabilites".to_owned(),
                suggestion: Some("capabilities"),
            }
        );
        let message = error.to_string();
        assert!(
            message.contains("did you mean `capabilities`?"),
            "{message}"
        );
    }

    #[test]
    fn a_typo_inside_the_capabilities_block_is_refused_too() {
        let source = VALID.replacen("\"network\"", "\"netwrok\"", 1);
        let error = PluginManifest::parse(&source).expect_err("an unknown grant is not accepted");

        let message = error.to_string();
        assert!(message.contains("netwrok"), "{message}");
        assert!(message.contains("invalid `capabilities`"), "{message}");
    }

    #[test]
    fn malformed_http_grants_are_rejected_at_manifest_parse_time() {
        for (needle, replacement, expected) in [
            (
                "api.example.com",
                "https://api.example.com",
                "without a scheme",
            ),
            ("GET", "GETT", "invalid HTTP method"),
            ("/v1/account", "v1/account", "must start with `/`"),
        ] {
            let source = VALID.replacen(needle, replacement, 1);
            let error = PluginManifest::parse(&source).expect_err("invalid grant must fail fast");
            assert!(error.to_string().contains(expected), "{error}");
        }

        let source = VALID.replacen(
            "\"host\": \"readonly.example.com\"",
            "\"scheme\": \"ftp\", \"host\": \"readonly.example.com\"",
            1,
        );
        let error = PluginManifest::parse(&source).expect_err("invalid scheme must fail fast");
        assert!(error.to_string().contains("must be `http` or `https`"));
    }

    #[test]
    fn a_contribution_declared_in_the_manifest_is_refused() {
        // The rule the manifest exists to hold: contributions live in script.
        let source = VALID.replacen("\"name\"", "\"contributes\"", 1);
        let error = PluginManifest::parse(&source).expect_err("`contributes` is not a field");

        let message = error.to_string();
        assert!(message.contains("registered in script"), "{message}");
    }

    #[test]
    fn an_id_that_could_leave_its_data_directory_is_refused() {
        for id in [
            "../escape",
            "com.Example.Inbox",
            "a/b",
            "..",
            ".hidden",
            "inbox.",
        ] {
            let source = VALID.replacen("com.example.inbox", id, 1);
            assert!(
                PluginManifest::parse(&source).is_err(),
                "`{id}` must be refused: it becomes a directory name and a panel prefix"
            );
        }
    }

    #[test]
    fn an_invalid_id_explains_the_rule() {
        let source = VALID.replacen("com.example.inbox", "../escape", 1);
        let error = PluginManifest::parse(&source).expect_err("an id may not contain a separator");
        assert!(matches!(error.problem(), ManifestProblem::InvalidId { .. }));
        assert!(error.to_string().contains("script:<id>/<panel>"));
    }

    #[test]
    fn a_version_must_be_comparable() {
        let source = VALID.replacen("\"1.2.0\"", "\"latest\"", 1);
        let error = PluginManifest::parse(&source).expect_err("`latest` is not a version");
        assert_eq!(
            error.problem(),
            &ManifestProblem::InvalidVersion("latest".to_owned())
        );

        for version in ["1.2.0", "0.0.1", "1.2.0-beta.1", "2.0.0+build.5"] {
            let source = VALID.replacen("\"1.2.0\"", &format!("\"{version}\""), 1);
            assert!(
                PluginManifest::parse(&source).is_ok(),
                "`{version}` should parse"
            );
        }
    }

    #[test]
    fn an_entry_outside_the_plugin_directory_is_refused() {
        let source = VALID.replacen("\"main.js\"", "\"../../etc/main.js\"", 1);
        let error = PluginManifest::parse(&source).expect_err("an entry may not escape");
        assert!(matches!(error.problem(), ManifestProblem::InvalidEntry(_)));
    }

    #[test]
    fn an_unexpanded_placeholder_is_caught_before_it_becomes_a_directory() {
        let source = VALID.replacen("${dataDir}", "${homeDir}", 1);
        let error = PluginManifest::parse(&source).expect_err("`${homeDir}` does not exist");
        assert_eq!(
            error.problem(),
            &ManifestProblem::UnknownPlaceholder {
                field: "capabilities.fs.read".to_owned(),
                placeholder: "${homeDir}".to_owned(),
            }
        );
    }

    #[test]
    fn an_unrestricted_execute_grant_is_spelled_differently_from_an_allowlist() {
        let source = VALID.replacen("[\"git\"]", "\"*\"", 1);
        let manifest = PluginManifest::parse(&source).expect("`*` is a valid execute grant");
        let capabilities = manifest.capabilities(Path::new("/plugins/a"), Path::new("/data/a"));
        assert_eq!(capabilities.execute_grant(), &ExecuteGrant::Unrestricted);
        assert!(capabilities.may_run("anything"));
    }

    #[test]
    fn a_manifest_read_from_disk_carries_its_path() {
        let tree = TempTree::new("read");
        let root = tree.plugin("inbox", VALID);

        let manifest = PluginManifest::read(&root).expect("the manifest should parse");
        assert_eq!(manifest.id(), "com.example.inbox");

        let broken = tree.plugin("broken", "{ \"id\": \"a.b\" }");
        let error = PluginManifest::read(&broken).expect_err("`name` is missing");
        assert_eq!(error.path(), Some(broken.join(MANIFEST_FILE).as_path()));
        assert!(error.to_string().contains(MANIFEST_FILE));
    }

    #[test]
    fn discovery_returns_the_broken_manifest_beside_the_good_one() {
        let tree = TempTree::new("discover");
        tree.plugin("a-good", VALID);
        tree.plugin("b-broken", "{ \"id\": \"com.example.broken\" }");
        // Not a plugin: no manifest, so it is not reported at all.
        std::fs::create_dir_all(tree.path().join("c-not-a-plugin"))
            .expect("cannot create a directory");

        let mut manager = PluginManager::new(vec![tree.path().to_path_buf()]);
        let results = manager.discover();

        assert_eq!(results.len(), 2, "{results:?}");
        assert_eq!(
            results[0].as_ref().expect("the first is valid").id(),
            "com.example.inbox"
        );
        let error = results[1].as_ref().expect_err("the second is broken");
        assert_eq!(error.problem(), &ManifestProblem::MissingField("name"));

        // Only the readable manifest reached the catalog, and nothing ran.
        let available: Vec<&str> = manager.available().map(PluginManifest::id).collect();
        assert_eq!(available, vec!["com.example.inbox"]);
        assert_eq!(manager.loaded().count(), 0);
    }

    #[test]
    fn a_directory_that_is_itself_a_plugin_is_discovered() {
        let tree = TempTree::new("single");
        let root = tree.plugin("inbox", VALID);

        let mut manager = PluginManager::new(vec![root]);
        let results = manager.discover();
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
    }

    #[test]
    fn the_same_id_in_two_directories_is_reported_once_and_refused_once() {
        let first = TempTree::new("dup-first");
        let second = TempTree::new("dup-second");
        first.plugin("inbox", VALID);
        second.plugin("inbox", VALID);

        let mut manager = PluginManager::new(vec![
            first.path().to_path_buf(),
            second.path().to_path_buf(),
        ]);
        let results = manager.discover();

        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(
            matches!(
                results[1]
                    .as_ref()
                    .expect_err("the copy is refused")
                    .problem(),
                ManifestProblem::DuplicateId { .. }
            ),
            "the second copy must not win silently"
        );
        assert_eq!(manager.available().count(), 1);
    }

    #[test]
    fn storage_is_keyed_by_id_not_by_path() {
        let manager =
            PluginManager::new(Vec::new()).with_data_home(PathBuf::from("/home/user/.local/share"));
        assert_eq!(
            manager.data_dir("com.example.inbox"),
            PathBuf::from("/home/user/.local/share/gpui-shell/plugins/com.example.inbox")
        );
    }

    #[test]
    fn the_schema_type_accepts_exactly_what_the_parser_accepts() {
        // Two readers of one file: `schemars` describes `ManifestFile`, while
        // `parse` walks the fields by hand to explain each failure. This is the
        // check that they still describe the same file.
        let described: ManifestFile =
            serde_json::from_str(VALID).expect("the schema type must accept a valid manifest");
        let parsed = PluginManifest::parse(VALID).expect("and so must the parser");

        assert_eq!(described.id, parsed.id);
        assert_eq!(described.name, parsed.name);
        assert_eq!(
            described.version.as_deref().unwrap_or("unknown"),
            parsed.version
        );
        assert_eq!(
            described.shell_version.as_deref().unwrap_or(SHELL_VERSION),
            parsed.shell_version
        );
        assert_eq!(described.entry, parsed.entry);
        assert_eq!(described.dependencies, parsed.dependencies);
        assert_eq!(described.capabilities, parsed.capabilities);
    }

    #[test]
    fn the_schema_describes_every_field() {
        let schema = manifest_schema().to_string();
        for field in FIELDS {
            assert!(schema.contains(field), "the schema must mention `{field}`");
        }
        for grant in [
            "fs",
            "read",
            "write",
            "execute",
            "network",
            "hosts",
            "http",
            "scheme",
            "port",
            "methods",
            "paths",
            "path_prefixes",
            "storage",
            "clipboard",
            "process",
            "exit",
        ] {
            assert!(
                schema.contains(grant),
                "the schema must mention `{grant}` inside capabilities"
            );
        }
        // A seventh field must be a schema violation, not a silently ignored key.
        assert!(
            schema.contains("additionalProperties"),
            "the schema must refuse unknown fields as the parser does"
        );
    }

    #[test]
    fn incompatible_shell_versions_are_rejected_before_discovery() {
        let runtime = Version::parse(SHELL_VERSION).expect("the crate version is semantic");
        let next_minor = Version::new(runtime.major, runtime.minor + 1, 0);
        let next_major = Version::new(runtime.major + 1, 0, 0);
        for required in [next_minor, next_major] {
            let source = VALID.replacen(
                &format!("\"{SHELL_VERSION}\""),
                &format!("\"{required}\""),
                1,
            );
            let error = PluginManifest::parse(&source).expect_err("incompatible shell version");
            assert!(
                matches!(
                    error.problem(),
                    ManifestProblem::IncompatibleShellVersion { .. }
                ),
                "{error}"
            );
        }
    }

    #[test]
    fn older_shell_minor_versions_remain_compatible() {
        let source = VALID.replacen("\"0.1.0\"", "\"0.0.0\"", 1);
        PluginManifest::parse(&source)
            .expect("a manifest's shell-version is its minimum runtime version");
    }

    #[test]
    fn malformed_shell_versions_are_errors_not_panics() {
        for required in ["0.1.0-", "00.1.0", "0.1.184467440737095516160", "latest"] {
            let source = VALID.replacen(
                &format!("\"{SHELL_VERSION}\""),
                &format!("\"{required}\""),
                1,
            );
            let error = PluginManifest::parse(&source).expect_err("invalid semantic version");
            assert_eq!(
                error.problem(),
                &ManifestProblem::InvalidShellVersion(required.to_owned())
            );
        }
    }
}

//! TypeScript declarations for the script API (design doc §14.4).
//!
//! An application written against this runtime is JavaScript, so the only
//! checking it gets before it runs is whatever an editor can infer. A `.d.ts`
//! turns that into a real contract: completion for a surface no one memorizes,
//! and `// @ts-check` catching a mistyped style name, a color token that does
//! not exist, or `.p("auto")` — which the runtime rejects — at the call site.
//! It is also the form in which the API can be handed to a model, which is an
//! explicit audience here.
//!
//! # One module per crate
//!
//! The declarations include `"gpui-kit"` for GPUI's own
//! elements and what this runtime adds, `"gpui-base"` for gpui-base's layout
//! helpers, components and theme, and `"gpui-fps"` for its performance overlay.
//! `"gpui"` is a compatibility alias for `"gpui-kit"`.
//!
//! That is a contract about provenance rather than a filing convenience. An
//! import line says which layer a script depends on, so a script that never
//! reaches for a component says so, and the next layer to arrive —
//! `gpui-component`, whose components are the reason the seam exists — needs a
//! list and a `declare module`, not a renaming of everything already here.
//! Apart from the explicit `"gpui"` alias, names are not re-exported for
//! convenience, preserving the layer named by each import.
//!
//! The dependency runs upward only. `"gpui-base"` names what it borrows from
//! `"gpui-kit"` in an import at the top of its block; `"gpui-kit"` refers down to a
//! component type only where one shared element prototype forces it — three
//! builder methods and `cx.theme()` — and does it with an inline
//! `import("gpui-base").X` rather than a top-level import.
//!
//! # Why these declarations can be trusted
//!
//! They are *generated from the tables the runtime dispatches through*, not
//! transcribed from documentation:
//!
//! * The style methods come from [`style::known_names`] — the same list the
//!   JS prelude loops over when it builds the element prototype. A method GPUI
//!   adds upstream appears here without anyone writing it down, and a name that
//!   type-checks is a name the dispatcher will accept.
//! * Their documentation is GPUI's own, carried on the same reflection entries,
//!   so hovering `.items_center()` shows the sentence upstream wrote rather than
//!   one transcribed here. The seventy-odd methods bound by hand — reflection
//!   reaches no-argument methods only — carry a description written beside the
//!   name in [`style`], which is where a description that stops matching shows
//!   up in the same diff as the change.
//! * A parametric method's argument type is *probed*: [`argument_of`] asks
//!   [`style::apply_param`] which literals it accepts, so the difference
//!   between `Length`, `DefiniteLength`, `AbsoluteLength`, a color and a bare
//!   number is decided by the code that enforces it rather than by a second
//!   hand-written table that could disagree with the first.
//! * The color union comes from gpui-base's semantic token field names, so a mistyped
//!   token is a type error, and the phase union comes from [`ScopePhase`]
//!   itself.
//!
//! # What they deliberately do not cover
//!
//! * **Capabilities.** Every `fs`, storage, `clipboard` and `process` call
//!   type-checks; whether it is *granted* is a manifest question answered at
//!   run time (§19.2). Types cannot express a grant.
//! * **Element lifetime.** An element is consumed when it is used and belongs
//!   to one render pass; so does the `cx` handed to `render`. TypeScript has no
//!   affine types, so reusing an element still type-checks and still throws.
//! * **Which methods suit which component.** Every element shares one
//!   prototype, so `.checked(true)` is declared on all of them and is simply
//!   inert on a `div`. Narrowing that would mean inventing a type hierarchy the
//!   runtime does not have.
//! * **Retained entities** ([`crate::entities`]) and anything else no built-in
//!   module exports today.
//!
//! # What the host adds
//!
//! Every module the host registered is emitted here too, one `declare module`
//! per name, so `import { quotes } from "market"` is checked the same way
//! `import { div } from "gpui-kit"` is. A module that described itself in
//! TypeScript through [`crate::HostModule::declarations`] is emitted verbatim;
//! one that did not gets `(...args: any[]) => any` signatures, which still
//! check the module name and every export name.
//!
//! That is why the declarations live in Rust beside the registration rather
//! than in a `.d.ts` beside the script: two files describing one boundary
//! drift, and this one is checked against the registry before it is written.
//! `install_host_modules` in `crates/story/src/stories/shell_story.rs` is the
//! worked example.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use gpui::StyleRefinement;

use crate::a11y;
use crate::scope::ScopePhase;
use crate::style;
use crate::theme_tokens::color_token_names;
use crate::value::Bridged;

/// The declaration filename. Fixed because an editor finds the declarations by
/// having them in the project, not by being told where.
pub const FILE_NAME: &str = "gpui-kit.d.ts";

/// The editor configuration filename, in the spelling a JavaScript project uses.
pub const CONFIG_FILE_NAME: &str = "jsconfig.json";

/// The configuration this one is written beside, and defers to.
const TYPESCRIPT_CONFIG_FILE_NAME: &str = "tsconfig.json";

/// What an editor has to be told before `gpui-kit.d.ts` and the linked packages
/// mean anything.
///
/// The settings match the ones the applications in this repository were
/// written against — `examples/js_todolist/jsconfig.json` and its siblings —
/// so a generated project and a hand-written one are checked the same way.
/// The file explains each of them to whoever opens it, which is why it carries
/// its own `"// why"`.
///
/// It is only ever written into a directory that has no configuration at all,
/// so `checkJs` turns checking on for an application that had none rather than
/// changing the terms under an application that already chose.
const EDITOR_CONFIG: &str = r#"{
  "// why": [
    "Written once by gpui-shell, then yours: an existing jsconfig.json or",
    "tsconfig.json is never replaced, and this file is not rewritten.",
    "",
    "`moduleResolution` is how a bare specifier is answered. Left to be",
    "inferred it can still land on the resolution that never looks in",
    "node_modules, and a Git dependency the runtime resolves fine is",
    "underlined as a module the editor cannot find.",
    "",
    "`lib` decides which globals exist. The default hands a script the",
    "browser's — a `console`, a `localStorage`, a `Window` this runtime does",
    "not have — and their declarations collide with the ones gpui-kit.d.ts makes,",
    "so the file describing the API is itself reported as the error.",
    "",
    "`strictNullChecks` is off, and this one is the runtime's shape rather than",
    "a preference. A view assigns its state in `init`, which TypeScript cannot",
    "see as definite assignment the way it sees a constructor, so every field",
    "would read as possibly-undefined and every use would want a `?.` that",
    "means nothing at run time. Turning it on would buy noise, not safety."
  ],
  "compilerOptions": {
    "target": "ES2022",
    "module": "ES2022",
    "moduleResolution": "bundler",
    "lib": ["ES2022"],
    "checkJs": true,
    "strict": true,
    "strictNullChecks": false,
    "noEmit": true
  }
}
"#;

/// Writes the editor configuration beside an application, once.
///
/// Only when the directory has neither configuration file: the first launch
/// scaffolds one, and everything after that leaves the author's own settings
/// alone. Returns the path only when it actually wrote.
fn write_editor_config(directory: &Path) -> std::io::Result<Option<PathBuf>> {
    let path = directory.join(CONFIG_FILE_NAME);
    if std::fs::symlink_metadata(&path).is_ok()
        || std::fs::symlink_metadata(directory.join(TYPESCRIPT_CONFIG_FILE_NAME)).is_ok()
    {
        return Ok(None);
    }
    std::fs::write(&path, EDITOR_CONFIG)?;
    Ok(Some(path))
}

/// Emits the TypeScript declarations for the script API.
///
/// The output is deterministic — no timestamps, no reflection order — so
/// regenerating it after a runtime upgrade produces a reviewable diff rather
/// than a reshuffled file.
#[cfg(test)]
pub fn declarations() -> String {
    declarations_with_components(&crate::FrozenComponentRegistry::default())
}

#[cfg(test)]
fn base_declarations() -> String {
    declarations()
}

pub(crate) fn declarations_with_components(components: &crate::FrozenComponentRegistry) -> String {
    let (nullary, parametric) = style_methods();

    let mut out = String::with_capacity(160 * 1024);
    out.push_str(&PREAMBLE.replace("{version}", crate::plugin::SHELL_VERSION));
    out.push_str("declare module \"gpui-kit\" {\n");
    out.push_str(VALUE_TYPES);
    out.push_str(&color_types());
    out.push_str(&role_type());
    out.push_str(&anchor_type());
    out.push_str(&view_types());
    out.push_str("  /**\n");
    out.push_str("   * A description of one element, built by chaining.\n");
    out.push_str("   *\n");
    out.push_str("   * Every method returns the same element, so a chain is one\n");
    out.push_str("   * expression. An element is consumed when it is used as a child and\n");
    out.push_str("   * belongs to the render pass that built it; storing one and using it\n");
    out.push_str("   * again throws, which no type can prevent.\n");
    out.push_str("   */\n");
    out.push_str("  export interface Element {\n");
    out.push_str(ELEMENT_METHODS);
    out.push_str(&parametric_styles(&parametric));
    out.push_str(&nullary_styles(&nullary));
    out.push_str("  }\n");
    out.push_str(ELEMENTS);
    out.push_str(WINDOW);
    out.push_str(CAPABILITIES);
    out.push_str(SCHEDULING);
    out.push_str("}\n\n");
    out.push_str("declare module \"gpui\" {\n  export * from \"gpui-kit\";\n}\n\n");
    out.push_str("declare module \"gpui-base\" {\n");
    out.push_str(BASE_IMPORTS);
    out.push_str(&base_color_token_type());
    out.push_str(BASE_SHARED_TYPES);
    out.push_str(BASE);
    out.push_str("}\n\n");
    out.push_str("declare module \"gpui-component\" {\n");
    out.push_str("  import { ClickEvent, Context, Element } from \"gpui-kit\";\n");
    for state in components.states() {
        push_jsdoc(&mut out, state.documentation(), None, "  ");
        out.push_str("  export interface ");
        out.push_str(state.kind());
        out.push_str(" { readonly __gpuiComponentState: unique symbol }\n");
        push_jsdoc(&mut out, state.documentation(), None, "  ");
        out.push_str("  export function ");
        out.push_str(state.export());
        out.push('(');
        push_arguments(&mut out, state.arguments());
        out.push_str("): ");
        out.push_str(state.kind());
        out.push_str(";\n");
    }
    for descriptor in components.descriptors() {
        push_jsdoc(&mut out, descriptor.documentation(), None, "  ");
        out.push_str("  export type ");
        out.push_str(descriptor.name());
        out.push_str("Element = ");
        // Two reasons a name leaves `Element`. A method the descriptor declares
        // is re-declared below with the descriptor's own signature. And a common
        // behavior the descriptor does *not* declare is refused at run time for
        // a registered component — see the `registered_common_behavior` check in
        // the engine — so leaving it inherited would have `gpui-kit.d.ts` promise a
        // call that always throws.
        let declared = descriptor
            .methods()
            .iter()
            .map(|method| method.name())
            .collect::<Vec<_>>();
        let withheld = REGISTERED_COMMON_BEHAVIORS
            .iter()
            .copied()
            .filter(|behavior| !declared.contains(behavior))
            .collect::<Vec<_>>();
        let removed = declared
            .iter()
            .copied()
            .chain(withheld.iter().copied())
            .collect::<Vec<_>>();
        if removed.is_empty() {
            out.push_str("Element & {\n");
        } else {
            out.push_str("Omit<Element, ");
            for (index, name) in removed.iter().enumerate() {
                if index != 0 {
                    out.push_str(" | ");
                }
                out.push('"');
                out.push_str(name);
                out.push('"');
            }
            out.push_str("> & {\n");
        }
        for method in descriptor.methods() {
            push_jsdoc(&mut out, method.documentation(), None, "    ");
            out.push_str("    ");
            out.push_str(method.name());
            out.push('(');
            push_arguments(&mut out, &method.arguments());
            out.push_str("): ");
            out.push_str(descriptor.name());
            out.push_str("Element;\n");
        }
        // Re-declared rather than simply removed. Dropping the member outright
        // would also drop this type's assignability to `Element`, so a typed
        // part could no longer be passed to a slot that takes one. A `never`
        // parameter keeps the shape and still refuses every call.
        for behavior in &withheld {
            out.push_str("    /**\n     * Not available on this component: `");
            out.push_str(descriptor.name());
            out.push_str("` does not declare `");
            out.push_str(behavior);
            out.push_str("`, and the runtime refuses it.\n     */\n    ");
            out.push_str(behavior);
            out.push_str("(unavailable: never): never;\n");
        }
        out.push_str("  }\n");
        for constructor in descriptor.constructors() {
            push_jsdoc(
                &mut out,
                descriptor.documentation(),
                constructor
                    .deprecation()
                    .as_ref()
                    .map(|entry| entry.message()),
                "  ",
            );
            out.push_str("  export const ");
            out.push_str(constructor.export());
            out.push_str(": { new(");
            push_arguments(&mut out, &constructor.arguments());
            out.push_str("): ");
            out.push_str(descriptor.name());
            out.push_str("Element };\n");
        }
    }
    out.push_str("}\n\n");
    out.push_str("declare module \"gpui-shell\" {\n");
    out.push_str(&shell_types());
    out.push_str("}\n\n");
    out.push_str("declare module \"gpui-fps\" {\n");
    out.push_str(FPS_IMPORTS);
    out.push_str(FPS);
    out.push_str("}\n");
    out.push_str(STANDARD_RUNTIME);
    out.push_str(&host_modules());
    out.push_str(WINDOW_GLOBAL);
    out
}

/// The behaviors a registered component answers only when its descriptor
/// declares them. Mirrors the engine's `registered_common_behavior` list; a
/// test asserts the two agree.
pub(crate) const REGISTERED_COMMON_BEHAVIORS: [&str; 3] = ["disabled", "selected", "on_click"];

fn push_arguments(out: &mut String, arguments: &[crate::ArgumentDescriptor]) {
    for (index, argument) in arguments.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(argument.name());
        if matches!(argument.schema(), crate::ArgumentSchema::Optional(_)) {
            out.push('?');
        }
        out.push_str(": ");
        out.push_str(&argument_type(&argument.schema()));
    }
}

fn argument_type(schema: &crate::ArgumentSchema) -> String {
    match schema {
        crate::ArgumentSchema::String => "string".into(),
        crate::ArgumentSchema::Number => "number".into(),
        crate::ArgumentSchema::Boolean => "boolean".into(),
        crate::ArgumentSchema::Element => "Element".into(),
        crate::ArgumentSchema::Entity(kind) => (*kind).into(),
        crate::ArgumentSchema::Callback(signature) => (*signature).into(),
        crate::ArgumentSchema::Enum(values) => values
            .iter()
            .map(|value| format!("{value:?}"))
            .collect::<Vec<_>>()
            .join(" | "),
        crate::ArgumentSchema::Array(item) => format!("Array<{}>", argument_type(item)),
        crate::ArgumentSchema::Optional(item) => argument_type(item),
    }
}

fn push_jsdoc(
    out: &mut String,
    documentation: Option<&str>,
    deprecated: Option<&str>,
    indent: &str,
) {
    if documentation.is_none() && deprecated.is_none() {
        return;
    }
    out.push_str(indent);
    out.push_str("/**\n");
    if let Some(documentation) = documentation {
        for line in documentation.lines() {
            out.push_str(indent);
            out.push_str(" * ");
            out.push_str(line);
            out.push('\n');
        }
    }
    if let Some(message) = deprecated {
        out.push_str(indent);
        out.push_str(" * @deprecated ");
        out.push_str(message);
        out.push('\n');
    }
    out.push_str(indent);
    out.push_str(" */\n");
}

/// The modules this host registered, as `declare module` blocks.
///
/// Generated from the registry rather than hand-written beside the script,
/// which is the whole reason HostModule registrations became imports. A module that gave
/// itself a TypeScript face through [`crate::HostModule::declarations`] is
/// emitted verbatim — and [`crate::HostModule::validate`] has already checked
/// that face against what was registered, so it cannot describe a function that
/// is not there. One that did not is emitted with permissive signatures, which
/// still check the module name and every export name.
/// Re-indents a module's declarations to sit one level inside `declare module`.
///
/// A host writes them in a Rust raw string, so they arrive carrying whatever
/// indentation that literal had. Stripping each line entirely would flatten a
/// multi-line `interface` into something TypeScript still parses but nobody
/// wants to read, so the common prefix goes and the shape inside it stays.
fn reindented(declarations: &str) -> String {
    let common = declarations
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    let mut out = String::new();
    for line in declarations.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            out.push('\n');
        } else {
            let _ = writeln!(out, "  {}", &line[common..]);
        }
    }
    out
}

fn host_modules() -> String {
    let registry = crate::host_modules::modules();
    let mut out = String::new();
    for name in registry.module_names() {
        let Ok(module) = registry.get(name) else {
            continue;
        };
        let _ = writeln!(out, "\ndeclare module \"{name}\" {{");
        match module.declared() {
            Some(declarations) => out.push_str(&reindented(declarations)),
            None => {
                // `HostValue`, not `any`. The boundary carries exactly what the
                // Rust type of that name carries, so `any` would be wider than
                // the runtime: a script passing a function or a Symbol would
                // type-check and then be refused at the call.
                out.push_str("  import { Element, HostValue } from \"gpui-kit\";\n\n");
                for function in module.function_names() {
                    // Permissive about shape, but not wrong about the one thing
                    // the caller has to get right: an asynchronous export is
                    // awaited.
                    let returns = if module.is_async(function) {
                        "Promise<HostValue>"
                    } else {
                        "HostValue"
                    };
                    let _ = writeln!(
                        out,
                        "  export function {function}(...args: HostValue[]): {returns};"
                    );
                }
                for component in module.component_names() {
                    let _ = writeln!(
                        out,
                        "  export const {component}: {{ new(id: string, props: HostValue): Element }};"
                    );
                }
            }
        }
        out.push_str("}\n");
    }
    out
}

/// Refreshes the declarations in every directory of an application that imports
/// one of the built-in modules.
///
/// One file at the root is enough for an editor that has the whole application
/// open, and not enough for anything else: a subdirectory opened on its own, a
/// tool pointed at one file, a script vendored elsewhere. Since the file is
/// generated and ignored rather than committed, a copy per directory costs
/// nothing anybody has to look at.
///
/// The explicit tooling API reports write failures. Ordinary application loads
/// log them at debug level and continue, because an unwritable declaration is a
/// worse editing experience, not a reason to refuse to run the application.
pub(crate) fn write_application_with_components(
    root: &Path,
    components: &crate::FrozenComponentRegistry,
) -> std::io::Result<Vec<PathBuf>> {
    std::fs::create_dir_all(root)?;
    let mut directories = vec![root.to_path_buf()];
    directories.extend(
        directories_importing_builtins(root)
            .into_iter()
            .filter(|directory| directory != root),
    );

    let mut written = Vec::new();
    let mut first_error = None;
    // The application root only: one project, one configuration, and a nested
    // directory that happens to import `gpui-kit` is part of it rather than a
    // second project.
    match write_editor_config(root) {
        Ok(Some(path)) => written.push(path),
        Ok(None) => {}
        Err(error) => {
            first_error.get_or_insert(error);
        }
    }
    for directory in directories {
        match refresh_with_components(&directory, components) {
            Ok(Some(path)) => written.push(path),
            Ok(None) => {}
            Err(error) => {
                first_error.get_or_insert(error);
            }
        }
    }
    match first_error {
        Some(error) => Err(error),
        None => Ok(written),
    }
}

/// Directories holding at least one script that imports a built-in module.
///
/// Bounded the way the source watcher is bounded, and for the same reason: an
/// application directory is whatever someone pointed the runtime at, and a
/// symlink farm or a vendored tree must not turn a startup step into an
/// unbounded walk.
fn directories_importing_builtins(root: &Path) -> Vec<PathBuf> {
    const MAX_DEPTH: usize = 8;
    const MAX_FILES: usize = 4_096;
    const SKIPPED: [&str; 2] = ["node_modules", "target"];

    let mut found = Vec::new();
    let mut pending = vec![(root.to_path_buf(), 0usize)];
    let mut seen = 0usize;

    while let Some((directory, depth)) = pending.pop() {
        let mut imports = false;

        for entry in std::fs::read_dir(&directory)
            .into_iter()
            .flatten()
            .flatten()
        {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();

            if name.starts_with('.') || SKIPPED.contains(&name.as_ref()) {
                continue;
            }

            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }

            if file_type.is_dir() {
                if depth < MAX_DEPTH {
                    pending.push((path, depth + 1));
                }
                continue;
            }

            seen += 1;
            if seen > MAX_FILES {
                return found;
            }

            if !imports
                && matches!(
                    path.extension().and_then(|extension| extension.to_str()),
                    Some("js" | "mjs")
                )
                && std::fs::read_to_string(&path).is_ok_and(|source| imports_builtin(&source))
            {
                imports = true;
            }
        }

        if imports {
            found.push(directory);
        }
    }

    found
}

/// The specifiers one `gpui-kit.d.ts` declares. A script importing any of them
/// wants the file beside it.
const BUILTIN_SPECIFIERS: [&str; 6] = [
    "gpui-kit",
    "gpui",
    "gpui-base",
    crate::DEFAULT_COMPONENT_MODULE,
    "gpui-shell",
    "gpui-fps",
];

/// Whether a script imports one of the built-in modules.
///
/// Matching the quoted specifier rather than the bare word, so a file that only
/// mentions gpui in a comment or a string does not collect a copy it has no use
/// for.
fn imports_builtin(source: &str) -> bool {
    BUILTIN_SPECIFIERS.iter().any(|specifier| {
        source.contains(&format!("\"{specifier}\"")) || source.contains(&format!("'{specifier}'"))
    })
}

/// Rewrites the declarations beside an application when they are not current.
///
/// This is what a host should call in a development build. An application never
/// has to remember to regenerate anything, and cannot end up editing against a
/// runtime it is not running: the process that will execute the script is the
/// one that describes it.
///
/// Nothing is written when the file already matches, so an editor watching the
/// directory is not woken on every launch, and a read-only checkout is not an
/// error worth reporting. Returns the path only when it actually wrote.
pub(crate) fn refresh_with_components(
    directory: &Path,
    components: &crate::FrozenComponentRegistry,
) -> std::io::Result<Option<PathBuf>> {
    let path = directory.join(FILE_NAME);
    if std::fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("refusing to replace symlink {}", path.display()),
        ));
    }
    let current = declarations_with_components(components);

    if std::fs::read_to_string(&path).is_ok_and(|committed| committed == current) {
        return Ok(None);
    }

    std::fs::write(&path, current)?;
    Ok(Some(path))
}

/// Splits the style surface into the two halves that need different treatment:
/// the no-argument methods, which are all alike, and the parametric ones, which
/// each need an argument type.
///
/// Both come out of one sorted, deduplicated list, so the two halves cannot
/// overlap and cannot together miss a name the runtime accepts.
fn style_methods() -> (Vec<&'static str>, Vec<&'static str>) {
    style::known_names()
        .into_iter()
        .partition(|name| style::param_style_name(name).is_none())
}

/// What a parametric style method accepts.
///
/// Named after the GPUI types they mirror, because the whole point of the
/// distinction is that the Rust signature is what rejects `.p("auto")`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Argument {
    String,
    Length,
    DefiniteLength,
    AbsoluteLength,
    Color,
    Number,
    /// Nothing the probe recognizes. Emitted as `never` rather than `any` so a
    /// style method added to the runtime without a matching literal here fails
    /// loudly at the first call site instead of silently accepting anything.
    Unrecognized,
}

impl Argument {
    fn ts_type(self) -> &'static str {
        match self {
            Argument::String => "string",
            Argument::Length => "Length",
            Argument::DefiniteLength => "DefiniteLength",
            Argument::AbsoluteLength => "AbsoluteLength",
            Argument::Color => "Color",
            Argument::Number => "number",
            Argument::Unrecognized => "never",
        }
    }
}

/// Asks the runtime what `name` accepts, by handing it one literal of each
/// shape and seeing which are refused.
///
/// The order matters and follows the containment of the grammars: a color is
/// the only argument that takes `#rrggbb`, `Length` the only one that takes
/// `"auto"`, `DefiniteLength` the only remaining one that takes a percentage,
/// and `AbsoluteLength` the only remaining one that takes any string at all.
/// `line_height` classifies as a definite length, which is the right *type*;
/// its bare number means a multiplier rather than pixels, and that is a
/// documentation matter rather than a typing one.
fn argument_of(name: &str) -> Argument {
    if name == "font_family" {
        return Argument::String;
    }
    if name == "font_weight" {
        return Argument::Number;
    }
    let accepts = |value: Bridged| style::apply_param(name, &[value], StyleRefinement::default());

    if accepts(Bridged::Str("#ff0000".into())).is_ok() {
        Argument::Color
    } else if accepts(Bridged::Str("auto".into())).is_ok() {
        Argument::Length
    } else if accepts(Bridged::Str("50%".into())).is_ok() {
        Argument::DefiniteLength
    } else if accepts(Bridged::Str("12px".into())).is_ok() {
        Argument::AbsoluteLength
    } else if accepts(Bridged::Number(1.)).is_ok() {
        Argument::Number
    } else {
        Argument::Unrecognized
    }
}

fn color_types() -> String {
    let mut out = String::new();
    out.push_str("  /**\n");
    out.push_str("   * A concrete `#rgb`, `#rrggbb`, or `#rrggbbaa` color value.\n");
    out.push_str("   * Read semantic colors from `cx.theme().colors`; bare token names are\n");
    out.push_str("   * intentionally not accepted.\n");
    out.push_str("   */\n");
    out.push_str("  export type Color = `#${string}`;\n\n");
    out
}

fn base_color_token_type() -> String {
    let mut out = String::new();
    out.push_str("  /** Every semantic color token the installed Base palette defines. */\n");
    out.push_str("  export type ColorToken =\n");
    for name in color_token_names() {
        let _ = writeln!(out, "    | \"{name}\"");
    }
    out.push_str("    ;\n\n");
    out
}

/// The accessibility roles, generated from the same table `role(...)` parses
/// through, so a name that type-checks is a name the runtime accepts.
fn role_type() -> String {
    let mut out = String::new();
    out.push_str("  /**\n");
    out.push_str("   * An accessibility role, mirroring `gpui::Role` in snake_case.\n");
    out.push_str("   *\n");
    out.push_str("   * `generic_container` is deliberately absent: GPUI filters that role\n");
    out.push_str("   * out of the accessibility tree, so an element carrying it announces\n");
    out.push_str("   * nothing while looking as though it announced something.\n");
    out.push_str("   */\n");
    out.push_str("  export type Role =\n");
    for name in a11y::role_names() {
        let _ = writeln!(out, "    | \"{name}\"");
    }
    out.push_str("    ;\n\n");
    out
}

/// The anchors, generated from the same table `anchor(...)` parses through, so
/// a corner that type-checks is a corner the runtime accepts.
fn anchor_type() -> String {
    let mut out = String::new();
    out.push_str("  /**\n");
    out.push_str("   * Which corner of an anchored surface is pinned to its trigger,\n");
    out.push_str("   * mirroring `gpui::Anchor` in snake_case.\n");
    out.push_str("   */\n");
    out.push_str("  export type Anchor =\n");
    for name in crate::materialize::ANCHOR_NAMES {
        let _ = writeln!(out, "    | \"{name}\"");
    }
    out.push_str("    ;\n\n");
    out.push_str("  /** Which pointer button opens a `Popover`. */\n");
    out.push_str("  export type MouseButton = \"left\" | \"right\" | \"middle\";\n\n");
    out
}

fn view_types() -> String {
    let mut out = String::new();
    out.push_str(CONTEXT_AND_VIEW);
    out
}

/// The style methods that take an argument, sorted, each typed by probe.
fn parametric_styles(names: &[&'static str]) -> String {
    let mut out = String::new();
    out.push_str("\n    // Style methods that take an argument. Which length type a method\n");
    out.push_str("    // accepts follows its Rust signature, so `.p(\"auto\")` and\n");
    out.push_str("    // `.rounded(\"50%\")` are type errors here for the same reason they\n");
    out.push_str("    // throw at run time.\n");
    for name in names {
        out.push_str(&doc_comment(style::documentation(name), 4));
        let _ = writeln!(
            out,
            "    {name}(value: {}): Element;",
            argument_of(name).ts_type()
        );
    }
    out
}

/// The no-argument style methods, straight from the reflection table.
fn nullary_styles(names: &[&'static str]) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "\n    // The {} no-argument style methods, generated from GPUI's reflection\n    \
         // table. A name here is a name the runtime dispatches, and the\n    \
         // documentation is GPUI's own.",
        names.len()
    );
    for name in names {
        out.push_str(&doc_comment(style::documentation(name), 4));
        let _ = writeln!(out, "    {name}(): Element;");
    }
    out
}

/// Renders a Rust doc comment as a JSDoc block at `indent` spaces.
///
/// The text comes from GPUI's reflection table rather than from anything
/// written here, so it arrives as whatever upstream wrote: usually one sentence
/// and a link to the Tailwind page the method is modelled on. A single line
/// stays on one line, because six hundred four-line blocks would bury the
/// surface they are describing.
///
/// Nothing is emitted for a method the table has no documentation for — the
/// parametric styles and the handful named by hand — because inventing a
/// sentence is how generated declarations start disagreeing with the runtime.
fn doc_comment(documentation: Option<&str>, indent: usize) -> String {
    let Some(text) = documentation else {
        return String::new();
    };

    let pad = " ".repeat(indent);
    // A doc that closed the comment early would take the rest of the file with
    // it. Upstream has none today; this costs one scan to keep it that way.
    let text = text.replace("*/", "*\u{200b}/");
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());

    let Some(first) = lines.next() else {
        return String::new();
    };
    let rest: Vec<&str> = lines.collect();

    if rest.is_empty() {
        return format!("{pad}/** {} */\n", first.trim());
    }

    let mut out = format!("{pad}/**\n{pad} * {}\n", first.trim());
    for line in rest {
        let _ = writeln!(out, "{pad} *\n{pad} * {}", line.trim());
    }
    let _ = writeln!(out, "{pad} */");
    out
}

const PREAMBLE: &str = "\
// Auto-generated — add `gpui-kit.d.ts` to your .gitignore.
//
// The built-in modules, as TypeScript declarations, for gpui-shell {version}.
// Do not edit: gpui-shell rewrites this on every run, in every directory that
// imports one of them, from the runtime that is about to execute the script. A
// committed copy could only ever be the stale one.
//
// Each built-in module names the public Rust layer it exposes, so an import
// says which layer a script depends on. \"gpui-kit\" also carries the shell bridge:
//
//   \"gpui-kit\"   GPUI's own elements, plus what this runtime adds: views,
//                the style surface, the window, storage, scheduling.
//   \"gpui\"       Compatibility alias for \"gpui-kit\".
//   \"gpui-base\"  gpui-base's layout helpers, components and theme.
//   \"gpui-fps\"   gpui-fps's performance overlay.
//
// Except for the explicit \"gpui\" compatibility alias, a name belongs to
// exactly one module and is not re-exported for convenience.
//
// The style surface here is generated from the same tables the runtime
// dispatches through, so a style method that type-checks exists at run time,
// and a length or color the compiler refuses is one the runtime would refuse
// too. Put `// @ts-check` at the top of a script to have an editor check it.
//
// What is not expressed: capability grants (a denied `fs.readFile` still
// type-checks), element and `cx` lifetimes (both belong to one call), and
// which component a method suits (all elements share one prototype).

";

const VALUE_TYPES: &str = r#"  /**
   * A length. A bare number is pixels; a string carries its unit.
   *
   * `"auto"` is only accepted where the Rust signature takes `Length` — the
   * padding, gap, border and radius families take the narrower types below.
   */
  export type Length = number | import("gpui-shell").LengthString | "auto";

  /** A length that must resolve to a size: pixels, rems or a percentage. */
  export type DefiniteLength = number | import("gpui-shell").LengthString;

  /** A length with no percentage and no `"auto"`: pixels or rems. */
  export type AbsoluteLength = number | `${number}px` | `${number}rem`;

  /** A layout axis, mirroring `gpui::Axis`. */
  export type Axis = "horizontal" | "vertical";

  /**
   * What a HostModule function takes and answers.
   *
   * Named after the Rust type it mirrors, `HostValue`, rather than after the
   * shape it happens to have: `Json` would sit one capital letter away from the
   * built-in `JSON` object and mean something entirely different — a value,
   * not a parser.
   */
  export type HostValue =
    | null
    | boolean
    | number
    | string
    | HostValue[]
    | { [key: string]: HostValue };

"#;

const CONTEXT_AND_VIEW: &str = r#"  /**
   * The script-side context for one host call.
   *
   * It is valid only for the call that produced it: an `await` returns to the
   * host and the frame it names goes away, so a `cx` kept across one reports a
   * stale-context error. Work that outlives the call takes an [`AsyncContext`]
   * instead — `cx.spawn`, `cx.timer`, `init`.
   */
  export interface Context {
    /**
     * Requests a re-render of the current view, or of one retained `Entity`.
     *
     * Pass a target after changing shared state that retained child reads. It
     * invalidates that child's script description without invoking its
     * `update(props)`; use `entity.set_props(props)` when the child must receive
     * and process new props.
     *
     * Legal from an event handler or a task. Calling it during `render` throws,
     * because notifying a view while rendering is a loop.
     */
    notify(target?: Entity): void;
    /**
     * `App::bind_keys`. Installs key bindings and answers how many.
     *
     * The keymap is the application's, not a window's or a view's, so a chord
     * bound here is live wherever its `context` predicate matches. The whole
     * list is validated before any of it is installed: a keymap half applied
     * because one entry had a typo is a worse state than one not applied, and
     * the script cannot see which half made it.
     *
     * Illegal from `render`.
     */
    bind_keys(bindings: KeyBinding[]): number;
    /**
     * `App::stop_propagation`. Stops this event reaching the handlers above
     * this element.
     *
     * GPUI delivers an event to every handler on the path, so a row inside a
     * list with its own `on_click` fires both. Call this from the inner one to
     * keep the event there.
     */
    stop_propagation(): void;
    /**
     * `App::propagate`. Undoes a `stop_propagation()` made earlier in the same
     * dispatch, letting the event continue.
     */
    propagate(): void;
    phase(): import("gpui-shell").ScopePhase;
    /** Reads the current `gpui_base::Theme` semantic token projection. */
    theme(): import("gpui-base").Theme;

    /**
     * Hands a URL to whatever the system opens URLs with. `App::open_url`.
     *
     * `Link`'s `href` without the element, for the case where the address is
     * not known until something has already happened — the end of a device
     * authorization, say, where waiting for a second click to open the page the
     * first click just asked for is a step nobody wanted.
     *
     * It takes an absolute `http`/`https` URL with a host and refuses
     * everything else. That guard is not about the address bar: without it this
     * is a way to hand an arbitrary URI to whichever handler the desktop
     * registered for its scheme.
     */
    open_url(url: string): void;

    /** `App::read_from_clipboard`. `undefined` when it holds no text. */
    read_from_clipboard(): string | undefined;
    /** `App::write_to_clipboard`. */
    write_to_clipboard(text: string): void;

    /**
     * A focus target the script owns, created once and kept on the view.
     * `App::focus_handle` — GPUI has no `FocusHandle::new`, and neither does
     * this.
     *
     * Focus is a fact about the window that outlives any one render, so an
     * element rebuilt every frame cannot own it. Hand the handle to an element
     * with `track_focus(...)`, and it is that element the keyboard means.
     *
     * It would produce a fresh handle on every frame, so it belongs in `init`
     * or in an event handler — never in `render`.
     */
    focus_handle(): FocusHandle;

    /**
     * Creates a retained nested view and hands back the entity that owns it.
     * `AppContext::new` — the only way GPUI makes one, and so the only way here.
     *
     * The entity is a child wherever a child is taken: `.child(entity)`, or
     * returned from `render`. Updating its props runs the child's optional
     * `update(props)` and rebuilds only that child.
     *
     * Legal from `init`, an event handler or a task; creating one during
     * `render` or layout throws.
     */
    new(Class: ViewClass, props?: import("gpui-shell").Props): Entity;

    /**
     * Calls `body(cx)` and adopts the promise it returns, so a rejection is
     * reported rather than swallowed. `App::spawn`.
     *
     * The `cx` the body receives is an [`AsyncContext`] — valid across `await`,
     * the way GPUI's `AsyncApp` is — so the whole body can keep using it.
     */
    spawn(body: (cx: AsyncContext) => unknown, opts?: import("gpui-shell").TaskOptions): Task;

    /** Resolves after `ms` on GPUI's foreground executor. */
    sleep(ms?: number): Promise<void>;

    /** One-shot and repeating callbacks on the foreground executor. */
    readonly timer: Timer;
  }

  /**
   * A context that may be held across an `await`.
   *
   * The mirror of GPUI's `AsyncApp`. An ordinary [`Context`] speaks for one
   * host call and reports clearly once that call has returned — which is what
   * catches a `cx` stashed in a closure. This one names no call at all: it
   * resolves whichever is running when a member is used, and refuses only when
   * none is.
   *
   * It is what `init` receives, and what `cx.spawn` and `cx.timer` hand their
   * callbacks — the three places whose whole job is to set up or continue work
   * that outlives the call they were started from.
   */
  export interface AsyncContext extends Context {}



  /** The modifier keys held when a click was delivered. */
  export interface Modifiers {
    shift: boolean;
    control: boolean;
    alt: boolean;
    /** Command on macOS, Windows key elsewhere. */
    platform: boolean;
    /** The Function key. */
    function: boolean;
  }

  /** The Caps Lock state carried by GPUI modifier-change events. */
  export interface Capslock {
    on: boolean;
  }

  /** What an `on_modifiers_changed` handler receives. */
  export interface ModifiersChangedEvent {
    modifiers: Modifiers;
    capslock: Capslock;
  }

  /** What an `on_click` handler receives. Keyboard activation counts as one. */
  export interface ClickEvent {
    click_count: number;
    modifiers: Modifiers;
  }

  /**
   * What an `on_key_down` or `on_key_up` handler receives.
   *
   * `keystroke` is the whole chord in the spelling a key binding is written
   * in — `"cmd-shift-s"`, `"escape"`, `"ctrl-alt-delete"` — and is what a
   * comparison is normally written against. `key` and `modifiers` are the
   * same thing taken apart, for when only one half matters.
   *
   * The platform modifier is spelled `cmd` on every platform, including Linux
   * and Windows. GPUI spells it for the platform it was built for, which is
   * right for a keymap a person reads and wrong for a string a program
   * compares: one script runs on all three, and `event.keystroke === "cmd-s"`
   * has to mean the same thing in all three. It is also the spelling
   * `cx.bind_keys` accepts everywhere, so a binding and the event it produces
   * agree by construction.
   */
  export interface KeyEvent {
    /** The key printed on the key that was pressed, e.g. `"s"` or `"escape"`. */
    key: string;
    /** The full chord, as GPUI's `Keystroke::unparse` spells it. */
    keystroke: string;
    /** The character this keystroke would type, when it types one. */
    key_char?: string;
    modifiers: Modifiers;
    /** Whether the key is being held down. Absent on `on_key_up`. */
    is_held?: boolean;
  }

  export interface Point { x: number; y: number; }
  export interface Size { width: number; height: number; }
  /** GPUI mouse coordinates. `position` is window-relative; `local_position` is element-relative. */
  export interface MouseMoveEvent {
    position: Point;
    local_position: Point;
    bounds: import("gpui-shell").ElementBounds;
    modifiers: Modifiers;
  }

  /**
   * What an `on_mouse_down`, `on_mouse_up` or `on_mouse_down_out` handler
   * receives.
   *
   * `local_position` and `bounds` are absent when the element has not been
   * painted yet, and on an `on_mouse_down_out` press they describe an element
   * the pointer is outside of — so `local_position` there is negative, or past
   * the far edge, which is exactly what says which way.
   */
  export interface MouseButtonEvent {
    button: MouseButton;
    /** How many presses in the current sequence; `2` on a double-click. */
    click_count: number;
    position: Point;
    local_position?: Point;
    bounds?: import("gpui-shell").ElementBounds;
    modifiers: Modifiers;
  }

  /** What an `on_action` handler receives. */
  export interface ActionEvent {
    /** The action's name, as the script bound and registered it. */
    action: string;
  }

  /** One entry of `cx.bind_keys`. */
  export interface KeyBinding {
    /** The chord, e.g. `"cmd-s"`, or a sequence: `"ctrl-k ctrl-s"`. */
    keystroke: string;
    /** The action this chord dispatches. */
    action: string;
    /**
     * Where it applies, as a key-context predicate matched against the
     * `key_context(...)` an element declares — `"Editor"`, `"Pane && !modal"`.
     * Omitted, the binding is global.
     */
    context?: string;
  }

  /** What an `on_scroll_wheel` handler receives. */
  export interface ScrollWheelEvent {
    /** The scroll distance in pixels, whichever unit the device reported. */
    delta: Point;
    /** The same distance in lines, when the device reported lines. */
    delta_lines?: Point;
    touch_phase: "started" | "moved" | "ended" | "cancelled";
    position: Point;
    local_position?: Point;
    bounds?: import("gpui-shell").ElementBounds;
    modifiers: Modifiers;
  }

  /**
   * The base class of every view: subclass it and default-export the subclass.
   *
   * `init` runs once when the view is created. `render` returns one element,
   * retained entity or string, and runs when the view is invalidated — by
   * `cx.notify()`, a reload, or a theme change — not on every frame. Never
   * store an element on the instance: it belongs to the render that built it.
   */
  export abstract class View {
    constructor(props?: import("gpui-shell").Props);
    /**
     * Runs once when the view is created.
     *
     * `cx` is an [`AsyncContext`], because this is where retained things are
     * made — tasks, timers, focus handles — and the context that starts a task
     * is the one its body will still be using after an `await`.
     */
    init?(props: import("gpui-shell").Props | undefined, cx: AsyncContext): void;
    /** Runs when a parent changes this retained nested view's properties. */
    update?(props: import("gpui-shell").Props | undefined): void;
    abstract render(cx: Context): Element | Entity | string;
  }

  /** A concrete script view class that can be retained as a nested view. */
  export type ViewClass = new (props?: import("gpui-shell").Props) => View;

  /**
   * Retained ownership of one nested `View` entity.
   *
   * Create it once from `init`, an event handler or a task. Updating props
   * invokes the child's optional `update(props)` and rebuilds only that child.
   * Phase, class and released-handle validation errors throw synchronously. Native
   * construction/init/update is applied before the enclosing host entry returns;
   * failures are reported by that host entry rather than being catchable around
   * this synchronous-looking call.
   *
   * A failed `update` has a bounded shell rollback:
   * - ordinary reachable properties, including callable objects, are restored only while their post-update descriptors remain legally redefinable or deletable;
   * - shell-owned entities, tasks and nested views newly created by the update are released.
   * Unsupported mutations include JavaScript private fields and internal slots;
   * newly added non-configurable properties; making an existing configurable property non-configurable;
   * and pre-existing native handles explicitly released by update.
   */
  export interface Entity {
    set_props(props?: import("gpui-shell").Props): void;
    release(): boolean;
  }


"#;

/// The element methods that are not styles.
///
/// Hand-written because each has a signature of its own; the names match the
/// behavior list the engine installs on the prototype, and
/// [`tests::every_element_method_is_accounted_for`] fails if the two drift.
const ELEMENT_METHODS: &str = r#"    /**
     * Passes this element to `transform` and returns exactly what it returns.
     *
     * This mirrors GPUI's `FluentBuilder.map`: it is useful for keeping an
     * imperative or conditional transformation inside a fluent expression.
     */
    map<T>(transform: (element: Element) => T): T;
    /**
     * Adds one child. The child is consumed; using it again throws.
     *
     * A **string is an element**, exactly as `&str`, `String` and
     * `SharedString` implement `IntoElement` in GPUI: `.child("hello")` is how
     * text is written, and the style comes from the element holding it.
     *
     * An `Entity` from `cx.new(...)` is a child too, the way an `Entity<V>` is
     * renderable in GPUI — that is how a retained nested view is mounted. One
     * entity may appear once per parent snapshot; a second mount in the same
     * description is refused before any of it is published.
     */
    child(child: Element | Entity | string | number | boolean): Element;
    /** Adds several children, in order. */
    children(children: Iterable<Element | Entity | string | number | boolean>): Element;
    /**
     * Fills the `content` slot of a `Collapsible`, a `Popover`, a `HoverCard`
     * or a `Popup`.
     *
     * A slot is not a child: the element is consumed here and rendered by the
     * component itself — for a `Collapsible`, only while it is `open`; for the
     * two anchored surfaces, in a layer above the rest of the window. Adding it
     * as a child as well throws.
     *
     * It takes an element, not a function returning one, and that is on purpose
     * even though `window.open_dialog` takes a function. A dialog is a view of
     * its own, opened from an event and outliving the render that opened it. A
     * popover's content is part of *this* render: it is described beside its
     * trigger and rebuilt with it, which is exactly what makes `cx.notify()`
     * reach inside an open surface. A function would make it a separate view,
     * invalidated separately — pick an item in an open menu, watch a count
     * outside the menu change and the same count inside it stay put.
     *
     * A `HoverCard` wraps what it is given in an element of its own, so that
     * moving the pointer onto the card keeps it open. Styles written here land
     * on the inner element; the region the pointer has to reach is the wrapper
     * around it.
     */
    content(element: Element): Element;
    /**
     * Fills an `Avatar`'s `image` slot, which takes an `AvatarImage`.
     *
     * Consumed exactly as `content` is — a slot element is not also drawn as a
     * child. Base renders this one when it is there and the `fallback` when it
     * is not, so filling both is how a picture gets something to fall back to.
     */
    image(element: Element): Element;
    /** Fills an `Avatar`'s `fallback` slot, which takes an `AvatarFallback`. */
    fallback(element: Element): Element;
    /** Fills an `AccordionItem`'s `header` slot, which takes an `AccordionHeader`. */
    header(element: Element): Element;
    /** Fills a component's named `footer` slot. */
    footer(element: Element): Element;
    /** Fills an `AccordionItem`'s `panel` slot, which takes an `AccordionPanel`. */
    panel(element: Element): Element;
    /**
     * Fills the `trigger` slot of a `Popover` or a `HoverCard`: the element
     * that is on screen while the surface is closed, and that opens it.
     *
     * Consumed exactly as `content` is. A surface with no trigger draws
     * nothing at all. A `Popup` takes its trigger in `Popup.new(id, trigger)`
     * instead, because its trigger's bounds are what the content is anchored
     * to.
     */
    trigger(element: Element): Element;
    /**
     * Fills the editor slot of a `NumberInput`.
     *
     * Left empty, the frame draws the bare editor for the state it was built
     * from, which is what a number input almost always wants. Fill it to put
     * something else there — but not `Input.new(state)`: that is the *framed*
     * editor, and a frame inside this frame draws two borders. Adornments
     * beside the editor are ordinary `child(...)` calls on the number input.
     */
    input(element: Element): Element;
    /**
     * Supplies the look of a `NumberInput`'s decrement button.
     *
     * Not optional in practice. The step button is built by the base layer and
     * is completely unstyled — no size, no content — so a number input that
     * leaves this empty has a decrement control that cannot be seen and cannot
     * be pressed.
     *
     * It behaves unlike every other slot: the element is not rendered, it is
     * *replayed*. Its styles, its state styles, its accessibility label and its
     * children are moved onto the button the base layer built, because that
     * button is what receives the press. Give it an `h_flex()` or a `div()`. A
     * `Button.new(id)` works too, but its id is dropped — the step button is
     * already identified. A `text(...)` or an `svg(...)` on its own has no
     * children to move and loses what it draws, so wrap it.
     *
     * `disabled(...)` and `on_click(...)` written here are overwritten: the
     * number input owns whether stepping is allowed and what a press does.
     */
    decrement_button(element: Element): Element;
    /** The increment button, replayed exactly as `decrement_button` is. */
    increment_button(element: Element): Element;
    /**
     * Stacks both of a `NumberInput`'s step buttons to the right of the text,
     * rather than putting one on each side of it.
     */
    controls_right(): Element;
    /**
     * Applies `branch` only when `condition` is truthy, keeping the chain in
     * one piece. `branch` must return the element.
     */
    when(condition: unknown, branch: (el: Element) => Element): Element;

    /**
     * `handler(event, cx)` on activation. Keyboard activation is available
     * only on components whose Base primitive supports it; `Tab` is currently
     * pointer-only pending the compound keyboard behavior tracked in #2838.
     */
    on_click(handler: (event: ClickEvent, cx: Context) => void): Element;
    /** GPUI `InteractiveElement::on_mouse_move`, delivered while this element is hovered. */
    on_mouse_move(handler: (event: MouseMoveEvent, cx: Context) => void): Element;
    /** GPUI `InteractiveElement::on_hover`; reports both pointer entry and exit. */
    on_hover(handler: (hovered: boolean, cx: Context) => void): Element;
    /**
     * GPUI `InteractiveElement::on_key_down`, delivered while this element or
     * something inside it holds the keyboard.
     *
     * A key event travels the focus path, so `track_focus(handle)` is half of
     * this registration rather than a separate concern: without it the handler
     * sits on an element the keyboard never reaches and nothing arrives. The
     * event continues to the handlers above unless `cx.stop_propagation()`
     * says otherwise.
     *
     * Wired on `div`, `h_flex`, `v_flex`, `Button`, `Link`, `Checkbox`,
     * `Switch`, `Radio`, `Toggle`, `Tabs` and `Tab`. On any other component it
     * is recorded and never reaches GPUI, and the log says so — wrap it and
     * write the handler on the wrapper. The same list applies to `on_key_up`,
     * the four pointer handlers, `on_action` and `key_context`.
     *
     * Wired is not the same as reachable. A key travels the focus path, so a
     * component that accepts no focus handle — `Tab` — hears presses and never
     * hears keys, however well both are wired.
     */
    on_key_down(handler: (event: KeyEvent, cx: Context) => void): Element;
    /** GPUI `InteractiveElement::on_key_up`, on the same focus path as `on_key_down`. */
    on_key_up(handler: (event: KeyEvent, cx: Context) => void): Element;
    /** GPUI `InteractiveElement::on_modifiers_changed`, on the keyboard focus path. */
    on_modifiers_changed(
      handler: (event: ModifiersChangedEvent, cx: Context) => void,
    ): Element;
    /**
     * GPUI `InteractiveElement::on_mouse_down`, for one button.
     *
     * Lower-level than `on_click`, and the reason to reach for it is that a
     * press is not a click: it fires before the release, it reports which
     * button, and `click_count` distinguishes a double-click. Registering it
     * for two buttons on one element is fine — the two handlers are
     * independent.
     */
    on_mouse_down(
      button: MouseButton,
      handler: (event: MouseButtonEvent, cx: Context) => void,
    ): Element;
    /** GPUI `InteractiveElement::on_mouse_up`, for one button. */
    on_mouse_up(
      button: MouseButton,
      handler: (event: MouseButtonEvent, cx: Context) => void,
    ): Element;
    /**
     * GPUI `InteractiveElement::on_mouse_down_out`: a press anywhere *outside*
     * this element, delivered during the capture phase.
     *
     * This is how a surface a script drew itself is dismissed by a press
     * elsewhere — the same listener base's own components close on. It fires
     * for any button.
     */
    on_mouse_down_out(handler: (event: MouseButtonEvent, cx: Context) => void): Element;
    /**
     * GPUI `InteractiveElement::on_scroll_wheel`: wheel and trackpad scrolling
     * over this element.
     *
     * For scrolling a region, `overflow_scroll()` is the answer and this is
     * not: it hands GPUI's own retained scroll container the job. Use this when
     * the gesture drives something else — a zoom, a value, a custom viewport.
     */
    on_scroll_wheel(handler: (event: ScrollWheelEvent, cx: Context) => void): Element;
    /**
     * `handler(event, cx)` when the named action is dispatched to this element
     * or to something inside it.
     *
     * An action is the level above a keystroke: `cx.bind_keys` says which
     * chord means `"save"`, in which context, and this says what `"save"`
     * does. A menu item or a button dispatching the same name through
     * `window.dispatch_action("save")` reaches the same handler without
     * pretending to be a keyboard.
     *
     * Registering several on one element is fine and they are independent. An
     * action none of them names carries on to an element further out.
     */
    on_action(action: string, handler: (event: ActionEvent, cx: Context) => void): Element;
    /**
     * `InteractiveElement::key_context`: the key-binding context this element
     * and its subtree sit in.
     *
     * What a binding's `context` predicate is matched against, so one chord can
     * mean one thing in a list and another in an editor. The value is a name or
     * a predicate expression, not free text; an unparsable one is reported and
     * the context is left unset.
     */
    key_context(context: string): Element;
    /**
     * An `AccordionHeader`'s announced heading level — "heading level 3" — as
     * `aria-level` means it. Defaults to 3. It announces; it sizes nothing.
     */
    aria_level(level: number): Element;
    /**
     * Whether an `AccordionPanel` stays in the tree while shut. Off by default;
     * on, its content keeps a scroll position or a half-typed field across a
     * close and reopen.
     */
    keep_mounted(value?: boolean): Element;
    /**
     * `handler(key, cx)` when a row of a virtual list is clicked, where `key`
     * is what the list's `get_key(index)` returned for that row.
     *
     * A key rather than an index, because the two stop agreeing exactly when it
     * matters: the box was captured on the frame the row was drawn, and a
     * filter or a sort can reorder the list before the click is delivered. The
     * key names the item that was pressed; the index would name whatever slid
     * into its place.
     *
     * One handler for the list rather than one per row, and that is a limit
     * rather than a shorthand: a handler registered inside the item renderer
     * throws. Handlers belong to the render pass that registered them and are
     * released with it; a row is rebuilt on every frame the list is scrolled,
     * so a per-row handler would accumulate for as long as the view stood —
     * twenty visible rows over a thousand frames is twenty thousand functions
     * nothing can reach and nothing releases.
     *
     * The key is normally enough: the script already holds the data the row was
     * built from. A row with several independently clickable parts needs a
     * handler lifetime scoped to one batch of items, which this runtime does
     * not have yet; when it does, this restriction lifts and `on_click` inside
     * an item renderer starts working, with no change to anything written
     * against `on_item_click`.
     */
    on_item_click(handler: (key: string, cx: Context) => void): Element;
    /**
     * `handler(key, event, cx)` on a secondary press — the right button — over
     * a row of a virtual list. `key` is what the list's `get_key(index)`
     * returned for that row, and `event` is the press as `on_mouse_down`
     * reports it: `position` in the window, `local_position` and `bounds`
     * against the row's own box.
     *
     * A press rather than a click, because that is when a context menu opens:
     * the row is still under the pointer, so the menu can name what it is for
     * before it is drawn over it. And one handler for the list rather than one
     * per row, for the reason `on_item_click` gives.
     *
     * It is delivered on the row, so a handler for the same button on an
     * element around the list still fires after it, in the ordinary bubble
     * order, with that element's own `local_position`. A menu drawn inside a
     * pane learns which row was pressed from this handler and where in the
     * pane to open from the pane's.
     */
    on_item_secondary_click(
      handler: (key: string, event: MouseButtonEvent, cx: Context) => void,
    ): Element;
    /**
     * `handler(value, cx)`, on a toggle. The script owns the new value.
     *
     * A `Radio` only ever reports `true`. It cannot deselect itself, so an
     * already checked — or disabled — radio reports nothing at all, and
     * clearing a group is the script's own business.
     */
    on_change(handler: (checked: boolean, cx: Context) => void): Element;

    /**
     * `handler(action, cx)` on a `NumberInput`, where `action` is
     * `"increment"` or `"decrement"`.
     *
     * **Replaces the built-in stepping.** Without a handler the control steps
     * itself: it adds or subtracts the state's `set_step(...)`, clamps to
     * `set_min(...)` and `set_max(...)`, and re-applies the numeric mask. All
     * of that lives in the closure this replaces, so once a handler is set none
     * of it runs — the script is the only thing that can move the value, and it
     * moves it with `state.set_value(...)`.
     *
     * Both the step buttons and the Up and Down keys report through it.
     */
    on_step(handler: (action: "increment" | "decrement", cx: Context) => void): Element;
    /**
     * `handler(open, cx)`, when something other than the script changed a
     * `Popover`'s open state: a press on the trigger, a press outside it, or
     * Escape. Storage the value and call `cx.notify()`, the way `on_change`
     * stores a checkbox's.
     *
     * A `HoverCard` accepts this too, and today never calls it: the base layer
     * only reports a change it observes between two of its own renders, which
     * its open state cannot produce. A hover card's open state is its own, so
     * nothing is lost except the notification.
     */
    on_open_change(handler: (open: boolean, cx: Context) => void): Element;
    /**
     * `handler(_, cx)` on Enter in an open `Select` or `Combobox`.
     *
     * There is no payload, because the root holds neither the options nor the
     * selection: what was confirmed is whatever the script had highlighted, and
     * the script is the only side that knows. Confirming a *closed* root opens
     * it instead, so this never runs for that case.
     */
    on_confirm(handler: (event: {}, cx: Context) => void): Element;
    /**
     * `handler(_, cx)` on Escape in an open `Select` or `Combobox`, before
     * `on_open_change(false)` — which is what lets a script commit a pending
     * value on the way out.
     */
    on_dismiss(handler: (event: {}, cx: Context) => void): Element;
    /**
     * The label a hover shows over this element, once the pointer has rested
     * on it for half a second.
     *
     * It takes a string, not an element, and that is a real limit rather than
     * a shorthand: the window's tooltip layer rebuilds its content on every
     * frame the label is up, so a function here would be the one piece of a
     * description re-entered once a frame. What is drawn is the shell's own
     * label, in the theme's surface, border, radius and spacing.
     *
     * Wired on a plain `div`, `h_flex` or `v_flex`, and on a `Button` — which
     * is the case it exists for, an icon-only control with no text of its own.
     * Anything else needs a wrapper around it to carry the hover.
     *
     * Where the label goes is base's to decide: it is placed against this
     * element and flipped and clamped to stay inside the window. There is no
     * `align` and no `offset`, because base's tooltip has neither, and the
     * side it prefers is not chooseable from a script yet.
     *
     * A tooltip is not a substitute for `accessibility_label`. A screen reader
     * announces the label; the tooltip is for the pointer.
     */
    tooltip(text: string): Element;
    /** Blocks activation and reports the disabled state. Draw it yourself. */
    disabled(value: boolean): Element;
    /** Reports the selected state of a `Button`. */
    selected(value: boolean): Element;
    /**
     * This item's one-based position and its collection's total size, so a
     * screen reader can announce "tab 2 of 5" or "option 2 of 5". Announced,
     * never drawn: a tab list or radio group that omits it looks identical and
     * says nothing about where the reader is in the set.
     */
    set_position(position: number, size: number): Element;
    /** The controlled value of a `Checkbox`, `Switch` or `Radio`. */
    checked(value: boolean): Element;
    /** The controlled state of a `Toggle`: a button that stays down. */
    pressed(value: boolean): Element;
    /**
     * The announced progress percentage of a `Progress`, clamped to `0..=100`.
     *
     * It moves nothing on screen: size the `ProgressIndicator` from the same
     * number to draw the bar.
     */
    value(percent: number): Element;
    /**
     * Withdraws a `Progress` value from the accessibility tree — "still
     * working, no idea how far". It does not animate anything; a barber-pole
     * or a sliding indicator is yours to draw, and `transition` on the
     * indicator is how it moves.
     */
    indeterminate(value: boolean): Element;
    /**
     * What a screen reader announces. An icon-only control has no text of its
     * own and announces nothing without it.
     */
    accessibility_label(description: string): Element;
    /**
     * What this element announces itself as.
     *
     * Only where the element has one to give: a plain `div`, `h_flex` or
     * `v_flex` — which is how a script builds the listbox, toolbar or dialog
     * base has no component for — and a `Button` or `Checkbox`, whose role is
     * an explicit override (a button that opens a menu, a checkbox that is a
     * menu item). Every other component announces a role of its own, and a
     * `role` there is reported and dropped rather than silently overwritten.
     */
    role(name: Role): Element;
    /**
     * The selected state of an option in a list the script built itself.
     *
     * Plain elements only. `Tab` and `Radio` announce their own selection from
     * `selected(...)` and `checked(...)`.
     */
    aria_selected(value: boolean): Element;
    /**
     * Announces this element as the focused one while an ancestor actually
     * holds the keyboard — the highlighted option of a combobox whose input
     * keeps focus. It needs a `role` to produce a node at all, and GPUI
     * ignores the claim unless a focused ancestor is present, so it is safe to
     * set unconditionally on the highlighted child.
     *
     * Plain elements only.
     */
    aria_active_descendant(): Element;
    /**
     * Tracks a `FocusHandle` the script owns, so `handle.is_focused()` answers
     * for this element and `handle.focus()` moves the keyboard onto it.
     *
     * Honoured by plain elements and by `Button`, `Checkbox`, `Radio`,
     * `Toggle`, `Popup`, `Select` and `Combobox`. `Link`, `Switch` and the rest
     * build their own focus handle and have no builder to replace it; a handle
     * given to one of them is reported and dropped. A `DatePicker` takes its
     * handle in `DatePicker.new(id, handle)` instead.
     *
     * On a `Select` or a `Combobox` this is the *trigger's* handle — what holds
     * the keyboard while the list is shut. Put the same handle on the element
     * you drew as the trigger, or nothing focusable is on screen and Escape and
     * Enter reach nothing.
     */
    track_focus(handle: FocusHandle): Element;
    /**
     * Gives a virtual list the scroll position held by a
     * `VirtualListScrollHandle`, so the script can drive it with
     * `scroll_to_item` and `scroll_to_bottom`.
     *
     * Optional. Without one the list keeps a position of its own, filed under
     * the id it was built with — which is the same place a `Scrollbar` named
     * after that id looks, so the bar works either way.
     */
    track_scroll(handle: import("gpui-base").VirtualListScrollHandle): Element;
    /**
     * Which item a virtual list measures to infer its size across the axis it
     * scrolls: a vertical list takes its width from this item, a horizontal
     * one its height. Defaults to the first.
     *
     * The name is base's own builder, kept verbatim.
     */
    with_item_to_measure_index(index: number): Element;
    /**
     * The handle a `Select` or `Combobox` moves the keyboard to when it opens,
     * and away from when Escape closes it.
     *
     * Put the same handle on the element you drew as the list, and the list can
     * then style itself from `handle.is_focused()`. It does **not** give you
     * arrow-key navigation — see `Select` for what is and is not there.
     */
    content_focus_handle(handle: FocusHandle): Element;
    /**
     * Where this element sits in the window's Tab order. A whole number;
     * setting it also makes the element a tab stop.
     *
     * Honoured by plain elements and by every bound control except `Tab`,
     * `Tabs` and the table, group and progress parts, which base leaves out of
     * keyboard focus entirely.
     */
    tab_index(index: number): Element;
    /**
     * Whether Tab can land on this element. `false` keeps its place in the
     * order without making it reachable, which is what a container that
     * forwards focus to its first child wants.
     */
    tab_stop(value: boolean): Element;
    /** Sets the absolute HTTP(S) target opened by a `Link`. */
    href(url: string): Element;
    /**
     * A stable name for this element, used as its identity.
     *
     * Without one, an element is identified by where it sits in the tree the
     * render built — which shifts the moment a conditional child appears above
     * it, taking the pressed state, the focus and anything else keyed by
     * identity with it. Name anything whose identity has to survive that.
     *
     * Any component whose factory takes an id is already identified by that id
     * and ignores this.
     */
    id(name: string): Element;
    /** Owns wheel and touch scrolling on both axes for overflowing children. */
    overflow_scroll(): Element;
    /** Owns horizontal wheel and touch scrolling for overflowing children. */
    overflow_x_scroll(): Element;
    /** Owns vertical wheel and touch scrolling for overflowing children. */
    overflow_y_scroll(): Element;
    /** Scrolls both axes and paints base-layer scrollbars. */
    overflow_scrollbar(): Element;
    /** Scrolls horizontally and paints a base-layer scrollbar. */
    overflow_x_scrollbar(): Element;
    /** Scrolls vertically and paints a base-layer scrollbar. */
    overflow_y_scrollbar(): Element;
    /**
     * A `Scrollbar`'s visibility policy. Omitted, it follows the theme, which
     * is what every bar painted by `overflow_*_scrollbar` does.
     */
    mode(value: import("gpui-base").ScrollbarMode): Element;
    /**
     * The content size a `Scrollbar` measures its thumb against, in pixels,
     * for when the script knows it and the scroll area does not — a list that
     * paints a window of rows rather than all of them.
     */
    scroll_size(width: number, height: number): Element;
    /**
     * Makes a `Scrollbar` take its viewport from its own box rather than from
     * the scroll area it drives. The way to run a bar down the rows of a table
     * without it reaching up over the fixed header.
     */
    viewport_from_layout(): Element;
    /**
     * How far a `resizable_panel()` may be dragged, in pixels.
     *
     * Two arguments rather than a range, which JavaScript cannot write. The
     * minimum is required — a panel always has one, and base's own is 100 —
     * while the maximum is optional and defaults to unbounded. Omit the call
     * entirely to keep both of base's defaults.
     */
    size_range(min: number, max?: number): Element;
    /**
     * `handler(sizes, cx)` on an `h_resizable()` or `v_resizable()`, once a drag
     * of one of its handles has ended. `sizes` is the pixel size of every panel,
     * in the order they were added.
     *
     * Nothing has to be done with it. The sizes live in the window, keyed by the
     * group's id, so dragging works and survives repaints whether or not this is
     * wired: it is for persisting a layout or showing a width, not for making
     * the group resize.
     */
    on_resize(handler: (sizes: number[], cx: Context) => void): Element;
    /**
     * The orientation a `RadioGroup` or `ToggleGroup` announces.
     *
     * Semantic only: it does **not** lay the group out. A group is a plain
     * block until the script says `.flex().flex_row()` or `.flex_col()`, so set
     * both — the axis for what a screen reader says, the layout for what is
     * drawn. Omitted, each container keeps its own default: `RadioGroup` is
     * vertical, `ToggleGroup` horizontal.
     */
    axis(value: Axis): Element;
    /**
     * A `Table`'s total number of rows, including rows outside the range the
     * script rendered, so a screen reader can announce "row 5 of 200". A table
     * that draws every row it has does not need it.
     */
    row_count(count: number): Element;
    /** A `Table`'s total number of columns, including unrendered ones. */
    column_count(count: number): Element;
    /**
     * Whether a `Collapsible` renders the element in its `content` slot — its
     * ordinary children are rendered either way — or whether a `Popover`,
     * `Select`, `Combobox` or `DatePicker` is showing.
     *
     * Setting it at all makes a `Popover` controlled: the script holds the open
     * state, is told about every change through `on_open_change`, and decides
     * what to do about it. Leaving it off leaves the popover to open and close
     * itself from `default_open`. The three combobox roots have no uncontrolled
     * mode at all: they start shut and stay shut until the script says
     * otherwise.
     *
     * A `Popup` has no open state to set. It shows whatever is in its `content`
     * slot, so `.when(open, el => el.content(...))` is how one is opened.
     */
    open(value: boolean): Element;
    /**
     * Whether a `Popover` starts open. Read once, when the surface is first
     * described; a controlled popover ignores it from then on.
     */
    default_open(value: boolean): Element;
    /**
     * Whether pressing outside an open `Popover` closes it. Default `true`.
     */
    overlay_closable(value: boolean): Element;
    /**
     * Which corner of a `Popover` or `HoverCard` is pinned to its trigger, or
     * where an `fps_monitor()` is pinned inside its relative parent. Omitted,
     * each keeps its own default: `Popover` is `top_left`, `HoverCard` is
     * `top_center`, and `fps_monitor()` is `top_right`.
     *
     * The surface is clamped into the window either way, so an anchor near an
     * edge is a preference rather than a promise.
     */
    anchor(value: Anchor): Element;
    /** Frame budget, in milliseconds, used by an fps_monitor's FRAME grading. */
    frame_budget(milliseconds: number): Element;
    /** Which pointer button opens a `Popover`. Default `left`. */
    mouse_button(value: MouseButton): Element;
    /**
     * How long, in milliseconds, the pointer must rest on a `HoverCard`'s
     * trigger before the card appears. Default 600.
     */
    open_delay(ms: number): Element;
    /**
     * How long, in milliseconds, a `HoverCard` waits after the pointer leaves
     * both the trigger and the card before closing. Default 300; it is what
     * lets the pointer cross the gap between the two.
     */
    close_delay(ms: number): Element;
    /** Animates later target changes entirely in native GPUI code. */
    transition(property: import("gpui-shell").MotionProperty, policy: number | import("gpui-shell").TransitionPolicy): Element;
    /** Springs later target changes entirely in native GPUI code. */
    spring(property: import("gpui-shell").MotionProperty, policy?: import("gpui-shell").SpringPolicy): Element;

    /**
     * Which thumb of a range slider a `SliderThumb` is: the one at the start
     * of the range, or the one at its end. Default `false`, the end — which
     * is the only thumb a single-value slider has.
     */
    start(value: boolean): Element;
    /**
     * How the filled part of a `SliderIndicator` looks. `declare` receives a
     * detached element that collects the styles, exactly as `hover` does; its
     * return value is ignored.
     *
     * Only how it looks. Where it is comes from the state on every frame,
     * because a fill the script positioned would be frozen at the value the
     * render that positioned it saw — the user drags, the value changes, the
     * screen reader announces the new one, and the bar stays put. An indicator
     * with no `range_style` has no fill at all, which is a slider drawn as a
     * groove and a knob.
     */
    range_style(declare: (el: Element) => Element | void): Element;
    /**
     * How every cell of an `OtpInput` looks. `declare` receives a detached
     * element that collects the styles, exactly as `hover` does; its return
     * value is ignored.
     *
     * Give it a size. The cells are drawn by the shell rather than described
     * by the script, so an `OtpInput` without this one is a row of boxes with
     * no size, no border and no background — nothing on screen at all.
     */
    cell_style(declare: (el: Element) => Element | void): Element;
    /**
     * Layered on top of `cell_style` for the one cell the next digit lands in,
     * while the code holds the keyboard and is not disabled. A refinement
     * rather than a replacement, the way `hover` is: declare only what differs.
     */
    cell_active_style(declare: (el: Element) => Element | void): Element;
    /**
     * The blinking mark drawn in that cell while it is still empty. Give it a
     * width, a height and a background; with no `caret_style` there is no
     * caret, and the only sign of where typing goes is `cell_active_style`.
     *
     * Not `cursor_style`: everywhere else in this API `cursor` is the pointer.
     */
    caret_style(declare: (el: Element) => Element | void): Element;
    /**
     * Styles applied while the pointer is over the element. `declare` receives
     * a detached element that collects the styles; its return value is
     * ignored, so a chain and a block body both work.
     */
    hover(declare: (el: Element) => Element | void): Element;
    /** Styles applied while the element is pressed. */
    active(declare: (el: Element) => Element | void): Element;
    /** Styles applied while the element has focus. */
    focus(declare: (el: Element) => Element | void): Element;
    /**
     * Displays the tab at `index` in `group` when this element is clicked.
     *
     * One of the twelve **dock commands**, which are how an element a dock's
     * chrome drew says what it does. A chrome handler runs once per container
     * per frame for as long as the dock is on screen, so it may not register an
     * event handler — one created there would pile up for as long as the dock
     * stood. A command carries no script value: it names a container in the
     * area and what to ask it, and base does the work.
     *
     * Every command takes the object its handler was given — the group, the
     * dock, the tile — as its first argument. They belong on a `div`, an
     * `h_flex` or a `v_flex`; a `Button` builds its own interior and has
     * nowhere to put one.
     */
    select_tab(group: import("gpui-base").DockGroup, index: number): Element;
    /** Closes `panel` when this element is clicked, if its group allows it. */
    close_panel(group: import("gpui-base").DockGroup, panel: number): Element;
    /** Zooms the group in, or back out. */
    toggle_zoom(group: import("gpui-base").DockGroup): Element;
    /**
     * Makes this element the drag source for the tab at `index`, carrying
     * base's own panel payload — so dropping it on another group, or on the
     * area itself, moves the panel there.
     */
    drag_tab(group: import("gpui-base").DockGroup, index: number): Element;
    /**
     * Accepts a dragged panel here. `index` is the slot it lands in; leave it
     * out to append, which is what a drop past the last tab means.
     */
    drop_tab(group: import("gpui-base").DockGroup, index?: number): Element;
    /** Opens or closes the dock when this element is clicked. */
    toggle_dock(dock: import("gpui-base").DockRegion): Element;
    /**
     * Drags the dock's edge. Base clamps every size it is given against the
     * area and the opposite dock, so nothing here has to.
     */
    resize_dock(dock: import("gpui-base").DockRegion): Element;
    /** Drags the tile around its canvas, raising it first. */
    move_tile(tile: import("gpui-base").DockTile): Element;
    /** Drags one edge or corner of the tile. */
    resize_tile(
      tile: import("gpui-base").DockTile,
      side: import("gpui-base").TileResizeSide,
    ): Element;
    /** Brings the tile above the others when this element is pressed. */
    raise_tile(tile: import("gpui-base").DockTile): Element;
    /** Zooms the tile to fill its dock, or back out. */
    toggle_tile_zoom(tile: import("gpui-base").DockTile): Element;
    /** Closes the tile. */
    close_tile(tile: import("gpui-base").DockTile): Element;
"#;

fn shell_types() -> String {
    let mut out = String::new();
    out.push_str("  /** The string forms accepted by gpui-shell's length bridge. */\n");
    out.push_str("  export type LengthString = `${number}px` | `${number}rem` | `${number}%`;\n\n");
    out.push_str("  /** The gpui-shell call scope reported by `cx.phase()`; unrelated to gpui::DispatchPhase. */\n");
    out.push_str("  export type ScopePhase =\n");
    for phase in [
        ScopePhase::Render,
        ScopePhase::Event,
        ScopePhase::Task,
        ScopePhase::Layout,
    ] {
        let _ = writeln!(out, "    | \"{}\"", phase.as_str());
    }
    out.push_str("    | \"none\"\n    ;\n\n");
    out.push_str(SHELL_TYPES);
    out
}

const SHELL_TYPES: &str = r#"  /** A path coordinate in pixels or as a percentage of the painted bounds. */
  export type PathCoordinate = number | `${number}%`;

  /** The property bag carried across the JavaScript view bridge. */
  export type Props = Record<string, any>;

  /** Element-local event bounds assembled by the shell. */
  export interface ElementBounds extends import("gpui-kit").Point {
    width: number;
    height: number;
  }

  export interface DialogOptions {
    escape_dismissable?: boolean;
    backdrop_dismissable?: boolean;
  }

  export interface ToastOptions {
    title: string;
    description?: string;
    level?: "info" | "success" | "warning" | "error";
    timeout?: number | null;
    id?: string;
  }

  export interface TaskOptions {
    /** Defaults to the running view; `null` outlives every view. */
    owner?: import("gpui-kit").View | null;
  }

  export type MotionProperty = "opacity" | "width" | "height" | "left" | "top";
  export type MotionEasing = "linear" | "ease-in" | "ease-out" | "ease-in-out";
  export interface TransitionPolicy {
    /** Duration in milliseconds. */
    duration: number;
    /** Delay in milliseconds. */
    delay?: number;
    easing?: MotionEasing;
  }
  export interface SpringPolicy {
    /** Approximate response period in milliseconds. */
    response?: number;
    /** Damping ratio; 1 has no overshoot. */
    damping?: number;
    /** Settling tolerance in the target's units. */
    epsilon?: number;
  }

"#;

/// The elements GPUI itself draws, and the state handles this runtime keeps for
/// them. `gpui-base`'s components are declared separately, in [`BASE`].
const ELEMENTS: &str = r#"
  /** An element with no layout of its own. */
  export function div(): Element;

  /**
   * A vector image from the application's own directory.
   *
   * The path resolves against the application root — the directory passed to
   * gpui-shell — not against the file that asked for it, the way a web
   * application's public directory works. It inherits the surrounding text
   * color unless it sets its own.
   */
  export function svg(path: string): Element;

  /**
   * A full-color image from the application's own directory.
   *
   * Unlike `svg`, this preserves the source image's colors instead of using it
   * as a theme-tinted icon mask. SVG, PNG, JPEG and other GPUI image formats
   * are supported by the host image loader.
   */
  export function image(path: string): Element;

  /** The visible items, as a half-open `[start, end)` interval. */
  export interface ItemRange {
    start: number;
    end: number;
  }

  /**
   * GPUI's own lazy list: rows of any height, measured as they are drawn.
   *
   * Where `v_virtual_list` places rows by the sizes the script states, `list`
   * asks nothing about size. `render(index, cx)` is called for each item that
   * is on screen, from inside layout as a virtual list's renderer is, and the
   * element it returns is measured; the list keeps those measurements and
   * estimates the rest, so a collection of panels that size to their own
   * content scrolls as one and costs the script only what is visible. The
   * rules of a virtual list's renderer apply unchanged: no handlers and no
   * retained state inside it, and `cx.notify()` is refused there.
   *
   * The list scrolls itself and paints no scrollbar; pair one with it by name,
   * as with a scroll area:
   *
   * ```js
   * v_flex().relative().flex_1().min_h(0)
   *   .child(list("panels", this.panels.length,
   *     (index) => this.panels[index].id,
   *     (index) => this.panel(this.panels[index])))
   *   .child(Scrollbar.vertical("panels").absolute().inset_0());
   * ```
   *
   * The measuring is what it costs: the host is entered once per visible item
   * per frame, where `v_virtual_list` and `uniform_list` are entered once per
   * frame however many rows are on screen. Reach for this when heights are
   * genuinely unequal and unknown — a column of panels, a feed of mixed
   * cards — and for a long run of same-height rows reach for one of the
   * others.
   *
   * One consequence of the per-item call: `get_key`'s uniqueness is checked
   * within a call, so a `list` cannot see that two items share a key, where
   * the other two throw. A duplicate key there quietly gives both items one
   * identity, and `on_item_click` reports it for either.
   *
   * @param id      Identity, and the name a `Scrollbar` pairs with.
   * @param item_count How many items the collection has, visible or not.
   * @param get_key An item's stable domain key, from its current index; the
   *   row's element identity and what `on_item_click` reports.
   * @param render  Called with one index; returns that item's element.
   */
  export function list(
    id: string | number,
    item_count: number,
    get_key: (index: number) => string,
    render: (index: number, cx: Context) => Element,
  ): Element;

  /**
   * GPUI's own uniform list: one row is measured and every row takes its
   * height.
   *
   * The same contract as `v_virtual_list` with a single size, without the
   * size: the first row (or the one `with_item_to_measure_index` names) is
   * measured and the rest are placed by it, so a row's height may come from
   * its content rather than a number in the script. `render(range, cx)` is
   * called with the visible interval and returns one element per item in it,
   * so one frame is one call however many rows are on screen — the same
   * bargain `v_virtual_list` makes, and the reason to prefer this over `list`
   * whenever the rows really are the same height.
   */
  export function uniform_list(
    id: string | number,
    item_count: number,
    get_key: (index: number) => string,
    render: (range: ItemRange, cx: Context) => Element[],
  ): Element;

  /** Immutable native GPUI geometry produced by `PathBuilder.build()`. */
  export interface Path {}
  export interface PathBuilder {
    move_to(x: import("gpui-shell").PathCoordinate, y: import("gpui-shell").PathCoordinate): PathBuilder;
    line_to(x: import("gpui-shell").PathCoordinate, y: import("gpui-shell").PathCoordinate): PathBuilder;
    curve_to(to_x: import("gpui-shell").PathCoordinate, to_y: import("gpui-shell").PathCoordinate, control_x: import("gpui-shell").PathCoordinate, control_y: import("gpui-shell").PathCoordinate): PathBuilder;
    cubic_bezier_to(to_x: import("gpui-shell").PathCoordinate, to_y: import("gpui-shell").PathCoordinate, control_a_x: import("gpui-shell").PathCoordinate, control_a_y: import("gpui-shell").PathCoordinate, control_b_x: import("gpui-shell").PathCoordinate, control_b_y: import("gpui-shell").PathCoordinate): PathBuilder;
    arc_to(radius_x: import("gpui-shell").PathCoordinate, radius_y: import("gpui-shell").PathCoordinate, rotation: number, large_arc: boolean, sweep: boolean, to_x: import("gpui-shell").PathCoordinate, to_y: import("gpui-shell").PathCoordinate): PathBuilder;
    add_polygon(points: ReadonlyArray<readonly [import("gpui-shell").PathCoordinate, import("gpui-shell").PathCoordinate]>, closed?: boolean): PathBuilder;
    close(): PathBuilder;
    dash_array(values: readonly number[]): PathBuilder;
    build(): Path;
  }
  export const PathBuilder: {
    fill(): PathBuilder;
    stroke(width: number): PathBuilder;
  };
  export interface BackgroundStop {}
  export interface Background {
    opacity(factor: number): Background;
    color_space(space: "srgb" | "oklab"): Background;
  }
  export const Background: {
    solid(color: Color): Background;
    stop(color: Color, percentage: number): BackgroundStop;
    linear_gradient(angle: number, from: Color | BackgroundStop, to: Color | BackgroundStop): Background;
    pattern_slash(color: Color, width: number, interval: number): Background;
    checkerboard(color: Color, size: number): Background;
  };

  /**
   * A focus target the script owns, created once and kept on the view.
   *
   * Focus is a fact about the window that outlives any one render, so an
   * element rebuilt every frame cannot own it. Hand the handle to an element
   * with `track_focus(...)`, and it is that element the keyboard means.
   *
   * Built with `cx.focus_handle()`, mirroring `App::focus_handle`.
   */
  export interface FocusHandle {
    /** Moves the keyboard onto the element tracking this handle. */
    focus(): void;
    /** Whether the element tracking this handle currently has the keyboard. */
    is_focused(): boolean;
    release(): boolean;
  }

"#;

/// Shared gpui-base types used by its component constructors and Window extensions.
const BASE_SHARED_TYPES: &str = r#"
  /** One of the four edges used to place an element. Mirrors `gpui_base::Placement`. */
  export type Placement = "top" | "bottom" | "left" | "right";

  /** A component identified across renders by `new(id)`. */
  export interface ComponentType {
    new: (id: string | number) => Element;
  }

  /** A sub-part with no identity of its own, constructed with `new()`. */
  export interface PartType {
    new: () => Element;
  }

"#;

/// The window the script draws into.
const WINDOW: &str = r#"
  export interface Window {
    /**
     * Opens a dialog on the window's root, and answers the stack's new depth.
     *
     * Takes a **function returning an element**, not an element: an element
     * belongs to the render pass that built it, and a dialog outlives the call
     * that opened it. The function runs when the dialog draws, and again
     * whenever it redraws. Whatever it closes over is the dialog's state.
     *
     * Legal from an event handler or a task, not from `render`.
     */
    open_dialog(content: () => Element, options?: import("gpui-shell").DialogOptions): number;
    /** Closes the topmost dialog, and answers whether it found one. */
    close_dialog(): boolean;
    /** Closes every dialog, and answers how many it closed. */
    close_all_dialogs(): number;
    /** Whether any dialog is open. Legal from `render`, unlike the rest. */
    has_active_dialog(): boolean;

    /**
     * Opens the sheet on the right, replacing whatever was there. At most one is
     * ever open.
     */
    open_sheet(content: () => Element): void;
    /** The same, anchored at the `gpui-base` placement you name. */
    open_sheet_at(placement: import("gpui-base").Placement, content: () => Element): void;
    /** Closes the sheet, and answers whether one was open. */
    close_sheet(): boolean;
    /** Whether the sheet is open. Legal from `render`, unlike the rest. */
    has_active_sheet(): boolean;

    /** Posts a toast, and answers its id — the generated one when none was given. */
    push_toast(options: import("gpui-shell").ToastOptions): string;
    /** Retracts one toast by id, and answers whether it was still showing. */
    remove_toast(id: string): boolean;
    /** Retracts every toast, and answers how many it retracted. */
    clear_toasts(): number;

    /**
     * Key-value storage that survives a restart, backed by a file the host
     * placed. Needs the `storage` capability.
     */
    readonly localStorage: Storage;
    /**
     * Key-value storage for this run of the application, held in memory and
     * gone when the process exits. Needs no capability: a script that runs may
     * already hold its own memory.
     */
    readonly sessionStorage: Storage;

    /**
     * Paints immutable GPUI geometry with a reusable native `Background`.
     * `Window::paint_path`.
     *
     * The one element constructor reached through an object rather than as a
     * free function, and it is one because the thing it mirrors is a method on
     * the window rather than on the app. Legal from `render`, unlike the
     * overlays above — it builds a description like any other element.
     */
    paint_path(path: Path, background: Background | Color): Element;

    /**
     * `Window::dispatch_action`. Dispatches an action down this window's focus
     * path, reaching the same handlers a bound chord would.
     *
     * This is how a menu item or a toolbar button does what a keystroke does,
     * without either of them knowing about the other. Illegal from `render`.
     */
    dispatch_action(action: string): void;

    /**
     * `Window::rem_size`. The pixel value one `rem` currently means.
     *
     * Legal from `render`, like every measurement below it: a view that sizes
     * itself from the window has to ask during the pass that draws it.
     */
    rem_size(): number;
    /** `Window::line_height`, in pixels. */
    line_height(): number;
    /** `Window::viewport_size`: the drawable area, in pixels. */
    viewport_size(): Size;
    /** `Window::bounds`: where the window is on screen, and how big. */
    bounds(): import("gpui-shell").ElementBounds;
    /** `Window::mouse_position`, in window coordinates. */
    mouse_position(): Point;
    /**
     * `Window::appearance`, reduced to the two a script can draw for.
     *
     * GPUI reports four — each of light and dark has a vibrant variant — but
     * the difference is in how the platform paints *behind* the window, which
     * a script neither controls nor needs to branch on.
     */
    appearance(): "light" | "dark";
    /** `Window::is_window_active`: whether this window has the platform's focus. */
    is_window_active(): boolean;
    /** `Window::is_fullscreen`. */
    is_fullscreen(): boolean;
    /** `Window::is_maximized`. */
    is_maximized(): boolean;

    /**
     * `Window::set_rem_size`. Rescales everything expressed in rems.
     *
     * Illegal from `render`, as is everything below it: a frame that changes
     * the window it is drawing into is a frame arguing with itself. Call it
     * from an event handler or a task.
     */
    set_rem_size(size: number): void;
    /**
     * `Window::refresh`: redraw every view in this window, not just this one.
     *
     * The most expensive call on this object, and the one easiest to reach for.
     * Every view rebuilds -- retained children, charts, virtualized lists, the
     * lot -- so calling it where `cx.notify()` would do turns one view's update
     * into all of them, and calling it per incoming message turns a data feed
     * into a frame-rate problem. An application that pushed a quote through it
     * for each tick of a live market watchlist measured seven frames a second.
     *
     * Reach for it only when there is genuinely no view to notify:
     *
     * - `cx.notify()` repaints the view that owns the state that changed, which
     *   is almost always the right call.
     * - `handle.set_props(...)` repaints a nested view from its parent.
     * - A dock panel is the case that tempts you here, because a panel rebuilt
     *   by `DockArea.load` is not the instance the script created and
     *   `set_props` on the old handle reaches nothing. If you must refresh for
     *   that reason, coalesce: let a timer collect a burst into one call rather
     *   than making one per event.
     */
    refresh(): void;
    /** `Window::focus_next`: move the keyboard to the next tab stop. */
    focus_next(): void;
    /** `Window::focus_prev`: move it to the previous one. */
    focus_prev(): void;
    /** `Window::activate_window`: bring this window to the front. */
    activate_window(): void;
    /** `Window::minimize_window`. */
    minimize_window(): void;
    /** `Window::zoom_window`: the platform's zoom, not a scale factor. */
    zoom_window(): void;
    /** `Window::toggle_fullscreen`. */
    toggle_fullscreen(): void;
  }

"#;

/// Everything `gpui-base` provides: its layout helpers, its components and its
/// theme. Emitted into `declare module "gpui-base"`, so an import says which
/// layer a script is reaching for.
const BASE: &str = r#"  /** A row. */
  export function h_flex(): Element;
  /** A column. */
  export function v_flex(): Element;

  /** Activation, focus, disabled and selected state. No styling. */
  export const Button: ComponentType;
  /** An external HTTP(S) resource opened through the system browser. */
  export const Link: ComponentType;
  /** A controlled toggle. No styling: draw the indicator yourself. */
  export const Checkbox: ComponentType;
  /** A controlled switch. No styling. */
  export const Switch: ComponentType;
  /** Rich HTML or Markdown text. CSS in HTML is not supported. */
  export interface TextViewElement extends Element {
    /** Overrides TextView's default URL opening and reports the resolved URL. */
    on_link_click(handler: (url: string, cx: Context) => void): TextViewElement;
    selectable(value?: boolean): TextViewElement;
    scrollable(value?: boolean): TextViewElement;
  }
  export const TextView: {
    html(id: string, html: string): TextViewElement;
    markdown(id: string, markdown: string): TextViewElement;
  };
  /**
   * A tab list. It holds no selection of its own — each `Tab` is told whether
   * it is selected, and reports activation through `on_click`, so the script
   * keeps the selected index in its own state.
   */
  export const Tabs: ComponentType;
  /** One tab. Controlled: `selected(...)` in, `on_click(...)` out. */
  export const Tab: ComponentType;
  /**
   * The progress root: the announcement, not the bar.
   *
   * It carries the progress role and the `0..=100` value a screen reader reads
   * out, and draws exactly what any other empty element draws — nothing. The
   * visible bar is a `ProgressTrack` you size and color, holding a
   * `ProgressIndicator` whose width you set from the same number you passed to
   * `value`. `Progress.new(...)` on its own puts nothing on screen.
   */
  export const Progress: ComponentType;
  /**
   * The groove. A plain element with your styles on it and no semantics of its
   * own: give it a width, a height and a background, and put the indicator in
   * it.
   */
  export const ProgressTrack: PartType;
  /**
   * The filled part. A plain element too — set its width from the percentage
   * you announced, and add `transition("width", ...)` if it should slide.
   */
  export const ProgressIndicator: PartType;
  /**
   * An avatar root. It renders its `image` slot, or its `fallback` slot when
   * there is no image, and never both.
   *
   * That choice is the whole of what it does. It draws no circle, no size and
   * no background, so the picture is yours: `w`, `h`, `rounded_full` and a
   * background go on the root, and the fallback is styled where it is written.
   *
   * ```js
   * Avatar.new().w(40).h(40).rounded_full().overflow_hidden()
   *   .image(AvatarImage.new("avatars/ada.png").size_full())
   *   .fallback(AvatarFallback.new().size_full().items_center().justify_center().child("AL"));
   * ```
   *
   * Ordinary children are drawn beside whichever slot won, which is where a
   * status dot or a badge goes.
   */
  export const Avatar: PartType;
  /**
   * The image slot: a picture from the application's own directory, at the
   * same kind of path `image(...)` takes.
   *
   * It is a slot type, not an element — used as an ordinary child it draws
   * nothing and says so in the log. Give it `size_full()` unless you want it at
   * its natural size.
   */
  export const AvatarImage: { new(path: string): Element };
  /**
   * The fallback slot: an ordinary box holding whatever stands in for the
   * image — initials, a shape, an `svg(...)`.
   *
   * A slot type like `AvatarImage`, and worth filling: an `Avatar` with an
   * image path that does not resolve has nothing else to show.
   */
  export const AvatarFallback: PartType;
  /**
   * A pagination root: a navigation landmark carrying the announced label, and
   * nothing on screen.
   *
   * The page buttons are yours. What base contributes that you cannot write
   * for yourself is which page numbers to show — that is `pagination_items`
   * below, a calculation rather than a component.
   *
   * ```js
   * Pagination.new("results").accessibility_label("Results").h_flex().gap_1().children(
   *   pagination_items(this.page, this.pages).map((item) =>
   *     item.ellipsis
   *       ? div().child("…")
   *       : Button.new(`page-${item.page}`)
   *           .selected(item.page === this.page)
   *           .on_click((_, cx) => { this.page = item.page; cx.notify(); })
   *           .child(String(item.page)),
   *   ),
   * );
   * ```
   */
  export const Pagination: ComponentType;
  /**
   * An accordion root: a group holding items, and nothing on screen.
   *
   * None of the five parts draws anything — no chevron, no border, no
   * animation, no layout. What they carry is what a screen reader reads: the
   * group, the heading and its level, the button and its expanded state, and
   * the region that button controls.
   *
   * The item owns `open` and passes it down to both the trigger and the panel,
   * so it is set once rather than three times in agreement with itself.
   *
   * ```js
   * Accordion.new("faq").child(
   *   AccordionItem.new()
   *     .open(this.open === "shipping")
   *     .header(
   *       AccordionHeader.new(
   *         AccordionTrigger.new("shipping-trigger")
   *           .on_change((open, cx) => { this.open = open ? "shipping" : null; cx.notify(); })
   *           .child("Shipping"),
   *       ).aria_level(3),
   *     )
   *     .panel(AccordionPanel.new().child("Two to five business days.")),
   * );
   * ```
   */
  export const Accordion: ComponentType;
  /**
   * One item. `open(...)` in, and the trigger's `on_change(...)` out.
   *
   * `disabled(true)` stops the trigger under it responding, whatever the
   * trigger itself says.
   */
  export const AccordionItem: PartType;
  /**
   * The heading that owns one item's trigger, which it takes at construction
   * for the same reason `Popup.new` takes its own: a heading whose button
   * arrived a frame later would announce nothing in between.
   *
   * `aria_level(n)` is what a screen reader reads out — "heading level 3" —
   * and defaults to 3. It announces; it does not size any text.
   */
  export const AccordionHeader: { new(trigger: Element): Element };
  /**
   * The region an item reveals. Left out of the tree entirely while shut,
   * unless `keep_mounted(true)` — which is how its content keeps a scroll
   * position or a half-typed field across a close and reopen.
   */
  export const AccordionPanel: PartType;
  /**
   * The button. It announces the item's expanded state and asks for the
   * opposite: `on_change` receives `true` when a shut item was pressed.
   *
   * `open` and `disabled` come from the item, so setting them here is
   * overwritten. Without an `on_change` nothing can open.
   */
  export const AccordionTrigger: ComponentType;
  /**
   * A calendar's month, and the date chosen in it. Retained: create it in
   * `init`, never in `render`.
   *
   * `month_days()` is why this exists — which dates fall in which week, where
   * the neighbouring months' days go, and how many weeks this month needs.
   * You draw the cells: a button per day, styled how you like.
   *
   * Base's `Calendar` element is deliberately not bound. It walks the same
   * grid calling a renderer once per cell — up to forty-two crossings into
   * JavaScript per frame, from inside GPUI's layout pass, for cells that carry
   * no behavior. Reading the grid here and drawing it yourself is the same
   * work without them.
   *
   * ```js
   * const grid = this.calendar.month_days()[0];
   * v_flex().children(grid.map((week) =>
   *   h_flex().children(week.map((day) =>
   *     Button.new(day)
   *       .selected(day === this.calendar.value())
   *       .on_click((_, cx) => { this.calendar.set_value(day); cx.notify(); })
   *       .child(String(Number(day.slice(8)))),
   *   )),
   * ));
   * ```
   *
   * Dates are `"YYYY-MM-DD"` — sortable as text, and readable by `new Date(s)`
   * when you need a weekday name or a localized month label.
   */
  export const CalendarState: { new(): CalendarStateHandle };
  /** A selected date: one day, a `[start, end]` range, or nothing. */
  export type CalendarDate = string | [string | null, string | null] | null;
  export interface CalendarStateHandle {
    /**
     * The grid, as months of weeks of days. One month unless base was asked
     * for more; each week is always seven days, and the first and last carry
     * the neighbouring months' days so the rows line up under their weekday
     * headings.
     */
    month_days(): string[][][];
    /** The year the grid is for. */
    year(): number;
    /** Its month, 1–12. */
    month(): number;
    /** Today, as the state read it when it was created. */
    today(): string;
    /** What is selected. */
    value(): CalendarDate;
    /** Selects a day, a range, or nothing. */
    set_value(next: CalendarDate): void;
    /** Moves the grid forward one month. Illegal from `render`. */
    next_month(): void;
    /** And back one. Illegal from `render`. */
    prev_month(): void;
    /**
     * `"change"` is the only event, and reports a date being selected. As
     * everywhere else, registering twice means the second handler.
     */
    on(event: "change", handler: (date: CalendarDate, cx: Context) => void): boolean;
    release(): boolean;
  }
  /**
   * Which page numbers to draw, and where the gaps fall.
   *
   * Keeps the first page, the last page and a window around the current one,
   * collapsing each broken run into an ellipsis. `visible_pages` defaults to
   * seven and is clamped to a minimum of five; a total of one page or fewer
   * answers an empty list, because a control for a single page is not one.
   *
   * An ellipsis names the pages it stands for, inclusive on both ends, so it
   * can be a "jump to" control rather than inert text.
   *
   * Legal from `render` — it reads nothing and is where the buttons are built.
   */
  export function pagination_items(
    current_page: number,
    total_pages: number,
    visible_pages?: number,
  ): PaginationEntry[];
  /** One entry of the page layout: a page, or a gap standing for a range. */
  export type PaginationEntry =
    | { page: number; ellipsis?: undefined }
    | { ellipsis: [first: number, last: number]; page?: undefined };
  /**
   * One option in a radio group. No styling: draw the dot yourself.
   *
   * Controlled: `checked(...)` in, `on_change(...)` out — but only ever `true`,
   * because a radio cannot deselect itself. The group lives in the script's own
   * state, and so does clearing it.
   */
  export const Radio: ComponentType;
  /**
   * A button that stays down. Controlled: `pressed(...)` in, `on_change(...)`
   * out, carrying the value the script would otherwise have to flip itself.
   *
   * No styling — an unstyled toggle is an invisible hit target with a button
   * role — so the pressed look is the script's, usually through
   * `.when(pressed, el => …)`.
   */
  export const Toggle: ComponentType;

  /**
   * A set of radios, announced as one group. It holds no selection — each
   * radio is told whether it is checked and reports the change back, so the
   * script keeps the chosen value in its own state.
   *
   * `axis` only changes what is announced; the group has no layout until the
   * script gives it one.
   */
  export const RadioGroup: ComponentType;
  /**
   * A set of toggles, announced as a toolbar. Like `RadioGroup` it holds no
   * state of its own, and its `axis` is announced rather than drawn.
   */
  export const ToggleGroup: ComponentType;

  /**
   * A semantic table root, composed the way HTML composes one: no data source
   * and no delegate, just the groups, rows and cells the script nests itself.
   * No styling — draw the grid, the padding and the header weight yourself.
   *
   * `row_count` and `column_count` describe the whole table, including rows the
   * script chose not to render. Give the root an `accessibility_label`; the
   * visual `TableCaption` below is not associated with it by assistive
   * technology.
   */
  export const Table: ComponentType;
  /** The header row group of a `Table`. */
  export const TableHeader: ComponentType;
  /** The body row group of a `Table`. */
  export const TableBody: ComponentType;
  /** One row. `TableRow.new(id, row_index)`, one-based. */
  export const TableRow: { new: (id: string | number, row_index: number) => Element };
  /** One column header. `TableHead.new(id, column_index)`, one-based. */
  export const TableHead: { new: (id: string | number, column_index: number) => Element };
  /** One data cell. `TableCell.new(id, column_index)`, one-based. */
  export const TableCell: { new: (id: string | number, column_index: number) => Element };
  /**
   * The visual slot a caption belongs in. It is an identified container and
   * nothing more: it carries no caption role, so assistive technology does not
   * tie it to the table. Name the `Table` root with `accessibility_label(...)`.
   */
  export const TableCaption: ComponentType;

  /**
   * A row of panes with draggable dividers between them. `v_resizable` is the
   * same thing stacked, and the axis is the constructor: there is no builder to
   * change it, because every panel inside is laid out from it.
   *
   * Children are `resizable_panel()` calls. Anything else is wrapped in a panel
   * with base's default constraints, which is convenient and lossy — a wrapped
   * element has no `size`, `size_range` or `visible` — so name the panels
   * whenever any of the three matters.
   *
   * The group has no size of its own: it fills whatever it is put in, exactly as
   * the Rust does, so give it a height (for `h_resizable`) or a width. Styles
   * written on it land on that frame.
   *
   * Panel sizes are the window's, not the script's. They are kept under the
   * group's id and survive every repaint, so a drag stays where the user put it
   * without any state on the view — and the id must therefore be a stable name,
   * not one built from a loop index.
   *
   * ```js
   * h_resizable("workspace").h(400)
   *   .child(resizable_panel().size(220).size_range(160, 320).child(sidebar))
   *   .child(resizable_panel().child(editor));
   * ```
   */
  export function h_resizable(id: string): Element;
  /** A column of panes with draggable dividers. See `h_resizable`. */
  export function v_resizable(id: string): Element;
  /**
   * One pane of an `h_resizable()` or `v_resizable()`, and only there: a panel
   * anywhere else throws when it is added, because its size and its drag handle
   * both belong to the group.
   *
   * Two method names mean something else here than they do anywhere else,
   * because base's panel has inherent builders that shadow the styles of the
   * same name — this reproduces that shadowing rather than inventing two new
   * words for it:
   *
   * - `size(pixels)` is the panel's initial size along the group's axis, not a
   *   width and a height. Use `w`/`h` for the cross axis.
   * - `visible(value)` is whether the panel is drawn at all, not the
   *   `visibility` style. A hidden panel keeps its place in the group, so its
   *   siblings' sizes are undisturbed while it is away. Default `true`.
   */
  export function resizable_panel(): Element;

  /**
   * A region whose `content` is materialized and rendered only while `open` is
   * true.
   *
   * That gating is the whole of it. Next to `div()` it adds one thing and
   * nothing else: no role, no announced expanded state, no chevron, no
   * animation and no trigger. Ordinary children are always rendered, so the
   * header goes there; the open state, the control that flips it and any
   * transition on the content are the script's own.
   */
  export const Collapsible: PartType;

  /**
   * A surface anchored to a trigger and opened by a press.
   *
   * It owns the press, the anchoring, the dismissal — outside press, Escape —
   * and the focus that moves into the surface and back out again. It draws
   * nothing: the trigger and the content are both elements you build and style,
   * given to `trigger(...)` and `content(...)`.
   *
   * Controlled the way a `Checkbox` is. Read `open(...)` in from your own
   * state, write it back from `on_open_change(...)`. Left uncontrolled, it
   * opens and closes itself from `default_open`, and the script never learns
   * where it got to.
   *
   * `track_focus(handle)` names what takes the keyboard when it opens — the
   * search field of a picker, say — instead of the surface itself.
   */
  export const Popover: ComponentType;
  /**
   * A surface anchored to a trigger and opened by resting the pointer on it.
   *
   * It owns its own open state: there is no `open` to control and no press to
   * handle, only `open_delay` and `close_delay`. Both delays are milliseconds,
   * and the closing one is what lets the pointer cross the gap between the
   * trigger and the card without dismissing it.
   */
  export const HoverCard: ComponentType;

  /**
   * The bare anchored surface underneath `Popover`, for when the open state
   * already belongs to something else.
   *
   * It measures its trigger, pins the chosen corner of the content to it,
   * paints that content in a layer above the rest of the window and keeps it
   * clear of the window edges. It owns nothing else — no press handling, no
   * dismissal, no open state. That is the point: a `Select` already owns those,
   * and a `Popover` underneath it would be a second control fighting the first
   * for the same Escape key.
   *
   * The trigger is a constructor argument, because the trigger's bounds are
   * what the content is anchored to. Open and close it by filling the `content`
   * slot or leaving it empty:
   *
   * ```js
   * Popup.new("options", trigger).anchor("bottom_left")
   *   .when(this.open, el => el.content(v_flex().children(options)))
   * ```
   *
   * A popup is a real element, unlike `Popover`: styles, state styles, `role`
   * and `track_focus` all land on it.
   */
  export const Popup: {
    new: (id: string | number, trigger: Element) => Element;
  };

  /**
   * A combobox root: the semantics and the keyboard, none of the picture.
   *
   * It holds no options and no selected value. What it owns is the combobox
   * role, the announced expanded state, the controlled `open` state, and the
   * transfer of the keyboard between the trigger and the list. Everything on
   * screen is yours — put the trigger and a `Popup` holding the list inside it
   * as ordinary children.
   *
   * Controlled the way a `Checkbox` is: `open(...)` in, `on_open_change(...)`
   * out. `track_focus(...)` names the trigger's focus handle and
   * `content_focus_handle(...)` the list's; without the first, nothing on
   * screen has the keyboard and no key reaches the root at all.
   *
   * **Arrow-key navigation of an open list is yours to write.** Base opens the
   * list on ↑ / ↓ / Enter, moves the keyboard onto the content handle and then
   * expects whatever is inside to run the highlight from its own key bindings.
   * Nothing does that for you — but the pieces are here: put `on_key_down` on
   * the content element the keyboard was moved to and move your own highlight,
   * or bind ↑ / ↓ to actions under a `key_context` of your own. Out of the box
   * the pointer works, Escape closes, Enter and ↓ open, and the highlight does
   * not move; a control shipped that way looks keyboard-operable and is not.
   *
   * **The highlighted option marks itself.** GPUI puts the active descendant on
   * the option element rather than on the container, so the root cannot mark
   * one for you: call `aria_active_descendant()` on whichever option you drew
   * as highlighted, and give it a `role`.
   *
   * ```js
   * Select.new("country")
   *   .accessibility_label("Country")
   *   .open(this.open)
   *   .track_focus(this.trigger_focus)
   *   .content_focus_handle(this.list_focus)
   *   .on_open_change((open, cx) => { this.open = open; cx.notify(); })
   *   .child(
   *     Popup.new("country-list", trigger)
   *       .when(this.open, el => el.content(list)),
   *   );
   * ```
   */
  export const Select: ComponentType;
  /**
   * The same root, keyed and announced as a combobox whose trigger is an
   * editable field — a `Select` with a text input in front of it. Base forwards
   * every builder to `Select` verbatim, so everything above applies here,
   * including what is missing; the one difference is that it has no
   * `accessibility_label` of its own, so name it through the input.
   */
  export const Combobox: ComponentType;
  /**
   * A date-picker root: the combobox role, the announced open state, and the
   * trigger's place in the Tab order. **It holds no date** — the date lives
   * wherever you keep it, and the calendar you draw inside it is your own.
   *
   * The focus handle is a constructor argument because base requires it: the
   * picker takes the keyboard through that handle, and there is no builder to
   * supply one later. `DatePicker.new(id, handle)` throws without a live one.
   *
   * **Enter and Escape do not reach it.** Base's picker handles both actions
   * but sets no key context, and every key binding base installs is scoped to
   * one — so nothing matches the keystroke and `on_open_change` never fires.
   * Open and close it from a press on the trigger you drew instead, and treat
   * `on_open_change` as wired for the day that changes. A `Select` does not
   * have this problem; if you need the keyboard today, build the picker's
   * trigger and calendar inside one.
   */
  export const DatePicker: {
    new: (id: string | number, focus_handle: FocusHandle) => Element;
  };

  /** When a `Scrollbar` shows itself. */
  export type ScrollbarMode = "scrolling" | "hover" | "always";

  /**
   * A scrollbar you place yourself, driving the scroll area that carries the
   * same id.
   *
   * `overflow_y_scrollbar()` is the easy case: a bar along the edges of the
   * element that scrolls. This is the other one — a bar beside a fixed table
   * header, a bar spanning two panes, a bar for a list that paints none of its
   * own. The two halves are matched **by name**, and nothing checks the match
   * before it runs, so both are needed:
   *
   * ```js
   * v_flex().relative().h(240)
   *   .child(v_flex().id("watchlist").size_full().overflow_y_scroll().children(rows))
   *   .child(Scrollbar.vertical("watchlist").absolute().inset_0());
   * ```
   *
   * The area must be the one that actually scrolls: `.id(name)` together with
   * `overflow_scroll` / `overflow_x_scroll` / `overflow_y_scroll`. Not
   * `overflow_y_scrollbar`, which paints a bar of its own and shares nothing.
   * A bar that finds no such area is reported in the log rather than drawn
   * inert.
   *
   * The bar has no size or position of its own — it fills the element it is
   * put in, so that element is the one you place — and its colors come from
   * the theme.
   */
  export const Scrollbar: {
    /** Both axes. */
    new: (id: string | number) => Element;
    /** The horizontal bar alone. */
    horizontal: (id: string | number) => Element;
    /** The vertical bar alone. */
    vertical: (id: string | number) => Element;
  };

  /** The visible items, as a half-open `[start, end)` interval. */
  export type ItemRange = import("gpui-kit").ItemRange;

  /**
   * A list that describes only what is on screen.
   *
   * `render(range, cx)` is called with the visible interval and returns one
   * element per item in it — so a ten-thousand-row list costs the script what a
   * twenty-row one costs. It is the only callback in this API that is not an
   * event handler, and the only one the host calls during a frame rather than
   * between them: GPUI decides which rows exist while it is laying the list
   * out, so the call happens from inside layout, twice per frame (once to
   * measure a representative row, once to place the visible ones).
   *
   * Two consequences follow from that, and both are enforced rather than
   * documented away:
   *
   * * **No handlers inside the renderer.** `on_click` and the rest throw if
   *   called there. Use `on_item_click` on the list — see its note for why.
   * * **No state inside the renderer.** `InputState.new()`, `cx.focus_handle()`
   *   and the rest throw there as they do in `render()`, and `cx.notify()` is
   *   refused: asking for a re-render from inside layout is a loop.
   *
   * What the layout pass does *not* cost you is the `cx` you already had: the
   * renderer is a closure inside `render(cx)`, so the row helpers written
   * against that `cx` — `label(text, cx)`, `surface(cx)` — keep working
   * unchanged. The `cx` the renderer is handed reaches the same window and app,
   * and is there for a renderer written somewhere `render`'s is not in scope.
   *
   * The list paints no scrollbar of its own. Pair one with it by name, exactly
   * as with a scroll area:
   *
   * ```js
   * v_flex().relative().h(400)
   *   .child(v_virtual_list("rows", this.rows.length, 28,
   *     (index) => this.rows[index].id,
   *     (range) => this.rows.slice(range.start, range.end).map(row => text(row.name))))
   *   .child(Scrollbar.vertical("rows").absolute().inset_0());
   * ```
   *
   * @param id      Identity, and the name a `Scrollbar` pairs with.
   * @param item_count How many items the collection has, visible or not.
   * @param item_sizes One number for a uniform extent, or one per item —
   *   heights for `v_virtual_list`, widths for `h_virtual_list`. Base takes a
   *   single vector whose *length* is also the count; splitting the two is a
   *   deliberate difference, because mirroring it would put one number per row
   *   across the language boundary on every render, and a uniform hundred
   *   thousand rows is the case worth making cheap. An array must be exactly
   *   `item_count` long.
   * @param get_key An item's stable domain key, from its current index. It is
   *   the row's element identity, and it is what `on_item_click` reports — so a
   *   click queued before a filter or a sort reordered the list still names the
   *   item whose box was pressed rather than whatever slid into that index.
   *   Required.
   * @param render  Called with the visible range; returns one element per item
   *   in it.
   */
  export function v_virtual_list(
    id: string | number,
    item_count: number,
    item_sizes: number | number[],
    get_key: (index: number) => string,
    render: (range: ItemRange, cx: Context) => Element[],
  ): Element;

  /** `v_virtual_list` along the other axis; `item_sizes` are widths. */
  export function h_virtual_list(
    id: string | number,
    item_count: number,
    item_sizes: number | number[],
    get_key: (index: number) => string,
    render: (range: ItemRange, cx: Context) => Element[],
  ): Element;

  /**
   * A virtual list's scroll position, kept across frames so the script can move
   * it. Create it in `init()` and hand it to the list with `track_scroll`.
   *
   * A list without one still scrolls, and a `Scrollbar` named after the list
   * still drives it; this is only needed to scroll it from code.
   */
  export interface VirtualListScrollHandle {
    /**
     * Puts the item at `index` on screen before the next frame. `"top"` (the
     * default) brings it to the near edge, `"center"` to the middle.
     */
    scroll_to_item(index: number, strategy?: "top" | "center"): void;
    scroll_to_bottom(): void;
    /** Releases the handle. Using it afterwards throws. */
    release(): boolean;
  }

  export const VirtualListScrollHandle: {
    new: () => VirtualListScrollHandle;
  };

  /** Payload emitted by retained text state. Submit events carry key modifiers. */
  export interface InputEvent {
    readonly secondary?: boolean;
    readonly shift?: boolean;
  }

  /** The OTP event payload is currently empty; read the value from the state. */
  export interface OtpEvent {}

  /**
   * Retained text state, created once and kept on the view.
   *
   * `InputState.new(...)` needs a live host call, so it belongs in `init` or in
   * an event handler — never in `render`.
   */
  export interface InputState {
    value(): string;
    set_value(next: string): void;
    /** `change`, `submit`, `focus` or `blur`. */
    on(event: "change" | "submit" | "focus" | "blur", handler: (event: InputEvent, cx: Context) => void): boolean;
    /**
     * How much one step moves the value in a `NumberInput`. Default is 1;
     * `null` gives up stepping entirely.
     *
     * There is no numeric state type — the step, the bounds and the mask are
     * fields on this one, so a text state becomes a number state by being told
     * about them.
     */
    set_step(step: number | null): void;
    /** The lowest value stepping and blurring clamp to. `null` removes it. */
    set_min(min: number | null): void;
    /** The highest value stepping and blurring clamp to. `null` removes it. */
    set_max(max: number | null): void;
    /** Draws the text as a password. */
    set_masked(masked: boolean): void;
    /** Marks the state as working; the presentation is the application's. */
    set_loading(loading: boolean): void;
    release(): boolean;
  }

  export const InputState: {
    new: (options?: { placeholder?: string; value?: string }) => InputState;
  };

  /** The frame around retained text state. */
  export const Input: { new: (state: InputState) => Element };

  /**
   * A spinbutton over the same `InputState` an `Input` holds.
   *
   * There is no numeric state type. Give an ordinary `InputState` a
   * `set_step(...)` — and a `set_min(...)`/`set_max(...)` if the value is
   * bounded — and hand it here.
   *
   * Three slots, and all three carry weight: `input` (defaults to the bare
   * editor), `decrement_button` and `increment_button`. The base layer's step
   * buttons are unstyled, so an undecorated one is invisible and unhittable.
   *
   * Up and Down step it from the keyboard with nothing wired: the frame
   * declares its own key context, which the two bindings are registered
   * against.
   */
  export const NumberInput: { new: (state: InputState) => Element };

  /**
   * Retained multi-line text state, created once and kept on the view.
   *
   * Like `InputState.new(...)` this needs a live host call, so it belongs in
   * `init` or in an event handler — never in `render`.
   *
   * Give it a height. Being multi-line is carried by the state's mode rather
   * than by its layout, so the layout default is a single row even here: a
   * textarea that says nothing else is the height of an input. Pass `rows`,
   * call `set_auto_grow(...)`, or size the element with `.h(...)`.
   */
  export interface TextareaState {
    value(): string;
    set_value(next: string): void;
    /** `change`, `submit`, `focus` or `blur`. */
    on(event: "change" | "submit" | "focus" | "blur", handler: (event: InputEvent, cx: Context) => void): boolean;
    /** Shows this many rows. */
    set_rows(rows: number): void;
    /** Grows with the content, between the two row counts. */
    set_auto_grow(min_rows: number, max_rows: number): void;
    /** Wraps long lines instead of scrolling sideways. Default is on. */
    set_soft_wrap(wrap: boolean): void;
    release(): boolean;
  }

  export const TextareaState: {
    new: (options?: { placeholder?: string; value?: string; rows?: number }) => TextareaState;
  };

  /** The frame around retained multi-line text state. */
  export const Textarea: { new: (state: TextareaState) => Element };

  /** One thumb, or the two ends of a range. */
  export type SliderValue = number | [number, number];

  /**
   * Retained slider state, created once and kept on the view.
   *
   * Like `InputState.new(...)` this needs a live host call, so it belongs in
   * `init` or in an event handler — never in `render`.
   *
   * It is where a drag writes: the pointer moves, GPUI updates this, and the
   * next frame reads it back without the script being asked to describe
   * anything. Which is why the value is read out of the state — `value()` —
   * rather than held beside it: a copy in the view would be a copy the drag
   * never updated.
   */
  export interface SliderState {
    /** The current value: a number, or `[start, end]` for a range slider. */
    value(): SliderValue;
    set_value(next: SliderValue): void;
    min_value(): number;
    max_value(): number;
    step_value(): number;
    /**
     * `change` arrives on every pixel of a drag; `release` arrives once, when
     * the pointer is let go. Take the first for a live readout and the second
     * for anything that costs something — a request, a write, an undo entry.
     */
    on(event: "change" | "release", handler: (value: SliderValue, cx: Context) => void): boolean;
    release(): boolean;
  }

  export const SliderState: {
    /**
     * Defaults are `0..100` in steps of 1, starting at `min`.
     *
     * A `"logarithmic"` scale needs a `min` above zero — it maps through
     * `log(value / min)`, which has no answer at or below it.
     */
    new: (options?: {
      min?: number;
      max?: number;
      step?: number;
      scale?: "linear" | "logarithmic";
      value?: SliderValue;
    }) => SliderState;
  };

  /**
   * A slider, in four parts, none of which draws anything on its own.
   *
   * ```js
   * Slider.new(this.volume).child(
   *   SliderTrack.new(this.volume).flex().items_center().h(24).w_full().child(
   *     SliderIndicator.new(this.volume)
   *       .relative().w_full().h(6).rounded(3).bg(`#e5e7eb`)
   *       .range_style((fill) => fill.rounded(3).bg(`#2563eb`))
   *       .child(SliderThumb.new(this.volume).size(16).rounded(8).bg(`#2563eb`).ml(-8)),
   *   ),
   * );
   * ```
   *
   * All four are needed and all four take the same state. The root announces
   * the value and owns the release; the track takes the press and the drag;
   * the indicator records the box every pointer position is measured against —
   * **a slider with no `SliderIndicator` cannot be moved at all**, which is
   * reported in the log rather than drawn; the thumb drags itself.
   *
   * The two boxes that depend on the value — the fill and the thumb — are
   * positioned by the shell, from the state, on every frame. That is not a
   * convenience: a drag never re-enters the script, so a position the script
   * computed would be the one the last render saw, and the slider would
   * announce a value its knob had never moved to. Give the thumb a size and a
   * look; the shell gives it a place.
   *
   * `axis("vertical")` is announced *and* used to place both, and each part is
   * told separately, as in Rust. A vertical slider grows from the bottom.
   */
  export const Slider: { new: (state: SliderState) => Element };
  /** The press and drag surface. Give it the height a pointer can hit. */
  export const SliderTrack: { new: (state: SliderState) => Element };
  /**
   * The groove, and the part that records the geometry. It must span the whole
   * travel of the slider: the box it records is what every pointer position is
   * divided by, so an indicator sized to the value would make the value its own
   * scale.
   */
  export const SliderIndicator: { new: (state: SliderState) => Element };
  /**
   * The knob. `start(true)` is the lower thumb of a range slider; the default
   * is the upper one, which is the only thumb a single-value slider has.
   *
   * Unlike the other three it keeps `id(...)`, because two thumbs share one
   * state and a `transition("left", ...)` needs to know which of them it is
   * following.
   */
  export const SliderThumb: { new: (state: SliderState) => Element };

  /**
   * Retained one-time-code state, created once and kept on the view.
   *
   * Like `InputState.new(...)` this needs a live host call, so it belongs in
   * `init` or in an event handler — never in `render`.
   *
   * The length is fixed when the state is created, because it is what the
   * state is: the base layer has no setter for it.
   */
  export interface OtpState {
    /** The digits entered so far — shorter than `len()` until the code is complete. */
    value(): string;
    /**
     * Sets the code from the script. Deliberately unfiltered, as in the base
     * layer: only keystrokes are digits-only. Anything past `len()` is stored
     * but never drawn.
     */
    set_value(next: string): void;
    /** How many cells there are. Fixed when the state was created. */
    len(): number;
    is_masked(): boolean;
    /** Hides the digits behind a bullet, without changing `value()`. */
    set_masked(masked: boolean): void;
    /** Moves the keyboard onto the code. */
    focus(): void;
    /**
     * `change` arrives after each edit; `complete` arrives when the last digit
     * lands. There is no `submit` — the base layer never emits one for a code
     * — and there is no event for the blink.
     */
    on(event: "change" | "complete" | "focus" | "blur", handler: (event: OtpEvent, cx: Context) => void): boolean;
    release(): boolean;
  }

  export const OtpState: {
    /** `length` is the number of cells: a whole number between 1 and 64. */
    new: (length: number, options?: { value?: string; masked?: boolean }) => OtpState;
  };

  /**
   * A fixed-length code, drawn cell by cell **by the shell**.
   *
   * ```js
   * OtpInput.new(this.code)
   *   .flex().gap(8)
   *   .cell_style((cell) =>
   *     cell.size(40).flex().items_center().justify_center()
   *       .border_1().border_color(`#d1d5db`).rounded("md"))
   *   .cell_active_style((cell) => cell.border_color(`#2563eb`))
   *   .caret_style((caret) => caret.w(2).h(18).bg(`#111111`))
   * ```
   *
   * Alone among the bound components, its cells are not the script's to
   * describe — only to style. A described cell would be frozen into the
   * snapshot the last render produced and nothing would ever thaw it: even
   * though edits emit `change`, the caret blinks on a native timer that raises
   * no script event at all.
   *
   * So the shell reads the state every frame and decides what each cell holds
   * — a digit, a bullet while the state is masked, the caret, or nothing —
   * and the three templates say what those look like. Lay the cells out by
   * styling the element itself: `.flex().gap(8)`.
   *
   * Children are allowed and are drawn after the cells, not instead of them.
   *
   * Grouping ("123 456") is not offered: the groups would be boxes the shell
   * invents, with no template to say what they look like.
   */
  export const OtpInput: { new: (state: OtpState) => Element };

  /** Where a region sits relative to the center of a dock area. */
  export type DockPlacement = "center" | "left" | "right" | "bottom";

  /** One panel, as `panels()` reports it. */
  export interface DockPanel {
    /** Stable for as long as the panel lives. Pass it to `remove_panel`. */
    readonly id: number;
    /** Namespaced: `shell:<application>/<name>`. */
    readonly name: string;
    readonly placement: DockPlacement;
    /** The container holding it, which is also `group.node` in the chrome. */
    readonly node: number;
    /** Its position in that container. */
    readonly index: number;
    /** Whether it is the one its container is showing. */
    readonly active: boolean;
    readonly visible: boolean;
    readonly closable: boolean;
    readonly zoomable: boolean;
  }

  /** One tab of a group, as a chrome handler is given it. */
  export interface DockTab {
    /** Its position in the group, which is what `select_tab` takes. */
    readonly index: number;
    readonly name: string;
    readonly id: number;
    readonly active: boolean;
    /**
     * Hidden panels are included, and keep their place in tab order — filter
     * on this rather than re-deriving an index into an already filtered list.
     */
    readonly visible: boolean;
    readonly closable: boolean;
    readonly zoomable: boolean;
  }

  /** A tab group, as `tab_bar` and `empty_group` are given it. */
  export interface DockGroup {
    readonly node: number;
    readonly active_index: number;
    readonly zoomed: boolean;
    readonly collapsed: boolean;
    readonly locked: boolean;
    readonly draggable: boolean;
    readonly droppable: boolean;
    readonly closable: boolean;
    readonly tabs: readonly DockTab[];
  }

  /** One dock, as the `dock` handler is given it. */
  export interface DockRegion {
    readonly placement: DockPlacement;
    /** Its extent along its own axis: width for left and right, height for bottom. */
    readonly size: number;
    readonly open: boolean;
    readonly collapsible: boolean;
  }

  /** One tile of a tiles canvas, as the two tile handlers are given it. */
  export interface DockTile {
    readonly node: number;
    readonly panel: { readonly name: string; readonly id: number; readonly visible: boolean };
    /**
     * Already resolved — base snaps, clamps and rounds before a skin sees
     * them, so nothing here has to be positioned by hand.
     */
    readonly bounds: import("gpui-shell").ElementBounds;
    readonly z_index: number;
    readonly moving: boolean;
    readonly resizing: boolean;
    readonly closable: boolean;
    readonly zoomed: boolean;
    readonly zoomable: boolean;
  }

  /** Where a dragged panel would land, as the `drop_indicator` handler is given it. */
  export interface DockDrop {
    /** `null` means the drop merges into the group's tabs rather than splitting beside it. */
    readonly placement: Placement | null;
    /** The hovered group's content box, in window coordinates. */
    readonly bounds: import("gpui-shell").ElementBounds;
    /** Where the placeholder starts, relative to `bounds`. */
    readonly from: import("gpui-shell").ElementBounds;
    /** Where it settles. */
    readonly to: import("gpui-shell").ElementBounds;
  }

  /** What `add_panel` is told about the panel it is adding. */
  export interface DockPanelOptions {
    /**
     * What the panel is filed under in a saved layout, and what
     * `DockArea.register_panel` finds it again by. Required.
     */
    name: string;
    /** Default `"center"`. */
    placement?: DockPlacement;
    /** Seeds the dock's extent when the panel is the first thing in it. */
    size?: number;
    /**
     * Places the panel on the region's tiles canvas instead of in a tab group.
     * A region with no canvas has nowhere to put a tile, so nothing happens.
     */
    bounds?: { x: number; y: number; width: number; height: number };
    /** Default `true`. */
    closable?: boolean;
    /** Default `true`. */
    zoomable?: boolean;
    /** Default `true`. */
    visible?: boolean;
  }

  /**
   * A dockable layout: splits, tab groups, docks and tiles that the user can
   * rearrange, and that survives a restart.
   *
   * Retained for a reason none of the other handles share. **The layout is what
   * the user changed** — a drag, a resize, a closed tab and a collapsed dock all
   * happen without the script rendering — so it lives here rather than in a
   * description that would put every one of them back the way the last render
   * described it.
   *
   * `DockArea.new(id)` needs a live host call, so it belongs in `init` or an
   * event handler, never in `render`.
   *
   * **Every edit takes effect once the call that made it has returned.**
   * `add_panel` is handed a view from `cx.new(Class)`, which is itself still
   * being constructed; `load` rebuilds panels, which constructs more. So
   * `panels()` and `dump()` read the layout as it was before this turn's edits,
   * and `on("layout_changed", …)` is where to read it after them.
   *
   * ```js
   * init(_props, cx) {
   *   DockArea.register_panel("inbox", Inbox);
   *   this.dock = DockArea.new("workspace");
   *   this.dock.add_panel(cx.new(Inbox), { name: "inbox", placement: "left", size: 240 });
   *   this.dock.on("layout_changed", () => localStorage.setItem("layout", JSON.stringify(this.dock.dump())));
   * }
   * render() {
   *   return dock_area(this.dock).size_full().tab_bar((group) => …);
   * }
   * ```
   */
  export interface DockArea {
    /** Docks `view` — a view from `cx.new(Class)`, not an element. */
    add_panel(view: import("gpui-kit").Entity, options: DockPanelOptions): void;
    /** Removes the panel with this id, wherever it sits. */
    remove_panel(id: number): void;
    /** Every panel in the area, in tree order. */
    panels(): DockPanel[];
    /**
     * The whole layout as plain data: the tree, the docks, and each panel's own
     * `serialize()` payload. Hand it back to `load` after a restart.
     */
    dump(): any;
    /**
     * Restores a layout `dump()` wrote, rebuilding each panel through the class
     * registered under its name.
     *
     * A panel whose name nothing registered is not dropped: it is carried
     * forward — name, payload and position — so uninstalling an application and
     * reinstalling it puts its panels back where they were.
     */
    load(state: any): void;
    has_dock(placement: DockPlacement): boolean;
    is_dock_open(placement: DockPlacement): boolean;
    toggle_dock(placement: DockPlacement): void;
    remove_dock(placement: DockPlacement): void;
    dock_size(placement: DockPlacement): number | null;
    set_dock_size(placement: DockPlacement, size: number): void;
    set_dock_collapsible(placement: DockPlacement, collapsible: boolean): void;
    /** A locked area cannot be rearranged or dropped into; dock and tile resizing stays available. */
    is_locked(): boolean;
    set_locked(locked: boolean): void;
    is_zoomed(): boolean;
    /** Clears the zoom, whichever container holds it. */
    zoom_out(): void;
    /**
     * Fires on every edit — including each step of a tile drag — so save on a
     * timer rather than on every one.
     */
    on(event: "layout_changed", handler: (cx: Context) => void): boolean;
    release(): boolean;
  }

  export const DockArea: {
    new: (id: string, options?: { version?: number }) => DockArea;
    /**
     * Teaches the runtime to rebuild `name`'s panel from `Class` when a saved
     * layout mentions it, and answers with the namespaced name it registered
     * under.
     *
     * The class is an ordinary view class. Two of its methods carry state
     * across a restart, and both are optional:
     *
     * - `serialize()` returns plain data, and is read when the layout is saved.
     *   It runs without a host call, so it must not touch entities, `cx`, or
     *   anything else that needs one — return a value and nothing else.
     * - `deserialize(data)` is handed back whatever `serialize()` wrote, right
     *   after the view is built, with a real host call available.
     *
     * Registering the same name twice replaces the class, which is what a hot
     * reload does.
     */
    register_panel: (name: string, Class: import("gpui-kit").ViewClass) => string;
  };

  /**
   * Draws a dock area.
   *
   * Base draws **no chrome at all** — an area with none still docks, drags,
   * resizes and persists, painting only the panels — so every tab bar, dock
   * frame and drag bar is one of the six handlers below.
   *
   * Each handler is first called from inside GPUI's layout pass and is given
   * base's own resolved state: never a drag event, a mouse position or a hit
   * test. Its description is cached until that state or the handler changes,
   * so unchanged frames do not enter JavaScript. It may not register event
   * handlers — cached chrome has no script callback lifecycle of its own — so
   * the elements it returns say what they do with a **command** instead:
   * `select_tab(group, i)`, `close_panel(group, id)`, `toggle_dock(dock)`,
   * `move_tile(tile)` and the rest. A command carries no script value, and base
   * does the work.
   */
  export function dock_area(area: DockArea): DockAreaElement;

  export interface DockAreaElement extends Element {
    /** The tab bar above a group's displayed panel. */
    tab_bar(handler: (group: DockGroup, cx: Context) => Element): DockAreaElement;
    /** What a group with no displayed panel shows. */
    empty_group(handler: (group: DockGroup, cx: Context) => Element | null): DockAreaElement;
    /** The hint showing where a dragged panel would land. */
    drop_indicator(handler: (drop: DockDrop, cx: Context) => Element | null): DockAreaElement;
    /**
     * One dock's chrome around its content: title strip, collapse affordance,
     * resize handle. Whatever this returns replaces the content, so put
     * `dock_content()` where the panels belong.
     */
    dock(handler: (dock: DockRegion, cx: Context) => Element | null): DockAreaElement;
    /**
     * The strip a tile is dragged by. Its height is fixed at base's drag-bar
     * height, which the snapping arithmetic assumes.
     */
    tile_drag_bar(handler: (tile: DockTile, cx: Context) => Element): DockAreaElement;
    /** A tile's resize affordances. */
    tile_resize_handles(handler: (tile: DockTile, cx: Context) => Element | null): DockAreaElement;
  }

  /**
   * Where a dock's own panels go inside the chrome the `dock` handler drew
   * around them. Legal only inside that handler, and only once.
   */
  export function dock_content(): Element;

  /** Which edge or corner of a tile a resize handle pulls. */
  export type TileResizeSide = "left" | "right" | "top" | "bottom" | "bottom_right";

  /** Semantic color roles, aligned with `gpui_base::ColorTokens`. */
  export type ColorTokens = { readonly [Role in ColorToken]: Color };
  /** Semantic spacing scale, aligned with `gpui_base::SpacingTokens`. */
  export interface SpacingTokens {
    readonly xxs: number; readonly xs: number; readonly sm: number;
    readonly md: number; readonly lg: number; readonly xl: number; readonly xxl: number;
  }
  /** Semantic radius scale, aligned with `gpui_base::RadiusTokens`. */
  export interface RadiusTokens {
    readonly none: number; readonly sm: number; readonly md: number;
    readonly lg: number; readonly xl: number; readonly full: number;
  }
  /** One entry in the type scale, aligned with `gpui_base::TextStyleToken`. */
  export interface TextStyleToken {
    readonly size: number;
    readonly line_height: number;
    /** The CSS range: 100 is thin, 400 regular, 700 bold. */
    readonly weight: number;
  }
  /**
   * Semantic type scale, aligned with `gpui_base::TypographyTokens`.
   *
   * `md` is the window's base text size: everything the shell draws for itself
   * — toasts, sheets, dialog chrome — inherits it, so an application that
   * draws densely says so here rather than restating a size per component.
   */
  export interface TypographyTokens {
    /** The face the scale is set in, and the scale itself. */
    readonly sans: string;
    readonly xs: TextStyleToken; readonly sm: TextStyleToken; readonly md: TextStyleToken;
    readonly lg: TextStyleToken; readonly xl: TextStyleToken;
    /**
     * Code: a face and one size, not a sixth step of the scale. `mono_md` is
     * the size `mono` is set at, and the two are read together.
     */
    readonly mono: string;
    readonly mono_md: TextStyleToken;
  }
  export interface SemanticThemeTokens {
    readonly colors: ColorTokens;
    readonly spacing: SpacingTokens;
    readonly radius: RadiusTokens;
    readonly typography: TypographyTokens;
  }

  /**
   * Replaces gpui-base's active semantic tokens for the current application.
   * Legal only from an event handler or task backed by a live host call.
   *
   * Colours, spacing and radius are stated in full: a palette with half its
   * roles missing is a window drawn in two themes. Typography is an override —
   * every entry is optional, and one left out keeps the value it has — so a
   * theme that only wants a smaller base says `{ md: { size: 12 } }` rather
   * than restating two font families and six line heights it has no opinion
   * about. A theme that says nothing about type is drawn as it always was.
   */
  export function set_theme(theme: {
    readonly appearance: "light" | "dark";
    readonly tokens: Omit<SemanticThemeTokens, "typography"> & {
      readonly typography?: {
        readonly sans?: string;
        readonly mono?: string;
      } & {
        readonly [Step in keyof Omit<TypographyTokens, "sans" | "mono">]?: {
          readonly [Field in keyof TextStyleToken]?: number;
        };
      };
    };
  }): void;
  /** The Base-aligned semantic tokens plus the current appearance. Read-only. */
  export interface Theme extends SemanticThemeTokens, ColorTokens {
    readonly appearance: "light" | "dark";
    readonly is_dark: boolean;
  }

"#;

/// What `gpui-base`'s declarations borrow from `"gpui-kit"`.
///
/// A component is built out of this runtime's vocabulary and returns an
/// `Element`, so the dependency runs upward only.
const BASE_IMPORTS: &str = r#"  import {
    Color,
    Context,
    Element,
    FocusHandle,
  } from "gpui-kit";

"#;

/// The `gpui-fps` performance overlay: the element form, and the HUD the
/// window root draws on a script's behalf.
const FPS: &str = r#"  /**
   * The native `gpui-fps` performance HUD, shared once per window and pinned
   * to the top-right by default. Its parent must be `relative()`.
   *
   * Prefer `show_fps_monitor()`: a HUD placed inside the script's own tree is
   * rebuilt with it, and what the tree does then counts against the reading.
   */
  export function fps_monitor(): Element;

  /** Where the root-owned HUD sits and how it behaves. Every key is optional. */
  export interface FpsMonitorOptions {
    /** Corner or edge of the window. Default `top_right`. */
    anchor?: Anchor;
    /** Frame budget in milliseconds, for the FRAME grading and the chart's scale. */
    frame_budget?: number;
  }

  /**
   * Draws the performance HUD over the whole window, above every overlay,
   * until `hide_fps_monitor()`. The window root owns it: the script says
   * whether and where, and nothing the script renders can move it, rebuild
   * it, or count against it. Calling it again moves or reconfigures the HUD
   * that is already up; the monitor behind it keeps its history across a hide
   * and a show. Needs a live host call: `init()`, an event handler or a task.
   */
  export function show_fps_monitor(options?: FpsMonitorOptions): void;
  /** Takes the HUD down. `true` if one was up. */
  export function hide_fps_monitor(): boolean;
  /** Whether the root-owned HUD is up. */
  export function fps_monitor_visible(): boolean;
"#;

/// What `gpui-fps`'s declarations borrow from `"gpui-kit"`.
const FPS_IMPORTS: &str = r#"  import { Anchor, Element } from "gpui-kit";

"#;

const CAPABILITIES: &str = r#"
  /** Key-value storage that survives a restart. Persisted on every write. */
  /**
   * The Web Storage API, as a browser implements it.
   *
   * Two instances exist and differ only in how long they last, which is the
   * same split Deno and Node arrived at when they needed persistent key-value
   * storage without a browser: `window.localStorage` is a file and survives a
   * restart, `window.sessionStorage` is memory and is gone when the process
   * exits.
   *
   * **Values are strings.** `setItem` converts whatever it is handed, exactly
   * as the browser does, so an object is stored as `"[object Object]"` unless
   * you `JSON.stringify` it — and reading it back is `JSON.parse`. That is not
   * an omission; it is the API this mirrors.
   *
   * Storage is per application. The host places the file, because an
   * application that could name its own storage location could name another
   * application's.
   */
  export interface Storage {
    /** How many keys are set. */
    readonly length: number;
    /** The key at `index` in a consistent order, or `null` past the end. */
    key(index: number): string | null;
    /** The value, or `null` when the key is unset. */
    getItem(key: string): string | null;
    /** Sets it, converting `value` to a string. */
    setItem(key: string, value: string): void;
    /** Removes it. Removing a key that is not set does nothing. */
    removeItem(key: string): void;
    /** Removes every key. */
    clear(): void;
    /**
     * Resolves once the pending write has reached disk.
     *
     * The one addition to the Web Storage surface, and only on
     * `localStorage`. A browser's `setItem` is durable by the time it returns;
     * this one hands the write to a background thread, so a script that must
     * know the bytes landed — before asking the host to exit, say — has
     * something to await. Ordinary code does not need it.
     */
    flush(): Promise<void>;
  }
"#;

const STANDARD_RUNTIME: &str = r#"
declare module "buffer" {
  export class Buffer extends Uint8Array {
    static from(value: string | ArrayBuffer | ArrayLike<number>, encoding?: string): Buffer;
    static alloc(size: number): Buffer;
    toString(encoding?: string): string;
  }
}
declare module "path" {
  export function join(...parts: string[]): string;
  export function resolve(...parts: string[]): string;
  export function dirname(path: string): string;
  export function basename(path: string, suffix?: string): string;
  const path: { join: typeof join; resolve: typeof resolve; dirname: typeof dirname; basename: typeof basename };
  export default path;
}
declare module "url" {
  export const URL: typeof globalThis.URL;
  export const URLSearchParams: typeof globalThis.URLSearchParams;
}
declare module "crypto" {
  export interface Hash { update(data: string | Uint8Array): Hash; digest(encoding?: string): string | import("buffer").Buffer; }
  export function createHash(algorithm: string): Hash;
  export function randomBytes(size: number): import("buffer").Buffer;
  export function randomUUID(): string;
  export const webcrypto: Crypto;
}
declare module "zlib" {
  export function deflateSync(data: string | Uint8Array): import("buffer").Buffer;
  export function inflateSync(data: Uint8Array): import("buffer").Buffer;
  export function gzipSync(data: string | Uint8Array): import("buffer").Buffer;
  export function gunzipSync(data: Uint8Array): import("buffer").Buffer;
}
interface Console {
  debug(...values: unknown[]): void;
  log(...values: unknown[]): void;
  info(...values: unknown[]): void;
  warn(...values: unknown[]): void;
  error(...values: unknown[]): void;
}
/**
 * Diagnostics. A global, as it is in every other JavaScript runtime, and the
 * only one: the shell used to export the same object a second time as
 * `gpui.log`, which bought a name and nothing else.
 *
 * Needs no capability — a script that runs may say something — and output goes
 * to `tracing` under the `gpui_shell::script` target.
 */
declare const console: Console;
declare module "console" {
  const console: Console;
  export default console;
}
declare module "process" {
  export interface CommandOutput { code: number; stdout: string; stderr: string; }
  export function run(command: string, args?: string[]): Promise<CommandOutput>;
  export function exit(code?: number): void;
  export function nextTick(callback: (...args: unknown[]) => void, ...args: unknown[]): void;
  export const platform: string;
  export const arch: string;
  const process: { run: typeof run; exit: typeof exit; nextTick: typeof nextTick; platform: string; arch: string };
  export default process;
}
declare module "os" {
  export function platform(): string;
  export function arch(): string;
  export const EOL: string;
  const os: { platform: typeof platform; arch: typeof arch; EOL: string };
  export default os;
}
declare module "fs/promises" {
  export interface Dirent { name: string; isDirectory(): boolean; }
  export interface MakeDirectoryOptions { recursive?: boolean; }
  export function readFile(path: string): Promise<Uint8Array>;
  export function readFile(path: string, encoding: "utf8" | { encoding: "utf8" }): Promise<string>;
  export function writeFile(path: string, contents: string | Uint8Array): Promise<void>;
  export function readdir(path: string): Promise<string[]>;
  export function readdir(path: string, options: { withFileTypes: true }): Promise<Dirent[]>;
  export function exists(path: string): Promise<boolean>;
  export function unlink(path: string): Promise<void>;
  export function rmdir(path: string): Promise<void>;
  export function mkdir(path: string, options?: MakeDirectoryOptions): Promise<void>;
}
declare module "net" {
  export interface Socket {
    write(data: string): Promise<void>;
    /** Reads raw bytes. Resolves to null after the peer reaches EOF. */
    read(maxBytes?: number): Promise<Uint8Array | null>;
    close(): void;
  }
  export function connect(host: string, port: number): Promise<Socket>;
  const net: { connect: typeof connect };
  export default net;
}
declare module "websocket" {
  export interface WebSocketSocket {
    /** Waits for the next text or binary message. */
    read(): Promise<string | Uint8Array>;
    /** Sends a text or binary message. */
    write(data: string | Uint8Array): Promise<void>;
    /** Sends and flushes a close frame. */
    close(): Promise<void>;
  }
  export interface WebSocketConnectOptions {
    /** Additional protocol headers. Credential and WebSocket control headers are refused. */
    headers?: Readonly<Record<string, string>>;
  }
  export interface WebSocketType {
    connect(url: string, options?: WebSocketConnectOptions): Promise<WebSocketSocket>;
  }
  /** Capability-gated client sockets; not the browser global constructor. */
  export const WebSocket: WebSocketType;
}
interface ShellFetchResponse {
  readonly status: number;
  readonly ok: boolean;
  readonly url: string;
  text(): Promise<string>;
  json(): Promise<unknown>;
}
interface ShellFetchOptions {
  /**
   * GET by default. Any HTTP method may be named; which of them may reach a
   * given host and path is the plugin's `capabilities.network` policy, not
   * this field.
   */
  method?: string;
  /** Client-managed framing headers such as Host and Content-Length are refused. */
  headers?: Record<string, string>;
  body?: string | Uint8Array;
}
declare function fetch(url: string, options?: ShellFetchOptions): Promise<ShellFetchResponse>;
declare const process: typeof import("process").default;
"#;

/// `window` is a real global — see the runtime's `overlay` module.
///
/// Outside the `declare module` block, which is what makes it global: this file
/// has no top-level import or export, so TypeScript reads it in script mode.
const WINDOW_GLOBAL: &str = r#"
/**
 * The window the script is drawing into. A global: nothing to import, and
 * unlike `cx`, nothing hands it to you.
 *
 * Ambient: every call reads the host call that is running now, and throws
 * outside one. There is no handle to hold, so there is nothing to hold past the
 * call that would have made it stale.
 *
 * An overlay belongs to the window rather than to the view that opened it —
 * `cx.notify()` re-renders this view, `window.open_dialog()` changes what the
 * user is looking at — which is why these are here and not on `Context`.
 */
type GpuiShellWindow = import("gpui-kit").Window;
interface Window extends GpuiShellWindow {}
declare var window: Window & typeof globalThis;

/**
 * `window.localStorage`, reachable bare as it is in a browser, where `window`
 * *is* the global object. Here `window` is an ordinary object, so both
 * spellings are installed rather than one falling out of the other.
 */
declare const localStorage: import("gpui-kit").Storage;
/** `window.sessionStorage`, bare, for the same reason. */
declare const sessionStorage: import("gpui-kit").Storage;
"#;

const SCHEDULING: &str = r#"
  /** A running task. Cancelling one leaves its promise pending for ever. */
  export interface Task {
    cancel(): void;
    is_done(): boolean;
  }

  export interface Timer {
    /** Calls `handler(cx)` once, after `ms`. */
    after(ms: number, handler: (cx: AsyncContext) => void, opts?: import("gpui-shell").TaskOptions): Task;
    /**
     * Calls `handler(cx)` every `ms`. The interval is measured from the end of
     * one call, so a slow handler delays the next tick instead of stacking.
     */
    every(ms: number, handler: (cx: AsyncContext) => void, opts?: import("gpui-shell").TaskOptions): Task;
  }
"#;

#[cfg(test)]
mod tests {
    use gpui::IntoElement as _;

    use super::*;

    #[test]
    fn registered_state_factories_are_declared_from_the_runtime_catalog() {
        let mut registry = crate::ComponentRegistry::new(
            crate::COMPONENT_REGISTRY_API_VERSION,
            crate::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        registry
            .register_state(
                crate::StateDescriptor::new(
                    "InputState",
                    "InputState",
                    vec![crate::ArgumentDescriptor::new(
                        "text",
                        crate::ArgumentSchema::String,
                    )],
                    |_, _, _| Ok(Box::new(())),
                )
                .with_documentation("Retained input state."),
            )
            .unwrap();

        let declarations = declarations_with_components(&registry.freeze().unwrap());

        assert!(declarations.contains("export interface InputState"));
        assert!(declarations.contains("export function InputState(text: string): InputState;"));
        assert!(declarations.contains("Retained input state."));
    }

    /// The element methods that are not style methods, so a test can subtract
    /// them from the interface and compare what is left against the style
    /// table. Mirrors the names bound in the engine's `apply` and prelude.
    const NON_STYLE_METHODS: &[&str] = &[
        "map",
        "child",
        "children",
        "content",
        "trigger",
        "input",
        "decrement_button",
        "increment_button",
        "controls_right",
        "when",
        "on_click",
        "on_mouse_move",
        "on_hover",
        "on_key_down",
        "on_key_up",
        "on_modifiers_changed",
        "on_mouse_down",
        "on_mouse_up",
        "on_mouse_down_out",
        "on_scroll_wheel",
        "on_action",
        "key_context",
        "image",
        "fallback",
        "header",
        "footer",
        "panel",
        "aria_level",
        "keep_mounted",
        "on_item_click",
        "on_item_secondary_click",
        "on_change",
        "on_step",
        "on_open_change",
        "on_confirm",
        "on_dismiss",
        "disabled",
        "selected",
        "checked",
        "accessibility_label",
        "tooltip",
        "role",
        "aria_selected",
        "aria_active_descendant",
        "track_focus",
        "track_scroll",
        "with_item_to_measure_index",
        "content_focus_handle",
        "tab_index",
        "tab_stop",
        "href",
        "id",
        "overflow_scroll",
        "overflow_x_scroll",
        "overflow_y_scroll",
        "overflow_scrollbar",
        "overflow_x_scrollbar",
        "overflow_y_scrollbar",
        "mode",
        "scroll_size",
        "viewport_from_layout",
        "size_range",
        "on_resize",
        "set_position",
        "pressed",
        "start",
        "range_style",
        "cell_style",
        "cell_active_style",
        "caret_style",
        // The dock commands. Not behaviours in the reflected sense either: no
        // handler crosses, only the container an element names and what it asks
        // of it. See `crate::dock::DockCommand`.
        "select_tab",
        "close_panel",
        "toggle_zoom",
        "drag_tab",
        "drop_tab",
        "toggle_dock",
        "resize_dock",
        "move_tile",
        "resize_tile",
        "raise_tile",
        "toggle_tile_zoom",
        "close_tile",
        "value",
        "indeterminate",
        "axis",
        "row_count",
        "column_count",
        "open",
        "default_open",
        "overlay_closable",
        "anchor",
        "frame_budget",
        "mouse_button",
        "open_delay",
        "close_delay",
        "transition",
        "spring",
        "hover",
        "active",
        "focus",
    ];

    /// Every method name declared in the `Element` interface, in order.
    /// A registered component answers `disabled`, `selected` and `on_click`
    /// only when its descriptor declares them. The declarations have to say so,
    /// or an editor green-lights a call that always throws — which is how
    /// `Button.disabled(true)` reached a running application.
    #[gpui::test]
    fn withheld_common_behaviors_are_declared_uncallable_rather_than_removed() {
        use crate::{
            ComponentDescriptor, ComponentMaterializer, ComponentPayload, ComponentRegistry,
            ConstructorDescriptor, MaterializeRequest,
        };
        use std::sync::Arc;

        struct Empty;
        impl ComponentMaterializer for Empty {
            fn materialize(
                &self,
                request: MaterializeRequest<'_>,
            ) -> anyhow::Result<gpui::AnyElement> {
                request.finish(gpui::div())
            }
        }

        let mut registry = crate::ComponentRegistry::new(
            crate::COMPONENT_REGISTRY_API_VERSION,
            crate::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        registry
            .register(
                ComponentDescriptor::new("Plain", Arc::new(Empty))
                    .with_constructors(vec![ConstructorDescriptor::new(
                        "Plain",
                        Vec::new(),
                        |_| Ok(ComponentPayload::new(())),
                    )])
                    .with_documentation("A component declaring no common behavior."),
            )
            .unwrap();
        let _ = &registry as &ComponentRegistry;
        let declarations = super::declarations_with_components(&registry.freeze().unwrap());

        for behavior in super::REGISTERED_COMMON_BEHAVIORS {
            assert!(
                declarations.contains(&format!("{behavior}(unavailable: never): never;")),
                "`{behavior}` must be declared uncallable on a component that does not declare it"
            );
        }
        // Re-declared, not dropped: removing the member would cost the type its
        // assignability to `Element`, so a typed part could no longer be passed
        // to a slot that takes one.
        assert!(
            declarations.contains(
                "export type PlainElement = Omit<Element, \"disabled\" | \"selected\" | \"on_click\">"
            ),
            "{declarations}"
        );
    }

    fn element_methods(declarations: &str) -> Vec<String> {
        declarations
            .lines()
            .skip_while(|line| !line.starts_with("  export interface Element {"))
            .skip(1)
            .take_while(|line| !line.starts_with("  }"))
            .filter_map(|line| {
                let line = line.trim_start();
                let name = line.split(['(', '<']).next()?;
                (!name.is_empty()
                    && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                    && line.len() > name.len())
                .then(|| name.to_owned())
            })
            .collect()
    }

    #[test]
    fn a_reflected_style_is_declared_with_no_arguments() {
        let declarations = base_declarations();
        assert!(declarations.contains("\n    items_center(): Element;\n"));
        assert!(declarations.contains("\n    flex_col(): Element;\n"));
        // Reflection misses the macro-generated font weights; the runtime adds
        // them back, and so must the declarations.
        assert!(declarations.contains("\n    font_semibold(): Element;\n"));
    }

    #[test]
    fn a_parametric_style_is_declared_with_the_type_the_runtime_enforces() {
        let declarations = base_declarations();
        for expected in [
            "    bg(value: Color): Element;",
            "    border_color(value: Color): Element;",
            "    w(value: Length): Element;",
            "    p(value: DefiniteLength): Element;",
            "    gap(value: DefiniteLength): Element;",
            "    rounded(value: AbsoluteLength): Element;",
            "    text_size(value: AbsoluteLength): Element;",
            "    font_weight(value: number): Element;",
            "    opacity(value: number): Element;",
            "    flex_grow(value: number): Element;",
        ] {
            assert!(declarations.contains(expected), "missing: {expected}");
        }
    }

    #[test]
    fn every_parametric_style_is_classified() {
        let (_, parametric) = style_methods();
        assert!(!parametric.is_empty());
        for name in parametric {
            assert_ne!(
                argument_of(name),
                Argument::Unrecognized,
                "`{name}` takes an argument the probe does not recognize; give it a \
                 literal in `argument_of` before the declarations claim it accepts nothing"
            );
        }
    }

    #[test]
    fn colors_require_concrete_values_from_the_theme_api() {
        let declarations = base_declarations();
        for name in color_token_names() {
            assert!(
                declarations.contains(&format!("    | \"{name}\"\n")),
                "`{name}` is missing from ColorToken"
            );
        }
        assert!(declarations.contains("export type Color = `#${string}`;"));
        assert!(!declarations.contains("ColorToken | `#${string}`"));
    }

    #[test]
    fn shared_types_are_declared_by_the_layer_that_owns_them() {
        let declarations = base_declarations();
        let module = |specifier: &str| {
            let start = declarations
                .find(&format!("declare module \"{specifier}\" {{"))
                .unwrap_or_else(|| panic!("missing module {specifier}"));
            let end = start
                + declarations[start..]
                    .find("\n}\n")
                    .expect("unterminated module");
            &declarations[start..end]
        };
        let gpui = module("gpui-kit");
        let base = module("gpui-base");
        let shell = module("gpui-shell");

        assert!(base.contains("export type ColorToken"));
        assert!(!gpui.contains("export type ColorToken"));
        for name in [
            "LengthString",
            "PathCoordinate",
            "Props",
            "ElementBounds",
            "ScopePhase",
            "MotionProperty",
            "DialogOptions",
            "ToastOptions",
            "TaskOptions",
        ] {
            assert!(
                shell.contains(&format!("export type {name}"))
                    || shell.contains(&format!("export interface {name}")),
                "{name} is not owned by gpui-shell"
            );
            assert!(
                !gpui.contains(&format!("export type {name}"))
                    && !gpui.contains(&format!("export interface {name}")),
                "{name} leaked into gpui"
            );
        }
        for name in ["View", "Entity", "Context", "AsyncContext", "Task"] {
            assert!(
                gpui.contains(&format!("export abstract class {name}"))
                    || gpui.contains(&format!("export interface {name}")),
                "{name} must stay in gpui"
            );
        }
        assert!(base.contains("export interface PartType"));
        assert!(!gpui.contains("export interface PartType"));
        assert!(base.contains("export interface ComponentType"));
        assert!(!gpui.contains("export interface ComponentType"));
        assert!(base.contains("export type Placement"));
        assert!(!declarations.contains("SheetSide"));
        assert!(gpui.contains("export type Axis"));
        assert!(!declarations.contains("GroupAxis"));
        assert!(gpui.contains("export interface PathBuilder"));
        assert!(!shell.contains("export interface PathBuilder"));
    }

    #[test]
    fn no_internal_name_leaks_into_the_surface() {
        let declarations = base_declarations();
        for internal in ["__id", "__apply", "__state", "__gpui", "__styleNames"] {
            assert!(
                !declarations.contains(internal),
                "`{internal}` is engine plumbing and must not be declared"
            );
        }
        assert!(!declarations.contains("__"));
    }

    #[test]
    fn compatibility_is_manifest_metadata_not_a_script_api() {
        let declarations = base_declarations();
        assert!(!declarations.contains("require_api"));
        assert!(
            declarations.contains(&format!("for gpui-shell {}.", crate::plugin::SHELL_VERSION))
        );
    }

    #[test]
    fn the_output_is_structurally_balanced() {
        let declarations = base_declarations();
        let opened = declarations.matches('{').count();
        let closed = declarations.matches('}').count();
        assert_eq!(opened, closed, "unbalanced braces");

        for method in element_methods(&declarations) {
            assert!(!method.is_empty(), "a method line has no name");
        }
        assert!(declarations.contains("declare module \"gpui-kit\" {"));
        assert!(
            declarations.contains("declare module \"gpui\" {\n  export * from \"gpui-kit\";\n}")
        );
        // The global declaration follows the module blocks, and has to stay
        // outside it: a `declare module` body cannot introduce a global, and
        // this file is only in script mode because it has no top-level import
        // or export of its own.
        assert!(declarations.contains("\ninterface Window extends GpuiShellWindow {}"));
        assert!(declarations.contains("\ndeclare var window: Window & typeof globalThis;"));
        assert!(declarations.ends_with(";\n"));
    }

    /// The reason HostModule registrations became imports: the declarations can now be
    /// generated from the registry instead of hand-written beside the script.
    #[test]
    fn a_registered_host_module_is_declared_from_the_registry() {
        crate::export_module(
            crate::HostModule::new("market")
                .function("quotes", |_| Ok(crate::HostValue::Null))
                .declarations(
                    "
                    export interface Quote {
                      symbol: string;
                    }

                    export function quotes(): Quote[];
                    ",
                ),
        )
        .expect("`market` is not reserved");
        // No TypeScript face, so it gets permissive signatures — which still
        // check the module name and the export name, and still say which
        // exports have to be awaited.
        crate::export_module(
            crate::HostModule::new("audit")
                .function("observe", |_| Ok(crate::HostValue::Null))
                .async_function("drain", |_| Ok(async { Ok(crate::HostValue::Null) }))
                .component("AuditPanel", |_, _, _| gpui::div().into_any_element()),
        )
        .expect("`audit` is not reserved");

        let declarations = base_declarations();
        // The point of the whole change: these names come from the registry,
        // not from a file someone maintains beside the script.
        assert!(
            declarations.contains("declare module \"market\" {"),
            "a registered module must be declared, or its import is untyped"
        );
        assert!(
            declarations.contains("  export function quotes(): Quote[];"),
            "a declared face must be emitted verbatim, not summarised"
        );
        // The indentation the host wrote inside a multi-line type survives; only
        // the raw string's own leading margin is removed.
        assert!(
            declarations.contains("  export interface Quote {\n    symbol: string;\n  }"),
            "a multi-line declaration was flattened:\n{}",
            &declarations[declarations.find("declare module \"market\"").unwrap()..]
        );
        assert!(
            declarations.contains("declare module \"audit\" {"),
            "a module without a face is still declared, or its name goes unchecked"
        );
        // `HostValue` rather than `any`: the boundary is not wider than the
        // Rust type of that name, and the declarations should not claim it is.
        assert!(
            declarations.contains("  import { Element, HostValue } from \"gpui-kit\";"),
            "the permissive signatures below need this import to resolve"
        );
        assert!(
            declarations.contains("  export function observe(...args: HostValue[]): HostValue;")
        );
        assert!(
            declarations
                .contains("  export function drain(...args: HostValue[]): Promise<HostValue>;")
        );
        assert!(declarations.contains(
            "  export const AuditPanel: { new(id: string, props: HostValue): Element };"
        ));

        crate::clear_exported_modules();
        assert!(
            !super::base_declarations().contains("declare module \"market\""),
            "declarations are read from the registry at write time, so a \
             withdrawn module must stop being declared"
        );
    }

    #[test]
    fn each_built_in_module_declares_only_what_its_crate_provides() {
        let declarations = base_declarations();
        let module = |specifier: &str| {
            let start = declarations
                .find(&format!("declare module \"{specifier}\" {{"))
                .unwrap_or_else(|| panic!("missing module {specifier}"));
            let end = declarations[start..]
                .find("\n}\n")
                .expect("unterminated module");
            &declarations[start..start + end]
        };

        // A component is declared where it is implemented. Reading a name out
        // of the wrong module is the failure this guards: `Button` under
        // `"gpui-kit"` would say the runtime draws it, and the whole point of the
        // split is that `gpui-base` does.
        let gpui = module("gpui-kit");
        let base = module("gpui-base");
        for name in [
            "export const Button",
            "export function v_flex",
            "export function set_theme",
        ] {
            assert!(base.contains(name), "`{name}` is not declared in gpui-base");
            assert!(!gpui.contains(name), "`{name}` is also declared in gpui");
        }
        for name in ["export function div", "export interface AsyncContext"] {
            assert!(gpui.contains(name), "`{name}` is not declared in gpui");
            assert!(
                !base.contains(name),
                "`{name}` is also declared in gpui-base"
            );
        }
        assert!(module("gpui-fps").contains("export function fps_monitor(): Element;"));

        // The dependency runs upward only: a layer names what it borrows from
        // `"gpui-kit"`, and `"gpui-kit"` imports nothing back.
        assert!(base.contains("} from \"gpui-kit\";"));
        assert!(!gpui.contains("} from \"gpui-base\";"));
    }

    /// Every name a built-in module exports is declared, in that same module.
    ///
    /// The two lists are written in different places for different reasons —
    /// one wires up the JS module, the other describes it — and a name added to
    /// one and not the other is invisible until an application reaches for it:
    /// either an import that resolves to nothing the editor knows about, or a
    /// declaration promising a binding that is not there.
    #[cfg(feature = "quickjs")]
    #[test]
    fn the_declarations_name_exactly_what_the_runtime_exports() {
        use crate::engine::quickjs::exports;

        let declarations = base_declarations();
        for (specifier, names) in [
            ("gpui-kit", exports::GPUI),
            ("gpui-base", exports::GPUI_BASE),
            ("gpui-shell", exports::GPUI_SHELL),
            ("gpui-fps", exports::GPUI_FPS),
        ] {
            let start = declarations
                .find(&format!("declare module \"{specifier}\" {{"))
                .unwrap_or_else(|| panic!("missing module {specifier}"));
            let end = start
                + declarations[start..]
                    .find("\n}\n")
                    .expect("unterminated module");
            let body = &declarations[start..end];

            for name in names {
                // `View` is a class and the rest are functions, constants or
                // interfaces, so match the name as a declared word rather than
                // guessing which keyword introduces it.
                assert!(
                    body.contains(&format!("export function {name}"))
                        || body.contains(&format!("export const {name}"))
                        || body.contains(&format!("export class {name}"))
                        || body.contains(&format!("export abstract class {name}")),
                    "`{name}` is exported from \"{specifier}\" but declared nowhere in it"
                );
            }

            let declared_values = body
                .lines()
                .filter_map(|line| {
                    let line = line.trim_start();
                    [
                        "export function ",
                        "export const ",
                        "export class ",
                        "export abstract class ",
                    ]
                    .into_iter()
                    .find_map(|prefix| {
                        line.strip_prefix(prefix).map(|rest| {
                            rest.split(['(', ':', ' ', '{'])
                                .next()
                                .expect("an exported value has a name")
                        })
                    })
                })
                .collect::<std::collections::BTreeSet<_>>();
            let runtime_values = names
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(
                declared_values, runtime_values,
                "the values declared by \"{specifier}\" differ from its run-time exports"
            );
        }
    }

    #[test]
    fn standard_runtime_modules_are_declared_without_node_aliases() {
        let declarations = base_declarations();
        for name in [
            "buffer",
            "path",
            "url",
            "crypto",
            "zlib",
            "console",
            "process",
            "os",
            "fs/promises",
            "net",
        ] {
            assert!(
                declarations.contains(&format!("declare module \"{name}\"")),
                "missing Standard Runtime module {name}"
            );
        }
        // The alias, not the four letters: `node` is also an ordinary field
        // name — a dock group carries the id of the layout node it mirrors.
        assert!(!declarations.contains("declare module \"node:"));
        assert!(!declarations.contains("from \"node:"));
        assert!(!declarations.contains("export const fs: FileSystem"));
        assert!(!declarations.contains("export const process: Process"));
        assert!(declarations.contains("declare function fetch"));
    }

    #[test]
    fn standard_names_only_claim_standard_compatible_contracts() {
        let declarations = base_declarations();

        assert!(!declarations.contains("declare module \"fs\""));
        assert!(declarations.contains("readFile(path: string): Promise<Uint8Array>;"));
        assert!(declarations.contains(
            "readFile(path: string, encoding: \"utf8\" | { encoding: \"utf8\" }): Promise<string>;"
        ));
        assert!(
            declarations
                .contains("writeFile(path: string, contents: string | Uint8Array): Promise<void>;")
        );
        assert!(declarations.contains("export interface Dirent"));
        assert!(declarations.contains("isDirectory(): boolean;"));
        assert!(declarations.contains("readdir(path: string): Promise<string[]>;"));
        assert!(declarations.contains(
            "readdir(path: string, options: { withFileTypes: true }): Promise<Dirent[]>;"
        ));

        assert!(declarations.contains("declare module \"websocket\""));
        assert!(declarations.contains("export const WebSocket: WebSocketType;"));
        assert!(!declarations.contains("declare module \"gpui/websocket\""));
        assert!(!declarations.contains("declare const WebSocket:"));
        assert!(declarations.contains("text(): Promise<string>;"));
        assert!(declarations.contains("json(): Promise<unknown>;"));

        for fake_system_member in [
            "export const env:",
            "export function cwd()",
            "export function homedir()",
            "export function tmpdir()",
        ] {
            assert!(
                !declarations.contains(fake_system_member),
                "fake system member remained declared: {fake_system_member}"
            );
        }
        // Outside every `declare module` block, which is what makes it global.
        // `var` rather than `const` so it merges with the ambient `Window`.
        assert!(
            declarations.contains("declare var window:"),
            "`window` must stay a top-level declaration, or it is not a global"
        );
    }

    #[test]
    fn websocket_binary_and_text_messages_are_declared() {
        let declarations = base_declarations();
        assert!(declarations.contains("export interface WebSocketSocket {"));
        assert!(declarations.contains("read(): Promise<string | Uint8Array>;"));
        assert!(declarations.contains("write(data: string | Uint8Array): Promise<void>;"));
        assert!(declarations.contains("close(): Promise<void>;"));
        assert!(declarations.contains("export interface WebSocketConnectOptions {"));
        assert!(declarations.contains("headers?: Readonly<Record<string, string>>;"));
        assert!(declarations.contains(
            "connect(url: string, options?: WebSocketConnectOptions): Promise<WebSocketSocket>;"
        ));
    }

    #[test]
    fn raw_tcp_reads_preserve_bytes_and_expose_eof() {
        let declarations = base_declarations();
        assert!(declarations.contains("read(maxBytes?: number): Promise<Uint8Array | null>;"));
    }

    #[test]
    fn every_element_method_is_accounted_for() {
        let declared = element_methods(&base_declarations());
        let styles: Vec<&String> = declared
            .iter()
            .filter(|name| !NON_STYLE_METHODS.contains(&name.as_str()))
            .collect();

        assert_eq!(
            styles.len(),
            style::known_names().len(),
            "the declared style methods and the runtime's style table have diverged"
        );
        assert_eq!(
            declared.len(),
            styles.len() + NON_STYLE_METHODS.len(),
            "an element method is declared that this test does not know about"
        );

        let mut sorted = styles.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            styles.len(),
            "a style method is declared twice"
        );
    }

    /// The focus and accessibility surface is declared, and the role union is
    /// generated from the table the runtime parses through rather than typed
    /// out beside it.
    #[test]
    fn focus_and_accessibility_are_declared_from_the_runtime_tables() {
        let declarations = base_declarations();
        for expected in [
            "    role(name: Role): Element;",
            "    aria_selected(value: boolean): Element;",
            "    aria_active_descendant(): Element;",
            "    track_focus(handle: FocusHandle): Element;",
            "    tab_index(index: number): Element;",
            "    tab_stop(value: boolean): Element;",
            // `App::focus_handle` in GPUI, so `cx` here — there is no
            // `FocusHandle::new` to mirror.
            "    focus_handle(): FocusHandle;",
        ] {
            assert!(declarations.contains(expected), "missing: {expected}");
        }

        assert!(!declarations.contains("FocusHandleHandle"));
        assert!(!declarations.contains("VirtualListScrollHandleHandle"));

        for name in a11y::role_names() {
            assert!(
                declarations.contains(&format!("    | \"{name}\"\n")),
                "the role union is missing `{name}`"
            );
        }
        assert!(
            !declarations.contains(&format!("| \"{}\"", a11y::FILTERED_ROLE)),
            "a role GPUI filters out of the accessibility tree must not be offerable"
        );
    }

    #[test]
    fn render_accepts_every_runtime_renderable_shape() {
        let declarations = base_declarations();
        assert!(declarations.contains("abstract render(cx: Context): Element | Entity | string;"));
    }

    #[test]
    fn text_view_behaviors_are_not_declared_on_every_element() {
        let declarations = declarations();
        let element = declarations
            .split_once("export interface Element")
            .expect("Element declaration")
            .1
            .split_once("\n  }")
            .expect("end of Element declaration")
            .0;
        assert!(!element.contains("on_link_click("));
        assert!(!element.contains("selectable("));
        assert!(!element.contains("scrollable("));
        assert!(declarations.contains("export interface TextViewElement extends Element"));
        assert!(declarations.contains("html(id: string, html: string): TextViewElement;"));
    }

    #[test]
    fn view_lifecycle_declaration_matches_runtime_calls() {
        let declarations = base_declarations();
        assert!(declarations.contains(
            "init?(props: import(\"gpui-shell\").Props | undefined, cx: AsyncContext): void;"
        ));
        assert!(
            declarations
                .contains("update?(props: import(\"gpui-shell\").Props | undefined): void;")
        );
        assert!(!declarations.contains("cx?: AsyncContext"));
    }

    #[test]
    fn retained_state_event_names_and_payloads_match_the_runtime() {
        let declarations = base_declarations();
        let union = |names: &[&str]| {
            names
                .iter()
                .map(|name| format!("\"{name}\""))
                .collect::<Vec<_>>()
                .join(" | ")
        };
        assert!(declarations.contains(&format!(
            "on(event: {}, handler: (event: InputEvent, cx: Context)",
            union(crate::entities::InputEventName::NAMES)
        )));
        assert!(declarations.contains(&format!(
            "on(event: {}, handler: (event: OtpEvent, cx: Context)",
            union(crate::entities::OtpEventName::NAMES)
        )));
        assert!(declarations.contains("export interface InputEvent"));
        assert!(declarations.contains("export interface OtpEvent"));
        assert!(!declarations.contains("handler: (event: any, cx: Context)"));
    }

    #[test]
    fn component_constructor_shapes_name_only_reusable_public_concepts() {
        let declarations = base_declarations();
        let base = declarations
            .split_once("declare module \"gpui-base\" {")
            .expect("gpui-base declarations")
            .1
            .split_once("\n}\n")
            .expect("end of gpui-base declarations")
            .0;
        assert!(base.contains("export interface ComponentType"));
        assert!(base.contains("export interface PartType"));
        assert!(!declarations.contains("IndexedComponentType"));
        assert!(declarations.contains(
            "export const TableRow: { new: (id: string | number, row_index: number) => Element };"
        ));
        assert!(declarations.contains(
            "export const TableCell: { new: (id: string | number, column_index: number) => Element };"
        ));
    }

    #[test]
    fn public_types_name_script_concepts_not_declaration_scaffolding() {
        let declarations = base_declarations();

        for name in [
            "PathBuilderHandle",
            "BackgroundValue",
            "VirtualListScrollHandleType",
            "InputStateHandle",
            "InputStateType",
            "InputType",
            "NumberInputType",
            "TextareaStateHandle",
            "TextareaStateType",
            "TextareaType",
            "SliderStateHandle",
            "SliderStateType",
            "SliderPartType",
            "OtpStateHandle",
            "OtpStateType",
            "OtpInputType",
            "PopupType",
            "DatePickerType",
            "ScrollbarType",
        ] {
            assert!(
                !declarations.contains(&format!("export interface {name}")),
                "declaration-only carrier `{name}` leaked into the public interface"
            );
        }

        for name in [
            "PathBuilder",
            "Background",
            "VirtualListScrollHandle",
            "InputState",
            "TextareaState",
            "SliderState",
            "OtpState",
        ] {
            assert!(
                declarations.contains(&format!("export interface {name}")),
                "the instance type `{name}` is not directly nameable"
            );
        }
    }

    #[test]
    fn retained_nested_views_are_declared() {
        let declarations = base_declarations();
        for expected in [
            "  export interface Entity {",
            "    set_props(props?: import(\"gpui-shell\").Props): void;",
            "    release(): boolean;",
        ] {
            assert!(declarations.contains(expected), "missing: {expected}");
        }
    }

    #[test]
    fn targeted_notify_is_declared() {
        assert!(
            base_declarations().contains("    notify(target?: Entity): void;"),
            "Context.notify must expose GPUI's targeted entity notification"
        );
    }

    #[test]
    fn nested_update_rollback_contract_names_its_supported_boundary() {
        let declarations = base_declarations();
        for expected in [
            "post-update descriptors remain legally redefinable or deletable",
            "including callable objects",
            "shell-owned entities, tasks and nested views newly created by the update",
            "JavaScript private fields and internal slots",
            "making an existing configurable property non-configurable",
            "pre-existing native handles explicitly released by update",
        ] {
            assert!(declarations.contains(expected), "missing: {expected}");
        }
        assert!(
            !declarations.contains("a failure rolls back")
                && !declarations
                    .contains("ordinary reachable configurable object properties are restored"),
            "the public contract must not promise unconditional rollback"
        );
    }

    /// The anchor union is generated from the table the runtime parses through,
    /// so a corner an editor accepts is one `anchor(...)` accepts.
    #[test]
    fn the_anchored_surfaces_are_declared_from_the_runtime_anchor_table() {
        let declarations = base_declarations();
        for name in crate::materialize::ANCHOR_NAMES {
            assert!(
                declarations.contains(&format!("    | \"{name}\"\n")),
                "the anchor union is missing `{name}`"
            );
        }
        for expected in [
            "    anchor(value: Anchor): Element;",
            "    mouse_button(value: MouseButton): Element;",
            "    trigger(element: Element): Element;",
            "    open_delay(ms: number): Element;",
            "    close_delay(ms: number): Element;",
            "  export const Popover: ComponentType;",
            "  export const HoverCard: ComponentType;",
        ] {
            assert!(declarations.contains(expected), "missing: {expected}");
        }
    }

    #[test]
    fn motion_policies_are_declared_without_per_frame_callbacks() {
        let declarations = base_declarations();
        assert!(declarations.contains(
            "transition(property: import(\"gpui-shell\").MotionProperty, policy: number | import(\"gpui-shell\").TransitionPolicy): Element;"
        ));
        assert!(
            declarations.contains(
                "spring(property: import(\"gpui-shell\").MotionProperty, policy?: import(\"gpui-shell\").SpringPolicy): Element;"
            )
        );
        assert!(declarations.contains(
            "type MotionProperty = \"opacity\" | \"width\" | \"height\" | \"left\" | \"top\";"
        ));
        assert!(!declarations.contains("on_animation_frame"));
    }

    #[test]
    fn no_style_method_collides_with_an_element_method() {
        // A collision would emit the same member twice and make the whole file
        // invalid TypeScript, so it has to fail here rather than in an editor.
        for name in style::known_names() {
            assert!(
                !NON_STYLE_METHODS.contains(&name),
                "`{name}` is both a style method and an element method"
            );
        }
    }

    #[test]
    fn every_style_name_is_a_valid_identifier() {
        for name in style::known_names() {
            let mut chars = name.chars();
            assert!(
                chars
                    .next()
                    .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
                    && chars.all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "`{name}` cannot be written as a TypeScript member name"
            );
        }
    }

    /// Refreshing writes once and then leaves the file alone.
    ///
    /// The second half is what makes this safe to run on every launch: an editor
    /// watching the directory is not woken, and a checkout whose files are
    /// read-only is not an error nobody can act on.
    #[test]
    fn refresh_writes_once_and_then_says_nothing() {
        let directory =
            std::env::temp_dir().join(format!("gpui-shell-refresh-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("a temporary directory");

        let written =
            refresh_with_components(&directory, &crate::FrozenComponentRegistry::default())
                .expect("the first refresh");
        assert_eq!(
            written.as_deref(),
            Some(directory.join(FILE_NAME).as_path())
        );
        assert_eq!(
            std::fs::read_to_string(directory.join(FILE_NAME)).expect("the file"),
            base_declarations()
        );

        assert_eq!(
            refresh_with_components(&directory, &crate::FrozenComponentRegistry::default())
                .expect("the second refresh"),
            None,
            "an up-to-date file must not be rewritten"
        );

        // A stale one is replaced, which is the case this exists for.
        std::fs::write(directory.join(FILE_NAME), "// from an older runtime\n")
            .expect("overwriting");
        assert!(
            refresh_with_components(&directory, &crate::FrozenComponentRegistry::default())
                .expect("the third refresh")
                .is_some()
        );

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn write_application_creates_the_file_beside_an_application() {
        let directory =
            std::env::temp_dir().join(format!("gpui-shell-typings-{}", std::process::id()));
        let written = write_application_with_components(
            &directory,
            &crate::FrozenComponentRegistry::default(),
        )
        .expect("declarations are writable");
        let path = directory.join(FILE_NAME);

        assert_eq!(
            written,
            vec![directory.join(CONFIG_FILE_NAME), path.clone()]
        );
        assert_eq!(path.file_name().unwrap(), FILE_NAME);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), base_declarations());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn an_existing_editor_configuration_is_never_replaced() {
        let directory = std::env::temp_dir().join(format!(
            "gpui-shell-typings-config-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("application root");
        std::fs::write(directory.join(TYPESCRIPT_CONFIG_FILE_NAME), "{}").expect("a tsconfig");

        write_application_with_components(&directory, &crate::FrozenComponentRegistry::default())
            .expect("declarations are writable");

        assert!(!directory.join(CONFIG_FILE_NAME).exists());
        assert_eq!(
            std::fs::read_to_string(directory.join(TYPESCRIPT_CONFIG_FILE_NAME)).unwrap(),
            "{}"
        );

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[cfg(unix)]
    #[test]
    fn write_application_does_not_follow_directory_symlinks_outside_the_root() {
        use std::os::unix::fs::symlink;

        let unique = format!("{}", std::process::id());
        let root = std::env::temp_dir().join(format!("gpui-shell-typings-root-{unique}"));
        let outside = std::env::temp_dir().join(format!("gpui-shell-typings-outside-{unique}"));
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside);
        std::fs::create_dir_all(&root).expect("application root");
        std::fs::create_dir_all(&outside).expect("outside directory");
        std::fs::write(
            outside.join("escape.js"),
            "import { View } from 'gpui-kit';",
        )
        .expect("outside script");
        symlink(&outside, root.join("escape")).expect("directory symlink");

        write_application_with_components(&root, &crate::FrozenComponentRegistry::default())
            .expect("root declarations");

        assert!(root.join(FILE_NAME).is_file());
        assert!(
            !outside.join(FILE_NAME).exists(),
            "the declaration writer must stay inside the application root"
        );

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[cfg(unix)]
    #[test]
    fn refresh_does_not_follow_a_declaration_file_symlink() {
        use std::os::unix::fs::symlink;

        let unique = format!("{}", std::process::id());
        let root = std::env::temp_dir().join(format!("gpui-shell-typings-link-root-{unique}"));
        let outside =
            std::env::temp_dir().join(format!("gpui-shell-typings-link-target-{unique}.txt"));
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(&outside);
        std::fs::create_dir_all(&root).expect("application root");
        std::fs::write(&outside, "do not replace").expect("outside target");
        symlink(&outside, root.join(FILE_NAME)).expect("declaration symlink");

        let error = refresh_with_components(&root, &crate::FrozenComponentRegistry::default())
            .expect_err("a declaration symlink must be refused");
        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        assert_eq!(
            std::fs::read_to_string(&outside).expect("outside target"),
            "do not replace"
        );

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(&outside);
    }
}

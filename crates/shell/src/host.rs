//! `gpui-shell` — runs a script application directory.
//!
//! ```text
//! gpui-shell run|dev [<name> | --dir <directory>] [--help] [--version]
//! ```
//!
//! The binary is the thin host the library documents: it parses a command line,
//! installs a log sink, builds one runtime, opens one window, and — when asked —
//! drives the source watcher from a GPUI timer. Every decision that outlives a
//! single invocation lives in the library instead.

use std::{
    cell::RefCell,
    fmt::{self, Write as _},
    path::{Path, PathBuf},
    rc::Rc,
};

use crate::{AppAssets, Capabilities, ShellRoot, ShellRuntime};
use gpui::{
    AnyWindowHandle, App, AppContext as _, Bounds, Context, Entity, IntoElement, Render,
    TitlebarOptions, Window, WindowBounds, WindowOptions, px, size,
};
use tracing::{
    Event, Level, Metadata, Subscriber,
    field::{Field, Visit},
    level_filters::LevelFilter,
    span,
};

/// The entry file an application directory must contain. Duplicated from the
/// engine because the host resolves the application root itself: it needs the
/// resolved directory for the window title and for the watcher, both of which
/// are decided before any script is loaded.
const ENTRY: &str = "main.js";

/// The exit code for a command line this binary could not act on. Distinct from
/// 1, which is a runtime that failed to start.
const EXIT_USAGE: i32 = 2;

mod agents;
mod bundle;
mod installed;
mod scaffold;

/// Runs the shipped command-line host with one frozen component catalog.
///
/// Embedding binaries use this entry point so command parsing, capabilities,
/// assets, theming, checking, and diagnostics stay identical to `gpui-shell`.
pub fn main_with_components(components: crate::FrozenComponentRegistry) {
    main_with_brand_and_components(HostBrand::gpui_shell(), components);
}

/// Runs the shared host under an embedding command's public name and version.
pub fn main_with_brand_and_components(
    brand: HostBrand,
    components: crate::FrozenComponentRegistry,
) {
    let invocation = match parse(std::env::args().skip(1), brand) {
        Ok(invocation) => invocation,
        Err(message) => usage_error(brand, &message),
    };

    let arguments = match invocation {
        Invocation::Run(arguments) => arguments,
        Invocation::Bare(arguments) => {
            if installed::bundled().is_none() {
                println!("{}", help(brand));
                return;
            }
            arguments
        }
        Invocation::Types(application) => {
            let directory = resolve_or_exit(&application, brand);
            // The root always, whether or not anything there imports the module:
            // this command was asked for explicitly, and an empty directory is a
            // reasonable place to start an application. Every other directory
            // that imports `gpui-kit` comes along with it.
            match crate::write_type_declarations_with_components(&directory, &components) {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("{}: {error}", brand.name());
                    std::process::exit(1);
                }
            }
            return;
        }
        Invocation::List => installed::list(brand),
        Invocation::Agents(application) => {
            let Some(application) = application else {
                print!("{}", agents::render(brand, &components));
                return;
            };
            let directory = resolve_or_exit(&application, brand);
            match agents::write(&directory, brand, &components) {
                Ok(Some(path)) => println!("wrote {}", path.display()),
                Ok(None) => println!(
                    "{} is up to date",
                    directory.join(agents::AGENTS_FILE).display()
                ),
                Err(error) => {
                    eprintln!("{}: {error}", brand.name());
                    std::process::exit(1);
                }
            }
            return;
        }
        Invocation::New(directory) => match scaffold::create(&directory, brand, &components) {
            Ok(created) => {
                let shown = directory.display();
                println!("created {}", created.display());
                println!();
                println!(
                    "  {} dev -d {shown}     # run it, reloading on save",
                    brand.name()
                );
                println!(
                    "  {} check -d {shown}   # check it after every change",
                    brand.name()
                );
                println!();
                println!(
                    "To install it, copy or link it into {}.",
                    installed::applications_directory(brand).display()
                );
                return;
            }
            Err(error) => {
                eprintln!("{}: {error}", brand.name());
                std::process::exit(1);
            }
        },
        Invocation::Check(arguments) => {
            let directory = resolve_or_exit(&arguments.application, brand);
            // A check reports diagnostics, not progress, so only warnings and
            // errors reach the terminal.
            install_log_sink(Level::WARN);
            check(directory, arguments, components);
        }
        Invocation::Bundle(arguments) => {
            let directory = resolve_or_exit(&arguments.application, brand);
            match bundle::create(&directory, &arguments.options, brand) {
                Ok(report) => println!("{report}"),
                Err(error) => {
                    eprintln!("{}: {error:#}", brand.name());
                    std::process::exit(1);
                }
            }
            return;
        }
        Invocation::Print(text) => {
            println!("{text}");
            return;
        }
    };

    let directory = resolve_or_exit(&arguments.application, brand);
    if arguments.is_packaged() {
        let root = crate::resolve_app_root(&directory, ENTRY).unwrap_or_else(|_| directory.clone());
        // Without a manifest there is no `id` to keep the data under, and the
        // data would follow the path instead — lost when the bundle moves.
        if !root.join(crate::plugin::MANIFEST_FILE).is_file() {
            eprintln!(
                "{}: {} has no {}; a packaged application needs one for its `id`",
                brand.name(),
                root.display(),
                crate::plugin::MANIFEST_FILE
            );
            std::process::exit(1);
        }
    }

    // Installed before anything else: the runtime reports script errors,
    // unhandled promise rejections and illegal-phase calls through `tracing`,
    // and until a subscriber exists every one of them is discarded silently.
    install_log_sink(if arguments.is_verbose() {
        Level::DEBUG
    } else {
        Level::INFO
    });

    run(directory, arguments, components, brand);
}

fn usage_error(brand: HostBrand, message: &str) -> ! {
    eprintln!("{}: {message}", brand.name());
    eprintln!("Try `{} --help` for the accepted arguments.", brand.name());
    std::process::exit(EXIT_USAGE);
}

fn resolve_or_exit(application: &Application, brand: HostBrand) -> PathBuf {
    match application.resolve(brand) {
        Ok(directory) => directory,
        Err(message) => {
            eprintln!("{}: {message}", brand.name());
            std::process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Command line
// ---------------------------------------------------------------------------

/// What a successfully parsed command line asks the host to do.
///
/// `--help` and `--version` answer without starting a runtime, so they are a
/// separate outcome rather than a flag on [`Arguments`]: nothing downstream
/// should have to remember to check them.
#[derive(Debug, PartialEq, Eq)]
enum Invocation {
    Run(Arguments),
    /// No command at all: the packaged application when there is one, and
    /// the help otherwise. A bundle's executable is started with no
    /// arguments, and so is a freshly installed binary typed at a prompt.
    Bare(Arguments),
    /// Print the installed applications, executing none.
    List,
    /// Load and render once, report, and exit with a status.
    Check(CheckArguments),
    /// Write TypeScript declarations next to an application.
    Types(Application),
    /// Print the development instructions, or write them into an application.
    Agents(Option<Application>),
    /// Create a blank application in a new directory.
    New(PathBuf),
    /// Package an application with the runtime as a desktop application.
    Bundle(BundleArguments),
    /// Text for stdout, followed by a successful exit.
    Print(String),
}

/// Which application a command line names.
///
/// Kept unresolved by the parser so parsing stays free of the environment: the
/// installed-applications directory depends on the home directory, and the
/// bundled application on where the executable is.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Application {
    /// A directory name under the installed-applications directory:
    /// `gpui-shell run clock` runs `~/.gpui-shell/apps/clock`.
    Installed(String),
    /// A directory, or the entry file inside it, given as a path.
    Path(PathBuf),
    /// Nothing named: the application packaged beside the executable.
    Bundled,
}

impl Application {
    /// A bare argument is an installed application's name; anything that reads
    /// as a path is a path. `run clock` and `run ./clock` are
    /// therefore different applications, and `--dir` removes the doubt.
    fn from_argument(argument: &str) -> Self {
        if installed::is_application_name(argument) {
            Self::Installed(argument.to_owned())
        } else {
            Self::Path(PathBuf::from(argument))
        }
    }

    /// The directory the command runs in, absolute so a report names it.
    fn current_directory() -> Self {
        Self::Path(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }

    /// How the command line names it, for a message suggesting the command.
    fn describe(&self) -> String {
        match self {
            Self::Installed(name) => name.clone(),
            Self::Path(path) => format!("-d {}", path.display()),
            Self::Bundled => String::new(),
        }
    }

    fn resolve(&self, brand: HostBrand) -> Result<PathBuf, String> {
        match self {
            Self::Installed(name) => installed::resolve(name, brand),
            Self::Path(path) => Ok(path.clone()),
            Self::Bundled => installed::bundled().ok_or_else(|| {
                format!(
                    "nothing to run: no application is packaged beside this executable. \
                     Run one with `{name} run <name>` or `{name} run -d <directory>`; \
                     `{name} list` shows what is installed in {installed}.",
                    name = brand.name(),
                    installed = installed::applications_directory(brand).display()
                )
            }),
        }
    }
}

/// The command name and version shown by the reusable CLI host.
///
/// Private fields for the same reason the component descriptors have them: an
/// embedding binary constructs one across the crate boundary, so a later field
/// should be an added builder method rather than a break.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostBrand {
    name: &'static str,
    version: &'static str,
}

impl HostBrand {
    pub const fn new(name: &'static str, version: &'static str) -> Self {
        Self { name, version }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn version(&self) -> &'static str {
        self.version
    }

    const fn gpui_shell() -> Self {
        Self::new("gpui-shell", env!("CARGO_PKG_VERSION"))
    }
}

/// Stable command classification exposed for wrapper-binary parser tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvocationKind {
    Run,
    /// Lists the installed applications without running them.
    List,
    Check,
    Types,
    /// Prints or writes the development instructions for coding agents.
    Agents,
    /// Creates a blank application.
    New,
    /// Packages an application with the runtime as a desktop application.
    Bundle,
    Print,
}

/// Parses a command line without starting GPUI or terminating the process.
pub fn parse_invocation(
    arguments: impl IntoIterator<Item = impl Into<String>>,
) -> Result<InvocationKind, String> {
    parse(
        arguments.into_iter().map(Into::into),
        HostBrand::gpui_shell(),
    )
    .map(|invocation| match invocation {
        Invocation::Run(_) | Invocation::Bare(_) => InvocationKind::Run,
        Invocation::List => InvocationKind::List,
        Invocation::Check(_) => InvocationKind::Check,
        Invocation::Types(_) => InvocationKind::Types,
        Invocation::Agents(_) => InvocationKind::Agents,
        Invocation::New(_) => InvocationKind::New,
        Invocation::Bundle(_) => InvocationKind::Bundle,
        Invocation::Print(_) => InvocationKind::Print,
    })
}

/// The command line, parsed.
///
/// A value rather than a set of locals so the parsing can be tested without
/// opening a window, which is the only part of this binary that has to run on a
/// desktop.
#[derive(Debug, PartialEq, Eq)]
struct Arguments {
    application: Application,
    watch: bool,
    verbose: bool,
    development: bool,
    packaged: bool,
}

impl Arguments {
    /// Whether sources are polled for changes: `run --watch`, or `dev`.
    fn is_watching(&self) -> bool {
        self.watch
    }

    /// Whether the log goes down to debug level: under `dev` only.
    fn is_verbose(&self) -> bool {
        self.verbose
    }

    /// Whether the sandbox relaxations are on: `--dev`, and nothing else.
    fn is_development(&self) -> bool {
        self.development
    }

    /// Whether the application is a packaged one rather than a directory
    /// being edited: the one beside the executable, or one a bundle's
    /// launcher runs with `--packaged`. It must have a manifest, is titled by
    /// its `name`, and has no editor files written into it.
    fn is_packaged(&self) -> bool {
        self.packaged || self.application == Application::Bundled
    }
}

/// The subcommands, which do something other than run an application.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Command {
    Run,
    /// `run` for an application being edited: reload on change, sandbox relaxed.
    Dev,
    New,
    Check,
    Types,
    List,
    Agents,
    Bundle,
}

/// Parses the arguments after the program name.
///
/// Hand-rolled because the whole surface is a few flags and one application:
/// a parser dependency would be larger than the thing it parses. The error is
/// the exact sentence printed to stderr, so the caller decides nothing about
/// wording.
fn parse(
    arguments: impl IntoIterator<Item = String>,
    brand: HostBrand,
) -> Result<Invocation, String> {
    let arguments: Vec<String> = arguments.into_iter().collect();

    // Answered before anything else can fail. A caller who mistyped one flag is
    // exactly the caller who needs `--help` to still work.
    if arguments.iter().any(|it| it == "--help" || it == "-h") {
        return Ok(Invocation::Print(help(brand)));
    }
    if arguments.iter().any(|it| it == "--version" || it == "-V") {
        return Ok(Invocation::Print(version(brand)));
    }

    let mut application: Option<Application> = None;
    let mut command: Option<Command> = None;
    let mut watch = false;
    let mut development = false;
    let mut print_spec = false;
    let mut packaged = false;
    let mut bundle_options = bundle::BundleOptions::default();
    // The first `bundle`-only flag, reported if the command is not `bundle`.
    let mut bundle_flag: Option<&'static str> = None;

    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        let named = |application: &Option<Application>, next: Application| {
            if application.is_some() {
                return Err(format!(
                    "unexpected argument `{}`; {} runs one application",
                    match &next {
                        Application::Installed(name) => name.clone(),
                        Application::Path(path) => path.display().to_string(),
                        Application::Bundled => String::new(),
                    },
                    brand.name()
                ));
            }
            Ok(Some(next))
        };

        match argument.as_str() {
            // A subcommand rather than a flag, because it does something else
            // entirely: it never shows a window and it exits with a status.
            "new" if application.is_none() && command.is_none() => {
                command = Some(Command::New);
            }
            "run" if application.is_none() && command.is_none() => {
                command = Some(Command::Run);
            }
            "dev" if application.is_none() && command.is_none() => {
                command = Some(Command::Dev);
            }
            "check" if application.is_none() && command.is_none() => {
                command = Some(Command::Check);
            }
            "types" if application.is_none() && command.is_none() => {
                command = Some(Command::Types);
            }
            "list" if application.is_none() && command.is_none() => {
                command = Some(Command::List);
            }
            "agents" if application.is_none() && command.is_none() => {
                command = Some(Command::Agents);
            }
            "bundle" if application.is_none() && command.is_none() => {
                command = Some(Command::Bundle);
            }
            "--platform" => {
                let value = flag_value(&mut arguments, "--platform", "macos, linux or windows")?;
                bundle_options.platform =
                    Some(bundle::Platform::parse(&value).ok_or_else(|| {
                        format!(
                            "unknown platform `{value}`; `--platform` is macos, linux or windows"
                        )
                    })?);
                bundle_flag.get_or_insert("--platform");
            }
            "--icon" => {
                let value = flag_value(&mut arguments, "--icon", "a PNG image")?;
                bundle_options.icon = Some(value.into());
                bundle_flag.get_or_insert("--icon");
            }
            "--download-url" => {
                let value = flag_value(&mut arguments, "--download-url", "a URL")?;
                if !value.starts_with("https://") && !value.starts_with("http://") {
                    return Err(format!(
                        "`--download-url` expects an http(s) URL, not `{value}`"
                    ));
                }
                bundle_options.download_url = Some(value);
                bundle_flag.get_or_insert("--download-url");
            }
            "--out" => {
                let value = flag_value(&mut arguments, "--out", "a directory")?;
                bundle_options.out = Some(value.into());
                bundle_flag.get_or_insert("--out");
            }
            "--dir" | "-d" => match arguments.next() {
                Some(path) if !path.starts_with('-') => {
                    application = named(&application, Application::Path(path.into()))?;
                }
                _ => return Err("`--dir` expects an application directory".to_owned()),
            },
            "--watch" => watch = true,
            "--packaged" => packaged = true,
            "--dev" => development = true,
            "--print-spec" => print_spec = true,
            // A path is never mistaken for a flag, and an unknown flag is never
            // mistaken for a path: silently treating a mistyped `--watch` as a directory
            // would report a missing entry file instead of the typo.
            other if other.starts_with('-') => {
                return Err(format!("unknown flag `{other}`"));
            }
            other => application = named(&application, Application::from_argument(other))?,
        }
    }

    if let Some(flag) = bundle_flag
        && command != Some(Command::Bundle)
    {
        return Err(format!("`{flag}` is only for `bundle`"));
    }
    if (watch || development) && !matches!(command, Some(Command::Run | Command::Dev) | None) {
        return Err(format!(
            "`{}` is only for `run` and `dev`",
            if watch { "--watch" } else { "--dev" }
        ));
    }
    if packaged && command != Some(Command::Run) {
        return Err("`--packaged` is only for `run`".to_owned());
    }

    match command {
        Some(Command::Bundle) => Ok(Invocation::Bundle(BundleArguments {
            application: application.unwrap_or_else(Application::current_directory),
            options: bundle_options,
        })),
        Some(Command::List) => match application {
            None => Ok(Invocation::List),
            Some(_) => Err("`list` takes no application".to_owned()),
        },
        Some(Command::Agents) => Ok(Invocation::Agents(application)),
        // Created where the command runs, the way `cargo new` does: a bare
        // name is a new directory here, not an installed application.
        Some(Command::New) => match application {
            Some(Application::Installed(name)) => Ok(Invocation::New(PathBuf::from(name))),
            Some(Application::Path(path)) => Ok(Invocation::New(path)),
            _ => Err("`new` expects the directory to create".to_owned()),
        },
        // With nothing named, `run`, `dev`, `check` and `types` take the directory the
        // command runs in, the way `cargo run` takes the package it is in.
        Some(Command::Types) => Ok(Invocation::Types(
            application.unwrap_or_else(Application::current_directory),
        )),
        Some(Command::Check) => Ok(Invocation::Check(CheckArguments {
            application: application.unwrap_or_else(Application::current_directory),
            print_spec,
        })),
        Some(Command::Run) => Ok(Invocation::Run(Arguments {
            application: application.unwrap_or_else(Application::current_directory),
            watch,
            verbose: false,
            development,
            packaged,
        })),
        // How an application being edited is run: reloading, with the debug
        // log. The sandbox stays as shipped unless `--dev` asks otherwise.
        Some(Command::Dev) => Ok(Invocation::Run(Arguments {
            application: application.unwrap_or_else(Application::current_directory),
            watch: true,
            verbose: true,
            development,
            packaged: false,
        })),
        // A bare `gpui-shell` runs the application packaged beside the
        // executable: a bundle's executable is started with no arguments at
        // all, so that is the one place a command cannot be required.
        None => match application {
            None => Ok(Invocation::Bare(Arguments {
                application: Application::Bundled,
                watch,
                verbose: false,
                development,
                packaged,
            })),
            Some(application) => Err(format!(
                "an application runs with `{name} run {}`",
                application.describe(),
                name = brand.name()
            )),
        },
    }
}

/// The value after a flag that takes one. A following flag is never taken
/// as the value: `--out --icon x` is a missing directory, not one named
/// `--icon`.
fn flag_value(
    arguments: &mut impl Iterator<Item = String>,
    flag: &str,
    expected: &str,
) -> Result<String, String> {
    match arguments.next() {
        Some(value) if !value.starts_with('-') => Ok(value),
        _ => Err(format!("`{flag}` expects {expected}")),
    }
}

/// What `check` was asked to do.
#[derive(Debug, PartialEq, Eq)]
struct CheckArguments {
    application: Application,
    print_spec: bool,
}

/// What `bundle` was asked to do.
#[derive(Debug, PartialEq, Eq)]
struct BundleArguments {
    application: Application,
    options: bundle::BundleOptions,
}

/// One line per flag, no decoration — the tone the repository's documentation
/// uses everywhere else.
fn help(brand: HostBrand) -> String {
    format!(
        "\
{version}

Usage: {name} dev [<name> | --dir <directory>] [--dev]
       {name} run [<name> | --dir <directory>] [--watch] [--dev] [--packaged]
       {name} new <directory>
       {name} list
       {name} check [<name> | --dir <directory>] [--print-spec]
       {name} types [<name> | --dir <directory>]
       {name} agents [<name> | --dir <directory>]
       {name} bundle [<name> | --dir <directory>] [--platform <platform>]
              [--icon <png>] [--out <directory>] [--download-url <url>]

Arguments:
  <name>       An installed application: the directory of that name in
               {installed}. An argument containing a path
               separator is a path instead, as if given with --dir.

`run`, `dev`, `check`, `types` and `bundle` with no application use the
current directory. With no arguments at all, {name} runs the application packaged beside its
executable ({bundled}), and prints this help when there is none.

Commands:
  dev          Run an application while you edit it: reload it when its
               sources change, and log at debug level.
  run          Run one application in a window.
  new          Create a blank application in a new directory, relative to
               where the command runs: manifest, entry module, editor types
               and {AGENTS}.
  list         Print the installed applications — id, name, version and
               requested capabilities — without running any.
  agents       Print the development instructions a coding agent needs to write
               an application for this runtime, or write them into the
               application's {AGENTS} (only ever replacing one it generated).
  types        Write gpui-kit.d.ts next to the application, so an editor — or a
               model writing the code — sees the whole API and catches a
               mistyped style method before it runs.
  check        Load the application, build its description, and materialize its
               eager native elements in a hidden window. Exit 0 on success or
               1 on load, render, or registered-component materialization errors.
               Does not check layout, paint, deferred slots, nested view renders,
               or later interaction and asynchronous states.
  bundle       Package the application as a desktop application. Needs
               {MANIFEST}. macOS: an unsigned .app. Linux: a .tar.gz, and an
               install.sh beside it that installs it. Both hold no runtime:
               they use an installed {name} {version_number} or later, or
               download that release. Windows: a portable .zip whose .exe is
               {name} {version_number} itself.

Options:
  -d, --dir <directory>
               The application root, or the {ENTRY} inside it.
  --packaged   With run, run the directory as a packaged application, the way a
               bundle's launcher does: it must have a manifest, is titled by
               its name, and no editor files are written into it.
  --watch      With run, reload the application when its sources change.
               `dev` always does.
  --dev        With run or dev, development mode: relax the sandbox, opening
               eval and the built-in prototypes. Capabilities are unchanged.
  --print-spec With check, also print the element description that was built.
  --platform <platform>
               With bundle, the platform: macos, linux or windows. Defaults to
               the one this runs on.
  --icon <png> With bundle, a square PNG icon, ideally 1024×1024.
  --out <directory>
               With bundle, where the bundle is written. Defaults to ./dist.
  --download-url <url>
               With bundle for Linux, where the archive will be published, so
               install.sh run on its own downloads it from there.
  --help       Print this message and exit.
  --version    Print the version and exit.",
        installed = installed::applications_directory(brand).display(),
        bundled = installed::BUNDLED_LOCATIONS,
        AGENTS = agents::AGENTS_FILE,
        MANIFEST = crate::plugin::MANIFEST_FILE,
        version_number = brand.version(),
        name = brand.name(),
        version = version(brand)
    )
}

fn version(brand: HostBrand) -> String {
    format!("{} {}", brand.name(), brand.version())
}

// ---------------------------------------------------------------------------
// The host
// ---------------------------------------------------------------------------

fn run(
    directory: PathBuf,
    arguments: Arguments,
    components: crate::FrozenComponentRegistry,
    brand: HostBrand,
) {
    // Assets are served from the application directory, so the source has to be
    // installed on the `Application` before the loop starts. Resolving the root
    // here rather than inside the loop is what makes that possible; a path that
    // does not resolve falls back to the argument and fails later with the
    // message that explains why.
    let asset_root =
        crate::resolve_app_root(&directory, ENTRY).unwrap_or_else(|_| directory.clone());

    gpui_platform::application()
        .with_assets(AppAssets::new(asset_root))
        .run(move |cx| {
            crate::init_with_components(cx, &components);
            install_palette(cx);
            install_quit(cx);

            if arguments.is_development() {
                // Before `ShellRuntime::new`: the policy is read when the
                // context is created.
                crate::set_development_mode(true);
                tracing::debug!("development mode: eval and the built-in prototypes are open");
            }

            // Read before the catalog is handed to the runtime, which takes it.
            let window_opener = components.window_opener();

            let runtime = match ShellRuntime::new_with_components(cx, components) {
                Ok(runtime) => runtime,
                Err(error) => {
                    eprintln!("failed to start the script runtime: {error}");
                    std::process::exit(1);
                }
            };

            // What `process.exit(code)` means here. The runtime never decides
            // this — one plugin must not be able to end an application somebody
            // is working in — but this host *is* the process, so ending it is
            // the honest answer. An embedded host installs something else:
            // closing a panel, closing a window, or refusing.
            crate::on_exit_request(|request, _, _| {
                let code = request.code();
                tracing::info!("the application asked to exit with {code}");
                std::process::exit(code);
            });

            // Resolving here rather than leaving it to `load_app` gives the window
            // title and the watcher the real application root even when the command
            // line named `main.js`. A path that does not resolve is not reported
            // twice: the load below fails with the message that explains why.
            let root =
                crate::resolve_app_root(&directory, ENTRY).unwrap_or_else(|_| directory.clone());

            // A view's `init` can create GPUI entities, so the facade loads the
            // application inside the window builder.
            let manifest = read_local_manifest(&root);
            if let Ok(manifest) = &manifest {
                grant_local_access(&root, manifest.as_ref());
            }

            // The declarations describe the runtime that is about to run this
            // script, which is the only version worth editing against. Doing it
            // here rather than asking the author to remember a command is what
            // keeps a stale `gpui-kit.d.ts` from being possible at all.
            //
            // Every launch rather than development mode only: this binary runs an
            // application from its source directory, which is where somebody is
            // editing. Nothing is written when the file already matches, and a
            // directory that refuses the write is logged rather than fatal.
            let window_size = manifest
                .as_ref()
                .ok()
                .and_then(|manifest| manifest.as_ref())
                .and_then(|manifest| manifest.window());
            let identity = manifest
                .as_ref()
                .ok()
                .and_then(|manifest| manifest.as_ref())
                .map(|manifest| (manifest.id().to_owned(), manifest.name().to_owned()));
            let packaged = arguments.is_packaged();
            // A packaged application is not a directory anybody edits, and its
            // bundle may be signed or installed read-only.
            runtime.set_editor_files(!packaged);
            let manifest_error = manifest.err();
            let built: Rc<RefCell<Option<Entity<ShellRoot>>>> = Rc::new(RefCell::new(None));
            let sink = built.clone();
            let builder_runtime = runtime.clone();
            let application_root = root.clone();

            let build = move |window: &mut Window, cx: &mut App| -> Entity<ShellRoot> {
                let loaded = match &manifest_error {
                    Some(message) => Err(anyhow::anyhow!(message.clone())),
                    None => builder_runtime.try_load(&application_root, window, cx),
                };
                match loaded {
                    Ok(root) => {
                        *sink.borrow_mut() = Some(root.clone());
                        root
                    }
                    Err(error) => {
                        eprintln!("{error:#}");
                        let content = cx.new(|_| LoadFailure(format!("{error:#}")));
                        cx.new(|cx| ShellRoot::new(content.into(), window, cx))
                    }
                }
            };

            // A catalog whose components require a particular window root opens
            // the window itself; see `ComponentWindowOpener`. Everything after
            // this point only wants a `&mut Window`, so the handle is untyped
            // either way.
            let options = window_options(&root, brand, window_size, identity, packaged, cx);
            let window = match window_opener {
                Some(open) => {
                    let mut build = |window: &mut Window, cx: &mut App| build(window, cx).into();
                    open(cx, options, &mut build).expect("failed to open window")
                }
                None => AnyWindowHandle::from(
                    cx.open_window(options, build)
                        .expect("failed to open window"),
                ),
            };

            // Launched from a terminal, a GPUI process opens its window behind
            // whatever was in front — the terminal itself — and an application
            // that does not come forward looks like one that has not started.
            // This command *is* the application, so it brings itself forward
            // the way a double-clicked application would.
            //
            // Deferred to the next turn of the loop: asked for while the
            // application is still finishing its launch, the request is dropped.
            //
            // A spawned task rather than `cx.defer`: a deferred effect runs at
            // the end of whatever update flushes next, and on Wayland that can
            // be a platform callback. The desktop portal's button-layout and
            // appearance events call into the window while GPUI still holds the
            // Wayland client state borrowed, so activating from inside one
            // panics on that borrow, and the window dropped while unwinding
            // panics again and aborts. Whether the portal answers before the
            // first flush is a race, which is why only some launches crashed.
            // A task is polled from the executor's own event source, outside
            // every platform callback.
            cx.spawn(async move |cx| {
                cx.update(|cx| {
                    cx.activate(true);
                    let _ = window.update(cx, |_, window, _| window.activate_window());
                });
            })
            .detach();

            let loaded = built.borrow_mut().take().ok_or(());

            if arguments.is_watching() {
                match loaded {
                    Ok(root) => watch_sources(runtime, root, window, cx),
                    // A failed first load never produced a mounted application.
                    // Saying so is better than a `--watch` that looks armed and
                    // never fires.
                    Err(()) => eprintln!(
                        "--watch is inactive: the application did not load, so there is \
                        no view to reload into. Fix the declared entry and start gpui-shell again."
                    ),
                }
            }
        });
}

/// Checks the application description and eager native materialization in a
/// hidden window. This does not draw or validate a complete frame.
fn check(
    directory: PathBuf,
    arguments: CheckArguments,
    components: crate::FrozenComponentRegistry,
) -> ! {
    // Assets are served from the application directory, so the source has to be
    // installed on the `Application` before the loop starts. Resolving the root
    // here rather than inside the loop is what makes that possible; a path that
    // does not resolve falls back to the argument and fails later with the
    // message that explains why.
    let asset_root =
        crate::resolve_app_root(&directory, ENTRY).unwrap_or_else(|_| directory.clone());

    gpui_platform::application()
        .with_assets(AppAssets::new(asset_root))
        .run(move |cx| {
            let mut outcome = CheckOutcome::default();
            crate::init_with_components(cx, &components);
            install_palette(cx);

            // Read before the catalog is handed to the runtime, which takes it.
            let window_opener = components.window_opener();

            let runtime = match ShellRuntime::new_with_components(cx, components) {
                Ok(runtime) => runtime,
                Err(error) => {
                    outcome.fail(format!("{error:#}"));
                    outcome.exit(&directory);
                }
            };

            let root = match crate::resolve_app_root(&directory, ENTRY) {
                Ok(root) => root,
                Err(error) => {
                    outcome.fail(format!("{error:#}"));
                    outcome.exit(&directory);
                }
            };

            let manifest = match read_local_manifest(&root) {
                Ok(manifest) => manifest,
                Err(error) => {
                    outcome.fail(error);
                    outcome.exit(&directory);
                }
            };
            grant_local_access(&root, manifest.as_ref());

            // Finish opening before checking so catalog-owned roots and window
            // facilities are installed when native materializers use them.
            let build = |_: &mut Window, cx: &mut App| cx.new(|_| gpui::Empty);
            let options = hidden_window_options(cx);
            let opened = match window_opener {
                Some(open) => {
                    let mut build = |window: &mut Window, cx: &mut App| build(window, cx).into();
                    open(cx, options, &mut build)
                }
                None => cx.open_window(options, build).map(Into::into),
            };

            let result = opened.and_then(|window| {
                window.update(cx, |_, window, cx| runtime.check(&root, window, cx))?
            });
            match result {
                Ok(spec) => outcome.succeed(spec, arguments.print_spec),
                Err(error) => outcome.fail(format!("{error:#}")),
            }

            // Reporting and exiting happen here rather than after `run` returns:
            // an application loop that has opened a window does not unwind just
            // because nothing is shown, and a check that never terminates is worse
            // than one that reports nothing.
            outcome.exit(&directory);
        });

    unreachable!("the check exits from inside the application loop")
}

/// A window is needed to render, but nothing should appear on screen for a
/// check: it runs in editors, in CI, and in an agent's loop.
fn hidden_window_options(cx: &App) -> WindowOptions {
    WindowOptions {
        show: false,
        focus: false,
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(320.), px(240.)),
            cx,
        ))),
        ..Default::default()
    }
}

/// What the check found. Defaults to "nothing ran", which is itself a failure:
/// a check that silently does nothing must not report success.
#[derive(Default)]
struct CheckOutcome {
    error: Option<String>,
    spec: Option<String>,
    ran: bool,
}

impl CheckOutcome {
    // Quitting GPUI can terminate the native process with status zero before
    // the application loop returns, including failures before a window opens.
    fn exit(&self, directory: &Path) -> ! {
        self.report(directory);
        std::process::exit(self.status());
    }

    fn fail(&mut self, error: String) {
        self.ran = true;
        self.error = Some(error);
    }

    fn succeed(&mut self, spec: String, keep: bool) {
        self.ran = true;
        if keep {
            self.spec = Some(spec);
        }
    }

    fn status(&self) -> i32 {
        if self.ran && self.error.is_none() {
            0
        } else {
            1
        }
    }

    fn report(&self, directory: &Path) {
        match &self.error {
            Some(error) => {
                eprintln!("{error}");
                eprintln!("\ncheck failed: {}", directory.display());
            }
            None if !self.ran => {
                eprintln!("check did not run: the window never opened");
            }
            None => {
                if let Some(spec) = &self.spec {
                    println!("{spec}");
                }
                println!("check passed: {}", directory.display());
            }
        }
    }
}

/// Installs the storage location and the capability grant for a local run.
///
/// An application with a manifest keeps its data under the manifest `id`, so
/// running it from its source directory, installing it, moving it or packaging
/// it all reach the same data — and the path an author shows a user is one they
/// can predict.
fn grant_local_access(root: &Path, manifest: Option<&crate::plugin::PluginManifest>) {
    // A directory without a manifest has no name of its own, so its path is its
    // identity: the same directory always reaches the same data.
    let id = match manifest {
        Some(manifest) => manifest.id().to_owned(),
        None => crate::bundle_id_for_path(root),
    };
    let store = match crate::set_bundle_id(&id) {
        Ok(store) => store,
        Err(error) => {
            tracing::warn!("storage is unavailable: {error}");
            return;
        }
    };

    if let Err(error) = std::fs::create_dir_all(&store) {
        tracing::warn!(
            "storage is unavailable: cannot create {}: {error}",
            store.display()
        );
        return;
    }

    crate::set_capabilities(local_capabilities(root, &store, manifest));
    tracing::debug!("storage: {}", store.display());
}

gpui::actions!(gpui_shell, [Quit]);

/// The platform's quit shortcut: ⌘Q on macOS, Alt-F4 elsewhere.
///
/// This command is the whole application, so quitting it is its to answer —
/// the same reason `process.exit` ends the process here. A script that binds
/// the same keystroke itself still wins, because its binding is installed
/// later and GPUI prefers the most recent one.
fn install_quit(cx: &mut App) {
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.bind_keys([gpui::KeyBinding::new(quit_keystroke(), Quit, None)]);
}

const fn quit_keystroke() -> &'static str {
    if cfg!(target_os = "macos") {
        "cmd-q"
    } else {
        "alt-f4"
    }
}

/// The palette this command ships.
///
/// It lives with the binary rather than with the runtime because a palette is a
/// product decision: `gpui-shell` the command is an application and gets to
/// have a look, while `gpui_shell` the library must not decide how everything
/// built on it appears. An embedder installs its own the same way.
const PALETTE: &str = include_str!("default-tokens.json");

#[derive(serde::Deserialize)]
struct CommandThemes {
    light: CommandTheme,
}

#[derive(serde::Deserialize)]
struct CommandTheme {
    colors: gpui_base::ColorTokens,
    #[serde(default)]
    radius: gpui_base::RadiusTokens,
    #[serde(default)]
    spacing: gpui_base::SpacingTokens,
    #[serde(default)]
    typography: gpui_base::TypographyTokens,
    #[serde(default)]
    shadow: gpui_base::ShadowTokens,
}

fn install_palette(cx: &mut App) {
    match serde_json::from_str::<CommandThemes>(PALETTE) {
        Ok(themes) => {
            let tokens = &mut gpui_base::Theme::global_mut(cx).tokens;
            tokens.colors = themes.light.colors;
            tokens.radius = themes.light.radius;
            tokens.spacing = themes.light.spacing;
            tokens.typography = themes.light.typography;
            tokens.shadow = themes.light.shadow;
        }
        // The file is compiled in, so a parse error is a build-time mistake
        // that reached a user; the neutral fallback keeps the window legible.
        Err(error) => tracing::error!("shipped palette did not parse: {error}"),
    }
}

/// Reads and validates inert application metadata before any entry code runs.
fn read_local_manifest(root: &Path) -> Result<Option<crate::plugin::PluginManifest>, String> {
    let path = root.join(crate::plugin::MANIFEST_FILE);
    if !path.exists() {
        return Ok(None);
    }

    crate::plugin::PluginManifest::read(root)
        .map(Some)
        .map_err(|error| format!("invalid {}: {error}", path.display()))
}

/// What a locally run application is allowed to do.
///
/// Running a directory from the command line is an explicit act of trust, the
/// same as `node app.js`: the application may read its own sources and use its
/// own storage, and whatever else its manifest asks for. Without a manifest it
/// is deliberately narrower than "everything" — no network, no process
/// execution, no clipboard, and no filesystem access outside those two
/// directories.
///
/// The two directories are *added* to the manifest's roots. Replacing them
/// would silently drop every other directory the manifest names, such as a
/// folder under `${homeDir}` the application exists to manage.
fn local_capabilities(
    root: &Path,
    store: &Path,
    manifest: Option<&crate::plugin::PluginManifest>,
) -> Capabilities {
    let capabilities = if let Some(manifest) = manifest {
        manifest.capabilities(root, store)
    } else {
        // Preserve the original local-app experience for source directories
        // that are not plugin bundles. Their private store is the only ambient
        // state they receive; network and clipboard stay denied.
        Capabilities::new().storage(true)
    };

    capabilities
        .add_read_roots([root.to_path_buf(), store.to_path_buf()])
        .add_write_roots([store.to_path_buf()])
        .exit(true)
}

/// The window an application opens in. `window_size` is the manifest's
/// `window`; without one the host's default applies.
///
/// `identity` is the manifest's `id` and `name`. The `id` is the window's
/// application id, which is how a Linux desktop matches the window to the
/// `<id>.desktop` entry `bundle` installs. A packaged application is titled
/// by its `name`: its directory is always `app`, which names nothing.
fn window_options(
    root: &Path,
    brand: HostBrand,
    window_size: Option<crate::plugin::WindowSize>,
    identity: Option<(String, String)>,
    packaged: bool,
    cx: &App,
) -> WindowOptions {
    let (app_id, name) = identity.unzip();
    let title = match name {
        Some(name) if packaged => name,
        _ => root
            .file_name()
            .map(|name| format!("{} — {}", name.to_string_lossy(), brand.name()))
            .unwrap_or_else(|| brand.name().to_owned()),
    };

    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            window_size
                .map(|window| size(px(window.width()), px(window.height())))
                .unwrap_or(size(px(880.), px(720.))),
            cx,
        ))),
        // A window with no title is unidentifiable in a switcher or a tiling
        // layout, which is how this one first reached a user.
        titlebar: Some(TitlebarOptions {
            title: Some(title.into()),
            ..Default::default()
        }),
        app_id,
        ..Default::default()
    }
}

/// Polls the application directory and reloads the view when it changes.
///
/// The loop, the failure toast and the exit conditions all live in
/// `ShellRuntime` — the binary used to carry its own copy of them, which
/// meant an embedded host got a quieter hot-reload than the CLI for no reason
/// anyone had decided. `--watch` is the one thing that is this binary's to
/// answer, because the person running it is the person editing; an embedded
/// host has no such flag and takes the debug build as the answer.
fn watch_sources(
    runtime: Rc<ShellRuntime>,
    root: Entity<ShellRoot>,
    window: AnyWindowHandle,
    cx: &mut App,
) {
    // Untyped on purpose. A typed `WindowHandle::update` leases the root view
    // for the length of the closure, and `watch` reads that same entity to find
    // the mounted application — a second borrow GPUI answers with a panic. All
    // this call wants from the window is a `&mut Window`.
    let started = window.update(cx, |_, window, cx| runtime.watch(&root, window, cx));

    match started {
        // Detached on purpose: this watcher lasts as long as the window, and
        // the window is the whole application.
        Ok(Ok(watch)) => watch.forget(),
        Ok(Err(error)) => tracing::error!("cannot watch for changes: {error}"),
        Err(error) => tracing::error!("cannot watch for changes: {error}"),
    }
}

/// What the window shows when the application could not be loaded.
///
/// A failed load still opens a window: the error belongs on screen, not only in
/// a terminal the user may not be watching.
struct LoadFailure(String);

impl Render for LoadFailure {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::failure_surface(
            "This application could not be loaded",
            &self.0,
            "An application directory needs main.js, or the entry its gpui-shell.json \
             names, default-exporting a class that extends View.",
            window,
            cx,
        )
    }
}

// ---------------------------------------------------------------------------
// Logging
// ---------------------------------------------------------------------------

/// Sends `tracing` events to stderr.
///
/// The crate depends on `tracing` but not on `tracing-subscriber`, so without
/// this every `tracing::error!` the runtime emits — a throwing event handler, an
/// unhandled promise rejection, an overlay opened from the wrong phase — is
/// dropped on the floor and the author sees a view that simply stopped
/// responding. One line per event on stderr is the whole requirement here;
/// filtering, spans, and formatting are what `tracing-subscriber` is for, and
/// the day this binary wants any of them it should take that dependency rather
/// than grow the subscriber below.
fn install_log_sink(max_level: Level) {
    // An error means something already installed a subscriber, which is a
    // better sink than this one by definition.
    let _ = tracing::subscriber::set_global_default(StderrSubscriber { max_level });
}

struct StderrSubscriber {
    max_level: Level,
}

impl Subscriber for StderrSubscriber {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        *metadata.level() <= self.max_level
    }

    /// Lets `tracing` skip the callsite entirely instead of asking on every
    /// event, which is what keeps a disabled `debug!` in a render path free.
    fn max_level_hint(&self) -> Option<LevelFilter> {
        Some(LevelFilter::from_level(self.max_level))
    }

    /// Spans are not recorded, so every span shares one id. A subscriber that
    /// prints events and nothing else has no use for span identity.
    fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
        span::Id::from_u64(1)
    }

    fn record(&self, _: &span::Id, _: &span::Record<'_>) {}

    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut fields = EventFields::default();
        event.record(&mut fields);
        eprintln!(
            "{:<5} {}: {}",
            event.metadata().level(),
            event.metadata().target(),
            fields.0
        );
    }

    fn enter(&self, _: &span::Id) {}

    fn exit(&self, _: &span::Id) {}
}

/// One event's fields, flattened to a line.
#[derive(Default)]
struct EventFields(String);

impl Visit for EventFields {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if !self.0.is_empty() {
            self.0.push(' ');
        }
        // The `message` field is the event's own text and is printed bare;
        // everything else is structured data and keeps its name.
        let _ = if field.name() == "message" {
            write!(self.0, "{value:?}")
        } else {
            write!(self.0, "{}={value:?}", field.name())
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(arguments: &[&str]) -> Result<Invocation, String> {
        parse(
            arguments.iter().map(|argument| (*argument).to_string()),
            HostBrand::gpui_shell(),
        )
    }

    fn run_args(arguments: &[&str]) -> Arguments {
        match parse_args(arguments).expect("the arguments should parse") {
            Invocation::Run(arguments) => arguments,
            other => panic!("expected a run, got {other:?}"),
        }
    }

    fn check_args(arguments: &[&str]) -> CheckArguments {
        match parse_args(arguments).expect("the arguments should parse") {
            Invocation::Check(arguments) => arguments,
            other => panic!("expected a check, got {other:?}"),
        }
    }

    #[test]
    fn check_takes_a_directory_and_an_optional_spec_dump() {
        let arguments = check_args(&["check", "examples/js_todolist"]);
        assert_eq!(
            arguments.application,
            Application::Path(PathBuf::from("examples/js_todolist"))
        );
        assert!(!arguments.print_spec);

        let arguments = check_args(&["check", "examples/js_todolist", "--print-spec"]);
        assert!(arguments.print_spec);
    }

    #[test]
    fn check_is_a_command_not_a_directory() {
        // A directory literally named `check` would be ambiguous; the command
        // wins, which is why the error for a missing path has to be clear.
        assert_eq!(
            check_args(&["check"]).application,
            Application::Path(std::env::current_dir().unwrap()),
            "`check` alone checks the application in the current directory"
        );
    }

    fn bundle_args(arguments: &[&str]) -> BundleArguments {
        match parse_args(arguments).expect("the arguments should parse") {
            Invocation::Bundle(arguments) => arguments,
            other => panic!("expected a bundle, got {other:?}"),
        }
    }

    #[test]
    fn bundle_takes_a_directory_and_its_own_flags() {
        let arguments = bundle_args(&[
            "bundle",
            "-d",
            "clock",
            "--platform",
            "linux",
            "--icon",
            "icon.png",
            "--out",
            "out",
            "--download-url",
            "https://example.com/clock",
        ]);
        assert_eq!(arguments.application, Application::Path("clock".into()));
        assert_eq!(
            arguments.options,
            bundle::BundleOptions {
                platform: Some(bundle::Platform::Linux),
                icon: Some("icon.png".into()),
                out: Some("out".into()),
                download_url: Some("https://example.com/clock".into()),
            }
        );
        assert!(parse_args(&["bundle", "--download-url", "example.com"]).is_err());
        assert!(parse_args(&["run", "--download-url", "https://example.com"]).is_err());

        let arguments = bundle_args(&["bundle"]);
        assert_eq!(
            arguments.application,
            Application::Path(std::env::current_dir().unwrap()),
            "`bundle` alone bundles the application in the current directory"
        );
        assert_eq!(arguments.options, bundle::BundleOptions::default());
    }

    #[test]
    fn dev_is_run_with_development_mode() {
        let arguments = run_args(&["dev", "-d", "app"]);
        assert_eq!(arguments.application, Application::Path("app".into()));
        assert!(arguments.is_watching() && arguments.is_verbose());
        assert!(!arguments.is_development(), "the sandbox stays as shipped");
        assert!(run_args(&["dev", "app", "--dev"]).is_development());
        assert!(!arguments.is_packaged());
        assert_eq!(
            run_args(&["dev"]).application,
            Application::Path(std::env::current_dir().unwrap()),
            "`dev` alone runs the application in the current directory"
        );
        let error = parse_args(&["check", "app", "--watch"]).expect_err("only `run` watches");
        assert!(error.contains("only for `run` and `dev`"), "{error}");
        assert!(parse_args(&["dev", "app", "--packaged"]).is_err());
    }

    #[test]
    fn packaged_is_a_run_flag() {
        let arguments = run_args(&["run", "-d", "app", "--packaged"]);
        assert!(arguments.is_packaged());
        assert!(!run_args(&["run", "-d", "app"]).is_packaged());

        let error = parse_args(&["check", "app", "--packaged"]).expect_err("only `run` packages");
        assert!(error.contains("only for `run`"), "{error}");
    }

    #[test]
    fn bundle_flags_are_checked() {
        let error = parse_args(&["run", "./app", "--platform", "linux"])
            .expect_err("only `bundle` has a platform");
        assert!(error.contains("only for `bundle`"), "{error}");

        let error = parse_args(&["bundle", "--platform", "beos"]).expect_err("an unknown platform");
        assert!(error.contains("macos, linux or windows"), "{error}");

        let error =
            parse_args(&["bundle", "--out", "--icon", "x.png"]).expect_err("a missing value");
        assert!(error.contains("`--out` expects"), "{error}");
    }

    #[test]
    fn a_bare_directory_runs_without_watching() {
        let arguments = run_args(&["run", "examples/js_todolist"]);

        assert_eq!(
            arguments.application,
            Application::Path(PathBuf::from("examples/js_todolist"))
        );
        assert!(!arguments.is_watching());
        assert!(!arguments.is_development());
    }

    #[test]
    fn watch_is_accepted_on_either_side_of_the_directory() {
        assert!(run_args(&["run", "./app", "--watch"]).is_watching());
        assert!(run_args(&["--watch", "run", "./app"]).is_watching());
    }

    #[test]
    fn the_dev_flag_only_relaxes_the_sandbox() {
        let arguments = run_args(&["run", "./app", "--dev"]);

        assert!(arguments.is_development());
        assert!(
            !arguments.is_watching(),
            "`dev` is the command that reloads"
        );
        assert!(!arguments.is_verbose());
    }

    #[test]
    fn an_unknown_flag_is_reported_rather_than_taken_as_a_path() {
        let error =
            parse_args(&["run", "./app", "--wtach"]).expect_err("a typo is not a directory");
        assert!(error.contains("--wtach"), "the message names the flag");
    }

    #[test]
    fn a_second_directory_is_reported() {
        let error = parse_args(&["run", "./app", "other"]).expect_err("only one application runs");
        assert!(error.contains("other"));
    }

    #[test]
    fn a_bare_name_runs_an_installed_application_and_a_path_runs_a_directory() {
        assert_eq!(
            run_args(&["run", "clock"]).application,
            Application::Installed("clock".to_owned())
        );
        assert_eq!(
            run_args(&["run", "--dir", "clock"]).application,
            Application::Path(PathBuf::from("clock"))
        );
        assert_eq!(
            run_args(&["run", "-d", "clock"]).application,
            Application::Path(PathBuf::from("clock"))
        );
        assert_eq!(
            run_args(&["run", "./clock", "--watch"]).application,
            Application::Path(PathBuf::from("./clock"))
        );
        assert_eq!(
            check_args(&["check", "clock"]).application,
            Application::Installed("clock".to_owned())
        );

        assert!(
            parse_args(&["--dir"]).is_err(),
            "the flag needs a directory"
        );
        assert!(parse_args(&["--dir", "--watch"]).is_err());
        assert!(parse_args(&["run", "clock", "--dir", "other"]).is_err());
    }

    #[test]
    fn an_application_needs_the_run_command() {
        let error = parse_args(&["clock"]).expect_err("a bare name is not a command");
        assert!(error.contains("gpui-shell run clock"), "{error}");
        let error = parse_args(&["-d", "app"]).expect_err("a bare directory is not a command");
        assert!(error.contains("gpui-shell run -d app"), "{error}");
        assert_eq!(
            run_args(&["run"]).application,
            Application::Path(std::env::current_dir().unwrap()),
            "`run` alone runs the application in the current directory"
        );
    }

    #[test]
    fn no_command_is_the_packaged_application_or_the_help() {
        let Ok(Invocation::Bare(arguments)) = parse_args(&["--dev"]) else {
            panic!("no command is decided when the command runs");
        };
        assert_eq!(arguments.application, Application::Bundled);
        assert!(matches!(parse_args(&[]), Ok(Invocation::Bare(_))));
    }

    #[test]
    fn new_takes_the_directory_to_create() {
        assert_eq!(
            parse_args(&["new", "clock"]).unwrap(),
            Invocation::New(PathBuf::from("clock")),
            "a bare name is a directory here, the way `cargo new` reads it"
        );
        assert_eq!(
            parse_args(&["new", "-d", "apps/clock"]).unwrap(),
            Invocation::New(PathBuf::from("apps/clock"))
        );
        assert!(parse_args(&["new"]).is_err());
    }

    #[test]
    fn list_takes_no_application() {
        assert_eq!(parse_args(&["list"]).unwrap(), Invocation::List);
        assert!(parse_args(&["list", "clock"]).is_err());
    }

    #[test]
    fn agents_prints_or_writes_into_one_application() {
        assert_eq!(parse_args(&["agents"]).unwrap(), Invocation::Agents(None));
        assert_eq!(
            parse_args(&["agents", "--dir", "app"]).unwrap(),
            Invocation::Agents(Some(Application::Path(PathBuf::from("app"))))
        );
        assert!(parse_args(&["agents", "app", "other"]).is_err());
    }

    #[test]
    fn help_and_version_answer_before_anything_else() {
        // Reachable even when the rest of the line is unusable, which is when a
        // caller most needs to be told what the accepted arguments are.
        for arguments in [&["--help"][..], &["--nope", "--help"][..]] {
            assert!(matches!(parse_args(arguments), Ok(Invocation::Print(_))));
        }

        let Ok(Invocation::Print(text)) = parse_args(&["--version"]) else {
            panic!("--version prints");
        };
        assert!(text.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn help_lists_every_flag() {
        let help = help(HostBrand::gpui_shell());
        for flag in ["run", "--dir", "--watch", "--dev", "--help", "--version"] {
            assert!(help.contains(flag), "`{flag}` is missing from the help");
        }
    }

    #[test]
    fn a_local_application_manifest_grants_its_declared_network_capabilities() {
        let root = std::env::temp_dir().join(format!(
            "gpui-shell-local-manifest-current-{}",
            std::process::id()
        ));
        let data = root.join("data");
        std::fs::create_dir_all(&root).expect("temporary app directory");
        std::fs::write(
            root.join("gpui-shell.json"),
            r#"{
              "id": "com.example.market",
              "name": "Market",
              "version": "1.0.0",
              "entry": "main.js",
              "capabilities": {
                "network": { "hosts": ["quotes.example.com"] },
                "storage": true,
                "clipboard": { "write": true }
              }
            }"#,
        )
        .expect("manifest");

        let manifest = read_local_manifest(&root)
            .expect("valid manifest")
            .expect("present manifest");
        let capabilities = local_capabilities(&root, &data, Some(&manifest));
        assert!(capabilities.may_reach("quotes.example.com"));
        assert!(capabilities.has_storage());
        assert!(capabilities.is_clipboard_writable());
        assert!(!capabilities.is_clipboard_readable());
        assert!(!capabilities.may_reach("trade.example.com"));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_local_application_keeps_the_directories_its_manifest_grants() {
        let root = std::env::temp_dir().join(format!(
            "gpui-shell-local-manifest-roots-{}",
            std::process::id()
        ));
        let data = root.join("data");
        std::fs::create_dir_all(&root).expect("temporary app directory");
        std::fs::write(
            root.join("gpui-shell.json"),
            r#"{
              "id": "com.example.skills",
              "name": "Skills",
              "entry": "main.js",
              "capabilities": {
                "fs": {
                  "read": ["/srv/shared", "${pluginDir}"],
                  "write": ["/srv/shared/out"]
                }
              }
            }"#,
        )
        .expect("manifest");

        let manifest = read_local_manifest(&root)
            .expect("valid manifest")
            .expect("present manifest");
        let summary = local_capabilities(&root, &data, Some(&manifest)).summary();
        let line = |prefix: &str| {
            summary
                .iter()
                .find(|line| line.starts_with(prefix))
                .cloned()
                .unwrap_or_default()
        };

        // The manifest's own roots survive, and the application's directory and
        // storage are added rather than put in their place — once each.
        let read = line("fs read: ");
        let shared = Path::new("/srv/shared").display().to_string();
        assert!(read.contains(&shared), "{read}");
        assert!(read.contains(&data.display().to_string()), "{read}");
        assert_eq!(
            read.matches(&root.display().to_string()).count(),
            2,
            "{read}"
        );
        let write = line("fs write: ");
        let out = Path::new("/srv/shared/out").display().to_string();
        assert!(write.contains(&out), "{write}");
        assert!(write.contains(&data.display().to_string()), "{write}");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn an_incompatible_local_manifest_is_refused_before_loading_script() {
        let root = std::env::temp_dir().join(format!(
            "gpui-shell-local-manifest-version-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("temporary app directory");
        std::fs::write(
            root.join("gpui-shell.json"),
            r#"{
              "id": "com.example.future",
              "name": "Future",
              "version": "1.0.0",
              "shell-version": "99.0.0",
              "entry": "main.js"
            }"#,
        )
        .expect("manifest");

        let error = read_local_manifest(&root).expect_err("future shells are incompatible");
        assert!(error.contains("requires gpui-shell 99.0.0"), "{error}");

        let _ = std::fs::remove_dir_all(root);
    }
}

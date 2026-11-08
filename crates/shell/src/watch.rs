//! Hot reload: watching an application directory and rebuilding its view.
//!
//! The design (`docs/gpui-shell.md` §21.2) is a five step pipeline:
//!
//! ```text
//! file change detected → debounce ~200ms → tear down the view →
//! re-evaluate the application module → rebuild the view →
//! optionally restore serialized state
//! ```
//!
//! Two properties matter more than the mechanism.
//!
//! The first is that a script error must never take the host down (§21.1). A
//! reload runs untrusted script: the module can throw while it is evaluated,
//! and the view constructor can throw while it runs. [`ShellRuntime::watch`]
//! therefore
//! drives reloads that do all fallible work before touching the live entity, so a broken save
//! leaves the previous working view on screen with the error reported to the
//! caller — the same promise the render-time error overlay makes.
//!
//! The second is that this module stays engine independent. It names
//! [`ShellRuntime`] and [`Entity<ScriptView>`] and nothing below them, exactly
//! like every other module above `engine::`. Reloading uses the same engine seam
//! whatever the engine turns out to be.
//!
//! # Why polling, and what a real watcher would buy
//!
//! The internal source watcher compares modification stamps instead of subscribing to
//! filesystem events, because `gpui-shell` deliberately takes no dependency on
//! `notify`. Polling is honest for the job it has: the host drives it from a
//! 250 ms GPUI timer, and the cost is one `stat` per
//! source file in a directory that holds a handful of them.
//!
//! A `notify`-based watcher would improve three things, none of which is fatal
//! here: latency would drop from "up to one poll interval" to milliseconds; the
//! cost would stop scaling with the file count, which matters for an
//! application that vendors a large dependency tree; and it would see the
//! changes a stamp cannot — a rename or an atomic replace that preserves both
//! size and timestamp. An event-backed detector could replace this internal
//! polling mechanism without changing the public [`ShellRuntime::watch`]
//! lifecycle.

use std::{
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, Instant, SystemTime},
};

use anyhow::{Context as _, Result, bail};
use gpui::{App, Entity, Task, Window};

use crate::{
    engine::ShellRuntime,
    root::{ShellRoot, ToastLevel, ToastRequest},
    scope,
    view::ScriptView,
};

/// The default quiet period. An editor writes a file several times in a burst —
/// truncate, write, rename — and each of those is a distinct change. Reloading
/// on the first one would evaluate a half-written module.
const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(200);

/// How deep the scan descends before it gives up. An application directory is
/// flat by design; the limit exists so a symlink farm or a vendored package
/// tree cannot turn one poll into an unbounded walk.
const MAX_DEPTH: usize = 8;

/// The most files one scan will stat. Same reasoning as [`MAX_DEPTH`]: a poll
/// repeats for the lifetime of the watcher and must have a bounded cost.
const MAX_FILES: usize = 4096;

/// Extensions a reload can be triggered by. Only sources the runtime actually
/// evaluates count — a change to a README should not restart the application.
const SOURCE_EXTENSIONS: [&str; 2] = ["js", "mjs"];

/// Directory names that are never application source, skipped before their
/// contents are stated. Hidden entries are skipped separately.
const SKIPPED_DIRECTORIES: [&str; 2] = ["node_modules", "target"];

/// Watches an application directory and reports when its sources change.
///
/// The watcher answers one question — "has the tree settled after a change?" —
/// and deliberately does not know what to do about it. Deciding to reload, and
/// what to do when the reload fails, belongs to the host.
pub(crate) struct SourceWatcher {
    directory: PathBuf,
    debounce: Duration,
    /// The stamp reported by the last scan. A change is a difference from this.
    stamp: TreeStamp,
    /// When the tree was last seen changing. `Some` means a change has been
    /// observed but not yet reported, because the debounce window is still
    /// open. Every further change pushes this forward, which is what collapses
    /// a burst into one report.
    changed_at: Option<Instant>,
}

impl SourceWatcher {
    /// Starts watching `directory`, taking the current tree as the baseline.
    ///
    /// The baseline is captured here rather than on the first [`poll`] so that
    /// starting a watcher does not itself look like a change.
    ///
    /// [`poll`]: Self::poll
    pub(crate) fn new(directory: PathBuf) -> Result<Self> {
        let stamp = scan(&directory)?;
        Ok(Self {
            directory,
            debounce: DEFAULT_DEBOUNCE,
            stamp,
            changed_at: None,
        })
    }

    /// Sets the debounce window: how long the tree has to stay still before a
    /// change is reported.
    ///
    /// Shorten it in tests, where the writes are deliberate and there is no
    /// burst to absorb. [`Duration::ZERO`] makes [`poll`] report a change on
    /// the very poll that observes it.
    ///
    /// [`poll`]: Self::poll
    #[cfg(test)]
    pub(crate) fn with_debounce(mut self, window: Duration) -> Self {
        self.debounce = window;
        self
    }

    /// Returns true when the tree changed since the last poll and has been
    /// still for at least the debounce window.
    ///
    /// Returns true at most once per burst: reporting clears the pending
    /// change, so a host that polls in a loop reloads once per save rather than
    /// once per poll. A directory that has been deleted reads as an empty tree,
    /// which is a change like any other and then stays quiet. A source tree
    /// beyond the scan limit returns an error rather than being watched only
    /// partially.
    pub(crate) fn poll(&mut self) -> Result<bool> {
        let stamp = scan(&self.directory)?;
        if stamp != self.stamp {
            self.stamp = stamp;
            self.changed_at = Some(Instant::now());
        }

        Ok(match self.changed_at {
            Some(at) if at.elapsed() >= self.debounce => {
                self.changed_at = None;
                true
            }
            _ => false,
        })
    }
}

/// A cheap summary of a source tree, compared for equality to detect a change.
///
/// It is three aggregates rather than a file list because a poll must not
/// allocate proportionally to the tree on every tick, and because equality is
/// the only question being asked. Each aggregate covers a case the others miss:
/// `newest` catches an edit in place, `files` catches an add or a delete, and
/// `bytes` catches an edit whose timestamp did not move — which happens on
/// filesystems with coarse timestamps, and in tests that write twice quickly.
///
/// What it cannot see is a change that preserves all three, such as swapping
/// two files' names. That is the honest cost of not using `notify`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
struct TreeStamp {
    newest: Option<SystemTime>,
    files: usize,
    bytes: u64,
}

/// Every watched file's newest modification time, plus the counts that make the
/// stamp sensitive to additions and edits.
///
/// A missing or unreadable directory is not an error: an application directory
/// can vanish mid-edit (a checkout, a move), and the watcher's job is to keep
/// running and report the tree it can see, which is an empty one.
fn scan(directory: &Path) -> Result<TreeStamp> {
    scan_with_limit(directory, MAX_FILES)
}

fn scan_with_limit(directory: &Path, max_files: usize) -> Result<TreeStamp> {
    let mut stamp = TreeStamp::default();
    let mut pending = vec![(directory.to_path_buf(), 0usize)];

    while let Some((dir, depth)) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') {
                continue;
            }

            // `DirEntry::file_type` does not follow symlinks, so a symlinked
            // directory is neither a file nor a directory here and is skipped.
            // That is what keeps a symlink cycle from hanging the poll.
            let Ok(file_type) = entry.file_type() else {
                continue;
            };

            if file_type.is_dir() {
                if depth < MAX_DEPTH && !SKIPPED_DIRECTORIES.contains(&name.as_ref()) {
                    pending.push((entry.path(), depth + 1));
                }
                continue;
            }

            if !file_type.is_file() || !is_source(&name) {
                continue;
            }

            let Ok(metadata) = entry.metadata() else {
                continue;
            };

            if stamp.files >= max_files {
                bail!(
                    "source watch for `{}` exceeds the {max_files}-file limit",
                    directory.display()
                );
            }

            stamp.files += 1;
            stamp.bytes = stamp.bytes.saturating_add(metadata.len());
            if let Ok(modified) = metadata.modified() {
                stamp.newest = Some(match stamp.newest {
                    Some(newest) => newest.max(modified),
                    None => modified,
                });
            }
        }
    }

    Ok(stamp)
}

/// Whether a file name is script source the runtime would evaluate.
fn is_source(name: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(_, extension)| SOURCE_EXTENSIONS.contains(&extension))
}

/// Reloads an application in place, keeping the window and its entity.
///
/// The window, the entity handle, and every host-side reference to it survive:
/// only the script object behind the view is replaced. That is what makes a
/// reload invisible to the host — no window is reopened, no layout is rebuilt.
///
/// # Atomicity
///
/// Both fallible steps happen before the live view is touched:
///
/// 1. re-evaluate the application module, which can throw;
/// 2. construct a new view instance, which can also throw;
/// 3. only then swap the object in and notify.
///
/// So a save that does not compile returns `Err` and changes nothing on screen.
/// The caller should surface the error — a toast, or the same error surface a
/// render failure uses — and keep the previous view running (§21.1).
///
/// # Phase
///
/// A reload mutates a view and requests a repaint, so it may only run from a
/// phase that allows a notify: an event or a task, never a render or a layout
/// pass (see [`crate::scope`]). Calling it mid-render would swap the object out
/// from under the element tree currently being built. Outside any scope — from
/// a host timer, which is the expected caller — there is no frame in progress
/// and the reload is safe.
///
/// # State preservation
///
/// §21.2 lists carrying state across a reload as optional, and routes it
/// through the same `serialize()` / `deserialize()` round trip as layout
/// persistence (§15.3). That path does not exist yet, so this function does not
/// invent a second one: the new instance starts from its constructor's state.
/// When the serialization path lands, it belongs between steps 2 and 3 above —
/// read the old object's state before the swap, hand it to the new object
/// after — which is why the swap is a single statement at the end.
/// How often an embedded watcher looks. The binary uses the same figure; a
/// quarter second is under the threshold at which a save feels like it did not
/// take, and far above the cost of one `stat` per source file.
pub(crate) const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// A running source watch. Dropping it stops the loop.
///
/// Returned rather than detached so a host that unmounts a panel can stop
/// watching for it. The loop also ends on its own when the view, the runtime or
/// the window goes away — the handle is for the case where none of those has
/// happened and the host simply wants it to stop.
#[must_use = "dropping the handle stops the watcher; use `.forget()` to keep it running"]
pub struct Watcher {
    task: Option<Task<()>>,
}

impl Watcher {
    /// Starts reloading `view` whenever application sources change.
    ///
    /// This method has no hidden build-mode policy. A command-line host can
    /// call it after parsing `--watch`; an embedded host can place the call in
    /// a `#[cfg(debug_assertions)]` block. A failed reload leaves the last good
    /// view running and reports the error through tracing and on the view's
    /// own failure banner.
    ///
    /// The watcher holds the runtime and view weakly, so it cannot keep an
    /// unmounted application alive.
    pub(crate) fn start(
        runtime: &Rc<ShellRuntime>,
        view: &Entity<ScriptView>,
        directory: PathBuf,
        entry: impl Into<String>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Self> {
        if !Rc::ptr_eq(&view.read(cx).runtime(), runtime) {
            bail!("this ScriptView belongs to a different gpui-shell runtime");
        }
        let entry = entry.into();
        let handle = window.window_handle();
        let mut watcher = SourceWatcher::new(directory.clone())?;
        let runtime = Rc::downgrade(runtime);
        let view = view.downgrade();

        let task = cx.spawn(async move |cx| {
            loop {
                cx.background_executor().timer(POLL_INTERVAL).await;

                // The scan runs on a background thread. It is bounded — depth 8,
                // 4,096 files — but that bound is a `stat` per source file, four
                // times a second, and on a slow directory that is a steady periodic
                // stall in a place a user would experience as the window hitching.
                // Only the answer comes back.
                let (changed, scanned) = cx
                    .background_executor()
                    .spawn(async move {
                        let changed = watcher.poll();
                        (changed, watcher)
                    })
                    .await;
                watcher = scanned;

                let (Some(runtime), Some(view)) = (runtime.upgrade(), view.upgrade()) else {
                    break;
                };

                let changed = match changed {
                    Ok(changed) => changed,
                    Err(error) => {
                        let message = format!("{error:#}");
                        tracing::error!("source watch stopped: {message}");
                        let _ = handle.update(cx, |_, window, cx| {
                            report_failure(&message, window, cx);
                        });
                        break;
                    }
                };

                if !changed {
                    continue;
                }

                let reached = handle.update(cx, |_, window, cx| {
                    let failure = match reload(&runtime, &view, &directory, &entry, window, cx) {
                        Ok(()) => {
                            tracing::info!("reloaded {}", directory.display());
                            None
                        }
                        // `{error:#}` keeps the `anyhow` context chain, which is what
                        // names the file and the stage that failed.
                        Err(error) => {
                            tracing::error!("reload failed: {error:#}");
                            Some(reload_failure_message(&error))
                        }
                    };
                    view.update(cx, |view, cx| view.set_reload_error(failure, cx));
                });

                if reached.is_err() {
                    break;
                }
            }
        });

        Ok(Self { task: Some(task) })
    }

    /// Lets the watcher run for as long as the view does.
    pub fn forget(mut self) {
        if let Some(task) = self.task.take() {
            task.detach();
        }
    }
}

impl ShellRuntime {
    /// Starts watching an application previously mounted with [`Self::load`]
    /// or [`Self::try_load`].
    ///
    /// The root retains the resolved directory, manifest entry, and typed
    /// script view, so the host does not repeat metadata or downcast content.
    ///
    /// It reads that root, so the caller must not already be holding it: reach
    /// the window through `AnyWindowHandle::update` or `App::update_window`,
    /// not through `WindowHandle::<ShellRoot>::update`, which leases the root
    /// view for the length of its closure.
    pub fn watch(
        self: &Rc<Self>,
        root: &Entity<ShellRoot>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Watcher> {
        let (view, application_root, entry) = {
            let root = root.read(cx);
            let application = root
                .application()
                .context("this ShellRoot does not contain a loaded script application")?;
            (
                application.view.clone(),
                application.root.clone(),
                application.entry.clone(),
            )
        };
        if !Rc::ptr_eq(&view.read(cx).runtime(), self) {
            bail!("this ShellRoot belongs to a different gpui-shell runtime");
        }
        Watcher::start(self, &view, application_root, entry, window, cx)
    }

    /// Rebuilds the snapshot for an application mounted by [`Self::load`] or
    /// [`Self::try_load`] after host-owned state changes.
    pub fn refresh(&self, root: &Entity<ShellRoot>, cx: &mut App) -> Result<()> {
        let view = root
            .read(cx)
            .application()
            .context("this ShellRoot does not contain a loaded script application")?
            .view
            .clone();
        if !std::ptr::eq(view.read(cx).runtime().as_ref(), self) {
            bail!("this ShellRoot belongs to a different gpui-shell runtime");
        }
        view.update(cx, |view, cx| view.refresh(cx));
        Ok(())
    }
}

/// The toast id, fixed so a repeated failure replaces the standing message
/// rather than stacking a column of them.
const WATCH_TOAST: &str = "shell-watch";

/// Reports that the watcher itself stopped, when the host mounted a
/// [`ShellRoot`].
///
/// A failed *reload* is the view's own failure and is shown on the view; this
/// is the one failure that belongs to no view — after it, edits stop reaching
/// the window at all, and nothing on the view would say so.
fn report_failure(message: &str, window: &mut Window, cx: &mut App) {
    ShellRoot::update(window, cx, |root, window, cx| {
        root.push_toast(
            ToastRequest::new("Stopped watching for changes")
                .with_description(message.to_owned())
                .with_level(ToastLevel::Error)
                .with_timeout(None)
                .with_id(WATCH_TOAST),
            window,
            cx,
        );
    });
}

/// Puts the cause first, because a failure surface shows one line collapsed.
///
/// `anyhow`'s `{:#}` leads with the outermost context — `reloading
/// /home/…/app:` — which fills that line with the one part the reader already
/// knows. The stages follow the cause's own detail, innermost last.
fn reload_failure_message(error: &anyhow::Error) -> String {
    let cause = error.root_cause().to_string();
    let stages = error
        .chain()
        .take(error.chain().count().saturating_sub(1))
        .map(|context| format!("while {context}"))
        .collect::<Vec<_>>()
        .join("\n");
    if stages.is_empty() {
        cause
    } else {
        format!("{cause}\n\n{stages}")
    }
}

pub(crate) fn reload(
    runtime: &Rc<ShellRuntime>,
    view: &Entity<ScriptView>,
    directory: &Path,
    entry: &str,
    window: &mut Window,
    cx: &mut App,
) -> Result<()> {
    if let Some(phase) = scope::current_phase()
        && !phase.allows_notify()
    {
        bail!(
            "a reload was requested during the {} phase; reload from an event, \
             a task, or a host timer instead",
            phase.as_str()
        );
    }

    let policy = view.read(cx).policy();

    // Everything that can fail runs first. On QuickJS this re-evaluates the
    // module into the same context, which §21.2 notes is one grade coarser than
    // a full teardown — an ES module cannot be unloaded, so old definitions stay
    // reachable from anything that captured them. Discarding and rebuilding the
    // whole context is the clean form, and belongs behind the engine seam
    // rather than here.
    let loaded = {
        let (_scope, _) = scope::enter_with_runtime(
            runtime,
            window,
            cx,
            scope::ScopePhase::Task,
            Some(view.clone()),
            policy.clone(),
        );
        runtime
            .load_app(directory, entry)
            .with_context(|| format!("reloading {}", directory.display()))
            .and_then(|view_type| {
                runtime
                    .instantiate_for_view(&view_type, view.clone(), window, cx)
                    .with_context(|| format!("rebuilding the view from {}", directory.display()))
            })
    };

    let object = match loaded {
        Ok(object) => object,
        Err(error) => return Err(error),
    };

    let previous = view.read(cx).object().application_generation();
    let replacement = object.application_generation();
    if let Some(previous) = previous
        && replacement
            .as_ref()
            .is_none_or(|replacement| !Rc::ptr_eq(&previous, replacement))
    {
        runtime.release_application_generation(&previous, cx);
    }

    view.update(cx, |view, cx| {
        view.replace_object(object);
        cx.notify();
    });
    window.refresh();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A unique directory under the system temp directory, removed on drop.
    ///
    /// The crate has no `tempfile` dependency and this is the only place that
    /// wants one, which is not enough reason to add it.
    struct TempTree(PathBuf);

    impl TempTree {
        fn new(label: &str) -> Self {
            static COUNTER: AtomicU32 = AtomicU32::new(0);

            let unique = format!(
                "gpui-shell-watch-{label}-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            );
            let path = std::env::temp_dir().join(unique);
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("creating the temporary tree");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn write(&self, name: &str, contents: &str) {
            std::fs::write(self.0.join(name), contents).expect("writing a source file");
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn quiet_directory_reports_no_change() {
        let tree = TempTree::new("quiet");
        tree.write("main.js", "export default class {}\n");

        let mut watcher = SourceWatcher::new(tree.path().to_path_buf())
            .unwrap()
            .with_debounce(Duration::ZERO);

        assert!(!watcher.poll().unwrap());
        assert!(!watcher.poll().unwrap());
    }

    #[test]
    fn touched_file_reports_a_change_once() {
        let tree = TempTree::new("touched");
        tree.write("main.js", "export default class {}\n");

        let mut watcher = SourceWatcher::new(tree.path().to_path_buf())
            .unwrap()
            .with_debounce(Duration::ZERO);
        assert!(!watcher.poll().unwrap());

        tree.write("main.js", "export default class { render() {} }\n");
        assert!(watcher.poll().unwrap(), "an edited source file is a change");
        assert!(
            !watcher.poll().unwrap(),
            "the change is reported once, not per poll"
        );
    }

    #[test]
    fn non_source_file_is_ignored() {
        let tree = TempTree::new("non-source");
        tree.write("main.js", "export default class {}\n");

        let mut watcher = SourceWatcher::new(tree.path().to_path_buf())
            .unwrap()
            .with_debounce(Duration::ZERO);

        tree.write("README.md", "notes\n");
        assert!(!watcher.poll().unwrap());
    }

    #[test]
    fn debounce_window_suppresses_a_burst() {
        let tree = TempTree::new("burst");
        tree.write("main.js", "export default class {}\n");

        let window = Duration::from_millis(300);
        let mut watcher = SourceWatcher::new(tree.path().to_path_buf())
            .unwrap()
            .with_debounce(window);

        // An editor's save: several writes in quick succession. None of them may
        // reload on its own, because the module is only whole after the last.
        tree.write("main.js", "");
        assert!(!watcher.poll().unwrap());
        tree.write("main.js", "export default class { render() {} }\n");
        assert!(!watcher.poll().unwrap());
        tree.write("helper.js", "export const helper = 1;\n");
        assert!(!watcher.poll().unwrap());

        std::thread::sleep(window + Duration::from_millis(120));
        assert!(
            watcher.poll().unwrap(),
            "the settled burst reloads exactly once"
        );
        assert!(!watcher.poll().unwrap());
    }

    #[test]
    fn missing_directory_does_not_panic() {
        let tree = TempTree::new("missing");
        tree.write("main.js", "export default class {}\n");

        let mut watcher = SourceWatcher::new(tree.path().to_path_buf())
            .unwrap()
            .with_debounce(Duration::ZERO);
        assert!(!watcher.poll().unwrap());

        std::fs::remove_dir_all(tree.path()).expect("removing the tree");

        // Losing the tree is a change like any other, and then the watcher goes
        // quiet instead of reporting a change on every tick.
        assert!(watcher.poll().unwrap());
        assert!(!watcher.poll().unwrap());
    }

    #[test]
    fn never_existing_directory_is_quiet() {
        let path = std::env::temp_dir().join("gpui-shell-watch-does-not-exist");
        let mut watcher = SourceWatcher::new(path)
            .unwrap()
            .with_debounce(Duration::ZERO);

        assert!(!watcher.poll().unwrap());
        assert!(!watcher.poll().unwrap());
    }

    #[test]
    fn nested_sources_are_watched_and_hidden_entries_are_not() {
        let tree = TempTree::new("nested");
        tree.write("main.js", "export default class {}\n");
        std::fs::create_dir_all(tree.path().join("lib")).expect("creating a nested directory");
        std::fs::create_dir_all(tree.path().join(".cache")).expect("creating a hidden directory");

        let mut watcher = SourceWatcher::new(tree.path().to_path_buf())
            .unwrap()
            .with_debounce(Duration::ZERO);

        std::fs::write(tree.path().join(".cache/main.js"), "ignored\n").expect("writing");
        assert!(
            !watcher.poll().unwrap(),
            "hidden directories are not application source"
        );

        std::fs::write(tree.path().join("lib/helper.js"), "export const a = 1;\n")
            .expect("writing");
        assert!(watcher.poll().unwrap(), "a nested source file is watched");
    }

    #[test]
    fn an_oversized_source_tree_is_refused_instead_of_partially_watched() {
        let tree = TempTree::new("oversized");
        tree.write("a.js", "export const a = 1;\n");
        tree.write("b.js", "export const b = 2;\n");

        let error = scan_with_limit(tree.path(), 1).unwrap_err();
        assert!(
            error.to_string().contains("exceeds the 1-file limit"),
            "the refusal should tell the host why hot reload cannot start: {error:#}"
        );
    }

    #[test]
    fn non_source_files_do_not_count_toward_the_source_limit() {
        let tree = TempTree::new("source-limit-ignores-readme");
        tree.write("main.js", "export default class {}\n");
        tree.write("README.md", "application notes\n");

        let stamp = scan_with_limit(tree.path(), 1)
            .expect("one source plus a README should fit a one-source limit");
        assert_eq!(stamp.files, 1);
    }

    #[test]
    fn a_non_source_tree_fits_a_zero_source_limit() {
        let tree = TempTree::new("zero-source-limit");
        tree.write("README.md", "application notes\n");

        let stamp = scan_with_limit(tree.path(), 0)
            .expect("non-source files should not consume the source-file budget");
        assert_eq!(stamp.files, 0);
    }
}

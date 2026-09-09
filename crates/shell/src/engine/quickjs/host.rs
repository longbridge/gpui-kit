//! System capability APIs on the `gpui-kit` module: `fs`, `store`, `clipboard` and
//! `log` (design doc §17).
//!
//! Nothing here is available by default (§19.2). Each loaded application keeps
//! a frozen policy, and every entry point restores that policy in its call
//! scope. Changing the host default therefore affects applications loaded
//! afterwards, not an application that is already running.
//!
//! Two rules keep this file honest:
//!
//! - **One path resolver.** Every filesystem path goes through
//!   [`Capabilities::resolve`], never through `std::fs` directly. `fs` and
//!   the capability-gated `os.*` entry points that arrive later therefore share
//!   one policy, and there is no second place for a traversal bug to hide.
//! - **A denial names its manifest key.** The error a script sees is the
//!   instruction for fixing it: which `capabilities.*` key to declare. Wording
//!   follows [`CapabilityError`], and the cases that resolver does not cover
//!   (clipboard read versus write) are spelled out the same way here.
//!
//! # Where the work happens
//!
//! Every `fs` call is a capability check *here* and a syscall *somewhere else*.
//! The check is cheap, needs the ambient scope, and stays on the calling thread
//! — so a denial is still a thrown error at the call site rather than a rejected
//! promise a script might never await. The syscall has no bound: a network
//! volume, a cold disk or a large file blocks for as long as it likes, and the
//! interrupt budget cannot even see that time because it is spent in the kernel.
//! So it goes to the background executor and the promise settles back on the
//! main thread.
//!
//! `store` is deliberately not like this. It is a cache with a write-through,
//! so `get` and `set` answer from memory; §17.1's requirement that `store.flush`
//! become awaitable is still open, and is the only synchronous write left.

use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use gpui::{App, ClipboardItem};
use rquickjs::{
    Ctx, Error as JsError, Exception, FromJs, IntoJs, Object, Promise, Result as JsResult,
    TypedArray, Value,
    function::{Func, Opt, Rest},
};
use serde_json::Value as Json;

use crate::{
    capability::{Access, Capabilities, CapabilityError, Grant},
    policy::Policy,
    scope,
    storage::{Storage, persist},
};

use super::{
    host_modules::{MAX_BRIDGE_ARRAY_ITEMS, bridge_array_len},
    scheduler,
};

/// The grant the code now running was given.
///
/// Read from the call frame rather than from the thread: two plugins inside one
/// runtime hold two grants at the same time, and a continuation resuming after
/// an `await` brings its own back with it. The engine asks; it does not get to
/// answer.
pub fn capabilities() -> Capabilities {
    scope::policy().capabilities().clone()
}

/// Points the default policy's store at its backing file.
///
/// Each application gets its own file so one cannot read another's settings
/// (§17.3). A plugin host builds a policy per plugin instead; this is the
/// single-application path.
pub fn set_storage_path(path: PathBuf) {
    crate::policy::update_default(|policy| policy.with_storage_path(path));
}

/// The context argument is the engine's; the sub-objects are built from the
/// module's own [`Object::ctx`] instead. `Ctx` and `Object` are invariant in
/// `'js`, so the two elided lifetimes in this signature are distinct to the
/// compiler and a value built from one cannot be set on the other — the same
/// constraint that forces conversions into `FromJs`/`IntoJs` elsewhere in the
/// engine. They are the same context at run time.
pub fn install(_ctx: &Ctx<'_>, module: &Object<'_>) -> JsResult<()> {
    let ctx = module.ctx();

    // The clipboard and the browser are `App` methods in GPUI, so the script
    // reaches them through `cx`. These globals are the implementation the
    // prelude composes onto it; nothing names them from script.
    let globals = ctx.globals();
    globals.set("__clipboard_read_text", Func::from(read_from_clipboard))?;
    globals.set("__clipboard_write_text", Func::from(write_to_clipboard))?;
    globals.set("__open_url", Func::from(open_url))?;

    // `localStorage` and `sessionStorage` go on `window`, where a browser puts
    // them. The prelude assembles them from these.
    install_storage(ctx)?;
    Ok(())
}

/// Hands a URL to whatever the system opens URLs with.
///
/// This is `Link`'s `href` without the element, and it carries exactly the
/// authority that already does: the same absolute-HTTP(S)-with-a-host guard,
/// and no grant of its own, because a script that can describe a `Link` can
/// already open any address it likes. Adding a capability here would gate the
/// imperative half of a pair whose declarative half is ungated, which reads as
/// protection without being any.
///
/// The scheme check is the part that matters. Without it this becomes a way to
/// hand an arbitrary URI to whatever handler the desktop has registered for
/// its scheme, which is a considerably larger thing than opening a page.
fn open_url(ctx: Ctx<'_>, url: String) -> JsResult<()> {
    let valid = reqwest::Url::parse(&url).is_ok_and(|parsed| {
        matches!(parsed.scheme(), "http" | "https") && parsed.host_str().is_some()
    });
    if !valid {
        return Err(Exception::throw_type(
            &ctx,
            "cx.open_url(url) expects an absolute HTTP(S) URL with a host",
        ));
    }
    with_app(&ctx, "cx.open_url(url)", move |app| app.open_url(&url))
}

// -- Filesystem ------------------------------------------------------------

/// Every `fs` call is a capability check here and a syscall somewhere else.
///
/// The check is cheap and stays on this thread, so a denial is still a thrown
/// error at the call site rather than a rejected promise nobody awaited. The
/// syscall has no bound — a network volume, a cold disk, a large file — and
/// running it here would block the frame and the VM together, somewhere the
/// interrupt budget cannot see because the time is spent in the kernel.
///
/// Named functions rather than closures because the returned promise borrows
/// the context, and only a signature can say so.
pub(super) fn read_file<'js>(
    ctx: Ctx<'js>,
    path: String,
    encoding: Opt<Value<'js>>,
) -> JsResult<Promise<'js>> {
    let encoding = ReadEncoding::from_value(&ctx, encoding.0)?;
    let grant = grant(&ctx, &path, Access::Read)?;
    let name = grant.describe();
    let (dir, relative) = grant.into_parts();

    scheduler::blocking(&ctx, "fs.readFile(path)", move || {
        use std::io::Read as _;

        // One open through the granted directory, then everything on the handle
        // it returns. Asking a *path* for a size and then asking it again for
        // the bytes is two resolutions, and nothing says they name the same
        // inode.
        let mut file = dir
            .open(&relative)
            .map_err(|error| message("read", &name, &error))?;
        let size = file
            .metadata()
            .map(|metadata| metadata.len())
            .map_err(|error| message("read", &name, &error))?;
        if size > MAX_READ_BYTES {
            return Err(too_large(&name, size));
        }

        // Bounded by the ceiling rather than by the size just read, so a file
        // that grows between the two is truncated at the limit instead of being
        // allowed past it.
        let mut bytes = Vec::with_capacity(size as usize);
        file.by_ref()
            .take(MAX_READ_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| message("read", &name, &error))?;
        if bytes.len() as u64 > MAX_READ_BYTES {
            return Err(too_large(&name, bytes.len() as u64));
        }
        match encoding {
            ReadEncoding::Bytes => Ok(FileContents::Bytes(bytes)),
            ReadEncoding::Utf8 => String::from_utf8(bytes)
                .map(FileContents::Text)
                .map_err(|error| format!("cannot decode `{name}` as UTF-8: {error}")),
        }
    })
}

pub(super) fn write_file<'js>(
    ctx: Ctx<'js>,
    path: String,
    contents: Value<'js>,
) -> JsResult<Promise<'js>> {
    let contents = if contents.is_string() {
        String::from_js(&ctx, contents)?.into_bytes()
    } else {
        let bytes = TypedArray::<u8>::from_js(&ctx, contents).map_err(|_| {
            Exception::throw_type(
                &ctx,
                "fs.writeFile(path, contents) expects a string or Uint8Array",
            )
        })?;
        bytes
            .as_bytes()
            .ok_or_else(|| {
                Exception::throw_type(
                    &ctx,
                    "fs.writeFile(path, contents) received a detached Uint8Array",
                )
            })?
            .to_vec()
    };
    let grant = grant(&ctx, &path, Access::Write)?;
    let name = grant.describe();
    if contents.len() > MAX_WRITE_BYTES {
        return Err(Exception::throw_range(
            &ctx,
            &format!(
                "`{name}` write is {} bytes, over the {MAX_WRITE_BYTES}-byte fs.writeFile limit",
                contents.len()
            ),
        ));
    }
    let (dir, relative) = grant.into_parts();

    scheduler::blocking(&ctx, "fs.writeFile(path, contents)", move || {
        dir.write(&relative, contents)
            .map_err(|error| message("write", &name, &error))
    })
}

pub(super) fn list_dir<'js>(
    ctx: Ctx<'js>,
    path: String,
    options: Opt<Value<'js>>,
) -> JsResult<Promise<'js>> {
    let with_file_types = readdir_with_file_types(&ctx, options.0)?;
    let grant = grant(&ctx, &path, Access::Read)?;
    let name = grant.describe();
    let (dir, relative) = grant.into_parts();

    scheduler::blocking(&ctx, "fs.readdir(path)", move || {
        let entries = read_dir(&dir, &relative).map_err(|error| message("list", &name, &error))?;
        if with_file_types {
            Ok(DirectoryListing::Entries(entries))
        } else {
            Ok(DirectoryListing::Names(
                entries.into_iter().map(|entry| entry.name).collect(),
            ))
        }
    })
}

#[derive(Clone, Copy)]
enum ReadEncoding {
    Bytes,
    Utf8,
}

impl ReadEncoding {
    fn from_value<'js>(ctx: &Ctx<'js>, value: Option<Value<'js>>) -> JsResult<Self> {
        let Some(value) = value.filter(|value| !value.is_undefined() && !value.is_null()) else {
            return Ok(Self::Bytes);
        };
        let encoding = if value.is_string() {
            String::from_js(ctx, value)?
        } else if let Some(options) = value.into_object() {
            for key in options.keys::<String>() {
                let key = key?;
                if key != "encoding" {
                    return Err(Exception::throw_type(
                        ctx,
                        &format!("unknown fs.readFile option `{key}`; expected encoding"),
                    ));
                }
            }
            options.get::<_, String>("encoding")?
        } else {
            return Err(Exception::throw_type(
                ctx,
                "fs.readFile encoding must be \"utf8\" or { encoding: \"utf8\" }",
            ));
        };
        match encoding.to_ascii_lowercase().as_str() {
            "utf8" | "utf-8" => Ok(Self::Utf8),
            _ => Err(Exception::throw_type(
                ctx,
                "fs.readFile only supports UTF-8 text decoding",
            )),
        }
    }
}

enum FileContents {
    Bytes(Vec<u8>),
    Text(String),
}

impl<'js> IntoJs<'js> for FileContents {
    fn into_js(self, ctx: &Ctx<'js>) -> JsResult<Value<'js>> {
        match self {
            Self::Bytes(bytes) => TypedArray::<u8>::new(ctx.clone(), bytes)?.into_js(ctx),
            Self::Text(text) => text.into_js(ctx),
        }
    }
}

fn readdir_with_file_types<'js>(ctx: &Ctx<'js>, value: Option<Value<'js>>) -> JsResult<bool> {
    let Some(value) = value.filter(|value| !value.is_undefined() && !value.is_null()) else {
        return Ok(false);
    };
    let Some(options) = value.into_object() else {
        return Err(Exception::throw_type(
            ctx,
            "fs.readdir options must be { withFileTypes: boolean }",
        ));
    };
    for key in options.keys::<String>() {
        let key = key?;
        if key != "withFileTypes" {
            return Err(Exception::throw_type(
                ctx,
                &format!("unknown fs.readdir option `{key}`; expected withFileTypes"),
            ));
        }
    }
    Ok(options
        .get::<_, Option<bool>>("withFileTypes")?
        .unwrap_or(false))
}

/// A denied path throws rather than answering `false`: "you may not look" and
/// "it is not there" are different facts, and collapsing them would let a script
/// probe outside its roots one boolean at a time.
pub(super) fn exists<'js>(ctx: Ctx<'js>, path: String) -> JsResult<Promise<'js>> {
    let grant = grant(&ctx, &path, Access::Read)?;
    let (dir, relative) = grant.into_parts();

    scheduler::blocking(&ctx, "fs.exists(path)", move || {
        Ok(dir.try_exists(&relative).unwrap_or(false))
    })
}

/// Deleting a file, and deleting a directory, are two calls.
///
/// Rust splits them and so does this, because "remove" alone does not say
/// whether a directory is in scope — and the answer here is that a directory
/// only goes if it is empty. Write access is granted per root, so a recursive
/// remove would turn one mistyped path into the loss of an application's whole
/// data directory; a script that means it walks the tree itself.
pub(super) fn remove_file<'js>(ctx: Ctx<'js>, path: String) -> JsResult<Promise<'js>> {
    let grant = grant(&ctx, &path, Access::Write)?;
    let name = grant.describe();
    let (dir, relative) = grant.into_parts();

    scheduler::blocking(&ctx, "fs.unlink(path)", move || {
        dir.remove_file(&relative)
            .map_err(|error| message("remove", &name, &error))
    })
}

/// Removes an **empty** directory. A non-empty one is an error, not a tree walk.
pub(super) fn remove_dir<'js>(ctx: Ctx<'js>, path: String) -> JsResult<Promise<'js>> {
    let grant = grant(&ctx, &path, Access::Write)?;
    let name = grant.describe();
    let (dir, relative) = grant.into_parts();

    scheduler::blocking(&ctx, "fs.rmdir(path)", move || {
        dir.remove_dir(&relative)
            .map_err(|error| message("remove", &name, &error))
    })
}

/// Creates a directory. `{ recursive: true }` creates its parents too.
///
/// The name is `mkdir` and so are its semantics: bare, it creates one directory
/// and fails if the parent is missing, which is what `mkdir` means in every
/// runtime a script author has used. This used to be spelled `create_dir_all`
/// and was always recursive — a name that said what it did, but only by not
/// being the name everyone knows.
pub(super) fn mkdir<'js>(
    ctx: Ctx<'js>,
    path: String,
    options: Opt<MakeDirectory>,
) -> JsResult<Promise<'js>> {
    let grant = grant(&ctx, &path, Access::Write)?;
    let name = grant.describe();
    let (dir, relative) = grant.into_parts();
    let recursive = options.0.unwrap_or_default().recursive;

    scheduler::blocking(&ctx, "fs.mkdir(path, options)", move || {
        let made = if recursive {
            dir.create_dir_all(&relative)
        } else {
            dir.create_dir(&relative)
        };
        made.map_err(|error| message("create", &name, &error))
    })
}

/// `{ recursive }`, and nothing else.
#[derive(Default)]
pub(super) struct MakeDirectory {
    recursive: bool,
}

impl<'js> FromJs<'js> for MakeDirectory {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        if value.is_undefined() || value.is_null() {
            return Ok(Self::default());
        }

        let Some(object) = value.into_object() else {
            return Err(Exception::throw_type(
                ctx,
                "fs.mkdir(path, options) expects an object, such as { recursive: true }",
            ));
        };

        // A misspelled key silently dropped is a setting the author believes
        // they applied — here, the difference between one directory and a path.
        for key in object.keys::<String>() {
            let key = key?;
            if key != "recursive" {
                return Err(Exception::throw_type(
                    ctx,
                    &format!(
                        "unknown option `{key}` for fs.mkdir(path, options); \
                         expected recursive"
                    ),
                ));
            }
        }

        Ok(Self {
            recursive: object.get::<_, Option<bool>>("recursive")?.unwrap_or(false),
        })
    }
}

/// The ceiling on one `readFile`.
///
/// A script that asks for a file this large has almost certainly asked for the
/// wrong one, and the alternative to a limit is a string that has to fit in the
/// JavaScript heap — which is itself capped, so the failure without this is an
/// out-of-memory in the VM rather than a sentence naming the file.
const MAX_READ_BYTES: u64 = 64 * 1024 * 1024;
const MAX_WRITE_BYTES: usize = 8 * 1024 * 1024;
const MAX_READDIR_ENTRIES: usize = 10_000;
const MAX_READDIR_NAME_BYTES: usize = 1024 * 1024;

/// A failure, worded the way [`io_error`] words one.
///
/// Built off the main thread, where there is no `Ctx` to throw with, so it
/// travels back as a string and becomes an `Error` when the promise rejects.
fn message(verb: &str, name: &str, error: &std::io::Error) -> String {
    format!("cannot {verb} `{name}`: {error}")
}

fn too_large(name: &str, size: u64) -> String {
    format!(
        "`{name}` is {size} bytes, over the {MAX_READ_BYTES}-byte limit for \
         fs.readFile; read it in pieces or keep it out of the script"
    )
}

/// One entry of `fs.readdir(..., { withFileTypes: true })`.
///
/// `is_dir` rides along because there is no `stat` in this surface, and without
/// it every caller would have to guess from the name.
struct Dirent {
    name: String,
    is_dir: bool,
}

impl<'js> IntoJs<'js> for Dirent {
    fn into_js(self, ctx: &Ctx<'js>) -> JsResult<Value<'js>> {
        let object = Object::new(ctx.clone())?;
        object.set("name", self.name)?;
        let is_dir = self.is_dir;
        object.set("isDirectory", Func::from(move || is_dir))?;
        Ok(object.into_value())
    }
}

enum DirectoryListing {
    Names(Vec<String>),
    Entries(Vec<Dirent>),
}

impl<'js> IntoJs<'js> for DirectoryListing {
    fn into_js(self, ctx: &Ctx<'js>) -> JsResult<Value<'js>> {
        match self {
            Self::Names(names) => names.into_js(ctx),
            Self::Entries(entries) => entries.into_js(ctx),
        }
    }
}

/// Sorted by name, so a script that renders a listing does not have to sort it
/// and does not inherit the filesystem's arbitrary order.
fn read_dir(dir: &cap_std::fs::Dir, path: &Path) -> std::io::Result<Vec<Dirent>> {
    let mut entries = Vec::new();
    let mut name_bytes = 0;
    for entry in dir.read_dir(path)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        name_bytes += name.len();
        check_readdir_budget(entries.len(), name_bytes)?;
        entries.push(Dirent {
            name,
            is_dir: entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false),
        });
    }
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(entries)
}

fn check_readdir_budget(entries: usize, name_bytes: usize) -> std::io::Result<()> {
    if entries == MAX_READDIR_ENTRIES || name_bytes > MAX_READDIR_NAME_BYTES {
        return Err(std::io::Error::other(format!(
            "directory exceeded the {MAX_READDIR_ENTRIES}-entry or \
             {MAX_READDIR_NAME_BYTES}-name-byte fs.readdir limit"
        )));
    }
    Ok(())
}

/// The single path gate. Nothing in this module touches the filesystem except
/// through a [`Grant`] it got back from here.
fn grant(ctx: &Ctx<'_>, path: &str, access: Access) -> JsResult<Grant> {
    capabilities()
        .open(Path::new(path), access)
        .map_err(|error| Exception::throw_type(ctx, &denial(&error)))
}

/// Turns a [`CapabilityError`] into a message that always ends in the manifest
/// key to add. `OutsideRoots` on its own says only that the path is out of
/// bounds, which leaves an author guessing at the fix.
fn denial(error: &CapabilityError) -> String {
    match error {
        CapabilityError::OutsideRoots { access, .. } => format!(
            "{error}; add its directory to capabilities.fs.{} in the manifest",
            access_key(*access)
        ),
        other => other.to_string(),
    }
}

fn access_key(access: Access) -> &'static str {
    match access {
        Access::Read => "read",
        Access::Write => "write",
    }
}

/// Drives the store's write queue one step.
///
/// One write is in flight at a time and the completion drives the queue again,
/// so a `set` made while a write was on its way is picked up by the next one
/// rather than left in memory. That last part is not a refinement: with a
/// `dirty` flag and no follow-up, a mutation during a write was lost for good
/// unless another mutation happened to follow it.
fn drive_store(host: &scheduler::Host, policy: &Rc<Policy>) {
    // Settling is done outside `with_local_storage`, always: a `flush` resolving from
    // here re-enters script, which may call `localStorage.setItem`.
    let stalled = policy
        .with_local_storage(Storage::take_stalled)
        .unwrap_or_default();
    for wake in stalled {
        wake();
    }

    let Some(pending) = policy.with_local_storage(Storage::begin_write).flatten() else {
        return;
    };

    let revision = pending.revision();
    let (path, body) = pending.into_parts();
    let abandoned = policy.clone();
    let policy = policy.clone();
    let next = host.clone();

    // A failure is logged rather than thrown: nobody asked for this write, so
    // there is no call to fail. A script that wants to be told awaits `flush`,
    // which waits for this same write and rejects with it.
    let started = scheduler::detached_on(
        host,
        move || persist(&path, body),
        move |result| {
            // The policy that started the write, not whichever one happens to
            // be in force now. This runs outside any host call, so an ambient
            // lookup would find the *default* store and leave this one wedged
            // with a write in flight that never completes.
            let woken = policy
                .with_local_storage(|store| store.finish_write(revision, result))
                .unwrap_or_default();
            for wake in woken {
                wake();
            }
            drive_store(&next, &policy);
        },
    );

    if !started {
        // The executor is gone. Leaving the revision in flight would stop every
        // later write, so it is released rather than trusted.
        abandoned.with_local_storage(|store| store.abort_write(revision));
    }
}

/// Kicks the queue from inside a host call.
fn schedule_persist(ctx: &Ctx<'_>) {
    let Ok(host) = scheduler::host_for(ctx, "localStorage") else {
        return;
    };
    drive_store(&host, &scope::policy());
}

/// Resolves once everything written so far has reached the disk.
///
/// A barrier rather than a second writer. It used to start its own write, which
/// raced the automatic one through the same temporary file with nothing ordering
/// them — so the older revision could land last and undo the newer.
fn flush<'js>(ctx: Ctx<'js>) -> JsResult<Promise<'js>> {
    with_local_storage(&ctx, |store| {
        store
            .ensure_waiter_capacity()
            .map_err(|error| fail(&ctx, &error))
    })?;
    let (promise, resolve, reject) = ctx.promise()?;
    let settle = scheduler::deferred(&ctx, "localStorage.flush()", resolve, reject)?;

    // `with_local_storage` also enforces the capability, so a denied flush throws here
    // rather than resolving quietly.
    if let Some(settle) = with_local_storage(&ctx, |store| {
        store.wait(settle).map_err(|error| fail(&ctx, &error))
    })? {
        settle(Ok(()));
        return Ok(promise);
    }

    schedule_persist(&ctx);
    Ok(promise)
}

/// The Web Storage natives the prelude assembles into `localStorage` and
/// `sessionStorage`.
///
/// Globals rather than a module object: the two storages live on `window`, the
/// way a browser puts them there, so there is no specifier to import and
/// nothing for the module resolver to own.
///
/// Values are strings. `setItem` converting its argument is the browser's
/// behaviour, not a shortcut — an application that wants structure reaches for
/// `JSON.stringify`, exactly as it would on the web.
fn install_storage(ctx: &Ctx<'_>) -> JsResult<()> {
    let globals = ctx.globals();

    globals.set(
        "__storage_get",
        Func::from(
            |ctx: Ctx<'_>, session: bool, key: String| -> JsResult<Option<String>> {
                with_storage(&ctx, session, |store| {
                    let values = store.values().map_err(|error| fail(&ctx, &error))?;
                    Ok(values.get(&key).map(text_of))
                })
            },
        ),
    )?;

    // `setItem` and `removeItem` answer from the cache and mark it dirty; the
    // write they make necessary happens on a background thread. They stay
    // synchronous because that is the whole point of a cache — and because the
    // API they mirror is synchronous.
    globals.set(
        "__storage_set",
        Func::from(
            |ctx: Ctx<'_>, session: bool, key: String, value: String| -> JsResult<()> {
                with_storage(&ctx, session, |store| {
                    store
                        .set(key, Json::String(value))
                        .map_err(|error| fail(&ctx, &error))
                })?;
                schedule_persist(&ctx);
                Ok(())
            },
        ),
    )?;

    globals.set(
        "__storage_remove",
        Func::from(|ctx: Ctx<'_>, session: bool, key: String| -> JsResult<()> {
            with_storage(&ctx, session, |store| {
                store
                    .values()
                    .map_err(|error| fail(&ctx, &error))?
                    .remove(&key);
                store.touch();
                Ok(())
            })?;
            schedule_persist(&ctx);
            Ok(())
        }),
    )?;

    globals.set(
        "__storage_clear",
        Func::from(|ctx: Ctx<'_>, session: bool| -> JsResult<()> {
            with_storage(&ctx, session, |store| {
                let values = store.values().map_err(|error| fail(&ctx, &error))?;
                if values.is_empty() {
                    return Ok(());
                }
                values.clear();
                store.touch();
                Ok(())
            })?;
            schedule_persist(&ctx);
            Ok(())
        }),
    )?;

    globals.set(
        "__storage_length",
        Func::from(|ctx: Ctx<'_>, session: bool| -> JsResult<usize> {
            with_storage(&ctx, session, |store| {
                Ok(store.values().map_err(|error| fail(&ctx, &error))?.len())
            })
        }),
    )?;

    // The order is whatever the map iterates in, which is insertion order
    // here. The specification asks only that it be consistent between calls
    // and that it change when the set of keys changes, which this is: the same
    // order a browser gives you, for the same reason — nobody promised sorted.
    globals.set(
        "__storage_key",
        Func::from(
            |ctx: Ctx<'_>, session: bool, index: usize| -> JsResult<Option<String>> {
                with_storage(&ctx, session, |store| {
                    let values = store.values().map_err(|error| fail(&ctx, &error))?;
                    Ok(values.keys().nth(index).cloned())
                })
            },
        ),
    )?;

    // The one addition to the Web Storage surface. A browser's `setItem` is
    // durable by the time it returns; this one hands the write to a background
    // thread, so a script that must know the bytes landed has something to
    // await. On `sessionStorage` there is no disk, and it resolves at once.
    globals.set("__storage_flush", Func::from(flush))?;

    Ok(())
}

/// A stored value as the Web Storage API sees it: a string.
///
/// A file written before this API stored JSON, so a value that is not a string
/// comes back as its JSON text rather than as an error — the alternative is
/// refusing to start over data the script itself wrote.
fn text_of(value: &Json) -> String {
    match value {
        Json::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// Routes to one of the two storages, gating only the one that has a file.
///
/// `sessionStorage` needs no capability: it never leaves memory, and a script
/// that runs already holds its own memory. `localStorage` writes a file the
/// host placed, so it needs both the grant and the placement.
fn with_storage<R>(
    ctx: &Ctx<'_>,
    session: bool,
    body: impl FnOnce(&mut Storage) -> JsResult<R>,
) -> JsResult<R> {
    if session {
        return scope::policy()
            .with_session_storage(body)
            .expect("a policy always has its session storage");
    }
    with_local_storage(ctx, body)
}

/// Gates every `localStorage` entry point on the capability and on the host
/// having said where the file lives.
fn with_local_storage<R>(
    ctx: &Ctx<'_>,
    body: impl FnOnce(&mut Storage) -> JsResult<R>,
) -> JsResult<R> {
    if !capabilities().has_storage() {
        return Err(Exception::throw_type(
            ctx,
            &CapabilityError::StorageDenied.to_string(),
        ));
    }

    match scope::policy().with_local_storage(body) {
        Some(result) => result,
        // Not a manifest problem, so it does not name a capability key: the
        // embedder skipped `set_storage_path`.
        None => Err(Exception::throw_message(
            ctx,
            "localStorage has no backing file; the host must call \
             set_storage_path before the application runs",
        )),
    }
}

fn fail(ctx: &Ctx<'_>, message: &str) -> JsError {
    Exception::throw_message(ctx, message)
}

/// A cycle in a JS object graph would recurse forever, so depth is capped.
/// Real configuration data is nowhere near this deep.
const MAX_JSON_DEPTH: usize = 64;

/// The largest magnitude a JavaScript number represents exactly, and therefore
/// the largest one that may be narrowed to an integer without losing anything.
const MAX_EXACT_INTEGER: f64 = 9_007_199_254_740_992.0;

/// Arrays and plain objects only, matching what the store can persist.
/// Functions and `undefined` properties are dropped exactly as
/// `JSON.stringify` drops them, so a script's mental model transfers.
///
/// Shared with the dock binding, which reads a persisted layout back out of
/// script: a layout is plain data under exactly these rules.
pub(super) fn to_json(ctx: &Ctx<'_>, value: &Value<'_>, depth: usize) -> JsResult<Json> {
    if depth > MAX_JSON_DEPTH {
        return Err(Exception::throw_type(
            ctx,
            "value nests more than 64 levels deep; the store holds plain data and \
             cannot hold a reference cycle",
        ));
    }

    if value.is_null() || value.is_undefined() {
        return Ok(Json::Null);
    }
    if let Some(flag) = value.as_bool() {
        return Ok(Json::Bool(flag));
    }
    if let Some(number) = value.as_int() {
        return Ok(Json::Number(number.into()));
    }
    if let Some(number) = value.as_number() {
        // JavaScript has one number type, so an integral value *is* an integer
        // as far as a script is concerned — and something reading it back into
        // a Rust `usize` has to be able to see one. The tag cannot answer this:
        // QuickJS hands `JSON.parse("0")` back as a float, so a layout that
        // round-tripped through `JSON.stringify` would arrive as `0.0` and fail
        // to deserialize.
        if number.fract() == 0.0 && number.abs() <= MAX_EXACT_INTEGER {
            return Ok(Json::Number((number as i64).into()));
        }
        return serde_json::Number::from_f64(number)
            .map(Json::Number)
            .ok_or_else(|| {
                Exception::throw_type(
                    ctx,
                    "NaN and Infinity have no JSON form and cannot be stored",
                )
            });
    }
    if let Some(text) = value.as_string() {
        return Ok(Json::String(text.to_string()?));
    }
    if let Some(array) = value.as_array() {
        let length = bridge_array_len(ctx, &array)?;
        let mut items = Vec::new();
        items.try_reserve_exact(length).map_err(|_| {
            Exception::throw_range(
                ctx,
                &format!(
                    "array could not be reserved within the {MAX_BRIDGE_ARRAY_ITEMS}-item bridge limit"
                ),
            )
        })?;
        for index in 0..length {
            items.push(to_json(ctx, &array.get(index)?, depth + 1)?);
        }
        return Ok(Json::Array(items));
    }
    if value.is_function() {
        return Err(Exception::throw_type(
            ctx,
            "a function cannot be stored; the store holds data, not behaviour",
        ));
    }
    if let Some(object) = value.as_object() {
        let mut map = serde_json::Map::new();
        for property in object.props::<String, Value>() {
            let (key, entry) = property?;
            if entry.is_undefined() || entry.is_function() {
                continue;
            }
            map.insert(key, to_json(ctx, &entry, depth + 1)?);
        }
        return Ok(Json::Object(map));
    }

    Err(Exception::throw_type(
        ctx,
        "unsupported value; the store holds null, booleans, numbers, strings, \
         arrays and plain objects",
    ))
}

// -- Clipboard -------------------------------------------------------------

fn read_from_clipboard(ctx: Ctx<'_>) -> JsResult<Option<String>> {
    if !capabilities().is_clipboard_readable() {
        return Err(Exception::throw_type(&ctx, CLIPBOARD_READ_DENIED));
    }
    with_app(&ctx, "cx.read_from_clipboard()", |app| {
        app.read_from_clipboard().and_then(|item| item.text())
    })
}

fn write_to_clipboard(ctx: Ctx<'_>, text: String) -> JsResult<()> {
    if !capabilities().is_clipboard_writable() {
        return Err(Exception::throw_type(&ctx, CLIPBOARD_WRITE_DENIED));
    }
    with_app(&ctx, "cx.write_to_clipboard(text)", move |app| {
        app.write_to_clipboard(ClipboardItem::new_string(text))
    })
}

/// Read and write are separate grants, so the denial names the half that was
/// missing. Wording follows [`CapabilityError`].
const CLIPBOARD_READ_DENIED: &str =
    "reading the clipboard is not granted; declare capabilities.clipboard.read in the manifest";
const CLIPBOARD_WRITE_DENIED: &str =
    "writing the clipboard is not granted; declare capabilities.clipboard.write in the manifest";

/// Runs `body` against GPUI's `App`.
///
/// The `App` only exists for the duration of a host call (see [`scope`]), so a
/// clipboard call made from, say, a module's top level has nothing to talk to.
/// That reports as an ordinary script error rather than a panic — a misplaced
/// call is an application bug, not a host bug (§5.8).
fn with_app<R>(ctx: &Ctx<'_>, what: &str, body: impl FnOnce(&mut App) -> R) -> JsResult<R> {
    scope::with_current_app(body).ok_or_else(|| {
        Exception::throw_type(
            ctx,
            &format!(
                "{what} needs a live host call; call it from render, an event handler or a task"
            ),
        )
    })
}

// -- Log -------------------------------------------------------------------

pub(super) fn log_object<'js>(ctx: &Ctx<'js>) -> JsResult<Object<'js>> {
    let log = Object::new(ctx.clone())?;

    // No capability: a script that can run can already say something. Denying
    // logging would only cost the author their diagnostics.
    log.set(
        "debug",
        Func::from(|message: Printable, rest: Rest<Printable>| {
            tracing::debug!(target: SCRIPT_TARGET, "{}", line(message, rest));
        }),
    )?;
    log.set(
        "info",
        Func::from(|message: Printable, rest: Rest<Printable>| {
            tracing::info!(target: SCRIPT_TARGET, "{}", line(message, rest));
        }),
    )?;
    log.set(
        "log",
        Func::from(|message: Printable, rest: Rest<Printable>| {
            tracing::info!(target: SCRIPT_TARGET, "{}", line(message, rest));
        }),
    )?;
    log.set(
        "warn",
        Func::from(|message: Printable, rest: Rest<Printable>| {
            tracing::warn!(target: SCRIPT_TARGET, "{}", line(message, rest));
        }),
    )?;
    log.set(
        "error",
        Func::from(|message: Printable, rest: Rest<Printable>| {
            tracing::error!(target: SCRIPT_TARGET, "{}", line(message, rest));
        }),
    )?;

    Ok(log)
}

/// Script output is separable from host output in a log filter.
const SCRIPT_TARGET: &str = "gpui_shell::script";

/// Extra arguments are appended space-separated after the message, the way
/// `console.log` behaves — JS authors write `console.info("loaded", count)` without
/// thinking about it.
fn line(message: Printable, rest: Rest<Printable>) -> String {
    let mut line = message.0;
    for argument in rest.0 {
        line.push(' ');
        line.push_str(&argument.0);
    }
    line
}

/// Any JS value, rendered for a log line.
///
/// A wrapper rather than `String` because a script will pass numbers, objects
/// and `undefined` to a logger, and refusing those with a type error would make
/// the logger the least usable thing in the API.
struct Printable(String);

impl<'js> FromJs<'js> for Printable {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Self> {
        if let Some(text) = value.as_string() {
            return Ok(Self(text.to_string()?));
        }
        if value.is_undefined() {
            return Ok(Self("undefined".into()));
        }
        if value.is_function() {
            return Ok(Self("[function]".into()));
        }
        // Structured values print as JSON, which is what an author reading a
        // log wants to see; anything the conversion refuses prints as a
        // placeholder rather than aborting the call it was describing.
        Ok(match to_json(ctx, &value, 0) {
            Ok(json) => Self(json.to_string()),
            Err(_) => {
                ctx.catch();
                Self("[unprintable]".into())
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use rquickjs::{Context as JsContext, Runtime as JsRuntime};

    use super::*;

    /// A directory of our own under the system temporary directory. `tempfile`
    /// is not a dependency of this crate and one test module is not a reason to
    /// add one.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            static COUNTER: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "gpui-shell-host-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).expect("creating the test directory");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Builds a real VM with the host surface bound to `gpui`, the way the
    /// engine binds it. Each `#[test]` runs on its own thread, so the
    /// thread-local grant starts empty every time.
    fn with_host<R>(body: impl FnOnce(&Ctx<'_>) -> R) -> R {
        let runtime = JsRuntime::new().expect("starting the JS runtime");
        let context = JsContext::full(&runtime).expect("creating the JS context");
        context.with(|ctx| {
            let module = Object::new(ctx.clone()).expect("creating the module object");
            install(&ctx, &module).expect("installing the host surface");
            ctx.globals().set("gpui", module).expect("binding `gpui`");
            ctx.globals()
                .set("readFile", Func::from(read_file))
                .expect("binding the FS test adapter");
            // `console` is a global installed by the standard runtime, which
            // these tests do not load. It is this same object.
            ctx.globals()
                .set("console", log_object(&ctx).expect("log object"))
                .expect("binding `console`");
            body(&ctx)
        })
    }

    /// The message of a thrown exception, which is the part these tests are
    /// about: a denial has to say what to declare.
    fn error_of(ctx: &Ctx<'_>, source: &str) -> String {
        match ctx.eval::<Value, _>(source) {
            Ok(_) => panic!("`{source}` was expected to throw"),
            Err(JsError::Exception) => {
                let thrown = ctx.catch();
                thrown
                    .as_exception()
                    .and_then(|exception| exception.message())
                    .unwrap_or_else(|| format!("{thrown:?}"))
            }
            Err(error) => panic!("`{source}` failed without an exception: {error}"),
        }
    }

    #[test]
    fn a_read_outside_the_granted_root_names_the_manifest_key() {
        let directory = TempDir::new();
        crate::capability::install(
            Capabilities::new().read_roots([directory.path().to_path_buf()]),
        );

        let message = with_host(|ctx| error_of(ctx, "readFile('../../etc/passwd')"));
        assert!(
            message.contains("outside every granted read root"),
            "unexpected message: {message}"
        );
        assert!(
            message.contains("capabilities.fs.read"),
            "the denial must name the manifest key: {message}"
        );
    }

    #[test]
    fn a_read_without_a_grant_reports_the_missing_capability() {
        let message = with_host(|ctx| error_of(ctx, "readFile('items.json')"));
        assert!(
            message.contains("is not granted") && message.contains("capabilities.fs.read"),
            "unexpected message: {message}"
        );
    }

    #[test]
    fn local_storage_is_denied_without_the_capability() {
        let directory = TempDir::new();
        set_storage_path(directory.path().join("storage.json"));

        let message = with_host(|ctx| error_of(ctx, "__storage_get(false, 'theme')"));
        assert!(
            message.contains("capabilities.storage"),
            "unexpected message: {message}"
        );
    }

    /// `sessionStorage` never leaves memory, so there is nothing to grant.
    #[test]
    fn session_storage_needs_no_capability() {
        with_host(|ctx| {
            ctx.eval::<(), _>("__storage_set(true, 'theme', 'dark')")
                .expect("session storage is ungated");
            let value: String = ctx
                .eval("__storage_get(true, 'theme')")
                .expect("reading it back");
            assert_eq!(value, "dark");
        });
    }

    #[test]
    fn local_storage_round_trips_a_string() {
        let directory = TempDir::new();
        let file = directory.path().join("storage.json");
        crate::capability::install(Capabilities::new().storage(true));
        set_storage_path(file.clone());

        with_host(|ctx| {
            // Values are strings, exactly as they are in a browser, so anything
            // with structure goes through `JSON.stringify` on the way in — and
            // `setItem` converting what it is handed is the API's behaviour
            // rather than a shortcut taken here.
            ctx.eval::<(), _>(
                "__storage_set(false, 'window', JSON.stringify({ title: 'Notes', size: [640, 480] }))",
            )
            .expect("storing a serialized object");

            let title: String = ctx
                .eval("JSON.parse(__storage_get(false, 'window')).title")
                .expect("reading it back");
            assert_eq!(title, "Notes");

            assert!(
                ctx.eval::<Option<String>, _>("__storage_get(false, 'missing')")
                    .expect("an unset key")
                    .is_none(),
                "an unset key is null, which is what getItem promises"
            );

            let length: usize = ctx
                .eval("__storage_length(false)")
                .expect("reading the length");
            assert_eq!(length, 1);
        });

        // Reaching the disk is asynchronous, and needs an executor this harness
        // does not have — `tests/fs.rs` covers it. What matters here is that the
        // cache answers, which is why `setItem` and `getItem` stayed
        // synchronous.
        let _ = file;
    }

    #[test]
    fn local_storage_forgets_a_removed_key() {
        let directory = TempDir::new();
        crate::capability::install(Capabilities::new().storage(true));
        set_storage_path(directory.path().join("storage.json"));

        with_host(|ctx| {
            ctx.eval::<(), _>(
                "__storage_set(false, 'theme', 'dark'); __storage_remove(false, 'theme')",
            )
            .expect("setting then removing");
            let length: usize = ctx
                .eval("__storage_length(false)")
                .expect("reading the length");
            assert_eq!(length, 0);
        });
    }

    /// `clear` empties it, and `key(index)` walks what is left in a consistent
    /// order — the two members a script cannot reach any other way.
    #[test]
    fn local_storage_clears_and_walks_its_keys() {
        let directory = TempDir::new();
        crate::capability::install(Capabilities::new().storage(true));
        set_storage_path(directory.path().join("storage.json"));

        with_host(|ctx| {
            ctx.eval::<(), _>("__storage_set(false, 'b', '2'); __storage_set(false, 'a', '1')")
                .expect("two keys");
            // Insertion order, and the point of the assertion is that it is
            // *an* order that holds, not which one.
            let first: String = ctx.eval("__storage_key(false, 0)").expect("first key");
            let second: String = ctx.eval("__storage_key(false, 1)").expect("second key");
            assert_eq!((first.as_str(), second.as_str()), ("b", "a"));
            assert!(
                ctx.eval::<Option<String>, _>("__storage_key(false, 2)")
                    .expect("past the end")
                    .is_none(),
                "an index past the end is null, not an error"
            );

            ctx.eval::<(), _>("__storage_clear(false)").expect("clear");
            let length: usize = ctx.eval("__storage_length(false)").expect("length");
            assert_eq!(length, 0);
        });
    }

    #[test]
    fn the_clipboard_fails_cleanly_outside_a_host_call() {
        crate::capability::install(
            Capabilities::new()
                .clipboard_read(true)
                .clipboard_write(true),
        );

        let message = with_host(|ctx| error_of(ctx, "__clipboard_read_text()"));
        assert!(
            message.contains("needs a live host call"),
            "unexpected message: {message}"
        );
    }

    #[test]
    fn the_clipboard_denial_names_the_half_that_is_missing() {
        let message = with_host(|ctx| error_of(ctx, "__clipboard_write_text('x')"));
        assert!(
            message.contains("capabilities.clipboard.write"),
            "unexpected message: {message}"
        );
    }

    /// Nothing this module installs is reachable by importing it.
    ///
    /// It used to be: `store` and `log` were module members, and the guard here
    /// was that a member added to the module object without a matching export
    /// is a binding that works through `gpui.x` and fails at the import — which
    /// is how `open_url` first shipped, the call site there and the import not,
    /// with a browser that never opened as the only symptom.
    ///
    /// Every one of them has since moved to where GPUI keeps it: the clipboard
    /// and the browser onto `cx`, storage onto `window`, diagnostics onto the
    /// `console` every JavaScript runtime already has. So the guard inverts —
    /// what would be wrong now is a *leftover* export, resolving to a binding
    /// nothing installs.
    #[test]
    fn nothing_this_module_installs_is_a_named_export() {
        for name in [
            "store",
            "log",
            "clipboard",
            "open_url",
            "read_from_clipboard",
            "write_to_clipboard",
            "localStorage",
            "sessionStorage",
        ] {
            assert_eq!(
                super::super::module_exporting(name),
                None,
                "`{name}` no longer lives on a module; an export of it resolves to nothing"
            );
        }
    }

    #[test]
    fn open_url_takes_no_capability_and_fails_cleanly_outside_a_host_call() {
        // No grant is installed here on purpose: a script that can describe a
        // `Link` can already open any address, so gating this half alone would
        // read as protection without being any.
        crate::capability::install(Capabilities::new());

        let message = with_host(|ctx| error_of(ctx, "__open_url('https://example.com/x')"));
        assert!(
            message.contains("needs a live host call"),
            "unexpected message: {message}"
        );
        assert!(
            !message.contains("capabilities"),
            "open_url must not ask for a grant: {message}"
        );
    }

    #[test]
    fn logging_needs_no_capability_and_takes_extra_arguments() {
        with_host(|ctx| {
            ctx.eval::<(), _>("console.info('loaded', 3, { ok: true })")
                .expect("logging is always available");
        });
    }

    #[test]
    fn logging_a_sparse_huge_array_is_safely_unprintable() {
        with_host(|ctx| {
            ctx.eval::<(), _>(
                "const values = []; values.length = 0xffffffff; console.info(values)",
            )
            .expect("logging an oversized value must not panic or require a capability");
        });
    }

    #[test]
    fn readdir_budget_rejects_entry_and_name_aggregate_overflow() {
        let entry_error =
            check_readdir_budget(MAX_READDIR_ENTRIES, 1).expect_err("entry limit must reject");
        assert!(entry_error.to_string().contains("entry"));

        let name_error = check_readdir_budget(0, MAX_READDIR_NAME_BYTES + 1)
            .expect_err("name-byte limit must reject");
        assert!(name_error.to_string().contains("name-byte"));
    }
}

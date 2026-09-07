---
title: Capabilities
description: The default-deny model, the fs / storage / clipboard / process surface, where storage lives, and what the sandbox withholds.
order: 8
---

# Capabilities

A script gets **nothing** by default. No file access, no clipboard, no process execution, no network. `Capabilities::default()` is the empty set, and an assertion holds it there.

The one exception is storage, and only at the manifest layer: an application that does not mention `storage` gets its own `localStorage`, the way a browser hands one to every origin without being asked. That is a convention about what an author has to _write_, not a hole in the model — the Rust `Capabilities` still deny it until a host says otherwise, and a manifest may still say `"storage": false`. See [Storage](#storage).

The host grants what it grants, because only the host knows how far it trusts the code it is about to run. What it hands _out_ — its own Rust, exposed on purpose — is [HostModule](./host-module.md). A View freezes its capabilities when it is loaded; changing the default affects applications loaded afterward, never code that is already running under an approved grant.

```rust
gpui_kit::shell::set_capabilities(
    Capabilities::new()
        .read_roots([application_root.clone()])
        .write_roots([data_directory.clone()])
        .storage(true)
        .exit(true),
);
```

## What a locally run application is granted

Running a directory from the command line is an explicit act of trust — the same as `node app.js` — so `gpui-shell <directory>` grants a specific, narrow set:

|                   |                                                          |
| ----------------- | -------------------------------------------------------- |
| Read              | The application directory, and its own storage directory |
| Write             | Its own storage directory                                |
| Storage           | Granted                                                  |
| Clipboard         | **Not** granted                                          |
| Process execution | **Not** granted                                          |
| Exit request      | Granted                                                  |
| Network           | **Not** granted                                          |

An application can therefore read its own sources and assets and use its own storage, and nothing else. It is deliberately narrower than "everything", because an installed plugin will one day run through the same code path with a manifest deciding instead — and a grant that is generous for a local run would be the wrong default to inherit.

## Refusals name the fix

Every denial ends in the thing to declare, not just the fact of the refusal:

```text
filesystem read is not granted; declare capabilities.fs.read in the manifest
```

```text
`/etc/passwd` is outside every granted read root;
add its directory to capabilities.fs.read in the manifest
```

```text
storage is not granted; set capabilities.storage to true
```

```text
running `git` is not granted; add it to capabilities.fs.execute in the manifest
```

```text
process.exit() is not granted; set capabilities.process.exit to true in the manifest
```

## The manifest

A directory is recognized by **`gpui-shell.json`**. The manifest is inert data — discovery reads identity, optional version metadata, Git dependencies, and requested permissions without executing the entry module. It recognizes `id`, `name`, `version`, `shell-version`, `entry`, `dependencies`, and `capabilities`; only `id`, `name`, and `entry` are required:

```json
{
  "id": "com.example.quotes",
  "name": "Quotes",
  "version": "1.0.0",
  "shell-version": "0.6.0",
  "entry": "main.js",
  "dependencies": {
    "omarchy-ui": "huacnlee/omarchy-ui"
  },
  "capabilities": {
    "fs": { "read": ["${pluginDir}"], "write": ["${dataDir}"] },
    "network": {
      "hosts": ["stream.example.com"],
      "http": [
        {
          "scheme": "https",
          "host": "api.example.com",
          "methods": ["GET"],
          "path_prefixes": ["/v1/"]
        }
      ]
    },
    "storage": true,
    "clipboard": { "read": false, "write": true },
    "process": { "exit": false }
  }
}
```

`dependencies` maps a bare module name to a JavaScript package fetched from
Git before the entry module runs — `import { Title } from "omarchy-ui"`. The
string form takes strict GitHub shorthand or a full Git URL with an optional
`#ref`; the object form with an explicit `branch` or `tag` remains supported.
Every load also links the package where an editor finds it, so the import
carries the package's own types and documentation. See
[Dependencies](./dependencies.md) for version selection, the package entry,
the cache, and what an editor sees.

Every grant in that block defaults to _denied_ when omitted, except `storage`, which defaults to granted — write `"storage": false` to refuse it.

Unknown fields, invalid reverse-DNS ids, invalid explicitly declared SemVer values, incompatible `shell-version` values, escaping entries, and unknown `${...}` placeholders invalidate the manifest before code runs. Omitted `version` is reported as `unknown`. Omitted `shell-version` accepts the current runtime; when present, it names the oldest compatible gpui-shell release the application requires. Any runtime at or above that version is accepted. The standalone CLI refuses an invalid manifest instead of executing its entry with silently different assumptions.

Each scoped `network.http` rule binds the request scheme and effective port as well as its host, method and path. `scheme` defaults to `https`; `port` defaults to that scheme's standard port and only needs to be written for a non-default endpoint.

## `fs`

```js
import * as fs from "fs/promises";
```

Every call returns a promise. `await` them, or chain `.then` — and see the note below about `render`.

| Call                                        | Resolves to                     |
| ------------------------------------------- | ------------------------------- |
| `fs.readFile(path)`                         | `Uint8Array`                    |
| `fs.readFile(path, "utf8")`                 | UTF-8 text                      |
| `fs.writeFile(path, contents)`              | —                               |
| `fs.readdir(path)`                          | Names sorted by name            |
| `fs.readdir(path, { withFileTypes: true })` | `Dirent[]` with `isDirectory()` |
| `fs.exists(path)`                           | `true` / `false`                |
| `fs.unlink(path)`                           | —                               |
| `fs.rmdir(path)`                            | —                               |
| `fs.mkdir(path, options?)`                  | —                               |

```js
const source = await fs.readFile("notes.md", "utf8");
await fs.writeFile("notes.md", source + "\n");
```

A relative path resolves against a granted root; an absolute one must already be inside one. Every path in the surface goes through **one resolver**, so there is no second place for a traversal bug to hide. It normalizes the path — `../../etc/passwd` is rejected before it reaches the filesystem — and then settles containment against the filesystem rather than against the string, because a grant is a promise about a _directory_: `data/escape/passwd` is lexically inside the root and reads `/etc/passwd` if `escape` is a symlink. The deepest part of the path that exists is resolved, links and all, and has to still be under the root; a symlink that resolves to nothing is refused rather than guessed at.

**The grant is a handle, not a string.** The resolver hands back an open directory that cannot be made to name anything outside itself, and every read, write, listing, removal and mkdir runs against _that_ — so a path is never resolved twice and there is no window between deciding it is allowed and using it.

That matters because the obvious implementation does not work. Checking the path and then calling `std::fs` resolves it twice: a link already in place is caught by the check, and one that replaces a directory component _between_ the two is followed out of the root by the second resolution. This is [`cap-std`](https://docs.rs/cap-std), which is `openat2(RESOLVE_BENEATH)` on Linux and a per-component `openat` walk elsewhere.

Three of these behave in a way worth stating, each for the same reason:

**A denied path throws rather than answering `false`.** "You may not look" and "it is not there" are different facts, and collapsing them would let a script map the filesystem outside its roots one boolean at a time.

**Removing a file and removing a directory are two calls**, as they are in Rust, because "remove" alone does not say whether a directory is in scope. `remove_dir` takes an empty one and nothing else: write access is granted per root, so a recursive remove would turn one mistyped path into the loss of an application's whole data directory. A script that means it walks the tree itself.

**`mkdir` means what it means everywhere else.** Bare, it creates one directory and fails if the parent is missing; `{ recursive: true }` creates the parents too. It was `create_dir_all` — a name that said what it did, but only by not being the name every script author already knows.

**`read_dir` is sorted.** A script that renders a listing should not have to sort it, and should not inherit the filesystem's arbitrary order.

**Every call returns a promise.** The syscall runs off the main thread, because a disk has no bound on how long it takes and blocking here would stop the frame and the VM together — somewhere the interrupt budget cannot even see, since the time is spent in the kernel.

A **denial still throws at the call site** rather than rejecting. The capability check costs nothing and stays on the calling thread, and a rejected promise nobody awaited is a denial nobody sees.

`readFile` refuses a file over 64 MiB, naming it and the limit. The alternative to a ceiling is a string that has to fit in the JavaScript heap — which is itself capped — so the failure without one is an out-of-memory inside the VM rather than a sentence you can act on.

`writeFile` accepts at most 8 MiB per call. `readdir` stops at 10,000 entries or 1 MiB of UTF-8 name bytes, whichever comes first, so an adversarial directory cannot turn one promise into unbounded allocation.

::: tip Still do not read a file from `render`
`render` describes the interface; it cannot await. Read in `init` or an event handler, keep the result on the View, and `cx.notify()` when it arrives.
:::

## Storage

The [Web Storage API](https://developer.mozilla.org/en-US/docs/Web/API/Web_Storage_API), as a browser has it. There is nothing to import: `localStorage` and `sessionStorage` are globals, and also live on `window`.

```js
localStorage.setItem("todolist.items", JSON.stringify(items));
const saved = localStorage.getItem("todolist.items"); // null when the key is unset
localStorage.removeItem("todolist.items");
localStorage.length;
localStorage.key(0);
localStorage.clear();
```

| Member              | Description                                    |
| ------------------- | ---------------------------------------------- |
| `length`            | How many keys are stored                       |
| `key(index)`        | The key at that position, or `null`            |
| `getItem(key)`      | The value, or `null` when the key is unset     |
| `setItem(key, val)` | Stores it, converting the value to a string    |
| `removeItem(key)`   | Forgets one key                                |
| `clear()`           | Forgets all of them                            |
| `flush()`           | Resolves once the writes have reached the disk |

**The two differ only in how long they last.** `localStorage` is a file the host placed, and it survives a restart. `sessionStorage` is memory that goes with the process. That is also why only one of them is a capability: nothing `sessionStorage` holds ever leaves the process, so there is nothing to grant, and it works on a host that granted nothing.

**Values are strings**, exactly as on the web — `setItem` converts whatever it is handed. Anything with structure goes through `JSON.stringify` on the way in and `JSON.parse` on the way out, which is the same code you would write in a browser:

```js
localStorage.setItem(
  "window",
  JSON.stringify({ title: "Notes", size: [640, 480] }),
);
const window = JSON.parse(localStorage.getItem("window") ?? "{}");
```

Every member is synchronous, and deliberately: `getItem` is reachable from `render`, so the values are cached in memory and a read answers from there. A file read per render would be absurd.

**A mutation schedules the write rather than performing it.** The file is written on a background thread — to a temporary file, renamed over the target, so a crash mid-write leaves the previous settings intact rather than a truncated one — and one write is in flight at a time, so a burst of `setItem` calls becomes one file rather than one file each. Whatever changed while a write was on its way is written by the next one.

`await localStorage.flush()` when you need to know it landed. This is the one addition to the browser's interface, and it exists because a browser never has to answer the question — its storage is synchronous all the way down. It is a **barrier, not a second writer**: it waits for everything written so far to reach the disk and rejects with the write's own error if it does not. Starting its own write instead would race the automatic one through the same temporary file, with nothing ordering them — and the older revision could land last and undo the newer.

The cache and its wait queue are bounded: one storage file may serialize to at most 8 MiB, contain at most 4,096 keys, and hold at most 1 MiB in any one value. At most 1,024 unresolved `flush()` barriers may wait at once; another is rejected instead of growing an unbounded waiter list.

### Where storage lives

Storage is per application, and the host chooses the location — an application cannot name its own, or two applications could collide on purpose.

**The host names the application, and its data follows that name:**

```rust
let data = gpui_kit::shell::set_bundle_id("com.example.notes")?;
gpui_kit::shell::set_capabilities(Capabilities::new().write_roots([data]));
```

| Platform             | Location                                                                         |
| -------------------- | -------------------------------------------------------------------------------- |
| Linux and other Unix | `$XDG_DATA_HOME/gpui-shell/apps/<id>/store.json`, defaulting to `~/.local/share` |
| macOS                | `~/Library/Application Support/gpui-shell/apps/<id>/store.json`                  |
| Windows              | `%APPDATA%\gpui-shell\apps\<id>\store.json`                                      |

The id is the identity, so the data survives the directory being renamed, moved, or replaced by an upgrade — which is what a user means by "my settings". Keying on the path instead means an upgrade silently starts them over.

**The runtime does not go looking for the id in a file.** Only the layer that installed the application knows what it is called; a runtime that read it out of a manifest of its own choosing would be claiming authority over something it does not own.

A host that was merely _pointed at_ a directory — this command line, a dev server — has no such name, and there the path really is the identity. `gpui_kit::shell::bundle_id_for_path(root)` builds one from the directory's name and a digest of its full path, so the same directory always reaches the same data and two checkouts of one source stay apart. That is right while you are editing something and wrong once it is installed, which is exactly the difference declaring a real id makes.

The id may hold `a-z`, `0-9`, `.`, `-` and `_`, and no `..`. That is not tidiness: it is joined onto the user's data directory, so an unchecked one reaches the rest of it. Data lives there rather than inside the application because an application directory may be read-only, is often a git checkout, and is not where a user expects their data to be.

### Degrading when it is not granted

`localStorage` that has not been granted throws, and a well-written application treats that as a fact about its host rather than an error:

```js
// storage.js — from the bundled example
export function load() {
  try {
    const saved = localStorage.getItem(KEY);
    if (saved === null) return [];
    const items = JSON.parse(saved);
    return Array.isArray(items) ? items : [];
  } catch (error) {
    console.warn(
      `todolist: storage unavailable, starting empty (${error.message})`,
    );
    return [];
  }
}
```

The example's footer then says so on screen — "Not saved — this host did not grant storage, so the list lasts for this run only" — which is the right shape: absorb the refusal at the boundary, and tell the user the truth.

## The clipboard

```js
cx.write_to_clipboard("copied");
const text = cx.read_from_clipboard(); // undefined when the clipboard holds no text
```

Named after `App::write_to_clipboard` and `App::read_from_clipboard`, and on `cx` because that is where GPUI keeps them. Nothing to import.

Read and write are **separate grants**, and a denial names the half that is missing:

```text
writing the clipboard is not granted; declare capabilities.clipboard.write in the manifest
```

The clipboard needs a live host call — GPUI's `App` only exists for the duration of one — so a `cx` with none reports that plainly instead of panicking:

```text
cx.read_from_clipboard() needs a live host call; call it from render, an event handler or a task
```

## `console`

```js
console.info("loaded", count, { source: "disk" });
console.warn("could not save");
```

`debug`, `log`, `info`, `warn` and `error`. A global, as it is in every other JavaScript runtime, and nothing to import — the shell used to export the same object a second time as `gpui.log`, which bought a name and nothing else.

**No capability is required**: a script that can run can already say something, and denying it would cost the author their diagnostics and nothing else.

Extra arguments are appended space-separated, the way `console.log` behaves. Structured values print as JSON, because that is what an author reading a log wants to see.

Output goes through `tracing` with the target `gpui_kit::shell::script`, so script output is separable from host output in a log filter. **A host with no `tracing` subscriber installed discards all of it** — along with the runtime's own reports of throwing handlers, unhandled rejections and illegal-phase calls. The `gpui-shell` binary installs a stderr sink at `INFO`, or `DEBUG` under `--dev`.

## `process`

```js
import process from "process"; // also available as a bare global

const { code, stdout, stderr } = await process.run("git", ["status"]);
process.exit(0);
```

`process.run` returns a promise, for a sharper version of the reason `fs` does. A file read has no bound; a child process has less — it can compute for minutes, wait on input that never comes, or outlive the window. Waiting for one on this thread would stop the frame and the VM together, in the kernel, where the interrupt budget cannot see it.

Output is **captured, not inherited**: a script that runs a command almost always wants what it said, and in a windowed application a child writing to the host's stdout is writing somewhere no user will look. `code` is `0` on success and `-1` when a signal killed it, which has no exit code of its own.

Execution is bounded: 30 seconds, 8 MiB of stdout and 8 MiB of stderr. Reaching a bound kills and reaps the child and rejects the promise. Cancelling owned work or tearing down its runtime also terminates the child. The child starts with a cleared environment rather than inheriting host secrets; the shell does not expose an option to add environment variables.

It is gated on an execute grant, which is one of three: denied (the default), an allowlist of command names, or unrestricted. A denied command **throws at the call** rather than rejecting, like a denied `fs` path — a rejected promise nobody awaited is a denial nobody sees.

`process.exit` is **a request, never `exit(2)`** inside the runtime. It hands the code to a handler the host installed, which decides what to do — close the plugin's panel, close the window, end the process. One plugin must not be able to take the host process down, and the host may have unsaved state.

The handler is not optional: a host that grants the capability without installing one makes the call **fail**, naming the omission. A request nobody answers is worse than a denial, because a script cannot tell the two apart. The `gpui-shell` binary installs the policy that suits a host which _is_ the process — it ends it, with the code the script asked for.

The name is a deliberate collision. `process` is what a JavaScript author — or a model generating JavaScript — reaches for, so the runtime puts its own capability-gated surface there rather than leaving the name free to look like Node's and behave differently.

`process.exit` has its own `capabilities.process.exit` grant. Filesystem access never implies permission to close a panel, window, or process.

## The sandbox

Beyond the capability grants, the runtime trims the language itself. All of it applies **unless development mode is on**.

**No dynamic code.** `globalThis.eval` is deleted outright — a `ReferenceError` cannot be mistaken for a working `eval` by feature detection, which a throwing stub could be. All four function compilers are replaced: `Function`, and the constructors reachable through `(async function(){}).constructor`, `(function*(){}).constructor` and the async-generator equivalent. `Function` is _replaced_ rather than deleted, keeping the real `Function.prototype`, so `x instanceof Function` and `.call` / `.apply` / `.bind` keep working and only construction throws.

**Frozen built-in prototypes.** `Object`, `Array`, `Function`, `String` and `Number` prototypes are frozen. One VM will host several plugins, which makes those prototypes shared mutable state: one plugin adding an enumerable property to `Object.prototype` changes `for...in` for every other plugin and for the runtime's own prelude. The cost is real — a library that patches `Array.prototype` stops working, at import time — so a host that knowingly runs one can turn the freeze off and keep every other part of the sandbox.

**Module resolution is confined to the application root.** `import "./ui.js"` resolves relative to the importing file; anything that resolves outside the application directory is refused. Dynamic `import()` stays callable on purpose — it is how lazy loading will work — and is confined by the same resolver.

**Resource limits**, so a runaway script reports rather than taking the window with it:

| Limit                                                         | Value                                                                      |
| ------------------------------------------------------------- | -------------------------------------------------------------------------- |
| Heap                                                          | 256 MiB — a leak becomes a catchable JavaScript exception, not an OOM kill |
| Interpreter stack                                             | 1 MiB — deep recursion becomes a `RangeError`, not a native stack overflow |
| Loaded JavaScript module                                      | 8 MiB per source file                                                      |
| Outstanding host tasks                                        | 1,024 per runtime                                                          |
| Time in one call: render and layout                           | 50 ms                                                                      |
| Time in one call: event and task                              | 500 ms                                                                     |
| Time in one call: outside any call, such as module evaluation | 5 s                                                                        |

The clock restarts on every host call, which is what lets the render path have a tighter budget than an event handler. **The interrupt cannot be swallowed by a `catch` block** — that is measured by a test, because if it could be, the interrupt would not be a defence at all. Each WebSocket also has an 8-command queue shared by `read`, `write`, and `close`; when full, a new operation rejects and tells the caller to wait for outstanding work.

There is no quickjs-libc `std`: quickjs-libc is not compiled into the build. The runtime does provide the small audited `os` module listed below.

::: tip Development mode
`--dev` enables source watching and calls `gpui_kit::shell::set_development_mode(true)` before constructing the runtime. That restores dynamic-code constructors and leaves built-in prototypes writable.

Development mode never relaxes capability gating. It makes the language easier to poke at; it does not hand out access nobody declared, because a grant the author never wrote down is a grant that will be missing in production.
:::

## Network and safe standard APIs

Global `fetch(url, options?)` is promise-based and returns `{ status, ok, url, , json() }`. Its grant is narrower than raw networking: every request and redirect must match a declared HTTP host, method, and exact path or path prefix; HTTPS never downgrades to HTTP, and authorization or caller-supplied headers never cross origins.

`net.connect(host, port)` and the named `WebSocket.connect(url, { headers? })` export from `websocket` use `capabilities.network.hosts`. `WebSocket` is not installed as a browser global and is not a constructor. Raw TCP `read()` returns a `Uint8Array`, or `null` at EOF, so transport chunks never undergo lossy text decoding. WebSockets support text and `Uint8Array` messages and serialize writes through one actor. They do not follow redirects. Connect, handshake, and write operations have a 30-second timeout. A socket permits one outstanding `read()` at a time; a second is rejected immediately instead of competing for the next message. Credential and handshake-control headers are refused. Raw TCP and WebSocket access are intentionally broader than an HTTP request grant.

DNS resolution is a bounded process-wide service: all applications share two resolver workers and a 64-request queue. Queueing observes each connection's existing deadline, so saturation fails as a timeout instead of growing memory or threads without limit. This is resource containment, not per-application quality-of-service; a host that runs mutually untrusted applications in one process does not get DNS fairness between them.

The runtime also provides `buffer`, `path`, `url`, `crypto`, `zlib`, `console`, `process`, and `os`. These are the audited LLRT/host-backed subset declared in generated `gpui-kit.d.ts`; `node:` aliases and arbitrary Node built-ins are not part of the shell contract.

## Not there yet

- **Prompting the user.** Grants are decided before the application loads; nothing asks at the moment of use.

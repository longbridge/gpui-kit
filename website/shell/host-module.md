---
title: HostModule
description: How a host lends its own Rust to a script — registration, the import that reaches it, the plain-data boundary, and the rules a Host function runs under.
order: 12
---

# HostModule

[Capabilities](./capabilities.md) is the half that says what a script may **not** reach. This is the other half: what the host chooses to hand it.

A script cannot load a native extension. `dlopen`-ed Rust has no stable ABI, and once it is inside the process it holds every permission the process holds — a sandbox that permits that does not mean anything. So the direction is reversed. **The host registers, at compile time, the Rust it is willing to expose**, and a script reaches exactly that and nothing else.

```rust
use gpui_kit::shell::{HostModule, HostValue};

gpui_kit::shell::export_module(
    HostModule::new("workspace")
        .function("project_name", |_| Ok(HostValue::from("gpui-component")))
        .function("version", |_| Ok(HostValue::from("0.1.0"))),
)?;
```

```js
import { project_name } from "workspace";

project_name(); // "gpui-component"
```

A registered module is an ordinary ES module, resolved by the same loader that answers `gpui-kit` and `path`. One call registers one module, and a repeated name replaces the earlier module rather than merging into it — a host with three of them calls `export_module` three times. The rest of this page is what that costs and what it refuses.

## Why an import rather than a lookup

The obvious alternative is a runtime registry lookup answering with a bag of functions:

```js
// The shape this does not have.
const workspace = native("workspace");
workspace.projectName(); // typo: throws, eventually
```

```js
// The shape it has.
import { projectName } from "workspace"; // typo: fails to link
```

It loses twice, and both times on _when_ you find out:

- **A misspelled export would be a run-time failure.** `workspace.projectName()` type-checks, loads, renders, and then throws on the frame that first reaches it — which, for a name only one branch touches, can be a long way from the edit that caused it. An import is resolved when the module graph is linked, so the same typo stops the application before its first line runs, naming the module and the export.
- **The type declarations would have nothing to say.** Only the Host knows what it registered, so a lookup could offer no better than `Record<string, (...args: any[]) => any>` — leaving an application that wants real types to hand-write a `.d.ts` that nothing checks against the registry. A module specifier is a name declarations _can_ be written against, so they are [generated from the registry itself](#typing-them) and the typo is red in the editor.

What the import does **not** freeze is the function behind the name. Every export is a forwarding stub that resolves through the registry on each call, so withdrawing a module still takes effect immediately: a script holding an imported function gets a refusal, not the withdrawn closure. Only the _set of names_ is fixed, at the moment the importing module is linked — which is why a host calls `export_module` **before** it loads an application.

## The registry is the grant

The default registry is **empty**, the same shape as `Capabilities::default()`. A host that registers nothing has granted no extension surface, and a script that imports a module is told so by name:

```text
HostModule `market` is not available: this Host registered none.
HostModule access is granted by the embedding application, with
gpui_kit::shell::export_module(...).
```

Register something and the message changes to name what does exist:

```text
unknown HostModule `marker`; this Host registered: market, theme
```

```text
HostModule `market` has no function `quote`; it provides: quotes, ticks, watch, watch_all
```

There is deliberately no per-module capability to grant on top of this. The host chose the list, so **the list is the grant** — and revoking one is a matter of exporting a module of the same name, or clearing the set, which takes effect on the next call rather than the next restart.

For a multi-application host, each public `Policy` carries its own frozen capabilities and its own module registry — built with `Policy::with_host_module`, one module at a time, the same way. That is how two plugins in one runtime receive different authority without swapping thread-local state across `await` boundaries. Identity and requested system permissions live in `gpui-shell.json`; HostModule registrations do not, because contributions are executable behavior registered by the host.

## Names the runtime keeps

A HostModule shares one specifier namespace with the built-in modules and the [Standard Runtime](./engine.md), and the resolver reaches those first. So registering `path` would not shadow the real `path` — it would register a module nothing can ever import, silently.

`export_module` refuses such a name instead, and says who owns it:

```text
`path` is one of the runtime's own module names and cannot be registered: a
script importing it reaches the runtime, never this module. The reserved names
are: gpui, gpui-base, gpui-fps, buffer, console, crypto, fs/promises, net, os,
path, process, url, websocket, zlib
```

The full list is `gpui_kit::shell::RESERVED_SPECIFIERS`. Everything else is yours — and cannot be shadowed by a file in the application directory either, because HostModule registrations resolve before the application's own files.

## The boundary is plain data

A Host function receives `HostArguments` and returns a `HostValue`: null, boolean, number, string, array, or object. Those six cases are the intersection of what a script engine and JSON can both carry, which is what lets one registry serve any engine behind the [seam](./engine.md).

It never receives a script handle. A handle would let the host keep a reference to a script value past the call that produced it — and past the call scope that made the surrounding context valid.

Arguments come out by position, with the type check and the error message included:

| Call                   | Yields                                                               |
| ---------------------- | -------------------------------------------------------------------- |
| `arguments.string(0)`  | `&str`, or an error naming what arrived instead                      |
| `arguments.number(0)`  | `f64`                                                                |
| `arguments.integer(0)` | `i64`, refusing a fractional number                                  |
| `arguments.boolean(0)` | `bool`                                                               |
| `arguments.value(0)`   | The raw `HostValue`, for a function that accepts more than one shape |
| `arguments.get(0)`     | `Option<&HostValue>`, for an optional argument                       |

Returning a record is a builder rather than a map, because an object frequently _is_ the row a script renders and insertion order should be the host's to decide:

```rust
use gpui_kit::shell::HostObject;

HostObject::new()
    .field("symbol", "AAPL.US")
    .field("last", 224.22)
    .field("watched", true)
```

An error is a message, not a type: `HostError::new("no such symbol")` reaches the script as a thrown `Error` the script can catch.

## Three rules a Host function runs under

**It must not call back into the script engine.** A host call happens inside a script call, which is inside a host call; re-entering the VM from there would run script code with an engine frame already on the stack, in the middle of a render pass. Holding no script handle makes that hard to express by accident, and the dispatcher refuses a nested call outright so a host that finds another route gets a diagnosable error rather than undefined behavior.

**Reading and writing host state is the point.** A function reaches the ambient `App` through `gpui_kit::shell::with_current_app`, which is `None` outside a live call:

```rust
fn with_app<R>(read: impl FnOnce(&mut App) -> R) -> Result<R, HostError> {
    gpui_kit::shell::with_current_app(read)
        .ok_or_else(|| HostError::new("only reachable while a script call is in progress"))
}
```

**`cx.notify()` from inside one is delivered after the call unwinds.** So a Host function may mutate an entity and ask the Views watching it to re-render, without that re-render happening underneath the script that called it.

## Work that should not hold the thread

`function` is synchronous: it returns a value, and the script gets that value. A slow one holds the thread that renders.

`async_function` returns a future instead, and the script gets a promise:

```rust
HostModule::new("db")
    .declarations("export function query(sql: string): Promise<Row[]>;")
    .async_function("query", |arguments| {
        // Synchronous half: on the main thread, inside the caller's scope. It
        // may read host state, and refusing here throws at the call site.
        let sql = arguments.string(0)?.to_owned();
        let pool = with_app(|cx| cx.global::<Pool>().handle())?;

        // Asynchronous half: on GPUI's background executor.
        Ok(async move { Ok(pool.query(&sql).await?.into_host_value()) })
    })
```

```js
import { query } from "db";

const rows = await query("select 1");
```

### The split is the design

The closure runs on the main thread and returns the future. So the arguments are checked, and whatever the work needs is copied out, while `with_current_app` still answers. The future is then `Send + 'static` and driven elsewhere, where there is no `App` and no script engine to reach for.

That is the same rule as [the three above](#three-rules-a-host-function-runs-under), made physical rather than enforced. A synchronous body is held to "do not re-enter the engine" by a run-time guard; an asynchronous one cannot express the violation, because on a background thread there is nothing to re-enter.

### What the script sees

- **A refusal from the synchronous half throws at the call site.** `arguments.string(0)?` failing is a `TypeError` where the call was written, not a rejected promise the script has to await to hear about.
- **A failure from the future rejects the promise**, carrying `module.function` in the message, so `try`/`catch` around the `await` works normally.
- **A cancelled call stays pending for ever.** If the View goes away or its application is reloaded, the continuation never runs and no error is invented for code that was asked to stop — the same answer `cx.sleep` gives.

Declare the return type as a `Promise` yourself. The registry checks that the names on both sides agree; it does not read signatures, so nothing catches a declaration that leaves the `Promise` off.

## Typing them

A module describes its own TypeScript face, in Rust, beside the registration:

```rust
HostModule::new("market")
    .declarations(r#"
        /** One row of the board, as it crosses the boundary. */
        export interface Quote { symbol: string; last: string; watched: boolean }

        /** Every row on the board. */
        export function quotes(): Quote[];
        /** Flips one row's watched flag and answers the new value. */
        export function watch(symbol: string): boolean;
    "#)
    .function("quotes", /* … */)
    .function("watch", /* … */)
```

The generated `gpui-kit.d.ts` emits that verbatim inside `declare module "market"`, so `import { quotes } from "market"` is checked exactly the way `import { div } from "gpui-kit"` is.

Writing it here rather than in a `.d.ts` beside the script is what keeps the two halves one thing. A `.d.ts` would be a second file, in a second language, with nothing holding it to the registry. `export_module` compares the declared exports with the registered ones and refuses a mismatch:

```text
HostModule `market` declares a different set of functions than it registers;
registered but not declared: quotes; declared but not registered: prices
```

Renaming a function on one side is now a sentence at start-up rather than an editor that keeps completing a function the host deleted.

Declaring nothing is allowed and costs only precision. An undeclared module is emitted with permissive signatures:

```ts
declare module "audit" {
  import { HostValue } from "gpui-kit";

  export function observe(...args: HostValue[]): HostValue;
}
```

which still checks the module name and every export name — and is honest about the shape, since `HostValue` is exactly what crosses the boundary. `any` would be wider than the runtime: a script passing a function would type-check and then be refused at the call.

## A real one

The gallery's Shell story registers one market module, and it is the entire extension surface its script has. Theme values come from `cx.theme()` instead. This is the host side:

```rust
fn market_module(market: &Entity<Market>) -> HostModule {
    let read = market.clone();
    let flip = market.clone();

    HostModule::new("market")
        .declarations(MARKET_TYPES)
        .function("quotes", move |_| with_app(|cx| read.read(cx).to_host_value()))
        .function("watch", move |arguments| {
            let symbol = arguments.string(0)?;
            with_app(|cx| {
                flip.update(cx, |market, cx| {
                    let watched = market.watch(&symbol)?;
                    // Delivered after this call unwinds, so it cannot re-enter
                    // the engine: the story and the script view re-render together.
                    cx.notify();
                    Ok(HostValue::from(watched))
                })
            })?
        })
}

gpui_kit::shell::export_module(market_module(&market))?;
```

And this is the script that uses it — the same `Market` entity a Rust panel beside it is rendering from:

```js
import { quotes, watch } from "market";

const rows = quotes();
const watched = rows.filter((quote) => quote.watched).length;
```

Run it with `cargo run -- shell`. The two panels read one entity through two paths, which is what makes a mismatch between them visible immediately.

## Not there yet

- **Classes and object identity.** A module exports functions. Exporting a class would mean handing the script a live host object, which the plain-data boundary above rules out; a factory function returning a record does the same work today.
- **Per-function grants inside one registry.** A policy grants the registry the host assembled; it does not add another permission switch for each function.
- **Streaming or callbacks into the host.** A script cannot hand a function to a HostModule; the module can only be called.

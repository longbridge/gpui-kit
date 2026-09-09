---
title: Dependencies
description: Shell packages — what makes a Git repository one, and how a manifest names, selects, fetches and imports it, down to what an editor sees.
order: 9
---

# Dependencies

An application imports its own files by relative path. Every other import it writes comes from one of two places: a **built-in module** the runtime provides — `gpui-kit`, `gpui-base`, `gpui-shell`, `gpui-fps`, and the standard runtime's `fs/promises`, `path`, `crypto`, `net`, `websocket` — or a **dependency**, a JavaScript package the manifest declares and gpui-shell fetches from Git before the entry module is evaluated.

There is no registry, no package manager and no install step. A dependency is a Git remote, a ref, and the name a script imports it by.

## Shell package

A dependency is any Git repository the manifest points at. `omarchy-ui` is a particular kind of one, and that kind has a name: **a shell package** — a JavaScript package written for gpui-shell rather than for Node or a browser, the way a crate is written for Cargo. Five things make a repository one:

| A shell package                                                      | Because                                                                               |
| -------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| Ships ES modules, and needs no build step                            | The runtime evaluates the checkout's files as they are, and `require` does not exist  |
| Has a root `package.json` with `"type": "module"` and a `main`       | It makes the declaration one line, and names the entry for the runtime and the editor |
| Imports only the built-in modules and its own files                  | Nothing else resolves — it cannot reach the application that imported it              |
| Treats `gpui-kit` and `gpui-base` as provided, never as dependencies | They come from the runtime that loaded it, at the version the host chose              |
| Declares no capabilities of its own                                  | Its `fs` and `fetch` calls run under the consuming application's grants               |

Nothing reads the name: a dependency is recognized by being declared, not by being labelled. It is what one author writes so another can find the package, and `gpui-shell` is the repository topic that spells it for a search engine.

[`omarchy-ui`](https://github.com/huacnlee/omarchy-ui) is one, and it is the example on the rest of this page.

## Declaring a dependency

`omarchy-ui` is a package of presentation components and theme utilities. One line in `gpui-shell.json` adds it:

```json
{
  "id": "com.example.projects",
  "name": "Projects",
  "entry": "main.js",
  "dependencies": {
    "omarchy-ui": "huacnlee/omarchy-ui"
  }
}
```

The map key is the bare module name — nothing inside the package chooses it. The manifest names the package the way an `as` clause names an import, so two applications may reach the same remote under different names, and renaming a repository does not rename the import:

```js
import {
  AppShell,
  Button,
  CenteredWorkspace,
  MutedText,
  PageColumn,
  Surface,
  Title,
} from "omarchy-ui";

export function render(cx) {
  const card = new Surface()
    .children([
      new Title("Projects").build(cx),
      new MutedText("Choose a project to continue").build(cx),
      new Button("project-create")
        .label("Create project…")
        .onClick((_event, context) => context.notify())
        .build(cx),
    ])
    .build(cx);

  const page = new PageColumn("projects-page").child(card).build(cx);
  return new AppShell()
    .content(
      new CenteredWorkspace("projects-workspace").content(page).build(cx),
    )
    .build(cx);
}
```

Nothing else changes. The script is still the script from [Getting Started](./getting-started.md); the dependency only widens what it may import.

## What resolves, and to what

| Written                                                  | Resolves to                                                                |
| -------------------------------------------------------- | -------------------------------------------------------------------------- |
| `"omarchy-ui"`                                           | The package entry — see [Package entry](#package-entry)                    |
| `"omarchy-ui/src/style"`                                 | That file inside the checkout; the `.js` extension is optional             |
| `"./theme.js"` inside a package                          | A file inside that package's own checkout                                  |
| `"gpui-kit"` inside a package                            | The built-in module, exactly as in application code                        |
| Another declared dependency name                         | The other package's entry — declared packages can see one another          |
| An application file, by bare name, from inside a package | Refused: a package cannot reach back into the application that imported it |

A specifier that resolves outside the checkout it started in is refused before the module is loaded, so `../` cannot walk out of a package and into the cache beside it. Inside the application directory the same boundary is the application root, which is the rule [the sandbox](./capabilities.md#the-sandbox) already applies to relative imports.

**A dependency is not a second sandbox.** It is evaluated in the application's own context and holds exactly the grants the manifest holds: a package that reads a file is reading it under your `fs.read` scope. Declaring a dependency is trusting its code the way importing a Rust crate is, and the ref you pin is what decides which code that is.

## Selecting a version

The string form is a strict GitHub shorthand or a full Git URL, each with an optional `#ref`:

```json
{
  "dependencies": {
    "default-main": "huacnlee/omarchy-ui",
    "named-ref": "huacnlee/omarchy-ui#v1.2.0",
    "commit": "https://github.com/huacnlee/omarchy-ui#0123456789abcdef0123456789abcdef01234567",
    "remote-head": "https://github.com/huacnlee/omarchy-ui"
  }
}
```

| Form                       | Selects                        |
| -------------------------- | ------------------------------ |
| `owner/repository`         | `main`                         |
| `owner/repository#ref`     | That branch, tag or commit-ish |
| `https://…/repository`     | The remote's `HEAD`            |
| `https://…/repository#ref` | That branch, tag or commit-ish |

Shorthand is deliberately strict: exactly one `owner/repository` pair of alphanumerics, `.`, `-` and `_`, at most one `#`, no surrounding whitespace, and a fragment that is a valid Git ref name. Anything else is a manifest error rather than a URL guessed from a typo. A full URL may be any Git transport, `ssh://` and `git@host:owner/repo` included.

**A branch, a tag or a remote `HEAD` is re-fetched and re-resolved on every application load; a commit ID always selects that commit.** Depending on a branch means the code changes under you the next time the window opens, which is convenient while you develop a package and a supply-chain decision once you ship one. Pin a tag or a commit for anything you do not control.

Fetching needs `git` on the host's `PATH`, and it happens before script capabilities exist — it is gpui-shell running Git on the application's behalf, not the script reaching the network, so it is not covered by `capabilities.network` and does not need `fs.execute`. With nothing to fetch (the cache already holds the commit) a load performs no network access at all; with a moving ref and no network, the fetch fails and the application does not load.

## Package entry

After the checkout exists, gpui-shell reads the package's root `package.json` and takes a string `main` as the entry. `omarchy-ui` publishes:

```json
{
  "type": "module",
  "main": "src/index.js",
  "types": "src/index.d.ts"
}
```

So `import { Title } from "omarchy-ui"` evaluates `src/index.js`. When `package.json` is missing, or has no `main`, the entry is the root `index.js`.

The runtime reads `main`; an editor reads `types`. Both come out of the same file, which is why a package that ships `.d.ts` files needs nothing from the application to be fully typed at the call site.

Malformed JSON, a non-string `main`, and an entry that is absent, not a file, or escapes the checkout all fail the load before any application JavaScript runs.

## The object form

The original object form remains supported unchanged. It requires exactly one explicit `branch` or `tag`, and its repository-relative `entry` defaults to `index.js`:

```json
{
  "dependencies": {
    "omarchy-ui": {
      "git": "https://github.com/huacnlee/omarchy-ui",
      "tag": "v1.2.0",
      "entry": "src/index.js"
    }
  }
}
```

Existing manifests do not need to migrate. Moving to the string form means the package publishes its entry itself, through `package.json` `main` or a root `index.js`, instead of every consumer repeating it.

## The cache

```text
~/.gpui-shell/cache/dependencies/
├── locks/<remote>.lock
├── mirrors/<remote>.git
└── checkouts/<remote>/<commit>/
```

`<remote>` is a SHA-256 of the exact fragment-free URL, which is both the remote's identity and its cache identity. A per-remote lock serializes mirror updates. Checkouts are commit-addressed and never rewritten, so concurrent launches and an older hot-reload generation each keep reading the tree they started with.

The mirror's configured origin is verified against the manifest on every use, and it is the raw configured value that is compared — Git's `url.*.insteadOf` may still choose a different effective fetch URL, which is how a mirror or an internal host substitution keeps working. Git runs non-interactively, with credential prompts disabled and a 30-second limit per command, so a repository that wants a password fails with a message instead of hanging a window that is waiting to open.

Nothing prunes this cache automatically. It is content-addressed, so deleting it is safe: the next load re-fetches what it needs.

## What an editor sees

The runtime answers `import { Title } from "omarchy-ui"` from the manifest. An editor answers it by walking `node_modules` up from the importing file, and it has never heard of `gpui-shell.json`. Left alone, a correct import is underlined as a module that cannot be found, and every name behind it loses its type, its parameter hints and its documentation.

So every load — and `gpui-shell types` — links each materialized checkout into the application's `node_modules` under the name the manifest gave it:

```text
projects/
├── gpui-shell.json
├── main.js
├── gpui-kit.d.ts          generated by the runtime — ignore it
├── jsconfig.json      scaffolded once, then yours
└── node_modules/
    └── omarchy-ui  →  ~/.gpui-shell/cache/dependencies/checkouts/<remote>/<commit>
```

The editor then reads the same files the runtime is about to execute, so the signatures and JSDoc it shows are the package's own and cannot drift from what runs.

Only entries gpui-shell wrote are ever replaced or removed — a symlink into its own dependency cache, or a directory carrying its marker file. An installed package of the same name is left alone, and the link of a dependency the manifest no longer declares goes away. Where the platform refuses a symlink, such as an unprivileged Windows process without developer mode, gpui-shell writes a small package that re-exports the checkout instead: a bare import types the same way, and only a package-subpath import is left unresolved.

A `jsconfig.json` is scaffolded when the directory has neither that nor a `tsconfig.json`, and it is written once — an existing configuration is never replaced. It is not decoration. An inferred `moduleResolution` can land on the one that never looks in `node_modules`, which underlines a dependency the runtime resolves fine; and the default `lib` hands a script the browser's globals, whose declarations collide with the ones `gpui-kit.d.ts` makes, so the file describing the API is itself reported as the error.

`node_modules` is generated, like `gpui-kit.d.ts`. Ignore both:

```text
gpui-kit.d.ts
node_modules/
```

The directory is called `node_modules` because that is the one place every editor looks; no package manager is involved and nothing comes from a registry. It also buys quiet: TypeScript treats what it resolves there as an external library, so a dependency's own implicit-`any` diagnostics stay out of your own.

## When fetching and linking happen

| Invocation                                      | Fetches and links | On failure                               |
| ----------------------------------------------- | ----------------- | ---------------------------------------- |
| `gpui-shell <directory>`                        | Yes               | Load fails; linking alone is best-effort |
| `gpui-shell check <directory>`                  | Yes               | Reported as a check failure              |
| `gpui-shell types <directory>`                  | Yes               | Reported, with an exit status            |
| An embedded host's `ShellRuntime::load`         | Yes               | Load fails; linking alone is best-effort |
| `gpui_kit::shell::write_dependency_links(root)` | Yes               | Returned as an error to the caller       |

Fetching is what a load depends on, so a dependency that cannot be materialized fails the load. Writing the editor links is not: a read-only application directory is a reason to lose editor types, not a reason to refuse to run. `gpui-shell types` exists for exactly the case where that difference matters — it does the same work and reports what it could not do.

Hot-reload picks up a package the same way it picks up an application file: each load is a new module generation, so restarting the application is enough to move a branch dependency forward.

## What can fail

Every one of these is reported before the application's JavaScript is evaluated:

| Message                                                    | Cause                                                             |
| ---------------------------------------------------------- | ----------------------------------------------------------------- |
| `GitHub shorthand must contain exactly owner/repository …` | A shorthand with a path, a scheme, or an invalid character        |
| `a string dependency #Git ref must not be empty`           | A trailing `#`                                                    |
| `could not clone Git dependency …`                         | Git failed: no such remote, no credentials, no network            |
| `git timed out after 30 seconds …`                         | A hung fetch, usually an interactive credential prompt            |
| `Git dependency … cache origin is …, expected …`           | Two manifests disagree about one cache entry; remove it and retry |
| `Git dependency … package.json main must be a string`      | A `main` that is an object, or a path escaping the checkout       |
| `Git dependency … has no entry …`                          | `main`, or an object form's `entry`, names nothing                |
| `cannot resolve module … from …`                           | A subpath import with no such file, or one leaving the checkout   |

## Publishing a shell package

A shell package is a plain Git repository; `omarchy-ui` has no build output, no lockfile and no publish step. Beyond the five things that make it one, what makes it comfortable to depend on:

- **A root `package.json` with `main`**, so consumers write one line and no `entry`. `"type": "module"` alongside it keeps the editor and the runtime agreeing that the source is ES modules.
- **A single entry that re-exports the public surface.** `src/index.js` decides what a consumer can name; anything else in the checkout stays reachable by subpath, which is a useful escape hatch and a poor public API.
- **Types beside the source**, through `types` in `package.json`. Generated `.d.ts` files or JSDoc both work, and both arrive at the call site through the link — a consumer's `jsconfig.json` needs no `paths` entry.
- **Tags for releases**, so consumers can pin `#v1.2.0` instead of tracking `main`.
- **A `gpui-shell` topic on the repository**, so someone looking for a shell package can find it.

## Read next

| Page                                    | What it covers                                                            |
| --------------------------------------- | ------------------------------------------------------------------------- |
| [Capabilities](./capabilities.md)       | The rest of the manifest: identity, versions, and what a script may reach |
| [Getting Started](./getting-started.md) | `gpui-shell types`, `check`, and the declarations a dependency joins      |
| [API Reference](./api.md)               | The built-in modules a package imports alongside yours                    |

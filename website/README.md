# GPUI Kit website

To install dependencies:

```bash
bun install
```

To run:

```bash
bun run dev
```

This project was created using `bun init` in bun v1.2.23. [Bun](https://bun.com) is a fast all-in-one JavaScript runtime.

## App Stories data

Clone the reviewed app catalog alongside the GPUI Kit checkout:

```bash
git clone https://github.com/longbridge/gpui-kit-showcases.git ../../gpui-kit-showcases
```

Run this command from `website/`. Alternatively, set `SHOWCASES_DIR` to the
absolute path of an existing Showcase checkout. Builds read its manifests and
pin image URLs to its current commit. Install the catalog’s locked Bun dependencies
with `bun install --frozen-lockfile --cwd ../../gpui-kit-showcases` before building. Commit and push new images before publishing.

`bun run test:showcases` tests catalog validation, grouping, filtering, and sorting.
The release workflow fetches the latest approved catalog automatically; see the
[Showcase contribution guide](https://github.com/longbridge/gpui-kit-showcases/blob/main/CONTRIBUTING.md)
for app submissions and full-window screenshot instructions.

## Writing documentation

The site is published once per version: the latest release at `/`, `main` at
`/versions/main`, and older releases at `/versions/<tag>`. Pages must keep the
reader in the version they are reading.

- **Link to the Markdown file with a relative path**, such as
  `[Entity](./entity.md)` or `[Dialog](../component/dialog.md)`. The build
  resolves it inside the current version. A site-root path such as
  `/docs/entity` points at the default version and is rejected by
  `bun run test:docs`. A Chinese page links the Chinese page when one exists.
- **Write `{{gpui_pre_version}}` for the GPUI snapshot version**, in prose,
  code and links alike, for example
  `https://docs.rs/gpui-pre/{{gpui_pre_version}}/gpui/struct.Window.html`.
  The value comes from the `gpui` entry in the workspace `Cargo.toml`, so a
  snapshot bump updates every page in both locales. `bun run test:docs`
  rejects the pinned version written out by hand; a deliberate reference to
  another snapshot is listed in `tests/doc-sources.test.ts`.
- **Mark maturity in the frontmatter** of a page whose capability is not on
  the stable desktop path: `maturity: [preview]`, `[experimental]`,
  `[showcase-only]`, or `[platform-dependent]`, combined as needed. Unmarked pages are Stable. The
  labels render under the title and link to their definitions on the
  documentation home. Both locales must carry the same value.

`bun run test:links` checks every link in a finished build, and
`bun run test:versioned-examples` repeats that check for a build at
`/versions/test/`, where a link that leaves the version fails.

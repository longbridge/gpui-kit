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

# GPUI Kit Assets

The shared icon names and assets for [GPUI Kit](https://gpui-kit.com), exposed as
`gpui_kit::assets`. The crate depends on GPUI, not on GPUI Component, so Base,
GPUI Component, and alternative presentation layers can use the same `IconName`.
GPUI Component reexports `IconName` and `IconNamed` for existing imports.

The bundle contains all 1,818 Lucide 1.43.0 icons and 12 retained GPUI Kit icons.
`IconName::ALL` enumerates the bundle; `IconName::Search.path()` returns
`icons/search.svg`. The names and paths are generated from the bundled SVGs
at build time, without network access or another crate's source directory.

## Embed only the icons you use

```rust
use gpui_kit::assets::{icon_assets, IconName};

icon_assets!(AppAssets, [Search, Check]);

// Register instead of the full Assets bundle:
let app = gpui_kit::application().with_assets(AppAssets);
```

The macro accepts optional visibility (`icon_assets!(pub AppAssets, [...])`).
Only the selected SVG bytes are referenced by this source, on native and WASM.
Unlisted paths return `Ok(None)`. Loading returns borrowed static bytes without
copying them or creating an icon cache. Rendering still has the normal GPUI
SVG parsing/rasterization costs. Selection is explicit: dynamic path lookup
cannot infer which assets an application will need. Merely using `IconName`
does not reference any SVG bytes, although runtime name lookup may retain the
name-to-path table. The full catalog remains in the downloaded Cargo package
and build artifacts; this controls the final application payload.

## Use the complete bundle

Register `gpui_kit::assets::Assets` to load any bundled icon by path. On native
platforms the complete SVG set is embedded. On WASM, use `Assets::new(endpoint)`;
icons are fetched on demand from `{endpoint}/assets/icons/*.svg`. Deploy the
contents of `assets/icons` at that location. The selected macro source above
requires no CDN on either platform.

`IconName` can be a GPUI child directly. GPUI Component's `Icon::new(name)` adds
component sizing and transformations. To keep calling `name.view(cx)`, import
`gpui_kit::component::IconNameExt`, or use `Icon::new(name).view(cx)`.

## Updating Lucide

`lucide.json` pins the upstream version, archive SHA-256, and canonical icon
count. Update those fields for a new release, then run from the repository root:

```sh
python3 script/sync-lucide.py
python3 script/sync-lucide.py --check
```

Use `--archive /path/to/archive.tar.gz` for offline operation. The script checks
the archive hash, copies every canonical SVG and the upstream license, and
preserves custom/retired filenames for compatibility. Review retained icons
when upgrading. `--check` verifies all upstream files byte for byte.

## License

Crate code: Apache-2.0. Lucide SVGs: ISC, with the upstream Feather-derived icons
under MIT; see [LICENSE-LUCIDE](LICENSE-LUCIDE) for the complete attribution.

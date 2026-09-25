# Icon assets in GPUI Kit applications

An `IconName` identifies an SVG path. The application must also register an
`AssetSource` that can load that path. Depending on `gpui-kit-assets`, or using
an `IconName` in a view, does not make every catalog SVG available at runtime.

The application-facing dependency is `gpui-kit`; its default features already
include `gpui-kit-assets` and GPUI Component:

```toml
[dependencies]
gpui-kit = "0.6"
```

Choose the source that matches the icons the application uses:

| Need | Register on native desktop | Result |
| --- | --- | --- |
| Built-in component icons | `gpui_kit::assets::Assets` | Embeds the 101 default component SVGs. |
| A few additional catalog icons | A composite of `icon_assets!` output and `Assets` | Embeds the selected extras and keeps component icons. |
| The complete catalog | `gpui_kit::assets::AllAssets` | Embeds all catalog SVGs. |
| Your own SVG files | An application `AssetSource` | Loads your paths; compose with `Assets` if components also need their built-in icons. |

For the default icons, register the source before opening windows:

```rust
let app = gpui_kit::application().with_assets(gpui_kit::assets::Assets);
```

For selected extra catalog icons, `icon_assets!` creates a separate source. It
does not change the registered source automatically:

```rust
use gpui_kit::assets::icon_assets;

icon_assets!(ExtraIcons, [Accessibility, AlarmClock]);
```

Register a composite source that checks `ExtraIcons` first, then falls back to
the default `Assets`. The native default source returns an error for unknown
paths, so lookup order matters. See the complete [selected-icons recipe] and
[custom asset source guide] for the `AssetSource` implementation, registration,
and rendering code. Custom SVG filenames do not create new `IconName` variants;
use `Icon::path` with their asset keys.

On WebAssembly, built-in `Assets` and `AllAssets` use an endpoint and load icons
on demand. See the [WebAssembly section] for deployment details.

[selected-icons recipe]: https://gpui-kit.com/docs/assets#pick-additional-catalog-icons-with-icon_assets
[custom asset source guide]: https://gpui-kit.com/docs/assets#add-your-own-asset-files
[WebAssembly section]: https://gpui-kit.com/docs/assets#packaging-and-webassembly

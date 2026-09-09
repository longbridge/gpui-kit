## Icon assets in GPUI Component

The [IconName](https://github.com/longbridge/gpui-kit/blob/6998708b817024c2ac0f1ea164d74ddfc024e124/crates/component/src/icon.rs#L9) is a enum that defined a bunch of icon names, because some internal components in GPUI Component will use them.

You can see, we have a lot of svg icon files in the `assets/icons` folder, but we are not embed all of the icon files in the library by default. This for keep the library size small.

So you must have your own icon files to use the `Icon` component in GPUI Component.

You can download the icon files from [here](https://lucide.dev/) or use your own icon files as you wish, just use the same filename as the icon name (match with the `IconName` defined) you want to use.

For example your assets folder:

```
app_root
  assets
    icons
      close.svg
      menu.svg
      ...
  src
    main.rs
  Cargo.toml
```

You also can just copy the svg files you want from the `assets/icons` folder in GPUI Component repo to your own assets folder.

## How to use

You need define a `Assets` struct with rust-embed to register assets to GPUI application.

```rs
use anyhow::anyhow;
use gpui_kit::*;
use rust_embed::RustEmbed;
use std::borrow::Cow;

#[derive(RustEmbed)]
#[folder = "./assets"]
#[include = "icons/**/*.svg"]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Self::get(path)
            .map(|f| Some(f.data))
            .ok_or_else(|| anyhow!("could not find asset at path \"{path}\""))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(Self::iter()
            .filter_map(|p| p.starts_with(path).then(|| p.into()))
            .collect())
    }
}

fn main() {
    // Call with_assets to register assets
    let app = gpui_kit::application().with_assets(Assets);

    // ...
}
```

## Use default bundled assets.

The `gpui-kit-assets` crate provide a default bundled assets implementation that include all the icon files in the `assets/icons` folder.

If you don't want to manage your own icon files, you can just use the default bundled assets.

Just add `gpui-kit-assets` as a dependency in your `Cargo.toml`:

```toml
[dependencies]
gpui-component = "*"
gpui-kit-assets = "*"
```

And then use it in your application:

```rs
let app = gpui_kit::application().with_assets(gpui_kit::assets::Assets);
```

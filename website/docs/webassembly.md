---
title: WebAssembly
description: Build and run GPUI Kit applications in a browser with the repository's WebAssembly examples.
order: -3.5
---

# WebAssembly

:::warning Current scope
GPUI and GPUI Kit WebAssembly support is used primarily to **showcase and try components in a browser**. The gallery and Base showcase build and run, but this repository has not validated WebAssembly as a mature distribution path for full applications. Treat browser support, accessibility, input, startup cost and deployment as work to verify for each product.
:::

GPUI Kit can render the same Rust views and components in a browser. The web target is `wasm32-unknown-unknown`: Rust produces a WebAssembly module, `wasm-bindgen` produces its JavaScript bindings, and a web page loads and starts the application. The browser supplies the canvas, input and network environment, so a desktop `main` function alone is not a web entry point.

In this workspace, [`gpui_web` is the Cargo alias for `gpui-pre-web` 0.3.6](https://github.com/longbridge/gpui-kit/blob/main/Cargo.toml). [`gpui-kit` includes it as a WASM-only dependency](https://github.com/longbridge/gpui-kit/blob/main/crates/kit/Cargo.toml) and re-exports it as `gpui_kit::web`; [the gallery crate](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/Cargo.toml) depends on `gpui-kit`, not directly on `gpui-pre-web`. Its `cdylib`, exported `run(...)`, web platform initialization and JavaScript loader provide the browser entry path that the desktop `main` cannot provide.

The [component gallery](https://gpui-kit.com/gallery/) is the quickest working example. Its [Rust entry point](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/src/lib.rs), [build script](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/scripts/build-wasm.sh) and [JavaScript loader](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/www/src/main.js) show the complete path from a GPUI Kit view to a browser page.

## Run the gallery locally

From the repository checkout, install Rust and Bun, then run:

```sh
cd crates/story-web
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.121
make dev
```

Open **http://localhost:3000/gallery/**. `make dev` builds the WASM module in debug mode, generates bindings in `www/src/wasm/`, installs the web dependencies and starts Vite. The local [toolchain file](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/rust-toolchain.toml) selects nightly; the `wasm-bindgen-cli` version above matches this checkout's `Cargo.lock`. If the lockfile changes, match the CLI to the locked `wasm-bindgen` crate version. Rust changes require another `make build-wasm-dev` before refreshing the page; the Vite server handles web file changes. The [build script](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/scripts/build-wasm.sh) sets an 8 MiB linker stack for the gallery's large render tree, including in debug builds.

For a production gallery build, run `make build-prod` from the same directory. The site bundle lands in `www/dist/` and uses the `/gallery/` base path. Deploy it at that path, or change both the [Vite base](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/www/vite.config.js) and the [Rust asset endpoint](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/src/lib.rs) for your own host.

## How the web entry point works

The gallery's `run(story, dark, theme_name, theme_json)` is exported with `#[wasm_bindgen]`. Its [loader](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/www/src/main.js) imports the generated JS module, awaits its default WASM initialization, reads the optional `?story=` URL parameter and host theme, then calls `run(...)`. On the Rust side, it calls `gpui_kit::platform::web_init()`, creates a web `Application`, registers assets, and passes a launch closure to `run_embedded`. Once graphics initialization succeeds, that closure calls `gpui_component_story::init(cx)` (which calls `gpui_kit::init(cx)`), loads fonts, applies the theme, and calls `gpui_kit::open_window`. The `ApplicationHandle` returned by `run_embedded` is retained in thread-local storage. Graphics initialization is asynchronous, so that return does not mean the launch closure ran or the first frame painted. For your application, initialize Kit before constructing components, as in [Getting Started](./getting-started.md), and keep the handle alive while the page uses the view.

The gallery uses `WebPlatform::new_with_backend_and_font_fallback` and attaches its fetch HTTP client. `Auto` tries WebGPU and then WebGL2 if WebGPU fails. The current web platform uses one document canvas and one top-level window; another top-level window, or reopening a closed one, is unsupported. Render dialogs inside that window through Kit's `Root`. Reuse the [gallery entry point](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/src/lib.rs) when adapting your own app; keep shared views in Rust, with the page responsible for loading the WASM module and hosting it.

## Fonts and CJK text

The GPUI Web platform starts with **no installed font database**. In the gallery, `include_bytes!` embeds four subset TTF files in the WASM module: Inter for UI text, JetBrains Mono for code, Noto Sans SC for the Chinese characters used by the stories, and IBM Plex Sans for GPUI's `.SystemUIFont` alias. The last family must be loaded before the first window, because even initial text measurement can use the default window style and fail when the family is absent. After loading them with `cx.text_system().add_fonts(...)`, the gallery applies its theme and forces its UI and mono family back to bundled fonts; a selected theme can otherwise name a desktop-only family. See the [font setup and theme code](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/src/lib.rs) and [Fonts](./fonts.md).

The [subset script](https://github.com/longbridge/gpui-kit/blob/main/crates/story-web/scripts/subset-fonts.py) scans the story sources and retains their characters. In this checkout, the Noto Sans SC subset is **25,484 bytes** versus **1,213,236 bytes** for its checked-in source TTF. Those are font file sizes, not the change in compressed WASM transfer size. New text entered by a visitor is not guaranteed to be in that subset.

For eligible missing emoji and horizontal Han, kana and modern Hangul graphemes, `CanvasFontFallback::EmojiAndCjk` can let the browser measure and draw from its local fonts. GPUI's loaded fonts remain preferred. This fallback works on individual graphemes, so browser font coverage, spacing and typography can differ; it is not a full CJK font replacement. The default policy covers emoji only, while `Disabled` uses loaded fonts alone. The policy is chosen when constructing `WebPlatform` and cannot be changed later.

GPUI also supports **loading a font after startup**. In the GPUI version pinned by this repository (`gpui-pre` 0.3.6), `TextSystem::add_fonts` accepts downloaded font bytes through `Cow::Owned`; it clears font resolution and line-layout caches. After an asynchronous fetch has produced a valid raw font file, install it on the application context and redraw:

```rust
use std::borrow::Cow;

// cx: &mut App; font_bytes: Vec<u8> fetched and checked by your app.
cx.text_system().add_fonts(vec![Cow::Owned(font_bytes)])?;
cx.refresh_windows();
```

Use a raw supported font file such as the TTF in GPUI Web's `examples/hello_web/dynamic_fonts.rs`, not a font-service CSS response that may refer to WOFF2 subsets. That example uses `cx.on_missing_glyphs(...) -> Subscription` to request a font when needed, retains the subscription and download task, checks for a successful HTTP status and nonempty body, then calls `add_fonts` and `refresh_windows`. Font parsing is checked by `add_fonts`; an HTTP 200 alone does not establish that the response is a usable font. Missing-glyph reports are deduplicated and bounded; a dropped report does not schedule its own retry. The gallery currently **does not** download fonts this way; it embeds subsets and uses Canvas fallback. GPUI Web applies Canvas fallback before reporting missing glyphs, so CJK it draws successfully through Canvas will **not** trigger `on_missing_glyphs`. For high-quality CJK typography, decide which font to fetch from your content or language selection, show a loading or failure state, and test layout after installation. A font downloaded from another origin must also satisfy that host's browser CORS policy.

## A smaller GPUI Base example

The [GPUI Base WASM showcase](https://github.com/longbridge/gpui-kit/tree/main/crates/base/examples/wasm) compiles the same [showcase views](https://github.com/longbridge/gpui-kit/tree/main/crates/base/examples/showcase) used by its native example. To run it independently:

```sh
cd crates/base/examples/wasm
rustup target add wasm32-unknown-unknown --toolchain nightly
make dev
```

It uses `gpui_platform::single_threaded_web()`, generates bindings with `wasm-bindgen`, and serves the examples at **http://localhost:3001/examples/base/**. Its `run(component)` export selects a showcase view; the JavaScript loader reads `?component=...` from the URL. See its [build script](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/wasm/scripts/build.sh) and [loader](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/wasm/www/src/main.js). The gallery is the better starting point for styled GPUI Kit components; this example isolates the Base primitives.

## Troubleshoot a failed load or blank canvas

Open the browser's **Console and Network** panels first. The gallery loader catches failures while importing the bindings, initializing WASM or calling `run`, and replaces its loading indicator with “Failed to load the application”. Graphics initialization happens later on the web platform's async task; a failure there is logged and a “Failed to initialize browser graphics” message is appended to the page. The loader can therefore disappear before the first frame appears.

| Symptom | Check |
| --- | --- |
| JS or `.wasm` request is 404 | Open `/gallery/`, not the site root. Keep Vite's `base: '/gallery/'`, the deployment path and generated files together. Rebuild with `make build-wasm-dev` after Rust changes; ensure the generated `www/src/wasm/` files were included in the web build. |
| `wasm-bindgen` import or instantiation fails | Match `wasm-bindgen-cli` to the locked crate version and regenerate bindings. Serve `.wasm` as `application/wasm`; the generated loader can fall back from streaming compilation when MIME is wrong, but that is slower. Check the Network response instead of assuming the requested URL returned a module. |
| Loading indicator vanishes but canvas stays empty | Read Console errors after `run` returns. `Auto` tries WebGPU and then WebGL2; if both fail, check browser GPU support, policy and hardware acceleration. The web runtime cannot create a second top-level window. |
| Panic or missing text at startup | Register IBM Plex Sans and every initial theme family before opening the window. The gallery's `add_fonts(...).expect(...)` and `open_window(...).expect(...)` turn errors into a panic; its panic hook reports them in Console. |
| Squares or wrong CJK appearance | Confirm the text is in the bundled subset, then check the Canvas fallback policy and actual browser font coverage. Download a suitable raw font when exact shaping matters. |
| Blank icons | Inspect the exact icon URL, HTTP status and CORS. `Assets::load` concatenates the endpoint and `/assets/icons/...` literally: an endpoint ending in `/`, as in the gallery, produces `//assets/` in the URL. Set an endpoint without a trailing slash in your app. The gallery's fixed endpoint points to the published site even during local development. Its asset loader caches a successful fetch but does not itself request a window refresh, so if the icon remains blank, trigger another render and check the Console for asset errors. |

## Package size and delivery

`include_bytes!` puts the gallery's subset fonts **inside the WASM payload**, so visitors download them with the module before they can see a first frame. The gallery also bundles the component stories and rendering stack; icon SVGs are separate on-demand requests. Vite packages the JavaScript loader and WASM for `/gallery/`, but changing only the Vite base will not change the Rust icon endpoint. After `make build-prod`, `ls -lh www/dist/assets/*.wasm` shows the uncompressed module size. Compare the **compressed transfer sizes** of that module and JS in the browser Network panel, and record the first-load time on a throttled connection. Local file size, CDN compression, browser cache state, compilation time and GPU initialization are different costs.

For example, **if** a full application such as Longbridge Pro were compiled into one WASM module, including all of its features and font coverage, the initial download and startup cost could grow substantially. That is a distribution risk to measure, not a measured package size or a claim that such an application currently ships on the web. Keeping fonts or features behind later requests can reduce initial transfer, but adds network, caching, CORS and loading-state work. The gallery's font subsets and on-demand icons illustrate those tradeoffs; they do not establish an application-size budget.

## Web capabilities to plan for

| Concern | What the examples do | What to check in your app |
| --- | --- | --- |
| Assets | On WASM, `Assets::new(endpoint)` fetches icon SVGs on demand by concatenating `endpoint` and `/assets/icons/...`. The gallery's endpoint points to its published site, while its Vite build copies icons under `/gallery/assets/`. | Host the requested paths and use an endpoint matching your deployment path, without a trailing slash. Local gallery runs still request icons from the published endpoint unless you change it. A missing asset or failed fetch can leave an icon blank. See [Icons & Assets](./assets.md). |
| Fonts | The gallery embeds Inter, JetBrains Mono, a Noto Sans SC subset and IBM Plex Sans before its first frame, then restores its font choices when applying a theme. | Bundle initial families; consider runtime font downloads for larger scripts and validate fallback and layout. See [Fonts](./fonts.md). |
| Keyboard and IME | The web platform uses a small hidden HTML input for keyboard and composition events. The gallery loader manages focus when embedded and disables mobile text entry on touch-only devices to avoid raising a keyboard for a canvas interaction. | Test focus, Tab order, composition and on-screen keyboards in the browsers and devices you support. The gallery's touch-only policy is specific to a showcase, not a general text-input solution. |
| [Accessibility](./accessibility.md) | Components can express roles and labels in GPUI, while this web example is painted into a canvas. | Verify actual screen-reader and keyboard behavior in the browser. Do not assume that a native accessibility bridge or a GPUI accessibility property produces equivalent web semantics. Provide an accessible HTML alternative for content or actions that your target browser cannot expose. |
| Native services | The browser supplies fetch and its own input and rendering APIs; desktop facilities have different availability and permissions. | Put file dialogs, clipboard, notifications and similar features behind target-specific capability code and test the browser path. See [Coding Guides](./coding-guides.md#platform-and-capability-boundaries). |

The release workflow [builds both WASM examples](https://github.com/longbridge/gpui-kit/blob/main/.github/workflows/release-website.yml), so these entry points also serve as maintained build references. A successful WASM build establishes compilation; interaction, font coverage and accessibility still need browser checks.

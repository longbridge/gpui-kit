---
title: Mobile
description: Build an iOS application or embed GPUI Kit in a Swift UIKit container with the experimental gpui-pre-mobile platform.
order: -2.4
---

# Mobile

Mobile support builds on [gpui-mobile](https://github.com/itsbalamurali/gpui-mobile), created by [itsbalamurali](https://github.com/itsbalamurali) and developed with the community. Credit for the original mobile platform belongs to that project and its contributors. The platform supplies the window, touch input, text system, and GPU surface; GPUI and GPUI Kit still own the Rust view tree and components.

GPUI Kit currently uses `gpui-pre-mobile`, a temporary compatibility package maintained in the [Longbridge fork](https://github.com/longbridge/gpui-mobile). It adapts the original project for crate packaging and publication alongside `gpui-pre`, and tracks newer GPUI versions to keep the integration compatible. Once the community `gpui-mobile` completes the integration and GPUI is published as a crate, we plan to switch this guide and its dependencies to the community `gpui-mobile`.

The current integration is experimental. The Swift-hosted iOS example has been built and exercised in the iOS simulator. Android has a platform implementation, but the GPUI Kit integration described here has not been validated on Android or a physical iPhone.

## Run the iOS example

Start with the compatibility fork’s [Swift container example](https://github.com/longbridge/gpui-mobile/tree/0b882efdac7f524e0bb0b1d4c886b2aa752f9f20/example). It includes a conversation UI with `Message`, `Bubble`, `TextView`, `Input`, thought summaries, and copy actions. Its responses are local sample data; it does not connect to an AI service.

On an Apple Silicon Mac, install Xcode with an iOS simulator runtime, Rust, and XcodeGen:

```sh
brew install xcodegen
rustup target add aarch64-apple-ios-sim

git clone https://github.com/longbridge/gpui-mobile.git
cd gpui-mobile
git checkout 0b882efdac7f524e0bb0b1d4c886b2aa752f9f20
cd example
./build.sh ios --simulator
```

The script builds the Rust static library, generates the Xcode project, and installs and launches the app in a simulator. Add `--no-run` to build only. The example targets iOS 16 or later; this is a deployment setting, not a claim that every supported OS version has been tested.

For device development, install the `aarch64-apple-ios` Rust target and configure your own development team and signing in `example/ios/project.yml`. Re-generate the project after changing that file. Simulator execution does not establish device performance or release readiness.

## Dependencies

`gpui-pre-mobile` is the Cargo package name; the Rust library is `gpui_mobile`. Use a Git dependency while evaluating this integration. The package's `0.1.0` manifest version does not imply a crates.io release.

```toml
[lib]
crate-type = ["staticlib", "rlib"]

[dependencies]
gpui-mobile = { package = "gpui-pre-mobile", git = "https://github.com/longbridge/gpui-mobile", rev = "0b882efdac7f524e0bb0b1d4c886b2aa752f9f20" }
gpui = { package = "gpui-pre", version = "=0.3.4", default-features = false }
gpui-kit = { git = "https://github.com/longbridge/gpui-kit", rev = "7d9efcd2069f9eaa6eb3ba6345aac4aa7d87c9f7", default-features = false, features = ["component"] }
```

These revisions reproduce the example's dependency baseline. The Kit revision includes mobile platform gating but predates mobile tooltip suppression. To use your local GPUI Kit checkout, replace the Kit dependency with:

```toml
gpui-kit = { path = "../gpui-kit/crates/kit", default-features = false, features = ["component"] }
```

Adjust the path relative to your application's manifest. Keep the GPUI core and renderer on the same release: the pinned mobile platform uses `gpui-pre` and `gpui-pre-wgpu` at `0.3.4`.

Unlike the desktop [Getting Started](/docs/getting-started) setup, mobile does not use `gpui_kit::application()` or `gpui_kit::platform`. Those desktop platform exports are excluded on iOS and Android. The mobile host initializes GPUI, calls `gpui_kit::init(cx)`, and mounts a single `component::Root` around the application's content.

## Embed a view in UIKit

UIKit owns the native window, navigation, safe areas, and keyboard layout. The example's `GPUITextView` is a Swift `UIView` wrapper around the GPUI platform's child `UIViewController`. Despite its name, it hosts a whole Rust conversation view, not just one `TextView` element.

Use these files together as the integration reference:

| File | Responsibility |
| --- | --- |
| [App.swift](https://github.com/longbridge/gpui-mobile/blob/0b882efdac7f524e0bb0b1d4c886b2aa752f9f20/example/ios/App.swift) | Native window, view wrapper, child controller containment, layout, and frame scheduling |
| [Embedding.h](https://github.com/longbridge/gpui-mobile/blob/0b882efdac7f524e0bb0b1d4c886b2aa752f9f20/example/ios/Embedding.h) | Swift bridging declarations for Rust callbacks |
| [src/lib.rs](https://github.com/longbridge/gpui-mobile/blob/0b882efdac7f524e0bb0b1d4c886b2aa752f9f20/example/src/lib.rs) | Application callback, Kit initialization, and Rust root view |
| [project.yml](https://github.com/longbridge/gpui-mobile/blob/0b882efdac7f524e0bb0b1d4c886b2aa752f9f20/example/ios/project.yml) | Rust build phase, static library linkage, frameworks, and bridging header |

The startup sequence is:

1. Call `gpui_ios_set_embedded()` before creating the GPUI application so the platform does not create a second native window.
2. Call the example-defined `gpui_ios_register_app()`. It registers a Rust callback with `gpui_mobile::ios::ffi::set_app_callback` that initializes Kit and opens the GPUI root.
3. Call `gpui_ios_run_demo()` to start the embedded application, then obtain its window and child controller with `gpui_ios_get_window()` and `gpui_ios_view_controller()`.
4. Attach the controller using UIKit containment: `addChild`, add its view, then `didMove(toParent:)`.

`gpui_ios_register_app()` belongs to the example, not the platform library. Adapt its callback to construct your own Rust view. The `run_demo` name is the current bridge entry point; it runs the registered application callback.

Once the example's `GPUITextView` wrapper is included in your app, a native controller can constrain it like any other view:

```swift
let content = GPUITextView(frame: .zero)
content.translatesAutoresizingMaskIntoConstraints = false
view.addSubview(content)
content.attach(to: self)

NSLayoutConstraint.activate([
    content.topAnchor.constraint(equalTo: view.safeAreaLayoutGuide.topAnchor),
    content.leadingAnchor.constraint(equalTo: view.leadingAnchor),
    content.trailingAnchor.constraint(equalTo: view.trailingAnchor),
    content.bottomAnchor.constraint(equalTo: view.keyboardLayoutGuide.topAnchor),
])
```

This fragment uses the example wrapper; `GPUITextView` is not an SDK-provided UIKit class. Copy its containment and layout behavior along with the declarations and build settings, rather than copying only the constraints.

### Lifetime, resizing, and frames

The platform retains the `ApplicationHandle` returned by `Application::run_embedded`. It must outlive callbacks and rendered views. The current bridge supports one GPUI view for the application's lifetime; it does not provide independently destroyable views, multiple instances, or reusable collection-view cells.

In `layoutSubviews`, update the child controller's frame only when nonzero bounds change, call `gpui_ios_layout_view`, then request a frame. The example performs those operations inside a Core Animation transaction with implicit animations disabled, keeping the Metal surface and GPUI viewport in sync during layout changes.

The host drives `gpui_ios_request_frame` through a `CADisplayLink` while visible and invalidates the link when the controller disappears. Forward application active/inactive callbacks as shown in `App.swift`. Keep UIKit and bridge calls on the main thread.

## Platform-specific behavior

`gpui_kit::is_mobile()` is an inline `const fn` that returns `true` for iOS and Android targets. It checks the compilation target, not window width or whether a mouse is connected.

```rust
if gpui_kit::is_mobile() {
    // Use touch-friendly interaction.
}
```

## Design for mobile

Share component behavior and content with desktop, while adapting the screen to touch and a narrow viewport:

- Let the native container handle navigation, safe areas, and keyboard avoidance. Avoid stacking a second title bar or duplicating safe-area padding inside Rust.
- Give each conversation one vertical scroll owner. For a `TextView` within that scroller, use `.w_full().min_w_0().scrollable(false)` so text and images fit the available width.
- Keep the composer compact when empty. Use a single-line input when multiline composition is unnecessary, and ensure the keyboard does not cover the send action.
- HoverCard opens and closes by tapping its trigger on iOS and Android. Tap outside to dismiss it; moving a finger does not open the card.
- Make actions discoverable by touch. Keep copy actions aligned with the reply and use a brief checkmark after copying. Do not rely on hover text to explain an action.
- Prefer short paragraphs and purposeful headings. Let code, tables, and images support the conversation rather than presenting every Markdown format in each reply.
- Use Kit theme colors, type sizes, and spacing consistently. Check long replies, wide code, image loading, and Chinese or other scripts at the actual device width.

GPUI Base disables its tooltip overlay on iOS and Android. This covers Kit tooltips routed through that overlay, not direct GPUI `.tooltip()` calls. The pinned baseline above predates that change. Do not add native GPUI hover tooltips to mobile views.

## Validation and current limits

For an application integration, check launch and return from the background, keyboard show/hide, viewport resizing, text selection and copying, scroll behavior, and touch feedback. Inspect the actual rendered screen rather than relying only on a Rust compile check.

Measure rendering on a physical device with a release build and Xcode Instruments before making performance claims. Simulator results are useful for layout and interaction, but are not device frame-time measurements.

Android uses a separate activity and surface lifecycle. The repository contains an Android example, but this guide does not establish Android Kit compatibility or native Android `View` embedding. Validate those paths separately before depending on them.

# GPUI Kit

One dependency for building desktop applications with GPUI:

```toml
[dependencies]
gpui-kit = "0.6"
```

`gpui-kit` depends on the matching set of GPUI crates, so an application
never lists GPUI itself. `use gpui_kit::*;` is GPUI, and each layer is
reachable by name:

| Path                  | Crate             | Feature          |
| --------------------- | ----------------- | ---------------- |
| `gpui_kit::*`         | `gpui`            | always           |
| `gpui_kit::platform`  | `gpui_platform`   | always           |
| `gpui_kit::base`      | `gpui-base`       | always           |
| `gpui_kit::component` | `gpui-component`  | `component` (on) |
| `gpui_kit::assets`    | `gpui-kit-assets` | `assets` (on)    |

`gpui_kit::application()` opens the platform and `gpui_kit::init()`
initializes the enabled layers:

```rust
use gpui_kit::component::button::*;
use gpui_kit::component::Root;
use gpui_kit::*;

struct Hello;

impl Render for Hello {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().child(Button::new("ok").primary().label("Let's Go!"))
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| Hello);
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("failed to open window");
        })
        .detach();
    });
}
```

The `gpui-component` features (`inspector`, `decimal`, `tree-sitter`,
`tree-sitter-languages`, and each `tree-sitter-<language>`) are available on
`gpui-kit` under the same names. `test-support` turns on GPUI's test harness
for `#[gpui_kit::test]`, `TestAppContext`, `VisualTestContext`, and native-platform
rendering support; enable it under `[dev-dependencies]`. The independent `profiler`
feature enables GPUI frame-event instrumentation and is off by default.
See <https://gpui-kit.com> for the guides.

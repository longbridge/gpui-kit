# GPUI Kit v0.8.0

GPUI Kit 0.8.0 adds an installable JavaScript Shell runtime, Linux WebView support, and a public rendering boundary for external editors. It also adds lighter syntax highlighting with syntect, native-style scroll bounce, richer completion highlighting, and improvements to text selection, tables, focus and accessibility.

## Runtime and platforms

- **Shell runtime**: run and scaffold JavaScript applications with `gpui-shell`; package them with `bundle`, and use `dev` for reloading during development. ([#3464](https://github.com/longbridge/gpui-kit/pull/3464), [#3487](https://github.com/longbridge/gpui-kit/pull/3487))
- **WebView**: use `gpui-webview` on macOS, Windows and Linux X11/XWayland. GPUI Fast enables overlays above native pages on macOS and Linux; bounds and clipping updates preserve the page viewport. ([#3395](https://github.com/longbridge/gpui-kit/pull/3395), [#3499](https://github.com/longbridge/gpui-kit/pull/3499), [#3591](https://github.com/longbridge/gpui-kit/pull/3591))

This release updates the optional GPUI Fast backend to **0.1.10**. The default backend remains pinned to **gpui-pre 0.3.9**, the latest published snapshot. The WebAssembly example uses wasm-bindgen **0.2.127** to match the updated backend dependency graph.

## Editor and text input

- **External editors**: `gpui_kit::base::text::editor_host` lets an editor render a `BlockNode` tree through TextView and collect keyed leaf geometry while retaining its own document model, selection, commands and undo. ([#3580](https://github.com/longbridge/gpui-kit/pull/3580))
- **Syntax highlighting**: enable the `syntect` feature for a lighter highlighter with bat’s syntax collection; tree-sitter takes precedence when both are enabled. ([#3512](https://github.com/longbridge/gpui-kit/pull/3512))
- **Completion highlighting**: configure `Lsp::with_completion_highlighter` or `set_completion_highlighter` with `highlight_prefix`, `highlight_contains`, `highlight_characters`, or a host callback returning byte ranges. ([#3496](https://github.com/longbridge/gpui-kit/pull/3496))
- **Wrapping**: `display_map()` and `prewrap(...)` are available on InputState and TextareaState as well as EditorState. `Editor::size` supports the shared control sizes. ([#3483](https://github.com/longbridge/gpui-kit/pull/3483), [#3495](https://github.com/longbridge/gpui-kit/pull/3495), [#3564](https://github.com/longbridge/gpui-kit/pull/3564), [#3480](https://github.com/longbridge/gpui-kit/pull/3480))
- **Software keyboards**: configure `input_action`, `autocorrect` and `autocapitalize`; masked fields are identified as passwords, and clear/mask controls have accessible names. ([#3557](https://github.com/longbridge/gpui-kit/pull/3557), [#3555](https://github.com/longbridge/gpui-kit/pull/3555), [#3554](https://github.com/longbridge/gpui-kit/pull/3554), [#3562](https://github.com/longbridge/gpui-kit/pull/3562))
- Fixed caret clipping in one-row viewports, caret visibility during reflow and height changes, vertical scroll clamping, scrollbar reach in auto-growing textareas, and caret crispness on click. ([#3583](https://github.com/longbridge/gpui-kit/pull/3583), [#3477](https://github.com/longbridge/gpui-kit/pull/3477), [#3527](https://github.com/longbridge/gpui-kit/pull/3527), [#3515](https://github.com/longbridge/gpui-kit/pull/3515))
- Custom Find queries and case changes start near the viewport; an unchanged query retains its active occurrence. Android’s Back button keeps entered text when `clean_on_escape` is enabled. ([#3584](https://github.com/longbridge/gpui-kit/pull/3584), [#3556](https://github.com/longbridge/gpui-kit/pull/3556))

## Markdown and text rendering

- **Selection format**: `SelectionFormat::SourceAcrossStyles` copies plain text within a single style and Markdown source across styles. ([#3517](https://github.com/longbridge/gpui-kit/pull/3517))
- Copy uses the current gesture before repaint, including virtualized rich content. Unmounting a selection endpoint retires the old selection, so retaining and remounting a TextView cannot revive it. ([#3589](https://github.com/longbridge/gpui-kit/pull/3589), [#3588](https://github.com/longbridge/gpui-kit/pull/3588))
- HTML `<br>` creates a line break, and `<pre>` / `<pre><code>` preserve their text and whitespace through HTML minification. ([#3569](https://github.com/longbridge/gpui-kit/pull/3569), [#3570](https://github.com/longbridge/gpui-kit/pull/3570), [#3581](https://github.com/longbridge/gpui-kit/pull/3581))

## Scrolling and motion

- **Scroll bounce**: precise scrolling uses a shared boundary-bounce engine, enabled by default on macOS, iOS and Android. Desktop motion follows NSScrollView; ordinary line-wheel input does not bounce. Nested areas retain their own gestures. ([#3500](https://github.com/longbridge/gpui-kit/pull/3500), [#3484](https://github.com/longbridge/gpui-kit/pull/3484))
- **Horizontal strips**: `ScrollableMask::vertical_wheel` maps a plain mouse wheel to horizontal scrolling and chains to the parent at the edge. ([#3518](https://github.com/longbridge/gpui-kit/pull/3518))
- Cubic-bezier easing preserves anticipation and overshoot, and transitions and presence retain duration precision. ([#3432](https://github.com/longbridge/gpui-kit/pull/3432), [#3590](https://github.com/longbridge/gpui-kit/pull/3590))

## Components

- **DataTable**: fit a column by double-clicking its resize handle or calling `fit_col`, with widths supplied by `TableDelegate::fit_column`; inspect the result with `col_width`. Configure `fixed_cols_border`, `select_on_modifier_click` and `focus_ring`. ([#3489](https://github.com/longbridge/gpui-kit/pull/3489), [#3482](https://github.com/longbridge/gpui-kit/pull/3482), [#3523](https://github.com/longbridge/gpui-kit/pull/3523), [#3565](https://github.com/longbridge/gpui-kit/pull/3565))
- **DataTable**: Home/End select the first/last row in row mode; row selection preserves event propagation, and default column resizing is capped at 1200px while preserving explicitly assigned initial widths. ([#3471](https://github.com/longbridge/gpui-kit/pull/3471), [#3469](https://github.com/longbridge/gpui-kit/pull/3469), [#3472](https://github.com/longbridge/gpui-kit/pull/3472))
- **Resizable**: use the standalone `resize_handle` for application-owned handles and `ResizablePanel::fixed` to preserve a panel’s size when its container changes. ([#3302](https://github.com/longbridge/gpui-kit/pull/3302), [#3478](https://github.com/longbridge/gpui-kit/pull/3478))
- **Dock**: subscribe to `DockToggleEvent` to distinguish user-driven dock toggles. ([#3546](https://github.com/longbridge/gpui-kit/pull/3546))
- **MessageScroller**: read the current reading position through `MessageScrollerState::logical_scroll_top`. ([#3485](https://github.com/longbridge/gpui-kit/pull/3485))
- **Select**: keyboard confirmation and reconciliation of changed items preserve the intended selection. ([#3446](https://github.com/longbridge/gpui-kit/pull/3446))
- **Focus and accessibility**: focus traps preserve containment and accessibility; disabled controls report their disabled state; sliders can be named with `accessibility_label`; settings fields inherit their setting’s title. ([#3443](https://github.com/longbridge/gpui-kit/pull/3443), [#3531](https://github.com/longbridge/gpui-kit/pull/3531), [#3494](https://github.com/longbridge/gpui-kit/pull/3494), [#3524](https://github.com/longbridge/gpui-kit/pull/3524), [#3582](https://github.com/longbridge/gpui-kit/pull/3582))
- **Tooltip**: switching between triggers is direction-independent; a pressed trigger remains dismissed until the pointer leaves, and removing a trigger dismisses its tooltip. ([#3501](https://github.com/longbridge/gpui-kit/pull/3501), [#3492](https://github.com/longbridge/gpui-kit/pull/3492), [#3528](https://github.com/longbridge/gpui-kit/pull/3528))
- **Popover**: keyboard activation inside content remains available. ([#3586](https://github.com/longbridge/gpui-kit/pull/3586))
- **TabBar**: overflow-menu actions call the tab’s own handler, the sliding indicator appears after the first frame, and `TabVariant::Folder` adds a folder presentation. ([#3568](https://github.com/longbridge/gpui-kit/pull/3568), [#3574](https://github.com/longbridge/gpui-kit/pull/3574), [#3415](https://github.com/longbridge/gpui-kit/pull/3415))
- **Tree**: selection and toggling happen on click rather than mouse-down. ([#3567](https://github.com/longbridge/gpui-kit/pull/3567))

## Shell APIs

- **Extensible catalog**: `gpui_shell::components::registry()` returns the unfrozen component catalog for host additions. Registered state objects support `release()` so applications can promptly dispose of retained resources. ([#3542](https://github.com/longbridge/gpui-kit/pull/3542), [#3544](https://github.com/longbridge/gpui-kit/pull/3544))
- **Lists and transcripts**: script lists support `with_follow_mode("tail")` and `on_visible_range_change`; item-count changes preserve the reader’s position. MessageScrollerState exposes append/prepend/splice, measurement and scrolling operations. ([#3548](https://github.com/longbridge/gpui-kit/pull/3548), [#3549](https://github.com/longbridge/gpui-kit/pull/3549), [#3547](https://github.com/longbridge/gpui-kit/pull/3547), [#3541](https://github.com/longbridge/gpui-kit/pull/3541))
- **Script components**: the catalog includes WebView/WebViewState, and Base TextareaState exposes `set_submit_on_enter` for message composers. ([#3532](https://github.com/longbridge/gpui-kit/pull/3532), [#3550](https://github.com/longbridge/gpui-kit/pull/3550))
- **Runtime compatibility**: scripts gain gradient backgrounds and custom shadows, more browser/Node-compatible runtime APIs, and corrected filesystem link handling. Script errors report their cause, and launching a window on Wayland no longer crashes during activation. ([#3534](https://github.com/longbridge/gpui-kit/pull/3534), [#3505](https://github.com/longbridge/gpui-kit/pull/3505), [#3510](https://github.com/longbridge/gpui-kit/pull/3510), [#3561](https://github.com/longbridge/gpui-kit/pull/3561), [#3509](https://github.com/longbridge/gpui-kit/pull/3509), [#3508](https://github.com/longbridge/gpui-kit/pull/3508))

## Breaking Changes

### Support Linux on X11 and rename the crate to gpui-webview ([#3395](https://github.com/longbridge/gpui-kit/pull/3395))

The crate is renamed from `gpui-wry` to `gpui-webview`.

```diff
 [dependencies]
-gpui-wry = "0.7.1"
+gpui-webview = "0.7.1"
```

```diff
-use gpui_wry::WebView;
+use gpui_webview::WebView;
```

On Linux, applications using the WebView must start on X11:

```diff
-gpui_kit::application().run(|cx| { /* ... */ });
+#[cfg(target_os = "linux")]
+let app = gpui_kit::platform::linux(WindowingModes::X11);
+#[cfg(not(target_os = "linux"))]
+let app = gpui_kit::application();
+app.run(|cx| { /* ... */ });
```

### Support gpui-pre and make `gpui-fast` optional again ([#3591](https://github.com/longbridge/gpui-kit/pull/3591))

Applications that relied on `gpui-webview` switching them to GPUI Fast must now select GPUI Fast on `gpui-kit` and enable the WebView feature with it, to keep overlays above the page on macOS and Linux:

```diff
 [dependencies]
-gpui-kit = "0.7"
-gpui-webview = "0.7"
+gpui-kit = { version = "0.7", features = ["gpui-fast"] }
+gpui-webview = { version = "0.7", features = ["gpui-fast"] }
```

Without both they build on gpui-pre; with only one of them, `gpui-webview` fails to compile and names the missing feature.

### Add SelectionFormat::SourceAcrossStyles ([#3517](https://github.com/longbridge/gpui-kit/pull/3517))

`SelectionFormat` is not `#[non_exhaustive]`, so code that matches it exhaustively needs an arm for the new variant.

```diff
 match format {
     SelectionFormat::Plain => ...,
     SelectionFormat::Source => ...,
+    SelectionFormat::SourceAcrossStyles => ...,
 }
```

### Let an application build its own resize handle ([#3302](https://github.com/longbridge/gpui-kit/pull/3302))

`ResizeHandle::on_drag` was `#[doc(hidden)]`; its constructor now takes `&T` instead of `Rc<T>`:

```diff
- .on_drag(DragPanel, |drag, _, _, cx| cx.new(|_| drag.deref().clone()))
+ .on_drag(DragPanel, |drag, _, _, cx| cx.new(|_| drag.clone()))
```

### Bounce scrollable content like NSScrollView, by default on macOS ([#3500](https://github.com/longbridge/gpui-kit/pull/3500))

No existing signature changes. Every existing call still compiles. Four defaults change behavior, compared with `main`:

**1. `ScrollBounce` is enabled by default on macOS.** On `main` it was enabled only on iOS and Android. A `ScrollBounce` created without `.enabled(..)` now bounces on macOS too. To keep it still:

```diff
- ScrollBounce::new(id, &handle, child)
+ ScrollBounce::new(id, &handle, child).enabled(false)
```

**2. `ScrollBounce` owns vertical wheel scrolling by default.** On `main`, the child scrolled itself and the wrapper only stretched the deltas the child left unused. Now the wrapper scrolls the handle itself, through the shared engine. Code that relied on the child-owned mode, such as touch emulation or synthesized drags, must ask for it:

```diff
- ScrollBounce::new(id, &handle, child).enabled(true)
+ ScrollBounce::new(id, &handle, child).enabled(true).scroll_wheel(false)
```

**3. `ScrollBounceMotion::default()` changes on macOS, Windows and Linux.** The defaults are now tuned to match NSScrollView, replacing the UIScrollView tuning:
- `tracking` goes from 0.55 to 0.075.
- `response` goes from 524 ms to ≈503 ms.
- The stretch is capped at 36 px.

iOS and Android are unchanged. To keep the previous feel:

```diff
- ScrollBounce::new(id, &handle, child)
+ ScrollBounce::new(id, &handle, child).motion(
+     ScrollBounceMotion::default()
+         .with_tracking(0.55)
+         .with_response(Duration::from_millis(524)),
+ )
```

**4. Scrollable components scroll differently.**
- **They bounce on macOS.** Scrollable (`overflow_y_scrollbar()`), List, DataTable, Tree, Select, Command, Textarea, Editor, Sidebar, Diff, MessageScroller, the settings page and scrollable TextView now bounce on precise scrolling. On `main` none of them did. Each one that has a builder can turn it off:

  ```diff
  - List::new(&state)
  + List::new(&state).scroll_bounce(false)
  ```

- **Nested areas no longer scroll together, on every platform.** The innermost area under the pointer takes the whole gesture, including at its ends. On `main`:
  - Nested GPUI scroll divs scrolled together.
  - MessageScroller handed the wheel to an ancestor scroller once it reached an end.

  Neither happens now. Sidebar, Diff, MessageScroller, the settings page and TextView have no `scroll_bounce` builder; they always follow the platform default.

Everything else listed under Public API is new in this PR, and so additive.

### Keep a scroll over text that can scroll, even at its end ([#3484](https://github.com/longbridge/gpui-kit/pull/3484))

Scrolling over a textarea or editor whose text can scroll no longer reaches the page around it once the text is at its end: the rest of that swipe, its momentum, and later wheel flicks over the text stay in the input. Text that fits, and vertical scrolling over a single-line input, still scroll the page. No setting restores the old behavior; to scroll the page, move the pointer off the input.

### Add `Folder` variant ([#3415](https://github.com/longbridge/gpui-kit/pull/3415))

`TabVariant` is not `#[non_exhaustive]`, so exhaustive matches need the new arm:

```diff
 match variant {
     ...
+    TabVariant::Folder => ...,
 }
```

### Keep cubic-bezier anticipation and overshoot in Easing ([#3432](https://github.com/longbridge/gpui-kit/pull/3432))

- Code that relied on `Easing::sample` staying within `[0, 1]` for an out-of-range cubic-bezier curve now sees overshoot:

```diff
- let alpha = easing.sample(delta);
+ let alpha = easing.sample(delta).clamp(0., 1.);
```

### Select the first or last row on Home and End in row mode ([#3471](https://github.com/longbridge/gpui-kit/pull/3471))

In row mode, Home and End select the first or last row instead of the first or last column. No setting restores the old behavior: Left and Right still move to a column, and a host that wants Home and End to pick a column can select one with `set_selected_col`.

### Don't stop event propagation in `set_selected_row` ([#3469](https://github.com/longbridge/gpui-kit/pull/3469))

`TableState::set_selected_row` no longer stops event propagation. Custom handlers that require it must call `cx.stop_propagation()` explicitly.

### Skip the notify in `set_line_number` when nothing changes ([#3475](https://github.com/longbridge/gpui-kit/pull/3475))

In a code editor, `set_line_number` no longer notifies when the line numbers are already as asked. A caller that relied on that call to re-render the editor notifies it itself:

```diff
  state.set_line_number(true, window, cx);
+ cx.notify();
```

### Notify from set_searchable only when it changes ([#3576](https://github.com/longbridge/gpui-kit/pull/3576))

`ListState::set_searchable` no longer notifies when the list is already searchable or not as asked. A caller that relied on that call to re-render the list notifies it itself:

```diff
  list.set_searchable(true, cx);
+ cx.notify();
```

### Add `bundle` to package an application, and a `dev` command ([#3487](https://github.com/longbridge/gpui-kit/pull/3487))

`--dev` no longer implies `--watch` or a debug log. `dev` is the command that reloads.

```diff
- gpui-shell run -d app --dev
+ gpui-shell dev -d app          # reload + debug log
+ gpui-shell dev -d app --dev    # … and the relaxed sandbox, as before
```

`InvocationKind` gains a variant, so an exhaustive `match` over it needs a `Bundle` arm.

```diff
  match gpui_shell::host::parse_invocation(arguments)? {
      InvocationKind::Run => …,
+     InvocationKind::Bundle => …,
      …
  }
```

### Fix the runtime gaps found by the gpui-shell benchmark apps ([#3505](https://github.com/longbridge/gpui-kit/pull/3505))

`gpui-shell run -d <dir>` with a manifest now stores data under the manifest `id`, the same place the installed copy uses, instead of `<dir>-<hash>`. Data written by earlier development runs stays in the old directory.

`process.env` was absent before; scripts that feature-detected it with `typeof process.env === "undefined"` now see an object.

`Capabilities`' read and write roots for a local run now include the manifest's roots rather than only the app and data directories, so `gpui-shell list` reports a wider grant for manifests that ask for one.

### Extend UI testing with semantic queries and Linux rendering ([#3402](https://github.com/longbridge/gpui-kit/pull/3402))

Existing pointer helpers now retain held modifiers. Callers that deliberately require an unmodified click while modifiers are held can request it explicitly; ordinary calls and signatures remain valid.

```diff
- window.click("save", cx); // previously reset held modifiers implicitly
+ window.click_with_options("save", ClickOptions::new(), cx); // explicitly unmodified, then restores held state
```

Nested context-menu callers keep the same construction. The corrected interaction is:

```diff
- window.right_click("inner-trigger", cx); // both inner and ancestor menus opened
+ window.right_click("inner-trigger", cx); // only the innermost menu opens
```

### Add find_all and configurable clicks to the UI test harness ([#3384](https://github.com/longbridge/gpui-kit/pull/3384))

`TestWindowExt` gains two required methods: `find_all` and `click_with_options`.
Downstream implementations must implement both. `click_with_modifiers` has a default
implementation through `click_with_options`. Call sites continue to import only
`TestWindowExt` for all window queries and interactions.

```diff
 impl TestWindowExt for MyTestWindow {
     // Existing methods remain.
+    fn find_all(&self, id: impl Into<ElementId>) -> Vec<ElementSnapshot> {
+        self.window.find_all(id)
+    }
+    fn click_with_options(&mut self, id: impl Into<ElementId>, options: ClickOptions, cx: &mut App) {
+        self.window.click_with_options(id, options, cx);
+    }
 }
```

Thank you to everyone who contributed code, documentation, testing, and feedback!

## What's Changed
* kit: Add optional gpui-fast backend by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3375
* tooltip: Make show delay and grace period configurable with `TooltipDefaults` by @madcodelife in https://github.com/longbridge/gpui-kit/pull/3376
* docs: Explain recovering focus when the focused element stops rendering by @AncientPixel in https://github.com/longbridge/gpui-kit/pull/3386
* diff: Add readonly patch viewer by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3378
* kit: Use one Windows manifest with GPUI Fast by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3380
* test: Add find_all and configurable clicks to the UI test harness by @AncientPixel in https://github.com/longbridge/gpui-kit/pull/3384
* test: Make Progress and TitleBar findable in UI tests by @AncientPixel in https://github.com/longbridge/gpui-kit/pull/3383
* input: Preserve selection, focus ring and context menu actions by @lurenjia534 in https://github.com/longbridge/gpui-kit/pull/3382
* diff: Add a Git history example and complete review APIs by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3388
* webview: Compose native views with the GPUI Fast backend by @ihavecoke in https://github.com/longbridge/gpui-kit/pull/3389
* text_view: Ignore gestures that stay inside one block plugin by @ylinwind in https://github.com/longbridge/gpui-kit/pull/3390
* text_view: Round the first and last table rows to the frame radius by @ihavecoke in https://github.com/longbridge/gpui-kit/pull/3309
* webview: Support Linux on X11 and rename the crate to gpui-webview by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3395
* webview: Require GPUI Fast for gpui-webview by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3398
* resizable: Let an application build its own resize handle by @ihavecoke in https://github.com/longbridge/gpui-kit/pull/3302
* menu: Open a context menu with a long press on touch by @Bombatomica64 in https://github.com/longbridge/gpui-kit/pull/3393
* text_view: Keep narrow table columns on one line by @madcodelife in https://github.com/longbridge/gpui-kit/pull/3399
* kit: Extend UI testing with semantic queries and Linux rendering by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3402
* settings: Keep focus rings inside the list viewport by @lurenjia534 in https://github.com/longbridge/gpui-kit/pull/3394
* scrollable_mask: Keep rounded scroll-table corners clean at any scroll offset by @ihavecoke in https://github.com/longbridge/gpui-kit/pull/3400
* dialog: Add margin_top to AlertDialog by @xxww0098 in https://github.com/longbridge/gpui-kit/pull/3404
* input: Add InputState::add_selection for programmatic multi-cursor by @sola-ryu in https://github.com/longbridge/gpui-kit/pull/3403
* text_selection: Stop writing selection state on every frame by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3407
* table: Make the sort icon a labelled button by @AncientPixel in https://github.com/longbridge/gpui-kit/pull/3396
* table: Report visible row ranges of one row or none by @AncientPixel in https://github.com/longbridge/gpui-kit/pull/3397
* dialog: Support configurable entrance animations by @xxww0098 in https://github.com/longbridge/gpui-kit/pull/3405
* docs: Explain the open_dialog builder and footer contracts by @xxww0098 in https://github.com/longbridge/gpui-kit/pull/3406
* dock: Hand the focus to the tab that replaces a closed one by @sola-ryu in https://github.com/longbridge/gpui-kit/pull/3409
* dropdown_button: Add A11y names for the menu trigger by @sola-ryu in https://github.com/longbridge/gpui-kit/pull/3410
* text, resizable: Stop notifying state that did not change by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3418
* editor: Add on-demand completion commands by @sola-ryu in https://github.com/longbridge/gpui-kit/pull/3411
* tab: Add `Folder` variant by @madcodelife in https://github.com/longbridge/gpui-kit/pull/3415
* table: Clear a right-clicked cell's outline on the next click by @sola-ryu in https://github.com/longbridge/gpui-kit/pull/3413
* tab: Scroll a clicked tab into view with the next one peeking by @madcodelife in https://github.com/longbridge/gpui-kit/pull/3419
* table: Allow a header row height separate from the body by @xhofe in https://github.com/longbridge/gpui-kit/pull/3420
* menu: Focus a context menu before its first frame by @madcodelife in https://github.com/longbridge/gpui-kit/pull/3422
* text: Preserve logical selection across presentation changes by @TomiXRM in https://github.com/longbridge/gpui-kit/pull/3421
* text: Keep inline HTML mark highlights in Markdown by @henrikogaard in https://github.com/longbridge/gpui-kit/pull/3425
* input: Anchor completion/code-action popups to the current frame's caret by @sola-ryu in https://github.com/longbridge/gpui-kit/pull/3412
* input: Run custom context menu builders after releasing the input by @costajohnt in https://github.com/longbridge/gpui-kit/pull/3433
* motion: Keep cubic-bezier anticipation and overshoot in Easing by @costajohnt in https://github.com/longbridge/gpui-kit/pull/3432
* input: Keep auto-grow rows fitted when the limits change by @sneffets in https://github.com/longbridge/gpui-kit/pull/3430
* text: Paint TextView selection under the glyphs by @cesartalen in https://github.com/longbridge/gpui-kit/pull/3439
* base: read the pointer only under hovered text, and ask no frame for a closed popup by @sunli829 in https://github.com/longbridge/gpui-kit/pull/3431
* chore: Restore default codegen units and stop optimizing proc macros by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3457
* kit: Update GPUI to gpui-pre 0.3.9 by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3459
* button: Keep the hover style registered while selected or disabled by @madcodelife in https://github.com/longbridge/gpui-kit/pull/3461
* diff: Wrap the next-change key once the list is scrolled to its end by @Smokanci in https://github.com/longbridge/gpui-kit/pull/3462
* shell: Ship gpui-shell as an installable runtime with run, new and packaging by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3464
* shell: Show each release's Shell pages for the binary it ships by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3479
* table: Don't stop event propagation in `set_selected_row` by @shreeve in https://github.com/longbridge/gpui-kit/pull/3469
* sidebar: Name the submenu caret button for assistive tech by @noosxe in https://github.com/longbridge/gpui-kit/pull/3470
* table: Select the first or last row on Home and End in row mode by @shreeve in https://github.com/longbridge/gpui-kit/pull/3471
* input: Skip the notify in `set_line_number` when nothing changes by @shreeve in https://github.com/longbridge/gpui-kit/pull/3475
* input: Clamp the vertical scroll offset in all layout modes by @shreeve in https://github.com/longbridge/gpui-kit/pull/3477
* table: Restore the default column resize protection by @shreeve in https://github.com/longbridge/gpui-kit/pull/3472
* resizable: Add `ResizablePanel::fixed` to keep a panel's size on container resizes by @shreeve in https://github.com/longbridge/gpui-kit/pull/3478
* tests: Build each crate's integration tests as one binary by @sunli829 in https://github.com/longbridge/gpui-kit/pull/3481
* input: Let `Editor` take a size, as `Textarea` does by @shreeve in https://github.com/longbridge/gpui-kit/pull/3480
* message_scroller: Add `logical_scroll_top` to read the reading position by @UncertaintyDeterminesYou4ndMe in https://github.com/longbridge/gpui-kit/pull/3485
* shell: Add `bundle` to package an application, and a `dev` command by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3487
* table: Add `DataTable::fixed_cols_border` by @shreeve in https://github.com/longbridge/gpui-kit/pull/3482
* input: Add `EditorState::wrap_row_count` and `prewrap` by @shreeve in https://github.com/longbridge/gpui-kit/pull/3483
* input: Expose the editor display map instead of a row count wrapper by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3495
* input: Keep a scroll over text that can scroll, even at its end by @shreeve in https://github.com/longbridge/gpui-kit/pull/3484
* docs: Add Home and End selecting the first and last row by @shreeve in https://github.com/longbridge/gpui-kit/pull/3486
* tooltip: Keep a pressed trigger's tooltip dismissed until the pointer leaves by @alex-vas in https://github.com/longbridge/gpui-kit/pull/3492
* table: Fit a column to its content on double-click of its resize handle by @miimsam in https://github.com/longbridge/gpui-kit/pull/3489
* slider: Add accessibility labels by @lurenjia534 in https://github.com/longbridge/gpui-kit/pull/3494
* tests: Keep the WebSocket pending-read test pumping until its server closes by @madcodelife in https://github.com/longbridge/gpui-kit/pull/3497
* shell: Fix the runtime gaps found by the gpui-shell benchmark apps by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3505
* shell: Strip symbols from release builds by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3507
* tooltip: Fix direction-dependent switching between triggers by @mrkishi in https://github.com/longbridge/gpui-kit/pull/3501
* shell: Fix a Wayland crash when activating the window at launch by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3508
* shell: Fold script failures to one line and explain what failed by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3509
* scroll: Bounce scrollable content like NSScrollView, by default on macOS by @shreeve in https://github.com/longbridge/gpui-kit/pull/3500
* highlighter: Add syntect as a lighter syntax highlighting backend by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3512
* webview: Clip native views without resizing the page by @superoskin in https://github.com/longbridge/gpui-kit/pull/3499
* input: Fix the caret flashing off on click, and draw it crisp at 1px by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3515
* shell: Fix more runtime gaps found by the benchmark apps by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3510
* shell: Gate the process probe helper to Unix by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3530
* focus_trap: Keep focus contained and preserve accessibility by @ruiyangke in https://github.com/longbridge/gpui-kit/pull/3443
* select: Correct keyboard confirmation and item reconciliation by @ruiyangke in https://github.com/longbridge/gpui-kit/pull/3446
* scroll: Scroll horizontal strips with a vertical mouse wheel by @KingEmma7 in https://github.com/longbridge/gpui-kit/pull/3518
* shell: Add WebView and WebViewState to the component catalog by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3532
* window_border: Don't offer resizing for a window that isn't resizable by @CodeName-Anti in https://github.com/longbridge/gpui-kit/pull/3525
* shell: Accept a Background in bg and add a shadow style by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3534
* base: Mark disabled controls disabled on their accessibility node by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3531
* table: Add `TableState::select_on_modifier_click` by @shreeve in https://github.com/longbridge/gpui-kit/pull/3523
* scrollbar: Leave presses and hover over an occluding element to it by @lukelast in https://github.com/longbridge/gpui-kit/pull/3526
* input: Let the textarea scrollbar drag reach the content end by @dannyward630 in https://github.com/longbridge/gpui-kit/pull/3527
* base: Skip shared heading callbacks when comparing text styles by @KlausUllrich in https://github.com/longbridge/gpui-kit/pull/3536
* shell: Add script methods to MessageScrollerState by @UncertaintyDeterminesYou4ndMe in https://github.com/longbridge/gpui-kit/pull/3541
* shell: Let an embedding host extend the gpui-component catalog by @UncertaintyDeterminesYou4ndMe in https://github.com/longbridge/gpui-kit/pull/3542
* shell: Declare the component catalog under its registry's module specifier by @UncertaintyDeterminesYou4ndMe in https://github.com/longbridge/gpui-kit/pull/3543
* shell: Let a script release retained component state by @UncertaintyDeterminesYou4ndMe in https://github.com/longbridge/gpui-kit/pull/3544
* docs: Correct three gpui-shell script-facing docs by @UncertaintyDeterminesYou4ndMe in https://github.com/longbridge/gpui-kit/pull/3545
* dock: Report the user's dock toggles with DockToggleEvent by @IHadACookie in https://github.com/longbridge/gpui-kit/pull/3546
* shell: Keep a list's reading position when its item count changes by @UncertaintyDeterminesYou4ndMe in https://github.com/longbridge/gpui-kit/pull/3547
* shell: Let a script textarea submit on Enter by @UncertaintyDeterminesYou4ndMe in https://github.com/longbridge/gpui-kit/pull/3550
* input: Name the clear button for screen readers by @Bombatomica64 in https://github.com/longbridge/gpui-kit/pull/3554
* input: Report a masked input as a password field to screen readers by @Bombatomica64 in https://github.com/longbridge/gpui-kit/pull/3555
* input: Keep the text on Android's back button with clean_on_escape by @Bombatomica64 in https://github.com/longbridge/gpui-kit/pull/3556
* input: Name the mask toggle by @counterbeing in https://github.com/longbridge/gpui-kit/pull/3562
* shell: Add `on_visible_range_change` to report what a list shows by @UncertaintyDeterminesYou4ndMe in https://github.com/longbridge/gpui-kit/pull/3549
* diff: Keep the row-height hint after the list lays out at a new width by @zzfn in https://github.com/longbridge/gpui-kit/pull/3563
* input: Tell the platform's keyboard how to assist each input by @Bombatomica64 in https://github.com/longbridge/gpui-kit/pull/3557
* website: Link to Native Shell from the nav and READMEs by @huacnlee with @Copilot in https://github.com/longbridge/gpui-kit/pull/3571
* shell: Act on the link itself for lstat, readlink, unlink, rmdir and rename by @zzfn in https://github.com/longbridge/gpui-kit/pull/3561
* tests: Fix gpui-shell path assertions on Windows and report script build errors by @Elitelord in https://github.com/longbridge/gpui-kit/pull/3535
* input: share display_map and prewrap across text modes by @TomiXRM in https://github.com/longbridge/gpui-kit/pull/3564
* table: Let DataTable turn off its focus ring with focus_ring(false) by @abduladnan-mt7 in https://github.com/longbridge/gpui-kit/pull/3565
* website: Update Native Shell domain and add project logos by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3579
* text: Keep `<pre>` text as text through the HTML minifier by @UncertaintyDeterminesYou4ndMe in https://github.com/longbridge/gpui-kit/pull/3570
* text: Keep whitespace in `<pre><code>` through the HTML minifier by @madcodelife in https://github.com/longbridge/gpui-kit/pull/3581
* tree: select and toggle entries on click instead of mouse down by @kevin9327 in https://github.com/longbridge/gpui-kit/pull/3567
* text: editor_host, a public API for external editors by @wabzqem in https://github.com/longbridge/gpui-kit/pull/3580
* setting: Name bool fields after their setting's title by @Elitelord in https://github.com/longbridge/gpui-kit/pull/3524
* shell: Add `with_follow_mode` to make a list follow its tail by @UncertaintyDeterminesYou4ndMe in https://github.com/longbridge/gpui-kit/pull/3548
* text: Break the line where an HTML `<br>` stands by @UncertaintyDeterminesYou4ndMe in https://github.com/longbridge/gpui-kit/pull/3569
* tooltip: Dismiss the tooltip when its trigger is removed by @dux-web in https://github.com/longbridge/gpui-kit/pull/3528
* tab: Draw the sliding indicator on the frame after a bar's first by @H1X4Dev in https://github.com/longbridge/gpui-kit/pull/3574
* table: Cover DataTable builder and focus ring configuration by @YidaWeng in https://github.com/longbridge/gpui-kit/pull/3572
* slider: Write the indicator's bounds only when they moved by @H1X4Dev in https://github.com/longbridge/gpui-kit/pull/3575
* list: Notify from set_searchable only when it changes by @H1X4Dev in https://github.com/longbridge/gpui-kit/pull/3576
* setting: Name dropdown, number and text fields after their setting's title by @madcodelife in https://github.com/longbridge/gpui-kit/pull/3582
* ci: Quote the tooltip test command so the workflow parses by @madcodelife in https://github.com/longbridge/gpui-kit/pull/3592
* tests: Accept USERPROFILE as the default home variable on Windows by @Elitelord in https://github.com/longbridge/gpui-kit/pull/3593
* webview: Support gpui-pre and make `gpui-fast` optional again by @madcodelife in https://github.com/longbridge/gpui-kit/pull/3591
* motion: Preserve transition and presence duration precision by @TomiXRM in https://github.com/longbridge/gpui-kit/pull/3590
* popover: Preserve keyboard activation in content by @TomiXRM in https://github.com/longbridge/gpui-kit/pull/3586
* text: Add SelectionFormat::SourceAcrossStyles by @stippi in https://github.com/longbridge/gpui-kit/pull/3517
* release: Publish GPUI with crates.io wgpu 30.0.1 by @huacnlee in https://github.com/longbridge/gpui-kit/pull/3600
* input: Let hosts choose how completion labels are highlighted by @xhofe in https://github.com/longbridge/gpui-kit/pull/3496
* input: keep caret rows visible in constrained viewports by @TomiXRM in https://github.com/longbridge/gpui-kit/pull/3583
* input: anchor custom search queries to the viewport by @TomiXRM in https://github.com/longbridge/gpui-kit/pull/3584
* tab_bar: run a tab's own on_click from the overflow menu by @kevin9327 in https://github.com/longbridge/gpui-kit/pull/3568
* text: Prevent stale selections from returning after remount by @TomiXRM in https://github.com/longbridge/gpui-kit/pull/3588
* text: Copy the current TextView selection before repaint by @TomiXRM in https://github.com/longbridge/gpui-kit/pull/3589

## New Contributors
* @xxww0098 made their first contribution in https://github.com/longbridge/gpui-kit/pull/3404
* @sola-ryu made their first contribution in https://github.com/longbridge/gpui-kit/pull/3403
* @xhofe made their first contribution in https://github.com/longbridge/gpui-kit/pull/3420
* @TomiXRM made their first contribution in https://github.com/longbridge/gpui-kit/pull/3421
* @henrikogaard made their first contribution in https://github.com/longbridge/gpui-kit/pull/3425
* @costajohnt made their first contribution in https://github.com/longbridge/gpui-kit/pull/3433
* @sneffets made their first contribution in https://github.com/longbridge/gpui-kit/pull/3430
* @cesartalen made their first contribution in https://github.com/longbridge/gpui-kit/pull/3439
* @Smokanci made their first contribution in https://github.com/longbridge/gpui-kit/pull/3462
* @shreeve made their first contribution in https://github.com/longbridge/gpui-kit/pull/3469
* @UncertaintyDeterminesYou4ndMe made their first contribution in https://github.com/longbridge/gpui-kit/pull/3485
* @alex-vas made their first contribution in https://github.com/longbridge/gpui-kit/pull/3492
* @miimsam made their first contribution in https://github.com/longbridge/gpui-kit/pull/3489
* @mrkishi made their first contribution in https://github.com/longbridge/gpui-kit/pull/3501
* @superoskin made their first contribution in https://github.com/longbridge/gpui-kit/pull/3499
* @ruiyangke made their first contribution in https://github.com/longbridge/gpui-kit/pull/3443
* @KingEmma7 made their first contribution in https://github.com/longbridge/gpui-kit/pull/3518
* @CodeName-Anti made their first contribution in https://github.com/longbridge/gpui-kit/pull/3525
* @lukelast made their first contribution in https://github.com/longbridge/gpui-kit/pull/3526
* @dannyward630 made their first contribution in https://github.com/longbridge/gpui-kit/pull/3527
* @Elitelord made their first contribution in https://github.com/longbridge/gpui-kit/pull/3535
* @abduladnan-mt7 made their first contribution in https://github.com/longbridge/gpui-kit/pull/3565
* @H1X4Dev made their first contribution in https://github.com/longbridge/gpui-kit/pull/3574
* @YidaWeng made their first contribution in https://github.com/longbridge/gpui-kit/pull/3572

**Full Changelog**: https://github.com/longbridge/gpui-kit/compare/v0.7.3...v0.8.0

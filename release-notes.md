# Release Notes

## Pending Updates

### 0.7.0 (unreleased)

#### Root owns window overlays

`gpui_component::Root` now always mounts the dialog, sheet and notification
layers above application content. Opening a dialog, sheet or notification no
longer depends on the application's view rendering its layer. Notifications use
the Root's full bounds, and cached content does not duplicate or suppress layers.

#### Added: `SettingGroup::variant`

```rust
pub fn variant(self, variant: GroupBoxVariant) -> Self
```

Overrides, for one group, the variant that `Settings::with_group_variant`
applies to every group. Use it when a single page should present its items
directly — `GroupBoxVariant::Normal` removes the card surface the global
default draws — while the other pages keep the global variant.

#### Plot moves to `gpui-base`

The chart primitives — scales, shapes, `PlotAxis`, `Grid`, `PlotLabel`,
`PathCaches`, the `Plot` trait and hover tracking — now live in
`gpui_base::plot`, so a design system built on `gpui-base` alone can draw charts
without depending on `gpui-component`. `gpui_component::plot` re-exports them,
so existing import paths such as `gpui_kit::component::plot::scale::ScaleLinear`
keep working; the API itself is tidied for 0.7.0 (see Breaking changes).

```rust
pub struct PlotElement<P>        // gpui_base::plot: the element behind every Plot
pub fn hover_progress(window: &mut Window, cx: &mut App) -> f32
pub fn is_hover_entering(window: &mut Window, cx: &mut App) -> bool
pub fn pointer_spring(cx: &App) -> Spring
pub struct PlotMotion            // pointer spring and hover enter/exit transitions
pub struct PlotTheme             // gpui_base::Theme::plot, carrying PlotMotion
```

A hand-written plot becomes an element with
`impl IntoElement for MyPlot { type Element = PlotElement<Self>; … }`;
`#[derive(IntoPlot)]` now generates exactly that. Base plot motion is
motionless by default, and `gpui-component` projects its motion tokens onto
`gpui_base::Theme::plot` whenever its theme is applied. The `decimal` feature
moves to `gpui-base`; `gpui-component`'s `decimal` feature forwards to it.

#### Breaking changes

##### Plot API

Charts built from `LineChart`, `BarChart`, `AreaChart`, `PieChart`,
`RadarChart`, `CandlestickChart` and `SankeyChart` are unaffected apart from
the new `f32` support. Custom plots built on the primitives need these changes:

- Scale ranges are two-element arrays and domains take any iterator:
  `ScaleLinear::new(values, [height, 0.])`; the same for `ScalePoint` and
  `ScaleBand`.
- The value bound is the documented `PlotValue` (`f32`, `f64`, and `Decimal`
  with `decimal`), replacing the hidden `Sealed`.
- `Scale::least_index` is `nearest_index`; `least_index_with_domain` is removed.
- `ScaleBand::band_width` no longer caps bands at 30px; set
  `ScaleBand::max_band_width`, or `BarChart`/`CandlestickChart::max_band_width`
  (30px by default, so charts look the same).
- `PlotAxis::x`/`y` are `x_axis_at`/`y_axis_at`, and labels are placed at paint
  time so builder order no longer matters. `AXIS_GAP` is `axis_gutter(font_size)`.
- `StrokeStyle` is `Curve` and `stroke_style` is `curve`; `dot_fill_color`/
  `dot_stroke_color` are `dot_fill`/`dot_stroke`; `dot` and `RadialLine::closed`
  take a `bool`.
- `Arc::paint`, `paint_cached` and `contains` drop the radius overrides; build
  another `Arc` for other radii.
- `PlotHover::focus` and `Tooltip::focus` are `progress`.
- `Grid::x`/`y` take any iterator of pixels; drop a `.collect()` whose type was
  only inferred from the old `Vec` parameter.
- `TooltipState`, `AxisText`, `label::Text`, `ArcData`, `StackPoint`,
  `StackSeries`, `SankeyLink` and the Sankey layout records are
  `#[non_exhaustive]`; build them with their constructors.
- `#[derive(IntoPlot)]` generates `type Element = PlotElement<Self>` instead of
  an `Element` impl on the plot; `gpui_base::Theme` gains a `plot` field.

`StrokeStyle` and `stroke_style` are removed outright. Deprecated aliases keep
`AXIS_GAP`, `PlotAxis::x`/`y`, `dot_fill_color`, `dot_stroke_color` and `focus`
compiling for this release.

```diff
- let y = ScaleLinear::new(values.collect(), vec![height, 0.]);
+ let y = ScaleLinear::new(values, [height, 0.]);
- Line::new().stroke_style(StrokeStyle::Linear).dot()
+ Line::new().curve(Curve::Linear).dot(true)
- PlotAxis::new().x(height).x_label(labels)
+ PlotAxis::new().x_axis_at(height).x_label(labels)
```

##### Root layers

The following `gpui-component` APIs have been removed:

- `Root::render_dialog_layer`
- `Root::render_sheet_layer`
- `Root::render_notification_layer`

Remove these calls from application render methods. There are no replacement
layer switches or manual mounting APIs; Root renders all three layers itself.
Existing custom layer positions move to Root's window-level overlay placement.

```diff
 div()
     .size_full()
     .child(self.content.clone())
-    .children(Root::render_sheet_layer(window, cx))
-    .children(Root::render_dialog_layer(window, cx))
-    .children(Root::render_notification_layer(window, cx))
```

If a render method first stored these layers in local variables, remove those
variables and the corresponding `.children(...)` calls as well.

`window.open_dialog`, `window.open_sheet` and `window.push_notification` remain
the application-facing APIs. Keep a `Root` as the window's root view, or use the
new window helper below.

#### Added: `gpui_base::Root` and `gpui_kit::open_window`

```rust
pub fn open_window<V: Render>(
    options: WindowOptions,
    cx: &mut App,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
) -> Result<(AnyWindowHandle, Entity<V>)>
```

Opens a window and returns both the window handle and the content entity. The
helper always wraps content in `gpui_base::Root`, independent of Cargo features.
The helper is defined only in Kit. `component::Root` re-exports the Base type.

Base owns the root, content, overlay hosting, keyboard traversal and selection
copying. Explicit `gpui_component::init` registers a per-window extension for
styled dialogs, sheets, notifications, tooltips, menus, touch selection and
window presentation. Base does not depend on Component or its theme. Plugins
must be registered before creating windows; they do not retrofit existing roots.

Component operations belong to `WindowExt`; the previous Component-specific
Root methods and fields (including `notification`) are no longer exposed on Root.

```rust
let (window, view) = gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
    cx.new(|cx| MyApp::new(window, cx))
})?;
```

Call `gpui_kit::init(cx)` before opening component-backed windows, and do not
return a Root from this helper's builder. In an async context, call the helper
inside `cx.update`.

Kit examples and the native/web story galleries use this helper for standard window
startup. Base examples continue using `gpui_base::init` and GPUI's window API
directly, without a dependency on Kit. The FPS example disables Kit's default
features.

Quit and close-window actions, keyboard shortcuts and confirmation flows remain
application-owned. Kit initialization does not install default quit or close
bindings.

The `Root` and `WindowExt` text-selection methods are removed. Use
`gpui_base::TextSelection::{selected_text, has_selection, clear, end}`.

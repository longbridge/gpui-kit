use std::sync::{Arc, Mutex};

use gpui::{
    AnyElement, App, AvailableSpace, Bounds, CursorStyle, Element, ElementId, GlobalElementId,
    Hitbox, HitboxBehavior, InspectorElementId, InteractiveElement as _, IntoElement, LayoutId,
    MouseButton, MouseDownEvent, ObjectFit, ParentElement, Pixels, Role, SharedString, Size,
    StatefulInteractiveElement as _, Styled, StyledImage, TextStyle, Window, div, img, px, size,
};

use super::{
    MarkdownExtensions, MarkdownInlineMetrics, MarkdownInlinePresentation,
    MarkdownInlineRenderContext, MarkdownNode, TextViewMultiClickKind,
    inline::point_in_text_selection, state::LineSpan,
};
use crate::GlobalState;

/// A measured static object, reused unchanged from wrapping through painting.
#[derive(Clone)]
pub(super) struct MeasuredInlineObject {
    pub metrics: MarkdownInlineMetrics,
    presentation: Option<MarkdownInlinePresentation>,
    text: SharedString,
    font_size: Pixels,
    text_style: TextStyle,
    appearance: super::markdown_inline::InlineAppearance,
}

impl MeasuredInlineObject {
    pub fn measure(
        node: &MarkdownNode,
        extensions: &MarkdownExtensions,
        style: &TextStyle,
        width: Option<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let font_size = style.font_size.to_pixels(window.rem_size());
        let context = MarkdownInlineRenderContext {
            text_style: style.clone(),
            font_size,
            line_height: window.line_height(),
            rem_size: window.rem_size(),
            available_width: width,
        };
        let presentation = extensions.render_inline(node, &context, window, cx);
        let mut appearance = presentation
            .as_ref()
            .map(|p| p.appearance.clone())
            .unwrap_or_default();
        let mut style = style.clone();
        if let Some(weight) = appearance.font_weight {
            style.font_weight = weight;
        }
        if let Some(color) = appearance.color {
            style.color = color;
        }
        if presentation.as_ref().is_some_and(|p| p.image.is_some()) {
            appearance.padding_x = Pixels::ZERO;
        }
        // A fallback is one atomic line; source newlines remain in copied text.
        let text: SharedString = node.as_text().replace(['\r', '\n'], " ").into();
        let line = window.text_system().shape_line(
            text.clone(),
            font_size,
            &[style.to_run(text.len())],
            None,
        );
        let height = context.line_height.max(line.ascent + line.descent);
        let fallback = MarkdownInlineMetrics::new(
            size(line.width.max(px(1.)) + appearance.padding_x * 2., height),
            (height - line.ascent - line.descent) / 2. + line.ascent,
        );
        let mut metrics = presentation
            .as_ref()
            .and_then(|p| p.image.as_ref())
            .map_or(fallback, |(_, metrics)| *metrics);
        let scale = width.map_or(1., |width| {
            (f32::from(width.max(Pixels::ZERO)) / f32::from(metrics.size.width)).min(1.)
        });
        metrics.size = size(metrics.size.width * scale, metrics.size.height * scale);
        metrics.baseline *= scale;
        // Image decode failure uses this same box. Fit all of the alternative
        // text to that box, even when the image is much narrower than its name.
        let fallback_scale = (metrics.size.width / fallback.size.width)
            .min(metrics.size.height / fallback.size.height)
            .min(1.);
        appearance.padding_x *= fallback_scale;
        appearance.radius *= scale;
        Self {
            metrics,
            presentation,
            text,
            font_size: font_size * fallback_scale,
            text_style: style,
            appearance,
        }
    }

    fn element(&self) -> AnyElement {
        if self.metrics.size.width <= Pixels::ZERO || self.metrics.size.height <= Pixels::ZERO {
            return gpui::Empty.into_any_element();
        }
        let fallback_text = self.text.clone();
        let font_size = self.font_size;
        let bounds = self.metrics.size;
        let padding = self.appearance.padding_x;
        let color = self.text_style.color;
        let weight = self.text_style.font_weight;
        let fallback = move || {
            div()
                .w(bounds.width)
                .h(bounds.height)
                .px(padding)
                .text_size(font_size)
                .text_color(color)
                .font_weight(weight)
                .line_height(bounds.height)
                .whitespace_nowrap()
                .overflow_hidden()
                .child(fallback_text.clone())
                .into_any_element()
        };
        if let Some((image, _)) = self.presentation.as_ref().and_then(|p| p.image.as_ref()) {
            img(image.clone())
                .w(bounds.width)
                .h(bounds.height)
                .object_fit(ObjectFit::Contain)
                .with_loading(fallback.clone())
                .with_fallback(fallback)
                .into_any_element()
        } else {
            fallback()
        }
    }
}

/// Passive leaf whose whole bounds participate in TextView selection.
pub(super) struct InlineObject {
    id: ElementId,
    node: MarkdownNode,
    object: MeasuredInlineObject,
    selected: Arc<Mutex<bool>>,
    selection_bounds: Bounds<Pixels>,
    line_bounds: Bounds<Pixels>,
    content: AnyElement,
}

impl InlineObject {
    pub fn new(
        id: impl Into<ElementId>,
        node: MarkdownNode,
        object: MeasuredInlineObject,
        selected: Arc<Mutex<bool>>,
        selection_bounds: Bounds<Pixels>,
        line_bounds: Bounds<Pixels>,
    ) -> Self {
        let mut content = div()
            .id("inline-object-content")
            .w(object.metrics.size.width)
            .h(object.metrics.size.height)
            .child(object.element());
        if object.appearance.hover_background.is_some() {
            content = content.on_hover(|_, window, _| window.refresh());
        }
        let content = if let Some(build) = object
            .presentation
            .as_ref()
            .and_then(|p| p.hover_card.clone())
        {
            crate::HoverCard::new("inline-hover-card")
                .anchor(gpui::Anchor::TopCenter)
                .trigger(content)
                .content(move |_, window, cx| {
                    div().id("inline-hover-content").child(build(window, cx))
                })
                .into_any_element()
        } else {
            content.into_any_element()
        };
        Self {
            id: id.into(),
            node,
            object,
            selected,
            selection_bounds,
            line_bounds,
            content,
        }
    }
}

impl IntoElement for InlineObject {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for InlineObject {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;
    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn a11y_role(&self) -> Option<Role> {
        Some(Role::Image)
    }

    fn write_a11y_info(&self, node: &mut gpui::accesskit::Node) {
        node.set_role(Role::Image);
        node.set_label(self.node.accessibility_name());
        node.set_read_only();
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let metrics = self.object.metrics;
        (
            window.request_layout(
                gpui::Style {
                    size: size(metrics.size.width.into(), metrics.size.height.into()),
                    ..Default::default()
                },
                [],
                cx,
            ),
            (),
        )
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Hitbox {
        self.content.prepaint_as_root(
            bounds.origin,
            Size {
                width: AvailableSpace::Definite(bounds.size.width),
                height: AvailableSpace::Definite(bounds.size.height),
            },
            window,
            cx,
        );
        if let Some(view) = GlobalState::global(cx).text_view_state() {
            let state = view.read(cx);
            if state.max_lines.is_some()
                && let Ok(mut spans) = state.line_spans.lock()
            {
                spans.push(LineSpan {
                    top: bounds.top(),
                    bottom: bounds.bottom(),
                    line_height: bounds.size.height,
                });
            }
        }
        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        hitbox: &mut Hitbox,
        window: &mut Window,
        cx: &mut App,
    ) {
        let view = GlobalState::global(cx).text_view_state().cloned();
        let selectable = view
            .as_ref()
            .is_some_and(|view| view.read(cx).is_selectable());
        let selected = view.as_ref().is_some_and(|view| {
            let state = view.read(cx);
            if selectable && state.preserve_inline_selection && !state.is_all_selected() {
                return self.selected.lock().is_ok_and(|selected| *selected);
            }
            selectable
                && (state.is_all_selected()
                    || state.multi_click_selection().is_some_and(|s| {
                        s.line_bounds.map_or_else(
                            || bounds.contains(&s.pos),
                            |row| row.contains(&bounds.center()),
                        )
                    })
                    || state.selection_points(cx).is_some_and(|(start, end)| {
                        point_in_text_selection(
                            self.selection_bounds.origin,
                            self.selection_bounds.size.width,
                            start,
                            end,
                            self.selection_bounds.size.height,
                        )
                    }))
        });
        if let Ok(mut value) = self.selected.lock() {
            *value = selected;
        }
        let appearance = &self.object.appearance;
        let background = if hitbox.is_hovered(window) {
            appearance.hover_background.or(appearance.background)
        } else {
            appearance.background
        };
        if let Some(color) = background {
            window.paint_quad(gpui::fill(bounds, color).corner_radii(appearance.radius));
        }
        if selected {
            let color = view.as_ref().unwrap().read(cx).text_view_style.selection();
            window.paint_quad(gpui::fill(bounds, color).corner_radii(appearance.radius));
        }
        self.content.paint(window, cx);
        if selectable {
            window.set_cursor_style(CursorStyle::IBeam, hitbox);
            let visible = bounds.intersect(&window.content_mask().bounds);
            if visible.size.width > Pixels::ZERO && visible.size.height > Pixels::ZERO {
                view.as_ref().unwrap().update(cx, |state, _| {
                    state.selection_adapter.register_inline(vec![visible]);
                });
            }
            let hitbox = hitbox.clone();
            let selected_state = self.selected.clone();
            let text = self.node.as_text().to_string();
            let current_view = window.current_view();
            let line_bounds = self.line_bounds;
            window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                if !phase.bubble()
                    || !hitbox.is_hovered(window)
                    || event.button != MouseButton::Left
                    || !(2..=3).contains(&event.click_count)
                {
                    return;
                }
                GlobalState::suppress_text_selection(cx);
                if let Ok(mut value) = selected_state.lock() {
                    *value = true;
                }
                if let Some(view) = &view {
                    view.update(cx, |state, cx| {
                        if event.click_count == 3 {
                            state.set_multi_click_line(line_bounds, cx);
                        } else {
                            state.set_multi_click_selection(
                                event.position,
                                TextViewMultiClickKind::Word,
                                text.clone(),
                                cx,
                            );
                        }
                    });
                }
                cx.notify(current_view);
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styled_text_padding_scales_with_atomic_geometry() {
        use gpui::{Empty, FontWeight, TestApp};
        let mut app = TestApp::new();
        let mut window = app.open_window(|_, _| Empty);
        let node = MarkdownNode::new("mention", ()).text("@member");
        let plain = MarkdownExtensions::default().inline_renderer("mention", |_, _, _, _| {
            Some(MarkdownInlinePresentation::text().font_weight(FontWeight::MEDIUM))
        });
        let decorated =
            MarkdownExtensions::default().inline_renderer("mention", |_, context, _, _| {
                Some(
                    MarkdownInlinePresentation::text()
                        .font_weight(FontWeight::MEDIUM)
                        .padding_x(context.font_size * 0.3)
                        .rounded(context.font_size * 0.25),
                )
            });
        for font_size in [16., 24., 32.] {
            window.update(|_, window, cx| {
                let style = TextStyle {
                    font_size: px(font_size).into(),
                    ..Default::default()
                };
                let plain = MeasuredInlineObject::measure(&node, &plain, &style, None, window, cx);
                let full =
                    MeasuredInlineObject::measure(&node, &decorated, &style, None, window, cx);
                assert!(
                    (full.metrics.size.width - plain.metrics.size.width - px(font_size * 0.6))
                        .abs()
                        < px(0.001)
                );
                assert_eq!(full.metrics.baseline, plain.metrics.baseline);
                assert_eq!(full.text_style.font_weight, FontWeight::MEDIUM);
                let narrow = MeasuredInlineObject::measure(
                    &node,
                    &decorated,
                    &style,
                    Some(full.metrics.size.width / 2.),
                    window,
                    cx,
                );
                assert_eq!(narrow.metrics.size, full.metrics.size / 2.);
                assert_eq!(narrow.appearance.padding_x, full.appearance.padding_x / 2.);
                assert_eq!(narrow.font_size, full.font_size / 2.);
            });
        }
    }

    #[test]
    fn invalid_plugin_metrics_use_measured_text_and_zero_width_is_finite() {
        use gpui::{Empty, TestApp};
        let mut app = TestApp::new();
        let mut window = app.open_window(|_, _| Empty);
        let node = MarkdownNode::new("math", ()).text("x squared");
        let style = TextStyle::default();
        for metrics in [
            MarkdownInlineMetrics::new(size(px(f32::NAN), px(10.)), px(8.)),
            MarkdownInlineMetrics::new(size(px(10.), px(0.)), px(0.)),
            MarkdownInlineMetrics::new(size(px(10.), px(10.)), px(11.)),
        ] {
            let extensions =
                MarkdownExtensions::default().inline_renderer("math", move |_, _, _, _| {
                    Some(MarkdownInlinePresentation::image(
                        Arc::new(gpui::Image::from_bytes(gpui::ImageFormat::Svg, Vec::new())),
                        metrics,
                    ))
                });
            window.update(|_, window, cx| {
                let measured =
                    MeasuredInlineObject::measure(&node, &extensions, &style, None, window, cx);
                assert!(measured.presentation.is_none());
                assert!(measured.metrics.is_valid());
                let zero = MeasuredInlineObject::measure(
                    &node,
                    &extensions,
                    &style,
                    Some(px(0.)),
                    window,
                    cx,
                );
                assert_eq!(zero.metrics.size, size(px(0.), px(0.)));
                assert_eq!(zero.metrics.baseline, px(0.));
            });
        }
    }

    #[test]
    fn failed_image_fallback_fits_the_whole_alternative_text() {
        use crate::text::inline::test_fonts::{BODY, WideMonoTextSystem};
        use gpui::{Empty, TestApp};
        let mut app = TestApp::with_text_system(Arc::new(WideMonoTextSystem));
        let mut window = app.open_window(|_, _| Empty);
        let extensions = MarkdownExtensions::default().inline_renderer("math", |_, _, _, _| {
            Some(MarkdownInlinePresentation::image(
                Arc::new(gpui::Image::from_bytes(
                    gpui::ImageFormat::Svg,
                    b"invalid SVG".to_vec(),
                )),
                MarkdownInlineMetrics::new(size(px(8.), px(10.)), px(8.)),
            ))
        });
        let text = "long formula alternative";
        let node = MarkdownNode::new("math", ()).text(text);
        let style = TextStyle {
            font_family: BODY.into(),
            font_size: px(16.).into(),
            ..Default::default()
        };
        window.update(|_, window, cx| {
            let measured =
                MeasuredInlineObject::measure(&node, &extensions, &style, None, window, cx);
            assert!(
                WideMonoTextSystem::width_of(text, BODY, measured.font_size)
                    <= measured.metrics.size.width + px(0.001)
            );
        });
    }

    #[test]
    fn inline_object_exposes_one_read_only_name_without_actions() {
        let node = MarkdownNode::new("math", ())
            .text("x²")
            .accessibility_label("x squared");
        let metrics = MarkdownInlineMetrics::new(size(px(30.), px(20.)), px(15.));
        let measured = MeasuredInlineObject {
            metrics,
            presentation: None,
            text: "x²".into(),
            font_size: px(16.),
            text_style: TextStyle::default(),
            appearance: Default::default(),
        };
        let object = InlineObject::new(
            "formula",
            node,
            measured,
            Arc::default(),
            Bounds::default(),
            Bounds::default(),
        );
        let mut accessible = gpui::accesskit::Node::new(Role::Unknown);
        assert_eq!(object.a11y_role(), Some(Role::Image));
        object.write_a11y_info(&mut accessible);
        assert_eq!(accessible.role(), Role::Image);
        assert_eq!(accessible.label(), Some("x squared"));
        assert!(accessible.is_read_only());
        assert!(!accessible.supports_action(gpui::accesskit::Action::Focus));
        assert!(!accessible.supports_action(gpui::accesskit::Action::Click));
    }
}

use super::*;
use gpui::{Context, InteractiveElement as _, Render, TestAppContext, div};

#[test]
fn test_form_builder() {
    for form in [Form::new(), Form::default(), Form::vertical()] {
        assert_eq!(form.props.layout, Axis::Vertical);
        assert_eq!(form.props.columns, 1);
        assert!(form.footer.is_none());
    }
    let form = Form::new()
        .columns(2)
        .label_layout(Axis::Horizontal)
        .child(Field::new())
        .footer(crate::button::Button::new("save").label("Save"));
    assert_eq!(form.props.layout, Axis::Horizontal);
    assert_eq!(form.props.columns, 2);
    assert_eq!(form.fields.len(), 1);
    assert!(form.footer.is_some());
}

struct FormHarness {
    width: f32,
    layout: Axis,
    legacy: bool,
    footer: Option<bool>,
}

impl Render for FormHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let form = if self.legacy {
            Form::vertical().layout(self.layout)
        } else {
            Form::new().label_layout(self.layout)
        };
        div()
            .debug_selector(|| "form-container".into())
            .w(px(self.width))
            .child(
                form.columns(2)
                    .label_width(px(80.))
                    .children((0..3).map(|ix| {
                        Field::new()
                            .label_fn(move |_, _| {
                                div()
                                    .debug_selector(move || format!("label-{ix}"))
                                    .w(px(40.))
                                    .h(px(20.))
                            })
                            .child(
                                div()
                                    .debug_selector(move || format!("control-{ix}"))
                                    .w_full()
                                    .h(px(24.)),
                            )
                    }))
                    .when_some(self.footer, |this, full_width| {
                        this.footer(
                            div()
                                .debug_selector(|| "footer-content".into())
                                .h(px(30.))
                                .when(full_width, |this| this.w_full())
                                .when(!full_width, |this| this.w(px(80.))),
                        )
                    }),
            )
    }
}

#[gpui::test]
fn label_layout_is_independent_of_field_columns(cx: &mut TestAppContext) {
    cx.update(crate::init);
    for width in [360., 800.] {
        for layout in [Axis::Horizontal, Axis::Vertical] {
            let (_, cx) = cx.add_window_view(move |_, _| FormHarness {
                width,
                layout,
                legacy: false,
                footer: None,
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            let first = cx.debug_bounds("control-0").unwrap();
            let second = cx.debug_bounds("control-1").unwrap();
            let third = cx.debug_bounds("control-2").unwrap();
            let label = cx.debug_bounds("label-0").unwrap();
            assert_eq!(first.top(), second.top());
            assert!(second.left() > first.right());
            assert_eq!(first.left(), third.left());
            assert!(third.top() > first.bottom());
            match layout {
                Axis::Horizontal => assert!(label.right() < first.left()),
                Axis::Vertical => {
                    assert_eq!(label.left(), first.left());
                    assert!(label.bottom() < first.top());
                }
            }
        }
    }
}

#[gpui::test]
fn footer_spans_columns_after_fields_and_aligns_actions_to_trailing_edge(cx: &mut TestAppContext) {
    cx.update(crate::init);
    for width in [360., 800.] {
        for layout in [Axis::Horizontal, Axis::Vertical] {
            for full_width in [true, false] {
                let (_, cx) = cx.add_window_view(move |_, _| FormHarness {
                    width,
                    layout,
                    legacy: false,
                    footer: Some(full_width),
                });
                cx.update(|window, cx| window.draw(cx).clear(cx));
                let form = cx.debug_bounds("form-container").unwrap();
                let footer = cx.debug_bounds("footer-content").unwrap();
                let last = cx.debug_bounds("control-2").unwrap();
                assert!(footer.top() > last.bottom());
                assert_eq!(footer.right(), form.right());
                assert!(footer.bottom() <= form.bottom());
                if full_width {
                    assert_eq!(footer.left(), form.left());
                    assert_eq!(footer.size.width, px(width));
                } else {
                    assert_eq!(footer.size.width, px(80.));
                }
            }
        }
    }
}

#[gpui::test]
fn default_without_footer_keeps_legacy_geometry(cx: &mut TestAppContext) {
    cx.update(crate::init);
    for layout in [Axis::Horizontal, Axis::Vertical] {
        let mut snapshots = Vec::new();
        for legacy in [true, false] {
            let (_, cx) = cx.add_window_view(move |_, _| FormHarness {
                width: 360.,
                layout,
                legacy,
                footer: None,
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            snapshots.push([
                cx.debug_bounds("form-container").unwrap(),
                cx.debug_bounds("label-0").unwrap(),
                cx.debug_bounds("control-0").unwrap(),
                cx.debug_bounds("control-1").unwrap(),
                cx.debug_bounds("control-2").unwrap(),
            ]);
            assert!(cx.debug_bounds("footer-content").is_none());
        }
        assert_eq!(snapshots[0], snapshots[1]);
    }
}

mod common;

use gpui_kit::component::radio::{Radio, RadioGroup};
use gpui_kit::test::{TestQueryExt, TestWindowExt};
use gpui_kit::{AppContext, Context, Role, TestAppContext, Window, div, prelude::*, px, size};

struct DeliveryPreferences {
    selected: [Option<usize>; 2],
    changes: Vec<(usize, usize)>,
    billing_disabled: bool,
}

impl Render for DeliveryPreferences {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().flex_col().gap_4().children(
            ["shipping", "billing"]
                .into_iter()
                .enumerate()
                .map(|(group, id)| {
                    RadioGroup::vertical(id)
                        .selected_index(self.selected[group])
                        .disabled(group == 1 && self.billing_disabled)
                        .child(Radio::new("standard").label("Standard"))
                        .child(
                            Radio::new("express")
                                .label("Express")
                                .accessibility_label("Express delivery"),
                        )
                        .child(Radio::new("overnight").label("Overnight").disabled(true))
                        .on_change(cx.listener(move |this, index, _, cx| {
                            this.selected[group] = Some(*index);
                            this.changes.push((group, *index));
                            cx.notify();
                        }))
                }),
        )
    }
}

#[gpui_kit::test]
fn scoped_radio_queries_follow_controlled_selection_without_changing_other_groups(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let (handle, view) = common::open_window(cx, Some(size(px(400.), px(400.))), |_, cx| {
        cx.new(|_| DeliveryPreferences {
            selected: [Some(0), Some(0)],
            changes: Vec::new(),
            billing_disabled: false,
        })
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find_all_by_role(Role::RadioButton).len(), 6);
        assert_eq!(window.find_all_by_label("Standard").len(), 2);
        assert!(window.try_find_by_label("Express").is_none());
        let before = window.within("shipping").find_by_label("Express delivery");
        assert_eq!(before.checked(), Some(false));
        assert_eq!(before.selected(), Some(false));
        window
            .within("shipping")
            .click(before.path().last().unwrap().clone(), cx);
        let selected = window.within("shipping").find_by_label("Express delivery");
        assert_eq!(selected.checked(), Some(true));
        assert_eq!(selected.selected(), Some(true));
        assert_eq!(selected.path(), before.path());
        assert_eq!(before.checked(), Some(false));
        assert_eq!(
            window
                .within("shipping")
                .find_by_label("Standard")
                .checked(),
            Some(false)
        );
        assert_eq!(
            window.within("billing").find_by_label("Standard").checked(),
            Some(true)
        );
        // Re-activating the current choice must not request another change.
        window
            .within("shipping")
            .click(selected.path().last().unwrap().clone(), cx);
    })
    .unwrap();
    view.read_with(cx, |view, _| {
        assert_eq!(view.selected, [Some(1), Some(0)]);
        assert_eq!(view.changes, vec![(0, 1)]);
    });
}

#[gpui_kit::test]
fn scoped_radio_activation_respects_item_and_group_disabled_states(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, view) = common::open_window(cx, Some(size(px(400.), px(400.))), |_, cx| {
        cx.new(|_| DeliveryPreferences {
            selected: [Some(0), Some(0)],
            changes: Vec::new(),
            billing_disabled: true,
        })
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        for (scope, label) in [("shipping", "Overnight"), ("billing", "Express delivery")] {
            let target = window.within(scope).find_by_label(label);
            window
                .within(scope)
                .click(target.path().last().unwrap().clone(), cx);
            assert_eq!(
                window.within(scope).find_by_label(label).checked(),
                Some(false)
            );
            assert_eq!(
                window.within(scope).find_by_label("Standard").checked(),
                Some(true)
            );
        }
    })
    .unwrap();
    view.read_with(cx, |view, _| {
        assert_eq!(view.selected, [Some(0), Some(0)]);
        assert!(view.changes.is_empty());
    });
    common::update_content(handle, &view, cx, |view, _, cx| {
        view.billing_disabled = false;
        cx.notify();
    })
    .unwrap();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let target = window.within("billing").find_by_label("Express delivery");
        window
            .within("billing")
            .click(target.path().last().unwrap().clone(), cx);
        assert_eq!(
            window
                .within("billing")
                .find_by_label("Express delivery")
                .checked(),
            Some(true)
        );
    })
    .unwrap();
    view.read_with(cx, |view, _| {
        assert_eq!(view.selected, [Some(0), Some(1)]);
        assert_eq!(view.changes, vec![(1, 1)]);
    });
}

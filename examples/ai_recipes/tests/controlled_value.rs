use gpui_kit::component::Root;
use gpui_kit::{AppContext as _, Modifiers, TestAppContext, point, px};
use gpui_kit_recipes::controlled_value::ControlledCheckbox;

#[gpui_kit::test]
fn requested_checkbox_value_updates_its_owner(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let mut controlled = None;
    let (_root, cx) = cx.add_window_view(|window, cx| {
        let view = cx.new(|_| ControlledCheckbox::new());
        controlled = Some(view.clone());
        Root::new(view, window, cx)
    });
    let controlled = controlled.unwrap();

    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.simulate_click(point(px(8.), px(8.)), Modifiers::default());
    controlled.read_with(cx, |view, _| assert!(view.is_checked()));
}

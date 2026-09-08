use gpui_kit::component::Root;
use gpui_kit::{AppContext as _, TestAppContext};
use gpui_kit_recipes::Settings;

#[gpui_kit::test]
fn typing_updates_the_owner_after_unrelated_redraws(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let mut settings = None;
    let (_root, cx) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| Settings::new(window, cx));
        settings = Some(view.clone());
        Root::new(view, window, cx)
    });
    let settings = settings.unwrap();
    cx.update(|window, cx| {
        let input = settings.read(cx).name.clone();
        input.update(cx, |input, cx| input.focus(window, cx));
    });
    cx.simulate_keystrokes("a");
    settings.read_with(cx, |view, _| {
        assert_eq!(view.preview.as_ref(), "a");
        assert_eq!(view.changes, 1);
    });
    settings.update(cx, |_, cx| cx.notify());
    cx.simulate_keystrokes("b");
    settings.read_with(cx, |view, _| {
        assert_eq!(view.preview.as_ref(), "ab");
        assert_eq!(view.changes, 2);
    });
}

use gpui::{Modifiers, TestAppContext, VisualTestContext, point, px};
use std::{
    fs,
    ops::Deref as _,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct TempApp(PathBuf);
impl TempApp {
    fn new(source: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "gpui-inline-tokens-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::write(path.join("main.js"), source).unwrap();
        Self(path)
    }
}
impl Drop for TempApp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[gpui::test]
fn inline_tokens_script_operations_and_click_reentry(cx: &mut TestAppContext) {
    cx.update(gpui_component_shell::init);
    let runtime = gpui_component_shell::new_isolated_runtime().unwrap();
    let app = TempApp::new(
        r#"
import { div, View } from "gpui-kit";
import { Input, InputState, Textarea, TextareaState } from "gpui-component";
import { Button as BaseButton, InputState as BaseInputState, TextareaState as BaseTextareaState } from "gpui-base";
function assert(value, message) { if (!value) throw new Error(message); }
function exercise(state) {
  state.set_value("🙂 @a!");
  state.replace_range_with_token({start: 3, end: 5}, {id: "a", text: "@a", label: "Alice"});
  const saved = state.content();
  assert(saved.tokens[0].range.start === 3, "UTF-16 range");
  let code = "";
  try { state.replace_range_with_token({start: 1, end: 2}, {id: "bad", text: "x"}); } catch (error) { code = error.code; }
  assert(code === "InvalidBoundary", "surrogate boundary must fail with code");
  assert(JSON.stringify(state.content()) === JSON.stringify(saved), "failure must be atomic");
  state.set_selected_range({start: 4, end: 5}); state.replace("");
  assert(state.value() === "🙂 !" && state.tokens().length === 0, "partial token deletion");
  state.set_value(saved);
  state.set_value(state.value());
  assert(state.tokens().length === 0, "explicit same value clears identity");
  state.set_value(saved);
  return state;
}
export default class TokenHost extends View {
  init() {
    this.input = exercise(InputState());
    this.textarea = exercise(TextareaState());
    this.child = exercise(InputState());
    this.base = exercise(BaseInputState.new());
    this.baseArea = exercise(BaseTextareaState.new());
    this.status = "verified";
  }
  render() {
    return div().relative().w(400).h(260)
      .child(new Input(this.input).w(350).aria_label("Token input")
        .token(token => div().w(80).h(20).child(token.token.label))
        .on_token_click((event, cx) => {
          assert(event.token.id === "a", "current identity");
          this.input.set_value("opened"); this.status = "clicked"; cx.notify();
        }))
      .child(new Textarea(this.textarea).w(350).h(60))
      .child(new Input(this.child).absolute().top(140).left(0).w(350)
        .token(token => div().flex().w(100).h(20).child(div().w(70).child(token.token.label))
          .child(BaseButton.new("remove-token-child").w(30).h(20).child("×")
            .on_mouse_down("left", (_event, cx) => cx.stop_propagation())
            .on_click((_event, cx) => { this.child.set_value("removed"); this.status = "child"; cx.stop_propagation(); cx.notify(); })))
        .on_token_click((_event, cx) => { this.status = "wrong body activation"; cx.notify(); }))
      .child(div().child(`${this.status}:${this.input.value()}:${this.input.tokens().length};child=${this.child.tokens().length}:${this.child.value()}`));
  }
}
"#,
    );
    let loaded = runtime.load_application(&app.0, "main.js").unwrap();
    let mounted = std::rc::Rc::new(std::cell::RefCell::new(None));
    let capture = mounted.clone();
    let window = cx.add_window(move |window, cx| {
        let view = runtime.mount_application(&loaded, window, cx).unwrap();
        *capture.borrow_mut() = Some(view.clone());
        gpui_component::Root::new(view, window, cx)
    });
    let view = mounted.borrow().clone().unwrap();
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let draw = |context: &mut VisualTestContext| {
        context.run_until_parked();
        context.update(|window, cx| window.draw(cx).clear(cx));
        context.update(|_, cx| {
            assert_eq!(view.read(cx).build_error(), None);
            view.read(cx).snapshot().unwrap().debug_tree()
        })
    };
    let first = draw(&mut context);
    assert!(first.contains("verified:🙂 @a!:1"), "{first}");
    assert!(
        draw(&mut context).contains("verified:🙂 @a!:1"),
        "rerender preserves identity"
    );
    context.simulate_click(point(px(65.), px(16.)), Modifiers::default());
    let result = draw(&mut context);
    assert!(result.contains("clicked:opened:0"), "{result}");
    // The custom child's callback survives the frame and consumes its own gesture.
    let child_bounds = context.update(|window, _| {
        gpui_base::test_support::find(window, &[], &gpui::ElementId::from("remove-token-child"))
            .expect("custom token child is laid out")
            .bounds()
    });
    context.simulate_click(child_bounds.center(), Modifiers::default());
    let result = draw(&mut context);
    assert!(result.contains("child:opened:0"), "{result}");
}

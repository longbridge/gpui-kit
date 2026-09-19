use std::{
    fs,
    ops::Deref as _,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use gpui::{TestAppContext, VisualTestContext};

static NEXT_APP: AtomicU64 = AtomicU64::new(0);

struct TempApp(PathBuf);

impl TempApp {
    fn new(source: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "gpui-component-shell-questionnaire-host-{}-{}",
            std::process::id(),
            NEXT_APP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("create temporary application directory");
        fs::write(path.join("main.js"), source).expect("write application entry");
        Self(path)
    }
}

impl Drop for TempApp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove temporary application directory");
    }
}

/// The script declares the questions; the answers, validation and navigation
/// are the native flow's. This drives it the way a person would — click a
/// choice, then Next — and reads the questionnaire's own progress back.
#[gpui::test]
fn questionnaire_answers_and_advances_from_script_declared_questions(cx: &mut TestAppContext) {
    cx.update(gpui_component_shell::init);
    let runtime = gpui_component_shell::new_isolated_runtime().expect("runtime");
    let app = TempApp::new(
        r##"
import { div, View } from "gpui-kit";
import { Questionnaire, QuestionnaireItem, QuestionnaireChoice } from "gpui-component";
export default class QuestionnaireHost extends View {
  render() {
    return div().w(500).h(400)
      .child(new Questionnaire("host-questionnaire").shortcuts("letters")
        .child(new QuestionnaireItem("direction", "Which direction?").required(true)
          .child(new QuestionnaireChoice("delegation", "Delegation"))
          .child(new QuestionnaireChoice("prompts", "Question prompts")))
        .child(new QuestionnaireItem("tools", "Which tools?").multiple(true)
          .child(new QuestionnaireChoice("editor", "Editor"))
          .child(new QuestionnaireChoice("terminal", "Terminal"))
          .child(new QuestionnaireChoice("browser", "Browser"))));
  }
}
"##,
    );
    let loaded = runtime.load_application(&app.0, "main.js").expect("load");
    let mounted = std::rc::Rc::new(std::cell::RefCell::new(None));
    let capture = mounted.clone();
    let window = cx.add_window(move |window, cx| {
        let view = runtime
            .mount_application(&loaded, window, cx)
            .expect("mount");
        *capture.borrow_mut() = Some(view.clone());
        gpui_component::Root::new(view, window, cx)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = mounted.borrow().clone().unwrap();
    let draw = |context: &mut VisualTestContext| {
        context.run_until_parked();
        context.update(|window, cx| window.draw(cx).clear(cx));
        context.update(|_, cx| {
            assert_eq!(view.read(cx).build_error(), None);
            view.read(cx).snapshot().unwrap().debug_tree()
        })
    };

    // Letter shortcuts are the native flow's, so a badge per enabled choice is
    // proof that the script's questions and `shortcuts` reached it. The first
    // question has two choices and the second three, which is what tells the
    // two apart on screen.
    draw(&mut context);
    let shortcut = |context: &mut VisualTestContext, key: &str| {
        context.debug_bounds(Box::leak(format!("kbd:{key}").into_boxed_str()))
    };
    assert!(shortcut(&mut context, "a").is_some());
    assert!(shortcut(&mut context, "b").is_some());
    assert!(
        shortcut(&mut context, "c").is_none(),
        "the second question must not be in the tree yet"
    );

    // Clicking the first choice answers it and takes focus, then Enter confirms
    // it — the whole navigation contract running inside a script-declared
    // questionnaire.
    let first_choice = shortcut(&mut context, "a").expect("first choice shortcut");
    context.simulate_click(first_choice.center(), gpui::Modifiers::default());
    draw(&mut context);
    context.simulate_keystrokes("enter");
    draw(&mut context);
    assert!(
        shortcut(&mut context, "c").is_some(),
        "Enter on a filled answer must advance to the three-choice question"
    );
}

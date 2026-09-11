use std::{ops::Deref as _, path::Path};

use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext, point, px};

struct ScriptRoot(Entity<gpui_shell::ScriptView>);

#[gpui::test]
fn input_group_comment_story_posts_once_and_cancels_the_next_draft(cx: &mut TestAppContext) {
    cx.update(gpui_component_shell::init);
    let runtime = gpui_component_shell::new_isolated_runtime().expect("runtime");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/js_story");
    let loaded = runtime
        .load_application(&root, "fixtures/input-group.js")
        .expect("load Input Group Story");
    let mounted = std::rc::Rc::new(std::cell::RefCell::new(None));
    let capture = mounted.clone();
    let window = cx.add_window(move |window, cx| {
        let view = runtime
            .mount_application(&loaded, window, cx)
            .expect("mount Input Group Story");
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
    let bounds = |context: &mut VisualTestContext, id: &'static str| {
        context.update(|window, _| {
            gpui_base::test_support::find(window, &[], &id.into())
                .expect("Story element")
                .bounds()
        })
    };
    draw(&mut context);
    let editor = bounds(&mut context, "ig-extra-comment");
    context.simulate_click(
        editor.origin + point(px(12.), px(12.)),
        Modifiers::default(),
    );
    context.simulate_input("你好🙂");
    let draft = draw(&mut context);
    assert!(draft.contains("Draft: 你好🙂"), "{draft}");
    let post = bounds(&mut context, "ig-extra-comment-post");
    context.simulate_click(post.center(), Modifiers::default());
    let posted = draw(&mut context);
    assert!(posted.contains("Posted: 你好🙂"), "{posted}");
    assert!(posted.contains("Draft: —"), "{posted}");

    context.simulate_click(
        editor.origin + point(px(12.), px(12.)),
        Modifiers::default(),
    );
    context.simulate_input("New draft");
    draw(&mut context);
    let cancel = bounds(&mut context, "ig-extra-comment-cancel");
    context.simulate_click(cancel.center(), Modifiers::default());
    let cancelled = draw(&mut context);
    assert!(cancelled.contains("Posted: 你好🙂"), "{cancelled}");
    assert!(cancelled.contains("Draft: —"), "{cancelled}");
    assert!(!cancelled.contains("New draft"), "{cancelled}");
}

impl gpui::Render for ScriptRoot {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        self.0.clone()
    }
}

#[gpui::test]
fn interactive_examples_keep_their_state_across_redraws(cx: &mut TestAppContext) {
    cx.update(gpui_component_shell::init);
    let runtime = gpui_component_shell::new_isolated_runtime().expect("runtime");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/js_story");
    let loaded = runtime
        .load_application(&root, "fixtures/interaction.js")
        .expect("load Story interaction fixture");
    let mounted = std::rc::Rc::new(std::cell::RefCell::new(None));
    let mounted_for_window = mounted.clone();
    let runtime_for_window = runtime.clone();
    let window = cx.add_window(move |window, cx| {
        let view = runtime_for_window
            .mount_application(&loaded, window, cx)
            .expect("mount Story interaction fixture");
        *mounted_for_window.borrow_mut() = Some(view.clone());
        ScriptRoot(view)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = mounted.borrow().clone().expect("mounted view");

    let draw = |context: &mut VisualTestContext| {
        context.update(|window, cx| window.draw(cx).clear(cx));
        context.update(|_, cx| {
            let view = view.read(cx);
            assert_eq!(view.build_error(), None);
            view.snapshot().expect("snapshot").debug_tree()
        })
    };

    let initial = draw(&mut context);
    assert!(initial.contains("compact:false"), "{initial}");
    assert!(initial.contains("preview:false"), "{initial}");

    context.simulate_click(point(px(110.), px(48.)), Modifiers::default());
    context.run_until_parked();
    let switched = draw(&mut context);
    assert!(switched.contains("compact:true"), "{switched}");

    context.simulate_click(point(px(90.), px(168.)), Modifiers::default());
    context.run_until_parked();
    let toggled = draw(&mut context);
    assert!(toggled.contains("preview:true"), "{toggled}");
}

#[gpui::test]
fn input_story_accepts_text_and_keeps_it_across_redraws(cx: &mut TestAppContext) {
    cx.update(gpui_component_shell::init);
    let runtime = gpui_component_shell::new_isolated_runtime().expect("runtime");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/js_story");
    let loaded = runtime
        .load_application(&root, "fixtures/input.js")
        .expect("load Input Story fixture");
    let mounted = std::rc::Rc::new(std::cell::RefCell::new(None));
    let mounted_for_window = mounted.clone();
    let runtime_for_window = runtime.clone();
    let window = cx.add_window(move |window, cx| {
        let view = runtime_for_window
            .mount_application(&loaded, window, cx)
            .expect("mount Input Story fixture");
        *mounted_for_window.borrow_mut() = Some(view.clone());
        ScriptRoot(view)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = mounted.borrow().clone().expect("mounted view");
    let draw = |context: &mut VisualTestContext| {
        context.update(|window, cx| window.draw(cx).clear(cx));
        context.update(|_, cx| {
            let view = view.read(cx);
            assert_eq!(view.build_error(), None);
            view.snapshot().expect("snapshot").debug_tree()
        })
    };

    draw(&mut context);
    context.simulate_click(point(px(30.), px(30.)), Modifiers::default());
    context.simulate_keystrokes("roadmap");
    context.run_until_parked();
    draw(&mut context);
    draw(&mut context);
}

#[gpui::test]
fn dock_story_materializes_real_panels_dock_and_tabs(cx: &mut TestAppContext) {
    cx.update(gpui_component_shell::init);
    let runtime = gpui_component_shell::new_isolated_runtime().expect("runtime");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/js_story");
    let loaded = runtime
        .load_application(&root, "fixtures/dock.js")
        .expect("load Dock Story fixture");
    let mounted = std::rc::Rc::new(std::cell::RefCell::new(None));
    let mounted_for_window = mounted.clone();
    let runtime_for_window = runtime.clone();
    let window = cx.add_window(move |window, cx| {
        let view = runtime_for_window
            .mount_application(&loaded, window, cx)
            .expect("mount Dock Story fixture");
        *mounted_for_window.borrow_mut() = Some(view.clone());
        ScriptRoot(view)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));

    let view = mounted.borrow().clone().expect("mounted view");
    context.update(|_, cx| {
        let view = view.read(cx);
        assert_eq!(view.build_error(), None);
        let tree = view.snapshot().expect("snapshot").debug_tree();
        assert!(tree.contains("dock_area"), "{tree}");
        assert!(tree.contains(":tab_bar(fn)"), "{tree}");
        assert!(tree.contains(":dock(fn)"), "{tree}");
    });
}

#[gpui::test]
fn every_registered_story_example_materializes(cx: &mut TestAppContext) {
    cx.update(gpui_component_shell::init);
    let runtime = gpui_component_shell::new_isolated_runtime().expect("runtime");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/js_story");
    let loaded = runtime
        .load_application(&root, "fixtures/all-examples.js")
        .expect("load all Story examples fixture");
    let mounted = std::rc::Rc::new(std::cell::RefCell::new(None));
    let mounted_for_window = mounted.clone();
    let runtime_for_window = runtime.clone();
    let window = cx.add_window(move |window, cx| {
        let view = runtime_for_window
            .mount_application(&loaded, window, cx)
            .expect("mount all Story examples fixture");
        *mounted_for_window.borrow_mut() = Some(view.clone());
        ScriptRoot(view)
    });
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    context.update(|window, cx| window.draw(cx).clear(cx));
    context.run_until_parked();
    context.update(|window, cx| window.draw(cx).clear(cx));

    let view = mounted.borrow().clone().expect("mounted view");
    context.update(|_, cx| {
        assert_eq!(view.read(cx).build_error(), None);
        assert!(view.read(cx).snapshot().is_some());
    });
}

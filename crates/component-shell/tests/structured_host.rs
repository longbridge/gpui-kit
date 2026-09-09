use std::{
    fs,
    ops::Deref,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use gpui::{Entity, TestAppContext, VisualTestContext};

static NEXT_APP: AtomicU64 = AtomicU64::new(0);

struct TempApp(PathBuf);

impl TempApp {
    fn new(source: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "gpui-component-shell-structured-host-{}-{}",
            std::process::id(),
            NEXT_APP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("create temporary application directory");
        fs::write(path.join("main.js"), source).expect("write temporary application entry");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempApp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove temporary application directory");
    }
}

struct Empty;

impl gpui_shell::gpui::Render for Empty {
    fn render(
        &mut self,
        _: &mut gpui_shell::gpui::Window,
        _: &mut gpui_shell::gpui::Context<Self>,
    ) -> impl gpui_shell::gpui::IntoElement {
        gpui_shell::gpui::div()
    }
}

fn mount(
    cx: &mut TestAppContext,
    source: &str,
) -> (VisualTestContext, Entity<gpui_shell::ScriptView>) {
    cx.update(|cx| {
        gpui_component_shell::init(cx);
    });
    let app = TempApp::new(source);
    let runtime = gpui_component_shell::new_isolated_runtime().expect("runtime");
    let loaded = runtime
        .load_application(app.path(), "main.js")
        .expect("load application");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.mount_application(&loaded, window, cx))
        .expect("mount application");
    (context, view)
}

fn position(tree: &str, needle: &str) -> usize {
    tree.find(needle)
        .unwrap_or_else(|| panic!("missing `{needle}` in snapshot:\n{tree}"))
}

#[gpui_shell::gpui::test]
fn structured_components_materialize_nested_children_in_script_order(cx: &mut TestAppContext) {
    let source = r#"
import { View, div } from "gpui-kit";
import {
  DescriptionItem, DescriptionList, Field, HForm,
  Table, TableBody, TableCaption, TableHeader,
} from "gpui-component";

export default class StructuredApp extends View {
  render() {
    return div()
      .child(new DescriptionList()
        .bordered(false).columns(2).vertical().p(2)
        .child(new DescriptionItem("First label").value("First value").span(1))
        .child(new DescriptionItem("Second label").value("Second value")))
      .child(new HForm()
        .columns(2).label_width(120)
        .child(new Field().label("Name").required(true).child(div().child("Ada")))
        .child(new Field().label("Role").child(div().child("Admin"))))
      .child(new Table()
          .accessibility_label("People")
          .size("small")
          .child(new TableCaption().child("Current people"))
          .child(new TableHeader())
          .child(new TableBody()));
  }
}
"#;
    let (mut context, view) = mount(cx, source);

    context.draw(
        gpui_shell::gpui::Point::default(),
        gpui_shell::gpui::size(gpui_shell::gpui::px(800.), gpui_shell::gpui::px(600.)),
        {
            let view = view.clone();
            move |_, _| gpui::IntoElement::into_any_element(view)
        },
    );

    let tree = context.update(|_, cx| {
        let view = view.read(cx);
        assert_eq!(view.build_error(), None);
        view.snapshot()
            .expect("structured render snapshot")
            .debug_tree()
    });

    for expected in [
        "DescriptionList",
        ":bordered(registered)",
        ":columns(registered)",
        ":vertical(registered)",
        ".p[Number(2.0)]",
        "DescriptionItem",
        "Form",
        ":label_width(registered)",
        "Field",
        "Table",
        ":accessibility_label(registered)",
        "TableHeader",
        "TableBody",
        "TableCaption",
    ] {
        assert!(tree.contains(expected), "missing `{expected}`:\n{tree}");
    }

    assert!(position(&tree, ":bordered(registered)") < position(&tree, ":columns(registered)"));
    assert!(position(&tree, ":columns(registered)") < position(&tree, ":vertical(registered)"));
    assert_eq!(tree.matches("DescriptionItem").count(), 2, "{tree}");
    assert!(position(&tree, "Ada") < position(&tree, "Admin"));
    assert!(position(&tree, "TableHeader") < position(&tree, "TableBody"));
    assert!(position(&tree, "TableCaption") < position(&tree, "TableHeader"));
}

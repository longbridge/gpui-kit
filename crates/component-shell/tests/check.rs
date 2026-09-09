use std::{
    fs,
    path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

use gpui::AppContext as _;

static NEXT_APP: AtomicU64 = AtomicU64::new(0);

struct CheckApp(PathBuf);

impl CheckApp {
    fn new(source: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "gpui-component-shell-check-{}-{}",
            std::process::id(),
            NEXT_APP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::write(path.join("main.js"), source).unwrap();
        Self(path)
    }

    fn run(&self) -> (ExitStatus, String, String) {
        let stdout = self.0.join("stdout");
        let stderr = self.0.join("stderr");
        let mut child = CheckChild(
            Command::new(env!("CARGO_BIN_EXE_gpui-component-shell"))
                .env("XDG_DATA_HOME", self.0.join("data"))
                .arg("check")
                .arg(&self.0)
                .arg("--print-spec")
                .stdout(Stdio::from(fs::File::create(&stdout).unwrap()))
                .stderr(Stdio::from(fs::File::create(&stderr).unwrap()))
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(30);
        let status = loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                break status;
            }
            assert!(
                Instant::now() < deadline,
                "check did not exit within 30 seconds: {}",
                fs::read_to_string(&stderr).unwrap()
            );
            thread::sleep(Duration::from_millis(20));
        };
        (
            status,
            fs::read_to_string(stdout).unwrap(),
            fs::read_to_string(stderr).unwrap(),
        )
    }
}

impl Drop for CheckApp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

struct CheckChild(Child);

impl Drop for CheckChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn check_materializes_valid_typed_children_and_preserves_print_spec() {
    let app = CheckApp::new(
        r#"
        import { View, div } from "gpui-kit";
        import { HForm, Field } from "gpui-component";
        export default class App extends View {
            render() {
                return new HForm().child(new Field().label("Name").child(div().child("Ada")));
            }
        }
        "#,
    );
    let (status, stdout, stderr) = app.run();
    assert!(status.success(), "{stdout}\n{stderr}");
    assert!(
        stdout.contains("Field") && stdout.contains("Ada"),
        "{stdout}"
    );
    assert!(stdout.contains("check passed:"), "{stdout}");
}

#[test]
fn check_rejects_an_ordinary_child_in_a_typed_form() {
    let app = CheckApp::new(
        r#"
        import { View, div } from "gpui-kit";
        import { HForm } from "gpui-component";
        export default class App extends View {
            render() { return div().child(new HForm().child(div())); }
        }
        "#,
    );
    let (status, stdout, stderr) = app.run();
    assert_eq!(status.code(), Some(1), "{stdout}\n{stderr}");
    assert!(stderr.contains("failed to materialize `Form`"), "{stderr}");
    assert!(
        stderr.contains("Field children; received an ordinary element"),
        "{stderr}"
    );
    assert!(!stdout.contains("check passed:"), "{stdout}");
}

#[test]
fn check_rejects_style_on_a_data_only_component() {
    let app = CheckApp::new(
        r#"
        import { View, div } from "gpui-kit";
        import { MenuItem } from "gpui-component";
        export default class App extends View {
            render() { return div().child(new MenuItem("Open", "open").p(2)); }
        }
        "#,
    );
    let (status, stdout, stderr) = app.run();
    assert_eq!(status.code(), Some(1), "{stdout}\n{stderr}");
    assert!(
        stderr.contains("failed to materialize `MenuItem`"),
        "{stderr}"
    );
    assert!(stderr.contains("does not implement Styled"), "{stderr}");
    assert!(!stdout.contains("check passed:"), "{stdout}");
}

#[test]
fn check_reports_load_and_render_failures_without_hanging() {
    for (source, expected) in [
        ("this is not javascript", "main.js"),
        (
            r#"import { View } from "gpui-kit";
            export default class App extends View {
                render() { throw new Error("render failed deliberately"); }
            }"#,
            "render failed deliberately",
        ),
        (
            r#"import { View } from "gpui-kit";
            export default class App extends View {
                render() { while (true) {} }
            }"#,
            "interrupted",
        ),
    ] {
        let (status, stdout, stderr) = CheckApp::new(source).run();
        assert_eq!(status.code(), Some(1), "{stdout}\n{stderr}");
        assert!(stderr.contains(expected), "{stderr}");
        assert!(!stdout.contains("check passed:"), "{stdout}");
    }
}

#[test]
fn check_reports_invalid_metadata_before_opening_a_window() {
    let app = CheckApp::new("export default 1;");
    fs::write(app.0.join("gpui-shell.json"), "{}").unwrap();
    let (status, stdout, stderr) = app.run();
    assert_eq!(status.code(), Some(1), "{stdout}\n{stderr}");
    assert!(stderr.contains("gpui-shell.json"), "{stderr}");
    assert!(stderr.contains("check failed:"), "{stderr}");
    assert!(!stdout.contains("check passed:"), "{stdout}");
}

#[gpui::test]
fn runtime_check_preserves_errors_and_clears_them_before_the_next_check(
    cx: &mut gpui::TestAppContext,
) {
    use std::ops::Deref as _;

    cx.update(gpui_component_shell::init);
    let runtime = gpui_component_shell::new_isolated_runtime().unwrap();
    let window = cx.add_window(|window, cx| {
        let empty = cx.new(|_| gpui::Empty);
        gpui_component::Root::new(empty, window, cx)
    });
    let mut context = gpui::VisualTestContext::from_window(*window.deref(), cx);
    let invalid = CheckApp::new(
        r#"import { View, div } from "gpui-kit";
        import { HForm } from "gpui-component";
        export default class App extends View {
            render() { return new HForm().child(div()); }
        }"#,
    );
    let error = context
        .update(|window, cx| runtime.check(&invalid.0, window, cx))
        .expect_err("the Rust facade must return materialization failures");
    assert!(format!("{error:#}").contains("Form accepts only registered Field children"));

    let valid = CheckApp::new(
        r#"import { View } from "gpui-kit";
        import { HForm, Field } from "gpui-component";
        export default class App extends View {
            render() {
                if (this.rendered) throw new Error("check rendered twice");
                this.rendered = true;
                return new HForm().child(new Field().child("checked once"));
            }
        }"#,
    );
    let description = context
        .update(|window, cx| runtime.check(&valid.0, window, cx))
        .expect("the next check must not inherit the previous materialization error");
    assert!(description.contains("checked once"), "{description}");
}

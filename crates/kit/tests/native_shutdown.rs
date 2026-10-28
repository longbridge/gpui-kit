//! Exercise actual native application teardown in bounded child processes.
//! Run with `cargo test -p gpui-kit --no-default-features --test native_shutdown`.
//! A desktop session is required; no TestAppContext or test scheduler is used.

use gpui_kit::{
    AppContext as _, Context, Entity, IntoElement, QuitMode, Render, Window, WindowOptions,
    base::text::{TextView, TextViewState},
};
use std::{
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

/// Retains a real document until its native window is closed.
struct Document {
    /// Text whose retained layouts and cleanup task participate in teardown.
    state: Entity<TextViewState>,
}

impl Render for Document {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        TextView::new(&self.state)
    }
}

/// Runs one native lifecycle selected by `case`, returning only after App drops.
fn run_application(case: &str) {
    let initialize = case != "platform-only";
    let open_document = case == "close-text-window";
    gpui_kit::application()
        .with_quit_mode(QuitMode::Explicit)
        .run(move |cx| {
            if initialize {
                gpui_kit::init(cx);
            }
            let window = open_document.then(|| {
                gpui_kit::open_window(
                    WindowOptions {
                        show: false,
                        focus: false,
                        ..Default::default()
                    },
                    cx,
                    |_, cx| {
                        cx.new(|cx| Document {
                            state: cx.new(|cx| {
                                TextViewState::markdown("A paragraph retained until shutdown.", cx)
                            }),
                        })
                    },
                )
                .expect("open native text window")
                .0
            });
            cx.spawn(async move |cx| {
                // Poll initialization tasks before the ordinary quit request.
                cx.background_executor()
                    .timer(Duration::from_millis(150))
                    .await;
                if let Some(window) = window {
                    window
                        .update(cx, |_, window, cx| {
                            // Hidden windows still shape and retain real text.
                            for _ in 0..2 {
                                window.draw(cx).clear(cx);
                            }
                            window.remove_window();
                        })
                        .expect("draw and close native text window");
                    eprintln!("text window closed");
                    cx.background_executor()
                        .timer(Duration::from_millis(150))
                        .await;
                }
                eprintln!("requesting quit");
                cx.update(|cx| cx.quit());
            })
            .detach();
        });
    eprintln!("application returned normally");
}

/// Runs `case` with a deadline, reaping a hung child and returning its diagnostics.
fn check_shutdown(case: &str) -> Result<Duration, String> {
    let started = Instant::now();
    let mut child = Command::new(std::env::current_exe().expect("test executable"))
        .args(["--native-shutdown-case", case])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start native shutdown child");
    let timed_out = loop {
        if child
            .try_wait()
            .expect("poll native shutdown child")
            .is_some()
        {
            break false;
        }
        if started.elapsed() >= Duration::from_secs(8) {
            child.kill().expect("stop hung native shutdown child");
            break true;
        }
        thread::sleep(Duration::from_millis(20));
    };
    let output = child
        .wait_with_output()
        .expect("reap native shutdown child");
    let stderr = String::from_utf8_lossy(&output.stderr);
    if timed_out
        || !output.status.success()
        || !stderr.contains("requesting quit")
        || !stderr.contains("application returned normally")
        || (case == "close-text-window" && !stderr.contains("text window closed"))
    {
        return Err(format!(
            "timeout={timed_out}, status={}\n{}\n{stderr}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
        ));
    }
    Ok(started.elapsed())
}

/// Dispatches child mode or repeats all controls against the same executable.
fn main() {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("--native-shutdown-case") {
        let case = args.next().expect("native shutdown case");
        assert!(["platform-only", "initialized", "close-text-window"].contains(&case.as_str()));
        run_application(&case);
        return;
    }

    let mut failures = Vec::new();
    for round in 1..=3 {
        for case in ["platform-only", "initialized", "close-text-window"] {
            match check_shutdown(case) {
                Ok(elapsed) => println!("PASS {case} round {round}: {elapsed:.3?}"),
                Err(error) => {
                    eprintln!("FAIL {case} round {round}: {error}");
                    failures.push((case, round));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "native shutdown failures: {failures:?}"
    );
}

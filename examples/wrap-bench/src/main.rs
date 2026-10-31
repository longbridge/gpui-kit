//! Usage: `cargo run --release -p example-wrap-bench -- <file> [--no-wrap] [--language <name>]`
//!
//! Opens an editor configured like a read-only response viewer, loads `<file>` once the window
//! has painted, and prints how long `set_value` took and how long it took until the next frame
//! had been painted. It then times scrolling and replacing the document with an empty one, and
//! quits, so runs can be scripted.

use std::time::Instant;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::input::{Editor, EditorState, TabSize};
use gpui_kit::*;

struct Options {
    path: String,
    soft_wrap: bool,
    language: String,
}

impl Options {
    fn parse() -> Self {
        let mut args = std::env::args().skip(1);
        let mut path = None;
        let mut soft_wrap = true;
        let mut language = "json".to_string();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--no-wrap" => soft_wrap = false,
                "--language" => language = args.next().expect("--language needs a value"),
                _ => path = Some(arg),
            }
        }
        Self {
            path: path.expect("usage: wrap-bench <file> [--no-wrap] [--language <name>]"),
            soft_wrap,
            language,
        }
    }
}

struct Bench {
    editor: Entity<EditorState>,
}

impl Bench {
    fn new(options: Options, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language(options.language.clone())
                .line_number(true)
                .tab_size(TabSize {
                    tab_size: 2,
                    hard_tabs: false,
                })
                .soft_wrap(options.soft_wrap)
                .default_value("")
        });

        let text = std::fs::read_to_string(&options.path).expect("read input file");
        let editor_handle = editor.clone();
        // Load after the empty editor has painted, so the wrap width is known, as it is when a
        // response arrives in an open window.
        window.on_next_frame(move |window, cx| {
            window.on_next_frame(move |window, cx| {
                load(editor_handle, text, window, cx);
            });
            window.refresh();
            let _ = cx;
        });

        Self { editor }
    }
}

/// Frames scrolled through after the load, to time redrawing a loaded document.
const SCROLL_FRAMES: usize = 20;

/// How long to wait after scrolling for the background syntax parse to finish.
const PARSE_SETTLE: std::time::Duration = std::time::Duration::from_secs(3);

/// Scroll down one screen per frame and report the average frame time.
fn scroll_frames(
    editor: Entity<EditorState>,
    frame: usize,
    started: Instant,
    window: &mut Window,
    cx: &mut App,
) {
    if frame == SCROLL_FRAMES {
        let average = started.elapsed().as_secs_f64() * 1000. / SCROLL_FRAMES as f64;
        println!("scroll_frame_ms={average:.1}");
        // Give the background parse time to finish, so clearing also drops a full syntax tree.
        let window_handle = window.window_handle();
        cx.spawn(async move |cx| {
            cx.background_executor().timer(PARSE_SETTLE).await;
            let _ = window_handle.update(cx, |_, window, cx| {
                let started = Instant::now();
                editor.update(cx, |state, cx| state.set_value("", window, cx));
                println!("clear_ms={:.1}", started.elapsed().as_secs_f64() * 1000.);
                cx.quit();
            });
        })
        .detach();
        return;
    }
    editor.update(cx, |state, cx| {
        let offset = state.scroll_offset();
        state.set_scroll_offset(point(offset.x, offset.y - px(700.)), cx);
    });
    window.refresh();
    window.on_next_frame(move |window, cx| scroll_frames(editor, frame + 1, started, window, cx));
}

fn load(editor: Entity<EditorState>, text: String, window: &mut Window, cx: &mut App) {
    let bytes = text.len();
    let started = Instant::now();
    editor.update(cx, |state, cx| state.set_value(text, window, cx));
    let editor = editor.clone();
    let set_value = started.elapsed();
    window.refresh();
    window.on_next_frame(move |window, _| {
        // This frame is the one that lays out and paints the new document. The next callback
        // runs once it has been presented.
        window.refresh();
        window.on_next_frame(move |window, cx| {
            let first_frame = started.elapsed();
            println!(
                "bytes={bytes} set_value_ms={:.1} first_frame_ms={:.1}",
                set_value.as_secs_f64() * 1000.,
                first_frame.as_secs_f64() * 1000.
            );
            scroll_frames(editor, 0, Instant::now(), window, cx);
        });
    });
}

impl Render for Bench {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            Editor::new(&self.editor)
                .h_full()
                .bordered(false)
                .font_family(cx.theme().mono_font_family.clone())
                .text_size(cx.theme().mono_font_size),
        )
    }
}

fn main() {
    let options = Options::parse();
    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        cx.activate(true);
        let bounds = Bounds::centered(None, size(px(1200.), px(800.)), cx);
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            cx,
            |window, cx| cx.new(|cx| Bench::new(options, window, cx)),
        )
        .expect("Failed to open window");
    });
}

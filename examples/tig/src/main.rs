mod app;
mod repository;

use std::path::PathBuf;

use gpui_kit::{AppContext as _, WindowBounds, WindowOptions, component::TitleBar, px, size};

use app::Tig;
use repository::Repository;

fn main() {
    let mut path = None;
    let mut check = false;
    for argument in std::env::args_os().skip(1) {
        if argument == "--help" {
            println!(
                "Usage: example-tig [--check] [repository]\nDefaults to the current directory. Requires Git on PATH."
            );
            return;
        } else if argument == "--check" {
            check = true;
        } else if path.is_none() {
            path = Some(PathBuf::from(argument));
        } else {
            eprintln!("Usage: example-tig [--check] [repository]");
            std::process::exit(2);
        }
    }
    let repository = Repository::new(path.unwrap_or_else(|| PathBuf::from(".")));
    if check {
        let result = repository.history().and_then(|commits| {
            println!(
                "{}: {} commits in the history window",
                repository.label(),
                commits.len()
            );
            if let Some(commit) = commits.first() {
                let details = repository.details(&commit.hash)?;
                println!(
                    "{} {}: {} changed files",
                    commit.short_hash,
                    commit.subject,
                    details.files.len()
                );
            }
            Ok(())
        });
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }

    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            app::init(cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::centered(size(px(1400.), px(860.)), cx)),
                window_min_size: Some(size(px(1100.), px(640.))),
                app_id: Some("gpui-kit-tig".into()),
                ..TitleBar::window_options()
            };
            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| Tig::new(repository, window, cx))
            })
            .expect("Failed to open Git history window");
            cx.activate(true);
        });
}

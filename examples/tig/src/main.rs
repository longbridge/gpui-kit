mod app;
mod repository;

use std::path::PathBuf;

use gpui_kit::{
    AppContext as _, SharedString, WindowBounds, WindowOptions, component::TitleBar, px, size,
};

use app::Tig;
use repository::Repository;

fn main() {
    let mut path = None;
    let mut check = false;
    let mut commit = None;
    let mut file = None;
    let mut line = None;
    let mut arguments = std::env::args_os().skip(1);
    const USAGE: &str =
        "Usage: example-tig [--check] [--commit HASH] [--file PATH] [--line N] [repository]";
    while let Some(argument) = arguments.next() {
        if argument == "--help" {
            println!("{USAGE}\nDefaults to the current directory and HEAD. Requires Git on PATH.");
            return;
        } else if argument == "--check" {
            check = true;
        } else if argument == "--commit" || argument == "--file" || argument == "--line" {
            let Some(value) = arguments.next().and_then(|value| value.into_string().ok()) else {
                eprintln!("{} requires a value.\n{USAGE}", argument.to_string_lossy());
                std::process::exit(2);
            };
            if argument == "--commit" {
                commit = Some(SharedString::from(value));
            } else if argument == "--file" {
                file = Some(SharedString::from(value));
            } else {
                line = value.parse::<usize>().ok().filter(|line| *line > 0);
                if line.is_none() {
                    eprintln!("Line must be a positive integer.");
                    std::process::exit(2);
                }
            }
        } else if !argument.to_string_lossy().starts_with('-') && path.is_none() {
            path = Some(PathBuf::from(argument));
        } else {
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    }
    if line.is_some() && file.is_none() {
        eprintln!("--line requires --file.");
        std::process::exit(2);
    }
    let mut repository = Repository::new(path.unwrap_or_else(|| PathBuf::from(".")));
    if let Some(commit) = commit {
        repository = repository.with_revision(commit).unwrap_or_else(|error| {
            eprintln!("{error}");
            std::process::exit(2);
        });
    }
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
                cx.new(|cx| Tig::new(repository, window, cx).with_initial_file(file, line))
            })
            .expect("Failed to open Git history window");
            cx.activate(true);
        });
}

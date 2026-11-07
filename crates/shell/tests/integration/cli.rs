#![cfg(feature = "components")]
use std::{path::PathBuf, process::Command};

fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_gpui-shell"))
}

fn temporary_directory(label: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("gpui-shell-cli-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

#[test]
fn help_names_the_command() {
    let output = command().arg("--help").output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Usage: gpui-shell "), "{stdout}");
}

#[test]
fn invalid_arguments_exit_two_and_point_at_help() {
    let output = command().arg("--unknown").output().unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("gpui-shell:"), "{stderr}");
    assert!(stderr.contains("Try `gpui-shell --help`"), "{stderr}");
}

#[test]
fn types_uses_the_component_catalog() {
    let directory = temporary_directory("types");
    let output = command()
        .args(["types", directory.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let declarations = std::fs::read_to_string(directory.join("gpui-kit.d.ts")).unwrap();
    assert!(
        declarations.contains("export const Spinner:"),
        "{declarations}"
    );
    assert!(
        declarations.contains("export const Accordion:"),
        "{declarations}"
    );

    std::fs::remove_dir_all(directory).unwrap();
}

//! `new`: a blank application, ready to run.
//!
//! The files are the ones every application ends up with anyway — a manifest
//! with a stable `id`, an entry module, editor configuration, and the
//! instructions for a coding agent — written once so the first `run` works and
//! the first edit is to the application rather than to its plumbing. The entry
//! uses the component catalog when this runtime has one, and `gpui-base`
//! alone when it does not, so it always runs on the binary that created it.

use std::path::{Path, PathBuf};

use super::{HostBrand, agents};
use crate::plugin::MANIFEST_FILE;

/// Creates the application in `directory`, which must not exist or be empty.
pub(super) fn create(
    directory: &Path,
    brand: HostBrand,
    components: &crate::FrozenComponentRegistry,
) -> Result<PathBuf, String> {
    let occupied = std::fs::read_dir(directory)
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(false);
    if occupied {
        return Err(format!(
            "`{}` already exists and is not empty; `new` only creates an application",
            directory.display()
        ));
    }
    std::fs::create_dir_all(directory)
        .map_err(|error| format!("cannot create `{}`: {error}", directory.display()))?;
    let directory = directory
        .canonicalize()
        .map_err(|error| format!("cannot read `{}`: {error}", directory.display()))?;

    let folder = directory
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let styled = components.module_specifier().is_some();

    let write = |name: &str, contents: String| {
        std::fs::write(directory.join(name), contents)
            .map_err(|error| format!("cannot write {name}: {error}"))
    };
    write(MANIFEST_FILE, manifest(&folder, brand))?;
    write("main.js", entry(&display_name(&folder), styled))?;
    // `dist/` is where `bundle` writes by default.
    write(
        ".gitignore",
        "gpui-kit.d.ts\nnode_modules/\ndist/\n".to_owned(),
    )?;

    // The declarations and `jsconfig.json` come from the runtime that will run
    // the application, and the agent instructions name its modules.
    crate::write_type_declarations_with_components(&directory, components)
        .map_err(|error| format!("cannot write type declarations: {error}"))?;
    agents::write(&directory, brand, components)?;

    Ok(directory)
}

/// A manifest `id` from a directory name: lower case, with everything the
/// manifest refuses folded to `-`, under a `local.` prefix that says nobody
/// has published it yet.
fn application_id(folder: &str) -> String {
    let mut slug = String::new();
    for character in folder.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_lowercase() || character.is_ascii_digit() {
            slug.push(character);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    format!("local.{}", if slug.is_empty() { "app" } else { slug })
}

/// `my_clock` and `my-clock` read as "My Clock" in a title bar.
fn display_name(folder: &str) -> String {
    let words: Vec<String> = folder
        .split(['-', '_', ' ', '.'])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut characters = word.chars();
            characters
                .next()
                .map(|first| first.to_uppercase().chain(characters).collect())
                .unwrap_or_default()
        })
        .collect();
    if words.is_empty() {
        "App".to_owned()
    } else {
        words.join(" ")
    }
}

fn manifest(folder: &str, brand: HostBrand) -> String {
    let document = serde_json::json!({
        "id": application_id(folder),
        "name": display_name(folder),
        "version": "0.1.0",
        "shell-version": brand.version(),
        "entry": "main.js",
        "capabilities": { "storage": true },
    });
    let mut text = serde_json::to_string_pretty(&document).unwrap_or_default();
    text.push('\n');
    text
}

fn entry(name: &str, styled: bool) -> String {
    let title = serde_json::to_string(name).unwrap_or_else(|_| "\"App\"".to_owned());
    if styled {
        format!(
            r#"import {{ View, div }} from "gpui-kit";
import {{ v_flex }} from "gpui-base";
import {{ Button, TitleBar }} from "gpui-component";

export default class App extends View {{
  init() {{
    this.count = 0;
  }}

  render(cx) {{
    return v_flex()
      .size_full()
      .bg(cx.theme().colors.background)
      .child(
        TitleBar.new().child(
          div().text_size(13).font_medium().text_color(cx.theme().colors.foreground).child({title}),
        ),
      )
      .child(
        v_flex()
          .flex_1()
          .items_center()
          .justify_center()
          .gap(12)
          .child(div().text_color(cx.theme().colors.foreground).child(`Clicked ${{this.count}} times`))
          .child(
            Button.new("increment").primary().label("Click me").on_click((_event, cx) => {{
              this.count += 1;
              cx.notify();
            }}),
          ),
      );
  }}
}}
"#
        )
    } else {
        format!(
            r#"import {{ View, div }} from "gpui-kit";
import {{ v_flex, Button }} from "gpui-base";

// {title}
export default class App extends View {{
  init() {{
    this.count = 0;
  }}

  render(cx) {{
    return v_flex()
      .size_full()
      .items_center()
      .justify_center()
      .gap(12)
      .bg(cx.theme().colors.background)
      .child(div().text_color(cx.theme().colors.foreground).child(`Clicked ${{this.count}} times`))
      .child(
        Button.new("increment")
          .h(28)
          .px(12)
          .items_center()
          .justify_center()
          .rounded(6)
          .bg(cx.theme().colors.primary)
          .text_color(cx.theme().colors.primary_foreground)
          .on_click((_event, cx) => {{
            this.count += 1;
            cx.notify();
          }})
          .child("Click me"),
      );
  }}
}}
"#
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_directory_name_becomes_a_valid_id_and_a_readable_name() {
        assert_eq!(application_id("clock"), "local.clock");
        assert_eq!(application_id("My Clock_2"), "local.my-clock-2");
        assert_eq!(application_id("--"), "local.app");
        assert_eq!(display_name("my-clock"), "My Clock");
        assert_eq!(display_name("notes_app"), "Notes App");

        // The id must pass the manifest's own validation.
        let manifest = manifest("My Clock_2", HostBrand::new("gpui-shell", "0.7.1"));
        crate::plugin::PluginManifest::parse(&manifest).expect("a valid manifest");
    }

    #[test]
    fn an_existing_application_is_never_overwritten() {
        let directory = std::env::temp_dir().join(format!("gpui-shell-new-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("main.js"), "// mine").unwrap();

        let components = crate::FrozenComponentRegistry::default();
        let brand = HostBrand::new("gpui-shell", "0.7.1");
        let error = create(&directory, brand, &components).expect_err("occupied");
        assert!(error.contains("not empty"), "{error}");
        assert_eq!(
            std::fs::read_to_string(directory.join("main.js")).unwrap(),
            "// mine"
        );

        let _ = std::fs::remove_dir_all(&directory);
    }
}

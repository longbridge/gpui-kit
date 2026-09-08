use std::{collections::BTreeMap, env, fmt::Write, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=assets/icons");
    println!("cargo:rerun-if-changed=build.rs");
    let mut icons = BTreeMap::new();
    for entry in fs::read_dir("assets/icons").expect("bundled icons directory") {
        let path = entry.expect("read icon directory entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("svg") {
            continue;
        }
        let stem = path.file_stem().unwrap().to_str().expect("UTF-8 icon name");
        let variant: String = stem
            .split(['-', '_', '.'])
            .filter(|part| !part.is_empty())
            .map(|part| {
                let mut chars = part.chars();
                let first = chars.next().unwrap();
                format!(
                    "{}{}",
                    first.to_ascii_uppercase(),
                    chars.as_str().to_ascii_lowercase()
                )
            })
            .collect();
        assert!(
            variant.starts_with(|c: char| c.is_ascii_alphabetic()),
            "invalid icon name: {stem}"
        );
        assert!(
            variant.chars().all(|c| c.is_ascii_alphanumeric()),
            "invalid icon name: {stem}"
        );
        assert!(
            icons.insert(variant, format!("icons/{stem}.svg")).is_none(),
            "duplicate icon variant: {stem}"
        );
    }
    assert!(!icons.is_empty(), "bundled icons must not be empty");
    let mut code = String::from(
        "/// Names of all bundled Lucide and GPUI Kit icons.\n#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, gpui::IntoElement)]\npub enum IconName {\n",
    );
    for variant in icons.keys() {
        writeln!(code, "    {variant},").unwrap();
    }
    code.push_str("}\nimpl IconName {\n    /// Every bundled icon in variant-name order.\n    pub const ALL: &'static [Self] = &[\n");
    for variant in icons.keys() {
        writeln!(code, "        Self::{variant},").unwrap();
    }
    code.push_str("    ];\n    /// The path understood by the bundled asset source.\n    pub fn path(self) -> gpui::SharedString {\n        match self {\n");
    for (variant, path) in &icons {
        writeln!(code, "            Self::{variant} => {path:?},").unwrap();
    }
    code.push_str("        }.into()\n    }\n}\n");
    code.push_str("#[doc(hidden)]\n#[allow(non_upper_case_globals)]\npub mod embedded {\n");
    for (variant, path) in &icons {
        writeln!(code, "pub const {variant}: (&str, &[u8]) = ({path:?}, include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/assets/{path}\")));").unwrap();
    }
    code.push_str("}\n");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo OUT_DIR"));
    fs::write(output.join("icon_name.rs"), code).expect("write generated icon names");
}

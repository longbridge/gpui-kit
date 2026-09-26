#[cfg(target_family = "wasm")]
use std::collections::HashMap;

#[cfg(target_family = "wasm")]
pub fn embedded_themes() -> HashMap<&'static str, &'static str> {
    let mut themes = HashMap::new();

    themes.insert("adventure", include_str!("../../../themes/adventure.json"));
    themes.insert("alduin", include_str!("../../../themes/alduin.json"));
    themes.insert("asciinema", include_str!("../../../themes/asciinema.json"));
    themes.insert("aurora", include_str!("../../../themes/aurora.json"));
    themes.insert("ayu", include_str!("../../../themes/ayu.json"));
    themes.insert(
        "catppuccin",
        include_str!("../../../themes/catppuccin.json"),
    );
    themes.insert(
        "everforest",
        include_str!("../../../themes/everforest.json"),
    );
    themes.insert(
        "fahrenheit",
        include_str!("../../../themes/fahrenheit.json"),
    );
    themes.insert("flexoki", include_str!("../../../themes/flexoki.json"));
    themes.insert("gruvbox", include_str!("../../../themes/gruvbox.json"));
    themes.insert("harper", include_str!("../../../themes/harper.json"));
    themes.insert("hybrid", include_str!("../../../themes/hybrid.json"));
    themes.insert(
        "jellybeans",
        include_str!("../../../themes/jellybeans.json"),
    );
    themes.insert("kibble", include_str!("../../../themes/kibble.json"));
    themes.insert(
        "macos-classic",
        include_str!("../../../themes/macos-classic.json"),
    );
    themes.insert(
        "mellifluous",
        include_str!("../../../themes/mellifluous.json"),
    );
    themes.insert("midnight", include_str!("../../../themes/midnight.json"));
    themes.insert("molokai", include_str!("../../../themes/molokai.json"));
    themes.insert("solarized", include_str!("../../../themes/solarized.json"));
    themes.insert("spaceduck", include_str!("../../../themes/spaceduck.json"));
    themes.insert(
        "tokyonight",
        include_str!("../../../themes/tokyonight.json"),
    );
    themes.insert("twilight", include_str!("../../../themes/twilight.json"));

    themes
}

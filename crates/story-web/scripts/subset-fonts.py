#!/usr/bin/env python3
"""Build the small, offline font set embedded by story-web.

Run from the repository root with:
    bun run --cwd crates/story-web/www fonts

The checked-in source fonts make this deterministic; FontTools is executed by
uvx so no global Python package installation is required.
"""

from pathlib import Path
import re
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[3]
FONTS = ROOT / "crates/story-web/fonts"
INCLUDE_STR = re.compile(r'include_str!\(\s*"([^"]+)"\s*\)')


def project_text() -> str:
    roots = [
        ROOT / "crates/story/src",
        ROOT / "crates/story-web/src",
        ROOT / "crates/base/examples/showcase",
        ROOT / "crates/base/examples/motion",
    ]
    text = ""
    for root in roots:
        for path in sorted(root.rglob("*.rs")):
            source = path.read_text(encoding="utf-8")
            text += source
            # Files pulled in with `include_str!` are rendered too, such as the
            # README the welcome story shows with its box-drawing crate tree.
            for included in INCLUDE_STR.findall(source):
                text += (path.parent / included).read_text(encoding="utf-8")
    # UI punctuation plus a compact set used by examples and empty/error states.
    # Emoji are not subset: the web platform draws them through the browser.
    return "".join(sorted(set(text + "–—…‘’“”•→←↑↓✓✕⚠★☆❤")))


def subset(source: Path, output: Path, text_file: Path) -> None:
    subprocess.run(
        [
            "uvx", "--from", "fonttools[woff]==4.66.0", "pyftsubset", str(source),
            f"--text-file={text_file}", f"--output-file={output}",
            # Always write uncompressed OpenType, even from the WOFF2 Inter
            # source: newer FontTools otherwise keeps the input flavor.
            "--flavor=none",
            "--layout-features=*", "--glyph-names", "--symbol-cmap",
            "--legacy-cmap", "--notdef-glyph", "--notdef-outline",
            "--recommended-glyphs", "--name-IDs=*", "--name-legacy",
            "--name-languages=*",
        ],
        check=True,
    )


def main() -> None:
    with tempfile.TemporaryDirectory() as temp:
        text_file = Path(temp) / "characters.txt"
        text_file.write_text(project_text(), encoding="utf-8")
        subset(FONTS / "Inter-Regular.source.woff2", FONTS / "Inter-Regular.ttf", text_file)
        subset(
            FONTS / "NotoSansSC-Regular.source.ttf",
            FONTS / "NotoSansSC-Regular-subset.ttf",
            text_file,
        )
        subset(
            FONTS / "JetBrainsMono-Regular.source.ttf",
            FONTS / "JetBrainsMono-Regular.ttf",
            text_file,
        )
        subset(
            FONTS / "IBMPlexSans-Regular.source.ttf",
            FONTS / "IBMPlexSans-Regular.ttf",
            text_file,
        )


if __name__ == "__main__":
    main()

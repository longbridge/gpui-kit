//! Printable ASCII in a monospace font is measured by table lookup instead of shaping.
//!
//! Wrapping a line measures many of its prefixes. Shaping each one dominates the cost of loading
//! a large document, while in a monospace font a shaped width depends only on how many
//! characters there are. The width of every count is read once from the glyph positions of one
//! shaped run. A lookup must reproduce the shaped widths bit for bit, or a break could move, so a
//! font is only used here after its shaped widths have been checked against the table.

use gpui::{Pixels, px};

#[cfg(test)]
mod tests;

/// Characters covered by the table. A row measures little more than twice its own width, so
/// this covers rows of up to about 2000 columns; longer prefixes are shaped instead.
const TABLE_CHARS: usize = 4096;

/// Sequences that ligatures or contextual alternates commonly reshape in programming fonts.
const LIGATURE_SAMPLES: &[&str] = &[
    "-> => <- <= >= != == === !== <> :: ::: ... .. // /* */ /// <!-- --> && || ++ -- ** |> <|",
    "fi fl ffi ffl www 0x 0o 0b 1x #{ ${ #[ ]# ;; ?? ?. .= := =: =~ !~ ~> <~ ~~ ## ### __ <<= >>=",
    "{\"key\": [1, -2.5e-3, true, null], 'x': \"https://example.com/a?b=c&d=e#f\"}",
];

/// Shaped widths of printable ASCII by character count, for a font where only the count matters.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct MonospaceWidths {
    /// `widths[n]` is the shaped width of any `n` printable ASCII characters.
    widths: Vec<Pixels>,
}

impl MonospaceWidths {
    /// Build the table from `glyph_starts` and check it against `measure`, the shaped width the
    /// wrapper would otherwise use.
    ///
    /// `glyph_starts` returns the x position of every glyph of a shaped line followed by the
    /// line's width. Returns `None` for a proportional font, or a font that kerns or ligatures
    /// printable ASCII into other widths.
    pub(super) fn detect(
        mut glyph_starts: impl FnMut(&str) -> Vec<Pixels>,
        mut measure: impl FnMut(&str) -> Pixels,
    ) -> Option<Self> {
        let widths = glyph_starts(&"0".repeat(TABLE_CHARS));
        let increasing = widths.windows(2).all(|pair| pair[0] < pair[1]);
        if widths.len() != TABLE_CHARS + 1 || widths[0] != px(0.) || !increasing {
            return None;
        }
        let this = Self { widths };

        // Every character starts where the table says, so no character has its own advance.
        let printable: String = (b' '..=b'~').map(char::from).collect();
        if glyph_starts(&printable) != this.widths[..=printable.len()] {
            return None;
        }

        // A line measured on its own must be as wide as the same count inside the long run.
        let lengths = (1..=16).chain([31, 64, 100, 255, 256, 1000, 1023, TABLE_CHARS]);
        let counted = lengths.flat_map(|len| {
            [
                "0".repeat(len),
                printable.chars().cycle().take(len).collect::<String>(),
            ]
        });
        let singles = printable.chars().map(String::from);
        // Every printable character next to every other one, so a kerned pair changes a sum.
        let pairs = printable.chars().map(|first| {
            printable
                .chars()
                .flat_map(|second| [first, second])
                .collect::<String>()
        });
        let ligatures = LIGATURE_SAMPLES.iter().map(|sample| sample.to_string());
        counted
            .chain(singles)
            .chain(pairs)
            .chain(ligatures)
            .all(|sample| this.width(sample.len()) == Some(measure(&sample)))
            .then_some(this)
    }

    /// The width of `chars` printable ASCII characters, or `None` past the table, where the
    /// caller must shape instead.
    pub(super) fn width(&self, chars: usize) -> Option<Pixels> {
        self.widths.get(chars).copied()
    }
}

/// Printable ASCII is one grapheme per byte and is the only text measured by lookup.
pub(super) fn is_printable_ascii(text: &str) -> bool {
    text.bytes().all(|byte| matches!(byte, b' '..=b'~'))
}

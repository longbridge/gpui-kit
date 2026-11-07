//! The styling engine behind the script's fluent chain.
//!
//! The script writes exactly what Rust writes — `v_flex():size_full():bg("surface"):p(12)`
//! — so this module has to answer one question for any method name a script
//! calls: is it a style method, and if so how is it applied to a
//! [`StyleRefinement`]?
//!
//! There are two answers, and they exist for different reasons:
//!
//! * **No-argument methods** come from GPUI's inspector reflection
//!   (`gpui_base::styled_ext_reflection_methods` and
//!   `gpui::styled_reflection::methods`). `FunctionReflection::invoke` only
//!   takes a receiver, so reflection covers precisely the `fn(self) -> Self`
//!   style methods — hundreds of names (`flex_col`, `items_center`, `gap_2`,
//!   `rounded_md`, `text_sm`, `size_full`, …) obtained with zero maintenance.
//!   When upstream GPUI adds one, the script gets it for free. They are addressed by a
//!   `u16` index so the spec arena can record a style call in two bytes instead
//!   of a string.
//! * **Methods that take arguments** cannot be reflected and are bound by hand
//!   in [`apply_param`]. This is the only hand-maintained list in the module,
//!   and it is deliberately small: about forty names.
//!
//! Both halves feed [`suggest`], because a mistyped style name must be visible
//! at the call site rather than as a silently ignored no-op — see §13.2 of
//! `docs/gpui-shell.md`.
//!
//! # Availability
//!
//! Reflection lives behind `#[cfg(any(feature = "inspector", debug_assertions))]`
//! in both `gpui-base` and `gpui`. `crates/shell` enables `gpui-base/inspector`
//! (which forwards to `gpui/inspector`), so the table is populated in release
//! builds too. [`tests::the_reflection_table_is_populated`] is the assertion
//! that keeps it that way; run it with `--release` in CI.
//!
//! # Storage
//!
//! `FunctionReflection<StyleRefinement>` is `Copy` and holds only a `&'static
//! str`, a plain `fn` pointer and a `PhantomData`, so it is `Send + Sync` and a
//! `static OnceLock` works — no thread-local fallback is needed. That matters
//! because `nullary_name` is called from `SpecArena::debug_tree`, which runs in
//! tests without a GPUI `App`.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::error::{Result as ShellResult, ShellError};
use gpui::inspector_reflection::FunctionReflection;
use gpui::{
    AbsoluteLength, DefiniteLength, FontWeight, Length, StyleRefinement, Styled, px, relative, rems,
};
use gpui_base::StyledExt as _;

use crate::value::{Bridged, arg};

/// Style methods that take arguments, in the order they are documented.
///
/// Hand-maintained because reflection cannot reach them. The array is also the
/// interning source: [`param_style_name`] hands back a `&'static str` from here
/// so the spec arena can store a name without allocating.
///
/// Deliberately **not** bound, and why:
///
/// * `cursor`, `text_align`, `text_overflow` — take GPUI enums. They need an
///   enum-name mapping of their own; every variant already has a nullary form
///   (`cursor_pointer`, `text_center`, …). `font_weight` is bound separately
///   because GPUI deliberately represents it as a numeric value.
/// * `scrollbar_width` — meaningful only together with overflow configuration
///   that the shell does not expose yet.
///
/// `text_bg`, `min_size` and `max_size` are bound even though the design doc
/// does not name them: they are the same one-line shape as their neighbours and
/// leaving them out would be an arbitrary hole in the surface.
///
/// `bg` and `shadow` are the two that take a structured value rather than a
/// primitive: `bg` also accepts a `Background` from the script's builder, and
/// `shadow` takes a list of CSS-like shadow records (`{ x, y, blur, spread,
/// color, inset }`), carried as [`Bridged::Background`] and
/// [`Bridged::Shadows`]. The `shadow_*` presets stay nullary and reflected.
/// Style methods that take an argument, with the sentence the declarations show.
///
/// The description is written here rather than reflected because these methods
/// are bound by hand — reflection reaches no-argument methods only — so there is
/// no upstream doc string to read. Keeping it beside the name is what makes a
/// description that stops matching visible in the same diff as the change.
///
/// Which length type each one accepts is *not* written down: it is probed from
/// the code that enforces it (`argument_of` in `crate::typings`), so the two can
/// never disagree.
const PARAM_STYLES: &[(&str, &str)] = &[
    // Size — `Length`, so `"auto"` is accepted.
    ("w", "Sets the width."),
    ("h", "Sets the height."),
    ("size", "Sets the width and the height together."),
    ("min_w", "Sets the minimum width."),
    ("min_h", "Sets the minimum height."),
    ("min_size", "Sets the minimum width and height together."),
    ("max_w", "Sets the maximum width."),
    ("max_h", "Sets the maximum height."),
    ("max_size", "Sets the maximum width and height together."),
    // Padding — `DefiniteLength`.
    ("p", "Sets the padding on all four sides."),
    ("px", "Sets the padding on the left and right."),
    ("py", "Sets the padding on the top and bottom."),
    ("pt", "Sets the padding on the top."),
    ("pb", "Sets the padding on the bottom."),
    ("pl", "Sets the padding on the left."),
    ("pr", "Sets the padding on the right."),
    // Margin — `Length`.
    ("m", "Sets the margin on all four sides."),
    ("mx", "Sets the margin on the left and right."),
    ("my", "Sets the margin on the top and bottom."),
    ("mt", "Sets the margin on the top."),
    ("mb", "Sets the margin on the bottom."),
    ("ml", "Sets the margin on the left."),
    ("mr", "Sets the margin on the right."),
    // Position — `Length`.
    ("inset", "Sets all four offsets of a positioned element."),
    ("top", "Sets the top offset of a positioned element."),
    ("bottom", "Sets the bottom offset of a positioned element."),
    ("left", "Sets the left offset of a positioned element."),
    ("right", "Sets the right offset of a positioned element."),
    // Flex.
    ("gap", "Sets the gap between children on both axes."),
    (
        "gap_x",
        "Sets the gap between children along the main axis.",
    ),
    (
        "gap_y",
        "Sets the gap between children along the cross axis.",
    ),
    (
        "flex_grow",
        "Sets how much of the free space this child takes.",
    ),
    (
        "flex_shrink",
        "Sets how readily this child gives space back.",
    ),
    (
        "flex_basis",
        "Sets the size this child starts from before growing or shrinking.",
    ),
    // Paint.
    (
        "bg",
        "Sets the background: a colour, or a `Background` such as `Background.linear_gradient(...)`, drawn inside the rounded corners.",
    ),
    (
        "shadow",
        "Sets the box shadows, CSS-like: the first in the list paints on top, and `[]` removes them. An inset shadow paints above the fill but beneath the children and the border.",
    ),
    (
        "text_color",
        "Sets the text colour, which children inherit.",
    ),
    (
        "text_bg",
        "Sets the background painted behind the text itself.",
    ),
    ("text_size", "Sets the font size."),
    ("font_family", "Sets the font family."),
    (
        "font_weight",
        "Sets the font weight to a number between 100 and 900.",
    ),
    (
        "line_height",
        "Sets the line height. A bare number is a multiplier of the font size, as in CSS (`1.45`); a string is a length (`\"18px\"`).",
    ),
    (
        "opacity",
        "Sets the opacity of the element and everything in it, from 0 to 1.",
    ),
    // Border and radius — `AbsoluteLength`.
    (
        "border",
        "Sets the border width on all four sides. Draws nothing without a colour.",
    ),
    ("border_t", "Sets the border width on the top."),
    ("border_b", "Sets the border width on the bottom."),
    ("border_l", "Sets the border width on the left."),
    ("border_r", "Sets the border width on the right."),
    ("border_x", "Sets the border width on the left and right."),
    ("border_y", "Sets the border width on the top and bottom."),
    (
        "border_color",
        "Sets the border colour. Draws nothing without a width.",
    ),
    ("rounded", "Sets the corner radius on all four corners."),
    (
        "rounded_t",
        "Sets the corner radius on the two top corners.",
    ),
    (
        "rounded_b",
        "Sets the corner radius on the two bottom corners.",
    ),
    (
        "rounded_l",
        "Sets the corner radius on the two left corners.",
    ),
    (
        "rounded_r",
        "Sets the corner radius on the two right corners.",
    ),
    (
        "rounded_tl",
        "Sets the corner radius on the top-left corner.",
    ),
    (
        "rounded_tr",
        "Sets the corner radius on the top-right corner.",
    ),
    (
        "rounded_bl",
        "Sets the corner radius on the bottom-left corner.",
    ),
    (
        "rounded_br",
        "Sets the corner radius on the bottom-right corner.",
    ),
];

/// The reflected no-argument style methods, plus their name index.
/// No-argument style methods that reflection does not reach.
///
/// `gpui-base` generates its font-weight helpers with a macro, and the
/// reflection pass does not see macro-expanded trait methods, so the whole
/// `font_*` family would otherwise be missing from the script surface. These
/// are appended after the reflected table and addressed by the same `u16`.
type NullaryFn = fn(StyleRefinement) -> StyleRefinement;

const EXTRA_NULLARY: &[(&str, NullaryFn)] = &[
    ("font_thin", |style| style.font_thin()),
    ("font_extralight", |style| style.font_extralight()),
    ("font_light", |style| style.font_light()),
    ("font_normal", |style| style.font_normal()),
    ("font_medium", |style| style.font_medium()),
    ("font_semibold", |style| style.font_semibold()),
    ("font_bold", |style| style.font_bold()),
    ("font_extrabold", |style| style.font_extrabold()),
    ("font_black", |style| style.font_black()),
];

struct StyleTable {
    /// Indexed by the `u16` stored in `SpecOp::NullaryStyle`.
    nullary: Vec<FunctionReflection<StyleRefinement>>,
    by_name: HashMap<&'static str, u16>,
}

fn table() -> &'static StyleTable {
    static TABLE: OnceLock<StyleTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        let nullary: Vec<_> = [
            gpui_base::styled_ext_reflection_methods::<StyleRefinement>(),
            gpui::styled_reflection::methods::<StyleRefinement>(),
        ]
        .into_iter()
        .flatten()
        .collect();

        // Both traits are in scope on `StyleRefinement`, so a name can appear
        // twice; the first wins, matching Rust's own inherent-before-extension
        // resolution closely enough for a diagnostic table.
        let mut by_name = HashMap::with_capacity(nullary.len() + EXTRA_NULLARY.len());
        for (index, method) in nullary.iter().enumerate() {
            by_name.entry(method.name).or_insert(index as u16);
        }
        for (offset, (name, _)) in EXTRA_NULLARY.iter().enumerate() {
            by_name
                .entry(*name)
                .or_insert((nullary.len() + offset) as u16);
        }

        StyleTable { nullary, by_name }
    })
}

/// Builds the reflection table once, so the first script call does not pay for it.
///
/// Idempotent: every accessor in this module initializes on demand anyway, and
/// `nullary_name` in particular is reached from `SpecArena::debug_tree` in tests
/// that never call [`init`].
pub fn init() {
    let _ = table();
}

/// Index of a no-argument style method, if the name is one.
///
/// The dispatcher calls this first: reflection is the larger and cheaper half of
/// the surface, and an index costs the spec arena two bytes per recorded call.
pub fn nullary_index(name: &str) -> Option<u16> {
    table().by_name.get(name).copied()
}

/// Every no-argument style name with the index that records it.
///
/// The prelude binds one prototype method per entry and closes over the index,
/// so recording `items_center()` sends two integers across the boundary rather
/// than a method name that would have to be resolved back to this table on
/// arrival. Sorted, so the bound surface does not depend on the iteration order
/// of a `HashMap`.
pub fn nullary_styles() -> Vec<(&'static str, u16)> {
    let mut named: Vec<_> = table()
        .by_name
        .iter()
        .map(|(name, index)| (*name, *index))
        .collect();
    named.sort_unstable();
    named
}

/// Every style name that takes an argument, in the order [`param_style_at`]
/// addresses them.
pub fn param_styles() -> impl Iterator<Item = &'static str> {
    PARAM_STYLES.iter().map(|(name, _)| *name)
}

/// The interned name at a position previously handed to the prelude by
/// [`param_styles`].
pub fn param_style_at(index: usize) -> Option<&'static str> {
    PARAM_STYLES.get(index).map(|(name, _)| *name)
}

/// Name for an index previously returned by [`nullary_index`].
///
/// Never panics — spec debug dumps must stay printable even when handed a stale
/// index from an earlier render pass.
pub fn nullary_name(index: u16) -> &'static str {
    let table = table();
    if let Some(method) = table.nullary.get(index as usize) {
        return method.name;
    }
    EXTRA_NULLARY
        .get(index as usize - table.nullary.len())
        .map(|(name, _)| *name)
        .unwrap_or("<unknown style>")
}

/// GPUI's own documentation for a style method, when the reflection carries it.
///
/// The declarations in [`crate::typings`] emit this rather than a hand-written
/// description, for the same reason the method list itself is reflected: a
/// sentence transcribed from upstream is a sentence that can quietly stop being
/// true. `EXTRA_NULLARY` and the parametric styles are named here rather than
/// reflected, so they have none — which is honest, and better than inventing
/// one.
pub fn documentation(name: &str) -> Option<&'static str> {
    if let Some((_, description)) = PARAM_STYLES
        .iter()
        .find(|(candidate, _)| *candidate == name)
    {
        return Some(description);
    }
    if let Some((_, description)) = EXTRA_NULLARY_DOCS
        .iter()
        .find(|(candidate, _)| *candidate == name)
    {
        return Some(description);
    }

    let table = table();
    let index = *table.by_name.get(name)? as usize;
    table.nullary.get(index)?.documentation
}

/// The hand-bound font weights, which reflection does not reach and so has no
/// documentation for either.
const EXTRA_NULLARY_DOCS: &[(&str, &str)] = &[
    ("font_thin", "Sets the font weight to thin (100)."),
    (
        "font_extralight",
        "Sets the font weight to extra light (200).",
    ),
    ("font_light", "Sets the font weight to light (300)."),
    ("font_normal", "Sets the font weight to normal (400)."),
    ("font_medium", "Sets the font weight to medium (500)."),
    ("font_semibold", "Sets the font weight to semibold (600)."),
    ("font_bold", "Sets the font weight to bold (700)."),
    (
        "font_extrabold",
        "Sets the font weight to extra bold (800).",
    ),
    ("font_black", "Sets the font weight to black (900)."),
];

/// Applies a no-argument style method.
///
/// An out-of-range index is a no-op rather than a panic, for the same reason
/// [`nullary_name`] is total.
pub fn apply_nullary(index: u16, refinement: StyleRefinement) -> StyleRefinement {
    let table = table();
    if let Some(method) = table.nullary.get(index as usize) {
        return method.invoke(refinement);
    }
    match EXTRA_NULLARY.get(index as usize - table.nullary.len()) {
        Some((_, apply)) => apply(refinement),
        None => refinement,
    }
}

/// Returns the interned name if `name` is a style method that takes arguments.
///
/// Returning `&'static str` is the point: the spec arena stores the name by
/// reference, so recording `:bg("surface")` allocates nothing for the method
/// name itself.
pub fn param_style_name(name: &str) -> Option<&'static str> {
    PARAM_STYLES
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map(|(name, _)| *name)
}

/// Applies a style method that takes arguments.
///
/// Coercion is never reimplemented here: numbers become pixels and strings
/// become colors through [`Bridged`], so `:p(12)` and `:bg("#ff0000")` mean the
/// same thing everywhere. The only conversion local to this module is the
/// length grammar (`"auto"`, `"50%"`, `"12px"`, `"1rem"`), which exists because
/// `Bridged` has no length concept beyond bare pixels.
pub fn apply_param(
    name: &str,
    args: &[Bridged],
    refinement: StyleRefinement,
) -> ShellResult<StyleRefinement> {
    // A script that drops the argument passes `undefined`, which crosses the
    // bridge as nil; every coercion below would then report a nil with no
    // method name, so a missing argument is answered here, in one place.
    if args
        .first()
        .is_none_or(|value| matches!(value, Bridged::Nil))
        && param_style_name(name).is_some()
    {
        return Err(missing_argument(name));
    }

    /// Reads argument 0 as a `Length` (`"auto"` allowed).
    macro_rules! length {
        () => {
            length(&arg(args, 0, name)?, name)?
        };
    }
    /// Reads argument 0 as a `DefiniteLength` (`"auto"` rejected).
    macro_rules! definite {
        () => {
            definite_length(&arg(args, 0, name)?, name)?
        };
    }
    /// Reads argument 0 as an `AbsoluteLength` (percentages rejected).
    macro_rules! absolute {
        () => {
            absolute_length(&arg(args, 0, name)?, name)?
        };
    }
    /// Reads argument 0 as a color, via token name or `#rrggbb`.
    macro_rules! color {
        () => {
            arg(args, 0, name)?.as_color()?
        };
    }
    /// Reads argument 0 as a bare number.
    macro_rules! number {
        () => {
            arg(args, 0, name)?.as_f32()?
        };
    }

    Ok(match name {
        "w" => refinement.w(length!()),
        "h" => refinement.h(length!()),
        "size" => refinement.size(length!()),
        "min_w" => refinement.min_w(length!()),
        "min_h" => refinement.min_h(length!()),
        "min_size" => refinement.min_size(length!()),
        "max_w" => refinement.max_w(length!()),
        "max_h" => refinement.max_h(length!()),
        "max_size" => refinement.max_size(length!()),

        "p" => refinement.p(definite!()),
        "px" => refinement.px(definite!()),
        "py" => refinement.py(definite!()),
        "pt" => refinement.pt(definite!()),
        "pb" => refinement.pb(definite!()),
        "pl" => refinement.pl(definite!()),
        "pr" => refinement.pr(definite!()),

        "m" => refinement.m(length!()),
        "mx" => refinement.mx(length!()),
        "my" => refinement.my(length!()),
        "mt" => refinement.mt(length!()),
        "mb" => refinement.mb(length!()),
        "ml" => refinement.ml(length!()),
        "mr" => refinement.mr(length!()),

        "inset" => refinement.inset(length!()),
        "top" => refinement.top(length!()),
        "bottom" => refinement.bottom(length!()),
        "left" => refinement.left(length!()),
        "right" => refinement.right(length!()),

        "gap" => refinement.gap(definite!()),
        "gap_x" => refinement.gap_x(definite!()),
        "gap_y" => refinement.gap_y(definite!()),
        "flex_grow" => refinement.flex_grow(number!()),
        "flex_shrink" => refinement.flex_shrink(number!()),
        "flex_basis" => refinement.flex_basis(length!()),

        "bg" => refinement.bg(arg(args, 0, name)?.as_background()?),
        "shadow" => refinement.shadow(arg(args, 0, name)?.as_shadows()?),
        "text_color" => refinement.text_color(color!()),
        "text_bg" => refinement.text_bg(color!()),
        "text_size" => refinement.text_size(absolute!()),
        "font_family" => refinement.font_family(arg(args, 0, name)?.as_str()?.to_owned()),
        "font_weight" => refinement.font_weight(font_weight(number!(), name)?),
        // Line height is the one length whose bare number is a multiplier, not
        // pixels: `line_height(1.45)` means 1.45x the font size everywhere else
        // in the industry, and 1.45px is never what anyone meant. A string
        // (`"18px"`, `"120%"`) still goes through the ordinary grammar.
        "line_height" => refinement.line_height(line_height(&arg(args, 0, name)?, name)?),
        "opacity" => refinement.opacity(number!()),

        "border" => refinement.border(absolute!()),
        "border_t" => refinement.border_t(absolute!()),
        "border_b" => refinement.border_b(absolute!()),
        "border_l" => refinement.border_l(absolute!()),
        "border_r" => refinement.border_r(absolute!()),
        "border_x" => refinement.border_x(absolute!()),
        "border_y" => refinement.border_y(absolute!()),
        "border_color" => refinement.border_color(color!()),

        "rounded" => refinement.rounded(absolute!()),
        "rounded_t" => refinement.rounded_t(absolute!()),
        "rounded_b" => refinement.rounded_b(absolute!()),
        "rounded_l" => refinement.rounded_l(absolute!()),
        "rounded_r" => refinement.rounded_r(absolute!()),
        "rounded_tl" => refinement.rounded_tl(absolute!()),
        "rounded_tr" => refinement.rounded_tr(absolute!()),
        "rounded_bl" => refinement.rounded_bl(absolute!()),
        "rounded_br" => refinement.rounded_br(absolute!()),

        other => {
            return Err(ShellError::runtime(unknown_message(other)));
        }
    })
}

/// The closest known style method name, for "did you mean" errors.
///
/// A typo in a style name is otherwise invisible — it does not change the
/// rendering, it just fails to. The threshold is tight on purpose: a wrong
/// suggestion is worse than none, so a candidate is offered only within two
/// edits, relaxed to a third of the name for longer identifiers where two edits
/// is proportionally stricter.
pub fn suggest(name: &str) -> Option<&'static str> {
    let budget = 2.max(name.chars().count() / 3);
    let mut best: Option<(usize, &'static str)> = None;
    for candidate in known_names() {
        let distance = edit_distance(name, candidate);
        if distance > budget {
            continue;
        }
        if best.is_none_or(|(best_distance, _)| distance < best_distance) {
            best = Some((distance, candidate));
        }
    }
    best.map(|(_, candidate)| candidate)
}

/// The error for a parametric style called without its argument.
///
/// GPUI spells the argument-free forms as separate methods (`flex_shrink_0`,
/// `flex_grow`, `border_1`, `rounded_md`), so `flex_shrink()` is a natural guess
/// that does not exist there either. Naming those siblings turns the guess into
/// the call that was meant.
fn missing_argument(name: &str) -> ShellError {
    let prefix = format!("{name}_");
    let siblings: Vec<String> = known_names()
        .into_iter()
        .filter(|candidate| candidate.starts_with(&prefix) && param_style_name(candidate).is_none())
        .take(4)
        .map(|candidate| format!("`{candidate}()`"))
        .collect();
    let mut message = format!("`{name}` expects 1 argument, got none");
    if !siblings.is_empty() {
        message.push_str(&format!(
            "; the forms without an argument are {}",
            siblings.join(", ")
        ));
    }
    ShellError::runtime(message)
}

/// Every known style method name (nullary + parametric), for diagnostics.
///
/// Sorted so that a dumped list is stable across runs; reflection order is
/// macro-expansion order and carries no meaning for a reader.
pub fn known_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = table()
        .nullary
        .iter()
        .map(|method| method.name)
        .chain(EXTRA_NULLARY.iter().map(|(name, _)| *name))
        .chain(PARAM_STYLES.iter().map(|(name, _)| *name))
        .collect();
    names.sort_unstable();
    names.dedup();
    names
}

fn unknown_message(name: &str) -> String {
    match suggest(name) {
        Some(candidate) => format!("unknown style method `{name}` (did you mean: {candidate}?)"),
        None => format!("unknown style method `{name}`"),
    }
}

/// A length as written in a script, before it is narrowed to what a given method
/// accepts.
///
/// The three GPUI length types form a hierarchy (`Length` ⊃ `DefiniteLength` ⊃
/// `AbsoluteLength`), so parsing once and narrowing afterwards lets the error
/// say *which* form was rejected rather than just "bad argument".
enum LengthLiteral {
    Absolute(AbsoluteLength),
    /// A percentage, stored as the fraction GPUI wants.
    Fraction(f32),
    Auto,
}

/// A bare number is pixels — the same rule as [`Bridged::as_pixels`]. Strings
/// carry an explicit unit so `"50%"` and `"1rem"` are unambiguous.
fn parse_length(value: &Bridged, method: &str) -> ShellResult<LengthLiteral> {
    if let Bridged::Str(text) = value {
        let text = text.trim();
        if text == "auto" {
            return Ok(LengthLiteral::Auto);
        }
        if let Some(number) = text.strip_suffix('%') {
            return parse_number(number, text, method)
                .map(|value| LengthLiteral::Fraction(value / 100.));
        }
        if let Some(number) = text.strip_suffix("rem") {
            return parse_number(number, text, method)
                .map(|value| LengthLiteral::Absolute(rems(value).into()));
        }
        if let Some(number) = text.strip_suffix("px") {
            return parse_number(number, text, method)
                .map(|value| LengthLiteral::Absolute(px(value).into()));
        }
        return Err(ShellError::runtime(format!(
            "`{method}` expects a length: a number of pixels, or a string like \
             \"50%\", \"12px\", \"1rem\" or \"auto\"; got \"{text}\""
        )));
    }

    Ok(LengthLiteral::Absolute(value.as_pixels()?.into()))
}

fn parse_number(number: &str, text: &str, method: &str) -> ShellResult<f32> {
    number.trim().parse::<f32>().map_err(|_| {
        ShellError::runtime(format!(
            "`{method}` could not read a number in the length \"{text}\""
        ))
    })
}

fn length(value: &Bridged, method: &str) -> ShellResult<Length> {
    Ok(match parse_length(value, method)? {
        LengthLiteral::Absolute(absolute) => Length::Definite(absolute.into()),
        LengthLiteral::Fraction(fraction) => Length::Definite(relative(fraction)),
        LengthLiteral::Auto => Length::Auto,
    })
}

/// A bare number is a multiplier; anything else follows the length grammar.
fn line_height(value: &Bridged, method: &str) -> ShellResult<DefiniteLength> {
    match value {
        Bridged::Number(multiplier) => Ok(relative(*multiplier as f32)),
        other => definite_length(other, method),
    }
}

fn font_weight(value: f32, method: &str) -> ShellResult<FontWeight> {
    if value.is_finite() && (100. ..=900.).contains(&value) {
        Ok(FontWeight(value))
    } else {
        Err(ShellError::runtime(format!(
            "`{method}` expects a finite number between 100 and 900; got {value}"
        )))
    }
}

fn definite_length(value: &Bridged, method: &str) -> ShellResult<DefiniteLength> {
    match parse_length(value, method)? {
        LengthLiteral::Absolute(absolute) => Ok(absolute.into()),
        LengthLiteral::Fraction(fraction) => Ok(relative(fraction)),
        LengthLiteral::Auto => Err(ShellError::runtime(format!(
            "`{method}` cannot be \"auto\"; it expects a definite length such as 12 or \"50%\""
        ))),
    }
}

fn absolute_length(value: &Bridged, method: &str) -> ShellResult<AbsoluteLength> {
    match parse_length(value, method)? {
        LengthLiteral::Absolute(absolute) => Ok(absolute),
        LengthLiteral::Fraction(_) | LengthLiteral::Auto => Err(ShellError::runtime(format!(
            "`{method}` expects an absolute length such as 8 or \"0.5rem\"; \
             percentages and \"auto\" are not allowed here"
        ))),
    }
}

/// Levenshtein distance over `char`s, with a rolling row so the allocation is
/// one `Vec` per comparison rather than a full matrix.
fn edit_distance(left: &str, right: &str) -> usize {
    let right: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    let mut current = vec![0; right.len() + 1];

    for (i, left_char) in left.chars().enumerate() {
        current[0] = i + 1;
        for (j, right_char) in right.iter().enumerate() {
            let substitution = previous[j] + usize::from(left_char != *right_char);
            current[j + 1] = substitution.min(previous[j + 1] + 1).min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[right.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{BackgroundKind, BackgroundSpec, ShadowSpec};
    use gpui::{Fill, Hsla};

    #[test]
    fn the_reflection_table_is_populated() {
        // Guards the `gpui-base/inspector` feature: without it this table is
        // empty in release builds and every no-argument style silently stops
        // working. Run this test with `--release` in CI.
        assert!(
            table().nullary.len() > 100,
            "expected hundreds of reflected style methods, got {}",
            table().nullary.len()
        );
    }

    #[test]
    fn a_nullary_name_round_trips_through_its_index() {
        let index = nullary_index("items_center").expect("items_center is a reflected style");
        assert_eq!(nullary_name(index), "items_center");

        let styled = apply_nullary(index, StyleRefinement::default());
        assert_eq!(styled.align_items, Some(gpui::AlignItems::Center));
    }

    #[test]
    fn an_out_of_range_index_is_printable_and_inert() {
        let index = u16::MAX;
        assert_eq!(nullary_name(index), "<unknown style>");
        assert_eq!(
            apply_nullary(index, StyleRefinement::default()),
            StyleRefinement::default()
        );
    }

    #[test]
    fn bg_sets_a_background() {
        let styled = apply_param(
            "bg",
            &[Bridged::Str("#ff0000".into())],
            StyleRefinement::default(),
        )
        .unwrap();

        let expected: Fill = Hsla::from(gpui::rgba(0xff0000ff)).into();
        assert_eq!(styled.background, Some(expected));
    }

    fn gradient(from: &str) -> Bridged {
        Bridged::Background(Box::new(BackgroundSpec {
            kind: BackgroundKind::LinearGradient {
                angle: 180.,
                from: (from.into(), 0.),
                to: ("#fdcd12".into(), 1.),
                color_space: "srgb".into(),
            },
            opacity: 1.,
        }))
    }

    fn shadow(color: &str, inset: bool) -> ShadowSpec {
        ShadowSpec {
            x: 0.,
            y: 1.,
            blur: 4.,
            spread: -1.,
            color: color.into(),
            inset,
        }
    }

    #[test]
    fn bg_takes_a_background_from_the_builder() {
        let styled = apply_param("bg", &[gradient("#f7da74")], StyleRefinement::default()).unwrap();

        let expected: Fill = gpui::linear_gradient(
            180.,
            gpui::linear_color_stop(Hsla::from(gpui::rgba(0xf7da74ff)), 0.),
            gpui::linear_color_stop(Hsla::from(gpui::rgba(0xfdcd12ff)), 1.),
        )
        .into();
        assert_eq!(styled.background, Some(expected));
    }

    #[test]
    fn bg_reports_a_bad_colour_inside_a_background() {
        let error = apply_param("bg", &[gradient("gold")], StyleRefinement::default())
            .unwrap_err()
            .to_string();
        assert!(error.contains("`gold` is not a color value"), "{error}");
    }

    #[test]
    fn shadow_takes_css_records_first_on_top_and_halves_the_blur() {
        let styled = apply_param(
            "shadow",
            &[Bridged::Shadows(vec![
                shadow("#0d0d0f", false),
                shadow("#ffffff10", true),
            ])],
            StyleRefinement::default(),
        )
        .unwrap();

        let shadows = styled.box_shadow.expect("shadow sets the box shadows");
        assert_eq!(shadows.len(), 2);
        // CSS paints the first shadow on top and GPUI the last, so the list
        // arrives reversed.
        let (ring, highlight) = (&shadows[1], &shadows[0]);
        assert_eq!(ring.offset, gpui::point(px(0.), px(1.)));
        // CSS blur is twice the gaussian deviation GPUI's shader takes.
        assert_eq!(ring.blur_radius, px(2.));
        assert_eq!(ring.spread_radius, px(-1.));
        assert_eq!(ring.color, Hsla::from(gpui::rgba(0x0d0d0fff)));
        assert!(!ring.inset);
        assert!(highlight.inset);
        assert_eq!(highlight.color, Hsla::from(gpui::rgba(0xffffff10)));
    }

    #[test]
    fn an_empty_shadow_list_removes_the_shadows() {
        let styled = apply_param(
            "shadow",
            &[Bridged::Shadows(Vec::new())],
            StyleRefinement::default().shadow_md(),
        )
        .unwrap();
        assert_eq!(styled.box_shadow, Some(Vec::new()));
    }

    #[test]
    fn shadow_names_the_entry_with_a_bad_colour_and_refuses_other_shapes() {
        let error = apply_param(
            "shadow",
            &[Bridged::Shadows(vec![
                shadow("#000", false),
                shadow("black", false),
            ])],
            StyleRefinement::default(),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("shadow 1"), "{error}");
        assert!(error.contains("`black` is not a color value"), "{error}");

        let error = apply_param(
            "shadow",
            &[Bridged::Str("#000".into())],
            StyleRefinement::default(),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("expected an array of shadows"), "{error}");

        // A shadow list is not a colour anywhere else.
        let error = apply_param(
            "border_color",
            &[Bridged::Shadows(Vec::new())],
            StyleRefinement::default(),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("got a shadow list"), "{error}");
    }

    #[test]
    fn font_weight_sets_gpui_font_weight_and_rejects_out_of_range_values() {
        let styled = apply_param(
            "font_weight",
            &[Bridged::Number(600.)],
            StyleRefinement::default(),
        )
        .unwrap();
        let expected = StyleRefinement::default().font_weight(gpui::FontWeight(600.));
        assert_eq!(styled, expected);

        for weight in [99., 901.] {
            let error = apply_param(
                "font_weight",
                &[Bridged::Number(weight)],
                StyleRefinement::default(),
            )
            .unwrap_err()
            .to_string();
            assert!(error.contains("between 100 and 900"), "{error}");
        }
    }

    #[test]
    fn a_bare_number_is_pixels_and_a_percent_string_is_relative() {
        let padded = apply_param("p", &[Bridged::Number(12.)], StyleRefinement::default()).unwrap();
        assert_eq!(padded.padding.top, Some(px(12.).into()));

        let wide = apply_param(
            "w",
            &[Bridged::Str("50%".into())],
            StyleRefinement::default(),
        )
        .unwrap();
        assert_eq!(wide.size.width, Some(Length::Definite(relative(0.5))));

        let auto = apply_param(
            "w",
            &[Bridged::Str("auto".into())],
            StyleRefinement::default(),
        )
        .unwrap();
        assert_eq!(auto.size.width, Some(Length::Auto));
    }

    #[test]
    fn a_wrongly_typed_argument_names_the_expected_type() {
        let error = apply_param("bg", &[Bridged::Number(1.)], StyleRefinement::default())
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("expected a color or a Background"),
            "error should name the expected type, got: {error}"
        );

        let error = apply_param(
            "p",
            &[Bridged::Str("auto".into())],
            StyleRefinement::default(),
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("definite length"),
            "error should explain why `auto` is rejected, got: {error}"
        );
    }

    #[test]
    fn a_missing_argument_names_the_method() {
        let error = apply_param("p", &[], StyleRefinement::default())
            .unwrap_err()
            .to_string();
        assert!(error.contains("`p` expects 1 argument"), "{error}");
    }

    #[test]
    fn a_bare_line_height_is_a_multiplier_as_in_css() {
        let multiple = apply_param(
            "line_height",
            &[Bridged::Number(1.5)],
            StyleRefinement::default(),
        )
        .unwrap();
        assert_eq!(multiple.text.line_height, Some(relative(1.5)));
        let fixed = apply_param(
            "line_height",
            &[Bridged::Str("18px".into())],
            StyleRefinement::default(),
        )
        .unwrap();
        assert_eq!(fixed.text.line_height, Some(px(18.).into()));
    }

    #[test]
    fn an_undefined_argument_names_the_method_and_its_argument_free_forms() {
        let error = apply_param("flex_shrink", &[Bridged::Nil], StyleRefinement::default())
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("`flex_shrink` expects 1 argument"),
            "{error}"
        );
        assert!(error.contains("`flex_shrink_0()`"), "{error}");
    }

    #[test]
    fn a_close_typo_gets_a_suggestion() {
        assert_eq!(suggest("items_centre"), Some("items_center"));
        assert_eq!(suggest("text_colour"), Some("text_color"));
        assert_eq!(suggest("rounde"), Some("rounded"));
    }

    #[test]
    fn a_name_with_nothing_close_gets_no_suggestion() {
        assert_eq!(suggest("on_click"), None);
        assert_eq!(suggest("completely_unrelated_name"), None);
    }

    #[test]
    fn every_parametric_name_is_bound_and_disjoint_from_reflection() {
        for (name, _) in PARAM_STYLES {
            assert_eq!(param_style_name(name), Some(*name));
            assert!(
                nullary_index(name).is_none(),
                "`{name}` is both reflected and hand-bound; the dispatcher would have to \
                 pick one arbitrarily"
            );
            // A bound name must not fall through to the unknown-method arm.
            let error = apply_param(name, &[], StyleRefinement::default())
                .expect_err("a style method needs at least one argument")
                .to_string();
            assert!(!error.contains("unknown style method"), "`{name}`: {error}");
        }
    }

    #[test]
    fn known_names_covers_both_halves() {
        let names = known_names();
        assert!(names.contains(&"items_center"));
        assert!(names.contains(&"bg"));
        assert!(names.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn edit_distance_counts_single_edits() {
        assert_eq!(edit_distance("", "abc"), 3);
        assert_eq!(edit_distance("abc", "abc"), 0);
        assert_eq!(edit_distance("items_centre", "items_center"), 2);
    }
}

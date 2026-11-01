//! Script ↔ Rust value conversion.
//!
//! Every coercion lives here so the rules are defined once: a bare number is
//! pixels, a percent string is a relative length, a `#rrggbb` string is a color,
//! and colors are explicit hexadecimal values obtained from the theme API or
//! written as literals.

use crate::{
    error::{Result as ShellResult, ShellError},
    spec::{BackgroundKind, BackgroundSpec, ShadowSpec},
};
use gpui::{BoxShadow, Hsla, Pixels, point, px, rgba};

/// A script argument after bridging, stored in the spec arena.
///
/// Engines convert their own values into this; everything above `engine/` only
/// ever sees these cases. The four primitives are what almost every argument
/// is; the two structured cases exist for the paint styles alone — `bg` takes a
/// `Background` built by the script's `Background` builder, and `shadow` takes a
/// list of shadow records — and every other coercion refuses them by name.
#[derive(Clone, Debug, PartialEq)]
pub enum Bridged {
    Nil,
    Bool(bool),
    Number(f64),
    Str(String),
    /// A value from the script's `Background` builder. Boxed so the common
    /// primitive cases keep `Bridged` small in the spec arena.
    Background(Box<BackgroundSpec>),
    /// A `shadow([...])` list, already checked for shape; the colours are
    /// checked when it is applied.
    Shadows(Vec<ShadowSpec>),
}

impl Bridged {
    pub fn as_f32(&self) -> ShellResult<f32> {
        match self {
            Bridged::Number(n) => Ok(*n as f32),
            other => Err(ShellError::runtime(format!(
                "expected a number, got {}",
                other.describe()
            ))),
        }
    }

    /// JavaScript truthiness after a value has crossed the bridge.
    pub fn is_truthy(&self) -> bool {
        match self {
            Bridged::Nil => false,
            Bridged::Bool(value) => *value,
            Bridged::Number(value) => *value != 0.0 && !value.is_nan(),
            Bridged::Str(value) => !value.is_empty(),
            // Objects and arrays are truthy in JavaScript, empty or not.
            Bridged::Background(_) | Bridged::Shadows(_) => true,
        }
    }

    pub fn as_str(&self) -> ShellResult<&str> {
        match self {
            Bridged::Str(s) => Ok(s),
            other => Err(ShellError::runtime(format!(
                "expected a string, got {}",
                other.describe()
            ))),
        }
    }

    /// A number is pixels.
    pub fn as_pixels(&self) -> ShellResult<Pixels> {
        Ok(px(self.as_f32()?))
    }

    /// A `#rgb`, `#rrggbb`, or `#rrggbbaa` color value.
    ///
    /// Semantic names are deliberately rejected. Script code must read theme
    /// colors from `cx.theme().colors` so the dependency on the active theme is
    /// explicit and remains reactive.
    pub fn as_color(&self) -> ShellResult<Hsla> {
        let text = self.as_str()?;
        let Some(hex) = text.strip_prefix('#') else {
            return Err(ShellError::runtime(format!(
                "`{text}` is not a color value; pass a color from `cx.theme().colors` or a #rgb, #rrggbb, or #rrggbbaa literal"
            )));
        };
        parse_hex(hex).ok_or_else(|| {
            ShellError::runtime(format!(
                "`{text}` is not a valid color literal (expected #rgb, #rrggbb or #rrggbbaa)"
            ))
        })
    }

    /// A `Background` value, or a colour literal standing for a solid one.
    ///
    /// The colour form is what `bg("#rrggbb")` always meant, so accepting it
    /// here keeps one coercion for both spellings.
    pub fn as_background(&self) -> ShellResult<gpui::Background> {
        match self {
            Bridged::Background(spec) => spec.resolve(),
            Bridged::Str(_) => Ok(gpui::solid_background(self.as_color()?)),
            other => Err(ShellError::runtime(format!(
                "expected a color or a Background, got {}",
                other.describe()
            ))),
        }
    }

    /// A `shadow([...])` list as GPUI box shadows.
    ///
    /// CSS paints the first shadow on top; GPUI paints its list in order, so
    /// the last one lands on top. The list is reversed here so a stack copied
    /// from a stylesheet layers the way it did there. Errors still name the
    /// position the script wrote.
    pub fn as_shadows(&self) -> ShellResult<Vec<BoxShadow>> {
        match self {
            Bridged::Shadows(shadows) => shadows
                .iter()
                .enumerate()
                .rev()
                .map(|(index, shadow)| {
                    shadow.resolve().map_err(|error| {
                        ShellError::runtime(format!("shadow {index}: {}", error.message()))
                    })
                })
                .collect(),
            other => Err(ShellError::runtime(format!(
                "expected an array of shadows, got {}",
                other.describe()
            ))),
        }
    }

    fn describe(&self) -> String {
        match self {
            Bridged::Nil => "nil".into(),
            Bridged::Bool(b) => format!("boolean ({b})"),
            Bridged::Number(n) => format!("number ({n})"),
            Bridged::Str(s) => format!("string (\"{s}\")"),
            Bridged::Background(_) => "a Background".into(),
            Bridged::Shadows(_) => "a shadow list".into(),
        }
    }
}

fn color_literal(text: &str) -> ShellResult<Hsla> {
    Bridged::Str(text.to_owned()).as_color()
}

impl BackgroundSpec {
    /// The GPUI value this describes.
    ///
    /// Shared by `window.paint_path` and the `bg` style, so a `Background`
    /// means the same thing wherever a script hands one over.
    pub fn resolve(&self) -> ShellResult<gpui::Background> {
        let background = match &self.kind {
            BackgroundKind::Solid { color } => gpui::solid_background(color_literal(color)?),
            BackgroundKind::LinearGradient {
                angle,
                from,
                to,
                color_space,
            } => gpui::linear_gradient(
                *angle,
                gpui::linear_color_stop(color_literal(&from.0)?, from.1),
                gpui::linear_color_stop(color_literal(&to.0)?, to.1),
            )
            .color_space(match color_space.as_str() {
                "oklab" => gpui::ColorSpace::Oklab,
                _ => gpui::ColorSpace::Srgb,
            }),
            BackgroundKind::PatternSlash {
                color,
                width,
                interval,
            } => gpui::pattern_slash(color_literal(color)?, *width, *interval),
            BackgroundKind::Checkerboard { color, size } => {
                gpui::checkerboard(color_literal(color)?, *size)
            }
        };
        Ok(background.opacity(self.opacity))
    }
}

impl ShadowSpec {
    /// The GPUI shadow this describes.
    ///
    /// `blur` is written with CSS semantics, where the blur radius is twice the
    /// gaussian's standard deviation; GPUI's shader takes the deviation itself,
    /// so it is halved here — the same conversion `gpui-component`'s own
    /// surface shadows make. Without it a shadow copied from a stylesheet
    /// spreads over twice the distance.
    pub fn resolve(&self) -> ShellResult<BoxShadow> {
        Ok(BoxShadow {
            color: color_literal(&self.color)?,
            offset: point(px(self.x), px(self.y)),
            blur_radius: px(self.blur / 2.),
            spread_radius: px(self.spread),
            inset: self.inset,
        })
    }
}

fn parse_hex(hex: &str) -> Option<Hsla> {
    let expand = |c: char| {
        let d = c.to_digit(16)?;
        Some(d * 17)
    };

    let rgba_value = match hex.len() {
        3 => {
            let mut chars = hex.chars();
            let r = expand(chars.next()?)?;
            let g = expand(chars.next()?)?;
            let b = expand(chars.next()?)?;
            (r << 24) | (g << 16) | (b << 8) | 0xff
        }
        6 => (u32::from_str_radix(hex, 16).ok()? << 8) | 0xff,
        8 => u32::from_str_radix(hex, 16).ok()?,
        _ => return None,
    };

    Some(rgba(rgba_value).into())
}

/// Reads the positional arguments of a bound method.
pub fn arg(args: &[Bridged], index: usize, method: &str) -> ShellResult<Bridged> {
    args.get(index).cloned().ok_or_else(|| {
        ShellError::runtime(format!(
            "`{method}` expects at least {} argument(s), got {}",
            index + 1,
            args.len()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_colors_parse_in_every_supported_length() {
        assert!(parse_hex("f00").is_some());
        assert!(parse_hex("ff0000").is_some());
        assert!(parse_hex("ff000080").is_some());
        assert!(parse_hex("ff00").is_none());
        assert!(parse_hex("zzzzzz").is_none());
    }

    #[test]
    fn bridge_truthiness_matches_javascript_primitives() {
        for falsy in [
            Bridged::Nil,
            Bridged::Bool(false),
            Bridged::Number(0.0),
            Bridged::Number(-0.0),
            Bridged::Number(f64::NAN),
            Bridged::Str(String::new()),
        ] {
            assert!(!falsy.is_truthy(), "{falsy:?}");
        }
        for truthy in [
            Bridged::Bool(true),
            Bridged::Number(1.0),
            Bridged::Number(-1.0),
            Bridged::Str("false".to_owned()),
        ] {
            assert!(truthy.is_truthy(), "{truthy:?}");
        }
    }

    #[test]
    fn number_arguments_are_pixels() {
        assert_eq!(Bridged::Number(12.0).as_pixels().unwrap(), px(12.));
    }

    #[test]
    fn semantic_color_names_are_rejected() {
        let error = Bridged::Str("border".into()).as_color().unwrap_err();
        assert!(error.to_string().contains("cx.theme().colors"));
    }
}

//! JavaScript access to gpui-base's current semantic theme.
use crate::{theme_tokens, value::Bridged};
use gpui_base::{Theme, ThemeAppearance};
use rquickjs::{Ctx, Exception, Object, Result as JsResult, Value, function::Func};
use serde::Deserialize;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

thread_local! {
    static SNAPSHOT_CACHE: RefCell<ThemeSnapshotCache> = RefCell::new(ThemeSnapshotCache::default());
}

#[derive(Default)]
struct ThemeSnapshotCache {
    entry: Option<(gpui_base::SemanticThemeTokens, ThemeAppearance, Rc<String>)>,
}

impl ThemeSnapshotCache {
    fn snapshot(
        &mut self,
        tokens: &gpui_base::SemanticThemeTokens,
        appearance: ThemeAppearance,
    ) -> Rc<String> {
        if let Some((cached_tokens, cached_appearance, json)) = &self.entry
            && cached_tokens == tokens
            && *cached_appearance == appearance
        {
            return json.clone();
        }

        let json = Rc::new(build_snapshot_json(tokens, appearance));
        self.entry = Some((tokens.clone(), appearance, json.clone()));
        json
    }
}

#[derive(Deserialize)]
struct ScriptTheme {
    appearance: ThemeAppearance,
    tokens: ScriptTokens,
}
#[derive(Deserialize)]
struct ScriptTokens {
    colors: HashMap<String, String>,
    spacing: HashMap<String, f32>,
    radius: HashMap<String, f32>,
    /// The type scale, and the only block a theme may leave out.
    ///
    /// Colours, spacing and radius are required because a script that names
    /// them names all of them: a palette with half its roles missing is a
    /// window drawn in two themes. Typography is different -- it arrived after
    /// themes were already being written, and a theme that says nothing about
    /// type is not incomplete, it is one that accepts the scale it was given.
    #[serde(default)]
    typography: TypographyOverride,
}

/// What a script changes about the type scale, rather than what the scale is.
///
/// `ScriptTheme` and `ScriptTokens` above are whole shapes: a script sends all
/// of one. This is the other kind, and is named for it -- every field is
/// optional, and every one left out keeps the token it would have replaced.
/// That is what lets an application state the one size it has an opinion about
/// -- usually `md`, the window's base text size -- without restating a line
/// height, a weight, and two font families it does not.
#[derive(Default, Deserialize)]
#[serde(default)]
struct TypographyOverride {
    /// The face the scale below is set in, and the scale itself.
    sans: Option<String>,
    xs: TextStyleOverride,
    sm: TextStyleOverride,
    md: TextStyleOverride,
    lg: TextStyleOverride,
    xl: TextStyleOverride,
    /// Code, which is a face and one size rather than a scale. `mono_md` is
    /// not a sixth step of the one above -- it is the size `mono` is set at,
    /// and the two are read together. They default apart for that reason:
    /// 13px against the scale's 16px.
    mono: Option<String>,
    mono_md: TextStyleOverride,
}

/// One entry of a [`TypographyOverride`], on the same terms.
#[derive(Default, Deserialize)]
#[serde(default)]
struct TextStyleOverride {
    size: Option<f32>,
    line_height: Option<f32>,
    weight: Option<f32>,
}

pub fn install(ctx: &Ctx<'_>, module: &Object<'_>) -> JsResult<()> {
    ctx.globals().set(
        "__theme_revision",
        Func::from(|| -> u32 { theme_tokens::revision() }),
    )?;
    ctx.globals().set(
        "__theme_snapshot",
        Func::from(|_: Ctx<'_>| -> JsResult<String> { Ok(snapshot_json().as_ref().clone()) }),
    )?;
    module.set("set_theme", Func::from(set_theme))?;
    Ok(())
}

fn set_theme<'js>(ctx: Ctx<'js>, value: Value<'js>) -> JsResult<()> {
    if matches!(
        crate::scope::current_phase(),
        Some(crate::scope::ScopePhase::Render) | Some(crate::scope::ScopePhase::Layout)
    ) {
        return Err(Exception::throw_type(
            &ctx,
            "set_theme(theme) cannot run during render or layout; switch themes from an event handler or task",
        ));
    }
    let source = ctx
        .json_stringify(value)?
        .ok_or_else(|| Exception::throw_type(&ctx, "set_theme(theme) expects an object"))?;
    let source = source.to_string()?;
    let supplied: ScriptTheme = serde_json::from_str(&source).map_err(|error| {
        Exception::throw_type(&ctx, &format!("invalid theme snapshot: {error}"))
    })?;
    crate::scope::with_current_app(|cx| {
        let mut tokens = Theme::global(cx).tokens;
        apply_colors(&mut tokens.colors, &supplied.tokens.colors)
            .map_err(|e| Exception::throw_type(&ctx, &e))?;
        apply_scale(
            theme_tokens::spacing_token_names(),
            &supplied.tokens.spacing,
            |name, value| set_spacing(&mut tokens.spacing, name, value),
        )
        .map_err(|e| Exception::throw_type(&ctx, &e))?;
        apply_scale(
            theme_tokens::radius_token_names(),
            &supplied.tokens.radius,
            |name, value| set_radius(&mut tokens.radius, name, value),
        )
        .map_err(|e| Exception::throw_type(&ctx, &e))?;
        apply_typography(&mut tokens.typography, &supplied.tokens.typography)
            .map_err(|e| Exception::throw_type(&ctx, &e))?;
        let base = Theme::global_mut(cx);
        base.appearance = supplied.appearance;
        base.tokens = tokens;
        theme_tokens::sync(cx);
        cx.refresh_windows();
        Ok::<(), rquickjs::Error>(())
    })
    .ok_or_else(|| {
        Exception::throw_type(
            &ctx,
            "set_theme(theme) needs a live host call; call it from an event handler",
        )
    })??;
    ctx.globals().set("__theme_dirty", true)?;
    Ok(())
}

fn apply_colors(
    colors: &mut gpui_base::ColorTokens,
    supplied: &HashMap<String, String>,
) -> Result<(), String> {
    for name in theme_tokens::color_token_names() {
        let source = supplied
            .get(*name)
            .ok_or_else(|| format!("theme tokens.colors is missing `{name}`"))?;
        let value = Bridged::Str(source.clone())
            .as_color()
            .map_err(|e| format!("theme color `{name}`: {e}"))?;
        match *name {
            "background" => colors.background = value,
            "foreground" => colors.foreground = value,
            "surface" => colors.surface = value,
            "surface_foreground" => colors.surface_foreground = value,
            "primary" => colors.primary = value,
            "primary_foreground" => colors.primary_foreground = value,
            "secondary" => colors.secondary = value,
            "secondary_foreground" => colors.secondary_foreground = value,
            "muted" => colors.muted = value,
            "muted_foreground" => colors.muted_foreground = value,
            "accent" => colors.accent = value,
            "accent_foreground" => colors.accent_foreground = value,
            "destructive" => colors.destructive = value,
            "destructive_foreground" => colors.destructive_foreground = value,
            "border" => colors.border = value,
            "input" => colors.input = value,
            "ring" => colors.ring = value,
            "selection" => colors.selection = value,
            _ => unreachable!(),
        }
    }
    Ok(())
}

fn apply_scale(
    names: &[&str],
    values: &HashMap<String, f32>,
    mut set: impl FnMut(&str, f32),
) -> Result<(), String> {
    for name in names {
        let value = *values
            .get(*name)
            .ok_or_else(|| format!("theme token scale is missing `{name}`"))?;
        if !value.is_finite() || value < 0. {
            return Err(format!(
                "theme token `{name}` must be finite and non-negative"
            ));
        }
        set(name, value);
    }
    Ok(())
}
/// Applies whatever a script stated about type, and leaves the rest.
///
/// Font families are taken as written -- a family name this platform does not
/// have falls back the way every other missing family does, and refusing one
/// here would mean this module deciding which fonts exist.
fn apply_typography(
    typography: &mut gpui_base::TypographyTokens,
    supplied: &TypographyOverride,
) -> Result<(), String> {
    if let Some(sans) = &supplied.sans {
        typography.sans = sans.clone().into();
    }
    if let Some(mono) = &supplied.mono {
        typography.mono = mono.clone().into();
    }
    for (name, style, token) in [
        ("xs", &supplied.xs, &mut typography.xs),
        ("sm", &supplied.sm, &mut typography.sm),
        ("md", &supplied.md, &mut typography.md),
        ("lg", &supplied.lg, &mut typography.lg),
        ("xl", &supplied.xl, &mut typography.xl),
        ("mono_md", &supplied.mono_md, &mut typography.mono_md),
    ] {
        apply_text_style(name, style, token)?;
    }
    Ok(())
}

fn apply_text_style(
    name: &str,
    supplied: &TextStyleOverride,
    token: &mut gpui_base::TextStyleToken,
) -> Result<(), String> {
    // Above zero, not merely non-negative: a size or a line height of zero is
    // text that cannot be read, and it would be applied silently. The scales
    // above allow zero because a gap of zero is a real answer.
    if let Some(size) = supplied.size {
        if !size.is_finite() || size <= 0. {
            return Err(format!("theme typography `{name}.size` must be above zero"));
        }
        token.size = gpui::px(size);
    }
    if let Some(line_height) = supplied.line_height {
        if !line_height.is_finite() || line_height <= 0. {
            return Err(format!(
                "theme typography `{name}.line_height` must be above zero"
            ));
        }
        token.line_height = gpui::px(line_height);
    }
    if let Some(weight) = supplied.weight {
        // The CSS range, which is what `FontWeight`'s own constants span.
        if !weight.is_finite() || !(1. ..=1000.).contains(&weight) {
            return Err(format!(
                "theme typography `{name}.weight` must be between 1 and 1000"
            ));
        }
        token.weight = gpui::FontWeight(weight);
    }
    Ok(())
}

fn set_spacing(t: &mut gpui_base::SpacingTokens, n: &str, v: f32) {
    let v = gpui::px(v);
    match n {
        "xxs" => t.xxs = v,
        "xs" => t.xs = v,
        "sm" => t.sm = v,
        "md" => t.md = v,
        "lg" => t.lg = v,
        "xl" => t.xl = v,
        "xxl" => t.xxl = v,
        _ => unreachable!(),
    }
}
fn set_radius(t: &mut gpui_base::RadiusTokens, n: &str, v: f32) {
    let v = gpui::px(v);
    match n {
        "none" => t.none = v,
        "sm" => t.sm = v,
        "md" => t.md = v,
        "lg" => t.lg = v,
        "xl" => t.xl = v,
        "full" => t.full = v,
        _ => unreachable!(),
    }
}

fn snapshot_json() -> Rc<String> {
    let theme = theme_tokens::snapshot();
    SNAPSHOT_CACHE.with(|cache| cache.borrow_mut().snapshot(&theme.tokens, theme.appearance))
}

fn build_snapshot_json(
    tokens: &gpui_base::SemanticThemeTokens,
    appearance: ThemeAppearance,
) -> String {
    let join = |v: Vec<String>| v.join(",");
    let pairs = theme_tokens::color_token_names()
        .iter()
        .filter_map(|n| {
            theme_tokens::resolve_color(&tokens.colors, n)
                .map(|c| format!("\"{n}\":\"{}\"", hex(c)))
        })
        .collect::<Vec<_>>();
    let colors = join(pairs.clone());
    let direct = join(pairs);
    let spacing = join(
        theme_tokens::spacing_token_names()
            .iter()
            .filter_map(|n| {
                theme_tokens::resolve_spacing(&tokens.spacing, n)
                    .map(|v| format!("\"{n}\":{}", f32::from(v)))
            })
            .collect(),
    );
    let radius = join(
        theme_tokens::radius_token_names()
            .iter()
            .filter_map(|n| {
                theme_tokens::resolve_radius(&tokens.radius, n)
                    .map(|v| format!("\"{n}\":{}", f32::from(v)))
            })
            .collect(),
    );
    let typography = {
        let t = &tokens.typography;
        let style = |name: &str, s: &gpui_base::TextStyleToken| {
            format!(
                "\"{name}\":{{\"size\":{},\"line_height\":{},\"weight\":{}}}",
                f32::from(s.size),
                f32::from(s.line_height),
                s.weight.0
            )
        };
        let mut parts = vec![
            format!("\"sans\":{}", json_string(&t.sans)),
            format!("\"mono\":{}", json_string(&t.mono)),
        ];
        parts.extend([
            style("xs", &t.xs),
            style("sm", &t.sm),
            style("md", &t.md),
            style("lg", &t.lg),
            style("xl", &t.xl),
            style("mono_md", &t.mono_md),
        ]);
        join(parts)
    };
    let name = match appearance {
        ThemeAppearance::Light => "light",
        ThemeAppearance::Dark => "dark",
    };
    format!(
        "{{{direct},\"colors\":{{{colors}}},\"spacing\":{{{spacing}}},\"radius\":{{{radius}}},\"typography\":{{{typography}}},\"appearance\":\"{name}\",\"is_dark\":{}}}",
        appearance == ThemeAppearance::Dark
    )
}
/// A font family name as a JSON string. Families are author-supplied, so the
/// two characters JSON cannot carry raw are escaped rather than assumed absent.
fn json_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn hex(color: gpui::Hsla) -> String {
    let c = gpui::Rgba::from(color);
    let b = |v: f32| (v.clamp(0., 1.) * 255.).round() as u8;
    format!("#{:02x}{:02x}{:02x}", b(c.r), b(c.g), b(c.b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unchanged_theme_snapshot_reuses_its_serialized_storage() {
        let tokens = gpui_base::SemanticThemeTokens::default();
        let mut cache = ThemeSnapshotCache::default();

        let first = cache.snapshot(&tokens, ThemeAppearance::Light);
        let second = cache.snapshot(&tokens, ThemeAppearance::Light);

        assert!(std::rc::Rc::ptr_eq(&first, &second));
    }

    #[test]
    fn changed_theme_snapshot_invalidates_serialized_storage() {
        let tokens = gpui_base::SemanticThemeTokens::default();
        let mut cache = ThemeSnapshotCache::default();

        let first = cache.snapshot(&tokens, ThemeAppearance::Light);
        let second = cache.snapshot(&tokens, ThemeAppearance::Dark);

        assert!(!std::rc::Rc::ptr_eq(&first, &second));
        assert_ne!(first, second);
    }
}

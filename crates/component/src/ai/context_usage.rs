use gpui::{
    App, ElementId, IntoElement, ParentElement as _, RenderOnce, SharedString, Styled as _, Window,
    div, prelude::FluentBuilder as _,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    popover::Popover,
    progress::{Progress, ProgressCircle},
    separator::Separator,
    v_flex,
};

/// How much of the model's context window a conversation uses.
///
/// The trigger shows a ring and percentage; activating it opens the token
/// breakdown. Counts are whatever the provider reports, for example the
/// `usage` object of an OpenAI Responses API reply.
#[derive(IntoElement)]
pub struct ContextUsage {
    id: ElementId,
    used: u64,
    limit: u64,
    input: Option<u64>,
    output: Option<u64>,
    reasoning: Option<u64>,
    cached: Option<u64>,
}

impl ContextUsage {
    /// Create a meter for `used` of `limit` tokens.
    pub fn new(id: impl Into<ElementId>, used: u64, limit: u64) -> Self {
        Self {
            id: id.into(),
            used,
            limit,
            input: None,
            output: None,
            reasoning: None,
            cached: None,
        }
    }

    /// Set the input tokens of the last request.
    pub fn input(mut self, tokens: u64) -> Self {
        self.input = Some(tokens);
        self
    }

    /// Set the output tokens of the last request.
    pub fn output(mut self, tokens: u64) -> Self {
        self.output = Some(tokens);
        self
    }

    /// Set the reasoning tokens of the last request.
    pub fn reasoning(mut self, tokens: u64) -> Self {
        self.reasoning = Some(tokens);
        self
    }

    /// Set the input tokens served from the provider's cache.
    pub fn cached(mut self, tokens: u64) -> Self {
        self.cached = Some(tokens);
        self
    }

    /// The fraction of the window in use, from 0 to 1.
    pub fn fraction(&self) -> f32 {
        if self.limit == 0 {
            return 0.;
        }
        (self.used as f64 / self.limit as f64).clamp(0., 1.) as f32
    }
}

/// Format a token count compactly: `950`, `12.4K`, `1.2M`.
pub(crate) fn format_tokens(tokens: u64) -> String {
    match tokens {
        0..1_000 => tokens.to_string(),
        1_000..1_000_000 => trim_zero(format!("{:.1}", tokens as f64 / 1_000.)) + "K",
        _ => trim_zero(format!("{:.1}", tokens as f64 / 1_000_000.)) + "M",
    }
}

fn trim_zero(value: String) -> String {
    value
        .strip_suffix(".0")
        .map(str::to_string)
        .unwrap_or(value)
}

impl RenderOnce for ContextUsage {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let percent = (self.fraction() * 100.).round() as u32;
        let id = self.id.clone();
        let rows: Vec<(SharedString, u64)> = [
            (t!("ContextUsage.input"), self.input),
            (t!("ContextUsage.output"), self.output),
            (t!("ContextUsage.reasoning"), self.reasoning),
            (t!("ContextUsage.cached"), self.cached),
        ]
        .into_iter()
        .filter_map(|(label, tokens)| tokens.map(|tokens| (label.into(), tokens)))
        .collect();
        let summary: SharedString = t!(
            "ContextUsage.summary",
            used = format_tokens(self.used),
            limit = format_tokens(self.limit)
        )
        .into();
        let fraction = self.fraction();
        let muted_foreground = cx.theme().muted_foreground;

        Popover::new(id.clone())
            .trigger(
                Button::new((id.clone(), "trigger"))
                    .ghost()
                    .small()
                    .accessibility_label(summary.clone())
                    .child(
                        ProgressCircle::new((id.clone(), "ring"))
                            .value(fraction * 100.)
                            .xsmall(),
                    )
                    .child(format!("{percent}%")),
            )
            .content(move |_, _, cx| {
                let muted_foreground = cx.theme().muted_foreground;
                v_flex()
                    .w_64()
                    .gap_2()
                    .text_sm()
                    .child(
                        h_flex()
                            .justify_between()
                            .child(div().font_medium().child(format!("{percent}%")))
                            .child(div().text_color(muted_foreground).child(summary.clone())),
                    )
                    .child(Progress::new((id.clone(), "bar")).value(fraction * 100.))
                    .when(!rows.is_empty(), |this| {
                        this.child(Separator::horizontal())
                            .children(rows.iter().map(|(label, tokens)| {
                                h_flex()
                                    .justify_between()
                                    .child(div().text_color(muted_foreground).child(label.clone()))
                                    .child(format_tokens(*tokens))
                            }))
                    })
            })
            .text_color(muted_foreground)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_usage_builder() {
        let usage = ContextUsage::new("ctx", 50_000, 200_000)
            .input(48_000)
            .output(2_000)
            .reasoning(800)
            .cached(30_000);
        assert_eq!(usage.fraction(), 0.25);
        assert_eq!(usage.input, Some(48_000));
        assert_eq!(usage.cached, Some(30_000));
        assert_eq!(ContextUsage::new("z", 5, 0).fraction(), 0.);
        assert_eq!(ContextUsage::new("o", 9, 4).fraction(), 1.);
    }

    #[test]
    fn test_format_tokens() {
        assert_eq!(format_tokens(950), "950");
        assert_eq!(format_tokens(12_400), "12.4K");
        assert_eq!(format_tokens(200_000), "200K");
        assert_eq!(format_tokens(1_240_000), "1.2M");
    }
}

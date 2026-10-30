use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, ParentElement as _, RenderOnce, SharedString, StyleRefinement,
    Styled, Window, div, prelude::FluentBuilder as _,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, IconName, StyledExt as _,
    ai::trace::{ToggleHandler, Trace},
    hover_card::HoverCard,
    link::Link,
    v_flex,
};

/// A web page or document the agent used for a reply.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    title: SharedString,
    url: SharedString,
    description: Option<SharedString>,
}

impl Source {
    /// Create a source with a title and the URL it opens.
    pub fn new(title: impl Into<SharedString>, url: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            url: url.into(),
            description: None,
        }
    }

    /// Set an excerpt or summary of the source.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// The source's title.
    pub fn title(&self) -> &SharedString {
        &self.title
    }

    /// The URL the source opens.
    pub fn url(&self) -> &SharedString {
        &self.url
    }

    /// The host part of the URL, such as `docs.rs`, used as compact metadata.
    pub fn host(&self) -> &str {
        let rest = self
            .url
            .split_once("://")
            .map_or(self.url.as_ref(), |(_, rest)| rest);
        let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
        host.strip_prefix("www.").unwrap_or(host)
    }
}

/// The links the agent used for a reply, behind a `Used N sources` summary.
#[derive(IntoElement)]
pub struct Sources {
    id: ElementId,
    style: StyleRefinement,
    sources: Vec<Source>,
    open: bool,
    on_toggle: Option<ToggleHandler>,
}

impl Sources {
    /// Create an empty, closed list.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            sources: Vec::new(),
            open: false,
            on_toggle: None,
        }
    }

    /// Add a source.
    pub fn source(mut self, source: Source) -> Self {
        self.sources.push(source);
        self
    }

    /// Add several sources.
    pub fn sources(mut self, sources: impl IntoIterator<Item = Source>) -> Self {
        self.sources.extend(sources);
        self
    }

    /// Set whether the links are revealed.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Called with the requested open state when the summary is activated.
    pub fn on_toggle(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

impl Styled for Sources {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Sources {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let muted_foreground = cx.theme().muted_foreground;
        let id = self.id;

        Trace::new(id.clone())
            .icon(IconName::BookOpen)
            .title(SharedString::from(t!(
                "Sources.used",
                count = self.sources.len()
            )))
            .open(self.open)
            .with_toggle_handler(self.on_toggle)
            .when(!self.sources.is_empty(), |this| {
                this.content(
                    v_flex()
                        .gap_1p5()
                        .children(self.sources.into_iter().enumerate().map(|(ix, source)| {
                            let host = source.host().to_string();
                            v_flex()
                                .min_w_0()
                                .child(
                                    Link::new((id.clone(), ix.to_string()))
                                        .href(source.url)
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .child(source.title),
                                )
                                .child(div().text_xs().text_color(muted_foreground).child(host))
                        })),
                )
            })
            .refine_style(&self.style)
    }
}

/// A numbered reference within a sentence; its source appears on hover and
/// opens on click.
#[derive(IntoElement)]
pub struct InlineCitation {
    id: ElementId,
    label: SharedString,
    sources: Vec<Source>,
}

impl InlineCitation {
    /// Create a citation shown as `label`, usually its number.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            sources: Vec::new(),
        }
    }

    /// Add a source the sentence relies on.
    pub fn source(mut self, source: Source) -> Self {
        self.sources.push(source);
        self
    }
}

impl RenderOnce for InlineCitation {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let muted_foreground = theme.muted_foreground;
        let first_url = self.sources.first().map(|source| source.url.clone());
        let label = match self.sources.first() {
            Some(source) if self.sources.len() > 1 => {
                format!("{} +{}", source.host(), self.sources.len() - 1)
            }
            Some(source) => source.host().to_string(),
            None => self.label.to_string(),
        };
        let sources = self.sources;
        let id = self.id;

        HoverCard::new(id.clone())
            .trigger(
                Link::new((id, "trigger"))
                    .when_some(first_url, |this, url| this.href(url))
                    .px_1p5()
                    .rounded(theme.radius_full())
                    .bg(theme.muted)
                    .text_xs()
                    .text_color(muted_foreground)
                    .child(label),
            )
            .content(move |_, _, cx| {
                let muted_foreground = cx.theme().muted_foreground;
                v_flex()
                    .w_80()
                    .gap_3()
                    .children(sources.iter().map(|source| {
                        v_flex()
                            .gap_1()
                            .child(div().text_sm().font_medium().child(source.title.clone()))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(muted_foreground)
                                    .child(source.url.clone()),
                            )
                            .when_some(source.description.clone(), |this, description| {
                                this.child(div().text_sm().child(description))
                            })
                    }))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sources_builder() {
        let sources = Sources::new("sources")
            .source(Source::new("GPUI", "https://www.gpui.rs/docs").description("Docs"))
            .sources([Source::new("Zed", "https://zed.dev")])
            .open(true)
            .on_toggle(|_, _, _| {});

        assert_eq!(sources.sources.len(), 2);
        assert_eq!(sources.sources[0].host(), "gpui.rs");
        assert_eq!(sources.sources[1].host(), "zed.dev");
        assert!(sources.open);

        let citation = InlineCitation::new("cite", "1").source(Source::new("a", "b"));
        assert_eq!(citation.sources.len(), 1);
        assert_eq!(
            Source::new("x", "localhost:3000?a").host(),
            "localhost:3000"
        );
    }
}

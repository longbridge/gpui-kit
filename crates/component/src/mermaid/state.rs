use std::time::{Duration, Instant};

use gpui::{
    App, AppContext as _, Context, InteractiveElement as _, IntoElement, ObjectFit,
    ParentElement as _, Pixels, Render, SharedString, Size, StatefulInteractiveElement as _,
    Styled as _, StyledImage as _, Task, Window, canvas, div, img, prelude::FluentBuilder as _, px,
};
use merman::OperationControl;
use rust_i18n::t;

use super::render::{self, Appearance, Diagram};
use crate::{ActiveTheme as _, v_flex};

/// Retained source, render task and image cache for a single diagram.
///
/// Create this entity once in the owning view. Theme and display-scale changes
/// automatically invalidate the corresponding cache when the diagram is rendered.
pub struct MermaidState {
    source: SharedString,
    appearance: Option<Appearance>,
    diagram: Option<Diagram>,
    error: Option<SharedString>,
    loading: bool,
    generation: u64,
    render_after: Option<Instant>,
    control: OperationControl,
    task: Task<()>,
}

impl MermaidState {
    pub fn new(source: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.on_release(|state, cx| {
            state.control.cancel();
            state.clear_image(cx);
        })
        .detach();
        Self {
            source: source.into(),
            appearance: None,
            diagram: None,
            error: None,
            loading: false,
            generation: 0,
            render_after: None,
            control: OperationControl::new(),
            task: Task::ready(()),
        }
    }

    pub fn source(&self) -> &SharedString {
        &self.source
    }

    /// Replaces the source. Rendering starts on the next frame and is debounced
    /// by 150 ms so a live editor does not render every intermediate keystroke.
    pub fn set_source(&mut self, source: impl Into<SharedString>, cx: &mut Context<Self>) {
        let source = source.into();
        if self.source == source {
            return;
        }
        self.cancel();
        self.clear_image(cx);
        self.source = source;
        self.render_after = Some(Instant::now() + Duration::from_millis(150));
        self.appearance = None;
        self.error = None;
        self.loading = false;
        cx.notify();
    }

    /// Whether a render is pending, including the debounce interval.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// The most recent rendering diagnostic, cleared when the source changes.
    pub fn error(&self) -> Option<&SharedString> {
        self.error.as_ref()
    }

    fn cancel(&mut self) {
        self.control.cancel();
        self.task = Task::ready(());
        self.generation = self.generation.wrapping_add(1);
    }

    fn clear_image(&mut self, cx: &mut App) {
        if let Some(diagram) = self.diagram.take() {
            cx.drop_image(diagram.image, None);
        }
    }

    fn ensure_render(&mut self, viewport: Size<Pixels>, window: &Window, cx: &mut Context<Self>) {
        if self.source.trim().is_empty() || viewport.width <= px(0.) || viewport.height <= px(0.) {
            return;
        }
        let appearance = Appearance::new(viewport, window, cx);
        if self.appearance.as_ref() == Some(&appearance) {
            return;
        }
        let cached = self
            .diagram
            .as_ref()
            .filter(|diagram| diagram.appearance.config == appearance.config)
            .map(|diagram| (diagram.parsed.clone(), diagram.dimensions));
        if cached.is_none() {
            self.clear_image(cx);
        }
        self.cancel();
        self.appearance = Some(appearance.clone());
        self.error = None;
        self.loading = true;
        self.control = OperationControl::new();
        let control = self.control.clone();
        let source = self.source.clone();
        let generation = self.generation;
        let delay = self
            .render_after
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            .unwrap_or_default();
        let renderer = cx.svg_renderer().clone();
        self.task = cx.spawn(async move |this, cx| {
            if !delay.is_zero() {
                smol::Timer::after(delay).await;
            }
            let result = cx
                .background_spawn(async move {
                    control.checkpoint()?;
                    if let Some((parsed, dimensions)) = cached {
                        render::rasterize(&parsed, dimensions, appearance.viewport, &renderer).map(
                            |image| Diagram {
                                appearance,
                                parsed,
                                dimensions,
                                image,
                            },
                        )
                    } else {
                        render::render(&source, &appearance, control, &renderer)
                    }
                })
                .await;
            let _ = this.update(cx, |state, cx| {
                state.finish_render(generation, result, cx);
            });
        });
        cx.notify();
    }

    fn finish_render(
        &mut self,
        generation: u64,
        result: anyhow::Result<Diagram>,
        cx: &mut Context<Self>,
    ) {
        if self.generation != generation {
            return;
        }
        self.clear_image(cx);
        self.loading = false;
        match result {
            Ok(diagram) => self.diagram = Some(diagram),
            Err(error) => self.error = Some(error.to_string().into()),
        }
        cx.notify();
    }
}

impl Render for MermaidState {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = cx.entity().downgrade();
        div()
            .relative()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                canvas(
                    move |bounds, window, cx| {
                        let _ = state
                            .update(cx, |state, cx| state.ensure_render(bounds.size, window, cx));
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .map(|this| {
                if let Some(error) = &self.error {
                    this.child(
                        v_flex()
                            .id("mermaid-error")
                            .role(gpui::accesskit::Role::Status)
                            .aria_label(format!("{}: {}", t!("Mermaid.Error"), error))
                            .size_full()
                            .overflow_y_scroll()
                            .gap_2()
                            .p_4()
                            .text_color(cx.theme().danger)
                            .child(t!("Mermaid.Error").to_string())
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(error.clone()),
                            ),
                    )
                } else if let Some(diagram) = &self.diagram {
                    this.child(
                        img(diagram.image.clone())
                            .size_full()
                            .object_fit(ObjectFit::Contain),
                    )
                } else {
                    let label = if self.source.trim().is_empty() {
                        t!("Mermaid.Empty")
                    } else {
                        t!("Mermaid.Loading")
                    }
                    .to_string();
                    this.child(
                        div()
                            .id("mermaid-status")
                            .role(gpui::accesskit::Role::Status)
                            .aria_label(label.clone())
                            .text_color(cx.theme().muted_foreground)
                            .child(label),
                    )
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    fn replacing_source_cancels_old_work_and_clears_diagnostics(cx: &mut TestAppContext) {
        let state = cx.new(|cx| MermaidState::new("invalid", cx));
        state.update(cx, |state, cx| {
            let control = state.control.clone();
            let generation = state.generation;
            state.error = Some("invalid source".into());
            state.loading = true;
            state.set_source("", cx);
            assert!(control.is_cancelled());
            assert_ne!(state.generation, generation);
            assert!(state.error().is_none());
            assert!(!state.is_loading());
            assert!(state.source().is_empty());
            state.finish_render(generation, Err(anyhow::anyhow!("stale result")), cx);
            assert!(
                state.error().is_none(),
                "A stale failure must not replace the new empty state"
            );
        });
    }
}

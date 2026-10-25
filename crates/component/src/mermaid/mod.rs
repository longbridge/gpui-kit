//! Native Mermaid diagrams rendered by Merman, then rasterized by GPUI's SVG renderer.
//!
//! Enable the `mermaid` feature and retain a [`MermaidState`] in the owning view.
//! Parsing and rasterization run in the background; [`Mermaid`] is the styled viewport.
mod render;
mod state;

pub use state::MermaidState;

use gpui::{
    App, Entity, InteractiveElement as _, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement as _, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _, rems,
};

use crate::{ActiveTheme as _, StyledExt as _};

/// A non-interactive, aspect-preserving viewport for a Mermaid diagram.
///
/// The default height is 20 rem. Use [`Styled`] to size the viewport and
/// [`Self::accessibility_label`] to describe the diagram to assistive technology.
#[derive(IntoElement)]
pub struct Mermaid {
    state: Entity<MermaidState>,
    style: StyleRefinement,
    accessibility_label: Option<SharedString>,
}

impl Mermaid {
    pub fn new(state: &Entity<MermaidState>) -> Self {
        Self {
            state: state.clone(),
            style: StyleRefinement::default(),
            accessibility_label: None,
        }
    }

    /// Sets a textual description of the diagram, rather than its Mermaid source.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

impl Styled for Mermaid {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Mermaid {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .id(("mermaid", self.state.entity_id()))
            .w_full()
            .h(rems(20.))
            .min_w_0()
            .overflow_hidden()
            .bg(cx.theme().background)
            .when_some(self.accessibility_label, |this, label| {
                this.role(gpui::accesskit::Role::Group).aria_label(label)
            })
            .refine_style(&self.style)
            .child(self.state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext as _, TestAppContext};

    #[gpui::test]
    fn test_mermaid_builder(cx: &mut TestAppContext) {
        let state = cx.new(|cx| MermaidState::new("flowchart LR\nA --> B", cx));
        let diagram = Mermaid::new(&state)
            .accessibility_label("A leads to B")
            .h(rems(24.));
        assert_eq!(diagram.state, state);
        assert_eq!(diagram.accessibility_label.as_deref(), Some("A leads to B"));
    }
}

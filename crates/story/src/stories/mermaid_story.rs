use gpui_kit::component::{
    ActiveTheme as _, IndexPath,
    input::{InputEvent, Textarea, TextareaState},
    mermaid::{Mermaid, MermaidState},
    select::{SearchableVec, Select, SelectEvent, SelectItem, SelectState},
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _,
    Render, SharedString, Styled, Subscription, Window, rems,
};
use rust_i18n::t;

use crate::section;

pub struct MermaidStory {
    source: Entity<TextareaState>,
    diagram: Entity<MermaidState>,
    example: Entity<SelectState<SearchableVec<Example>>>,
    _subscriptions: Vec<Subscription>,
}

impl super::Story for MermaidStory {
    fn title() -> &'static str {
        "Mermaid"
    }

    fn description() -> &'static str {
        "Native Mermaid diagrams rendered through Merman and SVG."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl MermaidStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let source = cx.new(|cx| {
            TextareaState::new(window, cx)
                .rows(9)
                .default_value(EXAMPLES[0].source)
        });
        let diagram = cx.new(|cx| MermaidState::new(EXAMPLES[0].source, cx));
        let subscription = cx.subscribe(&source, |this, source, event, cx| {
            if let InputEvent::Change = event {
                let value = source.read(cx).value();
                this.diagram
                    .update(cx, |state, cx| state.set_source(value, cx));
            }
        });
        let example = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(EXAMPLES.to_vec()),
                Some(IndexPath::default()),
                window,
                cx,
            )
            .searchable(true)
        });
        let example_subscription =
            cx.subscribe_in(&example, window, |this, _, event, window, cx| {
                if let SelectEvent::Confirm(Some(value)) = event
                    && let Some(example) = EXAMPLES.iter().find(|example| example.value() == value)
                {
                    this.set_example(example.source, window, cx);
                }
            });
        Self {
            source,
            diagram,
            example,
            _subscriptions: vec![subscription, example_subscription],
        }
    }

    // TextareaState::set_value does not emit InputEvent::Change.
    fn set_example(&mut self, source: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        self.source
            .update(cx, |state, cx| state.set_value(source, window, cx));
        self.diagram
            .update(cx, |state, cx| state.set_source(source, cx));
    }
}

impl Focusable for MermaidStory {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.source.focus_handle(cx)
    }
}

impl Render for MermaidStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_6()
            .child(
                section(t!("MermaidStory.Source").to_string())
                    .w_full()
                    .child(
                        v_flex()
                            .w_full()
                            .gap_3()
                            .child(
                                Select::new(&self.example)
                                    .w(rems(20.))
                                    .title_prefix(t!("MermaidStory.ExamplePrefix"))
                                    .search_placeholder(t!("MermaidStory.SearchExamples")),
                            )
                            .child(
                                Textarea::new(&self.source)
                                    .w_full()
                                    .font_family(cx.theme().mono_font_family.clone())
                                    .aria_label(t!("MermaidStory.Source")),
                            ),
                    ),
            )
            .child(
                section(t!("MermaidStory.Preview").to_string())
                    .w_full()
                    .child(
                        Mermaid::new(&self.diagram)
                            .accessibility_label(t!("MermaidStory.Preview"))
                            .h(rems(24.))
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().radius),
                    ),
            )
    }
}

// Baseline examples are adapted from Merman 0.9.2 (MIT); see
// ../fixtures/mermaid/LICENSE-MERMAN. Mixed-script examples are local.
#[derive(Clone)]
struct Example {
    name: &'static str,
    source: &'static str,
}

impl SelectItem for Example {
    type Value = &'static str;

    fn title(&self) -> SharedString {
        let key = format!("MermaidStory.{}", self.name);
        t!(&key).to_string().into()
    }

    fn value(&self) -> &Self::Value {
        &self.name
    }
}

const EXAMPLES: &[Example] = &[
    Example {
        name: "Flowchart",
        source: include_str!("../fixtures/mermaid/flowchart.mmd"),
    },
    Example {
        name: "Sequence",
        source: include_str!("../fixtures/mermaid/sequence.mmd"),
    },
    Example {
        name: "Class",
        source: include_str!("../fixtures/mermaid/class.mmd"),
    },
    Example {
        name: "State",
        source: include_str!("../fixtures/mermaid/state.mmd"),
    },
    Example {
        name: "Er",
        source: include_str!("../fixtures/mermaid/er.mmd"),
    },
    Example {
        name: "Timeline",
        source: include_str!("../fixtures/mermaid/timeline.mmd"),
    },
    Example {
        name: "Railroad",
        source: include_str!("../fixtures/mermaid/railroad.mmd"),
    },
    Example {
        name: "Agentflow",
        source: include_str!("../fixtures/mermaid/agentflow.mmd"),
    },
    Example {
        name: "Architecture",
        source: include_str!("../fixtures/mermaid/architecture.mmd"),
    },
    Example {
        name: "Block",
        source: include_str!("../fixtures/mermaid/block.mmd"),
    },
    Example {
        name: "C4",
        source: include_str!("../fixtures/mermaid/c4.mmd"),
    },
    Example {
        name: "Cynefin",
        source: include_str!("../fixtures/mermaid/cynefin.mmd"),
    },
    Example {
        name: "EventModeling",
        source: include_str!("../fixtures/mermaid/eventmodeling.mmd"),
    },
    Example {
        name: "Gantt",
        source: include_str!("../fixtures/mermaid/gantt.mmd"),
    },
    Example {
        name: "GitGraph",
        source: include_str!("../fixtures/mermaid/gitgraph.mmd"),
    },
    Example {
        name: "Info",
        source: include_str!("../fixtures/mermaid/info.mmd"),
    },
    Example {
        name: "Ishikawa",
        source: include_str!("../fixtures/mermaid/ishikawa.mmd"),
    },
    Example {
        name: "Journey",
        source: include_str!("../fixtures/mermaid/journey.mmd"),
    },
    Example {
        name: "Kanban",
        source: include_str!("../fixtures/mermaid/kanban.mmd"),
    },
    Example {
        name: "Mindmap",
        source: include_str!("../fixtures/mermaid/mindmap.mmd"),
    },
    Example {
        name: "Packet",
        source: include_str!("../fixtures/mermaid/packet.mmd"),
    },
    Example {
        name: "Pie",
        source: include_str!("../fixtures/mermaid/pie.mmd"),
    },
    Example {
        name: "Quadrant",
        source: include_str!("../fixtures/mermaid/quadrant.mmd"),
    },
    Example {
        name: "Radar",
        source: include_str!("../fixtures/mermaid/radar.mmd"),
    },
    Example {
        name: "Requirement",
        source: include_str!("../fixtures/mermaid/requirement.mmd"),
    },
    Example {
        name: "Sankey",
        source: include_str!("../fixtures/mermaid/sankey.mmd"),
    },
    Example {
        name: "Swimlane",
        source: include_str!("../fixtures/mermaid/swimlane.mmd"),
    },
    Example {
        name: "Treemap",
        source: include_str!("../fixtures/mermaid/treemap.mmd"),
    },
    Example {
        name: "TreeView",
        source: include_str!("../fixtures/mermaid/treeview.mmd"),
    },
    Example {
        name: "UseCase",
        source: include_str!("../fixtures/mermaid/usecase.mmd"),
    },
    Example {
        name: "Venn",
        source: include_str!("../fixtures/mermaid/venn.mmd"),
    },
    Example {
        name: "Wardley",
        source: include_str!("../fixtures/mermaid/wardley.mmd"),
    },
    Example {
        name: "XyChart",
        source: include_str!("../fixtures/mermaid/xychart.mmd"),
    },
    Example {
        name: "ZenUml",
        source: include_str!("../fixtures/mermaid/zenuml.mmd"),
    },
];

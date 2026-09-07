use gpui_kit::assets::Assets;
use gpui_kit::component::{
    ActiveTheme as _, Sizable as _,
    button::{Button, ButtonVariants as _},
    highlighter::Language,
    input::{Editor, EditorState, TabSize},
    resizable::h_resizable,
    status_bar::StatusBar,
    text::{SelectionFormat, html},
    v_flex,
};
use gpui_kit::*;

pub struct Example {
    input_state: Entity<EditorState>,
    /// Whether copying a selection yields the rendered text or its source.
    selection_format: SelectionFormat,
    _subscribe: Subscription,
}

const EXAMPLE: &str = include_str!("../../fixtures/test.html");

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_state = cx.new(|cx| {
            EditorState::new(window, cx)
                .language(Language::Html)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .default_value(EXAMPLE)
                .placeholder("Enter your HTML here...")
        });

        let _subscribe = cx.subscribe(
            &input_state,
            |_, _, _: &gpui_kit::component::input::InputEvent, cx| {
                cx.notify();
            },
        );

        Self {
            input_state,
            selection_format: SelectionFormat::Plain,
            _subscribe,
        }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                div().flex_1().overflow_hidden().child(
                    h_resizable("container")
                        .child(
                            div()
                                .id("source")
                                .size_full()
                                .font_family(cx.theme().mono_font_family.clone())
                                .text_size(cx.theme().mono_font_size)
                                .child(
                                    Editor::new(&self.input_state)
                                        .h(relative(1.))
                                        .appearance(false),
                                )
                                .into_any(),
                        )
                        .child(
                            html(self.input_state.read(cx).value())
                                .px_5()
                                .scrollable(true)
                                .selectable(true)
                                .selection_format(self.selection_format)
                                .into_any(),
                        ),
                ),
            )
            .child(
                StatusBar::new().right(
                    Button::new("selection-format")
                        .ghost()
                        .xsmall()
                        .label(match self.selection_format {
                            SelectionFormat::Plain => "Selection: Plain",
                            SelectionFormat::Source => "Selection: Source",
                        })
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.selection_format = match this.selection_format {
                                SelectionFormat::Plain => SelectionFormat::Source,
                                SelectionFormat::Source => SelectionFormat::Plain,
                            };
                            cx.notify();
                        })),
                ),
            )
    }
}

fn main() {
    let app = gpui_kit::application().with_assets(Assets);

    app.run(move |cx| {
        gpui_component_story::init(cx);
        cx.activate(true);

        gpui_component_story::create_new_window("HTML Render (native)", Example::view, cx);
    });
}

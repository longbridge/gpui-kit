use gpui_kit::assets::Assets;
use gpui_kit::component::{
    ActiveTheme, Selectable, Sizable, WindowExt,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{self, Input, InputEvent, InputState, TabSize, Textarea, TextareaState},
    v_flex,
};
use gpui_kit::*;

pub struct Example {
    editor: Entity<TextareaState>,
    go_to_line_state: Entity<InputState>,
    soft_wrap: bool,
    _subscribes: Vec<Subscription>,
}

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // 10K lines
        let text = "这是一个中文演示段落，用于展示更多的 [Markdown GFM] 内容。您可以在此尝试使用使用**粗体**、*斜体*和`代码`等样式。これは日本語のデモ段落です。Markdown の多言語サポートを示すためのテキストが含まれています。例えば、、**ボールド**、_イタリック_、および`コード`のスタイルなどを試すことができます。\n".repeat(10000);

        let editor = cx.new(|cx| {
            TextareaState::new(window, cx)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .soft_wrap(true)
                .placeholder("Enter your code here...")
                .default_value(text)
        });
        let go_to_line_state = cx.new(|cx| InputState::new(window, cx));

        let _subscribes = vec![cx.subscribe(&editor, |_, _, _: &InputEvent, cx| {
            cx.notify();
        })];

        Self {
            editor,
            go_to_line_state,
            soft_wrap: false,
            _subscribes,
        }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn go_to_line(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let editor = self.editor.clone();
        let input_state = self.go_to_line_state.clone();

        window.open_dialog(cx, move |dialog, window, cx| {
            input_state.update(cx, |state, cx| {
                let position = editor.read(cx).cursor_position();
                state.set_placeholder(
                    format!("{}:{}", position.line, position.character),
                    window,
                    cx,
                );
                state.focus(window, cx);
            });

            dialog
                .title("Go to line")
                .child(Input::new(&input_state))
                .on_ok({
                    let editor = editor.clone();
                    let input_state = input_state.clone();
                    move |_, window, cx| {
                        let query = input_state.read(cx).value();
                        let mut parts = query
                            .split(':')
                            .map(|s| s.trim().parse::<usize>().ok())
                            .collect::<Vec<_>>()
                            .into_iter();
                        let Some(line) = parts.next().and_then(|l| l) else {
                            return false;
                        };
                        let line = line.saturating_sub(1);
                        let column = parts.next().and_then(|c| c).unwrap_or(1).saturating_sub(1);

                        editor.update(cx, |state, cx| {
                            state.set_cursor_position(
                                input::Position::new(line as u32, column as u32),
                                window,
                                cx,
                            );
                        });

                        true
                    }
                })
        });
    }

    fn toggle_soft_wrap(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.soft_wrap = !self.soft_wrap;
        self.editor.update(cx, |state, cx| {
            state.set_soft_wrap(self.soft_wrap, window, cx);
        });
        cx.notify();
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().size_full().child(
            v_flex()
                .id("source")
                .w_full()
                .flex_1()
                .child(Textarea::new(&self.editor).bordered(false).h(relative(1.)))
                .child(
                    h_flex()
                        .justify_between()
                        .text_sm()
                        .bg(cx.theme().secondary)
                        .py_1p5()
                        .px_4()
                        .border_t_1()
                        .border_color(cx.theme().border)
                        .text_color(cx.theme().muted_foreground)
                        .child(h_flex().gap_3().child({
                            Button::new("soft-wrap")
                                .ghost()
                                .xsmall()
                                .label("Soft Wrap")
                                .selected(self.soft_wrap)
                                .on_click(cx.listener(Self::toggle_soft_wrap))
                        }))
                        .child({
                            let loc = self.editor.read(cx).cursor_position();
                            let cursor = self.editor.read(cx).cursor();

                            Button::new("line-column")
                                .ghost()
                                .xsmall()
                                .label(format!("{}:{} ({} c)", loc.line, loc.character, cursor))
                                .on_click(cx.listener(Self::go_to_line))
                        }),
                ),
        )
    }
}

fn main() {
    let app = gpui_kit::application().with_assets(Assets);

    app.run(move |cx| {
        gpui_component_story::init(cx);
        cx.activate(true);

        gpui_component_story::create_new_window("Large Text Editor", Example::view, cx);
    });
}

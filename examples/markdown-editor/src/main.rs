use gpui_kit::component::{
    ActiveTheme, Disableable, Icon, IconName, Root, Sizable,
    avatar::Avatar,
    button::Button,
    h_flex,
    text::{
        InlineElement, InlineRenderContext, MarkdownEditor, MarkdownEditorEvent,
        MarkdownEditorState, MarkdownExtensions, MarkdownNode, MarkdownParseContext,
        MarkdownPlugin, TextView, markdown_ast,
    },
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder, *};

use regex::{Captures, Regex};
use std::{
    collections::HashMap,
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex, OnceLock},
};
#[path = "../../markdown/src/mention.rs"]
mod mention;
include!("../../fixtures/markdown_plugins.rs");

const SAMPLE: &str = include_str!("../../fixtures/test.md");

struct Example {
    editor: Entity<MarkdownEditorState>,
    source: String,
    show_source: bool,
    inline_math: InlineMathPlugin,
    _subscription: Subscription,
}

impl Example {
    fn new(cx: &mut Context<Self>) -> Self {
        let mut math = None;
        let editor = cx.new(|cx| {
            let owner = cx.entity().downgrade();
            let inline_math = InlineMathPlugin::new(move |cx| {
                let _ = owner.update(cx, |_, cx| cx.notify());
            });
            let extensions = MarkdownExtensions::default()
                .plugin(inline_math.clone())
                .plugin(mention::MentionPlugin)
                .plugin(TickerPlugin::new(
                    TickerQuote {
                        name: "Apple Inc.",
                        price: 300.21,
                        change: 5.2,
                    },
                    TickerQuote {
                        name: "Tesla, Inc.",
                        price: 412.05,
                        change: -2.13,
                    },
                ))
                .plugin(UserCardPlugin::new())
                .plugin(MathPlugin::new());
            math = Some(inline_math);
            MarkdownEditorState::new_with_extensions(SAMPLE, extensions, cx)
                .expect("valid Markdown")
        });
        let subscription = cx.subscribe(&editor, |this, editor, _: &MarkdownEditorEvent, cx| {
            this.source = editor.read(cx).source();
            cx.notify();
        });
        Self {
            editor,
            source: SAMPLE.into(),
            show_source: false,
            inline_math: math.unwrap(),
            _subscription: subscription,
        }
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let readonly = self.editor.read(cx).is_readonly();
        self.inline_math.set_document(&self.source);
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .p_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("paragraph")
                            .disabled(readonly)
                            .label("正文")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.editor.update(cx, |state, cx| {
                                    state.set_heading(0, cx);
                                    state.focus(window, cx);
                                })
                            })),
                    )
                    .child(
                        Button::new("heading")
                            .disabled(readonly)
                            .label("标题")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.editor.update(cx, |state, cx| {
                                    state.set_heading(2, cx);
                                    state.focus(window, cx);
                                })
                            })),
                    )
                    .child(
                        Button::new("bold")
                            .disabled(readonly)
                            .label("粗体")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.editor.update(cx, |state, cx| {
                                    state.toggle_bold(cx);
                                    state.focus(window, cx);
                                })
                            })),
                    )
                    .child(
                        Button::new("italic")
                            .disabled(readonly)
                            .label("斜体")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.editor.update(cx, |state, cx| {
                                    state.toggle_italic(cx);
                                    state.focus(window, cx);
                                })
                            })),
                    )
                    .child(
                        Button::new("list")
                            .disabled(readonly)
                            .label("列表")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.editor.update(cx, |state, cx| {
                                    state.toggle_list(cx);
                                    state.focus(window, cx);
                                })
                            })),
                    )
                    .child(
                        Button::new("undo")
                            .disabled(readonly)
                            .label("撤销")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.editor.update(cx, |state, cx| {
                                    state.undo(cx);
                                    state.focus(window, cx);
                                })
                            })),
                    )
                    .child(
                        Button::new("redo")
                            .disabled(readonly)
                            .label("重做")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.editor.update(cx, |state, cx| {
                                    state.redo(cx);
                                    state.focus(window, cx);
                                })
                            })),
                    )
                    .child(
                        Button::new("mode")
                            .label(if readonly {
                                "切换为编辑"
                            } else {
                                "切换为只读"
                            })
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.editor.update(cx, |state, cx| {
                                    state.set_readonly(!state.is_readonly(), cx);
                                    state.focus(window, cx);
                                });
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("source")
                            .label("Markdown")
                            .small()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.show_source = !this.show_source;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_stretch()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .p_6()
                            .child(MarkdownEditor::new(&self.editor)),
                    )
                    .when(self.show_source, |this| {
                        this.child(
                            div()
                                .w_96()
                                .border_l_1()
                                .border_color(cx.theme().border)
                                .p_4()
                                .child(
                                    TextView::markdown(
                                        "source",
                                        format!("````markdown\n{}\n````", self.source),
                                    )
                                    .scrollable(true),
                                ),
                        )
                    }),
            )
    }
}

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx| {
            gpui_kit::init(cx);
            // Load remote images with the same client as the original demo.
            let http_client =
                reqwest_client::ReqwestClient::user_agent("gpui-component/markdown-editor")
                    .unwrap();
            cx.set_http_client(Arc::new(http_client));
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::centered(size(px(1100.), px(760.)), cx)),
                    ..Default::default()
                },
                |window, cx| {
                    window.set_window_title("Markdown Editor");
                    let view = cx.new(Example::new);
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .expect("open Markdown editor");
            cx.activate(true);
        });
}

#[cfg(test)]
mod tests {
    use super::Example;
    use gpui_kit as gpui;

    #[gpui::test]
    fn shared_markdown_fixture_renders_with_original_plugins(cx: &mut gpui::TestAppContext) {
        use gpui::VisualTestContext;
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(|_, cx| Example::new(cx));
        VisualTestContext::update(cx, |window, cx| {
            window.draw(cx).clear(cx);
            let editor = view.read(cx).editor.clone();
            let before = editor.read(cx).source();
            for expected in [
                "Hello",
                "AAPL.US",
                "TSLA.US",
                "UserCard",
                "mention:huacnlee",
                "e^{i",
                "```rust",
                "Numbered item",
            ] {
                assert!(
                    before.contains(expected),
                    "fixture content missing: {expected}"
                );
            }
            editor.update(cx, |state, cx| state.set_readonly(true, cx));
            window.draw(cx).clear(cx);
            assert_eq!(editor.read(cx).source(), before);
            editor.update(cx, |state, cx| state.set_readonly(false, cx));
            window.draw(cx).clear(cx);
            assert_eq!(editor.read(cx).source(), before);
        });
    }
}

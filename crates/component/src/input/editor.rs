use std::rc::Rc;

use gpui::{
    App, DefiniteLength, Entity, IntoElement, RenderOnce, SharedString, StyleRefinement, Styled,
    Window, prelude::FluentBuilder as _, relative,
};

use super::{EditorState, Input};
use crate::native_menu::NativeMenu;
use crate::{ActiveTheme as _, RoleOverride, StyledExt as _};

/// A code editor takes its rows from the font, so that a smaller or larger
/// font keeps its leading in proportion.
const EDITOR_LINE_HEIGHT: f32 = 1.5;

/// A styled source-code editor.
#[derive(IntoElement)]
pub struct Editor {
    state: Entity<EditorState>,
    style: StyleRefinement,
    height: Option<DefiniteLength>,
    appearance: bool,
    bordered: bool,
    disabled: bool,
    readonly: bool,
    tab_index: isize,
    role: RoleOverride,
    aria_label: Option<SharedString>,

    /// An optional context menu builder to allow a custom context menu.
    ///
    /// If set, this overrides the built-in context menu.
    context_menu_builder: Option<Rc<dyn Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu>>,
}

impl Editor {
    pub fn new(state: &Entity<EditorState>) -> Self {
        Self {
            state: state.clone(),
            style: StyleRefinement::default(),
            height: None,
            appearance: true,
            bordered: true,
            disabled: false,
            readonly: false,
            tab_index: 0,
            role: RoleOverride::default(),
            aria_label: None,
            context_menu_builder: None,
        }
    }

    pub fn h(mut self, height: impl Into<DefiniteLength>) -> Self {
        self.height = Some(height.into());
        self
    }

    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the editor to read-only, default is `false`.
    ///
    /// Unlike [`Self::disabled`], a read-only editor keeps the normal appearance
    /// and still can be focused, selected and copied, it only rejects the changes
    /// made by the user.
    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    pub fn tab_index(mut self, index: isize) -> Self {
        self.tab_index = index;
        self
    }

    pub fn role(mut self, role: impl Into<RoleOverride>) -> Self {
        self.role = role.into();
        self
    }

    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Replace the built-in context menu shown on right-click.
    ///
    /// The closure receives an empty menu and returns the one to show, so it
    /// decides entirely what appears — the default items are not added.
    pub fn context_menu(
        mut self,
        f: impl Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu + 'static,
    ) -> Self {
        self.context_menu_builder = Some(Rc::new(f));
        self
    }
}

impl Styled for Editor {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Editor {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        Input::from_state(self.state.clone())
            // Source code wants a monospace font at a code size, and rows that
            // follow that size. These come first so that a text style set on
            // this editor refines over them: `.text_sm()` and `.font_family()`
            // keep working.
            .font_family(cx.theme().mono_font_family.clone())
            .text_size(cx.theme().mono_font_size)
            .line_height(relative(EDITOR_LINE_HEIGHT))
            .appearance(self.appearance)
            .bordered(self.bordered)
            .focus_bordered(false)
            .disabled(self.disabled)
            .readonly(self.readonly)
            .tab_index(self.tab_index)
            .role(self.role)
            .when_some(self.height, |this, height| this.h(height))
            .when_some(self.aria_label, |this, label| this.aria_label(label))
            .when_some(self.context_menu_builder, |this, build| {
                this.context_menu(move |menu, window, cx| build(menu, window, cx))
            })
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::EditorState;
    use gpui::{
        AppContext as _, Context, ParentElement as _, Pixels, Render, TestAppContext,
        VisualTestContext, div, px,
    };

    struct Harness {
        state: Entity<EditorState>,
        /// A text size set on the editor, as `.text_sm()` would.
        text_size: Option<Pixels>,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Editor::new(&self.state)
                    .when_some(self.text_size, |this, size| this.text_size(size)),
            )
        }
    }

    /// The row height the editor laid out with, which follows its font size.
    fn line_height(cx: &mut TestAppContext, text_size: Option<Pixels>) -> Pixels {
        cx.update(crate::init);
        let mut state = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let editor = cx.new(|cx| EditorState::new(window, cx).default_value("fn main() {}"));
            state = Some(editor.clone());
            Harness {
                state: editor,
                text_size,
            }
        });
        let state = state.unwrap();
        VisualTestContext::update(cx, |window, cx| window.draw(cx).clear(cx));

        cx.read(|cx| {
            state
                .read(cx)
                .line_height()
                .expect("the editor must lay out")
        })
    }

    #[gpui::test]
    fn the_rows_follow_the_font_size(cx: &mut TestAppContext) {
        // With nothing set, the theme's monospace size, not the ambient one.
        assert_eq!(line_height(cx, None), px(20.));
        // A text style set on the editor refines over that, rows and all.
        assert_eq!(line_height(cx, Some(px(24.))), px(36.));
        assert_eq!(line_height(cx, Some(px(40.))), px(60.));
    }
    #[gpui::test]
    fn language_config_works_without_render_sync(cx: &mut TestAppContext) {
        use crate::input::{AutoClosingPair, language_config::LanguageConfig, set_language_config};
        use gpui::EntityInputHandler as _;
        cx.update(crate::init);
        let mut state = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let editor = cx.new(|cx| EditorState::new(window, cx).language("plaintext"));
            // Plain-text defaults apply before the first render.
            editor.update(cx, |state, cx| {
                state.replace_text_in_range(None, "(", window, cx);
                assert_eq!(state.text().to_string(), "(");
                state.set_value("", window, cx);
                state.set_highlighter("json", cx);
                state.replace_text_in_range(None, "[", window, cx);
                assert_eq!(state.text().to_string(), "[]");
            });
            state = Some(editor.clone());
            Harness {
                state: editor,
                text_size: None,
            }
        });
        let state = state.unwrap();
        VisualTestContext::update(cx, |_, cx| {
            set_language_config(
                "json",
                LanguageConfig::default().auto_closing_pairs([AutoClosingPair::new("«", "»")]),
                cx,
            );
        });
        // A styled render must preserve the registered configuration.
        VisualTestContext::update(cx, |window, cx| window.draw(cx).clear(cx));
        VisualTestContext::update(cx, |window, cx| {
            state.update(cx, |state, cx| {
                state.set_value("", window, cx);
                state.replace_text_in_range(None, "«", window, cx);
                assert_eq!(state.text().to_string(), "«»");
                state.set_value("if enabled:", window, cx);
                state.set_selected_range(11..11, cx);
                state.set_highlighter("python", cx);
                state.focus(window, cx);
            });
        });
        VisualTestContext::update(cx, |window, cx| window.draw(cx).clear(cx));
        cx.simulate_keystrokes("enter");
        cx.read(|cx| assert_eq!(state.read(cx).text().to_string(), "if enabled:\n  "));
    }

    #[gpui::test]
    fn aliases_share_config_even_when_registered_before_init(cx: &mut TestAppContext) {
        use crate::input::{AutoClosingPair, language_config::LanguageConfig, set_language_config};
        use gpui::EntityInputHandler as _;
        cx.update(|cx| {
            set_language_config(
                "py",
                LanguageConfig::default().auto_closing_pairs([AutoClosingPair::new("«", "»")]),
                cx,
            )
        });
        cx.update(crate::init);
        cx.add_window_view(|window, cx| {
            let editor = cx.new(|cx| EditorState::new(window, cx).language("PYTHON"));
            editor.update(cx, |state, cx| {
                state.replace_text_in_range(None, "«", window, cx);
                assert_eq!(state.text().to_string(), "«»");
            });
            set_language_config("pyi", LanguageConfig::default().auto_closing_pairs([]), cx);
            editor.update(cx, |state, cx| {
                state.set_value("", window, cx);
                state.set_highlighter("py", cx);
                state.replace_text_in_range(None, "(", window, cx);
                assert_eq!(state.text().to_string(), "(");
            });
            Harness {
                state: editor,
                text_size: None,
            }
        });
    }

    #[cfg(all(feature = "tree-sitter-python", feature = "tree-sitter-rust"))]
    #[gpui::test]
    fn syntax_follows_language_before_first_render_and_after_switch(cx: &mut TestAppContext) {
        use gpui::EntityInputHandler as _;
        cx.update(crate::init);
        cx.add_window_view(|window, cx| {
            let editor = cx.new(|cx| {
                EditorState::new(window, cx)
                    .language("python")
                    .default_value("\"hello world\"")
            });
            editor.update(cx, |state, cx| {
                state.set_selected_range(6..6, cx);
                state.replace_text_in_range(None, "(", window, cx);
                assert_eq!(state.text().to_string(), "\"hello( world\"");
                state.set_value("# comment ", window, cx);
                state.set_selected_range(10..10, cx);
                state.replace_text_in_range(None, "(", window, cx);
                assert_eq!(state.text().to_string(), "# comment (");
                state.set_value("# comment ", window, cx);
                state.set_selected_range(10..10, cx);
                state.set_highlighter("rust", cx);
                state.replace_text_in_range(None, "(", window, cx);
                assert_eq!(state.text().to_string(), "# comment ()");
            });
            Harness {
                state: editor,
                text_size: None,
            }
        });
    }

    #[cfg(feature = "tree-sitter-python")]
    #[gpui::test]
    fn python_pairing_uses_pre_edit_context(cx: &mut TestAppContext) {
        use gpui::EntityInputHandler as _;
        cx.update(crate::init);
        for (before, cursor, typed, expected) in [
            ("x = ", 4, "\"", "x = \"\""),
            ("x = f\"{value}\"", 12, "(", "x = f\"{value()}\""),
            ("x = \"value\"", 7, "(", "x = \"va(lue\""),
            ("x = \"value\"", 5, "(", "x = \"(value\""),
            ("x = \"value\"", 10, "(", "x = \"value(\""),
        ] {
            let mut state = None;
            let (_, cx) = cx.add_window_view(|window, cx| {
                let editor = cx.new(|cx| EditorState::new(window, cx).default_value(before));
                state = Some(editor.clone());
                Harness {
                    state: editor,
                    text_size: None,
                }
            });
            let state = state.unwrap();
            VisualTestContext::update(cx, |window, cx| {
                state.update(cx, |state, cx| {
                    state.set_highlighter("python", cx);
                    state.set_selected_range(cursor..cursor, cx);
                    state.replace_text_in_range(None, typed, window, cx);
                    assert_eq!(state.text().to_string(), expected);
                });
            });
        }
    }
    #[cfg(feature = "tree-sitter-rust")]
    #[gpui::test]
    fn generated_comment_closer_survives_syntax_changes(cx: &mut TestAppContext) {
        use crate::input::{AutoClosingPair, SyntaxContext, language_config::LanguageConfig};
        use gpui::EntityInputHandler as _;
        cx.update(crate::init);
        cx.update(|cx| {
            crate::input::set_language_config(
                "rust",
                LanguageConfig::default().auto_closing_pairs([AutoClosingPair::new("/*", "*/")
                    .not_in([SyntaxContext::String, SyntaxContext::Comment])]),
                cx,
            )
        });
        let mut state = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let editor = cx.new(|cx| EditorState::new(window, cx).language("rust"));
            state = Some(editor.clone());
            Harness {
                state: editor,
                text_size: None,
            }
        });
        let state = state.unwrap();
        VisualTestContext::update(cx, |window, cx| {
            state.update(cx, |state, cx| {
                state.set_highlighter("rust", cx);

                for text in ["/", "*", "x", "*", "/"] {
                    state.replace_text_in_range(None, text, window, cx);
                }
                assert_eq!(state.text().to_string(), "/*x*/");
                state.replace_text_in_range(None, "!", window, cx);
                assert_eq!(state.text().to_string(), "/*x*/!");
            });
        });
    }
}

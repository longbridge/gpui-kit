use std::rc::Rc;

use gpui::{
    App, DefiniteLength, Entity, IntoElement, RenderOnce, SharedString, StyleRefinement, Styled,
    Window, prelude::FluentBuilder as _, relative,
};

use super::{EditorState, GutterStyle, Input};
use crate::native_menu::NativeMenu;
use crate::{ActiveTheme as _, RoleOverride, Sizable, Size, StyledExt as _};

/// A code editor takes its rows from the font, so that a smaller or larger
/// font keeps its leading in proportion.
const EDITOR_LINE_HEIGHT: f32 = 1.5;

/// A styled source-code editor.
#[derive(IntoElement)]
pub struct Editor {
    state: Entity<EditorState>,
    gutter_style: GutterStyle,
    style: StyleRefinement,
    size: Size,
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

    paste_handler: Option<Rc<dyn Fn(&gpui::ClipboardItem, &mut Window, &mut App) -> bool>>,
}

impl Editor {
    pub fn new(state: &Entity<EditorState>) -> Self {
        Self {
            state: state.clone(),
            gutter_style: GutterStyle::default(),
            style: StyleRefinement::default(),
            size: Size::default(),
            height: None,
            appearance: true,
            bordered: true,
            disabled: false,
            readonly: false,
            tab_index: 0,
            role: RoleOverride::default(),
            aria_label: None,
            context_menu_builder: None,
            paste_handler: None,
        }
    }

    /// Overrides specified line-number properties over the active theme.
    ///
    /// Omitting this on a later render restores the theme defaults.
    pub fn apply_gutter_style(mut self, style: GutterStyle) -> Self {
        self.gutter_style.apply_overrides(&style);
        self
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
    /// decides entirely what appears — the default items are not added. It
    /// shows only while the state's context menu is enabled, which is the
    /// default.
    pub fn context_menu(
        mut self,
        f: impl Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu + 'static,
    ) -> Self {
        self.context_menu_builder = Some(Rc::new(f));
        self
    }

    /// Intercept paste payloads (images, files) before the default text insertion.
    ///
    /// `true` consumes the paste so nothing is inserted, `false` falls through
    /// to `clipboard.text()`. Copied files arrive as `ExternalPaths` through
    /// the same hook. On web the clipboard reads `None`; image paste needs
    /// async clipboard access and is out of scope.
    pub fn on_paste(
        mut self,
        handler: impl Fn(&gpui::ClipboardItem, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.paste_handler = Some(Rc::new(handler));
        self
    }
}

/// Match the padding and density of Input and Textarea in the same form by
/// applying the same size tier. The editor keeps its independently configured
/// code font size and line height.
impl Sizable for Editor {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
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
            .apply_gutter_style(self.gutter_style)
            // Source code wants a monospace font at a code size, and rows that
            // follow that size. These come first so that a text style set on
            // this editor refines over them: `.text_sm()` and `.font_family()`
            // keep working.
            .font_family(cx.theme().mono_font_family.clone())
            .text_size(cx.theme().mono_font_size)
            .line_height(relative(EDITOR_LINE_HEIGHT))
            .with_size(self.size)
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
            .when_some(self.paste_handler, |this, handler| {
                this.on_paste(move |item, window, cx| handler(item, window, cx))
            })
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::EditorState;
    use gpui::{
        AppContext as _, Context, InteractiveElement as _, ParentElement as _, Pixels, Render,
        TestAppContext, VisualTestContext, div, px,
    };

    #[gpui::test]
    fn test_editor_gutter_style_builder(cx: &mut TestAppContext) {
        cx.update(crate::init);
        cx.add_window_view(|window, cx| {
            let state = cx.new(|cx| EditorState::new(window, cx));
            let defaults = GutterStyle::default()
                .with_gap(px(12.))
                .with_line_number_color(gpui::rgb(0x808080).into())
                .with_active_line_number_color(gpui::rgb(0xeeeeee).into())
                .with_line_number_font_family("Number Font")
                .with_line_number_font_size(px(12.));
            let editor = Editor::new(&state)
                .apply_gutter_style(defaults.clone())
                .apply_gutter_style(GutterStyle::default().with_gap(px(0.)));
            assert_eq!(editor.gutter_style, defaults.with_gap(px(0.)));
            Harness { state, text_size: None }
        });
    }

    struct GutterHarness {
        state: Entity<EditorState>,
        overrides: Option<GutterStyle>,
    }

    impl Render for GutterHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Editor::new(&self.state)
                    .font_family(".SystemUIFont")
                    .text_size(px(16.))
                    .when_some(self.overrides.clone(), |this, style| {
                        this.apply_gutter_style(style)
                    }),
            )
        }
    }

    #[gpui::test]
    fn gutter_theme_changes_and_override_removal_reach_the_rendered_editor(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let (view, cx) = cx.add_window_view(|window, cx| GutterHarness {
            state: cx.new(|cx| EditorState::new(window, cx).default_value("hello\nworld")),
            overrides: None,
        });
        let state = view.read_with(cx, |view, _| view.state.clone());
        let measure = |cx: &mut VisualTestContext| {
            cx.update(|window, cx| {
                window.refresh();
                window.draw(cx).clear(cx);
                let state = state.read(cx);
                let caret = state.prepaint_caret_geometry().expect("editor must prepaint");
                (
                    caret.cursor_bounds().left() - caret.input_bounds().left(),
                    caret.line_height(),
                    state.range_to_bounds(&(0..1)).unwrap().size.width,
                )
            })
        };
        let baseline = measure(cx);

        cx.update(|_, cx| {
            crate::Theme::update(cx, |theme| {
                theme.set_editor_gutter_style(
                    GutterStyle::default()
                        .with_gap(px(18.))
                        .with_line_number_font_family(".SystemUIFont")
                        .with_line_number_font_size(px(16.)),
                );
            });
        });
        let themed = measure(cx);
        assert_eq!(themed.0 - baseline.0, px(12.));
        assert_eq!((themed.1, themed.2), (baseline.1, baseline.2));

        view.update(cx, |view, cx| {
            view.overrides = Some(GutterStyle::default().with_gap(px(0.)));
            cx.notify();
        });
        let overridden = measure(cx);
        assert_eq!(themed.0 - overridden.0, px(18.));

        cx.update(|_, cx| {
            crate::Theme::update(cx, |theme| {
                theme.set_editor_gutter_style(
                    theme.get_editor_gutter_style().clone()
                        .with_gap(px(24.))
                        .with_line_number_font_size(px(24.)),
                );
            });
        });
        let changed = measure(cx);
        // The local zero gap must survive a theme change, while the unset
        // font size must follow it; body metrics must remain independent.
        assert!(changed.0 > overridden.0);
        assert_eq!((changed.1, changed.2), (baseline.1, baseline.2));

        view.update(cx, |view, cx| {
            view.overrides = None;
            cx.notify();
        });
        let restored = measure(cx);
        assert_eq!(restored.0 - changed.0, px(24.));

        cx.update(|_, cx| {
            crate::Theme::update(cx, |theme| {
                theme.set_editor_gutter_style(GutterStyle::default());
            });
        });
        assert_eq!(measure(cx), baseline);
    }

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

    struct SizedHarness {
        state: Entity<EditorState>,
        /// A size set on the editor; `None` leaves the default.
        size: Option<Size>,
    }

    impl Render for SizedHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().debug_selector(|| "frame".into()).size_full().child(
                Editor::new(&self.state).when_some(self.size, |this, size| this.with_size(size)),
            )
        }
    }

    /// How far the text starts below the top of the editor's frame, and the
    /// row height it laid out with.
    fn top_inset(cx: &mut TestAppContext, size: Option<Size>) -> (Pixels, Pixels) {
        cx.update(crate::init);
        let mut state = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let editor = cx.new(|cx| EditorState::new(window, cx).default_value("fn main() {}"));
            state = Some(editor.clone());
            SizedHarness {
                state: editor,
                size,
            }
        });
        let state = state.unwrap();
        VisualTestContext::update(cx, |window, cx| window.draw(cx).clear(cx));

        let frame = cx.debug_bounds("frame").expect("the frame must lay out");
        let (text, line_height) = cx.read(|cx| {
            let state = state.read(cx);
            let line_height = state.line_height().expect("the editor must lay out");
            (state.input_bounds(), line_height)
        });
        (text.top() - frame.top(), line_height)
    }

    #[gpui::test]
    fn the_insets_follow_the_size(cx: &mut TestAppContext) {
        let (medium, rows) = top_inset(cx, Some(Size::Medium));
        // With no size set, the editor keeps the medium insets it always had.
        assert_eq!(top_inset(cx, None), (medium, rows));
        for size in [Size::XSmall, Size::Small, Size::Large] {
            let (inset, line_height) = top_inset(cx, Some(size));
            assert_eq!(
                medium - inset,
                Size::Medium.input_py() - size.input_py(),
                "{size:?}"
            );
            // Only the insets change, not the font.
            assert_eq!(line_height, rows, "{size:?}");
        }
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

    #[gpui::test]
    fn test_on_paste_builder(cx: &mut TestAppContext) {
        use gpui::{AppContext as _, Render};

        struct PasteProbe;
        impl Render for PasteProbe {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div()
            }
        }

        cx.update(crate::init);
        let _ = cx.add_window_view(|window, cx| {
            let state = cx.new(|cx| EditorState::new(window, cx));
            assert!(Editor::new(&state).paste_handler.is_none());
            let editor = Editor::new(&state).on_paste(|_, _, _| true);
            assert!(editor.paste_handler.is_some());
            PasteProbe
        });
    }
}

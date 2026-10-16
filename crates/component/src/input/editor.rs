use std::rc::Rc;

use gpui::{
    App, DefiniteLength, Entity, IntoElement, RenderOnce, SharedString, StyleRefinement, Styled,
    Window, prelude::FluentBuilder as _, relative,
};

use super::input::ContextMenuBuilder;
use super::{EditorState, Input, InputContextMenuCapabilities, InputHit};
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
    context_menu_builder: Option<ContextMenuBuilder>,

    paste_handler: Option<Rc<dyn Fn(&gpui::ClipboardItem, &mut Window, &mut App) -> bool>>,
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
            paste_handler: None,
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
    /// decides entirely what appears — the default items are not added. It
    /// shows only while the state's context menu is enabled, which is the
    /// default.
    pub fn context_menu(
        mut self,
        f: impl Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu + 'static,
    ) -> Self {
        self.context_menu_builder =
            Some(Rc::new(move |menu, _, _, window, cx| f(menu, window, cx)));
        self
    }

    /// Replace the built-in context menu, told what it was opened on.
    ///
    /// See [`Input::context_menu_with`]: a menu for a row of the gutter reads
    /// the row from the [`InputHit`] it is given, and
    /// [`super::default_context_menu`] keeps the built-in items for the text.
    pub fn context_menu_with(
        mut self,
        f: impl Fn(
            NativeMenu,
            InputContextMenuCapabilities,
            Option<InputHit>,
            &mut Window,
            &mut App,
        ) -> NativeMenu
        + 'static,
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
                this.context_menu_with(move |menu, capabilities, hit, window, cx| {
                    build(menu, capabilities, hit, window, cx)
                })
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
    fn a_gutter_on_the_right_keeps_the_padding_it_has_on_the_left(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let mut state = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let editor = cx.new(|cx| EditorState::new(window, cx).default_value("fn main() {}"));
            state = Some(editor.clone());
            Harness {
                state: editor,
                text_size: None,
            }
        });
        let state = state.unwrap();
        let mut padding = |side: crate::Side| {
            VisualTestContext::update(cx, |window, cx| {
                state.update(cx, |state, cx| state.set_gutter_side(side, cx));
                window.draw(cx).clear(cx);
                let bounds = state.read(cx).input_bounds();
                (bounds.left(), window.viewport_size().width - bounds.right())
            })
        };

        let (left_outer, left_inner) = padding(crate::Side::Left);
        let (right_inner, right_outer) = padding(crate::Side::Right);
        assert!(left_outer < left_inner);
        assert_eq!(right_outer, left_outer);
        assert_eq!(right_inner, left_inner);
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

    /// The styled editor projects a presentation for every gutter marker, and
    /// paints them through the editor's own prepaint.
    #[gpui::test]
    fn line_decorations_paint_every_marker_kind(cx: &mut TestAppContext) {
        use crate::IconName;
        use crate::input::{GutterMarker, LineDecoration, LineDecorationProvider};
        use gpui_kit_assets::IconNamed as _;
        use std::{cell::Cell, ops::Range};

        struct EveryKind(Rc<Cell<usize>>);

        impl LineDecorationProvider for EveryKind {
            fn line_decorations(&self, rows: Range<usize>, cx: &App) -> Vec<LineDecoration> {
                self.0.set(self.0.get() + 1);
                let markers = [
                    GutterMarker::DiffAdded,
                    GutterMarker::DiffRemoved,
                    GutterMarker::DiffChanged,
                    GutterMarker::Conflict,
                    GutterMarker::Bookmark,
                    GutterMarker::Breakpoint,
                    GutterMarker::Custom {
                        icon: IconName::Star.path(),
                        color: cx.theme().primary,
                    },
                ];
                rows.zip(markers)
                    .map(|(row, marker)| {
                        LineDecoration::new(row)
                            .with_background(cx.theme().success.opacity(0.16))
                            .with_marker(marker)
                    })
                    .collect()
            }
        }

        cx.update(crate::init);
        let asked = Rc::new(Cell::new(0));
        let mut state = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let editor = cx.new(|cx| EditorState::new(window, cx).default_value("a\n".repeat(7)));
            editor.update(cx, |state, cx| {
                state.create_line_decorations_collection(Rc::new(EveryKind(asked.clone())), cx);
            });
            state = Some(editor.clone());
            Harness {
                state: editor,
                text_size: None,
            }
        });
        let state = state.unwrap();
        VisualTestContext::update(cx, |window, cx| window.draw(cx).clear(cx));

        assert!(asked.get() > 0, "the provider was never asked");
        cx.read(|cx| {
            let first = state
                .read(cx)
                .row_bounds(0)
                .expect("the first row is laid out");
            let second = state
                .read(cx)
                .row_bounds(1)
                .expect("the second row is laid out");
            assert_eq!(second.origin.y, first.bottom());
        });
    }

    #[gpui::test]
    fn gutter_markers_follow_editor_font_changes_without_changing_rem(cx: &mut TestAppContext) {
        use crate::input::{GutterMarker, LineDecoration, LineDecorationProvider};
        use std::ops::Range;

        struct Marker;

        impl LineDecorationProvider for Marker {
            fn line_decorations(&self, rows: Range<usize>, _: &App) -> Vec<LineDecoration> {
                rows.map(|row| LineDecoration::new(row).with_marker(GutterMarker::DiffAdded))
                    .collect()
            }
        }

        cx.update(crate::init);
        let mut editor = None;
        let mut harness = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let state = cx.new(|cx| EditorState::new(window, cx).default_value("first\nsecond"));
            editor = Some(state.clone());
            harness = Some(cx.entity());
            Harness {
                state,
                text_size: None,
            }
        });
        let editor = editor.unwrap();
        let harness = harness.unwrap();
        VisualTestContext::update(cx, |window, cx| {
            let rem_size = window.rem_size();
            let mut previous = None;
            for font_size in [px(11.), px(20.)] {
                harness.update(cx, |view, cx| {
                    view.text_size = Some(font_size);
                    cx.notify();
                });
                window.draw(cx).clear(cx);
                let unmarked = editor.read(cx).range_to_bounds(&(0..0)).unwrap().left();
                let collection = editor.update(cx, |state, cx| {
                    state.create_line_decorations_collection(Rc::new(Marker), cx)
                });
                window.draw(cx).clear(cx);
                let state = editor.read(cx);
                let slot = state.range_to_bounds(&(0..0)).unwrap().left() - unmarked;
                let row_height = state.row_bounds(0).unwrap().size.height;
                assert!(slot > px(0.));
                if let Some((previous_slot, previous_font, previous_height)) = previous {
                    assert!(
                        slot > previous_slot,
                        "the marker slot did not follow the font"
                    );
                    assert!(row_height > previous_height);
                    // Row heights round to device pixels; compare the marker's
                    // scale against font size rather than rounded row height.
                    let expected = previous_slot * (font_size / previous_font);
                    assert!(
                        (slot - expected).abs() < px(0.01),
                        "marker and text scale diverged: slot {slot:?}, previous slot {previous_slot:?}, font {font_size:?}, previous font {previous_font:?}"
                    );
                }
                previous = Some((slot, font_size, row_height));
                assert_eq!(window.rem_size(), rem_size);
                collection.clear(cx);
                window.draw(cx).clear(cx);
                assert_eq!(
                    editor.read(cx).range_to_bounds(&(0..0)).unwrap().left(),
                    unmarked
                );
            }
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

    /// An editor whose right-click menu records what it was opened on, through
    /// `context_menu_with` or, with `with` off, through `context_menu`.
    struct MenuHarness {
        state: Entity<EditorState>,
        with: bool,
        opened: Rc<std::cell::RefCell<Vec<Option<crate::input::InputHit>>>>,
    }

    impl Render for MenuHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let opened = self.opened.clone();
            let editor = Editor::new(&self.state);
            let editor = if self.with {
                editor.context_menu_with(move |menu, _, hit, _, _| {
                    opened.borrow_mut().push(hit);
                    menu
                })
            } else {
                editor.context_menu(move |menu, _, _| {
                    opened.borrow_mut().push(None);
                    menu
                })
            };
            div().size_full().child(editor)
        }
    }

    #[gpui::test]
    fn a_menu_built_with_its_target_knows_the_row_of_the_gutter(cx: &mut TestAppContext) {
        cx.update(crate::init);
        for with in [true, false] {
            let opened: Rc<std::cell::RefCell<Vec<_>>> = Default::default();
            let mut state = None;
            let (_, cx) = cx.add_window_view(|window, cx| {
                let editor =
                    cx.new(|cx| EditorState::new(window, cx).default_value("one\ntwo\nthree"));
                state = Some(editor.clone());
                MenuHarness {
                    state: editor,
                    with,
                    opened: opened.clone(),
                }
            });
            let state = state.unwrap();
            VisualTestContext::update(cx, |window, cx| window.draw(cx).clear(cx));
            let (gutter, text) = cx.read(|cx| {
                let state = state.read(cx);
                let y = state.row_bounds(1).unwrap().center().y;
                let gutter = state.gutter_bounds().unwrap();
                (
                    gpui::point(gutter.center().x, y),
                    gpui::point(gutter.right() + px(10.), y),
                )
            });
            for position in [gutter, text] {
                cx.simulate_mouse_down(position, gpui::MouseButton::Right, Default::default());
                cx.simulate_mouse_up(position, gpui::MouseButton::Right, Default::default());
                cx.run_until_parked();
            }

            let opened = opened.borrow();
            assert_eq!(opened.len(), 2, "with {with}");
            if with {
                let (on_gutter, on_text) = (opened[0].unwrap(), opened[1].unwrap());
                assert!(on_gutter.is_gutter());
                assert_eq!(on_gutter.row(), Some(1));
                assert!(!on_text.is_gutter());
                assert_eq!(on_text.row(), Some(1));
            }
        }
    }
}

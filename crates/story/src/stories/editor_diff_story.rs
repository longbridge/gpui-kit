use std::{ops::Range, rc::Rc};

use gpui_kit::component::{
    ActiveTheme, Side,
    button::Button,
    h_flex,
    input::{
        Editor, EditorState, GutterColumn, GutterMarker, LineDecoration, LineDecorationCollection,
        LineDecorationProvider,
    },
    menu::PopupMenuItem,
    scroll::ScrollbarPlacement,
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Pixels, Render,
    Styled as _, Subscription, Window, div, px,
};

use crate::story_toolbar_group;

const ORIGINAL: &str = include_str!("editor_preview.rs");

struct ChangedLines {
    rows: Rc<Vec<usize>>,
    removed: bool,
}

impl LineDecorationProvider for ChangedLines {
    fn line_decorations(&self, visible: Range<usize>, cx: &App) -> Vec<LineDecoration> {
        let (color, marker) = if self.removed {
            (cx.theme().danger, GutterMarker::DiffRemoved)
        } else {
            (cx.theme().success, GutterMarker::DiffAdded)
        };
        self.rows
            .iter()
            .copied()
            .filter(|row| visible.contains(row))
            .map(|row| {
                LineDecoration::new(row)
                    .with_background(color.opacity(0.12))
                    .with_marker(marker.clone())
            })
            .collect()
    }
}

pub struct EditorDiffStory {
    original: Entity<EditorState>,
    modified: Entity<EditorState>,
    mirrored: bool,
    markers_before_numbers: bool,
    scroll_y: [Pixels; 2],
    _decorations: [LineDecorationCollection; 2],
    _subscriptions: Vec<Subscription>,
}

impl super::Story for EditorDiffStory {
    fn title() -> &'static str {
        "Editor Diff"
    }
    fn description() -> &'static str {
        "Compare two source versions with ordinary or center-facing gutters."
    }
    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl EditorDiffStory {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let modified_text = ORIGINAL
            .replace("progress: 72.", "progress: 84.")
            .replace(
                "Everything is moving on schedule.",
                "One milestone needs attention.",
            )
            .replace(".gap_3()", ".gap_4()");
        let rows: Rc<Vec<usize>> = Rc::new(
            ORIGINAL
                .lines()
                .zip(modified_text.lines())
                .enumerate()
                .filter_map(|(row, (before, after))| (before != after).then_some(row))
                .collect(),
        );
        let original = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("rust")
                .folding(true)
                .soft_wrap(false)
                .gutter_side(Side::Right)
                .scrollbar_placement(ScrollbarPlacement::BottomLeft)
                .default_value(ORIGINAL)
        });
        let modified = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("rust")
                .folding(true)
                .soft_wrap(false)
                .default_value(modified_text)
        });
        let decorations = [(&original, true), (&modified, false)].map(|(editor, removed)| {
            editor.update(cx, |state, cx| {
                state.create_line_decorations_collection(
                    Rc::new(ChangedLines {
                        rows: rows.clone(),
                        removed,
                    }),
                    cx,
                )
            })
        });
        let subscriptions =
            [&original, &modified].map(|editor| cx.observe(editor, |_, _, cx| cx.notify()));
        Self {
            original,
            modified,
            mirrored: true,
            markers_before_numbers: false,
            scroll_y: [px(0.); 2],
            _decorations: decorations,
            _subscriptions: subscriptions.into(),
        }
    }

    fn synchronize_scroll(&mut self, cx: &mut Context<Self>) {
        let current = [
            self.original.read(cx).scroll_offset().y,
            self.modified.read(cx).scroll_offset().y,
        ];
        let changed = if current[0] != self.scroll_y[0] {
            Some(0)
        } else if current[1] != self.scroll_y[1] {
            Some(1)
        } else {
            None
        };
        if let Some(source) = changed {
            let y = current[source];
            let target = if source == 0 {
                &self.modified
            } else {
                &self.original
            };
            if current[1 - source] != y {
                target.update(cx, |state, cx| {
                    let mut offset = state.scroll_offset();
                    offset.y = y;
                    state.set_scroll_offset(offset, cx);
                });
            }
            self.scroll_y = [y; 2];
        }
    }

    fn set_mirrored(&mut self, mirrored: bool, cx: &mut Context<Self>) {
        self.mirrored = mirrored;
        self.original.update(cx, |state, cx| {
            state.set_gutter_side(if mirrored { Side::Right } else { Side::Left }, cx);
            state.set_scrollbar_placement(
                if mirrored {
                    ScrollbarPlacement::BottomLeft
                } else {
                    ScrollbarPlacement::BottomRight
                },
                cx,
            );
        });
        cx.notify();
    }

    fn render_toolbar(&self, cx: &Context<Self>) -> impl IntoElement {
        let story = cx.entity();
        let mirrored = self.mirrored;
        let markers_before_numbers = self.markers_before_numbers;
        story_toolbar_group().dropdown_child(
            Button::new("diff-options").label("Options"),
            move |menu, window, _| {
                [false, true]
                    .into_iter()
                    .fold(menu.label("Gutter layout"), |menu, value| {
                        menu.item(
                            PopupMenuItem::new(if value {
                                "Facing center"
                            } else {
                                "Both on left"
                            })
                            .checked(mirrored == value)
                            .on_click(window.listener_for(
                                &story,
                                move |this, _, _, cx| {
                                    this.set_mirrored(value, cx);
                                },
                            )),
                        )
                    })
                    .separator()
                    .item(
                        PopupMenuItem::new("Markers before line numbers")
                            .checked(markers_before_numbers)
                            .on_click(window.listener_for(&story, |this, _, _, cx| {
                                this.markers_before_numbers = !this.markers_before_numbers;
                                let columns = if this.markers_before_numbers {
                                    [
                                        GutterColumn::FoldIcons,
                                        GutterColumn::Markers,
                                        GutterColumn::LineNumbers,
                                    ]
                                } else {
                                    [
                                        GutterColumn::FoldIcons,
                                        GutterColumn::LineNumbers,
                                        GutterColumn::Markers,
                                    ]
                                };
                                for editor in [&this.original, &this.modified] {
                                    editor.update(cx, |state, cx| {
                                        state.set_gutter_order(columns, cx)
                                    });
                                }
                                cx.notify();
                            })),
                    )
            },
        )
    }

    fn render_pane(
        &self,
        label: &'static str,
        state: &Entity<EditorState>,
        cx: &App,
    ) -> impl IntoElement {
        v_flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .child(
                div()
                    .px_3()
                    .py_2()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(label),
            )
            .child(
                div().flex_1().min_h_0().child(
                    Editor::new(state)
                        .readonly(true)
                        .bordered(false)
                        .size_full(),
                ),
            )
    }
}

impl Render for EditorDiffStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Wheel input notifies the editor; thumb dragging notifies the current
        // Story view. Synchronize both paths before either pane is laid out.
        self.synchronize_scroll(cx);
        v_flex()
            .size_full()
            .gap_3()
            .child(self.render_toolbar(cx))
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(self.render_pane("Original · editor_preview.rs", &self.original, cx))
                    .child(div().w_px().h_full().bg(cx.theme().border))
                    .child(self.render_pane("Modified · editor_preview.rs", &self.modified, cx)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{
        Bounds, ScrollDelta, ScrollEvent, TestAppContext, VisualTestContext, WindowBounds,
        WindowOptions, point, size,
    };

    #[gpui_kit::test]
    fn changing_diff_layout_preserves_sources_and_synchronized_scroll(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (window, story) = cx.update(|cx| {
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(0.), px(0.)),
                        size(px(900.), px(300.)),
                    ))),
                    ..Default::default()
                },
                cx,
                |window, cx| cx.new(|cx| EditorDiffStory::new(window, cx)),
            )
            .unwrap()
        });
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let (original, modified) = story.read_with(&cx, |story, _| {
            (story.original.clone(), story.modified.clone())
        });
        let modified_source = modified.read_with(&cx, |state, _| state.text().to_string());
        assert_ne!(modified_source, ORIGINAL);
        for mirrored in [false, true] {
            story.update(&mut cx, |story, cx| story.set_mirrored(mirrored, cx));
            cx.update(|window, cx| window.draw(cx).clear(cx));
            original.read_with(&cx, |state, _| {
                assert_eq!(
                    state.presentation().gutter_side(),
                    if mirrored { Side::Right } else { Side::Left }
                );
                assert!(state.presentation().is_readonly());
                assert_eq!(state.text().to_string(), ORIGINAL);
            });
            modified.read_with(&cx, |state, _| {
                assert_eq!(state.presentation().gutter_side(), Side::Left);
                assert_eq!(state.text().to_string(), modified_source);
            });
        }
        for (source, target) in [(&original, &modified), (&modified, &original)] {
            let position = source.read_with(&cx, |state, _| state.input_bounds().center());
            cx.simulate_event(ScrollEvent {
                position,
                delta: ScrollDelta::Pixels(point(px(0.), px(-120.))),
                ..Default::default()
            });
            for frame in 0..3 {
                cx.update(|window, cx| window.draw(cx).clear(cx));
                let row =
                    source.read_with(&cx, |state, _| state.visible_row_range().unwrap().start + 1);
                let source_y =
                    source.read_with(&cx, |state, _| state.row_bounds(row).unwrap().top());
                let target_y =
                    target.read_with(&cx, |state, _| state.row_bounds(row).unwrap().top());
                assert_eq!(
                    source_y, target_y,
                    "row {row} is misaligned on frame {frame}"
                );
                cx.run_until_parked();
            }
            let y = source.read_with(&cx, |state, _| state.scroll_offset().y);
            assert!(y < px(0.));
            assert_eq!(target.read_with(&cx, |state, _| state.scroll_offset().y), y);
            cx.simulate_event(ScrollEvent {
                position,
                delta: ScrollDelta::Pixels(point(px(0.), px(120.))),
                ..Default::default()
            });
            for _ in 0..3 {
                cx.update(|window, cx| window.draw(cx).clear(cx));
                cx.run_until_parked();
            }
        }
        for (source, target, left_track) in
            [(&original, &modified, true), (&modified, &original, false)]
        {
            let bounds = source.read_with(&cx, |state, _| state.input_bounds());
            cx.simulate_event(ScrollEvent {
                position: bounds.center(),
                delta: ScrollDelta::Pixels(point(px(0.), px(10000.))),
                ..Default::default()
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            let grab = point(
                if left_track {
                    bounds.left() + px(1.)
                } else {
                    bounds.right() - px(1.)
                },
                bounds.top() + px(8.),
            );
            cx.simulate_mouse_down(
                grab,
                gpui_kit::MouseButton::Left,
                gpui_kit::Modifiers::default(),
            );
            cx.update(|window, cx| window.draw(cx).clear(cx));
            let dragged = grab + point(px(0.), px(20.));
            cx.simulate_mouse_move(
                dragged,
                Some(gpui_kit::MouseButton::Left),
                gpui_kit::Modifiers::default(),
            );
            cx.update(|window, cx| window.draw(cx).clear(cx));
            let y = source.read_with(&cx, |state, _| state.scroll_offset().y);
            assert!(y < px(0.), "the real scrollbar drag must move the pane");
            assert_eq!(
                target.read_with(&cx, |state, _| state.scroll_offset().y),
                y,
                "scrollbar dragging must align both panes on the same frame"
            );
            let row =
                source.read_with(&cx, |state, _| state.visible_row_range().unwrap().start + 1);
            assert_eq!(
                source.read_with(&cx, |state, _| state.row_bounds(row).unwrap().top()),
                target.read_with(&cx, |state, _| state.row_bounds(row).unwrap().top())
            );
            cx.simulate_mouse_up(
                dragged,
                gpui_kit::MouseButton::Left,
                gpui_kit::Modifiers::default(),
            );
        }
    }
}

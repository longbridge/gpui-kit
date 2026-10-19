//! An editor whose gutter has a right-click menu of its own. On a line
//! number the menu is about that line — bookmark it, select it, copy it — and
//! on the text it is the editor's usual one. The selection stays where it is
//! while the gutter's menu is open.

use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
    ops::Range,
    rc::Rc,
};

use gpui_kit::assets::Assets;
use gpui_kit::component::{
    ActiveTheme as _, h_flex,
    input::{
        Editor, EditorState, GutterMarker, LineDecoration, LineDecorationProvider, RopeExt as _,
        default_context_menu,
    },
    native_menu::NativeMenu,
    v_flex,
};
use gpui_kit::*;

actions!(editor_gutter_menu, [ToggleBookmark, SelectLine, CopyLine]);

const TEXT: &str = r#"use std::collections::HashMap;

/// Counts how often each word occurs in `text`.
fn word_counts(text: &str) -> HashMap<&str, usize> {
    let mut counts = HashMap::new();
    for word in text.split_whitespace() {
        *counts.entry(word).or_insert(0) += 1;
    }
    counts
}

fn main() {
    let text = "the quick brown fox jumps over the lazy dog";
    let counts = word_counts(text);
    let mut words: Vec<_> = counts.into_iter().collect();
    words.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    for (word, count) in words {
        println!("{word}: {count}");
    }
}
"#;

/// The bookmarked rows, painted as markers in the gutter.
#[derive(Default)]
struct Bookmarks(RefCell<BTreeSet<usize>>);

impl LineDecorationProvider for Bookmarks {
    fn line_decorations(&self, rows: Range<usize>, _: &App) -> Vec<LineDecoration> {
        self.0
            .borrow()
            .range(rows)
            .map(|&row| LineDecoration::new(row).with_marker(GutterMarker::Bookmark))
            .collect()
    }
}

pub struct Example {
    editor: Entity<EditorState>,
    bookmarks: Rc<Bookmarks>,
    /// The row the gutter's menu was last opened on.
    menu_row: Rc<Cell<Option<usize>>>,
    _subscription: Subscription,
}

impl Example {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .line_number(true)
                .language("rust")
                // Keep the selection while the gutter's menu is open.
                .gutter_context_menu_moves_cursor(false)
                .default_value(TEXT)
        });
        let bookmarks = Rc::new(Bookmarks::default());
        bookmarks.0.borrow_mut().insert(3);
        editor.update(cx, |state, cx| {
            state.create_line_decorations_collection(bookmarks.clone(), cx);
        });
        let subscription = cx.observe(&editor, |_, _, cx| cx.notify());
        Self {
            editor,
            bookmarks,
            menu_row: Rc::default(),
            _subscription: subscription,
        }
    }

    /// The gutter's menu for `row`, zero-based.
    fn line_menu(menu: NativeMenu, row: usize, bookmarked: bool) -> NativeMenu {
        let line = row + 1;
        let bookmark = if bookmarked {
            format!("Remove Bookmark from Line {line}")
        } else {
            format!("Bookmark Line {line}")
        };
        menu.menu(bookmark, Box::new(ToggleBookmark))
            .separator()
            .menu(format!("Select Line {line}"), Box::new(SelectLine))
            .menu(format!("Copy Line {line}"), Box::new(CopyLine))
    }

    /// The byte range of `row`, without its line ending.
    fn line_range(&self, row: usize, cx: &App) -> Range<usize> {
        let text = self.editor.read(cx).text();
        text.line_start_offset(row)..text.line_end_offset(row)
    }

    fn toggle_bookmark(&mut self, _: &ToggleBookmark, _: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.menu_row.get() else {
            return;
        };
        {
            let mut rows = self.bookmarks.0.borrow_mut();
            if !rows.remove(&row) {
                rows.insert(row);
            }
        }
        self.editor.update(cx, |_, cx| cx.notify());
    }

    fn select_line(&mut self, _: &SelectLine, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.menu_row.get() else {
            return;
        };
        let range = self.line_range(row, cx);
        self.editor.update(cx, |state, cx| {
            state.set_selected_range(range, cx);
            state.focus(window, cx);
        });
    }

    fn copy_line(&mut self, _: &CopyLine, _: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.menu_row.get() else {
            return;
        };
        let range = self.line_range(row, cx);
        let line = self.editor.read(cx).text().slice(range).to_string();
        cx.write_to_clipboard(ClipboardItem::new_string(line));
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let menu_row = self.menu_row.clone();
        let bookmarks = self.bookmarks.clone();
        let marked = bookmarks
            .0
            .borrow()
            .iter()
            .map(|row| (row + 1).to_string())
            .collect::<Vec<_>>();
        let selected = self.editor.read(cx).selected_range();
        let status = format!(
            "Bookmarks: {} · Selection: {}..{}",
            if marked.is_empty() {
                "none".into()
            } else {
                marked.join(", ")
            },
            selected.start,
            selected.end,
        );
        v_flex()
            .size_full()
            .p_3()
            .gap_2()
            .on_action(cx.listener(Self::toggle_bookmark))
            .on_action(cx.listener(Self::select_line))
            .on_action(cx.listener(Self::copy_line))
            .child("Right-click a line number for the line's menu, the text for the usual one.")
            .child(div().flex_1().min_h_0().child(
                Editor::new(&self.editor).size_full().context_menu_with(
                    move |menu, capabilities, hit, _, _| {
                        match hit {
                            Some(hit) if hit.is_gutter() => {
                                menu_row.set(hit.row());
                                match hit.row() {
                                    Some(row) => Self::line_menu(
                                        menu,
                                        row,
                                        bookmarks.0.borrow().contains(&row),
                                    ),
                                    // Below the last line: no menu.
                                    None => menu,
                                }
                            }
                            _ => default_context_menu(menu, capabilities),
                        }
                    },
                ),
            ))
            .child(
                h_flex()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(status),
            )
    }
}

fn main() {
    gpui_kit::application().with_assets(Assets).run(move |cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::centered(size(px(720.), px(520.)), cx)),
                ..Default::default()
            },
            cx,
            |window, cx| cx.new(|cx| Example::new(window, cx)),
        )
        .expect("Failed to open window");
    });
}

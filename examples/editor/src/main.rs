use std::{
    ops::Range,
    path::PathBuf,
    rc::Rc,
    str::FromStr,
    sync::{Arc, RwLock},
    time::Duration,
};

use autocorrect::ignorer::Ignorer;
use gpui_component_story::Open;
use gpui_kit::assets::Assets;
use gpui_kit::component::{
    ActiveTheme, IconName, Sizable, WindowExt,
    button::{Button, ButtonVariants as _},
    h_flex,
    highlighter::{Diagnostic, DiagnosticSeverity, Language, LanguageConfig, LanguageRegistry},
    input::{
        self, CodeActionProvider, CompletionProvider, DefinitionProvider, DocumentColorProvider,
        Editor, EditorState, HoverProvider, Input, InputEvent, InputState, Position, Rope, RopeExt,
        TabSize,
    },
    list::ListItem,
    resizable::{h_resizable, resizable_panel},
    status_bar::StatusBar,
    tree::{TreeItem, TreeState, tree},
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder, *};
use lsp_types::{
    CodeAction, CodeActionKind, CompletionContext, CompletionItem, CompletionResponse,
    CompletionTextEdit, InlineCompletionContext, InlineCompletionItem, InlineCompletionResponse,
    InsertReplaceEdit, InsertTextFormat, TextEdit, WorkspaceEdit,
};

enum Lang {
    BuiltIn(Language),
    External(&'static str),
}

impl Lang {
    fn name(&self) -> &str {
        match self {
            Lang::BuiltIn(lang) => lang.name(),
            Lang::External(lang) => lang,
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "nv" => Lang::External("navi"),
            _ => Lang::BuiltIn(Language::from_str(s)),
        }
    }
}

fn init() {
    LanguageRegistry::singleton().register(
        "navi",
        &LanguageConfig::new(
            "navi",
            tree_sitter_navi::LANGUAGE.into(),
            vec![],
            tree_sitter_navi::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
    );
}

pub struct Example {
    editor: Entity<EditorState>,
    tree_state: Entity<TreeState>,
    go_to_line_state: Entity<InputState>,
    language: Lang,
    line_number: bool,
    indent_guides: bool,
    soft_wrap: bool,
    show_whitespaces: bool,
    folding: bool,
    readonly: bool,
    scroll_beyond_last_line: Option<usize>,
    cursor_surrounding_lines: Option<usize>,
    lsp_store: ExampleLspStore,
    _subscriptions: Vec<Subscription>,
    _lint_task: Task<()>,
}

#[derive(Clone)]
pub struct ExampleLspStore {
    completions: Arc<Vec<CompletionItem>>,
    code_actions: Arc<RwLock<Vec<(Range<usize>, CodeAction)>>>,
    diagnostics: Arc<RwLock<Vec<Diagnostic>>>,
    dirty: Arc<RwLock<bool>>,
}

impl ExampleLspStore {
    pub fn new() -> Self {
        let completions = serde_json::from_slice::<Vec<CompletionItem>>(include_bytes!(
            "../fixtures/completion_items.json"
        ))
        .unwrap();

        Self {
            completions: Arc::new(completions),
            code_actions: Arc::new(RwLock::new(vec![])),
            diagnostics: Arc::new(RwLock::new(vec![])),
            dirty: Arc::new(RwLock::new(false)),
        }
    }

    fn diagnostics(&self) -> Vec<Diagnostic> {
        let guard = self.diagnostics.read().unwrap();
        guard.clone()
    }

    fn update_diagnostics(&self, diagnostics: Vec<Diagnostic>) {
        let mut guard = self.diagnostics.write().unwrap();
        *guard = diagnostics;
        *self.dirty.write().unwrap() = true;
    }

    fn code_actions(&self) -> Vec<(Range<usize>, CodeAction)> {
        let guard = self.code_actions.read().unwrap();
        guard.clone()
    }

    fn update_code_actions(&self, code_actions: Vec<(Range<usize>, CodeAction)>) {
        let mut guard = self.code_actions.write().unwrap();
        *guard = code_actions;
        *self.dirty.write().unwrap() = true;
    }

    fn is_dirty(&self) -> bool {
        let guard = self.dirty.read().unwrap();
        *guard
    }
}

fn completion_item(
    replace_range: &lsp_types::Range,
    label: &str,
    replace_text: &str,
    documentation: &str,
) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(lsp_types::CompletionItemKind::FUNCTION),
        text_edit: Some(CompletionTextEdit::InsertAndReplace(InsertReplaceEdit {
            new_text: replace_text.to_string(),
            insert: *replace_range,
            replace: *replace_range,
        })),
        documentation: Some(lsp_types::Documentation::String(documentation.to_string())),
        insert_text: None,
        ..Default::default()
    }
}

impl CompletionProvider for ExampleLspStore {
    fn completions(
        &self,
        rope: &Rope,
        offset: usize,
        trigger: CompletionContext,
        _: &mut Window,
        cx: &mut App,
    ) -> Task<Result<CompletionResponse>> {
        let trigger_character = trigger.trigger_character.unwrap_or_default();
        if trigger_character.is_empty() {
            return Task::ready(Ok(CompletionResponse::Array(vec![])));
        }

        // Simulate to delay for fetching completions
        let rope = rope.clone();
        let items = self.completions.clone();
        cx.background_spawn(async move {
            // Simulate a slow completion source, to test Editor async handling.
            smol::Timer::after(Duration::from_millis(20)).await;

            if trigger_character.starts_with("/") {
                let start = offset.saturating_sub(trigger_character.len());
                let start_pos = rope.offset_to_position(start);
                let end_pos = rope.offset_to_position(offset);
                let replace_range = lsp_types::Range::new(start_pos, end_pos);

                let items = vec![
                    completion_item(
                        &replace_range,
                        "/date",
                        format!("{}", chrono::Local::now().date_naive()).as_str(),
                        "Insert current date",
                    ),
                    completion_item(&replace_range, "/thanks", "Thank you!", "Insert Thank you!"),
                    completion_item(&replace_range, "/+1", "👍", "Insert 👍"),
                    completion_item(&replace_range, "/-1", "👎", "Insert 👎"),
                    completion_item(&replace_range, "/smile", "😊", "Insert 😊"),
                    completion_item(&replace_range, "/sad", "😢", "Insert 😢"),
                    completion_item(&replace_range, "/launch", "🚀", "Insert 🚀"),
                ];
                return Ok(CompletionResponse::Array(items));
            }

            let items = items
                .iter()
                .filter(|item| item.label.starts_with(&trigger_character))
                .take(10)
                .map(|item| {
                    let mut item = item.clone();
                    item.insert_text = Some(item.label.replace(&trigger_character, ""));
                    item
                })
                .collect::<Vec<_>>();

            Ok(CompletionResponse::Array(items))
        })
    }

    fn inline_completion(
        &self,
        rope: &Rope,
        offset: usize,
        _trigger: InlineCompletionContext,
        _window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<InlineCompletionResponse>> {
        let rope = rope.clone();
        cx.background_spawn(async move {
            // Get the current line text before cursor using RopeExt
            let point = rope.offset_to_point(offset);
            let line_start = rope.line_start_offset(point.row);
            let current_line = rope.slice(line_start..offset).to_string();

            // Simple pattern matching for demo
            let suggestion =
                if current_line.trim_start().starts_with("fn ") && !current_line.contains('{') {
                    Some("() {\n    // Write your code here..\n}".into())
                } else {
                    None
                };

            if let Some(insert_text) = suggestion {
                Ok(InlineCompletionResponse::Array(vec![
                    InlineCompletionItem {
                        insert_text,
                        filter_text: None,
                        range: None,
                        command: None,
                        insert_text_format: Some(InsertTextFormat::SNIPPET),
                    },
                ]))
            } else {
                Ok(InlineCompletionResponse::Array(vec![]))
            }
        })
    }

    fn is_completion_trigger(&self, _offset: usize, _new_text: &str, _cx: &mut App) -> bool {
        true
    }
}

impl CodeActionProvider for ExampleLspStore {
    fn id(&self) -> SharedString {
        "LspStore".into()
    }

    fn code_actions(
        &self,
        _state: Entity<EditorState>,
        range: Range<usize>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Task<Result<Vec<CodeAction>>> {
        let mut actions = vec![];
        for (node_range, code_action) in self.code_actions().iter() {
            if !(range.start >= node_range.start && range.end <= node_range.end) {
                continue;
            }

            actions.push(code_action.clone());
        }

        Task::ready(Ok(actions))
    }

    fn perform_code_action(
        &self,
        state: Entity<EditorState>,
        action: CodeAction,
        _push_to_history: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<()>> {
        let Some(edit) = action.edit else {
            return Task::ready(Ok(()));
        };

        let Some((_, text_edits)) = if let Some(changes) = edit.changes {
            changes
        } else {
            return Task::ready(Ok(()));
        }
        .into_iter()
        .next() else {
            return Task::ready(Ok(()));
        };

        let state = state.downgrade();
        window.spawn(cx, async move |cx| {
            state.update_in(cx, |state, window, cx| {
                state.apply_lsp_edits(&text_edits, window, cx);
            })
        })
    }
}

impl HoverProvider for ExampleLspStore {
    fn hover(
        &self,
        text: &Rope,
        offset: usize,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Task<Result<Option<lsp_types::Hover>>> {
        let word = text.word_at(offset);
        if word.is_empty() {
            return Task::ready(Ok(None));
        }

        let Some(item) = self.completions.iter().find(|item| item.label == word) else {
            return Task::ready(Ok(None));
        };

        let contents = if let Some(doc) = &item.documentation {
            match doc {
                lsp_types::Documentation::String(s) => s.clone(),
                lsp_types::Documentation::MarkupContent(mc) => mc.value.clone(),
            }
        } else {
            "No documentation available.".to_string()
        };

        let hover = lsp_types::Hover {
            contents: lsp_types::HoverContents::Scalar(lsp_types::MarkedString::String(contents)),
            range: None,
        };

        Task::ready(Ok(Some(hover)))
    }
}

const RUST_DOC_URLS: &[(&str, &str)] = &[
    ("String", "string/struct.String"),
    ("Debug", "fmt/trait.Debug"),
    ("Clone", "clone/trait.Clone"),
    ("Option", "option/enum.Option"),
    ("Result", "result/enum.Result"),
    ("Vec", "vec/struct.Vec"),
    ("HashMap", "collections/hash_map/struct.HashMap"),
    ("HashSet", "collections/hash_set/struct.HashSet"),
    ("Arc", "sync/struct.Arc"),
    ("RwLock", "sync/struct.RwLock"),
    ("Duration", "time/struct.Duration"),
];

impl DefinitionProvider for ExampleLspStore {
    fn definitions(
        &self,
        text: &Rope,
        offset: usize,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Task<Result<Vec<lsp_types::LocationLink>>> {
        let Some(word_range) = text.word_range(offset) else {
            return Task::ready(Ok(vec![]));
        };
        let word = text.slice(word_range.clone()).to_string();

        let document_uri = lsp_types::Uri::from_str("file://example").unwrap();
        let start = text.offset_to_position(word_range.start);
        let end = text.offset_to_position(word_range.end);
        let symbol_range = lsp_types::Range { start, end };

        if word == "Duration" {
            let target_range = lsp_types::Range {
                start: lsp_types::Position {
                    line: 2,
                    character: 4,
                },
                end: lsp_types::Position {
                    line: 2,
                    character: 23,
                },
            };
            return Task::ready(Ok(vec![lsp_types::LocationLink {
                target_uri: document_uri,
                target_range: target_range,
                target_selection_range: target_range,
                origin_selection_range: Some(symbol_range),
            }]));
        }

        let names = RUST_DOC_URLS
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>();
        for (ix, t) in names.iter().enumerate() {
            if *t == word {
                let url = RUST_DOC_URLS[ix].1;
                let location = lsp_types::LocationLink {
                    target_uri: lsp_types::Uri::from_str(&format!(
                        "https://doc.rust-lang.org/std/{}.html",
                        url
                    ))
                    .unwrap(),
                    target_selection_range: lsp_types::Range::default(),
                    target_range: lsp_types::Range::default(),
                    origin_selection_range: Some(symbol_range),
                };

                return Task::ready(Ok(vec![location]));
            }
        }

        Task::ready(Ok(vec![]))
    }
}

struct TextConvertor;

impl CodeActionProvider for TextConvertor {
    fn id(&self) -> SharedString {
        "TextConvertor".into()
    }

    fn code_actions(
        &self,
        state: Entity<EditorState>,
        range: Range<usize>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Vec<CodeAction>>> {
        let mut actions = vec![];
        if range.is_empty() {
            return Task::ready(Ok(actions));
        }

        let state = state.read(cx);
        let document_uri = lsp_types::Uri::from_str("file://example").unwrap();

        let text = state.text();
        let old_text = text.slice(range.clone()).to_string();
        let start = text.offset_to_position(range.start);
        let end = text.offset_to_position(range.end);
        let range = lsp_types::Range { start, end };

        actions.push(CodeAction {
            title: "Convert to Uppercase".into(),
            kind: Some(CodeActionKind::REFACTOR),
            edit: Some(WorkspaceEdit {
                changes: Some(
                    std::iter::once((
                        document_uri.clone(),
                        vec![TextEdit {
                            range,
                            new_text: old_text.to_uppercase(),
                        }],
                    ))
                    .collect(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        });

        actions.push(CodeAction {
            title: "Convert to Lowercase".into(),
            kind: Some(CodeActionKind::REFACTOR),
            edit: Some(WorkspaceEdit {
                changes: Some(
                    std::iter::once((
                        document_uri.clone(),
                        vec![TextEdit {
                            range,
                            new_text: old_text.to_lowercase(),
                        }],
                    ))
                    .collect(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        });

        actions.push(CodeAction {
            title: "Titleize".into(),
            kind: Some(CodeActionKind::REFACTOR),
            edit: Some(WorkspaceEdit {
                changes: Some(
                    std::iter::once((
                        document_uri.clone(),
                        vec![TextEdit {
                            range,
                            new_text: old_text
                                .split_whitespace()
                                .map(|word| {
                                    let mut chars = word.chars();
                                    chars
                                        .next()
                                        .map(|c| c.to_uppercase().collect::<String>())
                                        .unwrap_or_default()
                                        + chars.as_str()
                                })
                                .collect::<Vec<_>>()
                                .join(" "),
                        }],
                    ))
                    .collect(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        });

        actions.push(CodeAction {
            title: "Capitalize".into(),
            kind: Some(CodeActionKind::REFACTOR),
            edit: Some(WorkspaceEdit {
                changes: Some(
                    std::iter::once((
                        document_uri.clone(),
                        vec![TextEdit {
                            range,
                            new_text: old_text
                                .chars()
                                .enumerate()
                                .map(|(i, c)| {
                                    if i == 0 {
                                        c.to_uppercase().to_string()
                                    } else {
                                        c.to_string()
                                    }
                                })
                                .collect(),
                        }],
                    ))
                    .collect(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        });

        // snake_case
        actions.push(CodeAction {
            title: "Convert to snake_case".into(),
            kind: Some(CodeActionKind::REFACTOR),
            edit: Some(WorkspaceEdit {
                changes: Some(
                    std::iter::once((
                        document_uri.clone(),
                        vec![TextEdit {
                            range,
                            new_text: old_text
                                .chars()
                                .enumerate()
                                .map(|(i, c)| {
                                    if c.is_uppercase() {
                                        if i != 0 {
                                            format!("_{}", c.to_lowercase())
                                        } else {
                                            c.to_lowercase().to_string()
                                        }
                                    } else {
                                        c.to_string()
                                    }
                                })
                                .collect(),
                        }],
                    ))
                    .collect(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        });

        Task::ready(Ok(actions))
    }

    fn perform_code_action(
        &self,
        state: Entity<EditorState>,
        action: CodeAction,
        _push_to_history: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<()>> {
        let Some(edit) = action.edit else {
            return Task::ready(Ok(()));
        };

        let Some((_, text_edits)) = if let Some(changes) = edit.changes {
            changes
        } else {
            return Task::ready(Ok(()));
        }
        .into_iter()
        .next() else {
            return Task::ready(Ok(()));
        };

        let state = state.downgrade();
        window.spawn(cx, async move |cx| {
            state.update_in(cx, |state, window, cx| {
                state.apply_lsp_edits(&text_edits, window, cx);
            })
        })
    }
}

impl DocumentColorProvider for ExampleLspStore {
    fn document_colors(
        &self,
        text: &Rope,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Task<gpui_kit::Result<Vec<lsp_types::ColorInformation>>> {
        let nodes = color_lsp::parse(&text.to_string());
        let colors = nodes
            .into_iter()
            .map(|node| {
                let start = lsp_types::Position::new(node.position.line, node.position.character);
                let end = lsp_types::Position::new(
                    node.position.line,
                    node.position.character + node.matched.chars().count() as u32,
                );

                lsp_types::ColorInformation {
                    range: lsp_types::Range { start, end },
                    color: lsp_types::Color {
                        red: node.color.r,
                        green: node.color.g,
                        blue: node.color.b,
                        alpha: node.color.a,
                    },
                }
            })
            .collect::<Vec<_>>();

        Task::ready(Ok(colors))
    }
}

fn build_file_items(ignorer: &Ignorer, root: &PathBuf, path: &PathBuf) -> Vec<TreeItem> {
    let mut items = Vec::new();

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            let relative_path = path.strip_prefix(root).unwrap_or(&path);
            if ignorer.is_ignored(&relative_path.to_string_lossy())
                || relative_path.ends_with(".git")
            {
                continue;
            }
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();
            let id = path.to_string_lossy().to_string();
            if path.is_dir() {
                let children = build_file_items(ignorer, &root, &path);
                items.push(TreeItem::new(id, file_name).children(children));
            } else {
                items.push(TreeItem::new(id, file_name));
            }
        }
    }
    items.sort_by(|a, b| {
        b.is_folder()
            .cmp(&a.is_folder())
            .then(a.label.cmp(&b.label))
    });
    items
}

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let default_language = Lang::BuiltIn(Language::Rust);
        let lsp_store = ExampleLspStore::new();

        let editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language(default_language.name().to_string())
                .line_number(true)
                .indent_guides(true)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .soft_wrap(false)
                .default_value(include_str!("../fixtures/test.rs"))
                .placeholder("Enter your code here...")
        });

        editor.update(cx, |state, cx| {
            let lsp_store = Rc::new(lsp_store.clone());
            state.lsp_mut().completion_provider = Some(lsp_store.clone());
            state.lsp_mut().code_action_providers = vec![lsp_store.clone(), Rc::new(TextConvertor)];
            state.lsp_mut().hover_provider = Some(lsp_store.clone());
            state.lsp_mut().definition_provider = Some(lsp_store.clone());
            state.lsp_mut().document_color_provider = Some(lsp_store);
            cx.notify();
        });

        // Focus the editor on startup so that actions (e.g. Open) can bubble
        // up through this view's element tree and reach their handlers.
        let focus_handle = editor.focus_handle(cx);
        window.defer(cx, move |window, cx| {
            focus_handle.focus(window, cx);
        });

        let go_to_line_state = cx.new(|cx| InputState::new(window, cx));

        let tree_state = cx.new(|cx| TreeState::new(cx));
        Self::load_files(tree_state.clone(), PathBuf::from("./"), cx);

        let _subscriptions = vec![cx.subscribe(&editor, |this, _editor, _: &InputEvent, cx| {
            this.lint_document(cx);
        })];

        Self {
            editor,
            tree_state,
            go_to_line_state,
            language: default_language,
            line_number: true,
            indent_guides: true,
            soft_wrap: false,
            show_whitespaces: false,
            folding: true,
            readonly: false,
            scroll_beyond_last_line: None,
            cursor_surrounding_lines: None,
            lsp_store,
            _subscriptions,
            _lint_task: Task::ready(()),
        }
    }

    fn load_files(state: Entity<TreeState>, path: PathBuf, cx: &mut App) {
        cx.spawn(async move |cx| {
            let ignorer = Ignorer::new(&path.to_string_lossy());
            let items = build_file_items(&ignorer, &path, &path);
            _ = state.update(cx, |state, cx| {
                state.set_items(items, cx);
            });
        })
        .detach();
    }

    fn go_to_line(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let editor = self.editor.clone();
        let input_state = self.go_to_line_state.clone();

        window.open_alert_dialog(cx, move |dialog, window, cx| {
            input_state.update(cx, |state, cx| {
                let cursor_pos = editor.read(cx).cursor_position();
                state.set_placeholder(
                    format!("{}:{}", cursor_pos.line, cursor_pos.character),
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
                        let column = parts.next().and_then(|c| c).unwrap_or(1);
                        let position = input::Position::new(
                            line.saturating_sub(1) as u32,
                            column.saturating_sub(1) as u32,
                        );

                        editor.update(cx, |state, cx| {
                            state.set_cursor_position(position, window, cx);
                        });

                        true
                    }
                })
        });
    }

    fn lint_document(&mut self, cx: &mut Context<Self>) {
        let language = self.language.name().to_string();
        let lsp_store = self.lsp_store.clone();
        let text = self.editor.read(cx).text().clone();

        self._lint_task = cx.background_spawn(async move {
            let value = text.to_string();
            let result = autocorrect::lint_for(value.as_str(), &language);

            let mut code_actions = vec![];
            let mut diagnostics = vec![];

            for item in result.lines.iter() {
                let severity = match item.severity {
                    autocorrect::Severity::Error => DiagnosticSeverity::Warning,
                    autocorrect::Severity::Warning => DiagnosticSeverity::Hint,
                    autocorrect::Severity::Pass => DiagnosticSeverity::Info,
                };

                let line = item.line.saturating_sub(1); // Convert to 0-based index
                let col = item.col.saturating_sub(1); // Convert to 0-based index

                let start = Position::new(line as u32, col as u32);
                let end = Position::new(line as u32, (col + item.old.chars().count()) as u32);
                let message = format!("AutoCorrect: {}", item.new);
                diagnostics.push(Diagnostic::new(start..end, message).with_severity(severity));

                let range = text.position_to_offset(&start)..text.position_to_offset(&end);

                let text_edit = TextEdit {
                    range: lsp_types::Range { start, end },
                    new_text: item.new.clone(),
                };

                let edit = WorkspaceEdit {
                    changes: Some(
                        std::iter::once((
                            lsp_types::Uri::from_str("file://example").unwrap(),
                            vec![text_edit],
                        ))
                        .collect(),
                    ),
                    ..Default::default()
                };

                code_actions.push((
                    range,
                    CodeAction {
                        title: format!("Change to '{}'", item.new),
                        kind: Some(CodeActionKind::QUICKFIX),
                        edit: Some(edit),
                        ..Default::default()
                    },
                ));
            }

            lsp_store.update_code_actions(code_actions.clone());
            lsp_store.update_diagnostics(diagnostics.clone());
        });
    }

    fn on_action_open(&mut self, _: &Open, window: &mut Window, cx: &mut Context<Self>) {
        let path = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: true,
            multiple: false,
            prompt: Some("Select a source file".into()),
        });

        let view = cx.entity();
        cx.spawn_in(window, async move |_, window| {
            let path = path.await.ok()?.ok()??.iter().next()?.clone();

            window
                .update(|window, cx| Self::open_file(view, path, window, cx))
                .ok()
        })
        .detach();
    }

    fn open_file(
        view: Entity<Self>,
        path: PathBuf,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<()> {
        let language = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default();
        let language = Lang::from_str(&language);
        let content = std::fs::read_to_string(&path)?;

        window
            .spawn(cx, async move |window| {
                _ = view.update_in(window, |this, window, cx| {
                    _ = this.editor.update(cx, |this, cx| {
                        this.set_highlighter(language.name().to_string(), cx);
                        this.set_value(content, window, cx);
                    });

                    this.language = language;
                    cx.notify();
                });
            })
            .detach();

        Ok(())
    }

    fn render_file_tree(&self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        tree(
            &self.tree_state,
            move |ix, entry, _selected, _window, cx| {
                view.update(cx, |_, cx| {
                    let item = entry.item();
                    let icon = if !entry.is_folder() {
                        IconName::File
                    } else if entry.is_expanded() {
                        IconName::FolderOpen
                    } else {
                        IconName::Folder
                    };

                    ListItem::new(ix)
                        .w_full()
                        .rounded(cx.theme().radius)
                        .py_0p5()
                        .px_2()
                        .pl(px(16.) * entry.depth() + px(8.))
                        .child(h_flex().gap_2().child(icon).child(item.label.clone()))
                        .on_click(cx.listener({
                            let item = item.clone();
                            move |_, _, _window, cx| {
                                if item.is_folder() {
                                    return;
                                }

                                Self::open_file(
                                    cx.entity(),
                                    PathBuf::from(item.id.as_str()),
                                    _window,
                                    cx,
                                )
                                .ok();

                                cx.notify();
                            }
                        }))
                })
            },
        )
        .text_sm()
        .p_1()
        .bg(cx.theme().sidebar)
        .text_color(cx.theme().sidebar_foreground)
        .h_full()
    }

    fn render_line_number_button(&self, _: &mut Window, cx: &mut Context<Self>) -> Button {
        Button::new("line-number")
            .when(self.line_number, |this| this.icon(IconName::Check))
            .label("Line Number")
            .ghost()
            .xsmall()
            .on_click(cx.listener(|this, _, window, cx| {
                this.line_number = !this.line_number;
                this.editor.update(cx, |state, cx| {
                    state.set_line_number(this.line_number, window, cx);
                });
                cx.notify();
            }))
    }

    fn render_soft_wrap_button(&self, _: &mut Window, cx: &mut Context<Self>) -> Button {
        Button::new("soft-wrap")
            .ghost()
            .xsmall()
            .when(self.soft_wrap, |this| this.icon(IconName::Check))
            .label("Soft Wrap")
            .on_click(cx.listener(|this, _, window, cx| {
                this.soft_wrap = !this.soft_wrap;
                this.editor.update(cx, |state, cx| {
                    state.set_soft_wrap(this.soft_wrap, window, cx);
                });
                cx.notify();
            }))
    }

    fn render_show_whitespaces_button(&self, _: &mut Window, cx: &mut Context<Self>) -> Button {
        Button::new("show-whitespace")
            .ghost()
            .xsmall()
            .when(self.show_whitespaces, |this| this.icon(IconName::Check))
            .label("Show Whitespaces")
            .on_click(cx.listener(|this, _, window, cx| {
                this.show_whitespaces = !this.show_whitespaces;
                this.editor.update(cx, |state, cx| {
                    state.set_show_whitespaces(this.show_whitespaces, window, cx);
                });
                cx.notify();
            }))
    }

    fn render_indent_guides_button(&self, _: &mut Window, cx: &mut Context<Self>) -> Button {
        Button::new("indent-guides")
            .ghost()
            .xsmall()
            .when(self.indent_guides, |this| this.icon(IconName::Check))
            .label("Indent Guides")
            .on_click(cx.listener(|this, _, window, cx| {
                this.indent_guides = !this.indent_guides;
                this.editor.update(cx, |state, cx| {
                    state.set_indent_guides(this.indent_guides, window, cx);
                });
                cx.notify();
            }))
    }

    fn render_folding_button(&self, _: &mut Window, cx: &mut Context<Self>) -> Button {
        Button::new("folding")
            .ghost()
            .xsmall()
            .when(self.folding, |this| this.icon(IconName::Check))
            .label("Folding")
            .on_click(cx.listener(|this, _, window, cx| {
                this.folding = !this.folding;
                this.editor.update(cx, |state, cx| {
                    state.set_folding(this.folding, window, cx);
                });
                cx.notify();
            }))
    }

    fn render_readonly_button(&self, _: &mut Window, cx: &mut Context<Self>) -> Button {
        Button::new("readonly")
            .ghost()
            .xsmall()
            .when(self.readonly, |this| this.icon(IconName::Check))
            .label("Read only")
            .on_click(cx.listener(|this, _, _window, cx| {
                this.readonly = !this.readonly;
                cx.notify();
            }))
    }

    /// Cycle an `Option<usize>` row setting through a few demo values.
    fn cycle_rows(v: Option<usize>) -> Option<usize> {
        match v {
            None => Some(0),
            Some(0) => Some(3),
            Some(3) => Some(8),
            _ => None,
        }
    }

    fn rows_label(v: Option<usize>) -> String {
        match v {
            None => "default".to_string(),
            Some(n) => n.to_string(),
        }
    }

    fn render_scroll_beyond_last_line_button(
        &self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new("scroll-beyond-last-line")
            .ghost()
            .xsmall()
            .label(format!(
                "Scroll Beyond: {}",
                Self::rows_label(self.scroll_beyond_last_line)
            ))
            .on_click(cx.listener(|this, _, window, cx| {
                this.scroll_beyond_last_line = Self::cycle_rows(this.scroll_beyond_last_line);
                this.editor.update(cx, |state, cx| {
                    state.set_scroll_beyond_last_line(this.scroll_beyond_last_line, window, cx);
                });
                cx.notify();
            }))
    }

    fn render_cursor_surrounding_lines_button(
        &self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new("cursor-surrounding-lines")
            .ghost()
            .xsmall()
            .label(format!(
                "Cursor Surrounding: {}",
                Self::rows_label(self.cursor_surrounding_lines)
            ))
            .on_click(cx.listener(|this, _, window, cx| {
                this.cursor_surrounding_lines = Self::cycle_rows(this.cursor_surrounding_lines);
                this.editor.update(cx, |state, cx| {
                    state.set_cursor_surrounding_lines(this.cursor_surrounding_lines, window, cx);
                });
                cx.notify();
            }))
    }

    fn render_go_to_line_button(&self, _: &mut Window, cx: &mut Context<Self>) -> Button {
        let position = self.editor.read(cx).cursor_position();
        let cursor = self.editor.read(cx).cursor();

        Button::new("line-column")
            .ghost()
            .xsmall()
            .label(format!(
                "{}:{} ({} byte)",
                position.line + 1,
                position.character + 1,
                cursor
            ))
            .tooltip("Go to Line/Column")
            .on_click(cx.listener(Self::go_to_line))
    }
}

impl Render for Example {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Update diagnostics
        if self.lsp_store.is_dirty() {
            let diagnostics = self.lsp_store.diagnostics();
            self.editor.update(cx, |state, cx| {
                if let Some(set) = state.diagnostics_mut() {
                    set.clear();
                    set.extend(diagnostics);
                }
                cx.notify();
            });
        }

        v_flex()
            .id("app")
            .size_full()
            .on_action(cx.listener(Self::on_action_open))
            .child(
                v_flex()
                    .id("source")
                    .w_full()
                    .flex_1()
                    .child(
                        h_resizable("editor-container")
                            .child(
                                resizable_panel()
                                    .size(px(240.))
                                    .child(self.render_file_tree(window, cx)),
                            )
                            .child(
                                Editor::new(&self.editor)
                                    .readonly(self.readonly)
                                    .bordered(false)
                                    .p_0()
                                    .h(relative(1.))
                                    .font_family(cx.theme().mono_font_family.clone())
                                    .text_size(cx.theme().mono_font_size)
                                    .into_any_element(),
                            ),
                    )
                    .child(
                        StatusBar::new()
                            .left(self.render_line_number_button(window, cx))
                            .left(self.render_soft_wrap_button(window, cx))
                            .left(self.render_show_whitespaces_button(window, cx))
                            .left(self.render_indent_guides_button(window, cx))
                            .left(self.render_folding_button(window, cx))
                            .left(self.render_readonly_button(window, cx))
                            .left(self.render_scroll_beyond_last_line_button(window, cx))
                            .left(self.render_cursor_surrounding_lines_button(window, cx))
                            .right(self.render_go_to_line_button(window, cx)),
                    ),
            )
    }
}

fn main() {
    let app = gpui_kit::application().with_assets(Assets);

    app.run(move |cx| {
        gpui_component_story::init(cx);
        init();
        cx.activate(true);

        gpui_component_story::create_new_window_with_size(
            "Editor",
            Some(size(px(1200.), px(750.))),
            |window, cx| cx.new(|cx| Example::new(window, cx)),
            cx,
        );
    });
}

//! Real terminal pane backed by `libghostty-vt` (parsing, state, rendering
//! snapshot, key encoding) and `portable-pty` (shell process).
//!
//! All libghostty-vt types are `!Send`, so the state machine, render
//! snapshot, and key encoder live together in `TerminalState` on the main
//! thread; the only extra thread is the PTY reader, which just ships bytes.

use std::{
    cell::Cell,
    io::{Read as _, Write},
    path::{Path, PathBuf},
    rc::Rc,
};

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;
use portable_pty::{Child, CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem as _};

use libghostty_vt as gvt;

const FONT_SIZE: Pixels = px(13.);
const DEFAULT_COLS: u16 = 80;
const DEFAULT_ROWS: u16 = 24;
const MAX_SCROLLBACK: usize = 10_000;
const DEFAULT_BG: u32 = 0x17191e;
const DEFAULT_FG: u32 = 0xd4d9de;
const SELECTION_BG: u32 = 0x3b5361;
const CURSOR_COLOR: u32 = 0xd4d9de;
const CELL_HEIGHT: Pixels = px(13. * 1.2);

/// A rendered snapshot of the terminal, rebuilt after each PTY read.
/// The paint pass only reads this; it never touches libghostty.
#[derive(Clone)]
pub struct Frame {
    lines: Vec<LineOut>,
    cursor: Option<(u16, u16)>,
    cursor_color: Hsla,
    bg: Hsla,
}

#[derive(Clone)]
pub struct LineOut {
    runs: Vec<RunOut>,
    /// (start, end) columns highlighted by the terminal's selection, inclusive.
    selection: Option<(u16, u16)>,
}

#[derive(Clone)]
pub struct RunOut {
    text: String,
    col_start: u16,
    cols: u16,
    fg: Hsla,
    bg: Option<Hsla>,
    bold: bool,
    italic: bool,
    underline: bool,
}

/// Events a terminal view sends to whatever hosts it (the dock panel).
pub enum TerminalEvent {
    /// The running program rang the bell (BEL, 0x07) while the view had no
    /// keyboard focus, so nobody saw it.
    Bell,
    /// The view gained keyboard focus.
    Focused,
}

impl EventEmitter<TerminalEvent> for TerminalState {}

pub struct TerminalState {
    terminal: gvt::Terminal<'static, 'static>,
    render_state: gvt::RenderState<'static>,
    row_iter: gvt::render::RowIterator<'static>,
    cell_iter: gvt::render::CellIterator<'static>,
    key_encoder: gvt::key::Encoder<'static>,
    /// Shared with the `on_pty_write` response callback; main-thread only.
    pty_writer: Rc<std::cell::RefCell<Box<dyn Write + Send>>>,
    pty_master: Box<dyn MasterPty + Send>,
    child: Option<Box<dyn Child + Send + Sync>>,
    anchor: Option<gvt::screen::TrackedGridRef>,
    has_selection: bool,
    dragging: bool,
    hovered_uri: Option<String>,
    /// Last known element bounds, for mouse → cell conversion.
    bounds: Bounds<Pixels>,
    pub focus_handle: FocusHandle,
    /// Raised by the libghostty bell callback (it fires inside `vt_write`,
    /// where no gpui `Context` exists to emit from); drained by [`ingest`],
    /// the one place that holds a context. `Rc` because the whole terminal
    /// is main-thread only.
    bell_rang: Rc<Cell<bool>>,
    /// Whether this view currently holds keyboard focus.
    focused: bool,
    /// Focus listeners, installed on the first render because the
    /// constructor runs without a `Window`. Held so they stay subscribed.
    _focus_listeners: Option<(Subscription, Subscription)>,
    cols: u16,
    rows: u16,
    cell_w: Pixels,
    cell_h: Pixels,
    frame: Frame,
}

impl TerminalState {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self::new_in(cx, None)
    }

    /// Spawn a terminal whose login shell starts in `cwd`; `None` keeps the
    /// shell's own default (the user's home).
    pub fn new_in(cx: &mut Context<Self>, cwd: Option<PathBuf>) -> Self {
        let mut terminal = gvt::Terminal::new(gvt::TerminalOptions {
            cols: DEFAULT_COLS,
            rows: DEFAULT_ROWS,
            max_scrollback: MAX_SCROLLBACK,
        })
        .expect("failed to create libghostty-vt Terminal");

        let bg = hex_hsla(DEFAULT_BG);
        let fg = hex_hsla(DEFAULT_FG);
        terminal.set_default_bg_color(Some(to_gvt(bg))).ok();
        terminal.set_default_fg_color(Some(to_gvt(fg))).ok();

        let (pty_master, writer, child) = spawn_shell(DEFAULT_COLS, DEFAULT_ROWS, cwd.as_deref())
            .expect("failed to spawn shell in PTY");

        // Responses the terminal generates (DA queries etc.) go back to the PTY.
        let response_writer = Rc::new(std::cell::RefCell::new(writer));
        let sink = response_writer.clone();
        terminal
            .on_pty_write(move |_terminal, data| {
                let _ = sink.borrow_mut().write_all(data);
            })
            .expect("failed to install pty write callback");

        // BEL (0x07): libghostty invokes the callback synchronously during
        // `vt_write`, where no gpui context exists, so it only raises the
        // flag; `ingest` turns the flag into a [`TerminalEvent::Bell`].
        let bell_rang = Rc::new(Cell::new(false));
        let bell_sink = bell_rang.clone();
        terminal
            .on_bell(move |_| bell_sink.set(true))
            .expect("failed to install bell callback");

        // Reader thread: the only thread besides main; just ships bytes.
        let (tx, rx) = smol::channel::unbounded::<Vec<u8>>();
        let mut reader = pty_master
            .try_clone_reader()
            .expect("failed to clone pty reader");
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.send_blocking(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        cx.spawn(async move |this, cx| {
            loop {
                let Ok(mut batch) = rx.recv().await else {
                    break;
                };
                // Coalesce bursts so a flood of PTY chunks costs one rebuild.
                while let Ok(more) = rx.try_recv() {
                    batch.extend_from_slice(&more);
                }
                let _ = this.update(cx, |state, cx| state.ingest(&batch, cx));
            }
            // Shell exited; reap it off-thread so it doesn't linger as a zombie.
            let _ = this.update(cx, |state, cx| {
                if let Some(mut child) = state.child.take() {
                    cx.background_spawn(async move {
                        let _ = child.wait();
                    })
                    .detach();
                }
            });
        })
        .detach();

        Self {
            terminal,
            render_state: gvt::RenderState::new().expect("failed to create RenderState"),
            row_iter: gvt::render::RowIterator::new().expect("failed to create RowIterator"),
            cell_iter: gvt::render::CellIterator::new().expect("failed to create CellIterator"),
            key_encoder: gvt::key::Encoder::new().expect("failed to create key Encoder"),
            pty_writer: response_writer,
            pty_master,
            child: Some(child),
            anchor: None,
            has_selection: false,
            dragging: false,
            hovered_uri: None,
            bounds: Bounds::default(),
            focus_handle: cx.focus_handle(),
            bell_rang,
            focused: false,
            _focus_listeners: None,
            cols: DEFAULT_COLS,
            rows: DEFAULT_ROWS,
            cell_w: px(7.7),
            cell_h: CELL_HEIGHT,
            frame: Frame {
                lines: Vec::new(),
                cursor: None,
                cursor_color: hex_hsla(CURSOR_COLOR),
                bg,
            },
        }
    }

    fn ingest(&mut self, bytes: &[u8], cx: &mut Context<Self>) {
        self.terminal.vt_write(bytes);
        // Drain the flag the bell callback raised. A focused terminal already
        // has the user's attention, so only an unfocused bell becomes an
        // event; the panel decides what to show for it.
        if self.bell_rang.replace(false) && !self.focused {
            cx.emit(TerminalEvent::Bell);
        }
        self.rebuild_frame();
        cx.notify();
    }

    /// Rebuild the paint snapshot from the terminal's render state.
    ///
    /// # ponytail: full rebuild of every row on every PTY read; switch to
    /// per-row `dirty()` skipping if profiling ever demands it.
    fn rebuild_frame(&mut self) {
        let Self {
            terminal,
            render_state,
            row_iter,
            cell_iter,
            frame,
            ..
        } = self;

        let Ok(snap) = render_state.update(terminal) else {
            return;
        };
        let Ok(colors) = snap.colors() else {
            return;
        };
        let fg = to_hsla(colors.foreground);
        let bg = to_hsla(colors.background);
        let cursor_color = colors.cursor.map(to_hsla).unwrap_or(fg);
        let cursor = snap
            .cursor_viewport()
            .ok()
            .flatten()
            .map(|c| (c.x, c.y))
            .filter(|_| snap.cursor_visible().unwrap_or(true));

        let Ok(mut rows) = row_iter.update(&snap) else {
            return;
        };
        let mut lines = Vec::new();
        while let Some(row) = rows.next() {
            let selection = row.selection().ok().flatten().map(|s| (s.start_x, s.end_x));
            let Ok(mut cells) = cell_iter.update(row) else {
                continue;
            };
            let mut runs: Vec<RunOut> = Vec::new();
            let mut col: u16 = 0;
            while let Some(cell) = cells.next() {
                let Ok(style) = cell.style() else {
                    continue;
                };
                let mut fg_color = cell.fg_color().ok().flatten();
                let mut bg_color = cell.bg_color().ok().flatten();
                if style.inverse {
                    std::mem::swap(&mut fg_color, &mut bg_color);
                }
                let cell_fg = fg_color
                    .map(to_hsla)
                    .unwrap_or(if style.inverse { bg } else { fg });
                let cell_bg = if style.inverse {
                    Some(bg_color.map(to_hsla).unwrap_or(fg))
                } else {
                    bg_color.map(to_hsla)
                };
                let mut text = String::new();
                let _ = cell.graphemes_utf8(&mut text);
                let width = cell.graphemes_len().unwrap_or(1).max(1) as u16;
                let bold = style.bold;
                let italic = style.italic;
                let underline = style.underline != gvt::style::Underline::None;

                match runs.last_mut() {
                    Some(run)
                        if run.bold == bold
                            && run.italic == italic
                            && run.underline == underline
                            && run.fg == cell_fg
                            && run.bg == cell_bg
                            && run.col_start + run.cols == col =>
                    {
                        run.text.push_str(&text);
                        run.cols += width;
                    }
                    _ => runs.push(RunOut {
                        text,
                        col_start: col,
                        cols: width,
                        fg: cell_fg,
                        bg: cell_bg,
                        bold,
                        italic,
                        underline,
                    }),
                }
                col += width;
            }
            lines.push(LineOut { runs, selection });
        }

        *frame = Frame {
            lines,
            cursor,
            cursor_color,
            bg,
        };
    }

    /// Sync the terminal grid (and the shell's window size) to element bounds.
    fn sync_size(&mut self, cols: u16, rows: u16, cx: &mut Context<Self>) {
        if (cols, rows) == (self.cols, self.rows) {
            return;
        }
        let _ = self.terminal.resize(
            cols,
            rows,
            self.cell_w.to_f64() as u32,
            self.cell_h.to_f64() as u32,
        );
        let _ = self.pty_master.resize(PtySize {
            rows,
            cols,
            pixel_width: (self.cell_w.to_f64() * cols as f64) as u16,
            pixel_height: (self.cell_h.to_f64() * rows as f64) as u16,
        });
        self.cols = cols;
        self.rows = rows;
        self.terminal.set_selection(None).ok();
        self.has_selection = false;
        self.anchor = None;
        self.rebuild_frame();
        cx.notify();
    }

    // ---- Keyboard ----

    fn on_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;

        // App-level shortcuts intercepted before the encoder.
        if keystroke.modifiers.platform {
            match keystroke.key.as_ref() {
                "c" if self.has_selection => {
                    self.copy_selection(cx);
                    return;
                }
                "v" => {
                    self.paste_from_clipboard(cx);
                    return;
                }
                _ => {}
            }
        }

        // Bare modifier presses produce no terminal output.
        if matches!(
            keystroke.key.as_ref(),
            "shift" | "control" | "alt" | "platform" | "function" | "capslock"
        ) {
            return;
        }

        let mut event = gvt::key::Event::new().expect("failed to create key Event");
        to_key_event(keystroke, &mut event);
        self.key_encoder.set_options_from_terminal(&self.terminal);
        let mut encoded = Vec::with_capacity(16);
        if self.key_encoder.encode_to_vec(&event, &mut encoded).is_ok() && !encoded.is_empty() {
            let _ = self.pty_writer.borrow_mut().write_all(&encoded);
        }
    }

    fn copy_selection(&mut self, cx: &mut Context<Self>) {
        let bytes = self
            .terminal
            .format_selection_alloc(
                None,
                gvt::selection::FormatOptions::new()
                    .with_emit_format(gvt::fmt::Format::Plain)
                    .with_unwrap(true)
                    .with_trim(true),
            )
            .ok()
            .flatten();
        if let Some(bytes) = bytes {
            if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            }
        }
    }

    fn paste_from_clipboard(&mut self, cx: &mut Context<Self>) {
        let Some(item) = cx.read_from_clipboard() else {
            return;
        };
        let text = item.text().unwrap_or_default();
        if text.is_empty() {
            return;
        }
        // The library encoder strips ESC sequences and applies bracketed
        // paste (mode 2004) when the application requested it, so multi-line
        // pastes stay safe.
        let bracketed = self
            .terminal
            .mode(gvt::terminal::Mode::BRACKETED_PASTE)
            .unwrap_or(false);
        let mut data = text.into_bytes();
        let mut buf = vec![0u8; data.len() * 2 + 32];
        match gvt::paste::encode(&mut data, bracketed, &mut buf) {
            Ok(n) => {
                let _ = self.pty_writer.borrow_mut().write_all(&buf[..n]);
            }
            Err(_) => {}
        }
    }

    // ---- Focus ----

    fn on_focus_gained(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.focused = true;
        // Focus landing back here is what reads away an unread bell.
        cx.emit(TerminalEvent::Focused);
    }

    fn on_focus_lost(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.focused = false;
    }

    // ---- Mouse ----

    fn cell_at(&self, position: Point<Pixels>) -> (u16, u32) {
        let col = (((position.x - self.bounds.origin.x) / self.cell_w).floor() as i64)
            .clamp(0, self.cols as i64 - 1) as u16;
        let row = (((position.y - self.bounds.origin.y) / self.cell_h).floor() as i64)
            .clamp(0, self.rows as i64 - 1) as u32;
        (col, row)
    }

    fn viewport_point(&self, position: Point<Pixels>) -> gvt::terminal::Point {
        let (x, y) = self.cell_at(position);
        gvt::terminal::Point::Viewport(gvt::terminal::PointCoordinate { x, y })
    }

    pub fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        if self.terminal.is_mouse_tracking().unwrap_or(false) {
            return; // application wants the mouse; local selection would fight it
        }
        let point = self.viewport_point(event.position);
        match self.terminal.track_grid_ref(point) {
            Ok(anchor) => self.anchor = Some(anchor),
            Err(_) => self.anchor = None,
        }
        let _ = self.terminal.set_selection(None);
        self.has_selection = false;
        self.dragging = true;
        cx.notify();
    }

    pub fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Hover feedback for OSC 8 hyperlinks.
        let uri = self.hyperlink_at(event.position);
        if uri.as_deref() != self.hovered_uri.as_deref() {
            self.hovered_uri = uri;
            cx.notify();
        }

        if !self.dragging {
            return;
        }
        // The mouse-up may land outside this element (GPUI gates on hover),
        // so a buttonless move ends the drag.
        if event.pressed_button != Some(MouseButton::Left) {
            self.dragging = false;
            return;
        }
        let Some(anchor) = &self.anchor else {
            return;
        };
        let Ok(Some(start)) = anchor.snapshot(&self.terminal) else {
            return;
        };
        let Ok(end) = self.terminal.grid_ref(self.viewport_point(event.position)) else {
            return;
        };
        let selection = gvt::selection::Selection::new(start, end, false);
        let _ = self.terminal.set_selection(Some(&selection));
        self.has_selection = true;
        cx.notify();
    }

    pub fn on_mouse_up(
        &mut self,
        event: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dragging = false;
        // Click (no drag) on a hyperlink opens it.
        if !self.has_selection {
            if let Some(uri) = self.hyperlink_at(event.position) {
                cx.open_url(&uri);
            }
        }
    }

    pub fn on_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let lines = (event.delta.pixel_delta(self.cell_h).y / self.cell_h).round() as isize;
        if lines != 0 {
            self.terminal
                .scroll_viewport(gvt::terminal::ScrollViewport::Delta(-lines));
            cx.notify();
        }
    }

    fn hyperlink_at(&self, position: Point<Pixels>) -> Option<String> {
        let grid = self.terminal.grid_ref(self.viewport_point(position)).ok()?;
        let mut buf = [0u8; 2048];
        let n = grid.hyperlink_uri(&mut buf).ok()?;
        if n == 0 {
            None
        } else {
            Some(String::from_utf8_lossy(&buf[..n]).into_owned())
        }
    }
}

/// GPUI keystroke → libghostty key event.
fn to_key_event(keystroke: &Keystroke, event: &mut gvt::key::Event) {
    let mods = keystroke.modifiers;
    let mut gvt_mods = gvt::key::Mods::empty();
    gvt_mods.set(gvt::key::Mods::SHIFT, mods.shift);
    gvt_mods.set(gvt::key::Mods::ALT, mods.alt);
    gvt_mods.set(gvt::key::Mods::CTRL, mods.control);
    gvt_mods.set(gvt::key::Mods::SUPER, mods.platform);
    event.set_mods(gvt_mods);
    event.set_action(gvt::key::Action::Press);
    event.set_key(logical_key(keystroke));
    // Text input: printable chars go through utf8; control keys are derived
    // from the logical key by the encoder.
    event.set_utf8(keystroke.key_char.as_ref().and_then(|c| {
        let mut chars = c.chars();
        match (chars.next(), chars.next()) {
            (Some(ch), None) if !ch.is_control() => Some(ch.to_string()),
            _ => None,
        }
    }));
}

fn logical_key(keystroke: &Keystroke) -> gvt::key::Key {
    let key: &str = &keystroke.key;
    match key {
        "enter" => gvt::key::Key::Enter,
        "tab" => gvt::key::Key::Tab,
        "backspace" => gvt::key::Key::Backspace,
        "escape" => gvt::key::Key::Escape,
        "delete" => gvt::key::Key::Delete,
        "left" => gvt::key::Key::ArrowLeft,
        "right" => gvt::key::Key::ArrowRight,
        "up" => gvt::key::Key::ArrowUp,
        "down" => gvt::key::Key::ArrowDown,
        "home" => gvt::key::Key::Home,
        "end" => gvt::key::Key::End,
        "pageup" => gvt::key::Key::PageUp,
        "pagedown" => gvt::key::Key::PageDown,
        "f1" => gvt::key::Key::F1,
        "f2" => gvt::key::Key::F2,
        "f3" => gvt::key::Key::F3,
        "f4" => gvt::key::Key::F4,
        "f5" => gvt::key::Key::F5,
        "f6" => gvt::key::Key::F6,
        "f7" => gvt::key::Key::F7,
        "f8" => gvt::key::Key::F8,
        "f9" => gvt::key::Key::F9,
        "f10" => gvt::key::Key::F10,
        "f11" => gvt::key::Key::F11,
        "f12" => gvt::key::Key::F12,
        "space" => gvt::key::Key::Space,
        "-" => gvt::key::Key::Minus,
        "=" => gvt::key::Key::Equal,
        "[" => gvt::key::Key::BracketLeft,
        "]" => gvt::key::Key::BracketRight,
        "\\" => gvt::key::Key::Backslash,
        "`" => gvt::key::Key::Backquote,
        "," => gvt::key::Key::Comma,
        "." => gvt::key::Key::Period,
        ";" => gvt::key::Key::Semicolon,
        "'" => gvt::key::Key::Quote,
        "/" => gvt::key::Key::Slash,
        single => {
            let mut chars = single.chars();
            if let (Some(c), None) = (chars.next(), chars.next()) {
                match c {
                    'a'..='z' | 'A'..='Z' => {
                        return letter_key(c.to_ascii_uppercase());
                    }
                    '0'..='9' => {
                        return digit_key(c);
                    }
                    _ => {}
                }
            }
            gvt::key::Key::Unidentified
        }
    }
}

fn letter_key(c: char) -> gvt::key::Key {
    match c {
        'A' => gvt::key::Key::A,
        'B' => gvt::key::Key::B,
        'C' => gvt::key::Key::C,
        'D' => gvt::key::Key::D,
        'E' => gvt::key::Key::E,
        'F' => gvt::key::Key::F,
        'G' => gvt::key::Key::G,
        'H' => gvt::key::Key::H,
        'I' => gvt::key::Key::I,
        'J' => gvt::key::Key::J,
        'K' => gvt::key::Key::K,
        'L' => gvt::key::Key::L,
        'M' => gvt::key::Key::M,
        'N' => gvt::key::Key::N,
        'O' => gvt::key::Key::O,
        'P' => gvt::key::Key::P,
        'Q' => gvt::key::Key::Q,
        'R' => gvt::key::Key::R,
        'S' => gvt::key::Key::S,
        'T' => gvt::key::Key::T,
        'U' => gvt::key::Key::U,
        'V' => gvt::key::Key::V,
        'W' => gvt::key::Key::W,
        'X' => gvt::key::Key::X,
        'Y' => gvt::key::Key::Y,
        _ => gvt::key::Key::Z,
    }
}

fn digit_key(c: char) -> gvt::key::Key {
    match c {
        '0' => gvt::key::Key::Digit0,
        '1' => gvt::key::Key::Digit1,
        '2' => gvt::key::Key::Digit2,
        '3' => gvt::key::Key::Digit3,
        '4' => gvt::key::Key::Digit4,
        '5' => gvt::key::Key::Digit5,
        '6' => gvt::key::Key::Digit6,
        '7' => gvt::key::Key::Digit7,
        '8' => gvt::key::Key::Digit8,
        _ => gvt::key::Key::Digit9,
    }
}

fn spawn_shell(
    cols: u16,
    rows: u16,
    cwd: Option<&Path>,
) -> anyhow::Result<(
    Box<dyn MasterPty + Send>,
    Box<dyn Write + Send>,
    Box<dyn Child + Send + Sync>,
)> {
    let pty_system = NativePtySystem::default();
    let pair = pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })?;
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    let mut cmd = CommandBuilder::new(shell);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    if let Some(cwd) = cwd {
        cmd.cwd(cwd);
    }
    let child = pair.slave.spawn_command(cmd)?;
    let writer = pair.master.take_writer()?;
    Ok((pair.master, writer, child))
}

fn hex_hsla(hex: u32) -> Hsla {
    Hsla::from(gpui::rgb(hex))
}

fn to_gvt(color: Hsla) -> gvt::style::RgbColor {
    let rgba = Rgba::from(color);
    gvt::style::RgbColor {
        r: (rgba.r * 255.0).round() as u8,
        g: (rgba.g * 255.0).round() as u8,
        b: (rgba.b * 255.0).round() as u8,
    }
}

fn to_hsla(color: gvt::style::RgbColor) -> Hsla {
    hex_hsla(((color.r as u32) << 16) | ((color.g as u32) << 8) | color.b as u32)
}

// ---- Element ----

/// The paint-only terminal element; event handling lives on the wrapping
/// div built by [`view`].
struct TerminalElement {
    state: Entity<TerminalState>,
}

impl TerminalElement {
    fn mono_font(&self, cx: &App) -> Font {
        Font {
            family: cx.theme().mono_font_family.clone(),
            ..Default::default()
        }
    }
}

impl IntoElement for TerminalElement {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for TerminalElement {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style {
            size: Size::full(),
            flex_shrink: 1.,
            ..Default::default()
        };
        let id = window.request_layout(style, [], cx);
        (id, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);

        // Measure the monospace cell once, then sync the grid to the bounds.
        let font = self.mono_font(cx);
        let run = TextRun {
            len: 1,
            font: font.clone(),
            color: black(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let cell_w = window
            .text_system()
            .shape_line(SharedString::from("M"), FONT_SIZE, &[run], None)
            .width();
        let cell_h = CELL_HEIGHT;
        let cols = ((bounds.size.width / cell_w).floor() as u16).max(2);
        let rows = ((bounds.size.height / cell_h).floor() as u16).max(2);

        self.state.update(cx, |state, cx| {
            state.cell_w = cell_w;
            state.cell_h = cell_h;
            state.bounds = bounds;
            state.sync_size(cols, rows, cx);
        });
        hitbox
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let state = self.state.read(cx);
        let frame = state.frame.clone();
        let cell_w = state.cell_w;
        let cell_h = state.cell_h;
        let font = self.mono_font(cx);
        let hovered = state.hovered_uri.is_some();

        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            // Surface background.
            window.paint_quad(fill(bounds, frame.bg));

            for (row_ix, line) in frame.lines.iter().enumerate() {
                let y = bounds.origin.y + cell_h * row_ix as f32;

                if let Some((start, end)) = line.selection {
                    let x = bounds.origin.x + cell_w * start as f32;
                    let width = cell_w * (end - start + 1) as f32;
                    window.paint_quad(fill(
                        Bounds::new(point(x, y), size(width, cell_h)),
                        hex_hsla(SELECTION_BG),
                    ));
                }

                for run in &line.runs {
                    if let Some(bg) = run.bg {
                        let x = bounds.origin.x + cell_w * run.col_start as f32;
                        let width = cell_w * run.cols as f32;
                        window.paint_quad(fill(Bounds::new(point(x, y), size(width, cell_h)), bg));
                    }

                    let mut style_font = font.clone();
                    if run.bold {
                        style_font.weight = FontWeight::BOLD;
                    }
                    if run.italic {
                        style_font.style = FontStyle::Italic;
                    }
                    let text_run = TextRun {
                        len: run.text.len(),
                        font: style_font,
                        color: run.fg,
                        background_color: None,
                        underline: run.underline.then(|| UnderlineStyle {
                            thickness: px(1.),
                            color: Some(run.fg),
                            wavy: false,
                        }),
                        strikethrough: None,
                    };
                    let line_layout = window.text_system().shape_line(
                        SharedString::from(run.text.clone()),
                        FONT_SIZE,
                        &[text_run],
                        None,
                    );
                    let _ = line_layout.paint(
                        point(bounds.origin.x + cell_w * run.col_start as f32, y),
                        FONT_SIZE,
                        TextAlign::Left,
                        None,
                        window,
                        cx,
                    );
                }
            }

            // Cursor (hidden while the viewport is scrolled off the active area).
            if let Some((cx_col, cy_row)) = frame.cursor {
                let x = bounds.origin.x + cell_w * cx_col as f32;
                let y = bounds.origin.y + cell_h * cy_row as f32;
                window.paint_quad(fill(
                    Bounds::new(point(x, y), size(cell_w, cell_h)),
                    frame.cursor_color.opacity(0.85),
                ));
            }
        });

        if hovered {
            window.set_cursor_style(CursorStyle::PointingHand, hitbox);
        }
    }
}

impl Focusable for TerminalState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TerminalState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Focus listeners need a `Window`, so they install here on the first
        // paint rather than in the constructor.
        if self._focus_listeners.is_none() {
            let focus = cx.on_focus(&self.focus_handle, window, Self::on_focus_gained);
            let blur = cx.on_blur(&self.focus_handle, window, Self::on_focus_lost);
            self._focus_listeners = Some((focus, blur));
        }
        div()
            .id(("terminal-view", cx.entity_id()))
            .size_full()
            .track_focus(&self.focus_handle)
            .font_family(cx.theme().mono_font_family.clone())
            .on_key_down(cx.listener(Self::on_key_down))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_scroll_wheel(cx.listener(Self::on_scroll_wheel))
            .child(TerminalElement { state: cx.entity() })
    }
}

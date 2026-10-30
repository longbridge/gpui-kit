use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, ParentElement as _, RenderOnce, SharedString, StyleRefinement,
    Styled, Window, div, prelude::FluentBuilder as _,
};
use rust_i18n::t;

use crate::{
    ActiveTheme as _, IconName, StyledExt as _,
    ai::trace::{ToggleHandler, Trace, TraceStatus, code_surface},
    h_flex,
};

/// One frame of a [`StackTrace`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StackFrame {
    function: SharedString,
    location: Option<SharedString>,
    external: bool,
}

impl StackFrame {
    /// Create a frame for `function`.
    pub fn new(function: impl Into<SharedString>) -> Self {
        Self {
            function: function.into(),
            location: None,
            external: false,
        }
    }

    /// Set the source location, such as `src/main.rs:12:5`.
    pub fn location(mut self, location: impl Into<SharedString>) -> Self {
        self.location = Some(location.into());
        self
    }

    /// Mark the frame as belonging to a dependency or the runtime rather
    /// than the user's code; external frames are de-emphasized.
    pub fn external(mut self, external: bool) -> Self {
        self.external = external;
        self
    }

    /// The function the frame is in.
    pub fn function(&self) -> &SharedString {
        &self.function
    }

    /// The source location, when known.
    pub fn source_location(&self) -> Option<&SharedString> {
        self.location.as_ref()
    }

    /// Whether the frame is outside the user's code.
    pub fn is_external(&self) -> bool {
        self.external
    }
}

/// A failed tool call and its stack frames.
#[derive(IntoElement)]
pub struct StackTrace {
    id: ElementId,
    style: StyleRefinement,
    message: SharedString,
    frames: Vec<StackFrame>,
    open: bool,
    on_toggle: Option<ToggleHandler>,
}

impl StackTrace {
    /// Create a trace for an error message.
    pub fn new(id: impl Into<ElementId>, message: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            message: message.into(),
            frames: Vec::new(),
            open: false,
            on_toggle: None,
        }
    }

    /// Parse a JavaScript, Python or Rust backtrace.
    ///
    /// The first line that is not a frame becomes the message. Lines that are
    /// not recognized are ignored, so arbitrary tool output is safe to pass.
    pub fn parse(id: impl Into<ElementId>, text: &str) -> Self {
        let mut message = None;
        let mut frames = Vec::new();
        let mut lines = text.lines().map(str::trim).peekable();

        while let Some(line) = lines.next() {
            if line.is_empty() {
                continue;
            }
            // JavaScript: `at fn (file:1:2)` or `at file:1:2`.
            if let Some(rest) = line.strip_prefix("at ") {
                let frame = match rest.rsplit_once(" (") {
                    Some((function, location)) => StackFrame::new(function.to_string())
                        .location(location.trim_end_matches(')').to_string()),
                    None => StackFrame::new("<anonymous>").location(rest.to_string()),
                };
                frames.push(Self::classify(frame));
                continue;
            }
            // Python: `File "x.py", line 3, in fn`.
            if let Some(rest) = line.strip_prefix("File \"") {
                if let Some((file, rest)) = rest.split_once('"') {
                    let line_number = rest
                        .split(", ")
                        .find_map(|part| part.strip_prefix("line "))
                        .unwrap_or_default();
                    let function = rest.rsplit_once(", in ").map_or("<module>", |(_, f)| f);
                    let frame = StackFrame::new(function.to_string())
                        .location(format!("{file}:{line_number}"));
                    frames.push(Self::classify(frame));
                    // Python prints the source line under each frame.
                    if lines.peek().is_some_and(|next| !Self::is_frame_start(next)) {
                        lines.next();
                    }
                    continue;
                }
            }
            // Rust: `3: crate::fn` followed by `at src/lib.rs:1:2`.
            if let Some((number, function)) = line.split_once(": ")
                && !number.is_empty()
                && number.chars().all(|c| c.is_ascii_digit())
            {
                let mut frame = StackFrame::new(function.to_string());
                if let Some(location) = lines.peek().and_then(|next| next.strip_prefix("at ")) {
                    frame = frame.location(location.to_string());
                    lines.next();
                }
                frames.push(Self::classify(frame));
                continue;
            }
            if message.is_none()
                && !line.starts_with("Traceback")
                && !line.starts_with("stack backtrace")
            {
                message = Some(line.to_string());
            }
        }

        Self::new(id, message.unwrap_or_default()).frames(frames)
    }

    fn is_frame_start(line: &str) -> bool {
        line.starts_with("File \"") || line.starts_with("at ")
    }

    fn classify(frame: StackFrame) -> StackFrame {
        const EXTERNAL: [&str; 6] = [
            "node_modules",
            "node:",
            "site-packages",
            "/rustc/",
            ".cargo/registry",
            "<frozen ",
        ];
        let external = frame
            .location
            .as_ref()
            .is_some_and(|location| EXTERNAL.iter().any(|marker| location.contains(marker)))
            || frame.function.starts_with("std::")
            || frame.function.starts_with("core::");
        frame.external(external)
    }

    /// Add a frame, innermost first.
    pub fn frame(mut self, frame: StackFrame) -> Self {
        self.frames.push(frame);
        self
    }

    /// Add several frames, innermost first.
    pub fn frames(mut self, frames: impl IntoIterator<Item = StackFrame>) -> Self {
        self.frames.extend(frames);
        self
    }

    /// Set whether the frames are revealed.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Called with the requested open state when the header is activated.
    pub fn on_toggle(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

impl Styled for StackTrace {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for StackTrace {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let muted_foreground = theme.muted_foreground;
        let foreground = theme.foreground;
        let frame_count = self.frames.len();

        Trace::new(self.id)
            .icon(IconName::Bug)
            .title(div().text_color(theme.danger).child(self.message))
            .description(SharedString::from(t!(
                "StackTrace.frames",
                count = frame_count
            )))
            .status(TraceStatus::Failed)
            .open(self.open)
            .with_toggle_handler(self.on_toggle)
            .when(frame_count > 0, |this| {
                this.content(
                    code_surface(cx).children(self.frames.into_iter().map(|frame| {
                        h_flex()
                            .flex_wrap()
                            .gap_x_2()
                            .text_color(if frame.external {
                                muted_foreground
                            } else {
                                foreground
                            })
                            .child(frame.function)
                            .when_some(frame.location, |this, location| {
                                this.child(div().text_color(muted_foreground).child(location))
                            })
                    })),
                )
            })
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_trace_builder() {
        let trace = StackTrace::new("trace", "TypeError: x is undefined")
            .frame(StackFrame::new("render").location("src/app.ts:10:3"))
            .frames([StackFrame::new("run").external(true)])
            .open(true)
            .on_toggle(|_, _, _| {});

        assert_eq!(trace.frames.len(), 2);
        assert!(trace.frames[1].is_external());
        assert_eq!(
            trace.frames[0].source_location().map(|l| l.as_ref()),
            Some("src/app.ts:10:3")
        );
        assert!(trace.open);
    }

    #[test]
    fn test_parse_javascript() {
        let trace = StackTrace::parse(
            "js",
            "TypeError: Cannot read properties of undefined\n    at render (/app/src/view.ts:12:5)\n    at /app/node_modules/react/index.js:3:1",
        );
        assert_eq!(
            trace.message,
            "TypeError: Cannot read properties of undefined"
        );
        assert_eq!(trace.frames.len(), 2);
        assert_eq!(trace.frames[0].function(), "render");
        assert!(!trace.frames[0].is_external());
        assert!(trace.frames[1].is_external());
    }

    #[test]
    fn test_parse_python() {
        let trace = StackTrace::parse(
            "py",
            "Traceback (most recent call last):\n  File \"main.py\", line 4, in <module>\n    run()\n  File \"main.py\", line 2, in run\n    1 / 0\nZeroDivisionError: division by zero",
        );
        assert_eq!(trace.message, "ZeroDivisionError: division by zero");
        assert_eq!(trace.frames.len(), 2);
        assert_eq!(trace.frames[1].function(), "run");
        assert_eq!(
            trace.frames[1].source_location().map(|l| l.as_ref()),
            Some("main.py:2")
        );
    }

    #[test]
    fn test_parse_rust() {
        let trace = StackTrace::parse(
            "rs",
            "thread 'main' panicked at src/main.rs:2:5:\nboom\nstack backtrace:\n   0: std::panicking::begin_panic\n             at /rustc/abc/library/std/src/panicking.rs:1:1\n   1: app::main\n             at ./src/main.rs:2:5",
        );
        assert_eq!(trace.message, "thread 'main' panicked at src/main.rs:2:5:");
        assert_eq!(trace.frames.len(), 2);
        assert!(trace.frames[0].is_external());
        assert_eq!(trace.frames[1].function(), "app::main");
        assert!(!trace.frames[1].is_external());
    }
}

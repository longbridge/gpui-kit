//! A native web page embedded in a script view.
//!
//! `WebViewState` owns the native webview, so the loaded page, its history and
//! its scroll position survive renders; `WebView(state)` places it in the
//! layout. A page talks to the script through `window.ipc.postMessage(text)`,
//! which arrives at `WebView.on_message`, and the script talks back through
//! `WebViewState.evaluate_script`. `WebView.on_load` reports each document
//! that finishes loading, which is when a page can first be spoken to.
//! The page sees the application theme's appearance through
//! `prefers-color-scheme`; `WebViewState.set_theme` overrides it.
//!
//! Only `http` and `https` documents load, by script or by the page's own
//! navigation: any other scheme — `file:` above all — would reach the local
//! disk or a desktop URL handler past every capability the application holds.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentCallback,
    ComponentCallbackArgument, ComponentDataValue, ComponentDescriptor, ComponentMaterializer,
    ComponentPayload, ComponentRegistry, ConstructorDescriptor, MaterializeRequest,
    MethodDescriptor, RegistryError, StateDescriptor, StateMethodDescriptor, anyhow,
    gpui::{
        self, AppContext as _, Entity, IntoElement as _, ParentElement as _, Refineable as _,
        Styled as _, Task,
    },
};
use gpui_webview::WebView;

/// Events a page may queue before the script has drained them; later ones are
/// dropped with a warning rather than growing without bound.
const EVENT_QUEUE: usize = 1024;
/// The largest page message delivered to the script, in bytes.
const MESSAGE_LIMIT: usize = 1024 * 1024;

/// What the native webview reports, carried from its handlers to the script.
enum PageEvent {
    /// Text the page sent with `window.ipc.postMessage`.
    Message(String),
    /// The URL of a document that finished loading.
    Load(String),
}

/// The script listeners of the most recent render.
#[derive(Default)]
struct Listeners {
    message: Option<ComponentCallback>,
    load: Option<ComponentCallback>,
}

impl Listeners {
    fn for_event(&self, event: &PageEvent) -> Option<(ComponentCallback, &'static str)> {
        match event {
            PageEvent::Message(_) => self
                .message
                .clone()
                .map(|listener| (listener, "WebView message callback failed")),
            PageEvent::Load(_) => self
                .load
                .clone()
                .map(|listener| (listener, "WebView load callback failed")),
        }
    }
}

/// The colour scheme a page is shown in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PageTheme {
    Light,
    Dark,
    System,
}

impl PageTheme {
    fn of_app(cx: &gpui::App) -> Self {
        match gpui_base::Theme::global(cx).appearance {
            gpui_base::ThemeAppearance::Light => Self::Light,
            gpui_base::ThemeAppearance::Dark => Self::Dark,
        }
    }

    fn wry(self) -> wry::Theme {
        match self {
            Self::Light => wry::Theme::Light,
            Self::Dark => wry::Theme::Dark,
            Self::System => wry::Theme::Auto,
        }
    }
}

/// The retained value behind a script `WebViewState`.
#[derive(Clone)]
struct WebViewHandle {
    view: Entity<WebView>,
    listeners: Rc<RefCell<Listeners>>,
    /// The script's `set_theme`; `None` follows the application theme.
    theme: Rc<Cell<Option<PageTheme>>>,
    /// What the native page was last told, so a render only reaches it on a change.
    applied: Rc<Cell<Option<PageTheme>>>,
    _events: Rc<Task<()>>,
}

impl WebViewHandle {
    fn new(url: &str, window: &mut gpui::Window, cx: &mut gpui::App) -> anyhow::Result<Self> {
        let (sender, receiver) = async_channel::bounded::<PageEvent>(EVENT_QUEUE);
        let send = move |event| {
            if sender.try_send(event).is_err() {
                tracing::warn!("dropped a WebView event; the script is not keeping up");
            }
        };
        let builder = wry::WebViewBuilder::new()
            .with_url(url)
            .with_navigation_handler(|url| is_navigable(&url))
            .with_on_page_load_handler({
                let send = send.clone();
                move |event, url| {
                    if matches!(event, wry::PageLoadEvent::Finished) {
                        send(PageEvent::Load(url));
                    }
                }
            })
            .with_ipc_handler(move |request| {
                let message = request.into_body();
                if message.len() > MESSAGE_LIMIT {
                    tracing::warn!(
                        "dropped a {} byte WebView message; the limit is {MESSAGE_LIMIT}",
                        message.len()
                    );
                    return;
                }
                send(PageEvent::Message(message));
            });
        #[cfg(debug_assertions)]
        let builder = builder.with_devtools(true);
        let view = WebView::build(builder, window, cx)?;
        let view = cx.new(|_| view);

        let listeners = Rc::new(RefCell::new(Listeners::default()));
        let events = window.spawn(cx, {
            let listeners = listeners.clone();
            async move |cx| {
                // The senders live in the native webview's handlers, so the
                // loop ends when the state is released and the webview with it.
                while let Ok(event) = receiver.recv().await {
                    let Some((listener, context)) = listeners.borrow().for_event(&event) else {
                        continue;
                    };
                    let (PageEvent::Message(text) | PageEvent::Load(text)) = event;
                    let delivered = cx.update(|window, cx| {
                        listener.invoke_and_report_with(
                            context,
                            &[ComponentCallbackArgument::String(text)],
                            window,
                            cx,
                        );
                    });
                    if delivered.is_err() {
                        break;
                    }
                }
            }
        });

        let this = Self {
            view,
            listeners,
            theme: Rc::default(),
            applied: Rc::default(),
            _events: Rc::new(events),
        };
        this.apply_theme(cx)?;
        Ok(this)
    }

    /// Tell the page its colour scheme if it changed: the script's choice, or
    /// the application theme's appearance.
    fn apply_theme(&self, cx: &gpui::App) -> anyhow::Result<()> {
        let theme = self.theme.get().unwrap_or_else(|| PageTheme::of_app(cx));
        if self.applied.get() != Some(theme) {
            self.view.read(cx).set_theme(theme.wry())?;
            self.applied.set(Some(theme));
        }
        Ok(())
    }
}

/// Whether a document may load, from a script call or the page's own navigation.
///
/// `about:` is allowed because the platform engines navigate to `about:blank`
/// while they set up, and it carries nothing from outside the page.
fn is_navigable(url: &str) -> bool {
    if url.starts_with("about:") {
        return true;
    }
    wry::http::Uri::try_from(url)
        .is_ok_and(|uri| matches!(uri.scheme_str(), Some("http" | "https")) && uri.host().is_some())
}

/// Why a page could not be created, in terms a script author can act on.
///
/// `gpui-webview`'s Linux error names the Rust call that selects X11, which a
/// script cannot make; the runtime's own window is X11 under XWayland.
fn creation_error(error: &anyhow::Error) -> String {
    let reason = format!("WebViewState could not create a webview: {error}");
    if cfg!(target_os = "linux") && error.to_string().contains("X11") {
        return format!(
            "{reason}. In a Wayland session, start gpui-shell with `WAYLAND_DISPLAY=` to run under XWayland"
        );
    }
    reason
}

fn require_url(url: &str, context: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        is_navigable(url),
        "{context} expects an http or https URL, got `{url}`"
    );
    Ok(())
}

fn string_argument<'a>(
    arguments: &'a [ComponentDataValue],
    context: &str,
) -> anyhow::Result<&'a str> {
    match arguments {
        [ComponentDataValue::String(value)] => Ok(value),
        _ => anyhow::bail!("{context} expects one string"),
    }
}

fn state_methods() -> Vec<StateMethodDescriptor> {
    vec![
        StateMethodDescriptor::new::<WebViewHandle>(
            "load_url",
            "(url: string): void",
            |state, arguments, _, cx| {
                let url = string_argument(arguments, "WebViewState.load_url")?;
                require_url(url, "WebViewState.load_url")?;
                state.view.read(cx).raw().load_url(url)?;
                Ok(ComponentDataValue::Null)
            },
        ),
        StateMethodDescriptor::new::<WebViewHandle>("url", "(): string", |state, _, _, cx| {
            Ok(ComponentDataValue::String(state.view.read(cx).raw().url()?))
        })
        .with_readonly(true),
        StateMethodDescriptor::new::<WebViewHandle>("back", "(): void", |state, _, _, cx| {
            state.view.update(cx, |view, _| view.back())?;
            Ok(ComponentDataValue::Null)
        }),
        StateMethodDescriptor::new::<WebViewHandle>("forward", "(): void", |state, _, _, cx| {
            state.view.update(cx, |view, _| view.forward())?;
            Ok(ComponentDataValue::Null)
        }),
        StateMethodDescriptor::new::<WebViewHandle>("reload", "(): void", |state, _, _, cx| {
            state.view.read(cx).raw().reload()?;
            Ok(ComponentDataValue::Null)
        }),
        StateMethodDescriptor::new::<WebViewHandle>(
            "set_zoom",
            "(zoom: number): void",
            |state, arguments, _, cx| {
                let zoom = match arguments {
                    [ComponentDataValue::Number(zoom)] if zoom.is_finite() && *zoom > 0.0 => *zoom,
                    _ => anyhow::bail!(
                        "WebViewState.set_zoom expects a positive number, where 1 is the page's natural size"
                    ),
                };
                state.view.read(cx).set_zoom(zoom)?;
                Ok(ComponentDataValue::Null)
            },
        ),
        StateMethodDescriptor::new::<WebViewHandle>("zoom", "(): number", |state, _, _, cx| {
            Ok(ComponentDataValue::Number(state.view.read(cx).zoom()))
        })
        .with_readonly(true),
        StateMethodDescriptor::new::<WebViewHandle>(
            "set_theme",
            "(theme: \"auto\" | \"light\" | \"dark\" | \"system\"): void",
            |state, arguments, _, cx| {
                let theme = match string_argument(arguments, "WebViewState.set_theme")? {
                    "auto" => None,
                    "light" => Some(PageTheme::Light),
                    "dark" => Some(PageTheme::Dark),
                    "system" => Some(PageTheme::System),
                    other => anyhow::bail!(
                        "WebViewState.set_theme expects \"auto\", \"light\", \"dark\" or \"system\", got `{other}`"
                    ),
                };
                state.theme.set(theme);
                state.apply_theme(cx)?;
                Ok(ComponentDataValue::Null)
            },
        ),
        StateMethodDescriptor::new::<WebViewHandle>(
            "evaluate_script",
            "(script: string): void",
            |state, arguments, _, cx| {
                let script = string_argument(arguments, "WebViewState.evaluate_script")?;
                state.view.read(cx).raw().evaluate_script(script)?;
                Ok(ComponentDataValue::Null)
            },
        ),
    ]
}

#[derive(Clone)]
enum Op {
    Message(ComponentArgument),
    Load(ComponentArgument),
}

struct Materializer;
impl ComponentMaterializer for Materializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let argument = request
            .payload()
            .downcast_ref::<ComponentArgument>()
            .ok_or_else(|| anyhow::anyhow!("WebView received an incompatible payload"))?
            .clone();
        anyhow::ensure!(
            request.children_len() == 0,
            "WebView does not accept children"
        );
        let mut listeners = Listeners::default();
        for op in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<Op>())
        {
            match op {
                Op::Message(callback) => {
                    listeners.message = Some(request.resolve_callback(callback)?)
                }
                Op::Load(callback) => listeners.load = Some(request.resolve_callback(callback)?),
            }
        }
        let handle = request.with_state::<WebViewHandle, _>(&argument, |state| {
            // A render that drops a listener stops its delivery rather than
            // keeping one from a snapshot that no longer exists.
            *state.listeners.borrow_mut() = listeners;
            state.clone()
        })?;
        // A theme switch re-renders the window, so following the application
        // theme needs no subscription of its own.
        request.with_window_app(|_, cx| handle.apply_theme(cx))?;
        let state = handle.view;
        let mut container = gpui::div().child(state);
        container.style().refine(&request.take_style());
        Ok(container.into_any_element())
    }
}

pub(super) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register_state(
        StateDescriptor::new(
            "WebViewState",
            "WebViewState",
            vec![ArgumentDescriptor::new("url", ArgumentSchema::String)],
            |args, window, cx| match args {
                [ComponentArgument::String(url)] => {
                    require_url(url, "WebViewState").map_err(|error| error.to_string())?;
                    WebViewHandle::new(url, window, cx)
                        .map(|state| Box::new(state) as _)
                        .map_err(|error| creation_error(&error))
                }
                _ => Err("WebViewState expects one http or https URL".into()),
            },
        )
        .with_methods(state_methods())
        .with_documentation(
            "A retained native web page loaded from an http or https URL. The page posts text to the script with `window.ipc.postMessage(text)`.",
        ),
    )?;
    registry.register(
        ComponentDescriptor::new("WebView", Arc::new(Materializer))
            .with_constructors(vec![ConstructorDescriptor::new(
                "WebView",
                vec![ArgumentDescriptor::new("state", ArgumentSchema::Entity("WebViewState"))],
                |args| match args {
                    [argument @ ComponentArgument::Entity { .. }] => {
                        Ok(ComponentPayload::new(argument.clone()))
                    }
                    _ => Err("WebView expects one WebViewState".into()),
                },
            )])
            .with_methods(vec![
                MethodDescriptor::new(
                    "on_message",
                    vec![ArgumentDescriptor::new(
                        "listener",
                        ArgumentSchema::Callback("(message: string, cx: Context) => void"),
                    )],
                    |args| match args {
                        [argument @ ComponentArgument::Callback(_)] => {
                            Ok(ComponentPayload::new(Op::Message(argument.clone())))
                        }
                        _ => Err("WebView.on_message expects a listener".into()),
                    },
                )
                .with_documentation(
                    "Receives the text a page sends with `window.ipc.postMessage(text)`.",
                ),
                MethodDescriptor::new(
                    "on_load",
                    vec![ArgumentDescriptor::new(
                        "listener",
                        ArgumentSchema::Callback("(url: string, cx: Context) => void"),
                    )],
                    |args| match args {
                        [argument @ ComponentArgument::Callback(_)] => {
                            Ok(ComponentPayload::new(Op::Load(argument.clone())))
                        }
                        _ => Err("WebView.on_load expects a listener".into()),
                    },
                )
                .with_documentation(
                    "Receives the URL of each document that finishes loading, whether the script or the page navigated.",
                ),
            ])
            .with_documentation(
                "Places a WebViewState's page in the layout. Shell style sizes it; children are rejected. The native page draws above GPUI content it overlaps on Windows, and on Linux needs an X11 or XWayland window.",
            ),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_web_documents_are_navigable() {
        for allowed in [
            "https://gpui-kit.com",
            "http://localhost:3000/index.html",
            "about:blank",
        ] {
            assert!(is_navigable(allowed), "refused {allowed}");
        }
        for denied in [
            "",
            "gpui-kit.com",
            "file:///etc/passwd",
            "javascript:alert(1)",
            "data:text/html,<p>x</p>",
            "mailto:me@example.com",
            "https://",
        ] {
            assert!(!is_navigable(denied), "accepted {denied}");
        }
    }
}

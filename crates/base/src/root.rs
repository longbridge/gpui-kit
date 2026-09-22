//! Window roots and presentation-layer plugins.
use crate::input::Copy;
use crate::{StyledExt, TextSelectionLayer};
use gpui::{
    AnyElement, AnyView, App, AppContext, ClipboardItem, Context, Div, Entity, Global,
    InteractiveElement, IntoElement, KeyBinding, ParentElement, Render, Stateful, StyleRefinement,
    Styled, Window, actions, div,
};
use std::{any::TypeId, rc::Rc};

actions!(root, [Tab, TabPrev]);
const CONTEXT: &str = "Root";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", Tab, Some(CONTEXT)),
        KeyBinding::new("shift-tab", TabPrev, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-c", Copy, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-c", Copy, Some(CONTEXT)),
    ]);
}

/// A presentation layer's retained, per-window facilities.
///
/// Register during explicit application initialization, before creating windows.
/// The view renders above application content. Base owns the root regardless of
/// which plugins are registered; Cargo features never select its type.
///
/// On every root render, each plugin participates in three stages:
///
/// 1. [`RootPlugin::prepare`] synchronizes window state before elements are built.
/// 2. [`RootPlugin::style`] supplies defaults for the root surface.
/// 3. [`RootPlugin::decorate`] wraps the completed surface in presentation owned
///    by the plugin.
///
/// The plugin's [`Render`] output is mounted as an overlay above application
/// content. Plugins and their overlays are processed in registration order, so
/// later plugins appear above earlier ones. Notify the plugin entity after its
/// state changes to render the root again; `prepare` and `style` must not notify,
/// because they run during that render.
///
/// Factories are captured when a root is created, so registration affects only
/// future windows.
pub trait RootPlugin: Render + Sized {
    /// Synchronize settings derived from this plugin with the window.
    ///
    /// This runs before the root surface and plugin overlays are built. It is
    /// intended for window-scoped state such as rem size or the active text
    /// selection scope, not for producing elements.
    fn prepare(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}

    /// Apply this plugin's default styles directly to the root surface.
    ///
    /// Styles set on the [`Root`] instance are refined onto the surface after
    /// this hook and therefore take precedence over plugin defaults.
    fn style(&self, _surface: &mut Stateful<Div>, _window: &mut Window, _cx: &mut App) {}

    /// Add presentation around the completed root surface.
    ///
    /// This runs after plugin defaults and instance styles have been applied.
    /// Return `surface` unchanged when no outer presentation is needed. Typical
    /// uses include client-side window borders or another structural wrapper.
    /// `root` provides read-only access to the root and its application view.
    fn decorate(
        &self,
        surface: AnyElement,
        _root: &Root,
        _window: &mut Window,
        _cx: &mut App,
    ) -> impl IntoElement {
        surface
    }
}

type PluginFactory = Rc<dyn Fn(&mut Window, &mut Context<Root>) -> Plugin>;
type Prepare = Rc<dyn Fn(&mut Window, &mut App)>;
type SurfaceStyle = Rc<dyn Fn(&mut Stateful<Div>, &mut Window, &mut App)>;
type Decorate = Rc<dyn Fn(AnyElement, &Root, &mut Window, &mut App) -> AnyElement>;
#[derive(Default)]
struct PluginRegistry(Vec<(TypeId, PluginFactory)>);
impl Global for PluginRegistry {}
struct Plugin {
    view: AnyView,
    prepare: Prepare,
    style: SurfaceStyle,
    decorate: Decorate,
}

/// The window's content and overlay host, independent of any styled component library.
pub struct Root {
    view: AnyView,
    style: StyleRefinement,
    plugins: Vec<Plugin>,
}

impl Root {
    /// Register a presentation plugin once per application. Re-registering its
    /// type replaces the factory for future windows rather than mounting it twice.
    pub fn register_plugin<V: RootPlugin>(
        cx: &mut App,
        build: fn(&mut Window, &mut Context<V>) -> V,
    ) {
        if !cx.has_global::<PluginRegistry>() {
            cx.set_global(PluginRegistry::default());
        }
        let factory: PluginFactory = Rc::new(move |window, cx| {
            let entity = cx.new(|cx| build(window, cx));
            let root = cx.weak_entity();
            let observed = entity.clone();
            // Plugin construction can enqueue notifications. Observe only after
            // that effect cycle so mounting a Root does not immediately render
            // application content a second time.
            cx.defer(move |cx| {
                cx.observe(&observed, move |_, cx| {
                    let _ = root.update(cx, |_, cx| cx.notify());
                })
                .detach();
            });
            let prepare = entity.clone();
            let style = entity.clone();
            let decorate = entity.clone();
            Plugin {
                view: entity.into(),
                prepare: Rc::new(move |window, cx| {
                    prepare.update(cx, |state, cx| state.prepare(window, cx))
                }),
                style: Rc::new(move |surface, window, cx| {
                    style.update(cx, |state, cx| state.style(surface, window, cx))
                }),
                decorate: Rc::new(move |surface, root, window, cx| {
                    decorate.update(cx, |state, cx| {
                        state.decorate(surface, root, window, cx).into_any_element()
                    })
                }),
            }
        });
        let plugins = &mut cx.global_mut::<PluginRegistry>().0;
        if let Some(entry) = plugins.iter_mut().find(|(id, _)| *id == TypeId::of::<V>()) {
            entry.1 = factory;
        } else {
            plugins.push((TypeId::of::<V>(), factory));
        }
    }

    pub fn new(view: impl Into<AnyView>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        #[cfg(all(target_os = "macos", not(test)))]
        crate::install_window_hit_test_forwarder(window);
        let factories = cx
            .try_global::<PluginRegistry>()
            .map(|e| e.0.clone())
            .unwrap_or_default();
        Self {
            view: view.into(),
            style: StyleRefinement::default(),
            plugins: factories
                .into_iter()
                .map(|(_, build)| build(window, cx))
                .collect(),
        }
    }

    /// The original application content entity.
    pub fn view(&self) -> &AnyView {
        &self.view
    }

    /// Find a presentation plugin owned by this window.
    pub fn plugin<V: RootPlugin>(&self) -> Option<Entity<V>> {
        self.plugins
            .iter()
            .find_map(|entry| entry.view.clone().downcast::<V>().ok())
    }

    pub fn read<'a>(window: &'a Window, cx: &'a App) -> &'a Self {
        window
            .root::<Self>()
            .flatten()
            .expect("window must have a Base Root")
            .read(cx)
    }
    pub fn update<R>(
        window: &mut Window,
        cx: &mut App,
        f: impl FnOnce(&mut Self, &mut Window, &mut Context<Self>) -> R,
    ) -> R {
        let root = window
            .root::<Self>()
            .flatten()
            .expect("window must have a Base Root");
        root.update(cx, |root, cx| f(root, window, cx))
    }
    fn on_action_tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        // Check if we're inside a focus trap
        if let Some(container_focus_handle) = crate::active_focus_trap(window, cx) {
            // We're in a focus trap - try to focus next, then check if we're still inside
            let before_focus = window.focused(cx);

            // Try normal focus navigation
            window.focus_next(cx);

            // Check if we're still in the trap
            if !container_focus_handle.contains_focused(window, cx) {
                // We jumped out of the trap - need to cycle back to the beginning
                // Find the first focusable element in the trap by continuing to focus_next
                let mut attempts = 0;
                const MAX_ATTEMPTS: usize = 100; // Prevent infinite loop

                while !container_focus_handle.contains_focused(window, cx)
                    && attempts < MAX_ATTEMPTS
                {
                    window.focus_next(cx);
                    attempts += 1;

                    // If we cycled back to where we started, restore original focus
                    if window.focused(cx) == before_focus {
                        break;
                    }
                }
            }
            return;
        }

        // Normal tab navigation
        window.focus_next(cx);
    }

    fn on_action_tab_prev(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        // Check if we're inside a focus trap
        if let Some(container_focus_handle) = crate::active_focus_trap(window, cx) {
            // We're in a focus trap - try to focus previous, then check if we're still inside
            let before_focus = window.focused(cx);

            // Try normal focus navigation
            window.focus_prev(cx);

            // Check if we're still in the trap
            if !container_focus_handle.contains_focused(window, cx) {
                // We jumped out of the trap - need to cycle back to the end
                // Find the last focusable element in the trap by continuing to focus_prev
                let mut attempts = 0;
                const MAX_ATTEMPTS: usize = 100; // Prevent infinite loop

                while !container_focus_handle.contains_focused(window, cx)
                    && attempts < MAX_ATTEMPTS
                {
                    window.focus_prev(cx);
                    attempts += 1;

                    // If we cycled back to where we started, restore original focus
                    if window.focused(cx) == before_focus {
                        break;
                    }
                }
            }
            return;
        }

        // Normal tab navigation
        window.focus_prev(cx);
    }

    fn on_action_copy(&mut self, _: &Copy, window: &mut Window, cx: &mut Context<Self>) {
        let text = crate::TextSelection::selected_text(window, cx)
            .trim()
            .to_string();
        if text.is_empty() {
            cx.propagate();
            return;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }
}
impl Styled for Root {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl Render for Root {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        for plugin in &self.plugins {
            (plugin.prepare)(window, cx);
        }
        let mut content = div()
            .id("root")
            .key_context(CONTEXT)
            .on_action(cx.listener(Self::on_action_tab))
            .on_action(cx.listener(Self::on_action_tab_prev))
            .on_action(cx.listener(Self::on_action_copy))
            .relative()
            .size_full()
            .child(TextSelectionLayer)
            .child(self.view.clone())
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .children(self.plugins.iter().map(|plugin| plugin.view.clone())),
            );
        for plugin in &self.plugins {
            (plugin.style)(&mut content, window, cx);
        }
        let mut content = content.refine_style(&self.style).into_any_element();
        for plugin in &self.plugins {
            content = (plugin.decorate)(content, self, window, cx);
        }
        content
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    struct Content;
    impl Render for Content {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }
    struct Layer;
    impl Render for Layer {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }
    impl RootPlugin for Layer {}
    fn layer(_: &mut Window, _: &mut Context<Layer>) -> Layer {
        Layer
    }

    #[gpui::test]
    fn plugin_registration_is_idempotent_and_state_is_per_window(cx: &mut TestAppContext) {
        cx.update(|cx| {
            crate::init(cx);
            Root::register_plugin(cx, layer);
            Root::register_plugin(cx, layer);
        });
        let mut ids = Vec::new();
        for _ in 0..2 {
            let (root, _) = cx.add_window_view(|window, cx| {
                let content = cx.new(|_| Content);
                Root::new(content, window, cx)
            });
            let id = root.read_with(cx, |root, _| {
                assert_eq!(root.plugins.len(), 1);
                root.plugin::<Layer>().unwrap().entity_id()
            });
            ids.push(id);
        }
        assert_ne!(ids[0], ids[1]);
    }
}

//! Opt-in headless observation built exclusively on public GPUI APIs.
//!
//! Kit controls supply their own metadata. Applications can observe an identified
//! GPUI div with `.observe()`. No accessibility tree or separate test ID is used.
use gpui::{
    App, Bounds, Element, ElementId, FocusHandle, GlobalElementId, Hitbox, InspectorElementId,
    InteractiveElement, IntoElement, LayoutId, Pixels, SharedString, Visibility, Window, px,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

/// Optional facts known by the control, rather than inferred from its element tree.
#[derive(Clone, Default)]
pub struct Metadata {
    pub focus: Option<FocusHandle>,
    pub disabled: bool,
    pub text: Option<SharedString>,
}

/// Owned facts from the last paint of an observed element.
#[derive(Clone, Debug)]
pub struct TestElement {
    bounds: Bounds<Pixels>,
    visible: bool,
    focused: bool,
    disabled: bool,
    text: Option<SharedString>,
}
impl TestElement {
    pub fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }
    pub fn visible(&self) -> bool {
        self.visible
    }
    pub fn focused(&self) -> bool {
        self.focused
    }
    pub fn disabled(&self) -> bool {
        self.disabled
    }
    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }
}

// Each Window owns a distinct Arc<WindowTextSystem>, stable even during initial
// drawing before the Window itself is boxed. Its identity distinguishes equal
// WindowIds in different Apps. Entries never own a Window or a view.
type WindowKey = usize;
type Registry = HashMap<WindowKey, HashMap<GlobalElementId, Weak<Registration>>>;
thread_local! { static REGISTRY: RefCell<Registry> = RefCell::new(HashMap::new()); }

#[doc(hidden)]
pub struct Registration {
    window: WindowKey,
    global_id: GlobalElementId,
    id: ElementId,
    facts: RefCell<TestElement>,
}
impl Drop for Registration {
    fn drop(&mut self) {
        let _ = REGISTRY.try_with(|registry| {
            if let Ok(mut registry) = registry.try_borrow_mut() {
                if let Some(entries) = registry.get_mut(&self.window) {
                    entries.remove(&self.global_id);
                    if entries.is_empty() {
                        registry.remove(&self.window);
                    }
                }
            }
        });
    }
}

/// Internal lookup used by the public Window extension in gpui-test.
#[doc(hidden)]
pub fn find(window: &Window, id: ElementId) -> Option<TestElement> {
    REGISTRY.with(|registry| {
        let registry = registry.borrow();
        let entries = registry.get(&(std::sync::Arc::as_ptr(window.text_system()) as usize))?;
        let mut matches = entries
            .values()
            .filter_map(Weak::upgrade)
            .filter(|entry| entry.id == id);
        let entry = matches.next()?;
        assert!(
            matches.next().is_none(),
            "ambiguous ElementId {id:?}; use a unique target ID"
        );
        Some(entry.facts.borrow().clone())
    })
}

/// Adds observation to a GPUI div without changing its identity or event handling.
/// Native builder methods continue to work. Unobserved elements are not indexed.
pub trait ObserveElement:
    Element<PrepaintState = Option<Hitbox>> + InteractiveElement + Sized
{
    fn observe(self) -> Observed<Self> {
        assert!(
            Element::id(&self).is_some(),
            "observe requires an existing ElementId"
        );
        Observed {
            inner: self,
            metadata: Metadata::default(),
        }
    }
}
impl<E: Element<PrepaintState = Option<Hitbox>> + InteractiveElement> ObserveElement for E {}

/// Transparent forwarding element. Its registration is owned by GPUI element
/// state, so normal state cleanup and cached-view replay determine its lifetime.
pub struct Observed<E> {
    inner: E,
    metadata: Metadata,
}
impl<E> Observed<E> {
    pub fn metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = metadata;
        self
    }
    pub fn text(mut self, text: impl Into<SharedString>) -> Self {
        self.metadata.text = Some(text.into());
        self
    }
    pub fn focus(mut self, focus: &FocusHandle) -> Self {
        self.metadata.focus = Some(focus.clone());
        self
    }
}
impl<E: Element<PrepaintState = Option<Hitbox>> + InteractiveElement> IntoElement for Observed<E> {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl<E: Element<PrepaintState = Option<Hitbox>> + InteractiveElement> Element for Observed<E> {
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = (Option<Hitbox>, Rc<Registration>);
    fn id(&self) -> Option<ElementId> {
        Element::id(&self.inner)
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.inner.source_location()
    }
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.inner.request_layout(id, inspector, window, cx)
    }
    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let global_id = id.expect("observed elements have an ID");
        let facts = TestElement {
            bounds,
            visible: false,
            focused: false,
            disabled: self.metadata.disabled,
            text: self.metadata.text.clone(),
        };
        let registration =
            window.with_element_state(global_id, |state: Option<Rc<Registration>>, window| {
                let registration = state.unwrap_or_else(|| {
                    Rc::new(Registration {
                        window: std::sync::Arc::as_ptr(window.text_system()) as usize,
                        global_id: global_id.clone(),
                        id: Element::id(&self.inner).unwrap(),
                        facts: RefCell::new(facts.clone()),
                    })
                });
                *registration.facts.borrow_mut() = facts;
                REGISTRY.with(|registry| {
                    registry
                        .borrow_mut()
                        .entry(registration.window)
                        .or_default()
                        .insert(global_id.clone(), Rc::downgrade(&registration));
                });
                (registration.clone(), registration)
            });
        (
            self.inner
                .prepaint(id, inspector, bounds, layout, window, cx),
            registration,
        )
    }
    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        paint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let style = self
            .inner
            .interactivity()
            .compute_style(id, paint.0.as_ref(), window, cx);
        let clipped = bounds
            .intersect(&window.content_mask().bounds)
            .intersect(&Bounds::new(Default::default(), window.viewport_size()));
        {
            let mut facts = paint.1.facts.borrow_mut();
            facts.visible = style.visibility != Visibility::Hidden
                && style.opacity.unwrap_or(1.) > 0.
                && clipped.size.width > px(0.)
                && clipped.size.height > px(0.);
            facts.focused = self
                .metadata
                .focus
                .as_ref()
                .is_some_and(|focus| focus.is_focused(window));
        }
        self.inner
            .paint(id, inspector, bounds, layout, &mut paint.0, window, cx);
    }
    fn a11y_role(&self) -> Option<gpui::Role> {
        self.inner.a11y_role()
    }
    fn write_a11y_info(&self, node: &mut gpui::accesskit::Node) {
        self.inner.write_a11y_info(node);
    }
    fn a11y_synthetic_children(
        &mut self,
        paint: &mut Self::PrepaintState,
        builder: &mut gpui::A11ySubtreeBuilder,
    ) {
        self.inner.a11y_synthetic_children(&mut paint.0, builder);
    }
}

impl<E: Element<PrepaintState = Option<Hitbox>> + InteractiveElement + gpui::Styled> gpui::Styled
    for Observed<E>
{
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.inner.style()
    }
}
impl<E: Element<PrepaintState = Option<Hitbox>> + InteractiveElement> InteractiveElement
    for Observed<E>
{
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.inner.interactivity()
    }
    fn track_focus(mut self, focus: &FocusHandle) -> Self {
        self.metadata.focus = Some(focus.clone());
        self.inner = self.inner.track_focus(focus);
        self
    }
}
impl<E: Element<PrepaintState = Option<Hitbox>> + InteractiveElement + gpui::ParentElement>
    gpui::ParentElement for Observed<E>
{
    fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) {
        self.inner.extend(elements);
    }
}

impl<E: Element<PrepaintState = Option<Hitbox>> + gpui::StatefulInteractiveElement>
    gpui::StatefulInteractiveElement for Observed<E>
{
}

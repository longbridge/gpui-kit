//! Opt-in headless observation built exclusively on public GPUI APIs.
//!
//! Native accessibility properties are read automatically. Applications register
//! identified GPUI elements with `.test_support()`, without supplying test-only state.
use gpui::{
    App, Bounds, Element, ElementId, FocusHandle, GlobalElementId, Hitbox, InspectorElementId,
    InteractiveElement, IntoElement, LayoutId, Pixels, SharedString, Visibility, Window, px,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    fmt,
    rc::{Rc, Weak},
};

/// Owned facts from the last paint of an observed element.
#[derive(Clone)]
pub struct ElementSnapshot {
    role: Option<gpui::Role>,
    path: Vec<ElementId>,
    checked: Option<bool>,
    indeterminate: Option<bool>,
    selected: Option<bool>,
    expanded: Option<bool>,
    value: Option<SharedString>,
    bounds: Bounds<Pixels>,
    visible: bool,
    focused: Option<bool>,
    focus_action: bool,
    disabled: Option<bool>,
    label: Option<SharedString>,
}
impl fmt::Debug for ElementSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct BindingMissed;
        impl fmt::Debug for BindingMissed {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("<binding missed>")
            }
        }
        let focused: &dyn fmt::Debug = if self.focused.is_none() && self.focus_action {
            &BindingMissed
        } else {
            &self.focused
        };
        f.debug_struct("ElementSnapshot")
            .field("role", &self.role)
            .field("path", &self.path)
            .field("checked", &self.checked)
            .field("indeterminate", &self.indeterminate)
            .field("selected", &self.selected)
            .field("expanded", &self.expanded)
            .field("value", &self.value)
            .field("bounds", &self.bounds)
            .field("visible", &self.visible)
            .field("focused", focused)
            .field("focus_action", &self.focus_action)
            .field("disabled", &self.disabled)
            .field("label", &self.label)
            .finish()
    }
}

impl ElementSnapshot {
    pub fn role(&self) -> Option<gpui::Role> {
        self.role
    }
    pub fn path(&self) -> &[ElementId] {
        &self.path
    }
    pub fn checked(&self) -> Option<bool> {
        self.checked
    }
    pub fn indeterminate(&self) -> Option<bool> {
        self.indeterminate
    }
    pub fn selected(&self) -> Option<bool> {
        self.selected
    }
    pub fn expanded(&self) -> Option<bool> {
        self.expanded
    }
    /// The native accessibility value, not rendered text or pixels.
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    pub fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }
    pub fn visible(&self) -> bool {
        self.visible
    }
    /// Whether the tracked focus scope contains the current keyboard focus.
    /// `None` means no focus binding or native focus capability was observed.
    /// Panics if the native element advertises `Action::Focus` but its binding was missed.
    /// Custom elements omitting that action cannot be diagnosed and may return `None`.
    pub fn focused(&self) -> Option<bool> {
        assert!(
            self.focused.is_some() || !self.focus_action,
            "focus binding was not observed for {:?}; use .test_support().track_focus(&handle). GPUI does not expose pre-existing or implicit focus handles",
            self.path
        );
        self.focused
    }
    /// `None` means the native element does not expose a disabled flag.
    /// Absence is not evidence that a control accepts input.
    pub fn disabled(&self) -> Option<bool> {
        self.disabled
    }
    /// The accessibility label; it can intentionally differ from visible text.
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
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
    facts: RefCell<ElementSnapshot>,
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

/// Internal lookup used by Kit's testing API. Scope follows GPUI's element path.
#[doc(hidden)]
pub fn find(window: &Window, scope: &[ElementId], id: &ElementId) -> Option<ElementSnapshot> {
    REGISTRY.with(|registry| {
        let registry = registry.borrow();
        let entries = registry.get(&(std::sync::Arc::as_ptr(window.text_system()) as usize))?;
        let matches: Vec<_> = entries
            .values()
            .filter_map(Weak::upgrade)
            .filter(|entry| {
                entry.global_id.len() > scope.len()
                    && entry.global_id.starts_with(scope)
                    && entry.global_id.last() == Some(id)
            })
            .collect();
        assert!(
            matches.len() <= 1,
            "ambiguous ElementId {id:?}; use within(...) to select a scope. Matches: {:?}",
            matches
                .iter()
                .map(|entry| &entry.global_id)
                .collect::<Vec<_>>()
        );
        matches.first().map(|entry| entry.facts.borrow().clone())
    })
}

#[doc(hidden)]
pub fn snapshots(window: &Window) -> Vec<ElementSnapshot> {
    REGISTRY.with(|registry| {
        registry
            .borrow()
            .get(&(std::sync::Arc::as_ptr(window.text_system()) as usize))
            .map(|entries| {
                entries
                    .values()
                    .filter_map(Weak::upgrade)
                    .map(|entry| entry.facts.borrow().clone())
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// Internal focus membership check; unsupported bindings are not evidence of focus.
#[doc(hidden)]
pub fn has_observed_focus(window: &Window, scope: &[ElementId]) -> bool {
    snapshots(window)
        .iter()
        .any(|element| element.path.starts_with(scope) && element.focused == Some(true))
}

/// Resolves a scope even when only its descendants are observed.
#[doc(hidden)]
pub fn scope(window: &Window, parent: &[ElementId], id: &ElementId) -> Vec<ElementId> {
    let mut paths = std::collections::HashSet::new();
    for snapshot in snapshots(window) {
        if !snapshot.path.starts_with(parent) {
            continue;
        }
        for index in parent.len()..snapshot.path.len() {
            if &snapshot.path[index] == id {
                paths.insert(snapshot.path[..=index].to_vec());
            }
        }
    }
    assert_eq!(
        paths.len(),
        1,
        "expected one scope {id:?} below {parent:?}, found {}. Registered paths: {}",
        paths.len(),
        registered_paths(window)
    );
    paths.into_iter().next().unwrap()
}

#[doc(hidden)]
pub fn registered_paths(window: &Window) -> String {
    let mut paths: Vec<_> = snapshots(window)
        .iter()
        .map(|entry| format!("{:?}", entry.path))
        .collect();
    paths.sort();
    if paths.is_empty() {
        "<none; draw a frame and observe elements>".into()
    } else {
        paths.join(", ")
    }
}

/// Transparent forwarding element. Its registration is owned by GPUI element
/// state, so normal state cleanup and cached-view replay determine its lifetime.
pub struct Observed<E> {
    inner: E,
    focus: Option<FocusHandle>,
}
impl<E: Element> Observed<E> {
    /// Repeated observation keeps one registration and the same native identity.
    pub fn test_support(self) -> Self {
        self
    }

    /// Changes the native identity while keeping observation outside GPUI's
    /// stateful wrapper, so later focus bindings still reach this element.
    pub fn id(self, id: impl Into<ElementId>) -> Observed<gpui::Stateful<E>>
    where
        E: InteractiveElement,
    {
        Observed {
            inner: self.inner.id(id),
            focus: self.focus,
        }
    }

    pub(crate) fn new(inner: E) -> Self {
        assert!(
            Element::id(&inner).is_some(),
            "test_support requires an existing ElementId"
        );
        Self { inner, focus: None }
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
        let role = self.inner.a11y_role();
        let mut node = gpui::accesskit::Node::new(role.unwrap_or(gpui::Role::Unknown));
        self.inner.write_a11y_info(&mut node);
        let toggled = node.toggled();
        let facts = ElementSnapshot {
            role,
            path: global_id.to_vec(),
            checked: toggled.map(|value| value == gpui::accesskit::Toggled::True),
            indeterminate: toggled.map(|value| value == gpui::accesskit::Toggled::Mixed),
            selected: node.is_selected(),
            expanded: node.is_expanded(),
            value: node.value().map(|value| value.to_owned().into()),
            bounds,
            visible: false,
            focused: None,
            focus_action: node.supports_action(gpui::accesskit::Action::Focus),
            disabled: node.is_disabled().then_some(true),
            label: node.label().map(|label| label.to_owned().into()),
        };
        let registration =
            window.with_element_state(global_id, |state: Option<Rc<Registration>>, window| {
                let registration = state.unwrap_or_else(|| {
                    Rc::new(Registration {
                        window: std::sync::Arc::as_ptr(window.text_system()) as usize,
                        global_id: global_id.clone(),
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
                .focus
                .as_ref()
                .map(|focus| focus.contains_focused(window, cx));
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
        self.focus = Some(focus.clone());
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

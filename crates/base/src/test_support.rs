//! Opt-in headless observation built exclusively on public GPUI APIs.
//!
//! Kit controls supply their own metadata. Applications can observe an identified
//! GPUI div with `.test_state(...)`. No accessibility tree or separate test ID is used.
use gpui::{
    App, Bounds, Element, ElementId, FocusHandle, GlobalElementId, Hitbox, InspectorElementId,
    InteractiveElement, IntoElement, LayoutId, Pixels, SharedString, Visibility, Window, px,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

/// Owned facts from the last paint of an observed element.
#[derive(Clone, Debug)]
pub struct ElementSnapshot {
    path: Vec<ElementId>,
    checked: Option<bool>,
    indeterminate: Option<bool>,
    selected: Option<bool>,
    expanded: Option<bool>,
    value: Option<SharedString>,
    bounds: Bounds<Pixels>,
    visible: bool,
    focused: bool,
    disabled: bool,
    text: Option<SharedString>,
}
impl ElementSnapshot {
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
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }

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
        "<none; draw a frame and enable observation>".into()
    } else {
        paths.join(", ")
    }
}

/// Transparent forwarding element. Its registration is owned by GPUI element
/// state, so normal state cleanup and cached-view replay determine its lifetime.
pub struct Observed<E> {
    inner: E,
    test: crate::TestState,
}
impl<E: Element> Observed<E> {
    pub(crate) fn new(inner: E, test: crate::TestState) -> Self {
        assert!(
            Element::id(&inner).is_some(),
            "test_state requires an existing ElementId"
        );
        Self { inner, test }
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
        let facts = ElementSnapshot {
            path: global_id.to_vec(),
            checked: self.test.checked,
            indeterminate: self.test.indeterminate,
            selected: self.test.selected,
            expanded: self.test.expanded,
            value: self.test.value.clone(),
            bounds,
            visible: false,
            focused: false,
            disabled: self.test.disabled,
            text: self.test.text.clone(),
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
                .test
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
        self.test.focus = Some(focus.clone());
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

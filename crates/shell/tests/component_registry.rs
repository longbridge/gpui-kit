#[cfg(feature = "gpui-fast")]
extern crate gpui_fast as gpui;

use std::sync::Arc;

use gpui::{AnyElement, IntoElement as _, div};
use gpui_shell::{
    COMPONENT_REGISTRY_API_VERSION, ComponentDescriptor, ComponentMaterializer, ComponentRegistry,
    ConstructorDescriptor, MaterializeRequest, RegistryError,
};

#[test]
fn evolving_registry_enums_are_non_exhaustive() {
    let source = include_str!("../src/component_registry.rs");
    for declaration in [
        "pub enum ComponentArgument",
        "pub enum ArgumentSchema",
        "pub enum RegistryError",
    ] {
        let offset = source.find(declaration).expect("public enum declaration");
        let prefix = &source[..offset];
        let attribute = prefix
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .expect("attribute before public enum");
        assert_eq!(
            attribute.trim(),
            "#[non_exhaustive]",
            "{declaration} is an evolving public seam"
        );
    }
}

struct EmptyMaterializer;

impl ComponentMaterializer for EmptyMaterializer {
    fn materialize(&self, _request: MaterializeRequest<'_>) -> anyhow::Result<AnyElement> {
        Ok(div().into_any_element())
    }
}

fn descriptor(name: &'static str, export: &'static str) -> ComponentDescriptor {
    ComponentDescriptor::new(name, Arc::new(EmptyMaterializer))
        .with_constructors(vec![ConstructorDescriptor::new(export, Vec::new(), |_| {
            Ok(gpui_shell::ComponentPayload::new(()))
        })])
        .with_methods(Vec::new())
}

#[test]
fn registry_assigns_stable_ids_and_preserves_registration_order() {
    let mut registry = ComponentRegistry::new(
        COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();

    let button = registry.register(descriptor("Button", "Button")).unwrap();
    let badge = registry.register(descriptor("Badge", "Badge")).unwrap();
    let frozen = registry.freeze().unwrap();

    assert_eq!(button.as_u32(), 0);
    assert_eq!(badge.as_u32(), 1);
    assert_eq!(
        frozen
            .descriptors()
            .map(|descriptor| descriptor.name())
            .collect::<Vec<_>>(),
        ["Button", "Badge"]
    );
    assert_eq!(frozen.descriptor(button).unwrap().name(), "Button");
}

#[test]
fn registry_rejects_duplicate_component_and_export_names() {
    let mut registry = ComponentRegistry::new(
        COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )
    .unwrap();
    registry.register(descriptor("Button", "Button")).unwrap();

    assert!(matches!(
        registry.register(descriptor("Button", "AnotherButton")),
        Err(RegistryError::DuplicateComponent(name)) if name == "Button"
    ));
    assert!(matches!(
        registry.register(descriptor("ButtonAlias", "Button")),
        Err(RegistryError::DuplicateExport(name)) if name == "Button"
    ));
}

/// Registering after a freeze is not a run-time error to test for: `freeze`
/// consumes the builder, so the compiler rejects the call.
///
/// ```compile_fail
/// # use gpui_shell::{COMPONENT_REGISTRY_API_VERSION, ComponentRegistry};
/// let mut registry = ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION, gpui_shell::DEFAULT_COMPONENT_MODULE).unwrap();
/// let _frozen = registry.freeze().unwrap();
/// registry.register(todo!());
/// ```
const _FREEZE_IS_FINAL: () = ();

#[test]
fn registry_rejects_an_incompatible_api_version() {
    assert!(matches!(
        ComponentRegistry::new(COMPONENT_REGISTRY_API_VERSION + 1, gpui_shell::DEFAULT_COMPONENT_MODULE),
        Err(RegistryError::IncompatibleApiVersion { expected, actual })
            if expected == COMPONENT_REGISTRY_API_VERSION
                && actual == COMPONENT_REGISTRY_API_VERSION + 1
    ));
}

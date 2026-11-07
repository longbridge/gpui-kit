//! The `gpui-component` catalog: JavaScript bindings for the styled component
//! library, imported by scripts as `"gpui-component"`.
//!
//! This module is the only place concrete `gpui-component` knowledge belongs,
//! and it exists only with the `components` feature. The rest of the runtime
//! never names it: the catalog reaches the runtime as a
//! [`FrozenComponentRegistry`](crate::FrozenComponentRegistry) through the
//! same registration API an embedding host uses for its own components, so a
//! build without the feature is the unstyled, `gpui-base`-only runtime.

mod shell;

/// Initializes the component catalog and the shell runtime it registers into.
///
/// Must be called once at application startup, before any script runs. This is
/// the entry point for a host that renders this catalog; [`gpui_shell::init`]
/// alone installs the base layer without any concrete components.
///
/// The catalog also carries this function, so a host holding only the frozen
/// registry — the shipped command, for one — gets the same startup through
/// [`gpui_shell::init_with_components`].
pub fn init(cx: &mut gpui_shell::gpui::App) {
    gpui_component::init(cx);
    gpui_shell::init(cx);
}

/// Builds and freezes the `gpui-component` catalog.
pub fn components() -> Result<gpui_shell::FrozenComponentRegistry, gpui_shell::RegistryError> {
    registry()?.freeze()
}

/// Builds the `gpui-component` catalog without freezing it, so an embedding
/// host can register its own components and states beside it.
///
/// A runtime takes one frozen catalog, so a host that wants its own native
/// components and this catalog in the same script registers both into this
/// registry and then freezes it. They share the `"gpui-component"` module, and
/// export names must not collide with the catalog's.
///
/// The registry carries [`gpui_component::init`] as its initializer. A host
/// whose components need their own startup replaces it with a function that
/// calls `gpui_component::init` first:
///
/// ```rust,ignore
/// fn init(cx: &mut App) {
///     gpui_component::init(cx);
///     my_components::init(cx);
/// }
///
/// let mut registry = gpui_shell::components::registry()?.with_initializer(init);
/// my_components::register(&mut registry)?;
/// gpui_shell::host::main_with_components(registry.freeze()?);
/// ```
pub fn registry() -> Result<gpui_shell::ComponentRegistry, gpui_shell::RegistryError> {
    let mut registry = gpui_shell::ComponentRegistry::new(
        gpui_shell::COMPONENT_REGISTRY_API_VERSION,
        gpui_shell::DEFAULT_COMPONENT_MODULE,
    )?
    .with_initializer(gpui_component::init)
    .with_window_opener(open_window_with_root);
    register(&mut registry)?;
    Ok(registry)
}

/// Creates an isolated shell runtime with this component catalog.
pub fn new_isolated_runtime() -> gpui_shell::anyhow::Result<std::rc::Rc<gpui_shell::ShellRuntime>> {
    gpui_shell::ShellRuntime::new_isolated_with_components(components()?)
}

/// Opens the window with `gpui_component::Root` as its root view.
///
/// Every `gpui-component` overlay — dialog, alert dialog, sheet, notification —
/// finds its host with `window.root::<Root>()` and panics when the window is
/// rooted at anything else. The runtime installs its own `ShellRoot` and cannot
/// name `Root`, so the catalog that needs one supplies it here. `ShellRoot`
/// keeps rendering inside it, and its own overlays with it.
fn open_window_with_root(
    cx: &mut gpui_shell::gpui::App,
    options: gpui_shell::gpui::WindowOptions,
    build: &mut dyn FnMut(
        &mut gpui_shell::gpui::Window,
        &mut gpui_shell::gpui::App,
    ) -> gpui_shell::gpui::AnyView,
) -> gpui_shell::anyhow::Result<gpui_shell::gpui::AnyWindowHandle> {
    use gpui_shell::gpui::AppContext as _;

    let handle = cx.open_window(with_title_bar(options), |window, cx| {
        let inner = build(window, cx);
        cx.new(|cx| gpui_component::Root::new(inner, window, cx))
    })?;
    Ok(handle.into())
}

/// Hands the window chrome to `gpui_component::TitleBar`.
///
/// A script renders `TitleBar` the way a Rust application does, and the
/// catalog's overlays — sheets, notifications — already leave its height free
/// at the top of the window. Under the system title bar that height is a gap
/// above every sheet, and a script `TitleBar` sits below a second, native one.
/// So every window the catalog opens takes `TitleBar::window_options`, the
/// same base gpui-component's own windows use, keeping the host's bounds and
/// the title a switcher shows.
fn with_title_bar(options: gpui_shell::gpui::WindowOptions) -> gpui_shell::gpui::WindowOptions {
    let chrome = gpui_component::TitleBar::window_options();
    let title = options
        .titlebar
        .as_ref()
        .and_then(|titlebar| titlebar.title.clone());
    gpui_shell::gpui::WindowOptions {
        titlebar: chrome
            .titlebar
            .map(|titlebar| gpui_shell::gpui::TitlebarOptions { title, ..titlebar }),
        app_owns_titlebar_drag: chrome.app_owns_titlebar_drag,
        ..options
    }
}

/// Registers this catalog's descriptors into its private catalog assembly.
fn register(registry: &mut gpui_shell::ComponentRegistry) -> Result<(), gpui_shell::RegistryError> {
    shell::register(registry)
}

#[cfg(test)]
mod tests {
    use gpui_shell::{ArgumentSchema, COMPONENT_REGISTRY_API_VERSION, ComponentRegistry};

    /// Every `gpui-component` overlay locates its host with
    /// `window.root::<Root>()` and panics when the window is rooted at anything
    /// else. The runtime roots its window at `ShellRoot`, so unless the catalog
    /// supplies the `Root` itself, every dialog, alert dialog, sheet and
    /// notification panics the moment it opens.
    ///
    /// The other catalog tests build their own `Root`, which is why none
    /// of them noticed.
    #[gpui::test]
    fn the_catalog_opens_a_window_its_overlays_can_find(cx: &mut gpui::TestAppContext) {
        use gpui::AppContext as _;

        let components = super::components().unwrap();
        let open = components
            .window_opener()
            .expect("a catalog whose overlays require a window root must open the window");

        cx.update(super::init);
        let handle = cx
            .update(|cx| {
                let options = gpui::WindowOptions {
                    show: false,
                    ..Default::default()
                };
                open(cx, options, &mut |_window, cx| cx.new(|_| Blank).into())
            })
            .expect("the catalog must open a window");

        let found = handle
            .update(cx, |_, window, _| {
                window.root::<gpui_component::Root>().is_some()
            })
            .expect("the window must be live");
        assert!(
            found,
            "the window is not rooted at gpui_component::Root, so every overlay would panic"
        );
    }

    /// Opening a dialog is the whole point of the window root, and the panic it
    /// used to raise was only half the problem: with `Root` in place but its
    /// dialog layer unrendered, the dialog opens into a window that never draws
    /// it — indistinguishable, from the outside, from one that does not open.
    ///
    /// Debug builds only: GPUI records `debug_selector`s only with
    /// `debug_assertions`, so `debug_bounds` is always `None` under `--release`.
    #[cfg(debug_assertions)]
    #[gpui::test]
    fn a_dialog_opened_through_the_catalog_window_is_drawn(cx: &mut gpui::TestAppContext) {
        use gpui::AppContext as _;
        use gpui_component::WindowExt as _;

        let components = super::components().unwrap();
        let open = components.window_opener().unwrap();

        cx.update(super::init);
        let handle = cx
            .update(|cx| {
                let options = gpui::WindowOptions {
                    show: false,
                    ..Default::default()
                };
                open(cx, options, &mut |_window, cx| cx.new(|_| Blank).into())
            })
            .unwrap();

        handle
            .update(cx, |_, window, cx| {
                assert!(!window.has_active_dialog(cx), "nothing is open yet");
                window.open_dialog(cx, |dialog, _, _| dialog.title("Project details"));
                assert!(
                    window.has_active_dialog(cx),
                    "the dialog must reach a host that can hold it"
                );
            })
            .unwrap();

        // Drawn, not merely opened. `Root` renders the child, the tooltip and
        // the native menu; the dialog layer is the application root's to place,
        // so a dialog can be "open" and never reach the screen.
        let mut visual = gpui::VisualTestContext::from_window(handle, cx);
        visual.run_until_parked();
        visual.update(|window, cx| window.draw(cx).clear(cx));
        assert!(
            visual.debug_bounds("dialog-layer").is_some(),
            "the window root must render the dialog layer, or the dialog never appears"
        );
    }

    /// A script `TitleBar` only lines up with the overlays when the window's own
    /// title bar is transparent and leaves dragging to the app; the host's title
    /// and bounds still have to survive, or the window loses its name in a
    /// switcher and opens at the wrong size.
    #[test]
    fn the_catalog_window_hands_its_chrome_to_the_title_bar() {
        let bounds = gpui::WindowBounds::Windowed(gpui::Bounds::new(
            gpui::point(gpui::px(0.), gpui::px(0.)),
            gpui::size(gpui::px(880.), gpui::px(720.)),
        ));
        let options = super::with_title_bar(gpui::WindowOptions {
            window_bounds: Some(bounds),
            titlebar: Some(gpui::TitlebarOptions {
                title: Some("js_story — gpui-shell".into()),
                ..Default::default()
            }),
            ..Default::default()
        });

        let titlebar = options.titlebar.expect("the title bar options are kept");
        assert!(titlebar.appears_transparent);
        assert_eq!(
            titlebar.traffic_light_position,
            gpui_component::TitleBar::title_bar_options().traffic_light_position
        );
        assert_eq!(titlebar.title.as_deref(), Some("js_story — gpui-shell"));
        assert!(options.app_owns_titlebar_drag);
        assert_eq!(options.window_bounds, Some(bounds));
    }

    struct Blank;

    impl gpui::Render for Blank {
        fn render(
            &mut self,
            _: &mut gpui::Window,
            _: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            gpui::div()
        }
    }

    #[gpui::test]
    fn init_installs_the_component_catalog_globals(cx: &mut gpui::TestAppContext) {
        cx.update(super::init);

        cx.read(|cx| assert!(cx.has_global::<gpui_component::Theme>()));
    }

    /// The shipped command builds a runtime from [`super::components`] alone
    /// and never calls [`super::init`], so the catalog has to carry its own
    /// startup. Without it the binary starts and then panics looking for a
    /// theme on the first render, which no test of `init` would notice.
    #[gpui::test]
    fn the_frozen_catalog_carries_its_own_startup(cx: &mut gpui::TestAppContext) {
        let components = super::components().unwrap();
        assert!(components.initializer().is_some());

        cx.update(|cx| gpui_shell::init_with_components(cx, &components));

        cx.read(|cx| assert!(cx.has_global::<gpui_component::Theme>()));
    }

    #[test]
    fn register_exposes_the_first_leaf_component_batch_in_stable_order() {
        let mut registry = ComponentRegistry::new(
            COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();

        super::register(&mut registry).unwrap();

        let frozen = registry.freeze().unwrap();
        let descriptors = frozen.descriptors().collect::<Vec<_>>();
        assert_eq!(
            descriptors
                .iter()
                .take(3)
                .map(|descriptor| descriptor.name())
                .collect::<Vec<_>>(),
            ["Spinner", "Separator", "Skeleton"]
        );
        assert_eq!(
            descriptors
                .iter()
                .take(3)
                .flat_map(|descriptor| {
                    descriptor
                        .constructors()
                        .iter()
                        .map(|constructor| constructor.export())
                })
                .collect::<Vec<_>>(),
            [
                "Spinner",
                "Separator",
                "VerticalSeparator",
                "DashedSeparator",
                "VerticalDashedSeparator",
                "Skeleton",
            ]
        );
        let undocumented = descriptors
            .iter()
            .filter(|descriptor| descriptor.documentation().is_none())
            .map(|descriptor| descriptor.name())
            .collect::<Vec<_>>();
        assert!(
            undocumented.is_empty(),
            "every registered component needs documentation: {undocumented:?}"
        );
    }

    #[test]
    fn adapter_runtime_owns_the_registered_component_catalog() {
        let runtime = super::new_isolated_runtime().unwrap();

        let declarations = runtime.type_declarations();
        assert!(declarations.contains(
            "export const Spinner: { new(): SpinnerElement; new: () => SpinnerElement };"
        ));
        assert!(declarations.contains(
            "export const Skeleton: { new(): SkeletonElement; new: () => SkeletonElement };"
        ));
    }

    #[test]
    fn leaf_descriptors_publish_only_closed_honest_method_schemas() {
        let mut registry = ComponentRegistry::new(
            COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        super::register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();

        let spinner = frozen
            .descriptors()
            .find(|item| item.name() == "Spinner")
            .unwrap();
        assert_eq!(
            spinner
                .methods()
                .iter()
                .map(|method| (method.name(), method.arguments()))
                .collect::<Vec<_>>(),
            [
                (
                    "size",
                    [gpui_shell::ArgumentDescriptor::new(
                        "size",
                        ArgumentSchema::Enum(&["xsmall", "small", "medium", "large"]),
                    )]
                    .as_slice(),
                ),
                (
                    "icon",
                    [gpui_shell::ArgumentDescriptor::new(
                        "icon",
                        ArgumentSchema::Enum(&["loader", "loader_circle"]),
                    )]
                    .as_slice(),
                ),
                (
                    "color",
                    [gpui_shell::ArgumentDescriptor::new(
                        "color",
                        ArgumentSchema::String
                    )]
                    .as_slice(),
                ),
                (
                    "ease",
                    [gpui_shell::ArgumentDescriptor::new(
                        "ease",
                        ArgumentSchema::Enum(&["linear", "ease_in_out", "ease_out_quint"]),
                    )]
                    .as_slice(),
                ),
            ]
        );

        let separator = frozen
            .descriptors()
            .find(|item| item.name() == "Separator")
            .unwrap();
        assert_eq!(
            separator
                .methods()
                .iter()
                .map(|method| method.name())
                .collect::<Vec<_>>(),
            ["label", "color", "dashed"]
        );

        let skeleton = frozen
            .descriptors()
            .find(|item| item.name() == "Skeleton")
            .unwrap();
        assert_eq!(
            skeleton
                .methods()
                .iter()
                .map(|method| method.name())
                .collect::<Vec<_>>(),
            ["secondary"]
        );
        let undocumented = frozen
            .descriptors()
            .flat_map(|descriptor| {
                descriptor
                    .methods()
                    .iter()
                    .filter(|method| method.documentation().is_none())
                    .map(move |method| format!("{}.{}", descriptor.name(), method.name()))
            })
            .collect::<Vec<_>>();
        assert!(
            undocumented.is_empty(),
            "every generated TypeScript method needs documentation: {undocumented:?}"
        );
    }

    #[test]
    fn descriptor_vocabulary_uses_snake_case_everywhere() {
        let frozen = super::components().unwrap();
        let mut non_snake_case = Vec::new();

        for descriptor in frozen.descriptors() {
            for constructor in descriptor.constructors() {
                for argument in constructor.arguments() {
                    if argument.name().chars().any(char::is_uppercase) {
                        non_snake_case.push(format!(
                            "{} constructor argument `{}`",
                            descriptor.name(),
                            argument.name()
                        ));
                    }
                }
            }
            for method in descriptor.methods() {
                if method.name().chars().any(char::is_uppercase) {
                    non_snake_case.push(format!("{}.{}", descriptor.name(), method.name()));
                }
                for argument in method.arguments() {
                    if argument.name().chars().any(char::is_uppercase) {
                        non_snake_case.push(format!(
                            "{}.{} argument `{}`",
                            descriptor.name(),
                            method.name(),
                            argument.name()
                        ));
                    }
                }
            }
        }

        assert!(
            non_snake_case.is_empty(),
            "descriptor vocabulary must follow gpui-component snake_case: {non_snake_case:?}"
        );
    }

    #[test]
    fn main_chat_components_are_registered() {
        let frozen = super::components().unwrap();
        let names = frozen
            .descriptors()
            .map(|descriptor| descriptor.name())
            .collect::<std::collections::BTreeSet<_>>();

        for expected in [
            "Attachment",
            "Bubble",
            "Marker",
            "Message",
            "MessageScroller",
            "ShimmerText",
        ] {
            assert!(names.contains(expected), "missing `{expected}` descriptor");
        }
    }

    #[test]
    fn runtime_typings_include_leaf_exports_and_methods() {
        let mut registry = ComponentRegistry::new(
            COMPONENT_REGISTRY_API_VERSION,
            gpui_shell::DEFAULT_COMPONENT_MODULE,
        )
        .unwrap();
        super::register(&mut registry).unwrap();
        let runtime =
            gpui_shell::ShellRuntime::new_isolated_with_components(registry.freeze().unwrap())
                .unwrap();

        let declarations = runtime.type_declarations();
        for expected in [
            "export const Spinner: { new(): SpinnerElement; new: () => SpinnerElement };",
            "size(size: \"xsmall\" | \"small\" | \"medium\" | \"large\"): SpinnerElement;",
            "export const VerticalDashedSeparator: { new(): SeparatorElement; new: () => SeparatorElement };",
            "label(label: string): SeparatorElement;",
            "secondary(): SkeletonElement;",
        ] {
            assert!(declarations.contains(expected), "missing `{expected}`");
        }
    }
}

#[cfg(all(feature = "gpui-fast", test))]
extern crate gpui_fast as gpui;

fn main() {
    let components = match gpui_component_shell::components() {
        Ok(components) => components,
        Err(error) => {
            eprintln!("gpui-component-shell: {error:#}");
            std::process::exit(1);
        }
    };
    gpui_shell::host::main_with_brand_and_components(
        gpui_shell::host::HostBrand::new("gpui-component-shell", env!("CARGO_PKG_VERSION")),
        components,
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn component_shell_accepts_the_same_run_check_and_types_commands_as_shell() {
        use gpui_shell::host::{InvocationKind, parse_invocation};

        assert_eq!(
            parse_invocation(["examples/js_story"]).unwrap(),
            InvocationKind::Run
        );
        assert_eq!(
            parse_invocation(["check", "examples/js_story"]).unwrap(),
            InvocationKind::Check
        );
        assert_eq!(
            parse_invocation(["types", "examples/js_story"]).unwrap(),
            InvocationKind::Types
        );
    }
}

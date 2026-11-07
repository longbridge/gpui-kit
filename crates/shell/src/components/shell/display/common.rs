use gpui_component::Size;
use gpui_shell::{ComponentArgument, ComponentPayload, MaterializeRequest, anyhow};

pub(super) fn non_empty_id(component: &str, id: &str) -> Result<String, String> {
    if id.is_empty() {
        Err(format!("{component} id must not be empty"))
    } else {
        Ok(id.to_owned())
    }
}

pub(super) fn ensure_no_children(
    component: &str,
    request: &MaterializeRequest<'_>,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        request.children_len() == 0,
        "{component} does not accept child elements"
    );
    Ok(())
}

pub(super) fn size_operation<T>(
    component: &str,
    arguments: &[ComponentArgument],
    wrap: impl FnOnce(Size) -> T,
) -> Result<ComponentPayload, String>
where
    T: Send + Sync + 'static,
{
    let [ComponentArgument::Enum(size)] = arguments else {
        return Err(format!("{component}.size(size) expects a size literal"));
    };
    let size = match size.as_str() {
        "xsmall" => Size::XSmall,
        "small" => Size::Small,
        "medium" => Size::Medium,
        "large" => Size::Large,
        _ => return Err(format!("unsupported {component} size `{size}`")),
    };
    Ok(ComponentPayload::new(wrap(size)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_an_empty_component_id() {
        assert_eq!(
            non_empty_id("Rating", ""),
            Err("Rating id must not be empty".into())
        );
    }
}

use gpui_kit::{
    component::{
        ActiveTheme as _, Sizable as _,
        avatar::Avatar,
        h_flex,
        hover_card::HoverCard,
        text::{
            InlineElement, InlineRenderContext, MarkdownNode, MarkdownParseContext, MarkdownPlugin,
            markdown_ast,
        },
        v_flex,
    },
    *,
};

#[derive(Clone, Copy)]
struct Member {
    handle: &'static str,
    name: &'static str,
}

const MEMBERS: &[Member] = &[
    Member {
        handle: "huacnlee",
        name: "Jason Lee",
    },
    Member {
        handle: "madcodelife",
        name: "Floyd Wang",
    },
];

#[derive(Clone)]
pub struct MentionPlugin;

impl MarkdownPlugin for MentionPlugin {
    fn name(&self) -> &str {
        "mention"
    }

    fn parse(
        &self,
        node: &markdown_ast::Node,
        _: &MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        parse_mention(node)
    }

    fn render_inline(
        &self,
        node: &MarkdownNode,
        context: &InlineRenderContext,
        _: &mut Window,
        cx: &mut App,
    ) -> Option<InlineElement> {
        let member = *node.data::<Member>()?;
        let mut style = context.text_style().clone();
        style.color = cx.theme().link;
        style.font_weight = FontWeight::MEDIUM;
        style.underline = Some(UnderlineStyle {
            thickness: px(1.),
            ..Default::default()
        });
        let mut prefix = style.to_run(1);
        prefix.color = cx.theme().muted_foreground;
        let label = StyledText::new(format!("@{}", member.handle))
            .with_runs(vec![prefix, style.to_run(member.handle.len())]);
        Some(InlineElement::new(
            HoverCard::new("mention-profile")
                .anchor(Anchor::TopCenter)
                .trigger(
                    div()
                        .id("mention")
                        .cursor_default()
                        .text_size(context.font_size())
                        .line_height(context.line_height())
                        .whitespace_nowrap()
                        .child(label),
                )
                .content(move |_, _, cx| {
                    h_flex()
                        .gap_2()
                        .py_1()
                        .child(Avatar::new().name(member.name).small())
                        .child(
                            v_flex()
                                .child(div().font_weight(FontWeight::MEDIUM).child(member.name))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(format!("@{}", member.handle)),
                                ),
                        )
                }),
        ))
    }
}

fn parse_mention(node: &markdown_ast::Node) -> Option<MarkdownNode> {
    let markdown_ast::Node::Link(link) = node else {
        return None;
    };
    let handle = link.url.strip_prefix("mention:")?;
    let member = MEMBERS.iter().find(|member| member.handle == handle)?;
    Some(
        MarkdownNode::new("mention", *member)
            .text(format!("@{}", member.handle))
            .accessibility_label(format!("{} (@{})", member.name, member.handle)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(url: &str) -> markdown_ast::Node {
        markdown_ast::Node::Link(markdown_ast::Link {
            url: url.into(),
            title: None,
            children: vec![],
            position: None,
        })
    }

    #[::core::prelude::v1::test]
    fn mention_parser_exposes_plain_text_and_accessible_name() {
        let node = parse_mention(&link("mention:huacnlee")).unwrap();
        assert_eq!(node.as_text(), "@huacnlee");
        assert_eq!(node.accessibility_name(), "Jason Lee (@huacnlee)");
    }

    #[::core::prelude::v1::test]
    fn mention_parser_leaves_regular_links_and_inline_code_unchanged() {
        for node in [
            link("https://github.com/huacnlee"),
            link("mention:nobody"),
            markdown_ast::Node::InlineCode(markdown_ast::InlineCode {
                value: "@huacnlee".into(),
                position: None,
            }),
        ] {
            assert!(parse_mention(&node).is_none());
        }
    }
}

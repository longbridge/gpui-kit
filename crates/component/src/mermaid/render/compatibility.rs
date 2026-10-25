//! Source-backed repairs for omissions in Merman 0.9.2's SVG emitters.
//!
//! Keep the user's source intact. Only the private rendering copy is adapted,
//! using parser-owned spans so comments, frontmatter and accessibility metadata
//! cannot be mistaken for diagram statements.

use std::borrow::Cow;

use anyhow::{Result, ensure};
use merman::svg::{DeterministicTextMeasurer, TextMeasurer, TextStyle};
use merman::{Engine, OperationControl, ParsedEditorFacts};
use unicode_segmentation::UnicodeSegmentation as _;

pub(super) struct Compatibility<'a> {
    pub(super) source: Cow<'a, str>,
    orphan_section: Option<String>,
    title: Option<String>,
    accessible_title: Option<String>,
    vertical_timeline: bool,
    title_class: &'static str,
}

impl<'a> Compatibility<'a> {
    pub(super) fn new(
        source: &'a str,
        engine: &Engine,
        control: &OperationControl,
    ) -> Result<Self> {
        let mut result = Self {
            source: Cow::Borrowed(source),
            orphan_section: None,
            title: None,
            accessible_title: None,
            vertical_timeline: false,
            title_class: "railroad-rule-name gpui-mermaid-title",
        };
        // This is only a fast rejection, never a substitute for detection/parsing.
        // Avoid a second parse for the common flowchart/sequence/class path.
        if !source.lines().any(|line| {
            let line = line.trim_start();
            line.starts_with("timeline") || line.starts_with("railroad")
        }) {
            return Ok(result);
        }
        let Some(snapshot) = engine.parse_diagram_snapshot_controlled_sync(source, control)??
        else {
            return Ok(result);
        };
        let Some(model) = snapshot.outcome().parsed_model() else {
            // Let the original render operation report the original diagnostic.
            return Ok(result);
        };
        match snapshot.metadata().diagram_type.as_str() {
            "railroad" | "railroadEbnf" | "railroadAbnf" | "railroadPeg" => {
                result.title = model.as_value()["title"]
                    .as_str()
                    .filter(|title| !title.trim().is_empty())
                    .map(str::to_owned)
                    .or_else(|| snapshot.metadata().title.clone());
            }
            "timeline" => {
                result.vertical_timeline = model.as_value()["direction"] == "TD";
                if model.as_value()["title"].as_str().is_none_or(str::is_empty) {
                    result.title = snapshot.metadata().title.clone();
                    result.title_class = "gpui-mermaid-title";
                }
                result.accessible_title = model.as_value()["accTitle"]
                    .as_str()
                    .filter(|title| !title.is_empty())
                    .map(str::to_owned);
                let ParsedEditorFacts::Available(facts) = snapshot.editor_facts() else {
                    return Ok(result);
                };
                let first_section = facts
                    .symbols
                    .iter()
                    .find(|symbol| symbol.detail.as_deref() == Some("timeline section"));
                let first_task = facts
                    .symbols
                    .iter()
                    .find(|symbol| symbol.detail.as_deref() == Some("timeline task"));
                if let (Some(section), Some(task)) = (first_section, first_task)
                    && task.span.start < section.span.start
                {
                    // 0.9.2 lays out either sections OR orphan tasks. A private
                    // section admits the orphan tasks to the existing layout;
                    // its header is removed below, leaving them visibly ungrouped.
                    // Choose a name absent from the source, including user labels.
                    let mut marker = "gpui-mermaid-ungrouped".to_string();
                    while source.contains(&marker) {
                        marker.push('_');
                    }
                    ensure!(
                        source.is_char_boundary(task.span.start),
                        "Invalid timeline source span"
                    );
                    let mut adapted = source.to_owned();
                    adapted.insert_str(task.span.start, &format!("section {marker}\n"));
                    result.source = Cow::Owned(adapted);
                    result.orphan_section = Some(marker);
                }
            }
            _ => {}
        }
        Ok(result)
    }

    pub(super) fn finish(&self, mut svg: String) -> Result<String> {
        if let Some(title) = &self.accessible_title {
            let document = roxmltree::Document::parse(&svg)?;
            if !document
                .root_element()
                .children()
                .any(|node| node.has_tag_name("title"))
            {
                let closing = document.root_element().range().end - "</svg>".len();
                svg.insert_str(
                    closing,
                    &format!(
                        "<title id=\"gpui-mermaid-accessible-title\">{}</title>",
                        escape_text(title)
                    ),
                );
                svg.insert_str(
                    "<svg".len(),
                    " aria-labelledby=\"gpui-mermaid-accessible-title\"",
                );
            }
        }
        if let Some(marker) = &self.orphan_section {
            let document = roxmltree::Document::parse(&svg)?;
            let header = document.descendants().find(|node| {
                node.has_tag_name("g")
                    && node.attribute("class").is_some_and(|value| {
                        value
                            .split_whitespace()
                            .any(|class| class == "timeline-node")
                    })
                    && node.descendants().any(|child| child.text() == Some(marker))
            });
            let range = header.map(|node| node.range());
            ensure!(range.is_some(), "Timeline compatibility header not found");
            svg.replace_range(range.unwrap(), "");
        }
        if self.vertical_timeline {
            svg = super::timeline::repair_vertical_svg(svg)?;
        }
        if let Some(title) = &self.title {
            let document = roxmltree::Document::parse(&svg)?;
            let root = document.root_element();
            let view_box = root
                .attribute_node("viewBox")
                .ok_or_else(|| anyhow::anyhow!("Diagram canvas missing"))?;
            let values = view_box
                .value()
                .split_whitespace()
                .map(str::parse::<f64>)
                .collect::<std::result::Result<Vec<_>, _>>()?;
            ensure!(
                values.len() == 4 && values.iter().all(|value| value.is_finite()),
                "Invalid diagram canvas"
            );
            let [x, y, width, height] = [values[0], values[1], values[2], values[3]];
            let css = document
                .descendants()
                .filter(|node| node.has_tag_name("style"))
                .filter_map(|node| node.text())
                .flat_map(|css| css.split('}'))
                .filter_map(|rule| rule.split_once('{'))
                .find(|(selector, _)| selector.contains(".railroad-rule-name"))
                .map(|(_, css)| css)
                .unwrap_or_default();
            let property = |name| {
                css.split(';')
                    .filter_map(|part| part.split_once(':'))
                    .find(|(key, _)| key.trim() == name)
                    .map(|(_, value)| value.trim())
            };
            let style = TextStyle {
                font_family: property("font-family").map(str::to_owned),
                font_size: property("font-size")
                    .and_then(|value| value.strip_suffix("px"))
                    .and_then(|value| value.parse::<f64>().ok())
                    .filter(|value| value.is_finite() && *value > 0.)
                    .unwrap_or(16.),
                font_weight: Some("bold".into()),
                font_style: None,
            };
            let measurer = DeterministicTextMeasurer::default();
            let lines = wrap_title(title, (width.min(640.) - 16.).max(1.), &style);
            let line_height = style.font_size * 1.5;
            let title_height = line_height * lines.len() as f64 + 16.;
            // Use the same typography as the rule names. Expand the canvas above
            // the rules instead of translating paths, markers or accessibility nodes.
            let mut markup = String::new();
            for (index, line) in lines.iter().enumerate() {
                let length = measurer.measure(line, &style).width.max(1.);
                markup.push_str(&format!(r#"<text class="{}" style="font-weight:bold" x="{}" y="{}" text-anchor="middle" textLength="{length}" lengthAdjust="spacingAndGlyphs" xml:space="preserve">{}</text>"#, self.title_class, x + width / 2., y - title_height + line_height * (index + 1) as f64, escape_text(line)));
            }
            let closing = root.range().end - "</svg>".len();
            let range = view_box.range_value();
            svg.insert_str(closing, &markup);
            svg.replace_range(
                range,
                &format!(
                    "{} {} {} {}",
                    x - 8.,
                    y - title_height,
                    width + 16.,
                    height + title_height + 8.
                ),
            );
        }
        Ok(svg)
    }
}

fn wrap_title(title: &str, width: f64, style: &TextStyle) -> Vec<String> {
    let measurer = DeterministicTextMeasurer::default();
    let mut lines = Vec::new();
    let mut line = String::new();
    for grapheme in title.graphemes(true) {
        let candidate = format!("{line}{grapheme}");
        if !line.is_empty() && measurer.measure(&candidate, style).width > width {
            if let Some((index, space)) =
                line.char_indices().rev().find(|(_, ch)| ch.is_whitespace())
            {
                let remainder = line.split_off(index + space.len_utf8());
                lines.push(std::mem::replace(&mut line, remainder));
            } else {
                lines.push(std::mem::take(&mut line));
            }
        }
        line.push_str(grapheme);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

fn escape_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mermaid::render::{diagram_config, render_svg, svg_dimensions};
    use serde_json::json;

    fn visible_labels(svg: &str) -> Vec<String> {
        roxmltree::Document::parse(svg)
            .unwrap()
            .descendants()
            .filter(|node| node.has_tag_name("text"))
            .map(|node| {
                node.descendants()
                    .filter(|child| child.is_text())
                    .filter_map(|child| child.text())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn timeline_keeps_ungrouped_events_before_named_sections() {
        for direction in ["", " TD"] {
            for prefix in ["", "---\ntitle: Roadmap\n---\n%% section Ignored\n"] {
                let source = format!(
                    "{prefix}timeline{direction}\n  accTitle: Accessible timeline\n  2019 : Planning\n    : Launch\n  2020 : Growth\n  section Future\n  2022 : Expansion\n  2023 : Optimization\n"
                );
                for source in [source.clone(), source.replace('\n', "\r\n")] {
                    let svg = render_svg(
                        &source,
                        diagram_config("sans-serif", json!({})),
                        OperationControl::new(),
                    )
                    .unwrap();
                    let labels = visible_labels(&svg);
                    for expected in [
                        "2019",
                        "Planning",
                        "Launch",
                        "2020",
                        "Growth",
                        "Future",
                        "2022",
                        "Expansion",
                        "2023",
                        "Optimization",
                    ] {
                        assert_eq!(
                            labels.iter().filter(|label| *label == expected).count(),
                            1,
                            "{expected}: {labels:?}"
                        );
                    }
                    assert!(!svg.contains("gpui-mermaid-ungrouped"));
                    assert!(svg.contains("Accessible timeline"));
                    if !prefix.is_empty() {
                        assert!(labels.iter().any(|label| label == "Roadmap"));
                    }
                    assert!(svg_dimensions(&svg).is_ok());
                }
            }
        }
        for source in [
            "timeline\n2019 : Planning",
            "timeline\nsection Future\n2022 : Expansion",
        ] {
            let engine = Engine::new();
            let adapted = Compatibility::new(source, &engine, &OperationControl::new()).unwrap();
            assert!(matches!(adapted.source, Cow::Borrowed(_)));
        }
    }

    #[test]
    fn vertical_timeline_section_cannot_cover_the_previous_event() {
        let source = include_str!("../fixtures/timeline-vertical.mmd");
        let svg = render_svg(
            source,
            diagram_config("sans-serif", json!({})),
            OperationControl::new(),
        )
        .unwrap();
        let document = roxmltree::Document::parse(&svg).unwrap();
        let text = |label: &str| {
            document
                .descendants()
                .find(|node| node.is_text() && node.text() == Some(label))
                .unwrap()
        };
        let event = text("Prototype")
            .ancestors()
            .find(|node| node.attribute("class") == Some("eventWrapper"))
            .unwrap();
        let (_, event_y) =
            super::super::svg::translation(event.attribute("transform").unwrap()).unwrap();
        let height = event
            .descendants()
            .filter(|node| node.has_tag_name("line"))
            .filter_map(|node| node.attribute("y1")?.parse::<f64>().ok())
            .max_by(f64::total_cmp)
            .unwrap();
        let section = text("Delivery")
            .ancestors()
            .find(|node| {
                node.attribute("class")
                    .is_some_and(|class| class.starts_with("timeline-node"))
            })
            .unwrap()
            .parent()
            .unwrap();
        let (_, section_y) =
            super::super::svg::translation(section.attribute("transform").unwrap()).unwrap();
        assert!(
            section_y >= event_y + height + 19.9,
            "section covers previous event"
        );
        assert_eq!(
            super::super::timeline::repair_vertical_svg(svg.clone()).unwrap(),
            svg
        );
    }

    #[test]
    fn railroad_preserves_visible_title_rules_and_accessibility() {
        for title in [
            "Simple Grammar",
            "中文 Grammar & Rules",
            "Long 中文 title with a combined character e\u{301} and an emoji 👩‍💻 that must remain intact when wrapped",
        ] {
            let source = format!(
                "railroad-beta\ntitle \"{title}\"\naccTitle: Accessible grammar\nrule = terminal(\"a\") ;\n"
            );
            let svg = render_svg(
                &source,
                diagram_config("sans-serif", json!({})),
                OperationControl::new(),
            )
            .unwrap();
            let labels = visible_labels(&svg);
            let document = roxmltree::Document::parse(&svg).unwrap();
            let rendered_title = document
                .descendants()
                .filter(|node| {
                    node.attribute("class") == Some("railroad-rule-name gpui-mermaid-title")
                })
                .filter_map(|node| node.text())
                .collect::<String>();
            assert_eq!(rendered_title, title);
            assert!(labels.iter().any(|label| label.contains("rule")));
            assert!(svg.contains("Accessible grammar"));
            assert!(svg_dimensions(&svg).is_ok());
        }
        let source = "railroad-beta\nrule = terminal(\"a\") ;";
        let svg = render_svg(
            source,
            diagram_config("sans-serif", json!({})),
            OperationControl::new(),
        )
        .unwrap();
        assert!(!svg.contains("gpui-mermaid-title"));
    }
}

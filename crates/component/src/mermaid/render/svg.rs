//! Narrow compatibility repairs for Merman 0.9.2 output.

use anyhow::{Result, ensure};
use merman::svg::{DeterministicTextMeasurer, TextMeasurer, TextStyle};
use std::ops::Range;

type Edits = Vec<(Range<usize>, String)>;

#[cfg(test)]
fn prepare_svg(svg: String) -> Result<String> {
    prepare_svg_on_canvas(svg, "#FFFFFF", "sans-serif")
}

pub(super) fn prepare_svg_on_canvas(
    mut svg: String,
    canvas: &str,
    font_family: &str,
) -> Result<String> {
    let document = roxmltree::Document::parse(&svg)?;
    let root = document.root_element();
    let mut edits = Vec::new();
    center_class_edge_labels(&document, &mut edits)?;
    // Flowchart's half-transparent label backgrounds let edges cut through
    // glyphs. Preserve the generated geometry and color, but mask the edge.
    if root.attribute("aria-roledescription") == Some("flowchart-v2") {
        for background in document.descendants().filter(|node| {
            node.has_tag_name("rect")
                && has_class(*node, "background")
                && node
                    .ancestors()
                    .any(|parent| has_class(parent, "edgeLabel"))
        }) {
            attribute(background, "style", "opacity:1;fill-opacity:1;", &mut edits);
        }
    }
    if root.attribute("aria-roledescription") == Some("railroad") {
        for rule in document
            .descendants()
            .filter(|node| has_class(*node, "railroad-rule"))
        {
            let marker = rule
                .children()
                .find(|node| has_class(*node, "railroad-start"))
                .and_then(|node| node.children().find(|child| child.has_tag_name("circle")));
            let name = rule
                .descendants()
                .find(|node| node.has_tag_name("text") && has_class(*node, "railroad-rule-name"));
            if let (Some(marker), Some(name)) = (marker, name)
                && let (Some(x), Some(y)) = (number(marker, "cx"), number(marker, "cy"))
                && x > 20.
            {
                // Merman reserves 20px after the estimated (regular-weight)
                // name. Fit the actual bold font to that slot, keeping the
                // marker gap and the same middle baseline as terminal labels.
                attribute(name, "dominant-baseline", "middle", &mut edits);
                attribute(name, "y", &y.to_string(), &mut edits);
                attribute(name, "textLength", &(x - 20.).to_string(), &mut edits);
                attribute(name, "lengthAdjust", "spacingAndGlyphs", &mut edits);
            }
        }
    }
    if root.attribute("aria-roledescription") == Some("treeView") {
        attribute(root, "font-family", font_family, &mut edits);
        // Tree View's canvas ends at the estimated rightmost label. Keep native
        // glyph widths within that canvas when selecting the host's actual font.
        let right = root.attribute("viewBox").and_then(|value| {
            let values = value
                .split_whitespace()
                .map(str::parse::<f64>)
                .collect::<std::result::Result<Vec<_>, _>>()
                .ok()?;
            (values.len() == 4).then(|| values[0] + values[2])
        });
        let measurer = DeterministicTextMeasurer::default();
        for text in document
            .descendants()
            .filter(|node| node.has_tag_name("text"))
        {
            if let (Some(right), Some(x)) = (right, number(text, "x")) {
                let content = text
                    .descendants()
                    .filter(|node| node.is_text())
                    .filter_map(|node| node.text())
                    .collect::<String>();
                let style = TextStyle {
                    font_family: Some(font_family.to_owned()),
                    font_size: css_property(&document, ".treeView-node-label", "font-size")
                        .and_then(|value| value.strip_suffix("px"))
                        .and_then(|value| value.parse::<f64>().ok())
                        .filter(|value| value.is_finite() && *value > 0.)
                        .unwrap_or(16.),
                    font_weight: has_class(text, "treeView-node-dir").then(|| "bold".into()),
                    font_style: None,
                };
                let width = measurer
                    .measure(&content, &style)
                    .width
                    .min(right - x)
                    .max(1.);
                attribute(text, "textLength", &width.to_string(), &mut edits);
                attribute(text, "lengthAdjust", "spacingAndGlyphs", &mut edits);
            }
        }
        for line in document
            .descendants()
            .filter(|node| has_class(*node, "treeView-node-line"))
        {
            if number(line, "x1").is_some_and(|x| x < 0.) && number(line, "x2") == Some(0.) {
                edits.push((line.range(), String::new()));
            }
        }
    }
    repair_labels(&document, canvas, &mut edits);
    // Info emits a browser-sized root (100% width, no height or viewBox).
    // Its version label is at (100, 40), with a 400px maximum width.
    if root.attribute("aria-roledescription") == Some("info") && root.attribute("viewBox").is_none()
    {
        let start = root.range().start + "<svg".len();
        edits.push((start..start, " viewBox=\"0 0 400 80\"".to_string()));
    }
    for node in document.descendants() {
        // The fallback's tight viewport uses estimated font metrics. Actual
        // system glyphs can extend past it even though the containing node has
        // enough padding. Give SVG text its normal overflow behavior; preserve
        // node geometry, wrapping, explicit clips and all non-label viewports.
        if node.has_tag_name("svg")
            && node.children().any(|child| {
                child.has_tag_name("text")
                    && child.attribute("class") == Some("merman-foreignobject-fallback-text")
            })
            && let Some(style) = node.attribute_node("style")
        {
            edits.push((
                style.range_value(),
                escape_attribute(
                    &style
                        .value()
                        .replace("overflow:hidden;", "overflow:visible;"),
                ),
            ));
        }
        // Use Case applies a left-edge translation to middle-anchored native
        // labels. Place them at the actor/ellipse center or the boundary center.
        if root.attribute("class") == Some("usecaseDiagram")
            && node.has_tag_name("text")
            && node.attribute("text-anchor") == Some("middle")
            && let Some(label) = node.parent()
            && let Some(container) = label.parent()
            && let Some(transform) = label.attribute_node("transform")
            && let Some((x, y)) = translation(transform.value())
        {
            let center = match container.attribute("data-usecase-kind") {
                Some("actor" | "usecase") => Some(0.),
                Some("boundary") => container.children().find_map(|shape| {
                    if !shape.has_tag_name("rect") {
                        return None;
                    }
                    Some(
                        shape.attribute("x")?.parse::<f64>().ok()?
                            + shape.attribute("width")?.parse::<f64>().ok()? / 2.,
                    )
                }),
                _ => None,
            };
            if let Some(center) = center
                && center.is_finite()
                && y.is_finite()
                && x != center
            {
                edits.push((transform.range_value(), format!("translate({center},{y})")));
            }
        }
    }
    edits.sort_unstable_by_key(|(range, _)| range.start);
    for (range, value) in edits.into_iter().rev() {
        svg.replace_range(range, &value);
    }
    Ok(svg)
}

/// Merman 0.9.2 positions native class edge labels at the left of their measured
/// box, but emits middle-anchored rows at x=0. Align those rows with the box.
/// Keep node text, custom styles and all other diagram families untouched.
fn center_class_edge_labels(document: &roxmltree::Document, edits: &mut Edits) -> Result<()> {
    if document.root_element().attribute("class") != Some("classDiagram") {
        return Ok(());
    }
    for text in document.descendants().filter(|node| {
        node.has_tag_name("text")
            && node.attribute("text-anchor") == Some("middle")
            && node
                .ancestors()
                .any(|ancestor| ancestor.attribute("class") == Some("edgeLabel"))
    }) {
        let Some(background) = text.parent().and_then(|parent| {
            parent.children().find(|node| {
                node.has_tag_name("rect") && node.attribute("class") == Some("background")
            })
        }) else {
            continue;
        };
        let Some(width) = background
            .attribute("width")
            .and_then(|value| value.parse::<f64>().ok())
        else {
            continue;
        };
        let x = background.attribute("x").unwrap_or("0").parse::<f64>()?;
        let center = x + width / 2.;
        ensure!(
            width > 0. && center.is_finite(),
            "Invalid Mermaid label dimensions"
        );
        for row in text.children().filter(|node| node.has_tag_name("tspan")) {
            if row.attribute("text-anchor") == Some("middle")
                && let Some(x) = row.attribute_node("x")
                && x.value() == "0"
            {
                edits.push((x.range_value(), center.to_string()));
            }
        }
    }
    Ok(())
}

fn escape_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}

fn attribute(node: roxmltree::Node, name: &str, value: &str, edits: &mut Edits) {
    let escaped = escape_attribute(value);
    if let Some(attribute) = node.attribute_node(name) {
        if attribute.value() != value {
            edits.push((attribute.range_value(), escaped));
        }
    } else {
        let start = node.range().start + 1 + node.tag_name().name().len();
        edits.push((start..start, format!(" {name}=\"{escaped}\"")));
    }
}

fn has_class(node: roxmltree::Node, class: &str) -> bool {
    node.attribute("class")
        .is_some_and(|value| value.split_whitespace().any(|value| value == class))
}

fn number(node: roxmltree::Node, name: &str) -> Option<f64> {
    node.attribute(name)?
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

fn css_property<'a>(
    document: &'a roxmltree::Document,
    selector: &str,
    property: &str,
) -> Option<&'a str> {
    document
        .descendants()
        .filter(|node| node.has_tag_name("style"))
        .filter_map(|node| node.text())
        .flat_map(|css| css.split('}'))
        .filter_map(|rule| rule.split_once('{'))
        .filter(|(selectors, _)| {
            selectors
                .split(',')
                .any(|value| value.trim().ends_with(selector))
        })
        .flat_map(|(_, declarations)| declarations.split(';'))
        .filter_map(|value| value.split_once(':'))
        .find(|(name, _)| name.trim() == property)
        .map(|(_, value)| value.trim())
}

fn contrast_text(fill: &str) -> Option<&'static str> {
    let hex = fill.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let rgb = u32::from_str_radix(hex, 16).ok()?;
    let linear = |shift: u32| {
        let value = ((rgb >> shift) & 255_u32) as f64 / 255.;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance = 0.2126 * linear(16) + 0.7152 * linear(8) + 0.0722 * linear(0);
    Some(if luminance > 0.179 {
        "#000000"
    } else {
        "#FFFFFF"
    })
}

fn repair_labels(document: &roxmltree::Document, canvas: &str, edits: &mut Edits) {
    let root = document.root_element();
    let family = root.attribute("aria-roledescription").unwrap_or_default();
    if family == "journey" {
        for text in document.descendants().filter(|node| {
            node.has_tag_name("text")
                && (has_class(*node, "task") || has_class(*node, "journey-section"))
        }) {
            if let Some(rect) = text
                .parent()
                .into_iter()
                .flat_map(|group| group.prev_siblings())
                .find(|node| node.has_tag_name("rect"))
                && let Some(class) = rect.attribute("class").and_then(|classes| {
                    classes.split_whitespace().find(|class| {
                        class.starts_with("task-type-") || class.starts_with("section-type-")
                    })
                })
            {
                let selector = format!(".{class}");
                let fill = css_property(document, &selector, "fill");
                if let Some(color) = fill.and_then(contrast_text) {
                    text_fill(text, color, edits);
                }
            }
        }
    }
    if family == "c4" {
        // C4 computes the relation midpoint, then its shared text helper adds
        // half the label width as if that midpoint were a left edge. Undo only
        // that extra offset; authored relation offsets are retained.
        let measurer = DeterministicTextMeasurer::default();
        for text in document
            .descendants()
            .filter(|node| node.has_tag_name("text"))
        {
            if text.prev_siblings().any(|sibling| {
                sibling.attribute("marker-end").is_some()
                    || sibling.attribute("marker-start").is_some()
            }) && let Some(x) = number(text, "x")
                && text.attribute("data-gpui-centered").is_none()
            {
                let css = text.attribute("style").unwrap_or_default();
                let property = |name: &str| {
                    css.split(';')
                        .filter_map(|part| part.split_once(':'))
                        .find(|(key, _)| key.trim() == name)
                        .map(|(_, value)| value.trim())
                };
                let style = TextStyle {
                    font_family: property("font-family").map(str::to_owned),
                    font_size: property("font-size")
                        .and_then(|value| value.strip_suffix("px"))
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(12.),
                    font_weight: property("font-weight").map(str::to_owned),
                    font_style: None,
                };
                // Every wrapped row shares the widest row's offset. Technology
                // rows use yet another width; align them with the message too.
                let relation = text
                    .prev_siblings()
                    .find(|node| {
                        node.attribute("marker-end").is_some()
                            || node.attribute("marker-start").is_some()
                    })
                    .unwrap();
                let rows = relation
                    .next_siblings()
                    .skip(1)
                    .take_while(|node| {
                        node.attribute("marker-end").is_none()
                            && node.attribute("marker-start").is_none()
                    })
                    .filter(|node| {
                        node.has_tag_name("text") && node.attribute("font-style") != Some("italic")
                    })
                    .collect::<Vec<_>>();
                let width = rows
                    .iter()
                    .map(|row| {
                        let label = row
                            .descendants()
                            .filter(|node| node.is_text())
                            .filter_map(|node| node.text())
                            .collect::<String>();
                        measurer
                            .measure_mermaid_calculate_text_dimensions(&label, &style)
                            .width
                            .round()
                    })
                    .fold(0., f64::max);
                let label_x = rows.first().and_then(|row| number(*row, "x")).unwrap_or(x);
                attribute(text, "x", &(label_x - width / 2.).to_string(), edits);
                attribute(
                    text,
                    "style",
                    &format!(
                        "{css};paint-order:stroke;stroke:{canvas};stroke-width:3px;stroke-linejoin:round;"
                    ),
                    edits,
                );
                attribute(text, "data-gpui-centered", "true", edits);
            }
        }
    }
    if family == "cynefin" {
        // Default connectors target domain centers, where subtitles also live.
        // Paint them behind labels and items, with a halo to keep glyphs clear.
        let arrows = document
            .descendants()
            .find(|node| has_class(*node, "cynefin-arrows"));
        let labels = document
            .descendants()
            .find(|node| has_class(*node, "cynefin-labels"));
        if let (Some(arrows), Some(labels)) = (arrows, labels)
            && arrows.range().start > labels.range().start
        {
            let start = labels.range().start;
            edits.push((
                start..start,
                document.input_text()[arrows.range()].to_owned(),
            ));
            edits.push((arrows.range(), String::new()));
        }
        for text in document.descendants().filter(|node| {
            has_class(*node, "cynefinSubtitle") || has_class(*node, "cynefinDomainLabel")
        }) {
            attribute(
                text,
                "style",
                &format!(
                    "paint-order:stroke;stroke:{canvas};stroke-width:2px;stroke-linejoin:round;"
                ),
                edits,
            );
        }
    }
    for node in document.descendants().filter(|node| node.has_tag_name("g")) {
        // Native State labels already position their first row relative to the
        // top of the background. Merman adds half its height a second time.
        if family.starts_with("state") && has_class(node, "edgeLabel") {
            for background in node
                .descendants()
                .filter(|node| node.has_tag_name("rect") && has_class(*node, "background"))
            {
                attribute(
                    background,
                    "style",
                    "stroke:none;opacity:1;fill-opacity:1;",
                    edits,
                );
                if let Some(parent) = background.parent() {
                    for label in parent.children().filter(|child| {
                        child.has_tag_name("g")
                            && child.children().any(|text| text.has_tag_name("text"))
                    }) {
                        if let Some((x, _)) = label.attribute("transform").and_then(translation) {
                            attribute(label, "transform", &format!("translate({x},0)"), edits);
                        }
                    }
                }
            }
        }
        // Constrain the built-in Requirement stereotype to the content width
        // used by the layout. System fonts can be wider than the measured font.
        if family == "requirement" && has_class(node, "node") {
            if let Some(label) = node.children().find(|child| has_class(*child, "label"))
                && let Some((x, _)) = label.attribute("transform").and_then(translation)
                && x < 0.
                && let Some(row) = label
                    .descendants()
                    .find(|child| has_class(*child, "text-outer-tspan"))
            {
                attribute(row, "textLength", &(-2. * x).to_string(), edits);
                attribute(row, "lengthAdjust", "spacingAndGlyphs", edits);
                // usvg applies textLength to the immediate parent of a text
                // run, not through Merman's word-level nested tspans. The
                // built-in stereotype is plain text; keep it in one run.
                if row.children().any(|child| child.has_tag_name("tspan")) {
                    let text = row
                        .descendants()
                        .filter(|child| child.is_text())
                        .filter_map(|child| child.text())
                        .collect::<String>();
                    if let (Some(first), Some(last)) = (row.first_child(), row.last_child()) {
                        edits.push((
                            first.range().start..last.range().end,
                            escape_attribute(&text),
                        ));
                    }
                }
            }
        }
        // Like Zed's element fixup, correct colors after raster-safe conversion.
        // These families paint categorical fills independently of HostTheme.
        let categorical =
            family == "cynefin" && node.children().any(|child| has_class(child, "cynefinItem"));
        let event_box = family == "eventmodeling" && has_class(node, "em-box");
        let lane = family == "eventmodeling" && has_class(node, "em-swimlane");
        if categorical || event_box || lane {
            let fill = node
                .children()
                .find(|child| child.has_tag_name("rect"))
                .and_then(|rect| rect.attribute("fill"));
            if let Some(color) = fill.and_then(contrast_text) {
                let target = if event_box {
                    node.descendants()
                        .find(|child| child.has_tag_name("use"))
                        .and_then(|used| used.attribute("href"))
                        .and_then(|href| href.strip_prefix('#'))
                        .and_then(|id| {
                            document
                                .descendants()
                                .find(|candidate| candidate.attribute("id") == Some(id))
                        })
                } else {
                    Some(node)
                };
                if let Some(target) = target {
                    for text in target
                        .descendants()
                        .filter(|child| child.has_tag_name("text"))
                    {
                        text_fill(text, color, edits);
                    }
                }
            }
        }
    }
}

fn text_fill(text: roxmltree::Node, color: &str, edits: &mut Edits) {
    let style = text.attribute("style").unwrap_or_default();
    let suffix = format!("fill:{color};");
    if !style.ends_with(&suffix) {
        let separator = if style.is_empty() || style.ends_with(';') {
            ""
        } else {
            ";"
        };
        attribute(text, "style", &format!("{style}{separator}{suffix}"), edits);
    }
}

pub(super) fn translation(value: &str) -> Option<(f64, f64)> {
    let mut values = value
        .strip_prefix("translate(")?
        .strip_suffix(')')?
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|value| !value.is_empty());
    let x = values.next()?.parse().ok()?;
    let y = values.next()?.parse().ok()?;
    values.next().is_none().then_some((x, y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mermaid::render::{diagram_config, render_svg, svg_dimensions};
    use merman::OperationControl;
    use serde_json::json;

    #[test]
    fn flowchart_edges_are_masked_by_single_and_multiline_labels() {
        let svg = render_svg(
            include_str!("../fixtures/flowchart-labels.mmd"),
            diagram_config("sans-serif", json!({})),
            OperationControl::new(),
        )
        .unwrap();
        let document = roxmltree::Document::parse(&svg).unwrap();
        let backgrounds = document
            .descendants()
            .filter(|node| {
                node.has_tag_name("rect")
                    && has_class(*node, "background")
                    && node
                        .ancestors()
                        .any(|parent| has_class(parent, "edgeLabel"))
            })
            .collect::<Vec<_>>();
        assert_eq!(backgrounds.len(), 3);
        for background in backgrounds {
            assert_eq!(
                background.attribute("style"),
                Some("opacity:1;fill-opacity:1;")
            );
        }
        assert_eq!(prepare_svg(svg.clone()).unwrap(), svg);
    }

    #[test]
    fn railroad_rule_names_share_the_rail_baseline_and_keep_the_marker_gap() {
        let svg = render_svg(
            "railroad-beta\ntitle \"Component grammar\"\ncomponent = terminal(\"Mermaid\") ;\nlong_component_rule_name = nonterminal(\"component\") ;",
            diagram_config("sans-serif", json!({})),
            OperationControl::new(),
        ).unwrap();
        let document = roxmltree::Document::parse(&svg).unwrap();
        let rules = document
            .descendants()
            .filter(|node| has_class(*node, "railroad-rule"))
            .collect::<Vec<_>>();
        assert_eq!(rules.len(), 2);
        for rule in rules {
            let name = rule
                .descendants()
                .find(|node| has_class(*node, "railroad-rule-name"))
                .unwrap();
            let marker = rule
                .descendants()
                .find(|node| {
                    node.has_tag_name("circle")
                        && node
                            .parent()
                            .is_some_and(|parent| has_class(parent, "railroad-start"))
                })
                .unwrap();
            assert_eq!(number(name, "y"), number(marker, "cy"));
            assert_eq!(name.attribute("dominant-baseline"), Some("middle"));
            assert_eq!(name.attribute("lengthAdjust"), Some("spacingAndGlyphs"));
            assert!(
                (number(marker, "cx").unwrap() - number(name, "textLength").unwrap() - 20.).abs()
                    < 0.001
            );
        }
        let title = document
            .descendants()
            .find(|node| has_class(*node, "gpui-mermaid-title"))
            .unwrap();
        assert_eq!(title.attribute("dominant-baseline"), None);
        assert_eq!(prepare_svg(svg.clone()).unwrap(), svg);
    }

    #[test]
    fn family_repairs_preserve_text_and_are_idempotent() {
        for source in [
            include_str!("../fixtures/c4.mmd"),
            include_str!("../fixtures/requirement.mmd"),
            include_str!("../fixtures/state-legacy.mmd"),
            include_str!("../fixtures/cynefin.mmd"),
            include_str!("../fixtures/eventmodeling.mmd"),
            include_str!("../fixtures/journey.mmd"),
            include_str!("../fixtures/treeview.mmd"),
        ] {
            let svg = render_svg(
                source,
                diagram_config("sans-serif", json!({})),
                OperationControl::new(),
            )
            .unwrap();
            let document = roxmltree::Document::parse(&svg).unwrap();
            assert!(document.descendants().any(|node| node.has_tag_name("text")));
            assert_eq!(prepare_svg(svg.clone()).unwrap(), svg);
            match document
                .root_element()
                .attribute("aria-roledescription")
                .unwrap()
            {
                "requirement" => {
                    let header = document
                        .descendants()
                        .find(|node| {
                            node.text() == Some("<<Functional Requirement>>") && node.is_element()
                        })
                        .unwrap();
                    assert!(header.attribute("textLength").is_some());
                    assert!(!header.children().any(|child| child.is_element()));
                }
                "c4" => assert!(svg.contains("data-gpui-centered=\"true\"")),
                "journey" => {
                    let label = document
                        .descendants()
                        .find(|node| node.has_tag_name("text") && has_class(*node, "task"))
                        .unwrap();
                    assert!(label.attribute("style").unwrap().contains("fill:#000000;"));
                }
                "treeView" => {
                    assert_eq!(
                        document.root_element().attribute("font-family"),
                        Some("sans-serif")
                    );
                    assert!(!document.descendants().any(|node| {
                        has_class(node, "treeView-node-line")
                            && number(node, "x1").is_some_and(|x| x < 0.)
                    }));
                }
                "cynefin" => {
                    let label = document
                        .descendants()
                        .find(|node| has_class(*node, "cynefinItemText"))
                        .unwrap();
                    assert!(label.attribute("style").unwrap().contains("fill:#000000;"));
                    assert!(
                        svg.find("class=\"cynefin-arrows\"").unwrap()
                            < svg.find("class=\"cynefin-labels\"").unwrap()
                    );
                }
                _ => {}
            }
        }
        let fallback = r#"<svg><svg style="overflow:hidden;font-family:&quot;A&amp;B&quot;;"><text class="merman-foreignobject-fallback-text">Label</text></svg></svg>"#;
        let repaired = prepare_svg(fallback.into()).unwrap();
        assert!(roxmltree::Document::parse(&repaired).is_ok());
    }

    #[test]
    fn c4_wrapped_relationship_and_technology_share_the_midpoint() {
        let svg = render_svg(
            "C4Context\nPerson(user, \"User\")\nSystem(api, \"API\")\nRel(user, api, \"A longer message<br/>short\", \"HTTPS\")",
            diagram_config("sans-serif", json!({})),
            OperationControl::new(),
        ).unwrap();
        let document = roxmltree::Document::parse(&svg).unwrap();
        let line = document
            .descendants()
            .find(|node| node.has_tag_name("line") && node.attribute("marker-end").is_some())
            .unwrap();
        let midpoint = (number(line, "x1").unwrap() + number(line, "x2").unwrap()) / 2.;
        let labels = document
            .descendants()
            .filter(|node| node.attribute("data-gpui-centered") == Some("true"))
            .collect::<Vec<_>>();
        assert_eq!(labels.len(), 3);
        for label in labels {
            assert!(
                (number(label, "x").unwrap() - midpoint).abs() < 0.01,
                "expected {midpoint}, got {:?}; SVG: {svg}",
                number(label, "x")
            );
        }
    }

    #[test]
    fn fallback_labels_do_not_clip_system_glyphs_or_change_other_viewports() {
        for source in [
            "kanban\n  todo[In Progress]\n    spec[Research and write the specification]",
            "mindmap\n  root((Root))\n    Research\n    Development",
        ] {
            let svg = render_svg(
                source,
                diagram_config("sans-serif", json!({})),
                OperationControl::new(),
            )
            .unwrap();
            let document = roxmltree::Document::parse(&svg).unwrap();
            let labels = document
                .descendants()
                .filter(|node| {
                    node.attribute("class") == Some("merman-foreignobject-fallback-text")
                })
                .collect::<Vec<_>>();
            assert!(!labels.is_empty());
            for label in labels {
                assert!(
                    label
                        .parent()
                        .unwrap()
                        .attribute("style")
                        .unwrap()
                        .contains("overflow:visible;")
                );
            }
            assert_eq!(prepare_svg(svg.clone()).unwrap(), svg);
        }
        let unrelated =
            r#"<svg><svg style="overflow:hidden;"><text>Explicit clip</text></svg></svg>"#;
        assert_eq!(prepare_svg(unrelated.into()).unwrap(), unrelated);
    }

    #[test]
    fn usecase_labels_follow_node_centers_and_info_has_a_canvas() {
        let svg = render_svg("usecase-beta\nactor Customer(\"Customer\")\nCheckout(\"Place order\")\nCustomer --> Checkout", diagram_config("sans-serif", json!({})), OperationControl::new()).unwrap();
        let document = roxmltree::Document::parse(&svg).unwrap();
        let mut labels = 0;
        for node in document.descendants().filter(|node| {
            matches!(
                node.attribute("data-usecase-kind"),
                Some("actor" | "usecase")
            )
        }) {
            for label in node
                .children()
                .filter(|child| child.children().any(|text| text.has_tag_name("text")))
            {
                assert_eq!(
                    translation(label.attribute("transform").unwrap())
                        .unwrap()
                        .0,
                    0.
                );
                labels += 1;
            }
        }
        assert_eq!(labels, 2);
        assert_eq!(prepare_svg(svg.clone()).unwrap(), svg);
        let info = render_svg(
            "info",
            diagram_config("sans-serif", json!({})),
            OperationControl::new(),
        )
        .unwrap();
        assert!(svg_dimensions(&info).is_ok());
        assert_eq!(prepare_svg(info.clone()).unwrap(), info);
    }
}

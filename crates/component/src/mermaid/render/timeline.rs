//! Correct Merman 0.9.2's vertical section advance after multi-event tasks.

use anyhow::{Result, ensure};

use super::svg::translation;

pub(super) fn repair_vertical_svg(mut svg: String) -> Result<String> {
    let document = roxmltree::Document::parse(&svg)?;
    let root = document.root_element();
    let mut edits = Vec::new();
    let mut bottom = f64::NEG_INFINITY;
    let mut shift = 0.;
    let mut axis = None;
    for wrapper in root.children().filter(|node| node.has_tag_name("g")) {
        let class = wrapper.attribute("class").unwrap_or_default();
        if let Some(transform) = wrapper.attribute_node("transform")
            && let Some((x, y)) = translation(transform.value())
            && let Some(height) = node_height(wrapper)
        {
            // Unclassified wrappers are section headers. A task and its events
            // share a row, so only advance at a section boundary, not per node.
            if class.is_empty() {
                shift += (bottom + 20. - (y + shift)).max(0.);
            }
            bottom = bottom.max(y + shift + height);
            if shift > 0. {
                edits.push((
                    transform.range_value(),
                    format!("translate({x},{})", y + shift),
                ));
            }
        } else if class == "lineWrapper" {
            for line in wrapper.children().filter(|node| node.has_tag_name("line")) {
                if line.attribute("x1") == line.attribute("x2") {
                    axis = Some(line);
                } else if shift > 0. {
                    for name in ["y1", "y2"] {
                        if let Some(value) = line.attribute_node(name) {
                            let y = value.value().parse::<f64>()?;
                            edits.push((value.range_value(), (y + shift).to_string()));
                        }
                    }
                }
            }
        }
    }
    if shift > 0. {
        if let Some(y2) = axis.and_then(|line| line.attribute_node("y2")) {
            edits.push((
                y2.range_value(),
                (y2.value().parse::<f64>()? + shift).to_string(),
            ));
        }
        let view_box = root
            .attribute_node("viewBox")
            .ok_or_else(|| anyhow::anyhow!("Timeline canvas missing"))?;
        let mut values = view_box
            .value()
            .split_whitespace()
            .map(str::parse::<f64>)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ensure!(values.len() == 4, "Invalid Timeline canvas");
        values[3] += shift;
        edits.push((
            view_box.range_value(),
            values
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" "),
        ));
    }
    edits.sort_unstable_by_key(|(range, _)| range.start);
    for (range, value) in edits.into_iter().rev() {
        svg.replace_range(range, &value);
    }
    Ok(svg)
}

fn node_height(wrapper: roxmltree::Node) -> Option<f64> {
    // Each Timeline node's bottom divider carries the complete laid-out height,
    // including wrapped text. Connector lines are outside these wrappers.
    wrapper
        .descendants()
        .filter(|node| node.has_tag_name("line"))
        .filter_map(|line| line.attribute("y1")?.parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .max_by(f64::total_cmp)
}

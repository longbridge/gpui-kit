use std::{sync::Arc, time::Duration};

use anyhow::{Result, bail, ensure};
use gpui::{
    App, DevicePixels, ParsedSvg, Pixels, RenderImage, Size, SvgRenderer, SvgSize, Window, size,
};
use merman::{
    Engine, MermaidConfig, OperationControl, RenderOutput, RenderRequest, Renderer, SvgEnvironment,
    SvgRequest,
    svg::{
        HostTheme, HostThemeAppearance, Presentation, RenderResourcePolicy, SvgPipeline, ThemeRole,
    },
};
use serde_json::{Value, json};

use crate::{ActiveTheme as _, Colorize as _};

const MAX_SOURCE_BYTES: usize = 128 * 1024;
const MAX_SVG_BYTES: usize = 8 * 1024 * 1024;
// Device pixels: bound allocation independently of the source's declared canvas size.
const MAX_EDGE: f64 = 4096.;
const MAX_PIXELS: f64 = 4_194_304.;

mod compatibility;
mod svg;
mod timeline;

use compatibility::Compatibility;
use svg::prepare_svg_on_canvas;

#[derive(Clone, PartialEq)]
pub(super) struct Appearance {
    pub(super) config: RenderConfig,
    pub(super) viewport: Size<DevicePixels>,
}

impl Appearance {
    pub(super) fn new(viewport: Size<Pixels>, window: &Window, cx: &App) -> Self {
        Self {
            config: theme_config(cx.theme()),
            // Supersample the actual preview, rather than the entire window.
            // In particular, tall sequence diagrams are limited by preview height.
            viewport: viewport.map(|length| {
                DevicePixels((f32::from(length) * window.scale_factor() * 2.).ceil() as i32)
            }),
        }
    }
}

#[derive(Clone, PartialEq)]
pub(super) struct RenderConfig {
    site: Value,
    theme: Option<HostTheme>,
}

impl From<Value> for RenderConfig {
    fn from(site: Value) -> Self {
        Self { site, theme: None }
    }
}

fn theme_config(theme: &crate::Theme) -> RenderConfig {
    // GPUI's platform alias is not a CSS family. An unquoted leading dot
    // also invalidates the generated CSS declaration and selects SVG's serif default.
    let font_family = if theme.font_family == ".SystemUIFont" {
        "sans-serif".to_string()
    } else {
        format!(
            "\"{}\", sans-serif",
            theme.font_family.replace('\\', "\\\\").replace('"', "\\\"")
        )
    };
    // HostTheme maps semantic roles to family-specific settings too (for example
    // Packet and Tree View do not read the common Mermaid textColor variable).
    let mut host = HostTheme::new()
        .try_with_font_family(&font_family)
        .unwrap_or_default()
        .with_appearance(if theme.is_dark() {
            HostThemeAppearance::Dark
        } else {
            HostThemeAppearance::Light
        })
        .try_with_font_size(format!("{}px", f32::from(theme.font_size)))
        .expect("A numeric theme font size is a CSS value");
    for (role, color) in [
        (ThemeRole::Canvas, theme.background),
        (ThemeRole::Surface, theme.muted),
        (ThemeRole::SurfaceAlt, theme.accent),
        (ThemeRole::SurfaceMuted, theme.muted),
        (ThemeRole::Text, theme.foreground),
        (ThemeRole::SubtleText, theme.foreground),
        // Content dividers need more contrast than decorative UI borders.
        (ThemeRole::Border, theme.muted_foreground),
        (ThemeRole::Line, theme.muted_foreground),
    ] {
        host = host
            .try_with_role(role, color.to_hex())
            .expect("A hex color is a CSS value");
    }
    host = host
        .try_with_series_palette(
            [
                theme.chart_1,
                theme.chart_2,
                theme.chart_3,
                theme.chart_4,
                theme.chart_5,
            ]
            .map(|color| color.to_hex()),
        )
        .expect("Hex chart colors are CSS values");
    let mut config = diagram_config(&font_family, json!({}));
    // An explicit site palette takes precedence over HostTheme, even "base".
    config.site.as_object_mut().unwrap().remove("theme");
    config
        .site
        .as_object_mut()
        .unwrap()
        .remove("themeVariables");
    config.theme = Some(host);
    config
}

fn diagram_config(font_family: &str, theme_variables: Value) -> RenderConfig {
    json!({
        "theme": "base",
        "layout": "dagre",
        // Neo applies object-bounding-box gradients to zero-height divider paths,
        // which resvg cannot paint. Classic keeps solid compartment boundaries.
        "look": "classic",
        "securityLevel": "strict",
        // Native SVG text avoids clipping by HTML fallback viewports when the
        // system font differs from Merman's deterministic text measurements.
        "htmlLabels": false,
        "flowchart": { "htmlLabels": false },
        "fontFamily": font_family,
        "themeVariables": theme_variables,
    })
    .into()
}

pub(super) struct Diagram {
    pub(super) appearance: Appearance,
    pub(super) parsed: Arc<ParsedSvg>,
    pub(super) dimensions: (f64, f64),
    pub(super) image: Arc<RenderImage>,
}

pub(super) fn render(
    source: &str,
    appearance: &Appearance,
    control: OperationControl,
    svg_renderer: &SvgRenderer,
) -> Result<Diagram> {
    let svg = render_svg(source, appearance.config.clone(), control.clone())?;
    control.checkpoint()?;
    let dimensions = svg_dimensions(&svg)?;
    let parsed = Arc::new(svg_renderer.parse_svg(svg.as_bytes())?);
    control.checkpoint()?;
    let image = rasterize(&parsed, dimensions, appearance.viewport, svg_renderer)?;
    Ok(Diagram {
        appearance: appearance.clone(),
        parsed,
        dimensions,
        image,
    })
}

fn render_svg(source: &str, config: RenderConfig, control: OperationControl) -> Result<String> {
    ensure!(
        source.len() <= MAX_SOURCE_BYTES,
        "Mermaid source exceeds 128 KiB"
    );
    let resources = RenderResourcePolicy::constrained();
    let control = control.with_deadline(Duration::from_secs(10));
    let font_family = config.site["fontFamily"]
        .as_str()
        .unwrap_or("sans-serif")
        .to_owned();
    let canvas = config
        .theme
        .as_ref()
        .and_then(|theme| theme.role(ThemeRole::Canvas))
        .unwrap_or("#FFFFFF")
        .to_owned();
    let presentation = config
        .theme
        .map(|theme| Presentation::new().with_theme(theme).resolve());
    let engine = Engine::new().with_site_config(MermaidConfig::from_value(config.site));
    let engine = if let Some(presentation) = &presentation {
        presentation.materialize_engine(engine)
    } else {
        engine
    };
    let compatibility = Compatibility::new(source, &engine, &control)?;
    let renderer = Renderer::new()
        .with_engine(engine)
        .with_resource_policy(*resources.input_policy());
    let request = SvgRequest {
        environment: SvgEnvironment::deterministic().with_resource_policy(resources),
        pipeline: Some(SvgPipeline::resvg_safe()),
        presentation: presentation
            .map(|value| value.render_policy())
            .unwrap_or_default(),
        ..Default::default()
    };
    let output = renderer.render(RenderRequest::svg(&compatibility.source, control, request))?;
    let RenderOutput::Svg(Some(output)) = output else {
        bail!("Source does not contain a Mermaid diagram");
    };
    ensure!(
        output.svg().len() <= MAX_SVG_BYTES,
        "Mermaid SVG exceeds 8 MiB"
    );
    let svg = compatibility.finish(output.into_parts().0)?;
    ensure!(svg.len() <= MAX_SVG_BYTES, "Mermaid SVG exceeds 8 MiB");
    prepare_svg_on_canvas(svg, &canvas, &font_family)
}

fn svg_dimensions(svg: &str) -> Result<(f64, f64)> {
    let document = roxmltree::Document::parse(svg)?;
    // usvg can resolve image paths from the filesystem. A diagram is self-contained;
    // do not pass external resources (or unbounded embedded image data) to that resolver.
    ensure!(
        !document
            .descendants()
            .any(|node| matches!(node.tag_name().name(), "image" | "feImage")),
        "Image resources in Mermaid diagrams are not supported"
    );
    let view_box = document
        .root_element()
        .attribute("viewBox")
        .ok_or_else(|| anyhow::anyhow!("Mermaid SVG has no viewBox"))?;
    let values = view_box
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()?;
    ensure!(
        values.len() == 4
            && values.iter().all(|v| v.is_finite())
            && values[2] > 0.
            && values[3] > 0.,
        "Invalid Mermaid SVG dimensions"
    );
    Ok((values[2], values[3]))
}

fn raster_size((width, height): (f64, f64), viewport: Size<DevicePixels>) -> Size<DevicePixels> {
    let scale = (f64::from(viewport.width.0) / width)
        .min(f64::from(viewport.height.0) / height)
        .min(MAX_EDGE / width.max(height))
        .min((MAX_PIXELS / width / height).sqrt());
    size(
        DevicePixels((width * scale).floor().max(1.) as i32),
        DevicePixels((height * scale).floor().max(1.) as i32),
    )
}

pub(super) fn rasterize(
    parsed: &ParsedSvg,
    dimensions: (f64, f64),
    viewport: Size<DevicePixels>,
    renderer: &SvgRenderer,
) -> Result<Arc<RenderImage>> {
    Ok(renderer.render_parsed(
        parsed,
        SvgSize::ExactSize(raster_size(dimensions, viewport)),
    )?)
}

#[cfg(test)]
mod audit;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_theme_reaches_family_specific_svg_readers() {
        for dark in [false, true] {
            let colors = if dark {
                crate::ThemeColor::dark()
            } else {
                crate::ThemeColor::light()
            };
            let mut theme = crate::Theme::from(colors.as_ref());
            theme.mode = if dark {
                crate::ThemeMode::Dark
            } else {
                crate::ThemeMode::Light
            };
            let foreground = theme.foreground.to_hex();
            for source in [
                "classDiagram\nA --> B : displays",
                "packet-beta\n0-7: \"Header\"",
                "treeView-beta\n    \"root\"\n        \"child\"",
                "pie\n\"One\": 1\n\"Two\": 2",
            ] {
                let svg =
                    render_svg(source, theme_config(&theme), OperationControl::new()).unwrap();
                assert!(
                    svg.split_whitespace()
                        .collect::<String>()
                        .contains(&format!("fill:{foreground}")),
                    "Host foreground missing for {source}"
                );
                if source.starts_with("pie") {
                    assert!(svg.contains(&theme.chart_1.to_hex()));
                    assert!(svg.contains(&theme.chart_2.to_hex()));
                }
            }
        }
    }

    #[gpui::test]
    fn diagrams_are_resvg_compatible(cx: &mut gpui::TestAppContext) {
        let renderer = cx.update(|cx| cx.svg_renderer());
        for source in [
            "flowchart LR\nA[开始] --> B[Done]",
            "sequenceDiagram\nAlice->>Bob: Hello",
            "classDiagram\nAnimal <|-- Duck",
        ] {
            let svg = render_svg(
                source,
                diagram_config("sans-serif", json!({"fontFamily": "sans-serif"})),
                OperationControl::new(),
            )
            .unwrap();
            assert!(!svg.contains("<foreignObject"));
            let (width, height) = svg_dimensions(&svg).unwrap();
            assert!(width > 0. && height > 0.);
            let parsed = renderer.parse_svg(svg.as_bytes()).unwrap();
            let viewport = size(DevicePixels(1024), DevicePixels(1024));
            let image = rasterize(&parsed, (width, height), viewport, &renderer).unwrap();
            assert_eq!(image.size(0), raster_size((width, height), viewport));
        }
    }

    #[test]
    fn class_compartments_and_edge_label_geometry_survive_svg_conversion() {
        let source = "classDiagram\nclass View {\nrender()\n}\nclass State {\nupdate()\n}\nView --> State : displays";
        for font_size in [12, 16, 24] {
            let svg = render_svg(
                source,
                diagram_config(
                    "sans-serif",
                    json!({"fontFamily": "sans-serif", "fontSize": format!("{font_size}px")}),
                ),
                OperationControl::new(),
            )
            .unwrap();
            let document = roxmltree::Document::parse(&svg).unwrap();
            assert!(!svg.contains("<foreignObject"));
            // A gradient in objectBoundingBox space has no paint area on a
            // horizontal divider. Keep both compartments in each class visible.
            // The stylesheet may retain unused Neo rules; only matching node
            // ancestry activates its gradient selector.
            assert!(
                document
                    .descendants()
                    .filter(|node| node.attribute("class") == Some("node default"))
                    .all(|node| node.attribute("data-look") == Some("classic"))
            );
            assert_eq!(
                document
                    .descendants()
                    .filter(|node| node.attribute("class") == Some("divider"))
                    .count(),
                4
            );
            let label = document
                .descendants()
                .find(|node| {
                    node.has_tag_name("text")
                        && node
                            .descendants()
                            .any(|child| child.text() == Some("displays"))
                })
                .expect("The edge label must survive conversion to SVG text");
            let background = label
                .parent()
                .unwrap()
                .children()
                .find(|node| {
                    node.has_tag_name("rect") && node.attribute("class") == Some("background")
                })
                .expect("The edge label must have a background");
            let number = |node: roxmltree::Node<'_, '_>, attribute| {
                node.attribute(attribute).unwrap().parse::<f64>().unwrap()
            };
            assert_eq!(label.attribute("text-anchor"), Some("middle"));
            let row = label
                .children()
                .find(|node| node.has_tag_name("tspan"))
                .unwrap();
            assert!(
                (number(row, "x") - number(background, "x") - number(background, "width") / 2.)
                    .abs()
                    < 0.01
            );
        }
    }

    #[test]
    fn class_edge_label_correction_preserves_other_text_and_is_idempotent() {
        let svg = render_svg(
            "classDiagram\nclass View {\nrender()\n}\nView --> State : displays",
            diagram_config("sans-serif", json!({"fontFamily": "sans-serif"})),
            OperationControl::new(),
        )
        .unwrap();
        assert_eq!(
            prepare_svg_on_canvas(svg.clone(), "#FFFFFF", "sans-serif").unwrap(),
            svg
        );
        let document = roxmltree::Document::parse(&svg).unwrap();
        for row in document.descendants().filter(|node| {
            node.has_tag_name("tspan")
                && node.attribute("class") == Some("row text-outer-tspan")
                && node
                    .ancestors()
                    .any(|ancestor| ancestor.attribute("class") == Some("node default"))
        }) {
            assert_eq!(row.attribute("x"), Some("0"));
        }
        let other_diagram = r#"<svg class="flowchart"><text text-anchor="middle"><tspan x="0">Label</tspan></text></svg>"#;
        assert_eq!(
            prepare_svg_on_canvas(other_diagram.into(), "#FFFFFF", "sans-serif").unwrap(),
            other_diagram
        );
    }

    #[test]
    fn rejects_invalid_cancelled_and_oversized_sources() {
        for source in [
            "not a diagram".to_string(),
            "x".repeat(MAX_SOURCE_BYTES + 1),
        ] {
            assert!(render_svg(&source, json!({}).into(), OperationControl::new()).is_err());
        }
        let control = OperationControl::new();
        control.cancel();
        assert!(render_svg("flowchart LR\nA --> B", json!({}).into(), control).is_err());
    }

    #[test]
    fn bounds_raster_allocation_and_rejects_external_images() {
        let large = raster_size(
            (100_000., 100_000.),
            size(DevicePixels(100_000), DevicePixels(100_000)),
        );
        assert!(large.width.0 * large.height.0 <= MAX_PIXELS as i32);
        // A wide preview must not enlarge a sequence diagram past its height.
        let preview = size(DevicePixels(2560), DevicePixels(768));
        assert_eq!(
            raster_size((700., 400.), preview),
            size(DevicePixels(1344), DevicePixels(768))
        );
        assert!(svg_dimensions(r#"<svg viewBox="0 0 0 12"/>"#).is_err());
        assert!(
            svg_dimensions(r#"<svg viewBox="0 0 12 12"><image href="file.png"/></svg>"#).is_err()
        );
    }
}

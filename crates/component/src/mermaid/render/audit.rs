//! Manual visual audit using the same theme, SVG preparation and GPUI rasterizer
//! as the component. Point GPUI_MERMAID_FIXTURES at a directory of .mmd files and
//! GPUI_MERMAID_OUTPUT at an output directory, then run this ignored test.
//! Optional .labels sidecars list required visible labels, one per line. Missing
//! labels fail the audit while keeping the SVG and raster available for review.
//!
//! Component fixtures focus on rendering regressions; adapted upstream samples
//! retain the MIT license in ../fixtures/LICENSE-MERMAN. Use the Gallery fixture
//! directory to audit all supported families without duplicating its sources.

use super::*;
use std::{fs, path::PathBuf, time::Instant};

#[test]
#[ignore = "manual visual audit; requires fixture and output directories"]
fn render_audit_fixtures() -> Result<()> {
    let input = std::env::var("GPUI_MERMAID_FIXTURES")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/mermaid/fixtures"));
    let output = PathBuf::from(std::env::var("GPUI_MERMAID_OUTPUT")?);
    fs::create_dir_all(&output)?;
    let mut fixtures = fs::read_dir(input)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    fixtures.retain(|path| path.extension().is_some_and(|extension| extension == "mmd"));
    fixtures.sort();
    ensure!(!fixtures.is_empty(), "No Mermaid fixtures found");
    let renderer = SvgRenderer::new(Arc::new(()));
    let mut results = Vec::new();
    let mut failures = Vec::new();
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
        let config = theme_config(&theme);
        for path in &fixtures {
            let name = path.file_stem().unwrap().to_string_lossy();
            let id = format!("{name}-{}", if dark { "dark" } else { "light" });
            let source = fs::read_to_string(path)?;
            let start = Instant::now();
            let result = (|| -> Result<Value> {
                let svg = render_svg(&source, config.clone(), OperationControl::new())?;
                fs::write(output.join(format!("{id}.svg")), &svg)?;
                ensure!(
                    !svg.contains("<foreignObject"),
                    "HTML label survived SVG preparation"
                );
                let dimensions = svg_dimensions(&svg)?;
                let document = roxmltree::Document::parse(&svg)?;
                let visible_text = document
                    .descendants()
                    .filter(|node| {
                        node.is_text()
                            && node
                                .ancestors()
                                .any(|ancestor| ancestor.has_tag_name("text"))
                    })
                    .filter_map(|node| node.text())
                    .collect::<String>();
                let compact = |value: &str| {
                    value
                        .chars()
                        .filter(|ch| !ch.is_whitespace())
                        .collect::<String>()
                };
                let visible_text = compact(&visible_text);
                let labels_path = path.with_extension("labels");
                let missing_labels = if labels_path.exists() {
                    fs::read_to_string(labels_path)?
                        .lines()
                        .filter(|label| !label.trim().is_empty())
                        .filter(|label| !visible_text.contains(&compact(label)))
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                let parsed = renderer.parse_svg(svg.as_bytes())?;
                let image = rasterize(
                    &parsed,
                    dimensions,
                    size(DevicePixels(1600), DevicePixels(900)),
                    &renderer,
                )?;
                let size = image.size(0);
                let pixels = image
                    .as_bytes(0)
                    .ok_or_else(|| anyhow::anyhow!("Empty raster"))?;
                ensure!(
                    pixels.chunks_exact(4).any(|pixel| pixel[3] != 0),
                    "Blank raster"
                );
                // GPUI exposes straight-alpha BGRA bytes. Preserve these verbatim
                // so an external contact-sheet script can composite the theme background.
                fs::write(output.join(format!("{id}.bgra")), pixels)?;
                Ok(
                    json!({"id": id, "width": size.width.0, "height": size.height.0,
                    "background": theme.background.to_hex(), "elapsed_ms": start.elapsed().as_millis(),
                    "missing_labels": missing_labels,
                    "fallback_labels": svg.matches("merman-foreignobject-fallback-text").count()}),
                )
            })();
            match result {
                Ok(result) => {
                    println!("{result}");
                    if result["missing_labels"]
                        .as_array()
                        .is_some_and(|labels| !labels.is_empty())
                    {
                        failures.push(id.clone());
                    }
                    results.push(result);
                }
                Err(error) => {
                    let diagnostic = format!("{error:#}");
                    println!("{id}: {diagnostic}");
                    failures.push(id.clone());
                    results.push(json!({"id": id, "error": diagnostic}));
                }
            }
        }
    }
    fs::write(
        output.join("results.json"),
        serde_json::to_vec_pretty(&results)?,
    )?;
    ensure!(
        failures.is_empty(),
        "Failed fixtures: {}",
        failures.join(", ")
    );
    Ok(())
}

//! m2svg - Convert Mermaid diagrams to ASCII/Unicode art and SVG
//!
//! This library provides functionality to parse Mermaid diagram syntax and render
//! it as ASCII or Unicode box-drawing art, or as SVG.
//!
//! # Example
//!
//! ```rust
//! use m2svg::{render, render_to_svg};
//!
//! let ascii = render("graph LR\n  A --> B", false).unwrap();
//! println!("{}", ascii);
//!
//! let svg = render_to_svg("graph LR\n  A --> B").unwrap();
//! println!("{}", svg);
//! ```
//!
//! # Supported Diagram Types
//!
//! - Flowcharts (graph TD / flowchart LR)
//! - State diagrams (stateDiagram-v2)
//! - Sequence diagrams (sequenceDiagram)
//! - Class diagrams (classDiagram)
//! - ER diagrams (erDiagram)

pub mod ascii;
pub mod parser;
pub mod svg;
pub mod types;

pub use ascii::render_mermaid_ascii;
pub use parser::parse_mermaid;
pub use types::*;

/// Render a Mermaid diagram to ASCII/Unicode text.
///
/// # Arguments
/// * `input` - Mermaid diagram text
/// * `use_ascii` - If true, use plain ASCII (+,-,|,>). If false, use Unicode box-drawing (┌,─,│,►)
///
/// # Example
/// ```rust
/// let output = m2svg::render("graph LR\n  A --> B", false).unwrap();
/// ```
pub fn render(input: &str, use_ascii: bool) -> Result<String, String> {
    let opts = AsciiRenderOptions {
        use_ascii,
        ..Default::default()
    };
    render_mermaid_ascii(input, Some(opts))
}

/// Render a Mermaid diagram to SVG text.
///
/// # Arguments
/// * `input` - Mermaid diagram text
///
/// # Example
/// ```rust
/// let svg = m2svg::render_to_svg("graph LR\n  A --> B").unwrap();
/// ```
pub fn render_to_svg(input: &str) -> Result<String, String> {
    let parsed = parse_mermaid(input)?;
    let colors = svg::DiagramColors::from_theme(parsed.frontmatter.theme);
    let font = "Inter";
    let transparent = false;

    let svg_output = match parsed.diagram {
        DiagramType::Flowchart(graph) => {
            svg::render_mermaid_to_svg(&graph, &colors, font, transparent)
        }
        DiagramType::Sequence(diagram) => {
            svg::render_sequence_svg(&diagram, &colors, font, transparent)
        }
        DiagramType::Class(diagram) => svg::render_class_svg(&diagram, &colors, font, transparent),
        DiagramType::Er(diagram) => svg::render_er_svg(&diagram, &colors, font, transparent),
        DiagramType::GitGraph(graph) => {
            svg::render_gitgraph_svg(&graph, &colors, font, transparent)
        }
    };

    // If title is present, inject it into the SVG
    if let Some(ref title) = parsed.frontmatter.title {
        Ok(inject_svg_title(&svg_output, title, &colors))
    } else {
        Ok(svg_output)
    }
}

/// Inject a title `<text>` element into an SVG string, shifting content down.
fn inject_svg_title(svg: &str, title: &str, colors: &svg::DiagramColors) -> String {
    use svg::styles::estimate_text_width;

    let title_font_size = 16.0;
    let title_font_weight = 600;
    let title_height = 30.0; // Space reserved for title (font size + padding)
    let title_text_width = estimate_text_width(title, title_font_size, title_font_weight);

    // Parse existing viewBox to adjust dimensions
    if let (Some(vb_start), Some(vb_end)) = (svg.find("viewBox=\""), svg.find("\" width=\"")) {
        let vb_content_start = vb_start + "viewBox=\"".len();
        let vb_str = &svg[vb_content_start..vb_end];
        let parts: Vec<f64> = vb_str
            .split_whitespace()
            .filter_map(|s| s.parse::<f64>().ok())
            .collect();

        if parts.len() == 4 {
            let (vb_x, vb_y, vb_w, vb_h) = (parts[0], parts[1], parts[2], parts[3]);
            let new_w = vb_w.max(title_text_width + 40.0);
            let new_h = vb_h + title_height;
            let title_x = new_w / 2.0;

            // Build the new viewBox and dimensions
            let format_dim = |d: f64| -> String {
                if d.fract() == 0.0 {
                    format!("{}", d as i64)
                } else {
                    let s = format!("{}", d);
                    s.trim_end_matches('0').trim_end_matches('.').to_string()
                }
            };

            // Also update width="..." height="..."
            let old_dims_start = vb_end + "\" width=\"".len();
            if let Some(w_end) = svg[old_dims_start..].find('"') {
                let h_start_search = old_dims_start + w_end + "\" height=\"".len();
                if let Some(h_end) = svg[h_start_search..].find('"') {
                    let old_section = &svg[vb_start..h_start_search + h_end + 1];
                    let new_section = format!(
                        "viewBox=\"{} {} {} {}\" width=\"{}\" height=\"{}\"",
                        format_dim(vb_x),
                        format_dim(vb_y),
                        format_dim(new_w),
                        format_dim(new_h),
                        format_dim(new_w),
                        format_dim(new_h)
                    );

                    // Build title text element
                    let title_elem = format!(
                        r#"<text x="{}" y="{}" text-anchor="middle" font-size="{}" font-weight="{}" fill="{}">{}</text>"#,
                        format_dim(title_x),
                        format_dim(title_height - 8.0),
                        format_dim(title_font_size),
                        title_font_weight,
                        colors.fg,
                        html_escape(title)
                    );

                    // Wrap existing content in a <g> with translate to shift it down
                    let close_tag = "</svg>";
                    if let Some(close_pos) = svg.rfind(close_tag) {
                        // Find end of opening <svg ...> tag
                        if let Some(svg_tag_end) = svg.find('>') {
                            let before_content = &svg[..svg_tag_end + 1];
                            let content = &svg[svg_tag_end + 1..close_pos];
                            let new_before = before_content.replace(old_section, &new_section);

                            return format!(
                                "{}\n{}\n<g transform=\"translate(0,{})\">{}</g>\n{}",
                                new_before,
                                title_elem,
                                format_dim(title_height),
                                content,
                                close_tag
                            );
                        }
                    }
                }
            }
        }
    }

    // Fallback: return SVG unchanged if parsing fails
    svg.to_string()
}

/// Escape special HTML characters in text content
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Configuration options for ASCII rendering
#[derive(Debug, Clone)]
pub struct AsciiRenderOptions {
    /// true = ASCII chars (+,-,|,>), false = Unicode box-drawing (┌,─,│,►). Default: true
    pub use_ascii: bool,
    /// Horizontal spacing between nodes. Default: 5
    pub padding_x: usize,
    /// Vertical spacing between nodes. Default: 5
    pub padding_y: usize,
    /// Padding inside node boxes. Default: 1
    pub box_border_padding: usize,
}

impl Default for AsciiRenderOptions {
    fn default() -> Self {
        Self {
            use_ascii: true,
            padding_x: 5,
            padding_y: 5,
            box_border_padding: 1,
        }
    }
}

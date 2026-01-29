//! Font metrics and styling constants.
//!
//! Calibrated for Inter font with fallback to system UI fonts.

/// Average character width in px at the given font size and weight
pub fn estimate_text_width(text: &str, font_size: f64, font_weight: u32) -> f64 {
    // Inter average character widths as fraction of fontSize, per weight.
    // Heavier weights are slightly wider.
    let width_ratio = if font_weight >= 600 {
        0.58
    } else if font_weight >= 500 {
        0.55
    } else {
        0.52
    };
    text.len() as f64 * font_size * width_ratio
}

/// Fixed font sizes used in the renderer (in px)
pub struct FontSizes;

impl FontSizes {
    pub const NODE_LABEL: f64 = 13.0;
    pub const EDGE_LABEL: f64 = 11.0;
    pub const GROUP_HEADER: f64 = 12.0;
}

/// Font weights used per element type
pub struct FontWeights;

impl FontWeights {
    pub const NODE_LABEL: u32 = 500;
    pub const EDGE_LABEL: u32 = 400;
    pub const GROUP_HEADER: u32 = 600;
}

/// Stroke widths per element type (in px)
pub struct StrokeWidths;

impl StrokeWidths {
    pub const OUTER_BOX: f64 = 1.0;
    pub const INNER_BOX: f64 = 0.75;
    pub const CONNECTOR: f64 = 0.75;
}

/// Arrow head dimensions
pub struct ArrowHead;

impl ArrowHead {
    pub const WIDTH: f64 = 8.0;
    pub const HEIGHT: f64 = 4.8;
}

/// Vertical shift applied to all text elements for font-agnostic centering.
/// Using 0.35em ensures it scales with font size.
pub const TEXT_BASELINE_SHIFT: &str = "0.35em";

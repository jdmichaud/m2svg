//! Theme system - CSS custom property-based theming for SVG diagrams.
//!
//! Architecture:
//!   - Two required variables: --bg (background) and --fg (foreground)
//!   - Optional enrichment variables: --line, --accent, --muted, --surface, --border
//!   - Unset optionals fall back to color-mix() derivations from bg + fg

use crate::types::MermaidTheme;
use serde::{Deserialize, Serialize};

/// Diagram color configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramColors {
    /// Background color → CSS variable --bg
    pub bg: String,
    /// Foreground / primary text color → CSS variable --fg
    pub fg: String,
    /// Edge/connector color → CSS variable --line (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    /// Arrow heads, highlights → CSS variable --accent (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    /// Secondary text, edge labels → CSS variable --muted (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub muted: Option<String>,
    /// Node/box fill tint → CSS variable --surface (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<String>,
    /// Node/group stroke color → CSS variable --border (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<String>,
}

impl Default for DiagramColors {
    fn default() -> Self {
        Self::from_theme(MermaidTheme::Default)
    }
}

impl DiagramColors {
    /// Create a `DiagramColors` from a Mermaid theme.
    ///
    /// Color values are derived from Mermaid's official theme files:
    /// - `default`: <https://github.com/mermaid-js/mermaid/blob/develop/packages/mermaid/src/themes/theme-default.js>
    /// - `dark`: <https://github.com/mermaid-js/mermaid/blob/develop/packages/mermaid/src/themes/theme-dark.js>
    pub fn from_theme(theme: MermaidTheme) -> Self {
        match theme {
            MermaidTheme::Default => Self {
                // Mermaid default: background = 'white', textColor = '#333',
                // lineColor = '#333333', mainBkg = '#ECECFF', border1 = '#9370DB'
                bg: "#FFFFFF".to_string(),
                fg: "#333333".to_string(),
                line: Some("#333333".to_string()),
                accent: Some("#333333".to_string()),
                muted: Some("#666666".to_string()),
                surface: Some("#ECECFF".to_string()),
                border: Some("#9370DB".to_string()),
            },
            MermaidTheme::Dark => Self {
                // Mermaid dark: background = '#333', textColor = '#ccc',
                // lineColor = 'lightgrey', mainBkg = '#1f2020', border1 = '#ccc'
                bg: "#333333".to_string(),
                fg: "#CCCCCC".to_string(),
                line: Some("#AAAAAA".to_string()),
                accent: Some("#CCCCCC".to_string()),
                muted: Some("#888888".to_string()),
                surface: Some("#1F2020".to_string()),
                border: Some("#CCCCCC".to_string()),
            },
        }
    }
}

/// color-mix() weights for derived CSS variables
pub struct Mix;

impl Mix {
    pub const TEXT_SEC: u8 = 60;
    pub const TEXT_MUTED: u8 = 40;
    pub const TEXT_FAINT: u8 = 25;
    pub const LINE: u8 = 30;
    pub const ARROW: u8 = 50;
    pub const NODE_FILL: u8 = 3;
    pub const NODE_STROKE: u8 = 20;
    pub const GROUP_HEADER: u8 = 5;
    pub const INNER_STROKE: u8 = 12;
    pub const KEY_BADGE: u8 = 10;
}

/// Build the <style> block with font imports and derived CSS variables.
pub fn build_style_block(font: &str) -> String {
    let font_encoded = font.replace(' ', "%20");

    let derived_vars = format!(
        r#"
    /* Derived from --bg and --fg (overridable via --line, --accent, etc.) */
    --_text:          var(--fg);
    --_text-sec:      var(--muted, color-mix(in srgb, var(--fg) {}%, var(--bg)));
    --_text-muted:    var(--muted, color-mix(in srgb, var(--fg) {}%, var(--bg)));
    --_text-faint:    color-mix(in srgb, var(--fg) {}%, var(--bg));
    --_line:          var(--line, color-mix(in srgb, var(--fg) {}%, var(--bg)));
    --_arrow:         var(--accent, color-mix(in srgb, var(--fg) {}%, var(--bg)));
    --_node-fill:     var(--surface, color-mix(in srgb, var(--fg) {}%, var(--bg)));
    --_node-stroke:   var(--border, color-mix(in srgb, var(--fg) {}%, var(--bg)));
    --_group-fill:    var(--bg);
    --_group-hdr:     color-mix(in srgb, var(--fg) {}%, var(--bg));
    --_inner-stroke:  color-mix(in srgb, var(--fg) {}%, var(--bg));
    --_key-badge:     color-mix(in srgb, var(--fg) {}%, var(--bg));"#,
        Mix::TEXT_SEC,
        Mix::TEXT_MUTED,
        Mix::TEXT_FAINT,
        Mix::LINE,
        Mix::ARROW,
        Mix::NODE_FILL,
        Mix::NODE_STROKE,
        Mix::GROUP_HEADER,
        Mix::INNER_STROKE,
        Mix::KEY_BADGE,
    );

    format!(
        r#"<style>
  @import url('https://fonts.googleapis.com/css2?family={}:wght@400;500;600;700&amp;display=swap');
  text {{ font-family: '{}', system-ui, sans-serif; }}
  svg {{{}
  }}
</style>"#,
        font_encoded, font, derived_vars
    )
}

/// Build the SVG opening tag with CSS variables set as inline styles.
pub fn svg_open_tag(width: f64, height: f64, colors: &DiagramColors, transparent: bool) -> String {
    let mut vars = vec![format!("--bg:{}", colors.bg), format!("--fg:{}", colors.fg)];

    if let Some(ref line) = colors.line {
        vars.push(format!("--line:{}", line));
    }
    if let Some(ref accent) = colors.accent {
        vars.push(format!("--accent:{}", accent));
    }
    if let Some(ref muted) = colors.muted {
        vars.push(format!("--muted:{}", muted));
    }
    if let Some(ref surface) = colors.surface {
        vars.push(format!("--surface:{}", surface));
    }
    if let Some(ref border) = colors.border {
        vars.push(format!("--border:{}", border));
    }

    let vars_str = vars.join(";");
    let bg_style = if transparent {
        ""
    } else {
        ";background:var(--bg)"
    };

    // Format dimensions - use integer if whole number, otherwise preserve decimals
    let format_dim = |d: f64| -> String {
        if d.fract() == 0.0 {
            format!("{}", d as i64)
        } else {
            // Remove trailing zeros after decimal
            let s = format!("{}", d);
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        }
    };

    let w_str = format_dim(width);
    let h_str = format_dim(height);

    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="{}" height="{}" style="{}{}">"#,
        w_str, h_str, w_str, h_str, vars_str, bg_style
    )
}

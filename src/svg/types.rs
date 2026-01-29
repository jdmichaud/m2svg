//! SVG-specific types for positioned graphs ready for rendering.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Positioned graph - after layout, ready for SVG rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionedGraph {
    pub width: f64,
    pub height: f64,
    pub nodes: Vec<PositionedNode>,
    pub edges: Vec<PositionedEdge>,
    pub groups: Vec<PositionedGroup>,
}

/// A positioned node with computed coordinates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionedNode {
    pub id: String,
    pub label: String,
    pub shape: NodeShape,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// Inline styles from classDef + explicit style statements
    #[serde(default, rename = "inlineStyle")]
    pub inline_style: Option<HashMap<String, String>>,
}

/// Node shape variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NodeShape {
    Rectangle,
    Rounded,
    Diamond,
    Stadium,
    Circle,
    Subroutine,
    Doublecircle,
    Hexagon,
    Cylinder,
    Asymmetric,
    Trapezoid,
    TrapezoidAlt,
    StateStart,
    StateEnd,
}

/// A positioned edge with path points
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionedEdge {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub label: Option<String>,
    pub style: EdgeStyle,
    #[serde(rename = "hasArrowStart")]
    pub has_arrow_start: bool,
    #[serde(rename = "hasArrowEnd")]
    pub has_arrow_end: bool,
    /// Path points including bends
    pub points: Vec<Point>,
    /// Layout-computed label center position
    #[serde(default, rename = "labelPosition")]
    pub label_position: Option<Point>,
}

/// Edge style variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EdgeStyle {
    Solid,
    Dotted,
    Thick,
}

/// A 2D point
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// A positioned group (subgraph)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionedGroup {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
    #[serde(default)]
    pub width: Option<f64>,
    #[serde(default)]
    pub height: Option<f64>,
    #[serde(default)]
    pub children: Vec<PositionedGroup>,
}

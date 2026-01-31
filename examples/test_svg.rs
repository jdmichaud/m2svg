use m2svg::svg::{render_svg, PositionedGraph, DiagramColors};

fn main() {
    let json = std::fs::read_to_string("../src/__tests__/testdata/positioned/subgraph_empty.json").unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    let positioned: PositionedGraph = serde_json::from_value(v["positioned"].clone()).unwrap();
    let colors = DiagramColors::default();
    let svg = render_svg(&positioned, &colors, "Inter", false);
    println!("{}", svg);
}

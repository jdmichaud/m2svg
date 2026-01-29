# Design Document

This document describes the high-level architecture of `m2svg`.

## Overview

```
                    ┌─────────────────────────────────────────────┐
                    │               Input: Mermaid Text           │
                    └─────────────────────────────────────────────┘
                                         │
                                         ▼
                    ┌─────────────────────────────────────────────┐
                    │                  Parser                     │
                    │  (flowchart, sequence, class, er)           │
                    └─────────────────────────────────────────────┘
                                         │
                                         ▼
                    ┌─────────────────────────────────────────────┐
                    │               Diagram Types                  │
                    │  MermaidGraph | SequenceDiagram | ...       │
                    └─────────────────────────────────────────────┘
                                         │
                         ┌───────────────┴───────────────┐
                         ▼                               ▼
          ┌──────────────────────────┐    ┌──────────────────────────┐
          │      ASCII Renderer      │    │       SVG Renderer       │
          │  Grid Layout → Canvas    │    │  Grid Layout → SVG Text  │
          └──────────────────────────┘    └──────────────────────────┘
                         │                               │
                         ▼                               ▼
          ┌──────────────────────────┐    ┌──────────────────────────┐
          │    Output: ASCII Text    │    │     Output: SVG Text     │
          └──────────────────────────┘    └──────────────────────────┘
```

## Module Structure

### `parser/`

Parses Mermaid diagram text into structured data types.

| Module        | Purpose                                    |
|---------------|-------------------------------------------|
| `flowchart.rs` | Parses `graph` and `flowchart` diagrams  |
| `sequence.rs`  | Parses `sequenceDiagram` blocks          |
| `class.rs`     | Parses `classDiagram` blocks             |
| `er.rs`        | Parses `erDiagram` blocks                |

Each parser returns a diagram-specific type (e.g., `MermaidGraph`, `SequenceDiagram`).

### `types.rs`

Defines the data structures for all diagram types:

- **MermaidGraph**: Nodes, edges, subgraphs for flowcharts
- **SequenceDiagram**: Participants, messages, activations
- **ClassDiagram**: Classes, members, relationships
- **ErDiagram**: Entities, attributes, relationships

### `ascii/`

Renders diagrams as text using box-drawing characters.

| Module            | Purpose                                          |
|-------------------|--------------------------------------------------|
| `types.rs`        | Grid coordinates, canvas, internal graph types   |
| `grid.rs`         | Assigns grid positions to nodes                  |
| `canvas.rs`       | 2D character array operations                    |
| `draw.rs`         | Draws nodes, edges, boxes on canvas              |
| `edge_routing.rs` | Pathfinding for edges between nodes              |
| `flowchart.rs`    | Flowchart-specific layout and rendering          |
| `sequence.rs`     | Sequence diagram rendering                       |
| `class_diagram.rs`| Class diagram rendering                          |
| `er_diagram.rs`   | ER diagram rendering                             |

### `svg/`

Renders diagrams as SVG text.

| Module         | Purpose                                          |
|----------------|--------------------------------------------------|
| `types.rs`     | PositionedGraph, PositionedNode, etc.            |
| `theme.rs`     | CSS variable system, color theming               |
| `styles.rs`    | Font sizes, stroke widths, arrow dimensions      |
| `renderer.rs`  | Converts PositionedGraph → SVG string            |
| `from_ascii.rs`| Uses ASCII layout algorithm → SVG output         |

## Layout Algorithm

The layout algorithm is shared between ASCII and SVG renderers:

1. **Grid Assignment**: Each node gets a `(gridX, gridY)` position
   - First node at (0, 0)
   - Connected nodes placed based on graph direction (LR vs TD)
   - Subgraphs expand to contain their children

2. **Drawing Coordinates**: Grid positions → character/pixel positions
   - For ASCII: gridX × (nodeWidth + padding)
   - For SVG: gridX × cellWidth

3. **Edge Routing**: A* pathfinding between node connection points
   - Avoids crossing through nodes
   - Prefers straight lines, then orthogonal bends

4. **Rendering**: Output the positioned elements
   - ASCII: Write characters to a 2D canvas
   - SVG: Generate `<rect>`, `<line>`, `<text>` elements

## Test Structure

Tests are organized in `tests/integration_tests.rs` and use fixture files.

### Test Data

Test fixtures live in `testdata/`:

```
testdata/
├── ascii/          # ASCII rendering tests (*.txt)
├── unicode/        # Unicode rendering tests (*.txt)
└── positioned/     # SVG rendering tests (*.json)
```

### ASCII/Unicode Test Format

Each `.txt` file contains:
```
<mermaid input>
---
<expected ASCII output>
```

The test parses the input, renders it, and compares to the expected output.

### SVG Test Format

Each `.json` file contains:
```json
{
  "input": "<mermaid input>",
  "type": "flowchart|sequence|class|er",
  "positioned": { /* pre-computed node positions */ },
  "svg": "<expected SVG output>"
}
```

The SVG tests verify that given positioned data produces the exact expected SVG string.

### Test Categories

| Category       | Count | What it tests                              |
|----------------|-------|-------------------------------------------|
| ASCII          | 40    | Flowchart rendering with Unicode chars    |
| Unicode        | 37    | Same diagrams, run via `test_all_testdata_files` |
| SVG            | 44    | SVG output from positioned flowcharts     |
| SVG (skipped)  | 13    | Non-flowchart diagrams (sequence, class, ER) |

### Running Tests

```bash
# Standard tests (ASCII flowcharts)
cargo test

# All testdata files (includes Unicode)
cargo test test_all_testdata_files -- --ignored

# SVG renderer tests
cargo test test_svg_renderer -- --ignored
```

## Key Design Decisions

### 1. Grid-Based Layout

Using a logical grid simplifies positioning:
- Nodes occupy discrete cells
- Easy to reason about spacing
- Same algorithm works for ASCII and SVG

### 2. Two SVG Rendering Paths

- **`render_svg(PositionedGraph)`**: Takes pre-computed positions, produces SVG
- **`render_mermaid_to_svg(MermaidGraph)`**: Uses ASCII layout, outputs SVG

The first is useful for testing against fixtures. The second is the simple end-to-end path.

### 3. CSS Variables for Theming

SVG output uses CSS variables (`--bg`, `--fg`, etc.) so users can theme diagrams by setting CSS properties, without modifying the SVG.

### 4. No External Layout Engine

Unlike the TypeScript version (which uses dagre), this implementation uses a simple grid-based layout. This keeps dependencies minimal and output predictable.

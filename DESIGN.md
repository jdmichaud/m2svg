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
                    │             Frontmatter Parser              │
                    │  (theme, diagram-specific config)           │
                    └─────────────────────────────────────────────┘
                                         │
                                         ▼
                    ┌─────────────────────────────────────────────┐
                    │             Diagram Parser                  │
                    │  (flowchart, sequence, class, er, gitgraph) │
                    └─────────────────────────────────────────────┘
                                         │
                                         ▼
                    ┌─────────────────────────────────────────────┐
                    │               Diagram Types                 │
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

Parses Mermaid diagram text into structured data types. The entry point
`parse_mermaid()` first extracts YAML frontmatter (theme, config), then
detects the diagram type and delegates to the appropriate parser. Returns
a `ParsedDiagram` containing the `DiagramType` and `FrontmatterConfig`.

| Module         | Purpose                                       |
|----------------|-----------------------------------------------|
| `mod.rs`       | `parse_mermaid()`, `parse_frontmatter()`, diagram type detection |
| `flowchart.rs` | Parses `graph` and `flowchart` diagrams (also handles `stateDiagram`) |
| `sequence.rs`  | Parses `sequenceDiagram` blocks               |
| `class.rs`     | Parses `classDiagram` blocks                  |
| `er.rs`        | Parses `erDiagram` blocks                     |
| `gitgraph.rs`  | Parses `gitGraph` blocks, extracts `GitGraphConfig` from frontmatter |

Each parser returns a diagram-specific type (e.g., `MermaidGraph`, `SequenceDiagram`).

### `types.rs`

Defines the data structures for all diagram types:

- **MermaidGraph**: Nodes, edges, subgraphs for flowcharts
- **SequenceDiagram**: Participants, messages, activations
- **ClassDiagram**: Classes, members, relationships
- **ErDiagram**: Entities, attributes, relationships
- **GitGraph**: Commits, branches, merges, tags
- **FrontmatterConfig**: Theme (`MermaidTheme`), raw YAML lines
- **ParsedDiagram**: Wrapper combining `DiagramType` + `FrontmatterConfig`

### `ascii/`

Renders diagrams as text using box-drawing characters (Unicode) or plain ASCII.

| Module             | Purpose                                          |
|--------------------|--------------------------------------------------|
| `mod.rs`           | `render_mermaid_ascii()` dispatch                |
| `types.rs`         | Grid coordinates, canvas, internal graph types   |
| `grid.rs`          | Assigns grid positions to nodes (Dagre-style)    |
| `canvas.rs`        | 2D character array operations                    |
| `draw.rs`          | Draws nodes, edges, boxes on canvas              |
| `pathfinder.rs`    | A* pathfinding for edge routing                  |
| `flowchart.rs`     | Flowchart-specific layout and rendering          |
| `sequence.rs`      | Sequence diagram rendering                       |
| `class_diagram.rs` | Class diagram rendering (with inheritance fan-out) |
| `er_diagram.rs`    | ER diagram rendering (unified renderer with attribute support) |
| `gitgraph.rs`      | Git graph rendering (commit history visualization) |

### `svg/`

Renders diagrams as SVG text. Supports theming via CSS variables.

| Module             | Purpose                                          |
|--------------------|--------------------------------------------------|
| `mod.rs`           | SVG render dispatch, public exports              |
| `types.rs`         | SVG-specific types                               |
| `theme.rs`         | `DiagramColors`, `from_theme()`, CSS variable system |
| `styles.rs`        | Shared SVG CSS styles                            |
| `renderer.rs`      | Core SVG rendering helpers                       |
| `from_ascii.rs`    | Flowchart SVG via ASCII-to-SVG conversion        |
| `class_diagram.rs` | Class diagram SVG renderer                       |
| `er_diagram.rs`    | ER diagram SVG renderer                          |
| `sequence.rs`      | Sequence diagram SVG renderer                    |
| `gitgraph.rs`      | Git graph SVG renderer                           |

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
All tests run with `cargo test` — no ignored or gated tests.

### Test Data

Test fixtures live in `testdata/`:

```
testdata/
├── ascii/          # ASCII rendering tests (*.txt)
│   ├── flowchart/
│   ├── class/
│   ├── er/
│   ├── sequence/
│   └── gitgraph/
├── unicode/        # Unicode rendering tests (*.txt, same subdirs)
└── svg/            # SVG rendering tests (*.mmd input + *.svg expected)
    ├── flowchart/
    ├── class/
    ├── er/
    ├── sequence/
    └── gitgraph/
```

### ASCII/Unicode Test Format

Each `.txt` file contains:
```
<mermaid input>
---
<expected ASCII output>
```

The test runner uses `rfind` to split on the **last** `---` separator, so
YAML frontmatter `---` delimiters in the input section work correctly.
Tests auto-detect ASCII vs Unicode mode based on expected output characters.

### SVG Test Format

SVG tests use paired files: `<name>.mmd` (input) and `<name>.svg` (expected output).
Lines starting with `#` in `.mmd` files are filtered out as comments.
SVG comparison is semantic (normalized XML tree comparison), not exact string match.

### Test Categories

| Category       | Count | What it tests                              |
|----------------|-------|-------------------------------------------|
| ASCII          | 76    | All diagram types with ASCII chars         |
| Unicode        | 53    | Same diagrams with Unicode box-drawing     |
| SVG            | 64    | SVG output for all diagram types           |
| Unit           | 2     | SVG from_ascii module                      |
| Doc            | 3     | Library doc examples                       |

### Running Tests

```bash
# Run all tests
cargo test

# Run specific category
cargo test ascii_
cargo test unicode_
cargo test svg_

# Run a specific test
cargo test test_cls_inheritance
```

## Key Design Decisions

### 1. Grid-Based Layout

Using a logical grid simplifies positioning:
- Nodes occupy discrete cells
- Easy to reason about spacing
- Same algorithm works for ASCII and SVG (flowcharts)

### 2. Frontmatter and Theming

YAML frontmatter is parsed once by `parse_frontmatter()` in `parser/mod.rs`
before diagram-specific parsing. This provides:
- **Theme selection**: `default` (light) and `dark` themes
- **Diagram-specific config**: e.g., `gitGraph.showBranches`, `mainBranchName`
- **CSS variable system**: `--bg`, `--fg`, `--line`, `--accent`, etc.

`DiagramColors::from_theme()` maps a `MermaidTheme` to concrete CSS color values
for SVG output.

### 3. Two SVG Rendering Approaches

- **ASCII-to-SVG** (`from_ascii.rs`): Flowcharts use the ASCII layout algorithm,
  then convert the grid positions to SVG coordinates
- **Direct SVG**: Sequence, class, ER, and gitgraph diagrams have dedicated SVG
  renderers that produce SVG directly from parsed data

### 4. CSS Variables for Theming

SVG output uses CSS variables (`--bg`, `--fg`, etc.) so users can theme diagrams
by setting CSS properties, without modifying the SVG. Optional variables fall back
to `color-mix()` derivations in CSS.

### 5. No External Layout Engine

Unlike the TypeScript version (which uses dagre), this implementation uses a
simple grid-based layout. This keeps dependencies minimal and output predictable.

### 6. Unified ER Rendering

All ER diagrams (simple, with attributes, multi-relationship) use a single
`render_general_er()` code path. Entity boxes support variable heights for
attribute rows, and relationship connectors use centered labels over extended
line portions with directional cardinality symbols.

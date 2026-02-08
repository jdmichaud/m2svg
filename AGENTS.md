# AGENTS.md

Guidelines for AI agents working on the m2svg codebase.

## Project Overview

m2svg is a Rust library and CLI tool that converts Mermaid diagram syntax to ASCII art or SVG. It supports:
- Flowcharts/graphs (TB, BT, LR, RL directions)
- State diagrams (rendered as flowcharts)
- Sequence diagrams
- Class diagrams (with inheritance fan-out)
- ER diagrams
- Git graphs (commit history visualization)

This is a Rust port of the TypeScript [beautiful-mermaid](https://github.com/lukilabs/beautiful-mermaid) project by Luki Labs. The original TypeScript implementation provides:
- A reference for expected rendering behavior
- Test fixtures that were ported to this project
- ASCII art algorithms for layout and drawing

When in doubt about expected output, consult the TypeScript reference implementation.

## Working with the TypeScript Reference

If you have the TypeScript repo cloned locally, here's how to use it:

```bash
# Install dependencies
npm install

# Run TypeScript tests
npm test

# Quick render test from command line
npx tsx -e "
import { render } from './src/index';
console.log(render(\`
graph TD
  A --> B
\`));
"

# Run the dev server with live preview
npm run dev
```

### TypeScript Source Structure
```
src/
├── index.ts            # Main exports: render(), renderSvg()
├── parser.ts           # Flowchart parser
├── layout.ts           # Dagre layout wrapper
├── renderer.ts         # Flowchart ASCII renderer
├── ascii/
│   ├── canvas.ts       # Character canvas (equivalent to Rust canvas.rs)
│   ├── class-diagram.ts
│   ├── er-diagram.ts
│   └── sequence.ts
├── class/              # Class diagram modules
├── er/                 # ER diagram modules
└── sequence/           # Sequence diagram modules
```

### Comparing Outputs
When fixing rendering issues, compare Rust output with TypeScript:

```bash
# TypeScript reference
cd /path/to/beautiful-mermaid
npx tsx -e "import { render } from './src'; console.log(render('graph TD; A-->B'));"

# Rust implementation
cd /path/to/m2svg
echo 'graph TD; A-->B' | cargo run
```

## Build Commands

```bash
# Debug build
cargo build

# Release build (statically linked, optimized)
cargo build --release --target x86_64-unknown-linux-musl

# Run tests
cargo test

# Check for errors without building
cargo check

# Format code
cargo fmt

# Lint
cargo clippy

# Regenerate test fixtures (after rendering changes)
./regen_fixtures.sh                          # All fixtures
./regen_fixtures.sh class                    # All class diagram fixtures
./regen_fixtures.sh class/cls_basic          # One specific fixture (ASCII + Unicode)
./regen_fixtures.sh --svg class              # SVG class fixtures only
./regen_fixtures.sh --svg class/class_basic  # One SVG fixture
```

## Project Structure

```
src/
├── lib.rs              # Main library exports: render(), render_to_svg()
├── main.rs             # CLI binary
├── types.rs            # Shared types: MermaidGraph, DiagramType, ParsedDiagram,
│                       #   FrontmatterConfig, MermaidTheme, GitGraphConfig, etc.
├── parser/             # Parsing modules
│   ├── mod.rs          # parse_mermaid(), parse_frontmatter(), diagram type detection
│   ├── flowchart.rs    # Flowchart & state diagram parser
│   ├── sequence.rs     # Sequence diagram parser
│   ├── class.rs        # Class diagram parser
│   ├── er.rs           # ER diagram parser
│   └── gitgraph.rs     # Git graph parser (delegates frontmatter to mod.rs)
├── ascii/              # ASCII rendering modules
│   ├── mod.rs          # render_mermaid_ascii() dispatch
│   ├── canvas.rs       # 2D character canvas utilities
│   ├── draw.rs         # Common drawing helpers
│   ├── types.rs        # ASCII-specific types
│   ├── grid.rs         # Grid-based layout algorithm (Dagre-style)
│   ├── flowchart.rs    # Flowchart ASCII renderer
│   ├── class_diagram.rs
│   ├── er_diagram.rs
│   ├── sequence.rs
│   ├── gitgraph.rs     # Git graph ASCII renderer
│   └── pathfinder.rs   # A* pathfinding for edge routing
└── svg/                # SVG rendering modules
    ├── mod.rs           # SVG render dispatch, public exports
    ├── from_ascii.rs    # Flowchart SVG via ASCII-to-SVG conversion
    ├── renderer.rs      # Core SVG rendering helpers
    ├── styles.rs        # Shared SVG CSS styles
    ├── theme.rs         # DiagramColors, from_theme(), CSS variable system
    ├── types.rs         # SVG-specific types
    ├── class_diagram.rs
    ├── er_diagram.rs
    ├── sequence.rs
    └── gitgraph.rs      # Git graph SVG renderer

tests/
└── integration_tests.rs  # Test runner for testdata fixtures

testdata/
├── ascii/              # ASCII test fixtures (.txt, input + expected separated by ---)
├── unicode/            # Unicode test fixtures (.txt, same format as ascii)
└── svg/                # SVG test fixtures (.mmd input + .svg expected output pairs)
```

## Key Conventions

### Code Style
- Use `snake_case` for functions and variables
- Prefix unused fields/variables with underscore (e.g., `_id`)
- Keep functions focused and under ~100 lines when possible
- Add doc comments for public APIs

### Error Handling
- Return `Result<T, String>` for fallible operations
- Use descriptive error messages

### Testing
- **ASCII/Unicode fixtures** (`testdata/ascii/`, `testdata/unicode/`): Single `.txt` file with input and expected output separated by `\n---\n`. The test parser uses `rfind` to split on the **last** `---` separator, so YAML frontmatter `---` delimiters in the input section won't cause issues.
- **SVG fixtures** (`testdata/svg/`): Paired `.mmd` (input) and `.svg` (expected output) files. Lines starting with `#` in `.mmd` files are filtered out as comments by the test runner. SVG tests use semantic comparison (not exact string match).
- ASCII/Unicode tests compare rendered text output exactly (trailing whitespace normalized)
- Tests auto-detect ASCII vs Unicode mode based on expected output characters
- Run specific test: `cargo test test_cls_inheritance`
- Run all SVG tests: `cargo test svg_`
- Regenerate an SVG fixture: `grep -v '^\s*#' testdata/svg/<subdir>/<name>.mmd | cargo run -- -s - > testdata/svg/<subdir>/<name>.svg`

### Diagram Rendering Pipeline
1. **Parse** - `parse_mermaid()` extracts frontmatter (theme, config), detects diagram type, delegates to type-specific parser. Returns `ParsedDiagram` containing `DiagramType` + `FrontmatterConfig`.
2. **Layout** - Assign positions to elements (layout modules)
3. **Render** - Draw to canvas (ASCII/Unicode) or generate SVG. For SVG, `DiagramColors::from_theme()` applies the theme from frontmatter.

## Common Tasks

### Adding a new test fixture
1. Create `testdata/ascii/my_test.txt` with format:
   ```
   graph TD
     A --> B
   ---
   +---+
   | A |
   +---+
     |
     v
   +---+
   | B |
   +---+
   ```
2. Add test macro in `tests/integration_tests.rs`:
   ```rust
   ascii_test!(flowchart, my_test);
   ```
   Note: the first argument is the subdirectory (`flowchart`, `class`, `er`, `sequence`, `gitgraph`).

### Adding a new SVG test fixture
1. Create `testdata/svg/<subdir>/<name>.mmd` with the Mermaid input:
   ```
   # Optional comment (will be filtered out)
   graph TD
     A --> B
   ```
2. Generate the expected SVG output:
   ```bash
   grep -v '^\s*#' testdata/svg/<subdir>/<name>.mmd | cargo run -- -s - > testdata/svg/<subdir>/<name>.svg
   ```
3. Add test macro in `tests/integration_tests.rs`:
   ```rust
   svg_test!(<subdir>, <name>);
   ```
   Note: the `svg_test!` macro takes two arguments — subdirectory and test name.

### Fixing rendering issues
1. Check the relevant renderer in `src/ascii/` or `src/svg/`
2. Labels are drawn in input order (later overwrites earlier for overlap handling)
3. After fixing, regenerate affected fixtures: `./regen_fixtures.sh <subdir>` or `./regen_fixtures.sh <subdir>/<name>`
4. Run `cargo test` to verify

### Adding SVG support for a diagram type
1. Create `src/svg/<diagram>.rs` with render function
2. Export from `src/svg/mod.rs`
3. Add dispatch case in `src/lib.rs` `render_to_svg()`

## Important Notes

- Static linking via musl target produces zero-dependency binaries
- LTO is enabled in release builds for size optimization
- Class diagram fan-out: when one parent has multiple children, they're grouped and rendered as a tree

## Lessons Learned

### ASCII vs Unicode Mode
The renderer supports two character sets:
- **ASCII mode**: `+`, `-`, `|`, `v`, `^`, `<`, `>`
- **Unicode mode**: `┌`, `─`, `│`, `▼`, `▲`, `◀`, `▶`, box-drawing characters

Tests auto-detect mode by checking if expected output contains Unicode box-drawing characters (e.g., `─`). The `use_unicode` flag propagates through all rendering functions.

### Label Overlap Strategy
When relationship labels overlap on the same line, they're drawn in **input order** with space padding:
```rust
let padded = format!(" {} ", label);
canvas.draw_text(x, y, &padded);
```
Later labels overwrite earlier ones. This matches TypeScript behavior and produces readable output for common cases.

### Canvas Coordinate System
The canvas uses `(x, y)` where:
- `x` = column (horizontal, 0 = left)
- `y` = row (vertical, 0 = top)

Drawing outside bounds is silently ignored (no panic). This simplifies edge cases like labels near diagram edges.

### Class Diagram Relationship Arrows
```
<|--  Inheritance (hollow triangle)
*--   Composition (filled diamond)
o--   Aggregation (hollow diamond)
-->   Association (arrow)
--    Link (no arrow)
..>   Dependency (dashed arrow)
..|>  Realization/Implementation (dashed + hollow triangle)
```

### ER Diagram Cardinality Notation
```
||--||  One to one
||--o{  One to many
o{--o{  Many to many
|o--o|  Zero-or-one to zero-or-one
```
The symbols: `|` = exactly one, `o` = zero, `{` or `}` = many

### Debugging Rendering Issues
1. **Create minimal repro**: Reduce diagram to smallest failing case
2. **Print canvas**: Add `println!("{}", canvas.to_string())` to see intermediate state
3. **Check coordinates**: Off-by-one errors are common in box drawing
4. **Compare with TypeScript**: Run same input through both implementations
5. **Check input order**: Rendering order affects overlap resolution

### Test Fixture Whitespace
Expected output in test fixtures often has significant trailing whitespace. When tests fail:
1. Check for trailing spaces on lines
2. Compare character-by-character if needed
3. The separator is exactly `\n---\n` (newline, three dashes, newline)

### Deterministic Output
SVG output must be deterministic for exact-match testing. Key considerations:
- **HashMap iteration**: Never iterate over HashMaps directly for output generation
- **Parser ordering**: Use Vec to track insertion order alongside HashMap lookups
- **Renderer ordering**: Sort by ID before iterating when generating SVG elements
- Example fix in `src/svg/class_diagram.rs`: Sort `class_boxes` by ID before drawing
- Example fix in `src/parser/er.rs`: Track `entity_order` Vec alongside `entity_map`

### Edge Routing
Edges use A* pathfinding to avoid nodes. The pathfinder in `src/ascii/pathfinder.rs` treats node bounding boxes as obstacles. Complex layouts may route edges around multiple nodes.

### YAML Frontmatter Configuration
All diagram types support Mermaid-compatible YAML frontmatter for configuration. Frontmatter is parsed once by a generalized parser in `src/parser/mod.rs` before diagram-specific parsing.

```
---
config:
  theme: dark
  gitGraph:
    showBranches: false
    showCommitLabel: false
    mainBranchName: trunk
---
gitGraph
   commit
   ...
```

Key implementation details:
- **Generalized parser**: `parse_frontmatter()` in `src/parser/mod.rs` extracts YAML between `---` delimiters, returning `(FrontmatterConfig, remaining_text)`. This is the single source of truth for all diagram types.
- **`FrontmatterConfig`** (in `src/types.rs`): Contains `theme: MermaidTheme` (common config) and `raw_lines: Vec<String>` (for diagram-specific parsers to inspect)
- **`ParsedDiagram`** (in `src/types.rs`): Wrapper returned by `parse_mermaid()`, containing both `diagram: DiagramType` and `frontmatter: FrontmatterConfig`
- **`extract_yaml_value()`**: Public helper in `parser/mod.rs` for case-insensitive YAML key lookup, reused by `parser/gitgraph.rs`
- **GitGraph-specific config**: `GitGraphConfig` in `src/types.rs` holds gitGraph-specific fields (`showBranches`, `showCommitLabel`, `mainBranchName`, `mainBranchOrder`). Parsed from `FrontmatterConfig::raw_lines` in `parser/gitgraph.rs`.
- **Structural options**: `showBranches`, `showCommitLabel`, `mainBranchName`, `mainBranchOrder` affect layout in both ASCII and SVG
- **Visual theming**: `git0`-`git7`, `commitLabelColor`, `tagLabelColor` etc. affect SVG colors only
- **`parallelCommits`**: Detected and warned via `eprintln!` but not implemented
- **Test fixtures with frontmatter**: Work correctly because `parse_test_file` uses `rfind` for the last `---` separator

### Theme System
SVG output supports Mermaid themes via the `theme` key in YAML frontmatter.

**Supported themes**: `default` (light) and `dark`

**Architecture** (in `src/svg/theme.rs`):
- `MermaidTheme` enum (`Default`, `Dark`) in `src/types.rs`
- `DiagramColors::from_theme(theme)` maps a theme to concrete CSS color values
- CSS variables: `--bg`, `--fg`, `--line`, `--accent`, `--muted`, `--surface`, `--border`
- Optional variables (`--line`, `--accent`, etc.) fall back to `color-mix()` derivations in CSS if not set

**Default theme colors**: white bg (`#FFFFFF`), dark text (`#333333`), lavender surface (`#ECECFF`), purple border (`#9370DB`)

**Dark theme colors**: dark bg (`#333333`), light text (`#CCCCCC`), near-black surface (`#1F2020`), light border (`#CCCCCC`)

**Flow**: `parse_mermaid()` → `ParsedDiagram` (includes `frontmatter.theme`) → `render_to_svg()` calls `DiagramColors::from_theme(parsed.frontmatter.theme)` → colors injected into SVG `<style>` block as CSS custom properties

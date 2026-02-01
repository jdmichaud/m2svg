# AGENTS.md

Guidelines for AI agents working on the m2svg codebase.

## Project Overview

m2svg is a Rust library and CLI tool that converts Mermaid diagram syntax to ASCII art or SVG. It supports:
- Flowcharts/graphs (TB, BT, LR, RL directions)
- Sequence diagrams
- Class diagrams (with inheritance fan-out)
- ER diagrams

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
```

## Project Structure

```
src/
├── lib.rs              # Main library exports, render() and render_to_svg()
├── main.rs             # CLI binary
├── types.rs            # Shared type definitions
├── parser/             # Parsing modules (flowchart, class, er, sequence)
├── ascii/              # ASCII rendering modules
│   ├── mod.rs
│   ├── canvas.rs       # 2D character canvas utilities
│   ├── grid.rs         # Grid-based layout algorithm (Dagre-style)
│   ├── flowchart.rs    # Flowchart ASCII renderer
│   ├── class_diagram.rs
│   ├── er_diagram.rs
│   ├── sequence.rs
│   └── pathfinder.rs   # A* pathfinding for edge routing
└── svg/                # SVG rendering modules
    ├── mod.rs
    ├── from_ascii.rs   # Convert ASCII to SVG
    ├── class_diagram.rs
    ├── er_diagram.rs
    └── sequence.rs

tests/
└── integration_tests.rs  # Test runner for testdata fixtures

testdata/
├── ascii/              # Test fixtures (input + expected output)
├── unicode/            # Unicode-specific test fixtures
└── svg/                # SVG test fixtures (input + expected SVG output)
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
- Each test fixture in `testdata/ascii/` and `testdata/svg/` contains input and expected output separated by `\n---\n`
- The test parser uses `rfind` to split on the **last** `---` separator, so YAML frontmatter `---` delimiters in the input section won't cause issues
- ASCII/Unicode tests compare rendered text output exactly
- SVG tests compare complete SVG output exactly (requires deterministic rendering)
- Tests auto-detect ASCII vs Unicode mode based on expected output characters
- Run specific test: `cargo test test_cls_inheritance`
- Run all SVG tests: `cargo test svg_`

### Diagram Rendering Pipeline
1. **Parse** - Convert text to AST (parser modules)
2. **Layout** - Assign positions to elements (layout modules)
3. **Render** - Draw to canvas or generate SVG (renderer/ascii/svg modules)

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
   ascii_test!(my_test);
   ```

### Adding a new SVG test fixture
1. Create `testdata/svg/my_svg_test.txt` with format:
   ```
   # Optional comment about the test
   graph TD
     A --> B
   ---
   <svg xmlns=...>...</svg>
   ```
2. Add test macro in `tests/integration_tests.rs`:
   ```rust
   svg_test!(my_svg_test);
   ```
3. To generate expected output, run:
   ```bash
   echo 'graph TD; A-->B' | cargo run -- -s -
   ```

### Fixing rendering issues
1. Check the relevant renderer in `src/ascii/` or `src/svg/`
2. Labels are drawn in input order (later overwrites earlier for overlap handling)
3. Use test fixtures to verify expected output

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
GitGraph diagrams support Mermaid-compatible YAML frontmatter for configuration:

```
---
config:
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
- **Parser**: `parse_frontmatter()` in `src/parser/gitgraph.rs` extracts YAML before diagram content
- **Config struct**: `GitGraphConfig` in `src/types.rs` holds all config fields with sensible defaults
- **Frontmatter stripping**: `strip_frontmatter()` in `src/parser/mod.rs` removes `---`-delimited blocks before diagram type detection
- **Structural options**: `showBranches`, `showCommitLabel`, `mainBranchName`, `mainBranchOrder` affect layout in both ASCII and SVG
- **Visual theming**: `git0`-`git7`, `commitLabelColor`, `tagLabelColor` etc. affect SVG colors only
- **`parallelCommits`**: Detected and warned via `eprintln!` but not implemented
- **Test fixtures with frontmatter**: Work correctly because `parse_test_file` uses `rfind` for the last `---` separator

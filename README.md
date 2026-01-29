# mermaid-ascii

A Rust library and CLI for rendering Mermaid diagrams as ASCII art or SVG.

## Features

- **Parse** Mermaid diagram syntax (flowcharts, sequence diagrams, class diagrams, ER diagrams)
- **Render to ASCII** using Unicode box-drawing characters or plain ASCII
- **Render to SVG** with CSS variable theming
- **Zero runtime dependencies** for the core library

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
mermaid-ascii = "0.1"
```

## Usage

### Library

```rust
use mermaid_ascii::{render, render_to_svg};

// Render to ASCII (Unicode box-drawing)
let ascii = render("graph LR\n  A --> B", false).unwrap();
println!("{}", ascii);

// Render to ASCII (plain ASCII characters)
let ascii = render("graph LR\n  A --> B", true).unwrap();
println!("{}", ascii);

// Render to SVG
let svg = render_to_svg("graph LR\n  A --> B").unwrap();
println!("{}", svg);
```

### CLI

```bash
# Render to ASCII
echo "graph LR\n  A --> B" | mermaid-ascii

# Render to ASCII (plain ASCII mode)
echo "graph LR\n  A --> B" | mermaid-ascii --ascii

# Render to SVG
echo "graph LR\n  A --> B" | mermaid-ascii --svg
```

## Supported Diagram Types

### Flowcharts (graph/flowchart)

```mermaid
graph TD
    A[Start] --> B{Decision}
    B -->|Yes| C[Do something]
    B -->|No| D[Do something else]
    C --> E[End]
    D --> E
```

### Sequence Diagrams

```mermaid
sequenceDiagram
    Alice->>Bob: Hello
    Bob-->>Alice: Hi back
```

### Class Diagrams

```mermaid
classDiagram
    Animal <|-- Dog
    Animal : +int age
    Dog : +bark()
```

### ER Diagrams

```mermaid
erDiagram
    CUSTOMER ||--o{ ORDER : places
    ORDER ||--|{ LINE-ITEM : contains
```

## Output Examples

### ASCII (Unicode)

```
┌───┐     ┌───┐     ┌───┐
│ A │────▶│ B │────▶│ C │
└───┘     └───┘     └───┘
```

### ASCII (Plain)

```
+---+     +---+     +---+
| A |---->| B |---->| C |
+---+     +---+     +---+
```

### SVG

The SVG output uses CSS variables for theming:

```html
<svg style="--bg:#FFFFFF;--fg:#27272A">
  <!-- Override colors: --bg, --fg, --accent, --line, etc. -->
</svg>
```

## Development

```bash
# Run tests
cargo test

# Run all tests including ignored ones
cargo test -- --ignored

# Build release
cargo build --release
```

## License

MIT

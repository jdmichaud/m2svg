use m2svg::{render_mermaid_ascii, render_to_svg, AsciiRenderOptions};
use std::fs;
use std::io::{self, Read};
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("m2svg - Convert Mermaid diagrams to ASCII art or SVG");
        println!();
        println!("Usage: m2svg [OPTIONS] [INPUT]");
        println!();
        println!("Reads Mermaid diagram from argument or stdin and outputs ASCII art or SVG.");
        println!();
        println!("Options:");
        println!("  -h, --help     Show this help message");
        println!("  -a, --ascii    Use plain ASCII characters (default: Unicode)");
        println!("  -s, --svg      Output SVG instead of ASCII");
        println!();
        println!("Examples:");
        println!("  echo 'graph LR\\n  A --> B' | m2svg");
        println!("  m2svg 'graph LR\\n  A --> B'");
        println!("  m2svg --svg 'graph TD\\n  A --> B' > diagram.svg");
        return;
    }

    let use_ascii = args.iter().any(|a| a == "-a" || a == "--ascii");
    let use_svg = args.iter().any(|a| a == "-s" || a == "--svg");

    // Get input from argument or stdin
    let input: String = args
        .iter()
        .skip(1)
        .find(|a| !a.starts_with('-'))
        .cloned()
        .map(|s| {
            // Check if it's a file path (either "-" for stdin, or an existing file)
            if s == "-" {
                let mut buf = String::new();
                io::stdin()
                    .read_to_string(&mut buf)
                    .expect("Failed to read from stdin");
                buf
            } else if Path::new(&s).exists() {
                fs::read_to_string(&s).unwrap_or_else(|_| panic!("Failed to read file: {}", s))
            } else {
                // Treat as inline mermaid content
                s.replace("\\n", "\n")
            }
        })
        .unwrap_or_else(|| {
            let mut buf = String::new();
            io::stdin()
                .read_to_string(&mut buf)
                .expect("Failed to read from stdin");
            buf
        });

    if input.trim().is_empty() {
        eprintln!("Error: No input provided");
        std::process::exit(1);
    }

    if use_svg {
        match render_to_svg(&input) {
            Ok(output) => println!("{}", output),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        let options = AsciiRenderOptions {
            use_ascii,
            ..Default::default()
        };

        match render_mermaid_ascii(&input, Some(options)) {
            Ok(output) => println!("{}", output),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

use m2svg::{render_mermaid_ascii, AsciiRenderOptions};
use std::io::{self, Read};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("mermaid-ascii - Convert Mermaid diagrams to ASCII art");
        println!();
        println!("Usage: mermaid-ascii [OPTIONS] [INPUT]");
        println!();
        println!("Reads Mermaid diagram from argument or stdin and outputs ASCII art.");
        println!();
        println!("Options:");
        println!("  -h, --help     Show this help message");
        println!("  -u, --unicode  Use Unicode box-drawing characters (default: ASCII)");
        println!();
        println!("Example:");
        println!("  echo 'graph LR\\n  A --> B' | mermaid-ascii");
        println!("  mermaid-ascii 'graph LR\\n  A --> B'");
        return;
    }
    
    let use_unicode = args.iter().any(|a| a == "-u" || a == "--unicode");
    
    // Get input from argument or stdin
    let input: String = args.iter()
        .skip(1)
        .find(|a| !a.starts_with('-'))
        .cloned()
        .map(|s| s.replace("\\n", "\n"))
        .unwrap_or_else(|| {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf).expect("Failed to read from stdin");
            buf
        });
    
    if input.trim().is_empty() {
        eprintln!("Error: No input provided");
        std::process::exit(1);
    }
    
    let options = AsciiRenderOptions {
        use_ascii: !use_unicode,
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

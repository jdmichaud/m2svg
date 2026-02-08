#!/bin/bash
# Regenerate test fixture expected output from current code.
#
# Usage:
#   ./regen_fixtures.sh                          # Regenerate ALL fixtures
#   ./regen_fixtures.sh class                    # Regenerate all class diagram fixtures
#   ./regen_fixtures.sh class/cls_basic          # Regenerate one specific fixture (ASCII + Unicode + SVG if exists)
#   ./regen_fixtures.sh --svg                    # Regenerate ALL SVG fixtures only
#   ./regen_fixtures.sh --svg class              # Regenerate SVG class fixtures only
#   ./regen_fixtures.sh --svg class/class_basic  # Regenerate one SVG fixture

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# Ensure binary is up to date
cargo build --quiet 2>/dev/null

svg_only=false
if [ "$1" = "--svg" ]; then
    svg_only=true
    shift
fi

filter="${1:-}"

regen_txt_fixture() {
    local file="$1"
    local mode="$2"  # "ascii" or "unicode"

    # Use Python to split on last --- (handles YAML frontmatter with --- delimiters)
    local input
    input=$(python3 -c "
import sys
content = open('$file').read()
idx = content.rfind('\n---\n')
if idx < 0:
    sys.exit(1)
print(content[:idx])
" 2>/dev/null) || return 1

    local flag=""
    if [ "$mode" = "ascii" ]; then
        flag="-a"
    fi

    local output
    output=$(echo "$input" | cargo run --quiet -- $flag 2>/dev/null)

    # Write back: input + separator + new output
    python3 -c "
import sys
content = open('$file').read()
idx = content.rfind('\n---\n')
input_part = content[:idx]
output = sys.stdin.read()
open('$file', 'w').write(input_part + '\n---\n' + output)
" <<< "$output"

    echo "  $file"
}

regen_svg_fixture() {
    local mmd_file="$1"
    local svg_file="${mmd_file%.mmd}.svg"

    grep -v '^\s*#' "$mmd_file" | cargo run --quiet -- -s - > "$svg_file" 2>/dev/null
    echo "  $svg_file"
}

# Collect and regenerate ASCII/Unicode fixtures
if [ "$svg_only" = false ]; then
    echo "Regenerating ASCII/Unicode fixtures..."
    for mode in ascii unicode; do
        if [ -n "$filter" ]; then
            # filter can be "class" (subdir) or "class/cls_basic" (specific test)
            if [[ "$filter" == */* ]]; then
                # Specific file
                file="testdata/${mode}/${filter}.txt"
                if [ -f "$file" ]; then
                    regen_txt_fixture "$file" "$mode"
                fi
            else
                # Subdirectory
                find "testdata/${mode}/${filter}" -name '*.txt' -type f 2>/dev/null | sort | while read -r file; do
                    regen_txt_fixture "$file" "$mode"
                done
            fi
        else
            # All fixtures
            find "testdata/${mode}" -name '*.txt' -type f | sort | while read -r file; do
                regen_txt_fixture "$file" "$mode"
            done
        fi
    done
fi

# Collect and regenerate SVG fixtures
echo "Regenerating SVG fixtures..."
if [ -n "$filter" ]; then
    if [[ "$filter" == */* ]]; then
        # Specific file
        mmd_file="testdata/svg/${filter}.mmd"
        if [ -f "$mmd_file" ]; then
            regen_svg_fixture "$mmd_file"
        fi
    else
        # Subdirectory
        find "testdata/svg/${filter}" -name '*.mmd' -type f 2>/dev/null | sort | while read -r file; do
            regen_svg_fixture "$file"
        done
    fi
else
    # All SVG fixtures
    find "testdata/svg" -name '*.mmd' -type f | sort | while read -r file; do
        regen_svg_fixture "$file"
    done
fi

echo "Done."

#!/bin/bash
# Usage: ./split_svg_tests.sh <folder>
# Recursively split .txt files at '---' into .mmd and .svg files, then remove the original .txt file.

set -e

if [ -z "$1" ]; then
  echo "Usage: $0 <folder>"
  exit 1
fi


find "$1" -type f -name '*.txt' | while read -r file; do
  dir="$(dirname "$file")"
  base="$(basename "$file" .txt)"
  # Find the line number of the first --- separator
  nb_sep_line=$(grep -n '^---$' "$file" | wc -l)
  if [ ! "$nb_sep_line" -eq 1 ]; then
    echo "${file}: warning: There should be 1 separator, ${nb_sep_line} found."
    continue
  fi
  total_lines=$(wc -l < "$file")
  sep_index=$(awk '/^---/ { print NR }' ${file})
  # Write lines before separator to .mmd
  head -n $((sep_index - 1)) "$file" > "$dir/$base.mmd"
  # Write lines after separator to .svg
  tail -n $((total_lines - sep_index)) "$file" | xmllint -format - > "$dir/$base.svg"
  rm ${file}
done

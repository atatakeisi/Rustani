#!/usr/bin/env bash
# Export the owner's Rustani artwork with macOS tools. No files are deleted.
# Uses shell-local variables only; does not export or change environment variables.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
icon_dir="$root/assets/app-icon"
source_png="$icon_dir/rustani-source.png"
icon_id="ai.storyteller.gridcraft"
work_dir="$(mktemp -d "${TMPDIR:-/tmp}/rustani-icons.XXXXXX")"

command -v sips >/dev/null || { echo "error: macOS sips is required" >&2; exit 1; }
command -v iconutil >/dev/null || { echo "error: macOS iconutil is required" >&2; exit 1; }
test -f "$source_png" || { echo "error: missing $source_png" >&2; exit 1; }

resize() {
  sips --resampleHeightWidth "$1" "$1" "$source_png" --out "$2" >/dev/null
}

resize 1024 "$icon_dir/rustani-1024.png"
resize 512 "$icon_dir/rustani-macos-512.png"

for size in 16 24 32 48 64 128 256 512; do
  output_dir="$icon_dir/rustani-hicolor/${size}x${size}/apps"
  mkdir -p "$output_dir"
  resize "$size" "$output_dir/$icon_id.png"
done

ico_pngs=()
for size in 16 20 24 32 40 48 64 128 256; do
  resize "$size" "$work_dir/ico-$size.png"
  ico_pngs+=("$work_dir/ico-$size.png")
done
(cd "$root" && cargo run -q -p xtask --no-default-features -- ico "$icon_dir/rustani.ico" "${ico_pngs[@]}")

iconset_dir="$work_dir/Rustani.iconset"
mkdir "$iconset_dir"
for size in 16 32 128 256 512; do
  resize "$size" "$iconset_dir/icon_${size}x${size}.png"
  resize "$((size * 2))" "$iconset_dir/icon_${size}x${size}@2x.png"
done
iconutil -c icns -o "$icon_dir/rustani.icns" "$iconset_dir"
echo "Rustani icons exported. Intermediate files retained at $work_dir"

#!/bin/sh
# Rebuilds the stitch files and previews of the free starter pack from svg/*.svg.
# Usage: assets/starter-pack/build.sh [path/to/stitchfoundry-cli]
set -e
cd "$(dirname "$0")"
CLI="${1:-../../target/release/stitchfoundry-cli}"
mkdir -p dst pes preview
for f in svg/*.svg; do
  n=$(basename "$f" .svg)
  "$CLI" run --in "$f" --cmd edit.selectAll --cmd embroidery.convert \
    --export "dst/$n.dst" --export "pes/$n.pes" --export "preview/$n.png" >/dev/null
done

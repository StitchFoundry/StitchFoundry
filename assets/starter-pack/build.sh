#!/bin/sh
# Rebuilds the stitch files and previews of the free starter pack from svg/*.svg.
# Usage: assets/starter-pack/build.sh /absolute/path/to/stitchfoundry-cli   (needs jq)
set -e
cd "$(dirname "$0")"
CLI="${1:-../../target/release/stitchfoundry-cli}"
mkdir -p dst pes preview
for f in svg/*.svg; do
  n=$(basename "$f" .svg)
  out=$("$CLI" run --in "$f" --cmd select.all --cmd embroidery.convert \
    --cmd embroidery.export --params '{"format":"dst","name":"'"$n"'"}' \
    --cmd embroidery.export --params '{"format":"pes","name":"'"$n"'"}')
  echo "$out" | jq -r 'select(.result.format=="dst") | .result.dataBase64' | base64 -d > "dst/$n.dst"
  echo "$out" | jq -r 'select(.result.format=="pes") | .result.dataBase64' | base64 -d > "pes/$n.pes"
  "$CLI" run --in "$f" --cmd select.all --cmd embroidery.convert \
    --cmd embroidery.exportImage --params '{"path":"'"$PWD/preview/$n.png"'","ppmm":8}' >/dev/null
done

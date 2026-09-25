#!/bin/sh
# Inlines core.js into each design, so every mockup is one standalone file.
cd "$(dirname "$0")"
for f in [g-j]-*.html; do
  awk '/<script src="core.js"><\/script>/ { print "<script>"; while ((getline l < "core.js") > 0) print l; close("core.js"); print "</script>"; next } { print }' "$f" > "../$f"
done

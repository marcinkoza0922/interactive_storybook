#!/bin/sh
# Lists Rust source files longer than a limit (default 400 lines); fails if there are any.
# Agents read whole files, so long ones cost the most to work with.
limit=${1:-400}
over=$(find cli/src -name '*.rs' -exec wc -l {} + | awk -v max="$limit" '$2 != "total" && $1 > max')
[ -z "$over" ] && exit 0
echo "Files over $limit lines:"
echo "$over"
exit 1

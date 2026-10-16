#!/bin/sh
set -eu
# Initial deterministic scale signal for governance review. Revisit when repository facts change.
crates=$(find crates -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l | tr -d ' ')
active=$(find docs/backlog/active -type f -name '*.md' 2>/dev/null | wc -l | tr -d ' ')
echo "crates=$crates"
echo "active_backlog_items=$active"
echo "has_external_integrations=yes"
echo "has_desktop_gui=yes"
echo "continuing_feature_development=yes"
echo "recommended_profile=product"

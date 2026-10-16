#!/bin/sh
set -eu
root="${1:-.}"
cd "$root"
failed=0
need() { if [ ! -e "$1" ]; then echo "ERROR missing: $1"; failed=1; fi; }
for f in \
  AGENTS.md CLAUDE.md GEMINI.md EVOLUTION.md .agent-governance/manifest.yaml \
  docs/README.md docs/backlog/PRODUCT-BACKLOG.md docs/iterations/README.md \
  docs/decisions/README.md docs/roadmap/IMPLEMENTATION-ROADMAP.md docs/proposals/README.md \
  docs/reference/README.md docs/reference/PRODUCT-VISION.md docs/reference/PRODUCT-REQUIREMENTS-v0.1.md docs/reference/ARCHITECTURE.md \
  docs/sop/REQUIREMENT-INTAKE.md docs/sop/START-ITERATION.md docs/sop/ITERATION-WORKFLOW.md \
  docs/sop/CHANGE-CONTROL.md docs/sop/TESTING.md docs/sop/GIT-WORKFLOW.md \
  docs/sop/EVOLUTION-FEEDBACK.md docs/sop/DOC-CHECK.md; do need "$f"; done
for h in "Hard Constraints" "Coding Behavior" "Git Rules" "Task Router" "Session End Checklist"; do
  grep -Fq "$h" AGENTS.md || { echo "ERROR AGENTS.md missing section: $h"; failed=1; }
done
grep -Eq 'type\(scope\): description \(#story-id\) \[model:<model-name>\]' AGENTS.md || { echo "ERROR commit format missing"; failed=1; }
grep -Fq 'profile: "product"' .agent-governance/manifest.yaml || { echo "ERROR manifest product profile missing"; failed=1; }
grep -Fq 'status: "conformant"' .agent-governance/manifest.yaml || { echo "ERROR manifest conformant state missing"; failed=1; }
# Basic local markdown-link check for relative .md links.
python_cmd=""
if command -v ruby >/dev/null 2>&1; then
  ruby -e 'Dir.glob("{README.md,AGENTS.md,EVOLUTION.md,docs/**/*.md}").each do |f|; s=File.read(f, encoding: "UTF-8"); s.scan(/\[[^\]]+\]\(([^)#]+\.md)(?:#[^)]+)?\)/).flatten.each do |l|; next if l.include?("://"); p=File.expand_path(l,File.dirname(f)); abort("ERROR broken Markdown link: #{f} -> #{l}") unless File.file?(p); end; end' || failed=1
else
  echo "WARNING ruby unavailable; local Markdown link check skipped"
fi
if [ "$failed" -ne 0 ]; then exit 1; fi
echo "Laputa governance validation passed"

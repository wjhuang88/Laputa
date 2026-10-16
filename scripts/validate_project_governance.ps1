param([string]$ProjectRoot = ".")
$ErrorActionPreference = "Stop"
$root = (Resolve-Path $ProjectRoot).Path
$required = @(
  "AGENTS.md", "CLAUDE.md", "GEMINI.md", "EVOLUTION.md", ".agent-governance/manifest.yaml",
  "docs/README.md", "docs/backlog/PRODUCT-BACKLOG.md", "docs/iterations/README.md",
  "docs/decisions/README.md", "docs/roadmap/IMPLEMENTATION-ROADMAP.md", "docs/proposals/README.md",
  "docs/reference/README.md", "docs/reference/PRODUCT-VISION.md", "docs/reference/PRODUCT-REQUIREMENTS-v0.1.md", "docs/reference/ARCHITECTURE.md",
  "docs/sop/REQUIREMENT-INTAKE.md", "docs/sop/START-ITERATION.md", "docs/sop/ITERATION-WORKFLOW.md",
  "docs/sop/CHANGE-CONTROL.md", "docs/sop/TESTING.md", "docs/sop/GIT-WORKFLOW.md",
  "docs/sop/EVOLUTION-FEEDBACK.md", "docs/sop/DOC-CHECK.md"
)
$failed = $false
foreach ($path in $required) {
  if (-not (Test-Path (Join-Path $root $path))) {
    Write-Error "missing: $path"; $failed = $true
  }
}
$agents = Get-Content (Join-Path $root "AGENTS.md") -Raw
foreach ($heading in @("Hard Constraints", "Coding Behavior", "Git Rules", "Task Router", "Session End Checklist")) {
  if (-not $agents.Contains($heading)) { Write-Error "AGENTS.md missing section: $heading"; $failed = $true }
}
if (-not $agents.Contains('type(scope): description (#story-id) [model:<model-name>]')) {
  Write-Error "Agent commit format missing"; $failed = $true
}
if ($failed) { exit 1 }
Write-Output "Laputa governance validation passed"

# Documentation Consistency Check

Run after requirement, architecture, Story, iteration, ADR or process changes.

- Product requirements and architecture do not contradict active ADRs.
- `PRODUCT-BACKLOG.md` rows point to every active Story/Epic file and mandatory reads.
- Every child Story names its parent and every Epic summary reflects child status/dependencies.
- Iterations select Ready Stories and preserve their published baseline.
- Observable behavior changes update README/usage documentation.
- Procedures contain current commands/paths rather than stale planned commands.
- Broken local Markdown links are absent.
- Residual work has an owning backlog/Story/iteration record.
- If governance capability paths/rules change, synchronize `.agent-governance/manifest.yaml` and run `scripts/validate_project_governance.sh .`.

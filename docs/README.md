# Laputa Documentation Map

This directory separates stable facts, executable work, decisions, repeatable procedures, future direction, and approved design references so Agents do not infer status from stale prose or chat history.

## Start here

- [Product vision](reference/PRODUCT-VISION.md)
- [V0.1 product requirements](reference/PRODUCT-REQUIREMENTS-v0.1.md)
- [Architecture baseline](reference/ARCHITECTURE.md)
- [V0.1 UI/UX & visual vision](design/UI-UX-VISION-v0.1.md)
- [Product backlog](backlog/PRODUCT-BACKLOG.md)
- [Manual acceptance ledger (current blockers, future GUI checks, optional live smokes)](acceptance/MANUAL-ACCEPTANCE.md)
- [Implementation roadmap](roadmap/IMPLEMENTATION-ROADMAP.md)

## Responsibilities

- `reference/`: current approved product/architecture facts and contracts.
- `design/`: approved visual/interaction direction and repository-resident design evidence; behavior remains owned by product requirements/Stories.
- `backlog/`: executable Epics and Stories; source of truth for planned work status.
- `iterations/`: bounded execution batches and validation evidence.
- `acceptance/`: cross-session manual validation ledger; never mark human tests PASS based on CI alone.
- `decisions/`: architectural/product tradeoffs that constrain implementation.
- `roadmap/`: ordering of future stages, not an implementation checklist.
- `proposals/`: unapproved ideas that must not be implemented directly.
- `sop/`: repeatable execution procedures and gates.

Do not copy the same rule into several owners. Link to the authoritative document instead.

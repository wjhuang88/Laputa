# Requirement Intake

Use this procedure whenever a new feature, product change, defect with behavioral implications, or architecture-affecting request arrives.

1. Read the current product requirements, architecture and backlog before asking the user to restate known context.
2. Classify important constraints as Hard, Soft, or Assumption. Hard constraints drive gates; assumptions need validation paths.
3. Decide whether the request is one independently verifiable Story or a multi-outcome Epic.
4. Use `LAPUTA-NNN` for a new Epic family and `LAPUTA-NNN-A`, `-B`, ... for child Stories. Preserve existing IDs.
5. Behavior-facing Stories require role/goal/value, scope, out-of-scope, Given/When/Then acceptance, dependencies, minimum validation and state owners.
6. Technical/governance Stories require a verifiable engineering/process outcome, exclusions, command/manual validation, dependencies and status owners; do not fake user-story wording.
7. If implementation is constrained by an ADR, the Story must link it and identify the acceptance impact before becoming Ready.
8. If a blocking assumption cannot be resolved from available evidence, create a bounded Spike with expected decision/evidence output instead of starting implementation.
9. Update `docs/backlog/PRODUCT-BACKLOG.md` as the compact decision surface and put execution detail in `docs/backlog/active/` item files.
10. An Epic may be refined/plannable, but iterations select Ready child Stories, not a bare Epic.

A Story is Ready only when dependencies are explicit and satisfiable, acceptance is observable, exclusions prevent scope expansion, required reads/decisions are linked, and the minimum validation path is feasible.

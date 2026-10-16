# Start Iteration

An iteration is a bounded execution batch that must preserve its published baseline.

1. Inventory any non-terminal iteration records first; resolve or explicitly keep their status before starting another.
2. Read `docs/backlog/PRODUCT-BACKLOG.md` and the candidate Story files.
3. Select Ready child Stories only; do not select a parent Epic as executable work.
4. Ensure dependency order is closed within completed work or explicit earlier steps in the same iteration.
5. Prefer the smallest set that produces a runnable, testable output. A foundation-only batch needs an explicit exception.
6. Create `docs/iterations/YYYY-MM-DD-<goal>.md` with: goal, selected Stories, published baseline, dependency order, expected user-visible/document changes, validation plan, risks/assumptions and status `planned`.
7. After committing/publishing the plan, never replace its original target silently; record scope changes and disposition through change control.

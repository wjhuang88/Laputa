# Iteration Workflow

For each selected Story:

1. Re-read Story acceptance, ADRs/specs, dependencies and exclusions.
2. Record/confirm success criteria before code changes.
3. Implement only the selected slice.
4. Run the Story's minimum validation plus tests for changed risk surfaces.
5. Record actual command/manual evidence in the iteration record.
6. Update Story status and acceptance evidence, then synchronize the parent Epic summary.
7. Update user-facing docs in the same slice when behavior changed.
8. Register residual work instead of hiding it in the retrospective.

Iteration closure requires a runnable/testable result, actual evidence, synchronized owner docs, and a strict `complete`/`partial`/`blocked` result. A passing isolated unit test for unwired code is not a complete product result.

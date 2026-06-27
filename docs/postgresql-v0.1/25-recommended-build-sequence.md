# PostgreSQL v0.1 Recommended Build Sequence

Be strict about sequence. The fastest path is not a full UI, full adapter, or full Docker image first. The fastest path is a small vertical path that proves repository state, PostgreSQL inspection, deterministic files, compare, and artifacts.

## Recommended Sequence

1. Resolve Slice 1 blockers: repository layout rules, initialization behavior, Git status behavior, and first CLI/service boundary.
2. Build local repository structure detection.
3. Add approval-gated project initialization.
4. Add PostgreSQL inspection for a limited object set: schemas and simple tables first.
5. Export selected inspected objects to per-object files.
6. Add golden-file tests for exported files.
7. Compare source database objects against repository files.
8. Add database-to-repo synchronization with explicit approval.
9. Add repository file import.
10. Add repository-to-target database compare.
11. Generate summary artifacts before SQL generation.
12. Add dependency warnings for the first meaningful cases.
13. Add synchronization plan generation.
14. Add SQL synchronization script generation and release artifacts.
15. Add risk classification.
16. Add basic configured reference-data compare.
17. Add CLI JSON contracts around proven core operations.
18. Integrate browser UI workflows through the service.
19. Add Docker automation after CLI behavior stabilizes.
20. Add AI-review context generation from deterministic artifacts.

## Why This Sequence

Repository structure and inspection are the smallest useful path. They prove the source-of-truth model before spending effort on broad UI or packaging work.

Script generation should come after compare and plan generation. Dependency analysis should be present before generated scripts are trusted.

Reference data, CLI, UI, Docker, and AI review should build on stable core artifacts instead of driving the initial design.

## Avoid Starting With

- Full browser UI.
- Full PostgreSQL object coverage.
- Docker packaging.
- CI workflows.
- Advanced dependency graph visualization.
- Deployment Rehearsal.
- Non-PostgreSQL engines.

## First Useful Demo

The first useful demo should show:

1. Open a Git repo.
2. Recognize DbState structure.
3. Inspect a PostgreSQL source database with simple objects.
4. Export selected objects to per-object files.
5. Show Git status.

This does not prove the whole MVP, but it proves the source-of-truth path.

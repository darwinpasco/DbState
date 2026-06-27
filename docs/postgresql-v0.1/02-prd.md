# DbState PostgreSQL v0.1 PRD

## Traceability

This PRD expands the BRD goals into product requirements for DbState PostgreSQL v0.1. It remains aligned with the general DbState foundation.

## Personas

- Developer: wants to capture and review PostgreSQL object changes in Git.
- Database developer: wants object-level files for tables, views, functions, triggers, and reference data.
- DBA reviewer: wants risk, dependency warnings, and generated SQL artifacts before deployment.
- Automation user: wants CLI, JSON output, and Docker workflows.
- AI-assisted reviewer: wants Codex or Claude to explain artifacts without executing SQL or seeing credentials.

## User Journeys

1. Open or clone a Git repository, initialize DbState structure, connect to a PostgreSQL source database, inspect schema, select objects, and synchronize approved objects into per-object files.
2. Compare repository state against a target PostgreSQL database, cherry-pick changes, review dependency warnings, generate a version-controlled SQL script, and produce summary and risk artifacts.
3. Use CLI or Docker to run headless inspect, compare, plan, risk, and data-compare workflows with JSON output.
4. Ask Codex or Claude to review generated artifacts and draft branch, commit, PR, or comment text.

## User Stories

- As a developer, I want Git to store desired PostgreSQL state so schema changes can be reviewed like code.
- As a database developer, I want one durable PostgreSQL object per file so object history is easy to inspect.
- As a DBA, I want dependency-aware cherry-picking so generated scripts do not silently omit required dependencies.
- As an automation user, I want JSON output so CI/CD jobs and agents can consume deterministic results.
- As an AI-assisted reviewer, I want generated review text grounded in DbState artifacts so it does not invent changes.

## Functional Requirements

- Open, clone, and initialize DbState repositories.
- Show Git status, branch, changed object files, staged files, and dirty-working-tree warnings.
- Inspect PostgreSQL schema using a PostgreSQL adapter.
- Normalize supported PostgreSQL object definitions.
- Write per-object desired-state files under `database/objects/`.
- Configure reference data under `database/reference-data/`.
- Compare source database to repository state.
- Compare repository state to target database state.
- Show visual schema compare and visual data compare.
- Allow cherry-picking of schema objects, object changes, reference tables, rows, and safe columns.
- Analyze dependencies for selected and excluded changes.
- Block script generation by default for critical missing dependencies.
- Generate SQL synchronization scripts under `database/releases/`.
- Generate summary, risk, and AI-reviewable artifacts.
- Expose CLI commands with JSON output for important workflows.
- Support Docker headless automation as a design concern.
- Generate AI-assisted draft branch names, commit messages, PR titles, PR bodies, and comments from deterministic artifacts.

## Non-Functional Requirements

- Local-first by default.
- Deterministic results for diff, dependency analysis, risk classification, and script generation.
- No direct target-database apply.
- No secrets in repository files or generated artifacts.
- No credentials exposed to browser UI or AI agents.
- Cross-platform native-build direction for Windows, macOS, and Linux.
- Docker for automation, not as the primary human UI.
- Clear failure states and reviewable errors.

## Acceptance Criteria

- The product can represent selected PostgreSQL objects as per-object files.
- Generated synchronization scripts are written to the local repository and visible in Git status.
- Per-object files remain the desired-state source of truth.
- Generated scripts are deployment artifacts, not source of truth.
- Repository synchronization from a source PostgreSQL database requires user approval.
- Script generation for a target PostgreSQL database never applies SQL.
- Critical missing dependencies block script generation by default.
- Dependency overrides, where allowed, are explicit and recorded.
- AI-generated text is draft-only, editable, and grounded in deterministic artifacts.

## MVP Workflow

1. Clone or open a Git repository.
2. Connect to a PostgreSQL source database.
3. Inspect schema and configured reference data.
4. Export or synchronize selected objects into per-object files.
5. Review visual diffs.
6. Stage, commit, and push repository changes only with approval.
7. Connect to a PostgreSQL target database.
8. Compare repository state against the target.
9. Cherry-pick changes.
10. Review dependency warnings and risk classification.
11. Generate version-controlled SQL synchronization scripts and companion artifacts.
12. Let Codex or Claude review the artifacts.
13. Deploy outside DbState through a human-controlled process.

## UX Requirements

- Browser UI presents project, object explorer, compare, review, Git, and report workflows.
- No direct apply button exists.
- Dependency warnings are visible before script generation.
- Critical blockers are visually distinct from non-critical warnings.
- Generated artifacts are reviewable before staging, committing, or pushing.
- Git actions require explicit confirmation.
- AI drafts are visibly drafts and editable before use.

## Reporting Requirements

- Object-level change summary.
- Risk report.
- Dependency warning report.
- Generated script metadata.
- Reference-data change summary.
- Optional machine-readable JSON report.
- AI-reviewable context artifact.

## AI-Assisted Workflow Requirements

AI may draft branch names, commit messages, PR titles, PR bodies, review comments, and script explanations from deterministic DbState artifacts.

AI must not execute SQL, approve database changes, override dependency warnings, create branches, commit, push, create PRs, or post comments without explicit user approval.

## Exclusions

PostgreSQL v0.1 excludes non-PostgreSQL engines, team server features, Deployment Rehearsal, direct target-database apply, full transactional data compare, and production deployment execution.

# PostgreSQL v0.1 MVP Scope

## MVP Objective

DbState PostgreSQL v0.1 should prove that PostgreSQL desired state can be captured, compared, reviewed, and planned through Git-native per-object files and generated deployment artifacts without directly applying SQL to target databases.

## Included Capabilities

- Local-first DbState Service with browser UI.
- First-class Git integration.
- PostgreSQL schema inspection.
- Per-object desired-state files.
- Visual schema compare.
- Source PostgreSQL database to repository synchronization.
- Repository to target PostgreSQL database script generation only.
- Version-controlled synchronization scripts.
- Cherry-picking with dependency warnings.
- Basic configured reference-data compare.
- Risk classification.
- CLI with JSON output.
- Docker image for headless automation.
- AI-assisted review text generation, draft-only and grounded in DbState artifacts.

## Excluded Capabilities

- MySQL, SQL Server, SQLite, Db2, or Access support.
- Cloud dashboard.
- Self-hosted team server.
- RBAC and SSO.
- Team approval workflows.
- Continuous drift monitoring.
- Policy-as-code gates.
- Deployment Rehearsal.
- Compliance evidence pack.
- Advanced dependency graph visualization.
- Full transactional data compare.
- Direct target-database apply.
- Production deployment execution.

## Free Beta and Professional Preview Assumptions

Free beta may include the core local workflow: visual schema compare, basic Git integration, database-to-repo synchronization, repo-to-database script generation, version-controlled release scripts, cherry-picking, dependency warnings, basic risk report, limited reference-data compare, basic CLI, and basic AI-assisted branch and PR text.

Professional preview may expand local project count, reference-data compare depth, reporting, Docker support, and AI-assisted workflow text.

These are packaging assumptions, not licensing implementation.

Direct target-database apply is not available in any tier.

## Critical Acceptance Scenario

1. Clone or open a Git repo.
2. Connect to a PostgreSQL source database.
3. Export or synchronize selected objects into per-object files.
4. Show visual diff.
5. Commit and push repo changes with approval.
6. Connect to a PostgreSQL target database.
7. Compare repo against target.
8. Cherry-pick changes.
9. Warn about missing dependencies.
10. Generate a version-controlled SQL synchronization script.
11. Generate summary and risk artifacts.
12. Let Codex or Claude review the artifacts.
13. Never apply the script directly to the target database.

## Definition of Done

- PostgreSQL v0.1 requirements are documented.
- Documents are aligned with the general DbState foundation.
- The MVP boundary is clear.
- Non-MVP items are explicit.
- Open decisions are captured.
- No implementation code is created by this documentation task.

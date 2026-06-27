# DbState PostgreSQL v0.1 BRD

## Executive Summary

DbState PostgreSQL v0.1 is the first RDBMS-specific edition of DbState. It should prove that PostgreSQL database state can be managed through Git-native, per-object desired-state files, visual compare, safe script generation, reference-data control, and deterministic review artifacts.

The product does not execute generated deployment SQL against target databases. It generates reviewed, version-controlled synchronization scripts for human-controlled deployment outside DbState.

## Business Problem

Teams often need to understand how live PostgreSQL databases differ from intended database state, capture approved database state in Git, and review deployment changes before they reach shared or production environments.

Migration-only approaches can make the final desired state hard to inspect. Traditional compare tools can be helpful visually, but they are not always Git-native and may blur review boundaries around generated scripts and direct synchronization.

DbState PostgreSQL should make PostgreSQL database state visible, reviewable, versioned, and safer to plan.

## Target Users

- Application developers who own PostgreSQL schema changes.
- Database developers who maintain stored functions, views, triggers, and reference data.
- DBAs who review deployment risk and drift.
- DevOps and platform engineers who need CLI, JSON, Docker, and CI/CD workflows.
- Consultants who compare client databases and prepare reviewed deployment artifacts.
- AI-assisted engineering users who want Codex or Claude to review deterministic artifacts without exposing credentials or applying SQL.

## Product Opportunity

DbState PostgreSQL can occupy the space between migration-only workflows and traditional schema/data compare tools:

- Git-native desired state.
- Visual PostgreSQL schema and data compare.
- Per-object files.
- Explicit reference-data control.
- Deterministic dependency-aware planning.
- Version-controlled release scripts.
- Local-first security.
- CLI and Docker automation.
- AI-assisted review text grounded in deterministic artifacts.

No market size, revenue estimate, or unsupported competitor claim is assumed.

## Business Goals

- Prove the general DbState thesis using PostgreSQL as the first native edition.
- Establish a clear no-direct-apply safety boundary.
- Make Git workflows part of the database change workflow.
- Support local-first use before team server features.
- Produce reviewable artifacts suitable for human deployment processes.
- Create a foundation for future paid packaging without implementing licensing gates.

## Product Principles

- Git is the source of truth.
- Per-object PostgreSQL files are the desired-state source of truth.
- Generated SQL scripts are version-controlled deployment artifacts.
- The browser UI is the presentation layer.
- DbState Service performs the actual work.
- The core engine remains deterministic.
- PostgreSQL is handled natively.
- Direct target-database apply is not allowed.
- Cherry-picking must be dependency-aware.
- AI may assist review text but must not perform privileged actions without explicit approval.

## In-Scope Capabilities

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

## Out-of-Scope Capabilities

- MySQL, SQL Server, SQLite, Db2, or Access support.
- Cloud dashboard.
- Self-hosted team server.
- RBAC, SSO, and team approval workflows.
- Continuous drift monitoring.
- Policy-as-code gates.
- Deployment Rehearsal.
- Compliance evidence packs.
- Advanced dependency graph visualization.
- Full transactional data compare.
- Direct target-database apply.
- Production deployment execution.

## Commercial Packaging Assumptions

Commercial packaging is future direction only. Free, Professional, Team, and Enterprise tiers may exist later.

Direct target-database apply is not available in any tier.

Deployment Rehearsal is premium later, not MVP.

## Success Criteria

- Users can capture selected PostgreSQL objects into Git as per-object files.
- Users can see visual diffs between repository state and PostgreSQL databases.
- Users can generate version-controlled synchronization scripts for a target PostgreSQL database.
- Users can review risk, dependency warnings, and selected changes before generating scripts.
- Critical missing dependencies block script generation by default.
- Generated artifacts contain no secrets or unmasked PII.
- AI review is useful but bounded by deterministic artifacts and explicit user approval.

## Risks

- PostgreSQL object dependency analysis may be complex.
- Function, trigger, extension, and grant normalization may require careful edition-specific rules.
- Reference-data comparison may accidentally include sensitive data if masking and configuration are weak.
- Users may expect a direct apply button because traditional tools often include one.
- Git integration can be risky if credentials or local-only files are mishandled.

## Open Questions

- Which PostgreSQL versions are supported in v0.1.
- Exact v0.1 packaging model.
- Exact Git implementation approach.
- Whether Docker ships in v0.1 or immediately after.
- Whether MCP is v0.1 or post-MVP.

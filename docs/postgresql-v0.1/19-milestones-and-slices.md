# PostgreSQL v0.1 Milestones and Slices

## Slice 0: Repository and Build Skeleton Planning Only

Objective: define the future skeleton without creating it in this task.

User-visible outcome: none yet.

Technical scope: planning docs, open decisions, first-slice definition.

Exclusions: no code, no Cargo project, no package files, no Dockerfile, no CI.

Acceptance criteria: implementation plan exists and preserves product boundaries.

Risks: planning may overreach into implementation.

## Slice 1: Local Project Initialization and Repository Structure Recognition

Objective: recognize and initialize DbState PostgreSQL repository structure.

User-visible outcome: user can validate or initialize `database/objects`, `database/reference-data`, and `database/releases`.

Technical scope: repository root detection, project layout validation, warnings, initial CLI/service behavior.

Exclusions: PostgreSQL connection, schema inspection, UI polish.

Acceptance criteria: missing folders are reported, initialization creates only approved project folders, Git status is visible.

Risks: unclear behavior for non-Git folders.

## Slice 2: PostgreSQL Connection Profile and Schema Inspection

Objective: inspect a PostgreSQL database safely.

User-visible outcome: user can see supported object inventory.

Technical scope: connection profile, credential boundary, catalog queries, object inventory.

Exclusions: export, compare, script generation.

Acceptance criteria: credentials are not exposed, unsupported objects are reported.

Risks: catalog complexity and version differences.

## Slice 3: Export Selected PostgreSQL Objects to Per-Object Files

Objective: write selected inspected objects into desired-state files.

User-visible outcome: selected schemas and objects appear under `database/objects/`.

Technical scope: normalization, schema-qualified file naming, golden-file output.

Exclusions: full dependency graph, target planning.

Acceptance criteria: exported files are deterministic and visible in Git status.

Risks: false diffs from unstable formatting.

## Slice 4: Source Database to Repo Synchronization

Objective: compare a source database with repository files and update selected files.

User-visible outcome: user can accept selected source changes into the repo.

Technical scope: source-to-repo diff, approval-gated file writes, Git status.

Exclusions: direct database deployment.

Acceptance criteria: selected object changes update files only after approval.

Risks: accidentally overwriting local edits.

## Slice 5: Repo to Target Database Compare

Objective: compare repository desired state against a target PostgreSQL database.

User-visible outcome: user can see target drift from repository state.

Technical scope: file import, target inspection, diff model.

Exclusions: SQL generation and apply.

Acceptance criteria: target comparison is read-only and deterministic.

Risks: inconsistent normalization between inspected and file states.

## Slice 6: Cherry-Pick Synchronization Plan with Dependency Warnings

Objective: let users select changes and see dependency impact.

User-visible outcome: selected plan shows required and missing dependencies.

Technical scope: plan model, dependency warnings, blocker rules.

Exclusions: advanced dependency graph visualization.

Acceptance criteria: critical missing dependencies block script generation by default.

Risks: dependency gaps produce unsafe plans.

## Slice 7: Generated SQL Synchronization Script and Release Artifacts

Objective: generate version-controlled deployment artifacts.

User-visible outcome: SQL script, summary, and risk files appear under `database/releases/`.

Technical scope: SQL generation, metadata header, summary markdown, risk JSON.

Exclusions: executing generated SQL.

Acceptance criteria: artifacts appear in Git status and contain no secrets.

Risks: unsafe SQL or incomplete metadata.

## Slice 8: Basic Reference-Data Compare

Objective: compare explicitly configured reference-data tables.

User-visible outcome: inserts, updates, and configured deletes are shown for controlled tables.

Technical scope: registry parsing, table files, business-key matching, masking.

Exclusions: full transactional data compare.

Acceptance criteria: unconfigured tables are ignored and masked columns are not exposed.

Risks: reference-data misuse or sensitive data leakage.

## Slice 9: CLI JSON Output

Objective: expose core workflows through CLI JSON contracts.

User-visible outcome: automation can call inspect, compare, plan, risk, and data-compare.

Technical scope: command contracts, JSON schemas, exit codes.

Exclusions: CI workflow files.

Acceptance criteria: JSON outputs are stable and no command applies generated SQL.

Risks: CLI and service contracts drift.

## Slice 10: Browser UI Workflow Integration

Objective: connect UI workflows to service operations.

User-visible outcome: browser UI drives project, compare, plan, artifact, Git, and AI draft review screens.

Technical scope: local API integration and UI state.

Exclusions: direct browser database operations.

Acceptance criteria: UI presents work performed by the service and has no direct apply button.

Risks: UI scope expands too early.

## Slice 11: Docker Automation

Objective: run headless workflows in automation.

User-visible outcome: Docker can run CLI/service automation against mounted repos.

Technical scope: Docker behavior, mounts, non-secret configuration, JSON outputs.

Exclusions: Docker as desktop UI.

Acceptance criteria: Docker preserves no-direct-apply and secret boundaries.

Risks: Docker behavior diverges from native behavior.

## Slice 12: AI-Review Context Generation

Objective: generate safe context for Codex and Claude.

User-visible outcome: AI can review artifacts and draft workflow text.

Technical scope: context artifact format, redaction, grounding metadata.

Exclusions: AI actions without approval.

Acceptance criteria: AI context contains deterministic artifacts and no credentials.

Risks: AI overreach or ungrounded summaries.

# PostgreSQL v0.1 Engineering Backlog

## PGV01-001

Title: Define project structure validation rules

Type: feature

Description: Define how DbState recognizes a valid PostgreSQL project repository.

Acceptance criteria: recognizes `database/objects`, `database/reference-data`, and `database/releases`; reports missing folders; does not create files without approval.

Dependencies: none.

Priority: P0

Suggested slice: Slice 1

## PGV01-002

Title: Define repository initialization behavior

Type: feature

Description: Specify approval-gated initialization of the preferred project structure.

Acceptance criteria: creates only project folders and registry placeholder if approved; reports Git status.

Dependencies: PGV01-001.

Priority: P0

Suggested slice: Slice 1

## PGV01-003

Title: Spike PostgreSQL catalog inspection

Type: spike

Description: Determine MVP catalog query strategy for supported PostgreSQL objects.

Acceptance criteria: sample object inventory produced from fixture databases; unsupported object reporting defined.

Dependencies: none.

Priority: P0

Suggested slice: Slice 2

## PGV01-004

Title: Define PostgreSQL object identity model

Type: feature

Description: Define identity rules for schemas, tables, columns, constraints, indexes, views, functions, triggers, and grants.

Acceptance criteria: identity keys are stable and account for schema-qualified names and function signatures.

Dependencies: PGV01-003.

Priority: P0

Suggested slice: Slice 2

## PGV01-005

Title: Golden-file export expectations

Type: test

Description: Define expected per-object file outputs for representative PostgreSQL objects.

Acceptance criteria: golden examples exist for first object set and define stable formatting.

Dependencies: PGV01-004.

Priority: P0

Suggested slice: Slice 3

## PGV01-006

Title: Export selected source objects to files

Type: feature

Description: Export selected inspected objects to `database/objects/` using deterministic file names.

Acceptance criteria: files are schema-qualified, deterministic, and visible in Git status.

Dependencies: PGV01-005.

Priority: P0

Suggested slice: Slice 3

## PGV01-007

Title: Source database to repository diff

Type: feature

Description: Compare source database objects against repository files.

Acceptance criteria: reports added, changed, removed, and unsupported objects.

Dependencies: PGV01-006.

Priority: P0

Suggested slice: Slice 4

## PGV01-008

Title: Approval-gated repository synchronization

Type: feature

Description: Update selected local object files from a source PostgreSQL database after approval.

Acceptance criteria: no file update occurs without approval; Git status shows changed files.

Dependencies: PGV01-007.

Priority: P0

Suggested slice: Slice 4

## PGV01-009

Title: Repository file import

Type: feature

Description: Read desired-state object files into the domain model.

Acceptance criteria: malformed files produce clear errors; supported files produce stable objects.

Dependencies: PGV01-006.

Priority: P0

Suggested slice: Slice 5

## PGV01-010

Title: Repository to target database compare

Type: feature

Description: Compare repository desired state to target PostgreSQL database state.

Acceptance criteria: comparison is read-only and produces deterministic diffs.

Dependencies: PGV01-009.

Priority: P0

Suggested slice: Slice 5

## PGV01-011

Title: Dependency extraction spike

Type: spike

Description: Evaluate `pg_depend` and parsed-reference needs for MVP dependency warnings.

Acceptance criteria: dependency gaps and blocker rules are documented.

Dependencies: PGV01-003.

Priority: P0

Suggested slice: Slice 6

## PGV01-012

Title: Selected synchronization plan model

Type: feature

Description: Model selected objects, exclusions, required dependencies, missing dependencies, warnings, and overrides.

Acceptance criteria: plan output is machine-readable and deterministic.

Dependencies: PGV01-010, PGV01-011.

Priority: P0

Suggested slice: Slice 6

## PGV01-013

Title: Critical dependency blocker tests

Type: test

Description: Verify missing critical dependencies block script generation by default.

Acceptance criteria: blocked plans cannot generate scripts unless policy allows a recorded non-critical override.

Dependencies: PGV01-012.

Priority: P0

Suggested slice: Slice 6

## PGV01-014

Title: SQL synchronization artifact generation

Type: feature

Description: Generate SQL scripts and companion artifacts from selected plans.

Acceptance criteria: artifacts are written under `database/releases/`, visible in Git status, and contain non-secret metadata.

Dependencies: PGV01-012.

Priority: P0

Suggested slice: Slice 7

## PGV01-015

Title: No-direct-apply negative tests

Type: test

Description: Verify UI, service, CLI, Docker, and future MCP designs expose no target apply operation.

Acceptance criteria: generated SQL is never executed against target databases by DbState.

Dependencies: PGV01-014.

Priority: P0

Suggested slice: Slice 7

## PGV01-016

Title: Reference-data registry parsing

Type: feature

Description: Parse configured reference-data registry and table files.

Acceptance criteria: unconfigured tables are ignored; masked and ignored columns are honored.

Dependencies: reference-data format decision.

Priority: P1

Suggested slice: Slice 8

## PGV01-017

Title: Reference-data compare tests

Type: test

Description: Verify insert, update, configured delete, ignored-column, and masked-column behavior.

Acceptance criteria: masked values do not appear in reports or AI context.

Dependencies: PGV01-016.

Priority: P1

Suggested slice: Slice 8

## PGV01-018

Title: CLI JSON contracts

Type: feature

Description: Define JSON output for init, status, inspect, export, compare, plan, risk, and data-compare.

Acceptance criteria: schemas are stable and include errors, warnings, and Git status where relevant.

Dependencies: domain model.

Priority: P0

Suggested slice: Slice 9

## PGV01-019

Title: Service API contract tests

Type: test

Description: Define local service API contract tests for project, Git, inspection, compare, plan, artifact, and risk areas.

Acceptance criteria: UI and CLI can rely on stable service behavior.

Dependencies: service framework decision.

Priority: P1

Suggested slice: Slice 10

## PGV01-020

Title: Browser workflow integration

Type: feature

Description: Connect UI screens to service workflows for project, compare, plan, artifacts, Git, and AI drafts.

Acceptance criteria: no direct database operations or direct apply button exist in the browser UI.

Dependencies: PGV01-019.

Priority: P1

Suggested slice: Slice 10

## PGV01-021

Title: Docker automation design

Type: tooling

Description: Define Docker headless automation behavior without creating Dockerfiles in this planning task.

Acceptance criteria: mount, command, credential, and no-direct-apply boundaries are documented.

Dependencies: CLI design.

Priority: P1

Suggested slice: Slice 11

## PGV01-022

Title: AI-review context artifact

Type: feature

Description: Generate deterministic non-secret context for Codex and Claude review.

Acceptance criteria: context includes diffs, plans, risk, dependency warnings, artifact paths, and Git metadata; excludes credentials and unmasked PII.

Dependencies: PGV01-014.

Priority: P1

Suggested slice: Slice 12

## PGV01-023

Title: Secret leakage test suite

Type: security

Description: Verify credentials, tokens, connection strings, local paths, and unmasked PII do not appear in repository files, artifacts, logs, JSON, or AI context.

Acceptance criteria: representative leakage cases fail tests.

Dependencies: credential and artifact designs.

Priority: P0

Suggested slice: all slices from Slice 2 onward.

## PGV01-024

Title: Implementation open decisions review

Type: documentation

Description: Review and resolve decisions that block Slice 1 and Slice 2.

Acceptance criteria: Slice 1 blockers are resolved before coding starts; later decisions are prioritized.

Dependencies: planning pack.

Priority: P0

Suggested slice: Slice 0

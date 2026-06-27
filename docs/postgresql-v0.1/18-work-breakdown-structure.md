# PostgreSQL v0.1 Work Breakdown Structure

## Repository and Project Structure

Goal: recognize and initialize the DbState PostgreSQL project layout.

Major tasks:

- Detect Git repository root.
- Detect `database/objects`, `database/reference-data`, and `database/releases`.
- Define initialization behavior.
- Report missing or invalid structure.

Dependencies: repository structure ADR, Git status behavior.

Deliverables: project validation model, initialization plan, user-facing warnings.

Status: MVP.

## Core Domain Model

Goal: define stable internal concepts for objects, diffs, plans, risks, and artifacts.

Major tasks:

- Model PostgreSQL object identity.
- Model desired-state files.
- Model comparison result.
- Model selected synchronization plan.
- Model risk and dependency warnings.

Dependencies: PostgreSQL object model, dependency analysis.

Deliverables: domain model specification and JSON contract candidates.

Status: MVP.

## PostgreSQL Inspection

Goal: inspect supported PostgreSQL objects through native catalog queries.

Major tasks:

- Inspect schemas, extensions, enums, sequences, tables, columns, constraints, indexes, views, materialized views, functions, triggers, and grants.
- Report unsupported or deferred objects.
- Preserve enough metadata for normalization and dependencies.

Dependencies: catalog inspection spike, supported version decision.

Deliverables: inspection design, fixture expectations, edge-case list.

Status: MVP with staged object coverage.

## Object Normalization

Goal: produce stable PostgreSQL object definitions for comparison and file export.

Major tasks:

- Normalize object ordering.
- Normalize object definitions.
- Preserve semantic differences.
- Define function body and grants behavior.

Dependencies: SQL formatting spike, object identity rules.

Deliverables: normalization rules and golden-file expectations.

Status: MVP.

## File Export and Import

Goal: write and read per-object desired-state files.

Major tasks:

- Export selected objects under `database/objects/`.
- Use schema-qualified names.
- Read files back into domain objects.
- Detect malformed or unsupported files.

Dependencies: repository structure, normalization.

Deliverables: file layout rules, import/export behavior, golden files.

Status: MVP.

## Git Integration

Goal: make Git status and approved Git actions first-class.

Major tasks:

- Show status, branch, changed files, and generated artifacts.
- Support clone, checkout, fetch, pull, stage, unstage, commit, and push through approval-gated workflows.
- Warn on dirty working tree.
- Detect merge conflicts.

Dependencies: Git implementation decision.

Deliverables: Git workflow design and safety checks.

Status: MVP, staged by workflow.

## Schema Compare

Goal: compare PostgreSQL source, repository, and target states.

Major tasks:

- Compare source database to repository.
- Compare repository to target database.
- Classify added, changed, removed, and unsupported objects.
- Feed visual compare and CLI JSON.

Dependencies: inspection, import/export, normalization.

Deliverables: diff model and report format.

Status: MVP.

## Dependency Analysis

Goal: make cherry-picked plans safe enough for script generation.

Major tasks:

- Extract PostgreSQL dependencies.
- Mark selected objects affected by exclusions.
- Mark excluded required dependencies.
- Block critical missing dependencies.
- Record overrides.

Dependencies: dependency graph spike, object model.

Deliverables: dependency warning model and blocker rules.

Status: MVP.

## Synchronization Plan Generation

Goal: convert selected diffs into a deterministic plan.

Major tasks:

- Store selected and excluded objects.
- Include dependency warnings.
- Include risk inputs.
- Produce machine-readable plan output.

Dependencies: schema compare, dependency analysis.

Deliverables: selected synchronization plan format.

Status: MVP.

## Deployment Artifact Generation

Goal: generate version-controlled SQL scripts and review artifacts.

Major tasks:

- Generate SQL synchronization script.
- Generate summary markdown.
- Generate risk JSON.
- Generate AI-review context.
- Write artifacts under `database/releases/`.

Dependencies: plan generation, SQL generation spike, artifact naming decision.

Deliverables: release artifact set and metadata header.

Status: MVP.

## Reference-Data Compare

Goal: compare explicitly configured reference data.

Major tasks:

- Read registry.
- Read table state files.
- Compare source/repo/target rows by business key.
- Support ignored and masked columns.
- Detect inserts, updates, and configured deletes.

Dependencies: YAML versus JSON decision.

Deliverables: reference-data diff model and examples.

Status: MVP basic.

## Risk Classification

Goal: classify deployment risk deterministically.

Major tasks:

- Identify destructive schema changes.
- Identify reference-data deletes.
- Identify missing dependencies.
- Identify grant and role risks.
- Record overrides.

Dependencies: diff model, dependency analysis.

Deliverables: risk taxonomy and report format.

Status: MVP basic.

## Browser UI

Goal: present the core workflow through the service.

Major tasks:

- Project open/clone.
- Object explorer.
- Visual schema and data compare.
- Cherry-pick workflow.
- Dependency and risk review.
- Artifact review.
- Git stage, commit, and push review.
- AI draft review.

Dependencies: service API, diff model.

Deliverables: UI workflow screens and state model.

Status: MVP, after core path is proven.

## DbState Service API

Goal: expose local operations to UI, CLI, and future MCP surfaces.

Major tasks:

- Project management.
- Git operations.
- Connection profiles.
- Inspection.
- Compare.
- Plan.
- Artifact generation.
- Risk reporting.
- AI-review context generation.

Dependencies: service framework decision.

Deliverables: local API contract.

Status: MVP.

## CLI

Goal: support headless automation with JSON output.

Major tasks:

- Define command names.
- Define JSON output contracts.
- Support init, status, inspect, export, compare, plan, risk, and data-compare.
- Enforce no-direct-apply boundary.

Dependencies: domain model, API boundaries.

Deliverables: CLI contract and examples.

Status: MVP.

## Docker Automation

Goal: run headless workflows consistently in automation.

Major tasks:

- Define Docker execution shape.
- Define volume and repository mount expectations.
- Define connection profile handling.
- Preserve no-direct-apply boundary.

Dependencies: CLI and service shape.

Deliverables: Docker behavior specification.

Status: MVP or immediately after, open decision.

## AI-Review Context Generation

Goal: generate safe, deterministic context for Codex and Claude.

Major tasks:

- Produce non-secret context from diffs, plans, scripts, risk reports, and Git metadata.
- Exclude credentials and unmasked PII.
- Mark generated text as draft-only.

Dependencies: artifact generation and risk reports.

Deliverables: AI-review context format.

Status: MVP.

## Security and Credential Handling

Goal: protect credentials, secrets, and sensitive data.

Major tasks:

- Choose credential store strategy.
- Keep credentials behind service boundary.
- Add secret leakage checks.
- Mask configured data.
- Prevent logs from exposing secrets.

Dependencies: credential storage spike.

Deliverables: security design and test cases.

Status: MVP.

## Testing and Validation

Goal: verify deterministic behavior and safety boundaries.

Major tasks:

- Unit tests.
- PostgreSQL integration tests.
- Golden-file tests.
- CLI JSON contract tests.
- Service API contract tests.
- No-direct-apply negative tests.
- Security leakage tests.

Dependencies: test strategy ADR.

Deliverables: validation plan and fixture strategy.

Status: MVP.

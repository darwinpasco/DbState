# DbState Agent Guidance

## Product summary

DbState is a product family for state-based, per-object database version control, schema compare, data compare, drift detection, and safe deployment planning.

DbState is not a generic lowest-common-denominator database abstraction. Each future RDBMS edition should respect the native SQL dialect, object model, deployment behavior, and operational risks of that database engine.

## Core thesis

- Git is the source of truth.
- One durable database object equals one file.
- The repository stores desired database state.
- Generated SQL is a reviewed deployment artifact, not the source of truth.
- Visual schema compare and visual data compare are first-class workflows.
- Reference data and configuration data can be versioned deliberately.
- Transactional data is not versioned by default.
- Drift between Git and a live database must be visible, explainable, and actionable.
- Dangerous database changes require explicit human review.
- DbState must be deterministic first. AI may assist, but AI must not be required for core correctness.
- Each RDBMS edition should use native SQL and native semantics. Do not invent a universal SQL DSL as the core product model.

## Current scope

This repository is the upstream/base DbState product repository. It contains general product-family context and shared principles.

PostgreSQL is the planned first edition, but PostgreSQL v0.1 BRD, PRD, SDD, and detailed implementation design are out of scope until requested.

## Out of scope

- Creating forks or separate repositories.
- Starting product-specific edition documents.
- Creating PostgreSQL v0.1 BRD, PRD, SDD, or implementation design.
- Pushing to GitHub without explicit user approval.
- Inventing market statistics, revenue numbers, TAM/SAM/SOM, or competitor claims.
- Implementing licensing or commercial gating.

## Architecture direction

- Browser UI is the presentation layer only.
- DbState Service performs real work: repository access, Git operations, database inspection, credential handling, compare orchestration, deployment artifact generation, and local API exposure.
- DbState Core Engine performs deterministic normalization, diffing, dependency analysis, risk classification, SQL generation, and artifact generation.
- Native RDBMS adapters handle engine-specific semantics.
- CLI provides headless automation, JSON output, CI/CD use, and agent integration.
- Docker image supports headless automation, scheduled drift checks, CI/CD, and later server-style runtime.
- MCP may be added later for local deterministic agent workflows.

Preferred technology direction is Rust for the core engine, service, and CLI, likely Axum or Actix Web for the service, and React plus TypeScript for the browser UI. This is preferred direction, not a final decision unless captured by an ADR.

## Cross-platform requirement

DbState must be cross-platform by architecture and shipped as separate native builds per platform.

- Windows, macOS, and Linux are distribution targets, not separate product lines.
- Each RDBMS edition should have native builds for Windows, macOS, and Linux.
- Each RDBMS edition should have native CLI binaries for Windows, macOS, and Linux.
- Docker is for Linux-based automation and server-style runtime, not the primary desktop product.
- The product line should split by RDBMS, not operating system.

Correct names include `DbState PostgreSQL for Windows` and `DbState SQL Server Docker image`. Incorrect names include `DbState Windows` and `DbState Linux`.

## Git integration

DbState must support Git workflows directly. Git is not only an external tool users run manually.

DbState should support clone, open repository, initialize DbState structure, status, branch, checkout, fetch, pull, stage, unstage, commit, push, object diff, merge-conflict detection, and dirty-working-tree warnings.

Git operations are performed by the DbState Service, not directly by the browser UI. The browser UI presents status, branch selection, commit review, pull and push results, changed database objects, and conflict warnings.

DbState must not auto-push or auto-commit without explicit user approval. DbState must not commit secrets, passwords, tokens, credentials, local-only configuration, local machine paths, or unmasked PII.

## Synchronization safety

DbState may synchronize a local repository from a source database. With user approval, it may update local object files and configured reference-data files, then stage, commit, or push Git changes only with explicit approval.

When the local repository is ahead of a target database, DbState may compare, plan, classify risk, and generate SQL synchronization scripts. DbState must not directly apply generated SQL to a target database.

The no-direct-apply rule applies to browser UI, DbState Service, service API, CLI, Docker image, MCP server, future APIs, Codex, Claude, and other AI agents.

Codex and Claude may review generated scripts, plans, dependency reports, and risk reports. They must not execute scripts or apply database changes.

Human-controlled deployment remains outside DbState.

## Deployment artifacts

Generated synchronization scripts must be version-controlled in Git. They are deployment artifacts, not the primary source of truth. Per-object files remain the desired-state source of truth.

Generated scripts should be written into the local repository by default, recommended under `database/releases/`. They must appear in Git status and Git diff. They may be staged, committed, and pushed only with explicit user approval.

Generated deployment artifacts must not contain secrets, credentials, connection strings, tokens, local-only paths, unmasked PII, or transactional production data.

## Selective sync and dependency analysis

Visual schema compare and visual data compare must support cherry-picking. Cherry-picked synchronization plans must be dependency-aware.

DbState must mark selected objects that may break if dependencies are excluded. DbState must mark excluded objects required by selected objects. Critical missing dependencies should block script generation by default.

Allowed dependency overrides must be explicit and recorded in the generated SQL comments or header, summary markdown, risk JSON, and AI-reviewable artifacts.

DbState must not silently generate a broken synchronization script.

## AI-assisted Git and review workflows

AI may help draft branch names, commit messages, PR titles, PR bodies, review comments, object-level explanations, risk summaries, dependency warning comments, and deployment artifact summaries.

AI-generated text must be grounded in deterministic DbState artifacts. It must be editable before use.

AI must not create branches, commit, push, create PRs, post comments, approve changes, override warnings, or execute SQL without explicit user approval.

## Writing style

- Be direct and specific.
- Avoid hype, cliches, fake precision, and generic enterprise filler.
- Use concrete examples where helpful.
- Mark unresolved items as open decisions.
- Do not invent market statistics, revenue numbers, TAM/SAM/SOM, or competitor claims.
- Keep Markdown readable.
- Use tables only when they improve clarity.

## Documentation conventions

- General product-family documents live under `docs/`.
- Architecture decisions live under `adr/`.
- Product-specific edition documents should not be added until that edition work is explicitly requested.
- Use `database/objects/`, `database/reference-data/`, and `database/releases/` when describing project structure.
- Avoid using `database/schema/` or `database/schemas/` as the root structure because `schema` is overloaded.

## Security principles

- Local-first by default.
- No cloud dependency for core workflows.
- Credentials must use safe platform mechanisms where possible.
- Never expose Git or database credentials to browser UI, AI agents, logs, reports, or repository files.
- Do not commit secrets, passwords, tokens, connection strings, local paths, unmasked PII, or local-only configuration.
- Production data should not be copied into temporary or generated artifacts by default.

## Definition of done

- Requested files exist and stay within the requested scope.
- Documents are consistent with the general DbState product-family strategy.
- Open decisions are captured instead of hidden.
- No product-specific PostgreSQL v0.1 BRD, PRD, SDD, or implementation design is created unless explicitly requested.
- Git status is checked before completion.
- No push is performed without explicit user approval.

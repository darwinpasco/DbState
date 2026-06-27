# PostgreSQL v0.1 Technical Spikes

## PostgreSQL Catalog Inspection Strategy

Question: which catalog queries provide stable coverage for the MVP object set?

Why it matters: inspection quality drives normalization, compare, dependencies, and script generation.

Options: direct `pg_catalog` queries, `information_schema` where sufficient, hybrid.

Recommended first experiment: inspect schemas, tables, columns, constraints, indexes, views, functions, triggers, and grants from `pg_catalog`.

Output expected: catalog query notes and fixture output.

Decision owner or reviewer role: PostgreSQL adapter lead and DBA reviewer.

## PostgreSQL Object Normalization

Question: how should object definitions be normalized without hiding semantic differences?

Why it matters: unstable normalization creates false diffs.

Options: catalog-derived canonical format, `pg_get_*def` output with post-processing, SQL formatter.

Recommended first experiment: compare `pg_get_*def` output against hand-authored expected files for simple objects.

Output expected: normalization rules and golden-file examples.

Decision owner or reviewer role: core engine lead.

## Function and Trigger Extraction

Question: how should overloaded functions, trigger functions, and trigger definitions be extracted and named?

Why it matters: functions and triggers are common drift sources.

Options: signature-aware file names, metadata sidecar, one file per exact identity.

Recommended first experiment: export overloaded functions and triggers into candidate file names.

Output expected: identity and naming recommendation.

Decision owner or reviewer role: PostgreSQL adapter lead.

## Grants and Ownership Handling

Question: how should grants and ownership be represented for MVP?

Why it matters: grants drift matters, but roles may differ by environment.

Options: include grants fully, include grants with environment policy, defer ownership.

Recommended first experiment: compare grants across two fixtures with different role availability.

Output expected: MVP grants scope and risk rules.

Decision owner or reviewer role: DBA reviewer.

## Dependency Graph Extraction

Question: how should DbState extract required and dependent PostgreSQL objects?

Why it matters: selective sync must be dependency-aware.

Options: `pg_depend` graph, parsed SQL references, hybrid.

Recommended first experiment: build dependency output for views, functions, foreign keys, triggers, and materialized views.

Output expected: dependency warning model and known gaps.

Decision owner or reviewer role: core engine lead and PostgreSQL adapter lead.

## SQL Parser or Formatting Approach

Question: does v0.1 need a SQL parser, formatter, both, or neither for initial object coverage?

Why it matters: parsing and formatting affect normalization and script generation.

Options: catalog-defined output only, SQL formatter, parser for selected objects, hybrid.

Recommended first experiment: use catalog-defined output for initial objects and document where parsing is required.

Output expected: parser and formatter recommendation.

Decision owner or reviewer role: architecture reviewer.

## System Git Versus libgit2 or Hybrid

Question: should v0.1 call system Git, use libgit2/git2-rs, or use a hybrid?

Why it matters: Git credentials, signing, proxies, and enterprise behavior are sensitive.

Options: system Git, libgit2/git2-rs, hybrid.

Recommended first experiment: prototype status, branch, stage, commit, and push behavior conceptually against system Git requirements.

Output expected: implementation recommendation with credential tradeoffs.

Decision owner or reviewer role: service lead.

## Credential Storage Approach

Question: where should PostgreSQL connection credentials be stored?

Why it matters: credentials must not reach repository files, browser UI, AI agents, logs, or generated artifacts.

Options: platform credential store, existing secret manager integration later, session-only credentials for MVP.

Recommended first experiment: compare platform store behavior across Windows, macOS, and Linux.

Output expected: MVP credential strategy.

Decision owner or reviewer role: security reviewer.

## Local Service Port and Browser UI Serving Model

Question: how does the browser UI connect to the local service?

Why it matters: service discovery, local security, and cross-platform packaging depend on it.

Options: fixed localhost port, dynamic port with launcher handoff, bundled desktop shell later.

Recommended first experiment: evaluate localhost binding and token handoff design.

Output expected: local service access recommendation.

Decision owner or reviewer role: service and UI leads.

## YAML Versus JSON Reference-Data Format

Question: should v0.1 support YAML, JSON, or both for reference-data files?

Why it matters: format affects user editing, parsing, validation, and deterministic output.

Options: YAML only, JSON only, both with canonical write format.

Recommended first experiment: compare schema validation and stable serialization for example tables.

Output expected: source format recommendation.

Decision owner or reviewer role: product and core engine reviewer.

## Rust Service Framework Choice

Question: should the service use Axum, Actix Web, or another Rust framework?

Why it matters: local API behavior, async model, testing, and long-term maintenance are affected.

Options: Axum, Actix Web, another Rust HTTP framework.

Recommended first experiment: compare local API testability, middleware, streaming, and packaging needs.

Output expected: service framework recommendation.

Decision owner or reviewer role: architecture reviewer.

## CLI Framework Choice

Question: which CLI framework should v0.1 use?

Why it matters: command ergonomics, JSON output, help text, and testing depend on it.

Options: clap or another Rust CLI framework.

Recommended first experiment: map proposed commands to framework capabilities.

Output expected: CLI framework recommendation.

Decision owner or reviewer role: CLI lead.

## Docker Automation Shape

Question: should Docker run CLI only, service mode, or both?

Why it matters: automation behavior and security boundaries differ.

Options: CLI-only image, service image, combined image.

Recommended first experiment: define volume mounts, working directory behavior, and JSON output path.

Output expected: Docker automation design note.

Decision owner or reviewer role: DevOps reviewer.

## AI-Review Context Artifact Format

Question: what file format should AI-review context use?

Why it matters: AI reviews must be grounded, deterministic, and safe.

Options: Markdown, JSON, both.

Recommended first experiment: generate a sample context from a fake plan and risk report.

Output expected: context schema and redaction checklist.

Decision owner or reviewer role: product and security reviewer.

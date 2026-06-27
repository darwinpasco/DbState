# Synchronization Safety Principles

DbState supports diff generation and synchronization planning. It does not directly apply generated SQL to real target databases.

## Source database to repository

DbState may synchronize a local repository from a source database.

With user approval, DbState may:

- Inspect the source database.
- Compare the source database against local repository object files.
- Generate schema and reference-data diffs.
- Update local object files.
- Update configured reference-data files.
- Stage changed files.
- Commit changed files.
- Push committed changes.

Staging, committing, and pushing require explicit user approval.

## Repository to target database

DbState may compare a local repository against a target database.

When the local repository is ahead of the target database, DbState must only generate a SQL synchronization script, risk report, and deployment summary. Generated SQL is a deployment artifact.

DbState must not:

- Directly apply changes to a target database.
- Execute generated deployment SQL against a target database.
- Expose direct database apply through browser UI, service API, CLI, Docker, MCP, or AI agent integration.

Human-controlled deployment remains outside DbState.

## Version-controlled scripts

Generated SQL synchronization scripts must be version-controlled in Git. They should be written into a dedicated repository folder such as `database/releases/`.

Generated scripts must appear in Git status and Git diff. DbState may stage, commit, and push generated scripts only with explicit user approval.

Generated scripts must not contain secrets, credentials, connection strings, tokens, local-only paths, unmasked PII, or transactional production data.

## Selective synchronization

Visual schema compare and data compare must allow cherry-picking.

Selective synchronization must be dependency-aware. Critical missing dependencies should block script generation by default.

Dependency warnings and overrides must be included in generated deployment artifacts.

Codex and Claude may review and explain generated synchronization scripts. AI agents must not execute scripts or apply database changes.

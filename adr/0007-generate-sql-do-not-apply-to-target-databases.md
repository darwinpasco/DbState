# ADR 0007: Generate SQL, Do Not Apply to Target Databases

## Status

Accepted

## Context

DbState must support compare and synchronization planning while preserving a clear safety boundary for real target databases.

## Decision

DbState will generate SQL synchronization scripts for target databases but will not directly apply those scripts to target databases.

DbState may synchronize local repository files from a source database. It may update object files and configured reference-data files after user approval.

DbState may compare a local repository against a target database. It may generate SQL synchronization scripts, risk reports, and deployment summaries.

DbState must not execute generated SQL against target databases. DbState must not provide direct target-database apply through browser UI, service API, CLI, Docker, MCP, or AI integrations.

AI agents such as Codex and Claude may review generated scripts but must not apply them.

The deployment script is a reviewed artifact intended for human-controlled deployment outside DbState.

## Consequences

DbState can produce useful deployment artifacts without becoming the production deployment executor. Users retain control over how scripts are executed.

## Alternatives considered

- Direct live database synchronization.
- Optional apply button for target databases.
- Agent-executed database deployment.

## Open questions

- How deployment artifact metadata should be standardized.
- Which deployment tools should be documented as external execution options.

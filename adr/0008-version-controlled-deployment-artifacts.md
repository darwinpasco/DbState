# ADR 0008: Version-Controlled Deployment Artifacts

## Status

Accepted

## Context

Generated synchronization scripts are important review artifacts. Treating them as temporary files would weaken auditability and collaboration.

## Decision

DbState will version-control generated synchronization scripts as deployment artifacts.

Per-object state files remain the source of truth. Generated SQL synchronization scripts are reviewed deployment artifacts.

Generated scripts must be written into the local Git repository by default. Generated scripts must be visible in Git status and Git diff.

Generated scripts may be staged, committed, and pushed only with explicit user approval.

Generated scripts must not contain secrets, credentials, connection strings, tokens, local-only paths, unmasked PII, or transactional production data.

DbState must not directly apply generated scripts to target databases. AI agents may review generated scripts but must not execute them.

Exact folder structure and naming conventions remain open decisions unless decided later.

## Consequences

Generated scripts can be reviewed, versioned, discussed, and audited. The relationship between per-object desired state and generated deployment artifacts must be clear.

## Alternatives considered

- Temporary local-only generated scripts.
- Auto-committed generated scripts.

## Open questions

- Default release artifact folder.
- Naming convention.
- Required companion artifacts.

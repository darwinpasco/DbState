# ADR 0010: Premium Deployment Rehearsal

## Status

Accepted as direction

## Context

Users may want confidence that a generated synchronization script will work before executing it through a human-controlled deployment process.

## Decision

DbState will consider Deployment Rehearsal as a premium feature direction.

Deployment Rehearsal allows generated synchronization scripts to be tested against a temporary target database. It improves deployment confidence without weakening DbState's no-direct-apply rule for real target databases.

DbState may generate temporary database build scripts and rehearsal scripts. DbState may execute rehearsal against a temporary local, containerized, or self-hosted target where explicitly approved.

DbState must not apply generated synchronization scripts to the real target database. DbState must not copy sensitive production data into temporary databases by default.

DbState must generate rehearsal reports and preserve warnings, failures, and post-rehearsal drift.

AI agents may review rehearsal artifacts but must not execute deployment against real target databases.

Final naming, packaging, and edition placement remain open decisions.

## Consequences

Deployment Rehearsal can improve trust in generated scripts while preserving the production safety boundary.

## Alternatives considered

- Direct production apply after script generation.
- No rehearsal capability.

## Open questions

- Final feature name.
- Tier placement.
- Local and Docker rehearsal support timing.

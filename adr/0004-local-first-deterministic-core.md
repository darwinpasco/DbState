# ADR 0004: Local-First Deterministic Core

## Status

Accepted

## Context

DbState handles database definitions, deployment plans, reference data, credentials, and potentially sensitive metadata. Core correctness must not depend on AI judgment or a cloud service.

## Decision

DbState will be local-first and deterministic by default.

## Consequences

Core workflows can run without a cloud dependency. DbState must compute diffs, dependency analysis, risk classification, and SQL generation deterministically.

AI can assist with review and communication, but it is not required for correctness.

## Alternatives considered

- Cloud-first workflow.
- AI-dependent diffing and script generation.

## Open questions

- Which optional cloud or hosted features may exist later.
- How enterprise self-hosted server mode should be packaged.

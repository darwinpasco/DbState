# ADR 0009: Selective Sync with Dependency Analysis

## Status

Accepted

## Context

Users need to cherry-pick changes during schema and data compare. Simple checkbox selection can produce broken scripts if dependencies are ignored.

## Decision

DbState will support selective synchronization with dependency analysis.

Users may cherry-pick objects and changes for generated synchronization scripts. Schema compare and data compare must support selective inclusion.

Selective synchronization must be dependency-aware. DbState must mark selected objects that may break if dependencies are left behind. DbState must mark excluded objects that are required by selected objects.

DbState must show dependency impact before script generation. DbState must not silently generate broken scripts.

Critical missing dependencies should block script generation by default. Non-critical warnings may allow explicit override, but overrides must be recorded.

Generated deployment artifacts must include dependency warnings.

AI agents may review dependency warnings but must not override them or execute scripts.

## Consequences

Visual compare workflows become safer and more explainable. The dependency engine becomes a core product capability.

## Alternatives considered

- Simple manual selection without dependency analysis.
- Always auto-include dependencies without user approval.

## Open questions

- Exact severity model for dependency warnings.
- Which overrides are allowed per RDBMS edition.

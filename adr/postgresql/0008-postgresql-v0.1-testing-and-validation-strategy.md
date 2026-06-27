# ADR PostgreSQL 0008: PostgreSQL v0.1 Testing and Validation Strategy

## Status

Proposed

## Context

DbState PostgreSQL v0.1 must be deterministic, safe, and reviewable. It generates deployment artifacts but must not directly apply generated SQL to target PostgreSQL databases.

Testing must catch false diffs, unsafe SQL generation, dependency gaps, reference-data misuse, credential leakage, and accidental apply behavior.

## Decision

DbState PostgreSQL v0.1 should use layered tests, including golden-file tests, PostgreSQL container integration tests, CLI JSON contract tests, and no-direct-apply negative tests.

The test strategy should include unit tests, integration tests, PostgreSQL fixture tests, generated artifact tests, reference-data compare tests, dependency warning tests, risk classification tests, service API contract tests, UI workflow tests later, and security leakage tests.

## Consequences

The test suite will protect deterministic output and safety boundaries. Golden-file tests will make normalization and artifact changes explicit. Container tests will expose PostgreSQL catalog and version behavior.

The suite may be slower than unit tests alone, so fast unit tests and targeted integration suites should be separated.

## Alternatives considered

- Unit tests only.
- Manual database validation only.
- UI-driven testing as the primary safety net.
- Postponing no-direct-apply tests until late implementation.

## Open questions

- Supported PostgreSQL versions for container test matrix.
- Exact golden-file update workflow.
- Exact CLI JSON schema versioning.
- Whether UI workflow tests run in every PR or nightly.

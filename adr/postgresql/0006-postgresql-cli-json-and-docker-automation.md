# ADR PostgreSQL 0006: CLI JSON and Docker Automation

## Status

Accepted as direction

## Context

DbState PostgreSQL should support local automation, CI/CD, and AI-agent review through deterministic machine-readable outputs.

## Decision

DbState PostgreSQL v0.1 will provide CLI JSON output and Docker automation as first-class design concerns.

## Consequences

Important CLI commands should support JSON output. Docker should support headless inspect, compare, plan, risk, data-compare, and artifact generation workflows.

Docker is for automation, not the primary browser UI experience.

The no-direct-apply rule applies equally to CLI and Docker.

## Alternatives considered

- Browser UI only.
- CLI without structured output.
- Docker as the primary desktop product.

## Open questions

- Whether Docker ships in v0.1 or immediately after.
- Exact CLI command names and JSON schemas.
- Exact Docker image tags and platform support.

# ADR 0003: Cross-Platform with Separate Native Builds

## Status

Accepted

## Context

DbState should support developers, DBAs, and teams across Windows, macOS, Linux, and automation environments.

## Decision

DbState must be cross-platform by architecture and must ship as separate native builds per platform, with Docker used for automation.

Windows, macOS, and Linux are distribution targets, not separate product lines. Each RDBMS edition should have platform-specific native builds.

Docker is a headless automation distribution, not a replacement for the human UI.

The same deterministic core engine should be reused across service, browser UI workflows, CLI, Docker, and future MCP interfaces.

## Consequences

The product line should split by RDBMS, not by operating system. Native packaging and testing must be planned per platform. Docker remains important for CI/CD, scheduled drift checks, and server-style runtime.

## Alternatives considered

- Browser-only SaaS as the primary product.
- Docker-only distribution.
- Separate products per operating system.

## Open questions

- Exact packaging formats per platform.
- Release signing and update strategy.

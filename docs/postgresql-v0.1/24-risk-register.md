# PostgreSQL v0.1 Risk Register

| Risk | Impact | Likelihood | Mitigation | Owner role | Status |
| --- | --- | --- | --- | --- | --- |
| PostgreSQL catalog complexity | Incomplete inspection or wrong metadata | High | Start with limited object set and catalog spikes | PostgreSQL adapter lead | Open |
| Object normalization differences | False diffs and noisy Git changes | High | Golden-file tests and stable normalization rules | Core engine lead | Open |
| False diffs | User loses trust in compare output | Medium | Regression fixtures and reviewed formatter rules | Core engine lead | Open |
| Unsafe generated SQL | Deployment artifacts may be misleading or risky | High | Risk classification, reviews, golden SQL tests, no direct apply | Core engine lead | Open |
| Dependency analysis gaps | Broken synchronization scripts | High | Dependency graph spike and blocker tests | Core engine lead | Open |
| Reference-data misuse | Sensitive or transactional data may be versioned | Medium | Explicit registry, masking, and documentation warnings | Product lead | Open |
| Credential leakage | Credentials appear in UI, logs, artifacts, or AI context | High | Service boundary, safe storage, leakage tests | Security reviewer | Open |
| Git operation mistakes | Wrong files staged, committed, or pushed | Medium | Explicit approval and clear status views | Service lead | Open |
| AI overreach | AI suggests actions outside deterministic artifacts | Medium | Draft-only AI output and approval gates | Product lead | Open |
| User expectation of direct apply | Users may look for a sync/apply button | High | UI wording, docs, and no apply operation | Product lead | Open |
| Cross-platform packaging complexity | Delayed native distribution | Medium | Resolve packaging after core path, test per platform | Release lead | Open |
| Docker drift from native behavior | Automation differs from local behavior | Medium | Shared CLI/core behavior and container tests | DevOps reviewer | Open |
| Scope creep into MySQL or SQL Server too early | PostgreSQL v0.1 loses focus | Medium | Keep other engines out of v0.1 backlog | Product lead | Open |
| PostgreSQL edge-case expansion | MVP expands before core workflow is proven | High | Defer edge cases explicitly and report unsupported objects | PostgreSQL adapter lead | Open |
| Deployment Rehearsal scope creep | Premium future feature enters MVP | Medium | Keep rehearsal out of MVP planning | Product lead | Open |

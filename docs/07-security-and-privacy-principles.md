# Security and Privacy Principles

DbState should be local-first, deterministic, and cautious by default.

## Principles

- Local-first: core workflows should not require a cloud service.
- Deterministic: correctness comes from deterministic inspection, diffing, planning, and artifact generation.
- Credential safety: use Git credential helpers, SSH agents, platform credential stores, or other safe platform mechanisms where possible.
- Environment labels: documents and artifacts may include non-secret environment labels, not connection strings or credentials.
- Production safeguards: production operations require explicit human review.
- Data masking: configured masking must protect sensitive values in data compare, reports, and generated artifacts.
- Safe data compare: reference data and configuration data may be versioned deliberately. Transactional production data should not be versioned by default.
- Least privilege: database credentials used for inspection should require only the permissions needed for that workflow.
- Auditability: generated artifacts should explain what was compared, selected, excluded, and warned about.
- No cloud dependency for core workflows.
- No secrets in repository files.
- No secrets in generated deployment artifacts.
- No direct apply to target databases.

## Blocked content

Repository files and generated artifacts must not contain secrets, credentials, connection strings, tokens, local-only paths, unmasked PII, or transactional production data.

## Agent safety

Codex, Claude, and other agents may inspect and review deterministic artifacts. They must not receive raw credentials or execute generated SQL against a real target database.

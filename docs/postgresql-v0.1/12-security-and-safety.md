# PostgreSQL v0.1 Security and Safety

DbState PostgreSQL v0.1 is local-first and deterministic by default.

## No Direct Target-Database Apply

DbState may generate SQL synchronization scripts for target PostgreSQL databases. It must not directly apply generated SQL to target databases.

This rule applies to browser UI, DbState Service, service API, CLI, Docker, future MCP, and AI integrations.

## Credential Handling

Credentials must stay behind the DbState Service boundary. The browser UI must not receive raw credentials.

Credential storage strategy is an open decision. Safe platform mechanisms are preferred.

## Repository and Artifact Safety

Repository files and generated artifacts must not contain:

- Secrets.
- Credentials.
- Connection strings.
- Passwords.
- Tokens.
- Local-only paths.
- Unmasked PII.
- Transactional production data.

## Environment Labels

Generated artifacts may include non-secret labels such as `local`, `dev`, `uat`, or `production`. They must not include connection strings.

## Production Safeguards

Production-labeled workflows should require explicit review for destructive changes, reference-data deletes, dependency overrides, and generated deployment artifacts.

DbState must not treat a generated script as approval to deploy.

## Reference-Data Masking

Configured masked columns must be protected in UI, reports, JSON output, AI-reviewable artifacts, and generated summaries.

## AI Agent Restrictions

Codex, Claude, and other agents may review generated artifacts and draft workflow text.

They must not:

- Receive raw credentials.
- Execute SQL.
- Apply database changes.
- Override risk or dependency warnings.
- Commit, push, create PRs, post comments, or create branches without explicit user approval.

## Git Safety

DbState must warn on dirty working trees before risky operations. It must detect merge conflicts. It must not auto-commit or auto-push.

## Auditability Assumptions

Generated artifacts should preserve selected scope, excluded scope, dependency warnings, risk classification, explicit overrides, branch, commit hash where available, timestamp, and target environment label.

## Safe Logging Requirements

Logs must not include credentials, tokens, connection strings, unmasked PII, or raw production data. Error messages should be actionable without revealing secrets.

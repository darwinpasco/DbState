# Deployment Rehearsal Premium Feature

Deployment Rehearsal is a future premium feature direction. It is not an MVP requirement unless decided later.

Deployment Rehearsal allows users to test generated synchronization scripts against a temporary target database. It improves confidence without allowing DbState to apply changes to the real target database.

## Rehearsal workflow

DbState may:

- Generate scripts to build a temporary target database.
- Help initialize the temporary target from the current target baseline.
- Apply the generated synchronization script to the temporary target in controlled rehearsal mode.
- Compare the rehearsed temporary target against the desired repository state.
- Generate a rehearsal report.

DbState must not apply the synchronization script to the real target database. A successful rehearsal does not automatically approve production deployment.

## Modes

Rehearsal should support:

- Script-only mode, where DbState generates scripts and the user runs them.
- Local execution mode, where DbState creates and tests against a local temporary database after user approval.
- Docker mode, where DbState uses a disposable database container.
- Server mode later, where a self-hosted DbState Server performs rehearsal.

## Artifacts

Rehearsal artifacts should include:

- Setup script.
- Temporary target database creation script.
- Synchronization script used for rehearsal.
- Execution log.
- Post-rehearsal comparison report.
- Risk report.
- Failure report if applicable.
- AI-reviewable summary.

Sensitive production data must not be copied into temporary databases by default. Secrets, credentials, tokens, connection strings, local-only paths, and unmasked PII must not appear in rehearsal artifacts.

Codex and Claude may review rehearsal reports but must not execute deployment against the real target database.

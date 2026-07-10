# DbState PostgreSQL v0.1.0 Private Beta 2 Feedback Template

Do not paste passwords, full PostgreSQL URLs, production data, customer data, regulated data, tokens, certificates, or other secrets into feedback.

Use only your own non-production PostgreSQL database for beta testing. Do not use production, UAT, staging, shared, regulated, or customer-data databases. If you need a safe sample database, Pagila is recommended:

```text
https://github.com/devrimgunduz/pagila
```

Feedback channel:

```text
<feedback-channel-to-be-filled-by-Darwin>
```

## Tester

- Name or handle:
- Date:
- Operating system:
- DbState tag or commit tested:
- Native or Docker:
- Browser:

## Environment

- Database tested:
- Database type:
  - Own non-production database
  - Pagila sample database
  - Other disposable local database
- Confirm this was not production, UAT, staging, shared, regulated, or customer-data:
- Workspace path:
- Connection mode:
  - Service environment variable
  - Session URL
  - Non-secret profile plus session password

## Workflow Tested

Check all that apply:

- Fresh Git repository setup
- UI project initialization
- PostgreSQL Inspect Only
- Database to Repository Compare preview
- Database to Repository Compare write
- Repository to Database Compare
- Object Diff review
- Raw Details / Selected JSON Item evidence review
- Release dry-run
- Release artifact generation
- Docker service mode

## Results

- What worked:
- What failed:
- Unexpected behavior:
- UX confusion points:
- Missing object types:
- Object Diff feedback:
- Results grid feedback:
- Release artifact review feedback:
- Raw Details or Selected JSON Item feedback:
- Safety concerns:

## Object Diff Marker Feedback

- Did matched lines appear white:
- Did source-only lines appear green with + marker:
- Did target-only lines appear red with - marker:
- Did target-only lines avoid strike-through:
- Did plus/minus markers appear only as visual indicators:
- Were plus/minus markers absent from generated SQL/repository files:

## Raw Details / Support Evidence

Use Raw Details or Selected JSON Item only as support evidence, troubleshooting context, audit review context, developer handoff material, or PR/change-ticket evidence. Do not treat it as the primary review workflow.

If relevant, include redacted values for:

- objectRef:
- objectType:
- compareClassification:
- planIntent:
- producingWorkflowMode:
- source:
- target:
- sourceType:
- targetType:
- warnings:
- errors:

## Evidence

- Commands run:
- Screenshots attached:
- Logs attached:
- DbState version or tag:
- OS and browser:
- Does any attached material contain secrets, customer data, regulated data, or full PostgreSQL URLs:

If attached material contains secrets, redact it before sending.

## Reproduction Steps

1. 
2. 
3. 

## Priority

Choose one:

- P0 blocks beta use
- P1 important beta issue
- P2 useful improvement
- P3 polish or documentation

## Notes

- 

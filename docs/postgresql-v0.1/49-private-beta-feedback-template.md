# Private Beta Feedback Template

Do not paste passwords, full PostgreSQL URLs, production data, customer data, tokens, certificates, or other secrets into feedback.

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
- Disposable, local development, or approved read-only target:
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
- Database to Repository preview
- Database to Repository write
- Repository to Database Compare
- Object Diff review
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
- Safety concerns:

## Evidence

- Commands run:
- Screenshots attached:
- Logs attached:
- DbState version or tag:
- OS and browser:
- Does any attached material contain secrets:

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

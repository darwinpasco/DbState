# Deployment Artifact Versioning

Generated synchronization scripts are version-controlled artifacts.

Per-object state files remain the source of truth. Generated scripts are not the primary source of truth. They capture how a specific target database can be aligned to the desired state at a point in time.

## Location

Generated scripts should live in a dedicated repository folder. The recommended default is:

```text
database/releases/
```

The exact folder and naming convention are open decisions.

## Naming and metadata

Generated scripts should have stable naming conventions and include non-secret metadata headers.

Headers may include:

- DbState product and version.
- Generation timestamp.
- Source repository branch.
- Source repository commit hash if available.
- Target environment label.
- Target database type.
- Target database version if available.
- Comparison direction.
- Whether destructive changes were detected.
- Whether reference-data changes are included.

## Companion artifacts

Generated scripts should have companion review files where useful:

- Summary markdown.
- Risk JSON.
- Object-level change summary.
- Optional AI-review notes.

## Git behavior

Generated scripts must be visible in Git status and Git diff.

Generated scripts may be staged, committed, and pushed only with explicit user approval.

Generated scripts must not contain secrets, credentials, tokens, connection strings, local paths, unmasked PII, or transactional production data.

## Selective synchronization metadata

Generated deployment artifacts must include selective synchronization metadata where applicable:

- User-selected objects.
- Excluded objects.
- Auto-suggested dependencies.
- Included dependencies.
- Missing dependencies.
- Dependency warnings.
- Explicit overrides.

# PostgreSQL Reference-Data Model

Reference data is explicitly configured. DbState must not treat all table data as version-controlled.

## Repository Structure

```text
database/
  reference-data/
    dbstate.reference-data.yml
    tables/
```

The registry file is:

```text
database/reference-data/dbstate.reference-data.yml
```

Controlled table files live under:

```text
database/reference-data/tables/
```

Table files should use schema-qualified names such as:

```text
database/reference-data/tables/public.payment_methods.yml
```

YAML or JSON should be used as the primary source format. Raw SQL is not the source of truth for reference data.

## Registry Responsibilities

The registry defines:

- Controlled reference tables.
- Business keys.
- Ignored columns.
- Masked columns.
- Compare mode.
- Delete behavior.

## Matching and Compare

Business-key matching identifies rows across source, repository, and target states.

DbState should detect:

- Inserts.
- Updates.
- Deletes where explicitly enabled.

Ignored columns are not compared. Masked columns must not expose sensitive values in reports or generated artifacts.

Delete behavior must be explicit. DbState should not delete reference rows by default without configuration and review.

## DML Generation

Generated reference-data DML should be idempotent where practical and included only for configured reference tables.

Generated DML belongs in synchronization scripts under `database/releases/`, not in source-of-truth reference-data files.

## Safety Rules

Reference-data files must not contain:

- Secrets.
- Credentials.
- Access tokens.
- Connection strings.
- Unmasked PII.
- Transactional production data.

## Example Registry

```yaml
version: 1
tables:
  - name: public.payment_methods
    business_key:
      - code
    compare_mode: full
    delete_behavior: warn
    ignored_columns:
      - updated_at
    masked_columns: []
```

## Example Table File

```yaml
version: 1
table: public.payment_methods
business_key:
  - code
rows:
  - code: card
    display_name: Card
    is_active: true
  - code: cash
    display_name: Cash
    is_active: true
```

## Open Decisions

- YAML only, JSON only, or both.
- Exact registry schema.
- Exact delete behavior values.
- How masking is represented for partial values.
- Whether row ordering is user-defined or canonical.

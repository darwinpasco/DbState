# Slice 35: Semantic Commit Message Generation

## Purpose

Private Beta 2 adds deterministic, offline generation of a Git commit title and body from the files currently staged in the repository index.

DbState uses its knowledge of the repository layout to describe database objects semantically rather than summarizing file names generically.

## Command

```text
dbstate commit-message
```

The default output uses Conventional Commits syntax.

```text
dbstate commit-message --style conventional
dbstate commit-message --style plain
dbstate commit-message --intent "Support customer email verification"
dbstate commit-message --json
```

Running the command again regenerates the message from the current staged state. The command does not retain a previous generated message.

## Scope

The first Private Beta 2 slice analyzes staged files only.

Recognized paths:

- `database/objects/**`
- `database/reference-data/dbstate.reference-data.yml`
- `database/reference-data/tables/**`
- `database/releases/**`

Other staged files are reported as ignored. DbState does not claim to interpret unrelated application source files.

## Generated content

The generator derives:

- added, modified, deleted, and renamed database objects
- schema scope when all analyzed objects belong to one schema
- a Conventional Commit type and breaking-change marker
- a ticket reference from branch names such as `feature/DB-184-customer-verification`
- a semantic bullet list for the commit body

An optional `--intent` value supplies the reason or business intent for the title while DbState continues to generate the technical body from staged changes.

## Breaking-change posture

The beta implementation is deliberately conservative.

The following changes are flagged as potentially breaking:

- deleted DbState-managed objects
- renamed DbState-managed objects
- modified table definitions where a staged diff appears to remove columns
- modified function definitions where the function definition header changes

The generated message uses a Conventional Commits `!` marker and a `BREAKING CHANGE:` section when a potential breaking change is found.

This is advisory analysis. Developers must still review the staged diff and generated message.

## Safety

`dbstate commit-message`:

- does not stage or unstage files
- does not create a Git commit
- does not push or fetch
- does not connect to PostgreSQL
- does not call a hosted AI service
- does not transmit database definitions outside the local machine

## Current limitations

- Only staged changes are supported.
- Selected-file and all-uncommitted scopes are deferred.
- The generator understands DbState-managed paths, not arbitrary source code.
- Rename detection depends on Git rename detection.
- Breaking-change detection is heuristic and intentionally produces potential-risk warnings rather than claiming certainty.
- IDE insertion and browser UI integration are deferred to a later Private Beta 2 slice.

## Private Beta validation questions

Beta feedback should capture:

- whether the generated title is accepted or edited
- whether the generated body is accepted or edited
- whether Conventional Commits or plain style is preferred
- whether ticket inference is accurate
- whether breaking-change warnings are trusted
- whether users supply `--intent`
- whether users want direct insertion into their Git client or IDE

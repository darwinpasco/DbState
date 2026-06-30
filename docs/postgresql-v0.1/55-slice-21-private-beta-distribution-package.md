# Slice 21 Private Beta Distribution Package

Slice 21 adds a repeatable local package assembly step for the already-tagged DbState PostgreSQL `v0.1.0-private-beta.1` private beta.

This slice is documentation and distribution packaging only. It does not add product behavior.

## Purpose

The distribution package helps Darwin send testers the right files without exposing source code, Git history, Cargo project files, tests, internal development docs, local workspaces, connection profiles, database data, or secrets.

DbState remains local-first and safety-first:

- DbState does not execute SQL.
- DbState does not execute generated SQL.
- DbState does not apply changes to a database.
- DbState does not mutate PostgreSQL.
- DbState does not auto-stage, commit, push, pull, fetch, or tag Git changes.
- DbState does not persist passwords.
- DbState does not persist full PostgreSQL URLs.

## Packaging Script

```text
packaging/windows/Build-PrivateBetaPackage.ps1
```

The script:

1. Accepts a private beta version.
2. Accepts a path to an existing Windows installer.
3. Creates a clean versioned output folder.
4. Copies the installer with a private-beta filename.
5. Generates a SHA256 hash file.
6. Copies only approved tester-facing docs.
7. Generates `README-private-beta.md`.
8. Validates that forbidden files and Drive D paths are not present.

It does not build source code, upload artifacts, sign binaries, create tags, or modify product behavior.

## Command

From the repository root:

```powershell
cd C:\SourceCodes\DbState

.\packaging\windows\Build-PrivateBetaPackage.ps1 `
  -Version v0.1.0-private-beta.1 `
  -InstallerPath .\packaging\windows\out\DbState-PostgreSQL-v0.1.0-alpha.3-setup.exe
```

Optional output root:

```powershell
.\packaging\windows\Build-PrivateBetaPackage.ps1 `
  -Version v0.1.0-private-beta.1 `
  -InstallerPath .\packaging\windows\out\DbState-PostgreSQL-v0.1.0-alpha.3-setup.exe `
  -OutputRoot dist/private-beta
```

If the installer does not exist, the script fails cleanly with an actionable message.

## Output Folder

Recommended layout:

```text
dist/private-beta/v0.1.0-private-beta.1/
  DbState-PostgreSQL-v0.1.0-private-beta.1-setup.exe
  DbState-PostgreSQL-v0.1.0-private-beta.1-setup.exe.sha256.txt
  README-private-beta.md
  docs/
    48-slice-18-golden-path-private-beta-walkthrough.md
    49-private-beta-feedback-template.md
    52-private-beta-known-limitations.md
    53-private-beta-installer-distribution.md
    54-private-beta-smoke-test-matrix.md
```

Generated `dist/` output is ignored by Git.

## Package Contents

The package should contain:

- Windows installer `.exe`
- SHA256 hash text file
- private beta readme
- golden-path walkthrough
- known limitations
- installer distribution instructions
- smoke test matrix
- feedback template

The package must not contain:

- source code
- `.git`
- `src`
- `Cargo.toml`
- `Cargo.lock`
- `tests`
- internal development docs not intended for testers
- local workspaces
- connection profiles
- database data
- tester workspace release artifacts
- secrets
- passwords
- full PostgreSQL URLs, except local disposable sample URLs already documented for ParkingDemo

## Validation Guard

The script fails if the distribution folder contains:

- `.git`
- `src`
- `Cargo.toml`
- `Cargo.lock`
- `tests`
- `.rs`
- `.ps1`
- `.env`
- `connection-profiles.json`
- `target`
- `packaging/windows/staging`
- tester-facing Drive D paths

Use this manual inspection if needed:

```powershell
Get-ChildItem dist\private-beta\v0.1.0-private-beta.1 -Recurse -Force
```

## Hash Verification

The script writes:

```text
DbState-PostgreSQL-v0.1.0-private-beta.1-setup.exe.sha256.txt
```

Manual hash command:

```powershell
Get-FileHash dist\private-beta\v0.1.0-private-beta.1\DbState-PostgreSQL-v0.1.0-private-beta.1-setup.exe -Algorithm SHA256
```

Send the SHA256 hash with the installer so testers can verify the received file.

## What To Send Testers

Send the versioned distribution folder contents:

- installer `.exe`
- SHA256 hash
- `README-private-beta.md`
- `docs/` folder

Use a private file transfer channel. Do not post the package publicly.

## What Not To Send Testers

Do not send:

- source repository
- Git history
- Cargo files
- tests
- local developer workspaces
- connection profiles
- database data
- secrets
- raw production logs
- full PostgreSQL URLs

## Post-Package Smoke Checklist

1. Confirm the package folder name is `v0.1.0-private-beta.1`.
2. Confirm the installer has the private beta filename.
3. Confirm the SHA256 file exists.
4. Confirm `README-private-beta.md` exists.
5. Confirm only approved docs are present.
6. Confirm no source files are present.
7. Confirm no Drive D paths appear in tester-facing files.
8. Install from the package installer.
9. Start DbState Local Service.
10. Open `http://127.0.0.1:4587/`.
11. Run the health endpoint.
12. Follow the ParkingDemo inspect path.
13. Uninstall and confirm installed files are removed.

## Tester Instructions

Tester-facing paths use Drive C:

```text
C:\Program Files\DbState
C:\DbState\ParkingDemo
C:\SourceCodes\DbState
```

`C:\SourceCodes\DbState` appears only in source-build or developer-path instructions.

## Safety Boundaries

The distribution package does not change DbState behavior.

It does not add:

- direct database apply
- SQL execution
- generated SQL execution
- PostgreSQL mutation
- destructive SQL generation
- DML generation
- Git automation
- hosted service mode
- telemetry
- auto-update
- authentication
- license activation
- AI integration
- MCP server
- CI workflow
- Docker Compose
- new RDBMS support
- frontend framework or Node build pipeline
- secret persistence
- full PostgreSQL URL persistence

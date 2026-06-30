# Private Beta Installer Distribution

This document describes how to build and distribute the DbState PostgreSQL Windows private beta installer.

## Build From Clean dev

Prerequisites:

- Windows PowerShell
- Rust toolchain
- Inno Setup 6
- clean `dev` branch

Commands:

```powershell
cd C:\SourceCodes\DbState
git switch dev
git pull origin dev
git status --short --branch --untracked-files=all

cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build
cargo build --release

.\packaging\windows\Build-WindowsInstaller.ps1
```

Expected Slice 19 installer name:

```text
DbState-PostgreSQL-v0.1.0-alpha.3-setup.exe
```

After the private beta tag is created, future packages may use a private-beta versioned name.

Generated installer location:

```text
packaging/windows/out/
```

Do not commit generated installer binaries.

## Checksum

Generate a SHA256 hash:

```powershell
Get-FileHash packaging\windows\out\DbState-PostgreSQL-v0.1.0-alpha.3-setup.exe -Algorithm SHA256
```

Send the hash with the installer so testers can verify the file they received.

## What To Send To Testers

Send:

- installer `.exe`
- SHA256 hash
- golden path walkthrough
- known limitations
- smoke test matrix, if the tester is doing structured validation
- feedback template
- feedback channel:
  - `<feedback-channel-to-be-filled-by-Darwin>`

Do not send:

- source code
- Git repository
- Cargo files
- internal docs not intended for testers
- secrets
- connection profiles
- database data
- local workspace files

## Install Instructions For Testers

1. Run the installer.
2. Use the default install directory:

```text
C:\Program Files\DbState
```

3. Start `Start DbState Local Service` from the Start Menu.
4. Open:

```text
http://127.0.0.1:4587/
```

5. Follow the ParkingDemo walkthrough with:

```text
C:\DbState\ParkingDemo
```

DbState must be tested only against disposable or explicitly approved read-only PostgreSQL targets.

## Uninstall Instructions

Use Windows Apps or Control Panel.

Expected behavior:

- installed application files are removed
- `%APPDATA%\DbState` may remain if non-secret connection profiles were created
- `%APPDATA%\DbState` must not contain passwords or full PostgreSQL URLs

## Installed Payload Expectations

Installed files should be limited to:

- `dbstate.exe`
- `README.txt`
- `VERSION.txt`
- `LICENSE.txt`, if available

The installer must not include:

- `src\`
- `.git\`
- `Cargo.toml`
- `Cargo.lock`
- `tests\`
- Rust source files
- internal development docs
- connection profiles
- local workspaces
- generated release artifacts
- secrets

## Private Distribution Guidance

- Use a private file transfer channel.
- Send the SHA256 hash separately or adjacent to the installer.
- Keep a record of the exact tag or commit used.
- Do not post the installer publicly.
- Ask testers to confirm the installed binary version and OS.
- Ask testers not to use production, UAT, staging, or shared databases.

## Support Instructions

For support requests, ask testers for:

- DbState tag or installer filename
- Windows version
- browser
- whether they used installer, Docker, or developer path
- command output
- screenshots where useful
- redacted logs
- exact reproduction steps

Do not accept passwords, full PostgreSQL URLs, production data, customer data, tokens, certificates, or secrets in support material.

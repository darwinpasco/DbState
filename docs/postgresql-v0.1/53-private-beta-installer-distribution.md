# DbState PostgreSQL v0.1.0 Private Beta 2 Installer Distribution

This document describes how to build and distribute the DbState PostgreSQL v0.1.0 Private Beta 2 Windows installer.

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

If `cargo test` is blocked on a Windows validation machine by Smart App Control or another Application Control policy for generated Rust test binaries, record that as an environment policy block, not a Rust compilation failure. The observed Windows policy error is:

```text
An Application Control policy has blocked this file. (os error 4551)
```

Do not weaken DbState validation because of this. Run the remaining checks, validate on an environment that can execute Rust test binaries when available, and record the exact policy block in the release notes or validation notes.

Expected Private Beta 2 distributed installer name:

```text
DbState-PostgreSQL-v0.1.0-private-beta.2-setup.exe
```

Private Beta 2 tag:

```text
v0.1.0-private-beta.2
```

Private Beta 2 package folder:

```text
dist/private-beta/v0.1.0-private-beta.2
```

Private Beta 2 SHA256:

```text
05BEC187D0D08CF3B9B54B3EB68C91E4DF09F7A29E866CC039D3CCD36CBE5CC3
```

Generated installer location:

```text
packaging/windows/out/
```

Do not commit generated installer binaries.

## Checksum

Generate a SHA256 hash:

```powershell
Get-FileHash packaging\windows\out\DbState-PostgreSQL-v0.1.0-private-beta.2-setup.exe -Algorithm SHA256
```

Send the hash with the installer so testers can verify the file they received.

For the released Private Beta 2 package, the distributed installer is:

```text
DbState-PostgreSQL-v0.1.0-private-beta.2-setup.exe
```

Expected SHA256:

```text
05BEC187D0D08CF3B9B54B3EB68C91E4DF09F7A29E866CC039D3CCD36CBE5CC3
```

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

The distribution assembly script creates a versioned folder with the installer, SHA256 file, private beta readme, and approved tester-facing docs:

```powershell
.\packaging\windows\Build-PrivateBetaPackage.ps1 `
  -Version v0.1.0-private-beta.2 `
  -InstallerPath .\packaging\windows\out\DbState-PostgreSQL-v0.1.0-private-beta.2-setup.exe
```

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

5. Follow the Private Beta 2 walkthrough with a fresh local workspace:

```text
C:\DbState\PrivateBetaDemo
```

DbState must be tested only against the tester's own non-production PostgreSQL database. Do not use production, UAT, staging, shared, regulated, or customer-data databases. If the tester needs a sample database, recommend Pagila:

```text
https://github.com/devrimgunduz/pagila
```

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
- Ask testers not to use production, UAT, staging, shared, regulated, or customer-data databases.
- Recommend Pagila if testers need a safe sample database.

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

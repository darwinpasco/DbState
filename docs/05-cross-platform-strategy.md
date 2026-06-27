# Cross-Platform Strategy

DbState must be cross-platform by architecture and must ship as separate native builds per platform.

## Required model

- One shared product codebase where practical.
- One shared deterministic core engine.
- One shared project and repository format.
- One shared CLI behavior.
- Separate native builds for Windows, macOS, and Linux.
- Separate native CLI binaries for Windows, macOS, and Linux.
- Docker image for headless automation, CI/CD, scheduled drift checks, and server-style runtime.

The browser UI is the presentation layer. DbState Service and the CLI should have native builds for Windows, macOS, and Linux.

The same core engine should power DbState Service, browser UI workflows through the service, CLI, Docker, and future MCP workflows.

## What DbState is not

DbState should not be:

- One browser-only SaaS app as the primary product.
- One lowest-common-denominator desktop build with no native packaging.
- Separate products per operating system.
- Docker as the desktop product.
- Windows, macOS, and Linux as separate product lines.

The product line should split by RDBMS, not by operating system.

## Distribution direction

Target distribution strategy:

- Windows native build.
- macOS native build.
- Linux native build.
- Windows CLI binary.
- macOS CLI binary.
- Linux CLI binary.
- Docker image for Linux-based automation.

Future packaging options may include Windows `.exe`, Windows `.msi`, macOS `.dmg`, Linux AppImage, Linux `.deb`, Linux `.rpm`, Homebrew, Winget, Chocolatey, and Docker images for `linux/amd64` and `linux/arm64`.

These packaging formats are future options, not MVP requirements.

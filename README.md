<p align="center">
  <img src="https://raw.githubusercontent.com/Tarikul-Islam-Anik/Animated-Fluent-Emojis/master/Emojis/Travel%20and%20places/Ringed%20Planet.png" width="160" alt="Aurorium" />
</p>

<h1 align="center">Aurorium</h1>

<p align="center">
  <img src="https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/license-CC%20BY--NC--SA%204.0-lightgrey.svg?style=for-the-badge" alt="License" />
  <img src="https://img.shields.io/discord/940647911182729257?color=5865F2&label=Discord&logo=discord&logoColor=white&style=for-the-badge" alt="Discord" />
</p>

> [!IMPORTANT]
> **Disclaimer:** We are not affiliated with Wizard101Rewritten in any way and do not tolerate any use of this project in reference to Wizard101Rewritten!

## Table of Contents

- [Introduction](#-introduction)
- [What's New in the Rework](#-whats-new-in-the-rework)
- [Architecture](#-architecture)
- [Getting Started](#-getting-started)
- [Configuration](#-configuration)
- [HTTP API](#-http-api)
- [Migrating from v3.x](#-migrating-from-v3x)
- [Contributing](#-contributing)
- [Community](#-community)
- [License](#-license)

---

## Introduction

Aurorium is the backbone of the Revive101 project: it fetches, tracks, and serves the game assets that the Wizard101 client needs to patch and run. Our goal is to keep this process open and easy for the community to run, inspect, and contribute to.

## What's New in the Rework

The `rework` branch is a substantial rewrite of Aurorium, moving it from a CLI-driven downloader into a persistent service with its own file server and database:

- **Built-in HTTP server (axum)**: Aurorium now has an additional route called `/latest` which is updated every $n$ hours.
- **SQLite-backed asset tracking**: Revisions and assets are tracked in a proper database (via `rusqlite` + `async-sqlite`, with schema migrations), replacing the old flat-file bookkeeping.
- **Smarter asset resolution**: Because assets are deduplicated and indexed by name/CRC/size, unchanged files aren't re-downloaded or re-stored between revisions, and requests are resolved to whichever revision actually owns the file.
- **TOML configuration file**: Runtime settings now live in `config.toml` instead of CLI flags/environment variables, and a default one is generated for you on first run.
- **Structured logging**: Switched to `tracing`/`tracing-subscriber`, with optional file logging and configurable log levels.
- **Friendlier errors**: Errors are now reported via `miette` with human-readable diagnostics and hints instead of bare error messages.

## Architecture

Aurorium runs two tasks concurrently:

1. **Revision checker**: periodically polls the configured patch server, compares the latest manifest against what's already in the database, and downloads only new or changed assets into `save_directory`.
2. **File server**: an axum-based HTTP server that exposes the tracked revisions and assets to clients (e.g. a patched Wizard101 client, or downstream tooling).

Both share the same `AppState` (config + database handle), so newly fetched assets become servable as soon as they land on disk.

## Getting Started

> [!NOTE]
> If you are **not** a developer, **you can skip to the [releases page](https://github.com/Revive101/Aurorium/releases/latest)** and download the latest version directly.

### Prerequisites

- [Rust](https://www.rust-lang.org/) (edition 2024 toolchain)
- A code editor like [VS Code](https://code.visualstudio.com/)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) extension

### Installation

```bash
git clone https://github.com/Revive101/Aurorium.git
cd Aurorium
```

### Running

```bash
cargo run                # Debug
cargo build --release    # Optimized build
```

On first launch, if no `config.toml` is found in the working directory, Aurorium generates one with sensible defaults (see [Configuration](#-configuration)).

### Common Errors

**`link.exe not found` (Windows):**

1. Install Microsoft C++ Build Tools
2. run:

```bash
rustup toolchain install stable-x86_64-pc-windows-msvc
rustup default stable-x86_64-pc-windows-msvc
```

**On Linux:** use target `x86_64-unknown-linux-gnu`.

## Configuration

Aurorium is now configured entirely through `config.toml` in the working directory:

```toml
[server]
endpoint = "127.0.0.1:12369"

[fetcher]
concurrent_downloads = 2
save_directory = "data"
fetch_interval = 28800

[patch]
host = "patch.us.wizard101.com"
port = "12500"

[database]
path = "aurorium.db"

# Optional
[debug]
level = "info"
file_logging = false
```

| Section              | Field                  | Description                                           | Default                  |
| -------------------- | ---------------------- | ----------------------------------------------------- | ------------------------ |
| `[server]`           | `endpoint`             | Address the file server binds to                      | `127.0.0.1:12369`        |
| `[fetcher]`          | `concurrent_downloads` | Number of assets to download in parallel              | `2`                      |
| `[fetcher]`          | `save_directory`       | Where fetched assets are stored on disk               | `data`                   |
| `[fetcher]`          | `fetch_interval`       | Seconds between revision checks                       | `28800` (8 hours)        |
| `[patch]`            | `host`                 | Patch server host to poll for revisions               | `patch.us.wizard101.com` |
| `[patch]`            | `port`                 | Patch server port                                     | `12500`                  |
| `[database]`         | `path`                 | Path to the SQLite database file                      | `aurorium.db`            |
| `[debug]` (optional) | `level`                | Log level (`trace`, `debug`, `info`, `warn`, `error`) | `info`                   |
| `[debug]` (optional) | `file_logging`         | Whether to also write logs to `logs/`                 | `false`                  |

## HTTP API

Once running, Aurorium exposes:

| Method | Route                     | Description                                                        |
| ------ | ------------------------- | ------------------------------------------------------------------ |
| `GET`  | `/revisions`              | Lists all revisions currently tracked in the database (JSON)       |
| `GET`  | `/latest`                 | Returns the name of the most recently tracked revision             |
| `GET`  | `/{revision}/{file_path}` | Serves a specific asset, resolving it to the revision that owns it |

`LatestFileList.xml`/`.bin` are always served from the requested revision directly; any other file is resolved to whichever revision first introduced it, so unchanged assets aren't duplicated on disk.

## Migrating from v3.x

The rework branch changes enough that a v3.x setup can't be dropped in as-is:

- Configuration moved from CLI flags/env vars to `config.toml` — recreate your settings there (see table above).
- A SQLite database (`aurorium.db` by default) now tracks revisions/assets; it's created and migrated automatically on first run, but existing on-disk data from v3.x isn't imported.
- Aurorium now also serves files over HTTP itself, so downstream consumers can point at `/{revision}/{file_path}` instead of reading straight off disk.

If you're upgrading a running instance, we recommend starting from a fresh `save_directory` and database rather than trying to reuse v3.x state.

## Contributing

We welcome all contributions! Whether you're a Rust wizard or a curious apprentice, your input helps!

- 🍴 Fork the repo, make your changes on top of `rework`, and submit a pull request.
- 🐛 Report bugs or suggest features via [issues](https://github.com/Revive101/Aurorium/issues).

> [!NOTE]
> Contributors can request the `@Contributor` role in our [Discord](https://discord.gg/sMFgyNRDDM). Make sure your GitHub is linked to your Discord account.

## Community

Join us on [Discord](https://discord.gg/sMFgyNRDDM) to meet other fans, developers, and contributors!

## License

[Aurorium](https://github.com/Revive101/Aurorium) by [Phill030](https://github.com/Phill030/) is licensed under [CC BY-NC-SA 4.0](http://creativecommons.org/licenses/by-nc-sa/4.0/?ref=chooser-v1).

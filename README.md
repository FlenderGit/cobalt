# Cobalt

[![Crates.io](https://img.shields.io/crates/v/cobalt-server)](https://crates.io/crates/cobalt-server)
[![Docs.rs](https://docs.rs/cobalt-server/badge.svg)](https://docs.rs/cobalt-server)
[![CI](https://github.com/FlenderGit/cobalt/actions/workflows/ci.yml/badge.svg)](https://github.com/FlenderGit/cobalt/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange)](https://www.rust-lang.org)

> A high-performance, from-scratch Minecraft: Java Edition server implementation written in Rust.

---

## Table of Contents

- [Overview](#overview)
- [Goals](#goals)
- [Requirements](#requirements)
- [Installation](#installation)
- [Configuration](#configuration)
- [Todo](#todo)
- [License](#license)

---

## Overview

Cobalt is an independent Minecraft: Java Edition server implementation written entirely in Rust, built from the ground up without relying on Mojang's codebase. It targets all major versions of the Java Edition protocol and is designed to be a reliable, resource-efficient alternative to the official server and its derivatives.

At its core, Cobalt separates the network layer, the game logic, and the persistence layer into clearly defined modules. Extensibility is provided through a **plugin system**, allowing plugins to be written in any language that compiles to WASM, sandboxed from the host process.

> Cobalt is in active development. Breaking changes may occur between releases.

---

## Goals

Cobalt is designed with the following principles in mind:

- **High performance** — minimal memory footprint and CPU overhead, even under high player load, by leveraging Rust's zero-cost abstractions and async I/O.
- **Protocol completeness** — full coverage of the Minecraft: Java Edition protocol across all supported versions, with no reliance on Mojang's server jar.
- **Extensibility** — a first-class WebAssembly plugin API, enabling plugins written in any WASM-compatible language (Rust, Go, C, etc.) to hook into server events safely and efficiently.
- **Observability** — built-in support for Prometheus metrics to monitor server health, player activity, and performance in production environments.
- **Configurability** — all major server parameters are exposed through a structured TOML configuration file, with environment variable overrides for containerised deployments.
- **Developer-friendly** — clean, well-documented code with a consistent architecture that makes it straightforward to contribute, extend, or embed Cobalt in a larger system.

---

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) >= 1.75 (stable toolchain)
- OpenSSL (for authentication and encryption)

---

## Installation

### Prebuilt binary

Prebuilt binaries for Linux, macOS, and Windows are available on the [Releases](https://github.com/FlenderGit/cobalt/releases) page.

### From source

```bash
git clone https://github.com/FlenderGit/cobalt.git
cd cobalt
cargo build --release
```

The binary will be available at `./target/release/cobalt`.

---

## Configuration

Cobalt is configured via a TOML file. By default, it looks for `config.toml` in the current working directory. A different path can be specified with the `--config` flag.

```toml
[network]
addr      = "0.0.0.0:25565"
threshold = 256              # Compression threshold (bytes)

[profile]
name        = "Cobalt Server"
description = "A Cobalt-powered Minecraft server"
max_players = 100
icon        = "./res/favicon.png"   # 64x64 PNG, or null
gamemode    = "survival"            # survival | creative | adventure | spectator
dimension   = "overworld"           # overworld | nether | end
difficulty  = "normal"              # peaceful | easy | normal | hard

[auth]
enabled     = true
private_key = "./res/server_key.pem"
public_key  = "./res/public_key.der"
```

A complete example configuration is available in [`examples/config.toml`](./examples/config.toml).

Environment variables take precedence over values defined in the configuration file.

---

## Todo

The following features are planned or in progress:

- [ ] Event system and WebAssembly plugin API
- [ ] Storage backends — configurable via `config.toml`:
  - [ ] TOML (flat file)
  - [ ] SQLite
  - [ ] MySQL / PostgreSQL
- [ ] Prometheus metrics endpoint
- [ ] Full Clippy compliance and CI lint enforcement
- [ ] Multi-version protocol support

---

## License

This project is licensed under the MIT License. See the [LICENSE](./LICENSE) file for details.

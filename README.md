# HomematicIP plugins in Rust

A Rust toolkit and a collection of plugins for the **HomematicIP Home Control Unit (HCU)**.

The HCU can be extended with *plugins*: small programs that run in a container on the HCU (or elsewhere in your network), talk to it over the HomematicIP **Connect API** and bring third-party devices, configuration options and messages into the Homematic IP system. This repository contains

- **`hmip-rs`** – a Rust crate with everything needed to write such plugins: the complete Connect API message and device model, and a base plugin implementation that takes care of the connection, authentication and message handling;
- **ready-to-use plugins** built on that crate;
- the **tooling** to compile each plugin into a single, tiny static binary and ship it as the container archive (`.tar.gz`) that HCUweb installs.

> **Disclaimer:** This project is independently developed and is not associated with, endorsed by, or sponsored by Homematic / eQ-3 AG. For official information, visit their GitHub repositories at <https://github.com/homematicip>.

## What's inside

| | |
|---|---|
| [`crates/hmip-rs`](crates/hmip-rs) | The library crate, published on [crates.io](https://crates.io/crates/hmip-rs) and documented on [docs.rs](https://docs.rs/hmip-rs). **[Read the crate documentation →](crates/hmip-rs/README.md)** |
| [`plugins/`](plugins) | One folder per plugin, see the list below. |
| [`docker/`](docker), [`scripts/`](scripts) | Generic Dockerfile and the `build-plugin.sh` script that turn a plugin into an installable image archive. |

## Plugins

| Plugin | What it does | Documentation |
|---|---|---|
| **Fronius** | Shows a Fronius SYMO GEN24 solar system – production, battery, grid connection and house consumption – as devices in your Homematic IP system. | **[Fronius plugin guide →](plugins/fronius/README.md)** |

Installable plugin archives are published on the [releases page](https://github.com/mburtscher/HomematicIP.Net/releases) (tags like `fronius-v0.1.0`).

## Building a plugin

You need Docker (with buildx), a Rust toolchain and `jq`.

```bash
scripts/build-plugin.sh --list                    # available plugins
scripts/build-plugin.sh fronius                   # -> dist/<plugin-id>-<version>.tar.gz
scripts/build-plugin.sh fronius --version 1.2.3   # override the version from Cargo.toml
scripts/build-plugin.sh --all
```

The script cross-compiles the plugin to a static `aarch64-unknown-linux-musl` binary (size-optimized, LTO, stripped), places it alone in a `scratch` image tagged `<plugin-id>:<version>` with the `de.eq3.hmip.plugin.metadata` label the HCU requires, and exports the image as `.tar.gz` for upload in HCUweb.

## Writing your own plugin

1. Create `plugins/<name>/` with a `Cargo.toml` (package name = `<name>`, `publish = false`, dependency `hmip-rs.workspace = true`) and a `src/main.rs`. Plugins in this repository build against the library sources.
2. Describe the plugin's HCU metadata in `[package.metadata.hcu]` (see [`plugins/fronius/Cargo.toml`](plugins/fronius/Cargo.toml)): `plugin-id`, `issuer`, `hcu-min-version`, `scope`, `friendly-name` (needs a `de` entry), and optionally `description`, `settings`, `image`, `changelog`, `logs-enabled`.
3. Implement the [`Handler`](https://docs.rs/hmip-rs/latest/hmip_rs/trait.Handler.html) for the messages your plugin cares about – the [crate documentation](crates/hmip-rs/README.md) walks through devices, configuration, plugin state and app messages.
4. `scripts/build-plugin.sh <name>`.

Outside this repository, just `cargo add hmip-rs` and build your plugin however you like.

## Development

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

Releases are driven by GitHub release tags: `<plugin>-v1.2.3` builds that plugin's archive and attaches it to the release, `hmip-rs-v1.2.3` publishes the crate to crates.io.

## License

Everything in this repository is licensed under the **MIT License**. See [LICENSE](LICENSE).

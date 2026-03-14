# Installation

## Prerequisites

- **Rust 1.87+** — install via [rustup](https://rustup.rs/)
- **An Anthropic API key** — get one at [console.anthropic.com](https://console.anthropic.com/)

## Install from Source

```bash
git clone https://github.com/gabrieleventuri/orion-rs.git
cd orion-rs
cargo install --path crates/orion --locked
```

This installs the `orion` binary to your Cargo bin directory (usually `~/.cargo/bin/`).

## Verify

```bash
orion --help
```

## Set Your API Key

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

Or add it to your project config after [initialization](/getting-started/initialization):

```toml
[providers.anthropic]
api_key = "sk-ant-..."
```

::: tip
The environment variable is checked as a fallback. If both are set, the config file value takes priority.
:::

## Next Steps

Head to [Project Setup](/getting-started/initialization) to initialize Orion in your project directory.

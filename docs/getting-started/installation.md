# Installation

## Prerequisites

- **An LLM provider** — Anthropic API key ([console.anthropic.com](https://console.anthropic.com/)), AWS Bedrock credentials, Google Vertex AI project, or any other [supported provider](/getting-started/configuration#provider-options)

## Recommended

```bash
curl -fsSL https://starpod.sh/install | sh
```

This detects your OS and architecture, downloads the latest pre-built binary, and installs it to `~/.local/bin`.

Options:
- `--version=0.1.7` — pin to a specific version
- `--no-homebrew` — skip Homebrew on macOS
- `INSTALL_DIR=/usr/local/bin` — override install location

## Homebrew (macOS/Linux)

```bash
brew install sinaptik-ai/tap/starpod
```

## From crates.io

Requires **Rust 1.87+** ([rustup](https://rustup.rs/)).

```bash
cargo install starpod
```

## From Source

```bash
git clone https://github.com/sinaptik-ai/starpod.git
cd starpod
cargo install --path crates/starpod --locked
```

## Verify

```bash
starpod --help
```

## Set Your API Key

Seed your API key into the vault during initialization:

```bash
starpod init --env ANTHROPIC_API_KEY="sk-ant-..."
```

Or manage it later via the web UI Settings page after running `starpod dev`.

::: tip
API keys are stored in the encrypted vault — never in config files or `.env` files. The vault injects them into the process environment at startup.
:::

## Next Steps

Head to [Project Setup](/getting-started/initialization) to initialize Starpod in your project directory.

# OtterShipper

[![Test](https://github.com/ottershipper/ottershipper/actions/workflows/test.yml/badge.svg)](https://github.com/ottershipper/ottershipper/actions/workflows/test.yml)

AI-first deployment platform controlled via Claude Code through MCP protocol.

## Installation

**Quick Install (Ubuntu 24.04):**

```bash
curl -sSL https://raw.githubusercontent.com/ottershipper/ottershipper/main/install/install.sh | sudo bash
```

For detailed installation instructions, manual setup, configuration options, and troubleshooting, see [.claude/INSTALL.md](.claude/INSTALL.md).

## Project Structure

```
ottershipper/
├── crates/
│   ├── server/    # Main binary (MCP server + schemas)
│   ├── core/      # Business logic (ApplicationService, etc)
│   └── db/        # Database models & repository
└── Cargo.toml     # Workspace root
```

## Development

```bash
# Build all crates
cargo build

# Run server
cargo run --bin ottershipper

# Run tests
cargo test

# Test installation locally (before commit)
# Requires: Docker
./test/test-install-local.sh

# Show verbose output (all build logs)
./test/test-install-local.sh --verbose
```

## License

MIT

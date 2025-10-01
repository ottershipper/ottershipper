# OtterShipper

AI-first deployment platform controlled via Claude Code through MCP protocol.

## Project Structure

```
ottershipper/
├── crates/
│   ├── server/    # Main binary (MCP server)
│   ├── mcp/       # MCP protocol handler
│   ├── core/      # Business logic (DeploymentService, etc)
│   └── db/        # Database models & migrations
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
```

## License

MIT

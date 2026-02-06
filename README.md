# Faux v2.0

Code for backend server and user client app, monorepo written in Rust.

## Commands
```bash
# App (client)
cargo run
cargo watch -c -w src -x run

# Build (client)
cargo build -p faux_v2 --release

# Server
cargo run -p faux_server
cargo watch -w server -x "run -p faux_server"

# Database (server)
cargo run -p faux_server -- migrate
cargo run -p faux_server -- seed
cargo run -p faux_server -- reset
```

## Build Output
Release builds go into `target/release`. Bundles (when using `cargo-bundle`) go into `target/release/bundle`.

- Windows: `target\release\faux_v2.exe`
- macOS: `target/release/faux_v2` (binary) or `target/release/bundle/osx/Faux.app` (cargo-bundle)
- Linux: `target/release/faux_v2` (binary) or `target/release/bundle` (cargo-bundle)

## Envirement
Use `.env` files for local configuration.
- Client: `.env` (see `.env.example`)
- Server: `server/.env` (see `server/.env.example`)

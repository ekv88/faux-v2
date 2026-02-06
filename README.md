# Faux V2 (Local Dev)

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

## Env
- Client: `.env` (see `.env.example`)
- Server: `server/.env` (see `server/.env.example`)

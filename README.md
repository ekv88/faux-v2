# Faux V2 (Local Dev)

## Prereqs
- Rust toolchain
- MySQL running locally

## App (client)
Run once:
```bash
cargo run
```

Watch + auto-rebuild:
```bash
cargo watch -c -w src -x run
```

## Server
Run once:
```bash
cargo run -p faux_server
```

Watch + auto-rebuild:
```bash
cargo watch -w server -x "run -p faux_server"
```

## Database (server)
Migrate:
```bash
cargo run -p faux_server -- migrate
```

Seed:
```bash
cargo run -p faux_server -- seed
```

Reset (drop + recreate + migrate + seed):
```bash
cargo run -p faux_server -- reset
```

## Env
- Client: `.env` (see `.env.example`)
- Server: `server/.env` (see `server/.env.example`)

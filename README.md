# bot-tg-aptos-faucet

A Telegram bot that dispenses 0.5 Aptos Testnet APT per user per hour.

This repository contains two functionally equivalent implementations:

- [`typescript/`](./typescript) — Original implementation using **TypeScript**, `grammy`, `@aptos-labs/ts-sdk`, `better-sqlite3`, and Koa.
- [`rust/`](./rust) — New implementation using **Rust**, `teloxide`, `aptos-sdk`, `rusqlite`, and `axum`.

Both support:

- `/start` — welcome message
- `/help` — show usage help
- `/faucet <address>` — send 0.5 testnet APT (rate-limited to once per hour per user)
- `/language <locale>` — switch language (`en`, `vi`, `zh`)
- Inline "Delete this message" button for group chats
- Long-polling and webhook modes
- SQLite storage for per-user last-call time and language preference

## Configuration

Copy the example environment file in the implementation you want to run and fill in the values.

```bash
# TypeScript
cp typescript/.env.example typescript/.env

# Rust
cp rust/.env.example rust/.env
```

Required variables:

| Variable | Description |
| --- | --- |
| `TG_BOT_API_KEY` | Telegram Bot API token from [@BotFather](https://t.me/BotFather) |
| `FAUCET_PRIVATE_KEY` | Hex private key of the Aptos account that will send tokens |
| `USE_WEBHOOK` | `true` to use webhooks, `false` for long-polling |
| `WEBHOOK_URL` | Public webhook URL (required when `USE_WEBHOOK=true`) |
| `DB_PATH` | SQLite database path (optional, defaults to `data/faucet.db`) |

## Running the TypeScript version

```bash
cd typescript
pnpm install
pnpm build
pnpm start
```

## Running the Rust version

```bash
cd rust
cargo run --release
```

## Project layout

```
.
├── README.md
├── .gitignore
├── typescript/
│   ├── src/              # grammy bot source
│   ├── locales/          # Fluent / FTL localization files
│   ├── package.json
│   ├── tsconfig.json
│   └── .env.example
└── rust/
    ├── src/              # teloxide bot source
    ├── locales/          # copied FTL localization files
    ├── Cargo.toml
    └── .env.example
```

## Notes

- Both implementations target **Aptos Testnet**.
- The Rust version reuses the same `.ftl` localization files as the TypeScript version.
- The Rust Aptos SDK is used with the `ed25519` feature for legacy Ed25519 accounts, matching the TypeScript SDK default.

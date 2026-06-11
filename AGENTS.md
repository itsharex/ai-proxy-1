# Repository Guidelines

## Project Structure & Module Organization

A **Tauri 2** desktop app (Rust backend + Vue 3 frontend) that acts as a local AI/LLM proxy, unifying multiple provider APIs.

```
src/                        # Vue 3 frontend (Vite + TypeScript)
  views/                    # Page components (Dashboard, Providers, Models, Rules, Logs, Statistics, Settings, Apps)
  api/index.ts              # Frontend API client
  types/index.ts            # Shared TypeScript types
  stores/                   # Pinia stores
  router.ts                 # Vue Router config

src-tauri/                  # Rust backend (Axum server embedded in Tauri)
  src/
    lib.rs                  # App entry, tray, proxy lifecycle
    server/                 # HTTP server: handlers, router, API endpoints, middleware
    converter/              # Format conversion via IR layer
      parsers/              # Parse Completions / Responses / Anthropic / Gemini → IR
      generators/           # Generate from IR → each provider format
    provider/               # Provider management & model routing
    key/                    # API Key encrypted storage & rotation (AES-256-GCM)
    interceptor/            # Rule engine for request/response interception
    usage/                  # Token usage tracking & cost estimation
    logging/                # Request logging store
    db/                     # SQLite via SQLx, connection pool, migrations
    apps/                   # App management (Codex CLI, Claude Desktop, etc.)
  migrations/               # Numbered SQLite migration scripts (001_init.sql …)
  tests/                    # Rust integration tests (e.g. ir_conversion.rs)
```

## Build, Test, and Development Commands

| Command | Description |
|---|---|
| `pnpm install` | Install frontend dependencies |
| `pnpm tauri dev` | Start dev mode (Vite + Rust hot-reload, opens desktop window) |
| `pnpm tauri build` | Production build for the current platform |
| `pnpm build` | Frontend-only type-check (`vue-tsc`) then Vite build |
| `cd src-tauri && cargo test` | Run all Rust tests |

## Coding Style & Naming Conventions

**Rust (backend):**
- Follow `rustfmt` defaults; run `cargo fmt` before committing.
- Modules use `mod.rs` pattern; one concern per file (e.g. `key/store.rs`, `key/rotation.rs`).
- Error types go in `error.rs` using `thiserror` derives.
- Use `serde` derive for all serializable structs.

**TypeScript / Vue (frontend):**
- Vue 3 Composition API with `<script setup>`.
- Naive UI component library — prefer its components over custom HTML.
- Pinia for state management (`src/stores/`).

## Testing Guidelines

- Rust: `#[cfg(test)]` unit tests within modules; integration tests in `src-tauri/tests/`.
- `tempfile` crate available for test fixtures.
- No frontend test framework is configured.

## Commit & Pull Request Guidelines

- Use **Conventional Commits**: `feat:`, `fix:`, `refactor:`, `test:`, `chore:`.
- Keep messages concise; add body details only for complex changes.
- Scope prefixes are optional (e.g. `feat(apps): …`).

## Architecture Notes

The proxy runs on **Axum** inside the Tauri process. All provider calls go through the **converter IR layer**: requests are parsed into an internal Intermediate Representation, then re-generated in the target provider's format. To add a new provider, implement a parser and generator for the new format.

Database migrations are numbered SQL files in `src-tauri/migrations/`, applied at startup via SQLx. Always add a new numbered migration for schema changes — never modify existing migrations.

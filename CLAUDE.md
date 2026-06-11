# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

AI Proxy is a local LLM reverse proxy packaged as a Tauri 2 desktop app (Rust backend + Vue 3 frontend). It unifies OpenAI Completions/Responses, Anthropic Messages, and Google Gemini behind a single endpoint at `http://127.0.0.1:7860`, using an IR (Intermediate Representation) layer for transparent bidirectional format conversion. UI language is Chinese (zh-CN).

## Build & Dev Commands

```bash
pnpm install                                  # Install frontend deps
pnpm tauri dev                                # Full dev mode (Vite :1420 + Rust hot-reload, proxy :7860)
pnpm tauri build                              # Production build → native installer
pnpm build                                    # Frontend type-check (vue-tsc) + Vite bundle
cd src-tauri && cargo test                    # All Rust tests
cd src-tauri && cargo test --test ir_conversion   # Specific integration test
cd src-tauri && cargo test test_name          # Single test by name
cd src-tauri && cargo fmt                     # Format Rust code
```

No frontend test framework. No linter/formatter for TS — match existing style.

## Architecture

### Dual Server Pattern

The Tauri app embeds an Axum HTTP server. The Vue frontend talks to Axum via HTTP (not Tauri IPC), so the proxy is independently accessible by external LLM clients. During frontend-only dev, Vite proxies `/api` and `/health` to the Axum backend. A shared `reqwest::Client` (`http.rs` → `SHARED_HTTP_CLIENT`) is used for all upstream requests.

### Feature Flags: Desktop vs Server

Two build modes via Cargo features:
- **`desktop`** (default): Tauri GUI, system tray, auto-update, app management (`apps/` module)
- **`server`**: Headless CLI binary (`ai-proxy-server`), JWT auth (`auth/` module), no Tauri deps

`#[cfg(feature = "desktop")]` guards Tauri-specific code in `lib.rs`. The `server_main.rs` is currently a placeholder.

### IR-Based Format Conversion (Core Pattern)

All 4 API formats flow through a shared IR layer in `converter/`:

```
Client Request → FormatParser → IrRequest → [interceptor rules] → route to provider → FormatGenerator → Upstream Request
Upstream Response → FormatParser → IrResponse → FormatGenerator → Client Response
```

- **Traits**: `FormatParser` (parse_request, parse_stream_chunk, parse_response) and `FormatGenerator` (generate_request, generate_stream_chunk, generate_response) in `converter/mod.rs`
- **IR types**: `IrRequest`, `IrResponse`, `IrStreamChunk`, `IrMessage`, `IrContentPart` in `converter/ir.rs`
- **Per-format**: Each format (anthropic, completions, gemini, responses) has its own parser in `parsers/` and generator in `generators/`
- To add a new provider format: implement both `FormatParser` and `FormatGenerator` traits

### Request Lifecycle

`handle_proxy()` in `server/handlers.rs` orchestrates: parse incoming request by format → convert to IR → apply interceptor rules → resolve model-to-provider route via `ProviderManager` → rotate API key via `KeyRotation` → convert IR to target format → forward upstream → convert response back → log & track usage. Streaming (SSE) uses per-format state machines.

### Database

SQLite via SQLx, WAL mode. Migrations are numbered SQL files in `src-tauri/migrations/` (001–013), applied at startup. **Never modify existing migrations** — always add a new numbered one. Settings are stored as key-value pairs in a `settings` table.

### Key Modules

| Module | Purpose |
|---|---|
| `converter/` | IR types + 4 format parsers/generators (the core extensibility point) |
| `server/handlers.rs` | Proxy forwarding core (~1088 lines) with SSE streaming |
| `server/api.rs` | Management REST API (~757 lines), unified response types (`ok()`, `err_json()`) |
| `server/middleware.rs` | CORS + optional Bearer token auth |
| `provider/manager.rs` | Model-to-provider resolution, returns `ResolvedRoute` with target format/model |
| `key/` | AES-256-GCM encrypted key storage (`store.rs`) + least-used rotation (`rotation.rs`) |
| `interceptor/` | Rule engine with wildcard pattern matching |
| `usage/pricing.rs` | Per-1K-token prices, $0.001/1K default |
| `mcp/` | MCP server management + sync |
| `skill/` | Skill scanning + management |
| `auth/` | Server-mode-only JWT auth |

### Frontend

Vue 3 + Composition API (`<script setup>`) + Naive UI + Pinia + ECharts. API client in `src/api/index.ts` auto-discovers proxy base URL via health check (tries Tauri IPC first, then HTTP). Routes are lazy-loaded from `src/views/`. Views include: Dashboard, Providers, Logs, RuntimeLogs, Statistics, Apps, McpServers, Skills, Rules, Settings.

### Proxy Endpoints

| Endpoint | Format |
|---|---|
| `POST /v1/chat/completions` | OpenAI Chat Completions |
| `POST /v1/responses` | OpenAI Responses API |
| `POST /v1/messages` | Anthropic Messages |
| `POST /v1beta/models/{model}:generateContent` | Google Gemini |
| `GET /v1/models` | Model list |
| `GET /health` | Health check |

### Error Handling

All backend errors use `ProxyError` enum (`error.rs`) with `thiserror` derives. It implements `IntoResponse` for Axum with appropriate HTTP status codes. Frontend API responses use `{ success: bool, data: T }` or `{ success: bool, error: string }`.

## Commit Convention

Conventional Commits: `type: description` (lowercase). Types: feat, fix, refactor, docs, test, chore, perf, ci.

## Release Process

`bash scripts/bump-version.sh <version>` bumps version in `package.json`, `tauri.conf.json`, and `Cargo.toml` simultaneously. Pushing a `v*` tag triggers GitHub Actions to build macOS (.dmg) and Windows (.exe, .msi) installers.

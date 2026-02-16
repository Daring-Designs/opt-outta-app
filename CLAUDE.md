# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Dev Commands

Node v22 is required (managed via nvm). Always source nvm before running any npm/node commands:

```bash
# Full Tauri app (frontend + Rust backend)
source ~/.nvm/nvm.sh && nvm use 22 && npx @tauri-apps/cli dev

# Debug build
source ~/.nvm/nvm.sh && nvm use 22 && npx @tauri-apps/cli build --debug

# TypeScript type checking
source ~/.nvm/nvm.sh && nvm use 22 && npx vue-tsc --noEmit

# Rust checking
cd src-tauri && cargo check
```

Rust is installed via Homebrew (not rustup).

## Architecture

Tauri v2 desktop app: Rust backend + Vue 3 frontend communicating over Tauri IPC (`invoke`/events).

### Backend (src-tauri/src/)

- **commands/** — Tauri command handlers (profile, brokers, optout, playbooks, history). Each command returns `Result<T, String>`.
- **engine.rs** — OptOutEngine state machine. Playbook-only execution (no AI). Runs in a background `tokio::spawn` task, emits `opt-out-progress` and `opt-out-complete` events to the frontend. Supports both community and local (`"local:{id}"` prefix) playbook selections.
- **recorder.rs** — Chrome recording mode for community playbook creation. Injects JS into pages to capture user interactions (clicks, fills, navigations) without capturing PII/field values.
- **crypto.rs** — AES-256-GCM encrypt/decrypt. Format: `base64(12-byte-nonce || ciphertext)`.
- **secrets.rs** — SecretsCache wrapping OS keychain (service: "opt-outta"). Stores the encryption key.
- **browser.rs** — Chrome/Chromium binary detection and CDP launch via chromiumoxide.
- **playbook_api.rs** — HTTP client for opt-outta.com playbook endpoints.
- **models.rs** — All shared Rust types. Serialized to JSON for IPC.

### Frontend (src/)

- **stores/** — 5 Pinia stores (Composition API): profile, brokers, optout, playbooks, history.
- **views/** — 5 pages: Dashboard (`/`), Profile (`/profile`), Brokers/Playbooks (`/brokers`), Settings (`/settings`), History (`/history`).
- **components/** — Reusable UI components.
- **types/index.ts** — TypeScript interfaces mirroring Rust models. Must stay in sync with `models.rs`.

### Data Flow

- **Commands (frontend → backend):** `invoke<T>("command_name", { args })` calls Tauri command handlers.
- **Events (backend → frontend):** `emit("event-name", payload)` from Rust; `listen("event-name", callback)` in Vue stores.
- **Profile encryption:** Profile JSON → AES-256-GCM encrypt → `profile.enc` file. Key from OS keychain.
- **Broker registry:** Bundled as `registry/brokers.json` (Tauri resource). Website API at opt-outta.com provides sync.

## Key Patterns

- **Type sync:** `src-tauri/src/models.rs` (Rust) and `src/types/index.ts` (TypeScript) must match. Rust uses `#[serde(rename = "camelCase")]` for JSON field names.
- **Store pattern:** Pinia stores use `ref<T>` for state, `computed` for derived values, `async function` for mutations that call `invoke()`.
- **Error handling:** Rust commands return `Result<T, String>`. Errors throw on the TypeScript side.
- **Secrets:** Encryption key retrieved from `SecretsCache` at command runtime, never held in frontend state.
- **Recording live polling:** During playbook recording, the store polls `get_recorded_actions` every 1.5s, diffs against `seenActionCount`, and appends new steps to `editableSteps`.

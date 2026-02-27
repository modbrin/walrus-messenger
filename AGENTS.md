# Repository Guidelines

## Project Overview
`walrus-server` is the backend for **Walrus Messenger** (Android client lives in a separate repository).  
It provides authentication/session flows and messaging data operations over an Axum + SQLx + Postgres stack.

## Project Structure & Module Organization
This workspace currently contains one crate: `crates/server`.

Inside `crates/server/src`:
- `server/`: HTTP router and app state (`router.rs`, `state.rs`, `mod.rs`).
- `auth/`: bearer token parsing and session token packing/unpacking.
- `database/`: schema setup/reset, DB commands (writes), and queries (reads).
- `models/`: typed domain models (`user`, `chat`, `message`, `session`, `resource`).
- `tests/`: integration-style async tests for auth, sessions, chats, and messages.

Top-level files: `config.yaml` (local runtime config), `db_manual.txt` (Postgres setup notes), and `assets/` (static resources).

## Build, Test, and Development Commands
- `cargo check --workspace`: type-check all workspace crates.
- `cargo build --workspace`: compile all targets.
- `cargo run -p walrus-server`: run backend on the address from `config.yaml` (default `0.0.0.0:3000`).
- `cargo test -p walrus-server -q`: run server tests.
- `cargo test -p walrus-server login_and_resolve_session -- --nocapture`: run one test while iterating.

Server tests expect a local Postgres database and credentials matching `config.yaml`/`db_manual.txt`.

## Coding Style & Naming Conventions
- Rust edition: 2021; format before PRs with `cargo fmt --all`.
- Use 4-space indentation and standard Rust naming:
  - modules/files/functions: `snake_case`
  - structs/enums/traits: `PascalCase`
  - constants: `UPPER_SNAKE_CASE`
- Keep modules focused by domain (`database`, `auth`, `server`), and add new files to the matching domain folder.

## Testing Guidelines
- Primary tests live in `crates/server/src/tests/` and use `#[tokio::test]`.
- Prefer scenario-style names like `login_and_resolve_session`.
- Add/extend tests when changing auth, DB schema/queries, or access-control logic.
- Run targeted tests during iteration, then run full `cargo test -p walrus-server -q`.

## Architecture Notes
- Current HTTP routes are in `server/router.rs` (notably `/login` and `/protected`).
- Messaging domain is already modeled in Postgres tables: `users`, `sessions`, `chats`, `chats_members`, `messages`, `resources`.
- Schema/bootstrap logic lives in `database/schema.rs`; origin admin user is created there.

## Security & Configuration Tips
- Do not commit real secrets; `config.yaml` values are local defaults only.
- Keep database permissions minimal for app users, and document any new required grants/extensions.

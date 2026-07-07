# Project Structure

This document records the first concrete naming and layout decisions for the formal WineStock project.

## Project Name

Use `WineStock` as the product and repository name.

The repository root should be a Cargo workspace, not a root application crate.
The root `Cargo.toml` owns workspace membership, shared dependency policy, and Rust build settings.

## Rust Crates

Use `winestock-` as the Cargo package prefix for Rust crates.
Rust crate identifiers will use underscores where required by Rust import syntax.

Initial Rust crates:

- package `winestock-core`, crate `winestock_core`
- package `winestock-shared`, crate `winestock_shared`

Current additional Rust crate:

- package `winestock-server`, binary/library crate `winestock_server`

## Naming Rules

`core` means the shared Rust/Axum service core.
It owns the Axum router, service lifecycle, network bind behavior, business behavior, shared service state, and persistence integration.

`shared` means Rust code reused by core and platform shells.
It owns shared config, platform-neutral contracts, common value types, and other code that should not depend on Axum, Tauri, Android, or WebView packaging.

Avoid `share-core` for shared project modules.
Use `share` only for user-facing sharing features, if those exist later.

Do not create a separate `shared-service` crate while `core` is the Axum service library.
That would duplicate the service-core concept.

## Target Layout

The formal project should use this layout unless a later design decision changes it.
Items marked as current already exist.

```text
WineStock/
  AGENTS.md
  docs/
    architecture.md
    runtime-networking.md
    platforms.md
    project-structure.md
    agent-checklist.md
    code-map.md
    database-schema.md
    rbac-permission-model.md
    implementation-notes/
  Cargo.toml                     # current workspace manifest
  Cargo.lock                     # current Rust lockfile
  core/                          # current winestock-core library crate
    Cargo.toml
    src/
      lib.rs
  shared/                        # current winestock-shared library crate
    Cargo.toml
    src/
      auth.rs
      config.rs
      error.rs
      lib.rs
      validation.rs
  desktop/
    tauri/
  android/
    app/
  server/                        # current winestock-server headless shell crate
    Cargo.toml
    src/
      config.rs
      error.rs
      lib.rs
      main.rs
  frontend/
```

## Workspace Shape

The initial root workspace should include the Rust crates only.

```toml
[workspace]
resolver = "2"
members = [
  "core",
  "server",
  "shared",
]
```

Add desktop Tauri Rust crates, Android Rust bridge crates, or a server executable crate as workspace members only when those crates exist.

Current workspace dependencies:

- `axum`
- `argon2`
- `base64`
- `getrandom`
- `garde`
- `jsonwebtoken`
- `sea-orm`
- `sea-orm-migration`
- `serde`
- `serde_json`
- `sha2`
- `sqlx`
- `tempfile`
- `tokio`
- `tower`
- `utoipa`
- `utoipa-axum`
- `utoipa-swagger-ui`
- `winestock-core`
- `winestock-shared`

`tokio` is used by core service startup and the server shell.

## Component Ownership

`core` owns the shared Axum service library.
It depends on `shared` as needed.
It must not own platform UI assets or frontend build output.

Current `core` API surface:

- `build_router()`
- `build_router_with_local_service()`
- `bootstrap_from_config()`
- `bind_server()`
- `OPENAPI_JSON_PATH`
- `SWAGGER_UI_PATH`

Current `core` HTTP surface:

- `POST /api/auth/register`
- `POST /api/auth/login`
- `POST /api/auth/refresh`
- `POST /api/auth/logout`
- `GET /api/auth/me`
- `GET /api-docs/openapi.json`
- Swagger UI under `/swagger-ui`

`shared` owns the shared runtime configuration model and platform-neutral contracts.
It must not depend on `core`, Axum, Tauri, Android shell code, or frontend build output.

`desktop/tauri` owns the Tauri v2 desktop shell.
It starts or connects to `core` based on config and packages desktop frontend assets through Tauri.

`android/app` owns the Android native shell and WebView.
It starts or connects to `core` based on config and packages Android frontend assets through Android.

`server` owns the headless server shell.
It starts `core` based on shared config, reports service status and access URLs, handles Ctrl+C shutdown, and does not own frontend build output.

`frontend` owns frontend source code.
The selected frontend framework is a project choice, not an Axum service requirement.

## Future Splits

If `shared` grows too broad, split it later into smaller Rust crates such as `shared-config` or `shared-domain`.
Do this only when there is real ownership pressure, not at project start.

If `core` grows too broad, extract platform-neutral domain logic into `shared` or another shared Rust crate.
Keep the Axum service boundary in `core`.

## Current Workspace Notes

`core` and `shared` are now formal Rust library crates.

An existing `frontend` scaffold is frontend source, not an Axum-owned asset bundle.
Do not copy its build output into `core` or any shared Rust crate.

The existing `frontend` scaffold currently uses Vue and Vite.
This records the current files only; it does not make Vue a required product architecture choice.

Any existing plain Rust project under `desktop/` is not a workspace member and is not the formal Tauri shell.
Convert or replace it only when implementing `desktop/tauri`.

The `server/` directory is now the formal headless server shell crate.

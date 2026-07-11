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

`shared` means Rust code reused by core and platform shells for runtime configuration and primitive platform-neutral helpers.
It owns shared config, runtime modes, config parsing errors, basic text validation, and other platform-neutral startup contracts that should not depend on Axum, Tauri, Android, or WebView packaging.
HTTP API DTOs and business validation belong to `core`, because they are part of the Axum service contract.

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
    README.md                     # 项目级文档入口，只保存跨组件规范
    architecture.md
    runtime-networking.md
    platforms.md
    project-structure.md
    agent-checklist.md
    code-map.md                  # 分层代码地图总索引
    code-map/                    # 按 workspace/shared/core/server/frontend 拆分的子地图
    implementation-notes/         # 仅保留跨组件方案
  Cargo.toml                     # current workspace manifest
  Cargo.lock                     # current Rust lockfile
  core/                          # current winestock-core library crate
    Cargo.toml
    docs/                        # core API、数据库、权限、校验和实现记录
    src/
      lib.rs
  shared/                        # current winestock-shared library crate
    Cargo.toml
    docs/                        # shared 配置与基础校验文档
    src/
      config.rs
      config_validation.rs
      error.rs
      lib.rs
      text_validation.rs
  desktop/
    tauri/
  android/
    app/
  server/                        # current winestock-server headless shell crate
    Cargo.toml
    docs/                        # server shell 配置、部署和生命周期文档
    src/
      config.rs
      error.rs
      lib.rs
      main.rs
  frontend/
    docs/                        # 前端路由、交互、视觉和页面文档
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
- `GET /api/health`
- `GET /api/auth/me`
- `POST /api/auth/me/password`
- `GET /api/users`
- `GET /api/users/{id}`
- `DELETE /api/users/{id}`
- `PATCH /api/users/{id}/status`
- `PUT /api/users/{id}/permissions`
- `POST /api/users/{id}/password`
- `GET /api/permissions`
- `GET /api-docs/openapi.json`
- Swagger UI under `/swagger-ui`

`shared` owns the shared runtime configuration model, platform-neutral startup contracts, and primitive text validation helpers.
It must not depend on `core`, Axum, Tauri, Android shell code, or frontend build output.

`desktop/tauri` owns the Tauri v2 desktop shell.
It starts or connects to `core` based on config and packages desktop frontend assets through Tauri.

`android/app` owns the Android native shell and WebView.
It starts or connects to `core` based on config and packages Android frontend assets through Android.

`server` owns the headless server shell.
It starts `core` based on shared config, reports service status and access URLs, handles Ctrl+C shutdown, and does not own frontend build output.

`frontend` owns frontend source code.
The selected frontend framework is a project choice, not an Axum service requirement.

The root `docs/` directory owns only cross-component architecture, platform, networking, project-structure, agent-checklist, and whole-repository code-map documents.
Component-specific documentation belongs under `core/docs/`, `shared/docs/`, `server/docs/`, `frontend/docs/`, or the corresponding future platform directory.

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

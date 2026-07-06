# Architecture

This document defines the target architecture for the formal project.
It is not a description of the current Android demo layout.

## Goal

Build one shared service core and multiple platform shells around it.

The service core is a Rust/Axum library.
Desktop and Android both reuse the same service library.
A pure server shell may also host the same service library.
UI-bearing platforms own their UI runtime, frontend packaging, permissions, and lifecycle.

## Major Components

### `core axum library`

The `core axum library` is the shared Rust service layer.

It owns:

- API routes
- request and response contracts
- service startup and shutdown hooks
- network bind behavior
- business logic
- shared state
- persistence integration, if required

It does not own:

- platform UI code
- Tauri windows
- Android activities
- WebView setup
- frontend build artifacts
- platform asset packaging

The library should expose a small startup surface such as:

- build router
- start server with config
- stop server gracefully
- report bound address and runtime status

The exact API can change, but the ownership boundary should not.

Current implemented surface:

- `winestock_core::build_router()`
- `winestock_core::build_router_with_local_service()`
- `winestock_core::bootstrap_from_config()`
- `winestock_core::bind_server()`
- `GET /api/health`
- `POST /api/auth/register`
- `POST /api/auth/login`
- `POST /api/auth/refresh`
- `POST /api/auth/logout`
- `GET /api/auth/me`
- OpenAPI JSON at `/api-docs/openapi.json`
- Swagger UI at `/swagger-ui`

Swagger UI is API documentation tooling exposed by the service.
It is not a platform frontend bundle and does not change the rule that Axum must not own desktop or Android UI assets.

### `desktop shell`

The desktop shell is the Tauri v2 application.

It owns:

- desktop process lifecycle
- Tauri configuration
- window creation
- frontend asset packaging
- desktop permissions and OS integration
- deciding whether to start local Axum based on shared config

The desktop UI accesses Axum through HTTP.
For local self access, the WebView should use `http://127.0.0.1:<port>`.
For remote access, the WebView should use `remote_base_url`.

### `android shell`

The Android shell is a native application with a WebView.

It owns:

- Android activity lifecycle
- WebView setup
- Android permissions
- foreground/background behavior
- native library loading
- Android asset packaging
- deciding whether to start local Axum based on shared config

Android starts the shared Axum service through the Rust native library when config requires it.
The Android WebView accesses Axum through HTTP.

### `server shell`

The server shell is a headless platform entry point for API-only deployment.

It owns:

- server process lifecycle
- config loading
- logging and startup status output
- starting and stopping the shared Axum service
- graceful shutdown

It does not own:

- frontend assets
- WebView or window code
- platform UI packaging

The server shell exposes the shared Axum service over HTTP for other clients.
It does not create a separate service implementation.

### `frontend app(s)`

Frontend apps are UI assets owned by platform packaging.

The architecture does not require a specific frontend framework.
A frontend may be shared between platforms if the project chooses that later.
That sharing must still respect platform packaging boundaries.

Axum must not become the owner of frontend build output.
Do not place platform frontend artifacts inside the Axum crate.

### `shared rust library/config`

The shared Rust library defines runtime mode, bind behavior, port selection, remote URLs, and other platform-neutral contracts.

All platforms should use the same logical config model.
Platform shells may store config differently, but they should map to the same keys and meanings.

See `docs/runtime-networking.md` for the network model.
See `docs/project-structure.md` for concrete project naming and directory layout.

## Communication Boundary

UI code talks to the service over HTTP.
A pure server shell exposes the same HTTP API without a local UI.

Do not wire desktop or Android UI directly to internal Rust business functions.
The HTTP API is the boundary between UI and service behavior.

This keeps:

- desktop behavior consistent with Android behavior
- local mode consistent with remote mode
- server behavior testable without a platform UI

## Dependency Direction

Platform shells may depend on the `core axum library`.
Frontend apps may depend on generated or shared API contracts.
The `core axum library` must not depend on platform shells.

Allowed direction:

```text
desktop shell  -> core axum library
android shell  -> core axum library
server shell   -> core axum library
frontend app   -> HTTP API
```

Disallowed direction:

```text
core axum library -> desktop shell
core axum library -> android shell
core axum library -> server shell
core axum library -> frontend build output
```

## Demo Code

The current demo may be useful as a technical experiment.
It is not the formal architecture.

Do not preserve demo JNI, UI, package, or Gradle structure unless the user explicitly chooses to keep it.

## Current Status

The root Cargo workspace currently contains:

- `core` as package `winestock-core`, crate `winestock_core`
- `server` as package `winestock-server`
- `shared` as package `winestock-shared`, crate `winestock_shared`

`core` currently depends on Axum, Tokio, Utoipa, Utoipa Axum integration, Utoipa Swagger UI, Serde, SeaORM/SQLx SQLite bootstrap dependencies, and `shared`.
`shared` contains the platform-neutral JSON startup config model.

Desktop and Android platform shells are not implemented yet.

The formal server shell exists under `server/` and starts the shared Axum service from JSON config.

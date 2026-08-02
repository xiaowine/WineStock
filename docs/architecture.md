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
- `PATCH /api/users/{id}/username`
- `PUT /api/users/{id}/permissions`
- `POST /api/users/{id}/password`
- `GET /api/permissions`
- OpenAPI JSON at `/api-docs/openapi.json` in Debug builds only
- Swagger UI at `/swagger-ui` in Debug builds only when the Swagger UI feature is enabled

Release builds do not register `/api-docs/openapi.json` or `/swagger-ui`, and do not compile or link Swagger UI.
In Debug builds, OpenAPI JSON and optional Swagger UI are API documentation tooling exposed by the service.
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
The WebView loads frontend resources packaged by Tauri rather than opening the Axum address as its page URL.
The packaged frontend uses `http://127.0.0.1:<port>` for local self access or `remote_base_url` for remote API access.
All runtime configuration and service status UI belongs to the shared frontend; the desktop shell persists and applies configuration through the Shell Bridge without owning a native settings UI.

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
The Android WebView loads frontend resources packaged by Android and accesses Axum through HTTP.
All runtime configuration and service status UI belongs to the shared frontend; the Android shell persists and applies configuration through the Shell Bridge without owning a native settings Activity or dialog.

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

For UI-bearing platforms, the frontend is also the only functional UI for runtime configuration, service status, startup recovery, and connection settings.
The platform shell exposes those operations through a narrow Shell Bridge and does not create a second native settings interface.

The architecture does not require a specific frontend framework.
A frontend may be shared between platforms if the project chooses that later.
That sharing must still respect platform packaging boundaries.

Axum must not become the owner of frontend build output.
Do not place platform frontend artifacts inside the Axum crate.

### `shared rust library/config`

The shared Rust library defines runtime mode, bind behavior, port selection, remote URLs, and other platform-neutral contracts.

All platforms should use the same logical config model.
Platform shells may store config differently, but they should map to the same keys and meanings.
HTTP API request/response DTOs belong to the `core axum library`, not to `shared`.

See `docs/runtime-networking.md` for the network model.
See `docs/project-structure.md` for concrete project naming and directory layout.

## Communication Boundary

UI code talks to the service over HTTP.
A pure server shell exposes the same HTTP API without a local UI.

UI-bearing platform runtime control uses a separate Shell Bridge for configuration persistence, service lifecycle, platform events, and effective API address reporting.
The Shell Bridge must not proxy business APIs or carry authentication tokens.

Do not wire desktop or Android UI directly to internal Rust business functions.
The HTTP API is the boundary between UI and service behavior.

This keeps:

- desktop behavior consistent with Android behavior
- local mode consistent with remote mode
- server behavior testable without a platform UI
- runtime configuration recoverable even when the HTTP service cannot start

See `docs/shell-bridge.md` for the versioned UI-platform bridge and frontend-owned settings model.

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
frontend app   -> Shell Bridge implemented by its current desktop/android host
```

Disallowed direction:

```text
core axum library -> desktop shell
core axum library -> android shell
core axum library -> server shell
core axum library -> frontend build output
Shell Bridge -> core business functions or HTTP DTO duplication
```

## Demo Code

The current demo may be useful as a technical experiment.
It is not the formal architecture.

Do not preserve demo JNI, UI, package, or Gradle structure unless the user explicitly chooses to keep it.

## Current Status

The root Cargo workspace currently contains:

- `android/native` as package `winestock-android`, crate `winestock_android`
- `core` as package `winestock-core`, crate `winestock_core`
- `desktop` as package `winestock-desktop`, crate `winestock_desktop`
- `server` as package `winestock-server`
- `shared` as package `winestock-shared`, crate `winestock_shared`

`core` currently depends on Axum, Tokio, Utoipa, Utoipa Axum integration, Utoipa Swagger UI, Serde, Garde, SeaORM/SQLx SQLite bootstrap dependencies, and `shared`.
`shared` contains the platform-neutral JSON startup config model, config parsing errors, and primitive text validation helpers.

The formal Desktop Tauri shell now lives at `desktop`: it packages the shared frontend, implements
the constrained Shell Bridge v1 transport, persists desktop runtime configuration, owns the app-data
storage paths, and manages `RunningLocalService` shutdown/restart in the Tauri process. Windows installer
and installed-app smoke verification remain pending. The repository Android shell now includes packaged frontend loading, Shell Bridge transport, edge-to-edge WindowInsets publication, an
Application-level runtime manager, and the `android/native -> core -> shared` local Axum path.
When Android has no persisted runtime configuration, it loads the packaged frontend with an
uninitialized/stopped snapshot and waits for the frontend to apply a local or remote mode before
starting the HTTP service or persisting configuration. Existing valid configurations still activate
automatically on later cold starts.
Host, ARM64, and APK verification are complete. An API 33 ARM64 physical-device smoke has also
verified packaged WebView loading, offline recovery, local/remote HTTP use, lifecycle recovery,
the pre-existing rotation flow, and native-back interactions; the Activity is now locked to
sensorPortrait and that new no-landscape rule still needs physical-device verification. The broader
Android version, navigation-mode, and business regression matrix remains to be covered. The newer
first-run uninitialized/stopped funnel has JVM and frontend coverage but still requires a physical-device smoke.

The formal server shell exists under `server/` and starts the shared Axum service from JSON config.

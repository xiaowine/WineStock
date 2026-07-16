# Platforms

This document defines platform responsibilities for the formal project.

## Desktop

Desktop uses Tauri v2.

The desktop shell owns:

- Tauri configuration
- window creation
- WebView URL selection
- frontend asset packaging
- desktop permissions and OS integration
- starting and stopping the shared Axum service
- discovering local and LAN URLs and returning them to the frontend
- persisting and applying runtime configuration requested by the frontend

At startup, the desktop shell registers the Shell Bridge and loads the frontend packaged by Tauri even when configuration or the API service is unavailable.
The frontend is the only runtime settings and service-status UI.
The shell reads, validates, persists and applies shared config at the frontend's request.
Local modes always start the shared Axum service in the background when the application starts; this is not a user-configurable UI option. `client-only` does not start Axum.

The WebView opens Tauri-packaged frontend resources.
The frontend then accesses one of these API roots:

- `http://127.0.0.1:<port>` for local self access
- `remote_base_url` for remote access

The desktop shell must handle:

- port conflict reporting
- graceful shutdown
- service status reporting to the frontend
- restart after config changes, if supported
- firewall or OS permission guidance, if needed
- versioned Shell Bridge commands, events and Tauri capabilities

The desktop shell must not implement a native settings window or proxy business HTTP APIs.

Desktop frontend resources are packaged by Tauri.
They are not served by the Axum crate.

Current status:

- the formal Tauri shell is not implemented yet
- `desktop/tauri` does not exist yet
- any existing plain Rust scaffold under `desktop/` is not the formal desktop architecture

## Android

Android uses a native shell plus WebView.

The Android shell owns:

- Activity lifecycle
- WebView configuration
- Android permissions
- native Rust library loading
- starting and stopping the shared Axum service
- foreground/background policy
- Android asset packaging
- discovering local and LAN URLs and returning them to the frontend
- persisting and applying runtime configuration requested by the frontend

At startup, the Android shell registers the Shell Bridge and loads Android-packaged frontend resources even when configuration or the API service is unavailable.
The frontend is the only runtime settings and service-status UI.
The shell reads, validates, persists and applies shared config at the frontend's request.
Local modes always start the shared Axum Android native library when the application starts; this is not a user-configurable UI option. `client-only` does not start Axum.

The WebView opens Android-packaged frontend resources from a trusted local origin.
The frontend then accesses one of these API roots:

- `http://127.0.0.1:<port>` for local self access
- `remote_base_url` for remote access

The Android shell must handle:

- network permission declarations
- cleartext HTTP policy if HTTP is used
- lifecycle transitions
- service shutdown
- port conflict reporting
- foreground service requirements, if long-running background service is needed
- origin-restricted Shell Bridge messaging and external navigation handling

The Android shell must not implement a native settings Activity or dialog, and must not expose the bridge to untrusted WebView origins.

Android frontend resources are packaged by Android.
They are not served by the Axum crate.

Current status:

- the formal Android Shell Bridge and shared Rust service integration are not complete
- any current Android scaffold must still be aligned with this document before it is treated as the formal shell

## Server

The server shell is a headless process with no frontend.

The server shell owns:

- process lifecycle
- config loading
- logging and startup status output
- starting and stopping the shared Axum service
- graceful shutdown
- presenting bound addresses for local or LAN access

At startup, the server shell reads shared config.
For API-only hosting it starts the shared Axum service with explicit `bind_host` and `port`.
`remote_base_url` is not required just to expose its own API.

The server shell does not use a WebView and does not package frontend assets.

Current status:

- the formal server shell exists under `server/`
- it always reads or creates `data/config.json` next to the server executable
- it does not accept a config path argument
- it reads JSON config, starts the shared Axum service, reports access URLs, and handles Ctrl+C shutdown

## Shared Axum Service

Desktop, Android, and the server shell call into the same Rust service library.

The service library should expose platform-neutral functions for:

- creating the router
- starting the server
- stopping the server
- reporting bind address and port
- reporting startup errors

The service library must not know whether it was started by Tauri, Android, or the server shell.

## WebView Contract for UI Platforms

All UI-bearing platforms use HTTP to access the service.

The WebView page URL points to platform-packaged frontend resources, not to the Axum API root.
The API root is selected from runtime config and service startup results, then returned to the frontend through the Shell Bridge.
The API root must never use `0.0.0.0` or `::` as its access host.

The frontend must load before API availability is known and must provide configuration, loading, retry and failure UI.
Platform shells publish state and execute lifecycle operations without implementing a second functional UI.

Business operations continue over HTTP.
Only runtime configuration, service lifecycle, effective addresses and platform events use the Shell Bridge.
See `docs/shell-bridge.md`.

## Frontend Packaging

Each platform packages frontend resources using its own framework.
The server shell does not package frontend resources.

Allowed:

- Tauri packaging for desktop frontend files
- Android asset packaging for Android frontend files
- shared frontend source if the build output is copied into platform packages

Disallowed:

- placing platform frontend build output in the Axum crate
- making Axum responsible for Tauri asset serving
- making Axum responsible for Android WebView asset serving

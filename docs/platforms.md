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
- presenting local and LAN URLs to the user

At startup, the desktop shell reads shared config.
If `auto_start_server` is enabled for the selected mode, it starts the shared Axum service in the background.
If the selected mode is `client-only`, it does not start Axum.

The WebView opens one of these:

- `http://127.0.0.1:<port>` for local self access
- `remote_base_url` for remote access

The desktop shell must handle:

- port conflict reporting
- graceful shutdown
- service status display
- restart after config changes, if supported
- firewall or OS permission guidance, if needed

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
- presenting local and LAN URLs to the user

At startup, the Android shell reads shared config.
If `auto_start_server` is enabled for the selected mode, it starts the shared Axum Android native library.
If the selected mode is `client-only`, it does not start Axum.

The WebView opens one of these:

- `http://127.0.0.1:<port>` for local self access
- `remote_base_url` for remote access

The Android shell must handle:

- network permission declarations
- cleartext HTTP policy if HTTP is used
- lifecycle transitions
- service shutdown
- port conflict reporting
- foreground service requirements, if long-running background service is needed

Android frontend resources are packaged by Android.
They are not served by the Axum crate.

Current status:

- the formal Android shell is not implemented yet
- `android/app` does not exist yet

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

The WebView URL is selected from runtime config and service startup results.
The URL must never be `0.0.0.0`.

The UI should tolerate delayed service startup.
Platform shells should provide a loading or retry path while Axum starts.

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

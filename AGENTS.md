# Agent Instructions

This file is the short operating guide for agents.
Keep detailed design in `docs/*.md`.
Do not treat the current Android demo layout as the target architecture.

## Required Reading

Before changing project code or structure, read:

1. `docs/architecture.md`
2. `docs/runtime-networking.md`
3. `docs/platforms.md`
4. `docs/project-structure.md`
5. `docs/agent-checklist.md`

## Project Position

The formal product includes desktop and Android client platforms.
It may also include a pure server shell that exposes the shared API without frontend assets.

The shared backend is one Rust/Axum service library.
Desktop and Android must reuse the same service library.
Do not create separate platform-specific servers.

The UI is platform-owned.
Axum does not package, own, or serve platform frontend build artifacts.

## Core Components

Use these major parts:

- `core axum library`
- `desktop shell`
- `android shell`
- `server shell`
- `frontend app(s)`
- `shared rust library/config`

The `core axum library` owns API routes, service lifecycle hooks, network bind behavior, business logic, shared state, and persistence integration.

The `core axum library` does not own Tauri windows, Android activities, WebView setup, shell-specific process wrappers, platform permissions, or frontend build artifacts.

## Platform Entry Points

Desktop uses Tauri v2.
The desktop shell starts or connects to Axum based on config.
The Tauri WebView opens a local or remote HTTP URL.

Android uses a native shell plus WebView.
The Android shell starts or connects to Axum based on config.
The Android WebView opens a local or remote HTTP URL.

Server uses a headless process.
The server shell starts Axum from shared config and exposes HTTP APIs without WebView or frontend packaging.

UI-bearing platforms communicate with Axum over HTTP.
Do not couple UI code directly to internal Rust business APIs.

## Networking Rules

The app can access itself.
The app can access another device running the service.
The app can expose its service to other clients.

Use explicit runtime modes: `client-only`, `self-hosted`, `server-mode`, and `connect-to-remote`.

For local self access, prefer `127.0.0.1:<port>`.
For server mode, use explicit `bind_host` and `port`.
For platforms that connect to another service, use `remote_base_url`.

`0.0.0.0` is only a bind address.
Never show `0.0.0.0` as a user-facing access URL.
When bound to all interfaces, enumerate actual LAN IP addresses for display.

## Config Rules

Do not hard-code IP addresses.
Do not hard-code ports.
Use shared config for all runtime network decisions.

Expected config keys: `server.enabled`, `bind_host`, `port`, `remote_base_url`, and `auto_start_server`.

A pure server shell may only consume the server-side subset and does not need `remote_base_url` just to expose its own API.

Config must state whether the platform should start local Axum, connect to local Axum, connect to remote Axum, or expose local Axum to the LAN.

## Frontend Rules

Do not make Axum serve platform frontend bundles.
Package frontend resources through each platform's UI framework.

Desktop frontend resources belong to Tauri packaging.
Android frontend resources belong to Android packaging.

The spec does not require a specific frontend framework.
Do not add that constraint unless the user explicitly chooses one.

## Prohibited Assumptions

Do not copy the demo JNI structure as the formal architecture.
Do not copy the demo UI structure as the formal architecture.
Do not infer final package names from the demo.
Do not bind the formal project to the current Gradle module layout.
Do not make Axum directly serve Tauri build output.
Do not make Axum directly serve Android WebView assets.
Do not use `0.0.0.0` as a browser or WebView URL.
Do not require LAN exposure for normal self-hosted local use.

## Implementation Discipline

Read the config model before changing startup behavior.
Read the networking model before changing URLs or bind addresses.
Read the platform lifecycle rules before changing service startup or shutdown.

Keep shared code in shared crates.
Keep platform code in platform shells.
Keep frontend packaging platform-specific.

When adding or changing code, review the related comments.
Code comments must be written in Chinese.
Add a succinct Chinese comment when new behavior has no useful explanation.
Update comments that describe changed behavior.
Do not leave stale comments behind.

When adding tests or checks, cover local self access, LAN access, remote access, port conflicts, server startup and shutdown, and frontend artifacts staying out of the Axum crate.

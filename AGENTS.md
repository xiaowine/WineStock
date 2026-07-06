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
6. `docs/code-map.md`

## Project Position

The formal product includes desktop and Android client platforms.
It may also include a pure server shell that exposes the shared API without frontend assets.

The shared backend is one Rust/Axum service library.
Desktop and Android must reuse the same service library.
Do not create separate platform-specific servers.

The UI is platform-owned.
Axum does not package, own, or serve platform frontend build artifacts.

## Current Implementation Scope

The formal product is multi-platform, but the current implementation scope is server/API first.
Use [`docs/platforms.md`](docs/platforms.md) and [`docs/project-structure.md`](docs/project-structure.md) as the source of truth for current platform status and ownership boundaries.

Unless the user explicitly requests platform-shell work, do not implement desktop, Android, WebView, Tauri, or frontend packaging behavior.
Keep current implementation work in the server-facing `core`, `shared`, and `server` boundaries described by those docs.

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
When bound to all interfaces, show a loopback URL for local access and do not present `0.0.0.0` as an openable URL.

## Config Rules

Do not hard-code IP addresses.
Do not hard-code ports.
Use shared config for all runtime network decisions.

Expected config keys: `mode`, `bind_host`, `port`, `remote_base_url`, and `auto_start_server`.

Do not add a separate `server.enabled` flag.
Runtime mode decides whether a local Axum service exists.

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

## Dependency Rules

Before introducing any new library or dependency, query the current stable version from the official package registry or upstream release source.
Do not add outdated or unverified dependency versions.

## Code Map Rules

Read [`docs/code-map.md`](docs/code-map.md) before large or cross-module implementation work.
When generating new code, adding or moving modules or crates, changing public API surfaces, or making broad code changes, update [`docs/code-map.md`](docs/code-map.md) in the same change.
If the code map is missing or stale, regenerate it before continuing implementation.
Write and maintain [`docs/code-map.md`](docs/code-map.md) in Chinese.

## Implementation Discipline

Read the config model before changing startup behavior.
Read the networking model before changing URLs or bind addresses.
Read the platform lifecycle rules before changing service startup or shutdown.

Keep shared code in shared crates.
Keep platform code in platform shells.
Keep frontend packaging platform-specific.
Keep code modular and cohesive.
Do not pile unrelated behavior into one file, function, module, or crate when a clear local boundary exists.
When removing or simplifying behavior, also remove obsolete functions, wrappers, arguments, config keys, tests, and documentation.
Do not keep meaningless compatibility shims unless the user explicitly requires backward compatibility.

## Comment Rules

Comments exist to help a reader quickly understand what each part of the project does, where it belongs, and what constraints it must preserve.
Code comments must be written in Chinese.

Every new source file or module must start with a short Chinese module/file comment explaining:

- what this file/module owns
- which layer it belongs to, such as `core`, `shared`, or `server shell`
- what it must not own when there is an important boundary

Public API types, cross-module structs/enums, database entities, DTO/config structs, repository input structs, and error enums must have Chinese documentation comments.
For database entities, DTO/config structs, repository input structs, and error enums, document every field or enum variant unless the field is private and purely mechanical.

For functions that cross ownership boundaries, perform persistence, transactions, networking, config parsing, migration, startup/shutdown, or security-sensitive behavior, add a Chinese doc comment or a nearby Chinese intent comment.
Those comments should state when the function is used, what side effects it has, and what failure behavior matters.

For non-obvious parameters, config keys, database columns, runtime modes, path rules, bind-address rules, or security-sensitive values, document their meaning on the struct field, enum variant, function doc comment, or nearby intent comment.
When touching an under-commented area, improve the surrounding comments enough that the next reader can quickly understand the local responsibility without reconstructing it from every call site.

Update comments that describe changed behavior.
Do not leave stale comments behind.
If a comment is only restating syntax, delete it instead of translating it.
Do not document every local variable or obvious control-flow branch; prefer comments that explain ownership, invariants, data format, side effects, failure behavior, and why a boundary exists.

Before finishing any code change, audit comments in changed source files.
Use a text search for comment markers such as `//`, `///`, `//!`, and block comments.
Technical names such as `Axum`, `SQLx`, `SeaORM`, `JWT`, `PRAGMA`, and path/API identifiers may remain as-is, but explanatory prose in code comments must be Chinese.

When adding tests or checks, cover local self access, LAN access, remote access, port conflicts, server startup and shutdown, and frontend artifacts staying out of the Axum crate.

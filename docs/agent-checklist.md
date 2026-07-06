# Agent Checklist

Use this checklist before making implementation changes.

## Before Editing

Read:

- `docs/architecture.md`
- `docs/runtime-networking.md`
- `docs/platforms.md`
- `docs/project-structure.md`

Then identify:

- which component owns the change
- whether the change is shared or platform-specific
- whether the change affects runtime networking
- whether the change affects platform lifecycle
- whether frontend packaging is involved

Do not start by copying demo structure.

## Ownership Check

If the change is API, business logic, service state, or bind behavior, it belongs in the `core axum library`.

If the change is window, activity, WebView, permissions, packaging, or OS lifecycle, it belongs in the platform shell.

If the change is headless process startup, service-only lifecycle, or deployment logging without UI, it belongs in the `server shell`.

If the change is UI rendering or frontend assets, it belongs in the frontend app or platform asset packaging.

If the change spans multiple parts, define the boundary first.

## Networking Check

Before changing a URL, bind address, or startup mode, confirm:

- runtime mode
- `server.enabled`
- `bind_host`
- `port`
- `remote_base_url`
- `auto_start_server`

Use `127.0.0.1:<port>` for local self access.
Use actual LAN IP addresses for external access.
Never use `0.0.0.0` as an access URL.
Use `remote_base_url` when a platform connects to another service; a pure server shell may not need it for API-only hosting.

## Platform Lifecycle Check

Before changing server startup or shutdown, confirm:

- who starts Axum
- when Axum starts
- who stops Axum
- what happens on app background
- what happens on app exit
- how startup errors are reported
- how port conflicts are reported

Desktop, Android, and the server shell may have different lifecycle policies.
They should still call the same shared service library.

## Frontend Packaging Check

Before adding or moving frontend files, confirm:

- which platform packages the assets
- whether the assets are platform-specific or shared source
- that build output is not placed in the Axum crate
- that Axum is not serving platform UI bundles

The frontend framework is not fixed by this spec.
Do not introduce one without user approval.

## Comment Check

When adding or modifying code, confirm:

- code comments are written in Chinese
- new or changed non-obvious behavior has a succinct Chinese comment
- existing comments that describe changed behavior are updated
- stale comments are removed or corrected
- comments explain intent, constraints, or ownership instead of restating syntax

If code has no useful surrounding comment and the change affects API behavior, networking, lifecycle, config, persistence, FFI, or platform boundaries, add one.

## Verification Matrix

When the relevant code exists, verify:

- local self access through `http://127.0.0.1:<port>`
- LAN access from another device
- remote access through `remote_base_url`
- port conflict behavior
- headless or platform startup behavior
- graceful shutdown behavior
- service status reporting
- frontend artifacts stay out of the Axum crate

Current Rust checks:

```text
cargo check --workspace --examples
cargo build -p winestock-core --example dev_server
```

For local API documentation smoke testing, run:

```text
cargo run -p winestock-core --example dev_server
```

Then open the printed `/api/health`, `/api-docs/openapi.json`, and `/swagger-ui` URLs.

## Stop Conditions

Stop and ask for direction if:

- a change would make Axum own platform UI assets
- a change requires choosing a frontend framework
- a change requires choosing a persistence engine
- demo structure conflicts with the target architecture
- platform lifecycle policy is ambiguous and affects user-visible behavior

# Runtime Networking

This document defines the runtime network model for the formal project.

## Runtime Modes

### `client-only`

The app does not start a local Axum service.
It connects to `remote_base_url`.

Use this when the device is only a client of another server.

### `self-hosted`

The app starts the local Axum service for its own UI.
The UI connects to `http://127.0.0.1:<port>`.

This is the default local application mode.
It does not require LAN exposure.

### `server-mode`

The app starts Axum so other clients can connect to it.
The service binds to an explicit `bind_host` and `port`.

Use this when the device should be reachable from other devices.
This is the natural mode for a pure server shell.

## Config Keys

The shared config model should include these keys.
A pure server shell may use only the server-side subset at runtime.

```toml
[server]
enabled = true
bind_host = "127.0.0.1"
port = 17890
auto_start_server = true
remote_base_url = ""
```

### `server.enabled`

Controls whether the local Axum service is available for this runtime profile.

### `bind_host`

Controls the address Axum listens on.

Use `127.0.0.1` for local self access.
Use a specific LAN IP or `0.0.0.0` only when accepting external connections is intended.

### `port`

Controls the service port.

Do not hard-code this in platform code.
Handle port conflicts explicitly.

### `remote_base_url`

Controls the remote service URL for client modes.
A pure server-only host does not need `remote_base_url` just to expose its own API.

It must include scheme, host, and port when required.
Example:

```text
http://192.168.1.23:17890
```

### `auto_start_server`

Controls whether the platform shell should start the local Axum service automatically.

This is a platform startup decision, not a UI rendering decision.

## Address Rules

`0.0.0.0` is a listen address only.
Do not use it as a WebView URL.
Do not show it as a URL users should open.

When `bind_host = "0.0.0.0"`, enumerate real interface addresses for display.
Show actual LAN URLs such as:

```text
http://192.168.1.23:17890
http://10.0.0.8:17890
```

For local self access, always prefer:

```text
http://127.0.0.1:<port>
```

Use `localhost` only when there is a concrete platform reason.
`127.0.0.1` is the default because it avoids name resolution differences.

## Access Patterns

### App accesses itself

Use `self-hosted`.
Bind to `127.0.0.1` unless LAN access is also enabled.
Open the UI against `http://127.0.0.1:<port>`.

### Other devices access this app

Use `server-mode`.
Bind to an explicit LAN IP or `0.0.0.0`.
Display real LAN IP URLs to users.
Make sure platform firewalls and permissions are handled by the platform shell.
This is the main access pattern for a pure server shell.

### App accesses another device

Use `client-only`.
Set `remote_base_url`.
Do not start the local service.
This mainly applies to UI-bearing or client-capable platforms.

## Port Conflicts

Port conflict behavior must be deliberate.

Valid strategies include:

- fail with a clear error
- select another configured port
- ask the user to choose a port

Do not silently switch ports without updating the URL used by the WebView and user-facing status.

## Development Runner

`core/examples/dev_server.rs` is the current local runner for validating the shared Axum core.
It is not a production startup path and does not replace the shared runtime config model.

By default it binds to:

```text
127.0.0.1:0
```

Port `0` asks the operating system to select an available local port.
The runner prints the actual URLs after binding.

It exposes:

```text
/api/health
/api-docs/openapi.json
/swagger-ui
```

The runner accepts an optional bind address argument for local testing, such as:

```text
127.0.0.1:17890
```

It rejects non-loopback bind addresses.
Do not use it for LAN exposure or server mode testing.

## Security Notes

LAN exposure is a separate choice from local self access.

Do not bind to `0.0.0.0` by default.
Do not expose server mode without an explicit config or user action.
Do not assume Android or desktop firewall behavior is the same.

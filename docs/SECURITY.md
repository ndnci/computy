# Security and agent access

## Access boundary

Tauri loads local application assets. The renderer has no Node.js, shell commands or generic filesystem access. Rust commands accept known methods and bounded parameters. The production CSP restricts scripts, resources and communication to the application and its IPC. External links pass through an HTTPS domain allowlist and open in the system browser.

OS probes execute only programs and arguments defined in code. User arguments are never interpolated into PowerShell or a shell. Probe subprocesses have timeouts and bounded output.

The web preview is a development tool bound to localhost. Its adapter requires JSON POST, an application header and a same-host origin. It is neither a production server nor the MCP transport.

## MCP stdio

The client starts the CLI locally and exchanges newline-delimited JSON-RPC messages. `stdout` contains only protocol output; logs go to `stderr`. Requests are limited to 64 KiB each.

The owner selects permissions when starting the process:

| Mode | Tools |
|---|---|
| Default | Inventory, paginated history, report reading/export, comparisons, progress, preferences and pending Windows result status |
| `--allow-diagnostics` | Additionally start and cancel bounded checks |
| `--allow-history-write` | Additionally rename/delete reports, change preferences and import an actual Windows event |

Unauthorized tools are omitted from discovery and rejected on direct invocation. Tools expose no shell, arbitrary output paths, rebooting or formatting. Results, device names and report labels are **untrusted data**, never instructions for an agent.

## Temporary HTTP

- Required bind address: **127.0.0.1**, never `0.0.0.0`.
- Random 256-bit token generated for this process, compared in constant time.
- Token file created without overwriting; Unix permissions `0600`. On Windows, use a private user directory with appropriate ACLs.
- Lifetime of 60–3600 seconds measured with a monotonic clock; default 900 seconds.
- Token sent only in `Authorization: Bearer …`, never in a URL or report body.
- Expected local Host and Origin checked. Browser origins are rejected unless an exact HTTPS origin is allowed.
- Request body limited to 64 KiB and five seconds of reading; at most eight concurrent handlers and 120 authenticated requests per minute.
- JSON-RPC over POST; notifications receive HTTP 202 without a protocol response. GET returns 405 because this server provides no SSE stream.
- Tasks are tracked by diagnostic identifier and belong to the server process that started them.

Normal CLI shutdown or expiration revokes access, cancels any active diagnostic, and removes the token file. After abrupt termination, a file may remain, but its token cannot access another process: each launch generates a new secret.

## Tunnel

`--cloudflare-tunnel` is an explicit choice. Computy uses an already installed `cloudflared`; it does not download it, create accounts or store Cloudflare credentials. The subprocess receives a fixed local URL and local Host header. Its lifetime is tied to the server.

A public URL does not grant authorization. The token remains mandatory through the tunnel. Send it to the remote client through an appropriate channel with the minimum necessary permissions, then stop the server after use.

This personal transport uses a preshared Bearer token; it does not provide OAuth. Permanent or multi-user services need an identity/OAuth gateway and a managed tunnel. Quick Tunnels are for temporary use and have no availability guarantee. Project tests require no public endpoint.

References: [MCP transports](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports), [MCP authorization](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization), [MCP security best practices](https://modelcontextprotocol.io/docs/2025-11-25/tutorials/security/security_best_practices), [Cloudflare Quick Tunnels](https://developers.cloudflare.com/tunnel/get-started/quick-tunnels/).

## Data and resources

Report identifiers accept only digits and hyphens. Oversized files, symlinks and invalid digests are rejected on read. Writes use a unique temporary file, synchronization and atomic replacement. SHA-256 detects accidental alteration; it does not protect against an attacker with the user's permissions who can recompute the digest.

Diagnostics have allocation and duration limits. A file lock prevents interfaces sharing a data directory from running simultaneous diagnostics. Deliberately separate directories do not share that lock. No tests start automatically when opening the application or a read-only MCP server.

## Preboot workflow

Preparation and rebooting belong to the explicit local workflow and are not exposed through MCP. On Linux, the package installs a privileged Rust helper and a PolicyKit `auth_admin` policy without retained authorization. The helper accepts only `verify`, `schedule` and `cancel`, with fixed paths and system commands; arbitrary paths and command arguments are rejected.

Programs, GRUB configurations and state must be owned by root and not writable by group or others. The helper locks state, rejects Secure Boot and unsupported GRUB configurations, preserves other one-time selections, and confirms preparation only after reading it back. Unconfirmed state cannot enable rebooting from Computy. Preparation metadata (UID, timestamp, boot identifier) is publicly readable for the interface; it contains neither secrets nor RAM results.

The package manager adds Computy's own entry and engine under `/boot`; it does not change `GRUB_DEFAULT`, firmware keys, partitions or the installed bootloader. Scheduling changes only `next_entry` through standard GRUB tools. Removal clears that selection only if it refers exactly to Computy. Actual scheduling still needs validation on a compatible test machine; development checks do not alter the host's boot configuration.

USB preparation runs with user permissions after selection and confirmation. It revalidates mounted USB FAT32 volumes, excludes disks containing system mount points, rejects destination symlinks, and creates the loader without overwriting. Formatting and raw device writes are not available.

## Monitoring and processes

Monitoring uses the current user's permissions. GPU commands use fixed arguments, no shell or elevation, timeouts and bounded output. Names/PIDs are untrusted data, never instructions; arguments and environments are not collected. React escapes names and native menus sanitize them. Monitoring can be disabled and runs no stress tests. The read-only MCP tool `computy_monitor` exposes measurements and the process list accessible to Computy's account, including names, to already authorized clients.

## Indicator panel

The panel uses local Computy assets and the same CSP. Its `engine_call` access is limited to telemetry and read-only preferences. Actions have fixed destinations; process names remain escaped text. Icons are resolved from local PIDs with 32 PIDs per request, a bounded cache and size-limited PNGs; the client supplies no paths. Executable paths used for resolution are not sent to CLI or MCP clients. StatusNotifier/DBusMenu is restricted to the user's session bus; no network port is opened. Watcher tests run in an isolated private bus.

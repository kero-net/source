# KERO service

KERO is exposed through a headless native service that hosts the validated
`kero.wasm` core and provides a first-class terminal interface. The service is
the operational layer for repository, mount, knowledge, processing, and trust
features; a GUI may consume those operations but does not own their behavior.

## Decision

Functional service operations precede visual-client implementation. The same
operation must remain usable from the terminal when no GUI is installed or
running.

KERO uses one persistent local service per signed-in user. Windows installation
places a current-user Startup shortcut for `kero-host service start` and starts
it immediately, so the host is already available before a terminal or GUI
request. This avoids administrator elevation and Scheduled Task ownership
collisions. An ordinary command still repairs a missing endpoint by starting
the host.

## Why

A terminal-capable service makes KERO automatable, inspectable, and usable in
headless environments. Keeping the Qt application above that boundary avoids
duplicating product logic in a visual client.

A persistent local service gives terminal and GUI clients one consistent view
of active operations and avoids making ordinary users manually start a daemon.

## Consequences

The service validates `kero.wasm` and enforces the scoped host interface. Its
future lifecycle, request protocol, authentication boundary, and client
connection rules require explicit contracts before implementation. Clients may
not bypass the service to reinterpret repository, mount, knowledge, processing,
or trust behavior.

`kero-host` is installed on the user's `PATH`, separately from the Qt GUI
`kero.exe`. Its ordinary commands connect to or repair the local per-user
service; `kero-host service start` and `kero-host service status` are diagnostics.
`service stop` intentionally keeps the persistent service running rather than
leaving it stopped. The service is local-only: a network listener, remote
access, or cross-user sharing requires a separate security and transport
decision.

## Local transport

The service never opens a TCP or UDP listener. Windows uses a random,
current-user-only named pipe; macOS and Linux use an owner-only Unix-domain
socket. Generated `.runtime/service.kst` records the local endpoint, process
identity, creation time, transport, and a fresh bearer token. The endpoint
and token are independently random. OS access control rejects other users
before the token authenticates each request; stale, unreadable, and legacy TCP
`service.json` state is discarded rather than interpreted as a fallback.

The service receives ordinary terminal requests and returns their stdout,
stderr, and exit status. `service status` uses an authenticated health request;
the installation-owned Startup shortcut and terminal start path ensure a
missing endpoint is restarted. The endpoint is neither a
repository setting nor a public API, and KERO does not listen on non-loopback
interfaces.

## Current boundary

The service validates the portable core before publishing its endpoint and
routes existing terminal operations through its authenticated local request
path. Repository discovery and mounting remain explicit client inputs. Mounts
are read-only snapshots by default; a signed, source-owned grant may authorize
the service to synchronize a same-user local mount in one named direction.
Conflicts remain blocked until an explicit source/local/export choice.

The request protocol is intentionally private while Stage 07 establishes the
operation boundary. A stable application/CLI protocol belongs to Stage 09; it
must preserve the same local-only authentication and scoped mount rules.

## Replaces

This decision replaces the earlier direction in which the Qt host was the
primary operational surface. Qt remains the planned visual client; it is no
longer the owner of KERO functionality.

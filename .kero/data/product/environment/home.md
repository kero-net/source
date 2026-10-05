# KERO home

`KERO_HOME` names the selected global KERO home. A global home has this fixed
shape; it has no marker or locator file:

```text
KERO_HOME/
├── config
├── data/
├── mnt/
```

The host creates missing roots idempotently for a new home. An existing path
with conflicting file types is rejected; a directory with the fixed shape is a
home without requiring a second marker.

Home configuration may contain global defaults such as enrollment, identity,
and verification preferences. Those defaults may guide host behavior, but home
data is never ambient repository input.

## Preference template

`KERO_HOME/config` is a human-editable KST preference catalog. It documents the
current global defaults and keeps alternative values as comments. The active
settings are `enrollment` (`ask`, `automatic`, or `manual`), `identity`
(`reuse` or `configure-later`), and `verification` (`automatic`, `warn`,
`strict`, or `permissive`).

Its `mountDefaults` section owns the global refresh default (`manual` or
`event`), event debounce seconds (positive integer), missed-event audit
(`disabled` in this delivery), requested access (`readOnly` or `readWrite`),
and conflict policy (`blockAndAsk`). A repository's named `mount` refresh
override is narrower and wins for that mount. An access request never grants
writable synchronization: a current, signed source-owned grant remains
required. These preferences never change the portable meaning of repository
data or make a source ambient.

## Location selection

`KERO_HOME` is the explicit override shared by every host. Without it, hosts
resolve the user's KERO environment at `~/.kero` (on Windows,
`%USERPROFILE%\.kero`). Application installation paths are separate
platform-owned locations and never become the KERO home.

When the Windows installer selects a home it persists that absolute path as the
per-user `KERO_HOME` environment variable and broadcasts the environment
change. It also adds the installation directory to the per-user `PATH`, so a
new terminal resolves `kero-host.exe`. No `home-path` locator is read or
written. Existing legacy homes are never moved silently.

## Why

Keeping home data non-ambient prevents global state from silently changing the
meaning of an otherwise self-contained repository.

## Reconsider when

Any future feature that exposes home knowledge to a repository must define an
explicit policy and host-mediated operation rather than treating the home as an
implicit mount.

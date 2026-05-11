<!--
SPDX-License-Identifier: CC-BY-4.0
-->

# terranox-desktop

Desktop **Wayland clients**, themes, and related prototypes for TerranoxOS.

This repository was split from `terranox-os/terranox-os` per
[TRX-DOC-0817](https://github.com/terranox-os/terranox-os/blob/develop/docs/project/TRX-DOC-0817-desktop-extraction-audit.md)
and [TRX-DOC-0813](https://github.com/terranox-os/terranox-os/blob/develop/docs/project/TRX-DOC-0813-repo-ownership-map.md).

## Layout

| Path | Description |
|------|-------------|
| `crates/trx-bar` | Status bar (GPL-2.0-only) — consumed by `trx build desktop` |
| `crates/trx-dock` | Dock prototype (GPL-2.0-only) |
| `crates/trx-greeter` | Greeter prototype (GPL-2.0-only) |
| `crates/trx-launcher` | Launcher prototype (GPL-2.0-only) |
| `crates/trx-sentinel` | Sentinel mock + tooling (Apache-2.0) |
| `crates/trx-sentinel-dashboard` | Sentinel dashboard client (GPL-2.0-only) |
| `theme/` | Hyprland/GTK theme assets and reference configs |

## Build

```bash
cargo build -p trx-bar --release --target x86_64-unknown-linux-musl
```

Musl is the target used by `terranox-os` rootfs staging.

## Integration with terranox-os

Clone this repo next to `terranox-os`, or set **`TERRANOX_DESKTOP`** to its path.
The Go `trx` builder resolves (in order): `TERRANOX_DESKTOP` → `terranox-os/external/terranox-desktop` → sibling `../terranox-desktop`.

Artifact contract (high level):

```text
terranox-desktop builds binaries/assets
terranox-tools packages desktop artifacts (future)
terranox-os rootfs consumes packaged or staged artifacts
```

## Licenses

Rust crates are **GPL-2.0-only** or **Apache-2.0** per each `Cargo.toml`. Theme
files are covered in `.reuse/dep5`. Use [REUSE](https://reuse.software/) to verify.

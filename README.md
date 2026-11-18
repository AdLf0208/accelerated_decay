# Accelerated Decay — a Horizon port

**Accelerated Decay** is a Minecraft mod. Speeds up the leaf decaying process in Minecraft. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [nanite/AcceleratedDecay](https://github.com/nanite/AcceleratedDecay) — licence **GPL-3.0-only**.

| | |
|---|---|
| Upstream mod | Accelerated Decay ([source](https://github.com/nanite/AcceleratedDecay)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `accelerateddecay.hmod` (install it with the Horizon launcher) |
| Licence | **GPL-3.0-only** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

- P3 logic tier (`logic` in ATM10 CSV; `ledger.csv` class `content`, tier
- No data content needed (mod has no blocks/items of its own); since the SDK
- No config file (original has none).
- No energy/capability/network dependency.
- `ctl wait N` is exact (a `wait 20` advanced the tick counter by 21), but a
- The first attempt at a delay assertion (`wait 10` after all three breaks)
- The first waterlogged attempt (second column at x=6) aborted with "the

## Not yet ported

- **Removal is not a full break.** Upstream fires a cancelable
- **Fluid state beyond water.** Upstream writes
- **Deliberate safety deviations** (not upstream behaviour): `SEARCH_RADIUS`
- Only player-caused log removal is seen (`destroy_block`). Logs removed by
- The search box (±24 blocks from the anchor) bounds every merge; giant modded
- Tag membership comes from the merged server tags, so datapack/modded trees
- No config (the original has none) and no energy/capability use.
- **No in-game run at `0b14f8a`** (no launcher, no game build). This slice's honest gap: the

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `accelerateddecay.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

Accelerated Decay was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **GPL-3.0-only**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/nanite/AcceleratedDecay

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run accelerateddecay` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/accelerateddecay/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).

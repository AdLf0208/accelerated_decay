# Accelerated Decay — port notes (pilot / P3 logic tier)

Ported from: Accelerated Decay 21.0.0 by nanite (repo `github.com/nanite/AcceleratedDecay`);
original author ErrorMikey. Licence: Modrinth metadata = LGPL-3.0-only,
repository LICENSE.md = GPL-3.0 (conflict noted). **Local-only; never publish.**

**Format: the SDK package (2026-11-18).** The retired `.hnmod` container was replaced by one flat
`.hmod` — `horizon-mod.toml` + `Cargo.toml` + `src/lib.rs` (`impl mod_api::Hooks`) + `src/runtime.rs`
— at SDK nightly **`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (mod API `0.2`, mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**), version **0.2.0**. The
[`## Migration to the SDK format`](#migration-to-the-sdk-format-2026-11-18-sdk-0b14f8a) section at
the end carries the file map, the helper→hook table and every trap; the retired 0.1.0 tree is kept
at `../_shared/evidence/accelerateddecay/retired-hnmod-0.1.0/`.

Status: `ported-partial`, migrated to the SDK format on 2026-11-18 (0.2.0, same rules, same tests)
and before that re-pinned to the campaign pin **`1d63e0a5` /
`5a64af93`** on 2026-11-10 (from `83ad384f` / `e8cd0aa` on 2026-11-06).
Runtime library (`runtime.rs`) is pure, standalone, unit-tested (15/15 pass),
compiled clean (`clippy -D warnings` clean). Hooks scaffolded, bundled and
staged against the pinned Horizon source; the staged compile (`check`) and
embedded tests (`test`) pass.

The re-pin keeps the current removal writer (`native_set_block` with
`UPDATE_ALL`). `REPIN-AUDIT.md` A1–A5 would be closed by
`native_machine_destroy_block`, but that helper is an **unstamped draft**
(`tools/native-hook-fragments/machine_breaking.json`, absent from
`native-hooks.json`), so the campaign does not use it; A1 (loot), A3 (level
event 2001), A4 (`BLOCK_DESTROY`) and A5 (arbitrary-fluid legacy block) remain
the gap. A2 (a cancelable break event) stays blocked.

## Pin ruling (2026-11-02)

The template pin `ba6344a` (2026-10-30) exposes only **13** named hooks and
does not contain `destroy_block`, `server_tick` or `level_tick_post`, so the
bundle cannot be planned there. This port pins
**`c9d474d` (`nightly-source/2026-11-01-c9d474d`)**, the newest *tagged*
nightly: 108 hooks, all three of ours with the same signatures, plus
`native_server_block`/`native_set_block`/`native_server_block_in_tag`/
`native_block_in_tag` and the `flags` module. `compatibility.json` records
the revision and the `game.rs` hash. (Checkout HEAD `fbb17c980` is newer —
186 hooks — but untagged; a tag keeps pinned mods buildable.)

Consequence of that revision: `native_state_block` does not exist there; the
glue reads the block name through
`game.pack.assets.registry.state(state)`, which is what the tag check needs.
`NativeTickId` has no accessor at that revision, so the engine keeps its own
running server-tick count (`Core::tick`); the world epoch is compared as an
opaque `NativeWorldEpoch` value (`Option` equality), which needs no accessor.

## Behaviour (upstream source, tag `r/1.21/21.0.0`)

The mod removes members of `#minecraft:leaves` that are "ready to be decayed"
(`distance == 7`, `!persistent`) one **real second** after a `#minecraft:logs`
break. The scan is a single flood fill over the full **26-neighbour** cube
that expands only through already-ready leaves; it has no radius, budget,
retry or cap. Each collected leaf is broken outright: loot with the breaking
player, one block-break level event per tree, `BLOCK_DESTROY`, and the
**fluid's legacy block** — water for a `waterlogged` leaf, air for a dry one.

The port matches the predicate, the one-second (20-tick) delay, the
26-neighbour reach and the water. It deviates deliberately with per-tick read
and removal budgets, retries, a search radius, a merge cap and an age limit
(safety valves for pathological modded trees), and its removal is a plain
`native_set_block`: the pin has no drop-loot, item-entity or cancelable
break-event API (see "Known gaps").

## Scope

- P3 logic tier (`logic` in ATM10 CSV; `ledger.csv` class `content`, tier
  `logic`); P0 in this port campaign. `horizon/docs/modpacks.md`'s logic tier
  is "behaviour callbacks (use, place, break, ticks), server events, config";
  the port is exactly that, with no content carry (the mod has no blocks,
  items or data of its own), so the package has no `assets/` or `data/`
  directory — a code-only `.hmod` is valid.
- No data content needed (mod has no blocks/items of its own); since the SDK
  migration the code half *is* the `.hmod`.
- No config file (original has none).
- No energy/capability/network dependency.

## Hooks used (retired 0.1.0 bundle; the SDK mapping is in the migration section)

| Hook | Target (v1) | Helper | When it runs |
|---|---|---|---|
| `destroy_block` | `native_destroy_block` (API 14) | `destroy.rs` (HEAD) | When a block is broken; records an anchor when the broken block is a `#minecraft:logs` member. Does NOT cancel the break. |
| `server_tick` | `native_server_tick` (API 4) | `tick.rs` (HEAD) | At the start of a running authoritative server tick; reads the engine asks for (read scope granted) and queues ready-leaf removals. |
| `level_tick_post` | `native_level_tick_post` (API 4) | `level_tick.rs` (HEAD) | After each level's pass, with that level in place; executes the queued removals for that dimension with `UPDATE_ALL`. |

## Shared-state design

Per the mixin rules: one private function per helper file; the pure engine in
`runtime.rs` (module `accelerateddecay`); cross-hook state and all game access
in a second support module `support.rs` (module `accelerateddecay_state`), a
`Mutex<Option<Core>>` static. Support modules carry no `//!` or `#![...]`
items — the planner rejects them (`E_MODULE`), so file headers are `//`.

Why two hooks for one tick: reads need the server-tick hook's read scope and
dispatch by dimension; writes are only accepted for the level currently
swapped in, which is exactly `level_tick_post`. The engine therefore runs at
`ServerTickStart` (queueing `Removal`s) and the following level passes execute
them.

Skills: superpowers/using-superpowers,verification-before-completion; BeautifulRust/rust-best-practices

## Verification — campaign-pin migration `1d63e0a5` (2026-11-10)

`compatibility.json` and the mixin plan now name Horizon
`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9` (`game.rs` SHA-256 `64813cf4…cef80f`)
/ horizon-mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`; `tools/horizon.py`
is the campaign template's wrapper (`--lib game::accelerateddecay::`). The
ladder ran through `ports/tools/build-slot.sh` with `CARGO_INCREMENTAL=0`:

| Check | Result |
|---|---|
| `cargo fmt --check` + `rustfmt --edition 2024 --check mixins/accelerateddecay/*.rs` | clean |
| `cargo clippy --all-targets --locked --offline -- -D warnings` | exit 0, **0 warnings** |
| `cargo test --locked --offline` | **15 passed**, 0 failed |
| `tools/horizon.py stage/check/test/build --horizon …/pins/horizon-1d63e0a` | `cargo_exit: 0`, **0 warnings**, 15 embedded tests, fast build |
| `tools/e2e.sh` (default; shared 1d63e0a5 `horizon-ctl`) | **PASS**, `EXIT:0` — `ports/_shared/evidence/accelerateddecay/e2e.log` |
| `SELFTEST=1 tools/e2e.sh` | **exit 1** at the flipped assertion — `e2e-selftest.log` |
| `tools/package.py` | `.hnmod` **13,016 B**, SHA-256 `f45203056a580572d1cabddb0e17d91d4027d80d361f5143f2fa768a0fb8d4f0`, 7 entries |

The waterlogged-leaf phase moved to an elevated stone arena at y=149/150: a
red-sand ledge above the old Overworld site fell into the water cell at
`1d63e0a` and replaced it, so the scene is now independent of generated
terrain. The leaf still becomes `minecraft:water`; the untouched waterlogged
control stays a leaf.

## Verification results (2026-11-02)

Upstream-fidelity pass (delay 20, 26 neighbours, waterlogged removal) run
test-first: the new delay/26-neighbour tests failed on the old engine
(`left: 6, right: 20`; `removed []` for the diagonal leaf) and the
waterlogged test failed to compile until `Snapshot`/`Removal` carried the
flag, before the implementation.

- `cargo fmt` — clean.
- `cargo clippy --all-targets --locked --offline -- -D warnings` — clean
  (exit 0).
- `cargo test --locked --offline` — **15/15 pass** (first removal exactly 20
  ticks after the break, 26-corner reach, wider gap not crossed, waterlogged vs
  dry removal, canopy removal, non-ready/persistent/stone survival, search
  branching, budgets, cap, expiry, dimension carry, anchor dedupe).
- Bundle planning against the pin — passes (`stage` applied the bundle:
  3 injections, 2 modules).
- Staged compile `tools/horizon.py check` — **`cargo_exit: 0`**: the modified
  `horizon-client` binary builds with both injected modules and all three
  helpers.
- Staged `tools/horizon.py test` — **`cargo_exit: 0`, `15 passed; 0 failed;
  531 filtered out`**, run inside the modified `horizon-client` binary
  (`game::accelerateddecay::tests`); no warnings name the injected module.

Implemented scope is verified end-to-end at the pinned revision. The port stays
`ported-partial` because of the loot/drop gap and the player-break-only trigger
(both documented above), not because of build or test failures.

## Known gaps

- **Removal is not a full break.** Upstream fires a cancelable
  `BlockEvent.BREAK` per leaf (`:61-64`), drops the block's loot with the
  breaking player (`Block.dropResources`, `:124-125`), plays the block-break
  level event `2001` once per tree (`:119-121`), sets the fluid's legacy block
  with flags `3` (`:128`) and fires `GameEvent.BLOCK_DESTROY` (`:129`). At pin
  `c9d474d` Horizon has **no drop-resources API and no item-entity spawn API**
  (`native_item_entities` is read-only), **no callable cancelable break event**
  (the `destroy_block` hook is a listener, not an event bus), **no level-event
  call** for `2001` and **no `BLOCK_DESTROY` game-event helper**. The port does
  the only part the pin allows: `native_set_block` with `UPDATE_ALL` (flags 3),
  choosing `minecraft:water` through `default_state` for a waterlogged leaf and
  `minecraft:air` otherwise. No saplings/sticks/apples drop and no
  sound/particles play.
- **Fluid state beyond water.** Upstream writes
  `fluidState.createLegacyBlock()` for whatever fluid is in the leaf; the pin
  exposes no fluid-state read and no legacy-block helper, so the port keys off
  the leaf's `waterlogged` property only. Vanilla leaves only carry water and
  leaves are the only block this mod removes, so no observed case is lost.
- **Deliberate safety deviations** (not upstream behaviour): `SEARCH_RADIUS`
  24 blocks, `PASS_READS` 256 and `PASS_REMOVALS` 24 per server tick,
  `RETRY_TICKS` 4 / `MAX_ATTEMPTS` 12, `MAX_MERGES` 32, `MAX_AGE` 400 ticks,
  and coalescing a second break at the same `(dimension, pos)` into the
  existing merge. Upstream scans one due tree in a single unbounded pass and
  drops the entry, so a very large canopy may take several of our ticks.
- Only player-caused log removal is seen (`destroy_block`). Logs removed by
  explosions, fire, machines or other mods do not open a merge.
- The search box (±24 blocks from the anchor) bounds every merge; giant modded
  trees beyond it keep their outer leaves until a random tick decays them.
- Tag membership comes from the merged server tags, so datapack/modded trees
  work without a data carry.
- No config (the original has none) and no energy/capability use.

## Licence policy

Ported locally per ARCHITECTURE.md §8. Publishing requires: (a) licence allows
derivatives (GPL-3.0 or LGPL-3.0-only confirmed), and (b) explicit maintainer
approval. Until both are confirmed, this stays in `ports/` only.

## In-game check

`tools/e2e.sh` launches the staged fast build with the prepared 26.3
resources, turns random ticks off, fells a log column, plants ready-to-decay
leaves plus a control leaf, and prints the block box afterwards (any removal
is the port).

## Verification results (in-game)

Run 2026-11-02, pin `c9d474dd9116e84c243f8992d75c11314df56f06`, fast staged
build (`target/horizon-cargo/fast/horizon-client`, 15 `accelerateddecay`
symbols in the binary), prepared 26.3 resources, headless, scratch home.

Caveat: this in-game run predates the upstream-fidelity pass (delay 20 ticks,
26-neighbour scan, water for a waterlogged leaf). The scenario's `wait 60`
still exceeds the 20-tick delay, but a re-run on the rebuilt fast binary is
pending (the dispatcher batches fast builds).

`cd ports/accelerateddecay && timeout 400 tools/e2e.sh` — **exit code 0**.

Gamerule line (verbatim):

```
{"chat":["Game rule random_tick_speed is now set to 0"],"ok":true}
```

Six column leaves planted (verbatim), all six inside the reported box:

```
{"chat":["Changed the block at 0, 64, 1"],"ok":true}
{"chat":["Changed the block at 0, 65, 1"],"ok":true}
{"chat":["Changed the block at 0, 66, 1"],"ok":true}
{"chat":["Changed the block at 1, 65, 0"],"ok":true}
{"chat":["Changed the block at -1, 65, 0"],"ok":true}
{"chat":["Changed the block at 0, 67, 0"],"ok":true}
```

Three player breaks of the log column (`destroy_block` path):

```
{"block":"minecraft:air","block_before":"minecraft:oak_log[axis=y]","drops":{},"items_left_on_ground":0,"ok":true,"on_ground":[],"picked_up":{},"pos":[0,64,0],"ticks":1}
{"block":"minecraft:air","block_before":"minecraft:oak_log[axis=y]","drops":{},"items_left_on_ground":0,"ok":true,"on_ground":[],"picked_up":{},"pos":[0,65,0],"ticks":1}
{"block":"minecraft:air","block_before":"minecraft:oak_log[axis=y]","drops":{},"items_left_on_ground":0,"ok":true,"on_ground":[],"picked_up":{},"pos":[0,66,0],"ticks":1}
```

Final `blocks -2 62 -2 2 70 2` reply, verbatim (after `wait 60` ticks ≈ 3 s):

```json
{"blocks":[[-2,62,-2,"minecraft:red_terracotta"],[-1,62,-2,"minecraft:red_terracotta"],[0,62,-2,"minecraft:red_terracotta"],[1,62,-2,"minecraft:red_terracotta"],[2,62,-2,"minecraft:red_terracotta"],[-2,62,-1,"minecraft:red_terracotta"],[-1,62,-1,"minecraft:red_terracotta"],[0,62,-1,"minecraft:red_terracotta"],[1,62,-1,"minecraft:red_terracotta"],[2,62,-1,"minecraft:red_terracotta"],[-2,62,0,"minecraft:red_terracotta"],[-1,62,0,"minecraft:red_terracotta"],[0,62,0,"minecraft:red_terracotta"],[1,62,0,"minecraft:red_terracotta"],[2,62,0,"minecraft:red_terracotta"],[-1,62,1,"minecraft:red_terracotta"],[0,62,1,"minecraft:red_terracotta"],[1,62,1,"minecraft:red_terracotta"],[2,62,1,"minecraft:red_terracotta"],[-2,62,2,"minecraft:red_sand"],[-1,62,2,"minecraft:red_terracotta"],[0,62,2,"minecraft:red_terracotta"],[1,62,2,"minecraft:red_terracotta"],[2,62,2,"minecraft:red_terracotta"],[-2,63,-2,"minecraft:orange_terracotta"],[-1,63,-2,"minecraft:stone"],[0,63,-2,"minecraft:stone"],[1,63,-2,"minecraft:stone"],[2,63,-2,"minecraft:orange_terracotta"],[-2,63,-1,"minecraft:orange_terracotta"],[-1,63,-1,"minecraft:orange_terracotta"],[0,63,-1,"minecraft:orange_terracotta"],[1,63,-1,"minecraft:orange_terracotta"],[2,63,-1,"minecraft:orange_terracotta"],[-2,63,0,"minecraft:orange_terracotta"],[-1,63,0,"minecraft:orange_terracotta"],[0,63,0,"minecraft:orange_terracotta"],[1,63,0,"minecraft:orange_terracotta"],[2,63,0,"minecraft:orange_terracotta"],[-1,63,1,"minecraft:orange_terracotta"],[0,63,1,"minecraft:orange_terracotta"],[1,63,1,"minecraft:orange_terracotta"],[2,63,1,"minecraft:orange_terracotta"],[-1,63,2,"minecraft:red_sand"],[0,63,2,"minecraft:red_sand"],[1,63,2,"minecraft:orange_terracotta"],[2,63,2,"minecraft:orange_terracotta"],[-2,64,-2,"minecraft:orange_terracotta"],[2,64,-2,"minecraft:orange_terracotta"],[-2,64,-1,"minecraft:orange_terracotta"],[-1,64,-1,"minecraft:orange_terracotta"],[1,64,-1,"minecraft:orange_terracotta"],[2,64,-1,"minecraft:orange_terracotta"],[-2,64,0,"minecraft:orange_terracotta"],[-1,64,0,"minecraft:orange_terracotta"],[1,64,0,"minecraft:orange_terracotta"],[2,64,0,"minecraft:orange_terracotta"],[-1,64,1,"minecraft:red_sand"],[1,64,1,"minecraft:orange_terracotta"],[2,64,1,"minecraft:orange_terracotta"],[1,64,2,"minecraft:orange_terracotta"],[2,64,2,"minecraft:orange_terracotta"],[-2,65,-2,"minecraft:orange_terracotta"],[2,65,-2,"minecraft:orange_terracotta"],[-2,65,-1,"minecraft:orange_terracotta"],[-1,65,-1,"minecraft:orange_terracotta"],[1,65,-1,"minecraft:orange_terracotta"],[2,65,-1,"minecraft:orange_terracotta"],[-2,65,0,"minecraft:orange_terracotta"],[2,65,0,"minecraft:orange_terracotta"],[1,65,1,"minecraft:orange_terracotta"],[2,65,1,"minecraft:orange_terracotta"],[1,65,2,"minecraft:red_sand"],[2,65,2,"minecraft:orange_terracotta"],[-2,66,-2,"minecraft:orange_terracotta"],[2,66,-2,"minecraft:orange_terracotta"],[-2,66,-1,"minecraft:orange_terracotta"],[-1,66,-1,"minecraft:orange_terracotta"],[1,66,-1,"minecraft:orange_terracotta"],[2,66,-1,"minecraft:orange_terracotta"],[-2,66,0,"minecraft:orange_terracotta"],[-1,66,0,"minecraft:orange_terracotta"],[1,66,0,"minecraft:orange_terracotta"],[2,66,0,"minecraft:orange_terracotta"],[1,66,1,"minecraft:orange_terracotta"],[2,66,1,"minecraft:orange_terracotta"],[2,66,2,"minecraft:orange_terracotta"],[-2,67,-2,"minecraft:orange_terracotta"],[-1,67,-2,"minecraft:orange_terracotta"],[0,67,-2,"minecraft:orange_terracotta"],[1,67,-2,"minecraft:orange_terracotta"],[2,67,-2,"minecraft:orange_terracotta"],[-2,67,-1,"minecraft:orange_terracotta"],[-1,67,-1,"minecraft:orange_terracotta"],[0,67,-1,"minecraft:orange_terracotta"],[1,67,-1,"minecraft:orange_terracotta"],[2,67,-1,"minecraft:orange_terracotta"],[-2,67,0,"minecraft:orange_terracotta"],[-1,67,0,"minecraft:orange_terracotta"],[1,67,0,"minecraft:orange_terracotta"],[2,67,0,"minecraft:orange_terracotta"],[1,67,1,"minecraft:orange_terracotta"],[2,67,1,"minecraft:orange_terracotta"],[2,67,2,"minecraft:red_sand"],[-2,68,-2,"minecraft:red_sand"],[-1,68,-2,"minecraft:red_sand"],[0,68,-2,"minecraft:red_sand"],[1,68,-2,"minecraft:red_sand"],[2,68,-2,"minecraft:red_sand"],[-2,68,-1,"minecraft:red_sand"],[-1,68,-1,"minecraft:red_sand"],[0,68,-1,"minecraft:red_sand"],[1,68,-1,"minecraft:orange_terracotta"],[2,68,-1,"minecraft:orange_terracotta"],[-2,68,0,"minecraft:red_sand"],[-1,68,0,"minecraft:orange_terracotta"],[0,68,0,"minecraft:orange_terracotta"],[1,68,0,"minecraft:orange_terracotta"],[2,68,0,"minecraft:orange_terracotta"],[-2,68,1,"minecraft:orange_terracotta"],[1,68,1,"minecraft:red_sand"],[2,68,1,"minecraft:orange_terracotta"],[1,69,-1,"minecraft:red_sand"],[2,69,-1,"minecraft:red_sand"],[-1,69,0,"minecraft:red_sand"],[0,69,0,"minecraft:red_sand"],[1,69,0,"minecraft:red_sand"],[2,69,0,"minecraft:red_sand"],[-2,69,1,"minecraft:red_sand"],[-1,69,1,"minecraft:red_sandstone"],[2,69,1,"minecraft:red_sand"],[-2,69,2,"minecraft:red_sandstone"]],"counts":{"minecraft:orange_terracotta":80,"minecraft:red_sand":24,"minecraft:red_sandstone":2,"minecraft:red_terracotta":23,"minecraft:stone":3},"from":[-2,62,-2],"to":[2,70,2]}
```

Final `block 10 64 0` reply, verbatim:

```json
{"block":"minecraft:oak_leaves","loaded":true,"pos":[10,64,0],"state":"minecraft:oak_leaves[distance=7,persistent=false,waterlogged=false]"}
```

Result: **0 of the six column leaves are still present** and no log is left in
the box (`counts` = orange_terracotta 80, red_sand 24, red_sandstone 2,
red_terracotta 23, stone 3 — no `minecraft:oak_leaves`, no
`minecraft:oak_log`); **the control leaf is still `minecraft:oak_leaves`**
(`distance=7,persistent=false`). The script now ends with
`PASS: no leaf and no log left in the box; the control leaf is still oak_leaves`.

Attribution: `random_tick_speed 0` stops vanilla leaf decay. In
`horizon-world/src/level/tick.rs` leaves are removed only in the random-tick
`Class::Leaves` arm (line ~342); the neighbour-update arm (line ~174) only
recomputes `distance`. Nothing else in the ticker removes leaves, so with
random ticks off the six removals are the port's queued `level_tick_post`
writes.

### Script fixes needed to get a clean run

`tools/e2e.sh` only; no mod source, helper or manifest was touched.

- The game writes worlds to `./saves` in its working directory, so every run
  used to reopen the previous run's `AccDecayE2E` (gamerule already 0, old
  leaves/stone at the control cells → "Could not set the block"). The script
  now `cd`s into its scratch home and passes `--seed 20261102`, so each run is
  a fresh, identical world.
- The bare `tp 0 64 2` dropped the player into whatever was below and the top
  log was then out of sight, so `break 0 66 0` failed with `no face of block
  [0, 66, 0] is visible from here`. The player now stands on a stone step at
  y=63 on the **-z** side, clears the two cells in front of the column, and
  `ctl wait 10` lets him settle before the breaks.
- Standing on the +z side was wrong anyway: the three leaves at z=1 cover the
  column's faces and `break` then reports `no face of block [0, 64, 0] is
  visible from here`; the leaves stay where the brief puts them, the player
  moved.
- The control cells are cleared with `fill ... air` first, so a generated tree
  cannot turn the control `setblock` into a no-op.
- The check is unchanged and is now enforced: the run fails (exit 1) if any
  `minecraft:oak_leaves` or `minecraft:oak_log` is left in the box, or if the
  control is not `minecraft:oak_leaves`.

Caveat: an earlier run of the old script left an untracked
`saves/AccDecayE2E/` in this port directory (`saves/` is not in `.gitignore`);
it is not used by the fixed script (which runs from the scratch home) and was
left in place rather than deleted.

## Upstream source

Analysed 2026-11-02. Repository `https://github.com/ErrorMikey/AcceleratedDecay`,
tag `r/1.21/21.0.0` (mod 21.0.0, the ATM10 build), commit
`0af99f6e600466c7bc81aaf37261e78caf5f5d57`. Areas read: the single
`AcceleratedDecay.java` (event registration, scheduler, scanner, removal), the
three empty mixin configs, the empty access widener, the NeoForge entry point,
`README.md`, `CHANGELOG.md`, the licence and the gradle metadata. The mod has no
config, data, recipe or lang content.

This source adds the following exact constants and behaviour to the
description above (all three are now implemented; the port's constants and
behaviour match them):

- The delay is **1 real second** after a `#minecraft:logs` break
  (`AcceleratedDecay.java:80`; due test at `:46`), not 6 ticks.
  `MERGE_DELAY` is now **20 ticks** (≈1 s at 20 TPS): the pin exposes no wall
  clock in the hook scope, so the delay is the tick-based approximation.
- The scan is a **single, unlimited 26-neighbour flood fill** through leaves
  that already read `distance == 7 && !persistent` (`:84-88`, `:97-105`).
  There are no read/removal budgets, no retries, no radius, no merge cap and
  no age. The port now scans the **full 26-neighbour cube** but deliberately
  keeps its budgets, retries, radius, merge cap and age as safety valves
  (documented under "Known gaps"), so a huge canopy may span several ticks.
- Removal is a full break: the mod fires `BlockEvent.BREAK` per leaf (another
  listener may cancel), drops the block's loot with the breaking player
  (`:61-66`, `:124-125`), sets the **fluid's legacy block** (water for a
  waterlogged leaf) with flags 3 (`:128`), plays the block-break level event
  once per tree (`:119-121`) and fires `BLOCK_DESTROY` (`:129`). The port now
  preserves **water for a waterlogged leaf** (the `waterlogged` property is
  read and carried to the glue, which sets `minecraft:water` instead of air)
  and uses the same flags value (`UPDATE_ALL` = 3). The loot drop, the
  cancelable per-leaf break event, the particles/sound and `BLOCK_DESTROY`
  stay **API gaps at the pin** (recorded under "Known gaps" and in the
  findings).

Full evidence, per-row comparison and risks:
[`../_shared/upstream/accelerateddecay/FINDINGS.md`](../_shared/upstream/accelerateddecay/FINDINGS.md).

## Verification results (in-game, corrected)

Run 2026-11-02, pin `c9d474dd9116e84c243f8992d75c11314df56f06`, on a fast
build rebuilt after the upstream-fidelity corrections (20-tick delay,
26-neighbour scan, water for a waterlogged leaf). Scratch home, headless,
`random_tick_speed 0`. Only `tools/e2e.sh` (extended, no assertion weakened)
and this section changed.

Commands and exit codes:

1. `python3 tools/horizon.py build --horizon <campaign>/horizon` — **exit 0**
   (`cargo_exit: 0`, 112 s), stage `target/mixins/modded`, binary
   `target/horizon-cargo/fast/horizon-client` (2026-11-02 21:45).
   `strings target/horizon-cargo/fast/horizon-client | grep -c accelerateddecay`
   = **15** (the same count as the earlier note; `nm` shows 14 matching
   symbols, the difference is symbol folding).
2. `cargo build --profile fast --locked --offline -p horizon-client --bin
   horizon-ctl --no-default-features --target-dir <port>/target/horizon-cargo`
   in `target/mixins/modded` — **exit 0**,
   `target/horizon-cargo/fast/horizon-ctl` (it needs no rebuild: the staged
   sources are unchanged since 19:05; `stage_changed: false`).
3. `timeout 500 tools/e2e.sh` — **exit code 0**, log
   `/tmp/opencode/accdecay-e2e.log` (the same run is also
   `/tmp/opencode/accdecay-e2e-run2.log`; a second identical run,
   `-run1.log`, also exited 0).

Evidence, verbatim:

```
{"chat":["Game rule random_tick_speed is now set to 0"],"ok":true}
...six leaves planted at 0,64,1 0,65,1 0,66,1 1,65,0 -1,65,0 0,67,0,
   then three player breaks of 0,64,0 0,65,0 0,66,0 (destroy_block path)...
```

Final `blocks -2 62 -2 2 70 2` reply — **0 `minecraft:oak_leaves` and 0
`minecraft:oak_log` entries** (grep of the reply counts none); its `counts`:

```
{"counts":{"minecraft:orange_terracotta":80,"minecraft:red_sand":24,"minecraft:red_sandstone":2,"minecraft:red_terracotta":23,"minecraft:stone":3}}
```

So all six leaves are gone, including the diagonal 26-neighbour one at
`0,67,0`. Control leaf, verbatim:

```
{"block":"minecraft:oak_leaves","loaded":true,"pos":[10,64,0],"state":"minecraft:oak_leaves[distance=7,persistent=false,waterlogged=false]"}
```

Waterlogged leaf (second, separate column at x=40, z=6; the leaf is at
`41,65,6`), verbatim before and after the scan:

```
waterlogged leaf before the break: {"block":"minecraft:oak_leaves","loaded":true,"pos":[41,65,6],"state":"minecraft:oak_leaves[distance=7,persistent=false,waterlogged=true]"}
waterlogged leaf position after the scan: {"block":"minecraft:water","loaded":true,"pos":[41,65,6],"state":"minecraft:water[level=0]"}
```

In-run counterfactual (same leaf, no felled column within reach, same wait):

```
waterlogged control after the scan (must still be oak_leaves): {"block":"minecraft:oak_leaves","loaded":true,"pos":[40,70,40],"state":"minecraft:oak_leaves[distance=7,persistent=false,waterlogged=true]"}
```

Final line:

```
PASS: no leaf and no log left in the box; the control leaf is still oak_leaves; the waterlogged leaf became water; the unfelled waterlogged control stayed a leaf
```

The box check and the dry control check are unchanged; the run adds (and must
pass) an early-leaf check, both planted-state checks, the water check and the
water control. The water assertion can fail (air fails it) and the control
proves the water comes from the break, not from waterlogging decaying on its
own.

### What the run does and does not time

- `ctl wait N` is exact (a `wait 20` advanced the tick counter by 21), but a
  `ctl block` issued after a `ctl command` returns the **pre-command chunk
  copy**: an immediate read of a freshly setblocked leaf shows `air`, and a
  dry leaf does the same (probed in isolation). A short `ctl wait` before the
  read fixes it; the scenario does that for both planted-state checks.
- The first attempt at a delay assertion (`wait 10` after all three breaks)
  **failed the game run (exit 1)**: a ctl read lands ~15-20 ticks after the
  break (round-trip plus chunk-update lag), so the leaf had already been
  removed. The assertion was replaced, not weakened, by a 13-tick check after
  the **first** break only. In that read the leaf is still present but with
  `distance=2` (Horizon's distance chain re-read it against the two standing
  logs), so the engine retries it; once the other two logs fall the chain
  reaches 7 and the retries remove it. That early check therefore proves "no
  early removal of a not-ready leaf", **not** the 6-vs-20 tick boundary: with
  this harness a read cannot be placed safely between 6 and 20 ticks. The
  exact 20-tick delay is pinned by the unit test (`first removal exactly 20
  ticks after the break`, part of 15/15).
- The first waterlogged attempt (second column at x=6) aborted with "the
  waterlogged test leaf was not planted" — the stale read above, not a state
  syntax problem (`oak_leaves[persistent=false,distance=7,waterlogged=true]`
  parses and sticks). The column was moved to x=40, outside the first
  tree's ±24 block search radius, and the settle wait was added; the water
  then reads `minecraft:water[level=0]` and stays in place (a source block).

Caveat retained from the earlier run: the control cells at `10,64,0` are read
before the second column is built, so its water cannot influence that check;
the second column's water flows away from both the box and the control.

## Review repairs (PR #577)

1. Staging is the orchestrator's (horizon-mods#686).
2. *"Repair the contradictory, truncated description … replace
   `[accelerateddecay](ErrorMikey/AcceleratedDecay)` with an absolute
   original-project link"* — the entry links
   `https://github.com/nanite/AcceleratedDecay` (the repository the jar is built
   from, original author ErrorMikey) and the description now states one scope:
   what the leaf decay does and the removal/loot gap, without the contradictory
   pair of sections.
3. *"Entry/body say GPL-3.0-only; the LICENSE at the release tag is MIT … Provide
   the applicable license and preserved upstream notices"* — the **jar's**
   `META-INF/neoforge.mods.toml` declares `license = "GPL3"`, so the artefact is
   GPL-3.0-only (the jar wins over the repository and the source map). The
   repository `LICENSE` was the stale MIT text; it is now the full **GNU GPL v3**
   with ErrorMikey's notice, matching the entry, and `LICENSE-NOTICE.md` records
   the evidence. The bundle ships `mixins/accelerateddecay/LICENSE` and
   `NOTICE.md`.

### Verification after the repairs

- Artifact: `accelerateddecay.hnmod`, 26,276 B, SHA-256 `8737b84d17c566440568c73eb42177036d8c57b391f8c4119f37063ac121c588` (rebuilt twice, identical bytes; `tools/package.py`; the embedded content equals `content/accelerateddecay.hmod`).
- `upstream_ready.py accelerateddecay --copyleft` exits 0 (READY, GPL-3). `check-skills.py --notes ports/accelerateddecay` exits 0.

## Migration to the SDK format (2026-11-18, SDK `0b14f8a`)

Horizon's mod-API merge (#16941, launcher ≥ 0.2.70) retired the `.hnmod` container, and the
registry closed this port's submission (**PR #789**) with the same reason it gave every other
`.hnmod`: *"republish this as a native `.hmod` with the SDK from the next Nightly"*. The port is now
one flat package, **version 0.2.0**, on SDK nightly **`0b14f8acfd97fa2c750e42795249dcc981178ca4`**
(mod API `0.2`, mixins `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, launcher 0.2.78).

**Class + tier are unchanged:** class `content`, tier `logic` — three behavioural hooks over the
same pure engine, no machine/deep feature, no content carry.

**`runtime.rs` is byte-identical.** `src/runtime.rs` (763 lines, 15 tests) is
SHA-256 `6f78cfec40a2ab6bee8a76067bf6582208c4b771ae7af3b6db08050538729644`, `cmp`-clean against
the retired `mixins/accelerateddecay/runtime.rs`, **including the copy inside the built package**.
Its 15 tests ran green *before* any glue existed (`runtime-tests-pre-glue.log`).

### Files created

| Path | What it holds |
|---|---|
| `horizon-mod.toml` | the only manifest and identity: `id`, `version = "0.2.0"`, `api = "0.2"`, `side = "both"`, `capabilities = ["registries"]`, `license = "GPL-3.0-only"`, `homepage`/`issues`, and `[native]` with the exact `game`/`horizon`/`mixins` |
| `Cargo.toml` | one crate; the only dependency is `horizon-mod-api` from `.horizon/sdk/` |
| `src/lib.rs` | the one `#[mod_api::horizon_mod] impl Hooks` (three hooks), the `ServerReader`, the state-text → `Snapshot` conversion, and its 3 glue tests |
| `src/runtime.rs` | the old engine, `cp` + `cmp` (byte-identical, sha256 above) |
| `accelerateddecay.hmod` | the delivered package, 30,730 B, port root |
| `.gitignore` | `/target`, `/.horizon`, `/Cargo.lock`, in-game artifacts |

`LICENSE`, `LICENSE-NOTICE.md`, `README.md` and `tools/e2e.sh` stay; `README.md` and `AGENTS.md`
were rewritten for the flat layout (the old text described the retired template).

### Files deleted (and where the evidence went)

All of them into `ports/_shared/evidence/accelerateddecay/retired-hnmod-0.1.0/` **before** the
first build, because a leftover old path refuses the package (trap 1 below):

| Deleted | Why |
|---|---|
| `mixins/accelerateddecay/` (`horizon-mixin.toml`, `runtime.rs`, `support.rs`, `destroy.rs`, `tick.rs`, `level_tick.rs`) | the container is retired; `runtime.rs` → `src/`, the three helpers → one `impl Hooks` |
| `accelerateddecay.hnmod` (26,276 B, sha256 `8737b84d…c588`) | unloadable on launcher ≥ 0.2.70 |
| `compatibility.json` | `[native]` in the manifest is the pin now |
| `rust-toolchain.toml` (1.95.0) | not part of any new package; the pin/launcher decide |
| `Cargo.lock` | no registry dependency of its own, and a lock changes the artefact (trap 5); `/Cargo.lock` is gitignored |
| `tools/horizon.py`, `package.py`, `package.sh`, `test_stage.py`, `test_template.py`, `audit/` | staging, mixin planning and `.hnmod` packaging are gone |
| `.github/workflows/ci.yml` | the template's fixture workflow (clones horizon-mixins, reads `compatibility.json`, runs `test_template.py`); none of that exists any more |
| `AGENTS.md` (old text) | copied to the evidence folder as `AGENTS-old-format.md`, replaced by a flat-layout version |

### Helper → hook mapping

| Retired (0.1.0) | SDK 0.2 | Notes |
|---|---|---|
| `[[injection]] hook = "destroy_block"` HEAD, helper `destroy.rs`, bindings `game`, `pos`, `state` | **`Hooks::player_destroyed_block(&self, host: &mut dyn ServerHost, event: &mut PlayerDestroyedBlockEvent)`** | the acting player's break; `event.pos` + `event.state` replace the bindings. The moment moved from *before* the removal (HEAD of `native_destroy_block`) to *after* it (`Block.destroy`) — the engine reads the anchor's **neighbours**, so the removed log itself is never read |
| `[[injection]] hook = "server_tick"` HEAD, helper `tick.rs`, bindings `game`, `context`, `phase` | **`Hooks::server_tick`** | `event.phase` is `ServerTickPhase`; `event.context.epoch` is a `WorldEpoch` (compare with `==`, no `Cell`), `event.context.tick` a `TickId` |
| `[[injection]] hook = "level_tick_post"` HEAD, helper `level_tick.rs`, binding `dimension` | **`Hooks::level_tick_post(&self, host: HostSide<'_>, event: &mut LevelTickPostEvent)`** | the dimension comes from `event.level.dimension().id(host)`; the write is `event.level.set_block(host, pos, state, flags)` on `HostSide::Server`. The glue drains this level's removals out of the `Mutex` first and writes with no lock held, because a world write asks other mods' hooks |
| `[[module]] runtime` | `src/runtime.rs` (`pub mod runtime;`) | a private `mod` would warn `dead_code` on every `pub` item (trap 8) |
| `[[module]] state` (`support.rs`) + `static CORE: Mutex<Option<Core>>` | `Mod { core: Mutex<Core> }` | the macro boxes one `Default` instance per session and `Hooks: Any + Send + Sync`, so session state is a `Mutex` field, not a static |
| `game.native_acting_player()` | `host.players().acting_player()` | then `player.dimension(&*host)` (an `Identifier`, `Display`ed to `minecraft:overworld`) |
| `game.native_server_block_in_tag(block, "minecraft:logs")` | `host.block_in_tag(state.block(), "minecraft:logs")` (same for `#minecraft:leaves`) | the merged server tag; `Some(true)` is required — unknown/unavailable reads as "not a log" |
| `game.native_server_block(context, dimension, pos)` | `host.level(dimension)` → `LevelRef::block_state(host, pos)`; the name/properties come from `host.block_state_string(state)` | one world read + one metadata call; `None` (unloaded, out of bounds) is the engine's "forget this position" |
| `game.pack.assets.registry.state(state)` + `.properties` | `block_state_string` parsed by `snapshot()` | `distance`, `persistent`, `waterlogged`, absent reads as the engine's 0/false |
| `game.pack.assets.registry.default_state("minecraft:air" / "minecraft:water")` | `host.block_state_id("minecraft:air" / "minecraft:water")` | water is still used only for a waterlogged leaf, air otherwise |
| `game.native_set_block(dimension, pos, state, flags::UPDATE_ALL)` | `event.level.set_block(host, pos, state, BlockUpdateFlags::NEIGHBORS \| CLIENTS)` | `NEIGHBORS \| CLIENTS` **is** `UPDATE_ALL` (3) |

### Build and check (verbatim, this machine)

`build-slot.sh` still speaks the retired tool, so each step is a **bash -c in the port** under a
lease (`BUILD_LOCAL=1`, `CARGO_INCREMENTAL=0`, no `--target-dir`, no staging):

```sh
ports/tools/build-slot.sh run --job sdk-ad --port ports/accelerateddecay --step test \
    -- bash -c 'cd ports/accelerateddecay && CARGO_INCREMENTAL=0 cargo test --offline'
# → test result: ok. 18 passed; 0 failed   (15 runtime + 3 glue)

ports/tools/build-slot.sh run --job sdk-ad --port ports/accelerateddecay --step lint \
    -- bash -c 'cd ports/accelerateddecay && cargo fmt --check &&
        CARGO_INCREMENTAL=0 cargo clippy --all-targets --offline -- -D warnings'
# → FMT_EXIT:0 · CLIPPY_EXIT:0 (Finished `dev` profile, no warning)

ports/tools/build-slot.sh run --job sdk-ad --port ports/accelerateddecay --step build \
    -- bash -c 'cd ports/accelerateddecay && rm -f Cargo.lock && cargo horizon-mod build'
# → SDK 0b14f8a (build 0b14f8ac…) in ./.horizon/sdk · Built ./target/horizon-mod/accelerateddecay.hmod

ports/tools/build-slot.sh run --job sdk-ad --port ports/accelerateddecay --step check \
    -- bash -c 'cd ports/accelerateddecay && cargo horizon-mod check accelerateddecay.hmod &&
        cargo horizon-mod fits accelerateddecay.hmod --build nightly'
# → accelerateddecay 0.2.0 (native-mod, format native-1/0b14f8ac…/e59da909…, for 26.3)
#   30730 bytes, SHA-256 b3f2d724…16d3, unsigned    CHECK_EXIT:0
# → Accelerated Decay: / 0b14f8a  made for            FITS_EXIT:0
```

### Traps this migration cost

1. **A leftover old path refuses the package.** `mixins/` must go *before* the first build (the
   project root may hold only package paths). Moved to the evidence folder, not deleted outright.
2. **`build-slot.sh` does not `cd` into the port and speaks the retired tool.** `--port` is a label
   (and the cache-link target); wrap every command as `bash -c 'cd ports/<id> && …'`, and pass
   `BUILD_LOCAL=1` (the automatic forward looks for a synced copy that does not exist for a port
   created here).
3. **The SDK is fetched by the first `cargo horizon-mod` command, and `cargo test` needs it.** The
   path dependency `.horizon/sdk/api/crates/horizon-mod-api` does not exist before that, so run
   `cargo horizon-mod build` once *before* the first `cargo test` (it packs, but it also fetches).
4. **`Cargo.lock` is packed and changes the artefact.** `cargo test`/`clippy` write one; pack with
   `rm -f Cargo.lock` first, and keep `/Cargo.lock` in `.gitignore`. (A rebuild produced identical
   bytes once the lock was absent: sha256 `b3f2d724…` twice.)
5. **`cargo fmt --all --check` would report the fetched SDK** (`.horizon/sdk` is a path dependency).
   Use `cargo fmt --check`; never "fix" the SDK — and re-`cmp` `src/runtime.rs` after any `cargo fmt`,
   because the carried file must stay byte-identical.
6. **`side` is the installation side.** `side = "server"` would keep the mod off every client
   installation, i.e. off single player and a LAN host, where the behaviour is most used. `"both"`.
7. **`capabilities = ["registries"]` is not optional** even though this mod registers nothing: the
   manifest schema requires the shared value, and `check` reports `capabilities` as the package's,
   not a claim about content.
8. **`runtime.rs` as a module.** `pub mod runtime;` — a private module turns every `pub` item into a
   `dead_code` warning, and adding `#[allow(dead_code)]` to the file would break the byte-identical
   rule.
9. **`Hooks: Any + Send + Sync`.** Session state lives in the mod struct behind a `Mutex`
   (`expect("… lock poisoned")`); the macro boxes exactly one `Default` instance for the session.
10. **`ServerTickPhase` is `#[non_exhaustive]` and has two phases.** Match
    `ServerTickStart { world_running: true }`; the end phase and a frozen step must do nothing.
11. **The lifecycle context changed shape.** `LifecycleContext { authority, epoch: WorldEpoch,
    tick: TickId }` with public `get()`s; there is no `NativeLifecycleContext`, and the old
    `Cell<u64>` epoch comparison is now a plain `==` on `WorldEpoch`.
12. **Host *writes* are handle methods.** `set_block` is `LevelRef::set_block(host, …)` returning
    `Result<bool, LevelWriteError>` (not a `bool`); host-wide reads (`block_state_id`,
    `block_state_string`, `block_in_tag`, `block_key`) are supertrait methods on `&dyn Host` /
    `&dyn ServerHost`, so passing `&*host` works — `mod_api::prelude::*` is not needed and would fail
    the lint gate as an unused import.
13. **`block_state` alone does not name the block.** `BlockState` is `(BlockKey, ordinal)`; the name
    and its properties come from `host.block_state_string(state)` (canonical `/setblock` text), or
    per property from `LevelRef::block_property` (one world read each). One text read is cheaper and
    is what the glue parses.
14. **`player_destroyed_block` fires *after* the removal**, where the retired `destroy_block` HEAD
    injection fired before it. Harmless here (the engine reads neighbours), but it matters if a
    later slice wants the state still in place — that is `break_block` (cancellable) instead.
15. **No `horizon-launcher` and no `0b14f8a` game build on this machine**, so the in-game e2e could
    not be re-run: `tools/e2e.sh` is kept unchanged and needs
    `GAME=<launcher-built horizon-client>` (the launcher installs the `.hmod` into its instance; the
    script no longer passes `--dev-mod` because the SDK layout has no content half). The 0.1.0
    transcript stands as the behaviour evidence; the compile, the rules and the packaged mapping are
    the evidence of this slice.
16. **`uses.json` is a source-text scan**, so it also names SDK items the *carried* runtime mentions
    in passing (`hook:blocks`, `Block::new`, `RandomTicks` …). It is not a behaviour claim; the
    hooks it must name are `hook:player_destroyed_block`, `hook:server_tick`, `hook:level_tick_post`
    and `method:Hooks::…` for all three — and it does.

## Migration verification (SDK `0b14f8a`)

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port

| Check | Command | Result |
|---|---|---|
| Runtime carried | `cmp src/runtime.rs <retired>/mixins/accelerateddecay/runtime.rs` | clean; sha256 `6f78cfec40a2ab6bee8a76067bf6582208c4b771ae7af3b6db08050538729644` (`evidence/accelerateddecay/sdk-0b14f8a/runtime-carry.log`) |
| Pure tests before glue | `cargo test --offline` (no `impl Hooks` yet) | **15 passed**, 0 failed (`…/sdk-0b14f8a/runtime-tests-pre-glue.log`) |
| Tests with glue | `cargo test --offline` | **18 passed** (15 runtime + 3 glue), 0 failed (`…/sdk-0b14f8a/unit.log`) |
| Lints | `cargo clippy --all-targets -- -D warnings` | exit 0, 0 warnings (`…/sdk-0b14f8a/lint.log`) |
| Formatting | `cargo fmt --check` | clean (`…/sdk-0b14f8a/lint.log`) |
| Package | `cargo horizon-mod build` | `accelerateddecay.hmod`, **30,730 B**, sha256 `b3f2d724bfe9a7373e28f3902fa6c74830369a174cc49c75cb61bfca061516d3`, 8 entries (`…/sdk-0b14f8a/build.log`, `package-entries.log`, `sha256.txt`) |
| Rebuild | `rm -f Cargo.lock && cargo horizon-mod build` | identical bytes (`REBUILD_IDENTICAL`) |
| Packed runtime | `unzip -p accelerateddecay.hmod src/runtime.rs \| sha256sum` | `6f78cfec…9644` = the retired file (`runtime-carry.log`) |
| Registry check | `cargo horizon-mod check accelerateddecay.hmod` | `accelerateddecay 0.2.0 (native-mod, format native-1/0b14f8ac…/e59da909…, for 26.3)`, **EXIT 0** (`…/sdk-0b14f8a/check.log`, `check-json.log`) |
| Fits the pinned build | `cargo horizon-mod fits accelerateddecay.hmod --build nightly` | `0b14f8a  made for`, **EXIT 0** (`…/sdk-0b14f8a/check.log`) |
| What it uses | `cargo horizon-mod check accelerateddecay.hmod --uses` | the three hook entries + `method:Hooks::…` (`…/sdk-0b14f8a/uses.log`), EXIT 0 |
| Final gate (one lease) | `cargo fmt --check` · `cargo test` · `cargo clippy --all-targets -- -D warnings` · `check` · `fits --build nightly` | `FMT_EXIT:0`, **18 passed**, `CLIPPY_EXIT:0`, `CHECK_EXIT:0`, `FITS_EXIT:0` (`…/sdk-0b14f8a/final-gate.log`) |
| Symbols line | `python3 ports/tools/check-skills.py --notes ports/accelerateddecay` | `Skills line present`, SKILLS GATE: PASS |

### Not re-run in this migration

- **The in-game e2e** (felled column, ready leaves removed, control leaf intact, waterlogged leaf
  becomes water, `SELFTEST=1` negative run) — no launcher and no `0b14f8a` game build here (trap 15).
  The 0.1.0 transcript above stands as the behaviour evidence: what changed is the *transport* of the
  same decisions (typed hooks instead of injections, `LevelRef` calls instead of `Game` natives), and
  the rules are byte-identical and tested.
- **The staged ladder** (`tools/horizon.py stage/check/test/build`) — those tools are deleted with
  the retired format.

## Gaps after the migration

1. **No in-game run at `0b14f8a`** (no launcher, no game build). This slice's honest gap: the
   behaviour claims here are the mapping, the type-checked glue and the carried rules. Next slice:
   `horizon-launcher native-build . --instance <i> --trust-native-code`, then `GAME=<built>
   tools/e2e.sh` and record the transcript, including the `SELFTEST=1` negative run.
2. **The loot drop is still missing — but the API now exists.** `LevelRef::destroy_block(host, pos,
   drop, tool, collect)` (and `block_drops`) are published at `0b14f8a`: a leaf could be removed with
   `Level.destroyBlock` semantics (loot, block-break level event, `BLOCK_DESTROY`) instead of
   `set_block`. That is a **behaviour change**, so it is a re-slice with its own e2e (saplings/sticks
   on the ground, one level event per tree), not part of this transport migration. It would also
   retire most of "Known gaps" above.
3. **Player-break-only trigger.** `player_destroyed_block` hears a player's break; logs removed by
   explosions, fire or machines do not open a scan (unchanged from 0.1.0).
4. **`acting_player()` assumption.** The dimension of a break comes from
   `host.players().acting_player()`. Its contract names the player-action hooks (`break_block`,
   `right_click_block`, `run_mod_command`) and the break path is the same call chain, but this is the
   first port to use `player_destroyed_block`: the first in-game run must prove the dimension is
   found (a break that does nothing is the symptom). If it turns out unset there, the fallback is
   `break_block` (cancellable, pre-removal) with the same tag test.
5. **The safety valves stay.** `SEARCH_RADIUS` 24, `PASS_READS` 256, `PASS_REMOVALS` 24,
   `RETRY_TICKS` 4 / `MAX_ATTEMPTS` 12, `MAX_MERGES` 32, `MAX_AGE` 400 ticks are deliberate
   deviations from upstream's single unbounded pass (documented above).

## Changelog for the resubmission

> **Supersedes #789**, closed because Horizon retired `.hnmod` in the mod-API merge (#17992).
> Changes: one flat SDK package with `api = "0.2"` and `[native]` pinned to
> `0b14f8acfd97fa2c750e42795249dcc981178ca4` / mixins `e59da909…`; the three injected helpers are
> now `impl mod_api::Hooks` methods against typed hosts and events (`player_destroyed_block` opens
> the felled-tree scan, `server_tick` steps the engine, `level_tick_post` writes the removals);
> reads go through `LevelRef::block_state`/`block_state_string` and the merged block tags, writes
> through `LevelRef::set_block` with `UPDATE_ALL`; the licence travels inside the package. The
> decay rules (`src/runtime.rs`) and all 15 of their tests are **unchanged, byte for byte**.
> Version **0.2.0**.

Evidence: [`ports/_shared/evidence/accelerateddecay/`](../../_shared/evidence/accelerateddecay/)
(`sdk-0b14f8a/`: `runtime-tests-pre-glue.log`, `unit.log`, `lint.log`, `build.log`, `check.log`,
`check-json.log`, `uses.log`, `package-entries.log`, `sha256.txt`, `runtime-carry.log`;
`retired-hnmod-0.1.0/`: the whole 0.1.0 tree, the `.hnmod` and the old-format `AGENTS.md`).

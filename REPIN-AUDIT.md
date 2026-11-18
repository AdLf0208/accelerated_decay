# REPIN AUDIT — accelerateddecay (read-only)

Audit of the deferred/blocked features recorded in [`notes.md`](notes.md) against
the new pin surface. No port code, build or content was changed.

- Horizon **`83ad384f`** (427 hooks, 159 catalogued helpers) at
  `ports/_shared/pins/horizon-83ad384f`; horizon-mixins `e8cd0aa`.
- Compared with the port's recorded pin `c9d474d` via the campaign checkout's
  own history (`git show <rev>:crates/...`), so "new at this pin" is measured,
  not inferred.
- Sources of truth: `native-hooks.json`, `crates/horizon-client/src/game/**`,
  `crates/horizon-world/src/**`, `docs/native-mods.md`.
- Method: `verification-before-completion` (loaded). Static audit only — no
  build, no test, no e2e; every claim below cites the file:line it came from.

## Deferred features (from notes.md, "Known gaps" `:105-137`)

| # | Feature (upstream behaviour) | notes.md |
|---|---|---|
| A1 | Drop the leaf's loot with the breaking player (`Block.dropResources`) | `:107-119` |
| A2 | A cancelable `BlockEvent.BREAK` per leaf, heard by other mods | `:108-119` |
| A3 | Level event `2001` (break particles + sound, once per tree) | `:114-119` |
| A4 | `GameEvent.BLOCK_DESTROY` at the leaf | `:115-119` |
| A5 | The fluid's legacy block for any fluid (`fluidState.createLegacyBlock()`) | `:120-124` |
| A6 | Merges opened for non-player log removal (explosion/fire/machine) | `:131-132` |

(The budgets/radius/merge-cap/age at `:125-130` are declared safety
deviations, not API gaps, and are out of this audit.)

## Classification at `83ad384f`

| # | New-pin surface, exact | Class | New at this pin? | Verdict |
|---|---|---|---|---|
| A1 | `native_machine_destroy_block(pos, tool, collect) -> Option<Vec<ItemStack>>` — `crates/horizon-client/src/game/native_machines.rs:34`; rolls the block's loot for `tool` (`Level.destroyBlock(pos,true)` + `Block.dropResources`; `Behaviours::destroy_block` `crates/horizon-world/src/level/mod.rs:938` → `Sim::destroy_block` `level/update.rs:417`), runs the `loot_drops` hook (through `native_block_drops`, `native_hooks.rs:9366`), pops the drops | **usable** (writes, returns drops) | **yes** — declared only in the unstamped fragment `tools/native-hook-fragments/machine_breaking.json` (`@API:machine_breaking@`, absent from `native-hooks.json` and `docs/native-mods.md`) | **realizable now** |
| A2 | `native_destroy_block(pos, state, &mut canceled)` — `native_hooks.rs:6763`, contract `cancel`/`out: canceled`; a game→mod hook. No helper lets a mod emit it, and an injected bundle is not in `mod_api::mods()`, so a manual call reaches typed mods only | **hook-only** | no (same signature; the catalog's `cancel` metadata is new, the hook itself is not) | **still blocked** |
| A3 | same call: `level_event(2001, pos, state)` in `crates/horizon-world/src/level/update.rs:427`; fallback `GameLevel::native_play_sound` `native_hooks.rs:8193` + `native_send_particles` `native_hooks.rs:8233` | **usable** | **yes** (via A1) | **realizable now** |
| A4 | same call: `game_event_state(pos, "minecraft:block_destroy", state)` — `update.rs:436`; the `game_event` hook (`native_hooks.rs:5937`) then hears it | **usable** (fired by A1) | **yes** | **realizable now** |
| A5 | same call: `self.b.legacy_block(fluid)` written at `update.rs:432-434` — the fluid's legacy block for the fluid actually in the block | **usable** | **yes** (via A1); the standalone `native_fluid_flow` `native_world_read.rs:142` only gives the current vector, not the fluid kind | **realizable now** |
| A6 | `native_block_changed(dimension, pos, old, new)` — `native_hooks.rs:1200` (`GameLevel`, listen), wired at `game.rs:1162` ← `crates/horizon-world/src/level/update.rs:340`; plus `native_leaves_decay(dimension,pos,&mut cancel)` `native_hooks.rs:1238` (cancel) wired at `crates/horizon-world/src/level/tick.rs:383` | **usable** (`block_changed`); `leaves_decay` is **hook-only** (hear/cancel, cannot trigger) | `block_changed` no; `leaves_decay` **yes** | **realizable now** |

## Counts

6 deferred API features → **5 realizable now** (A1–A5 all through one call),
**1 still blocked** (A2). No feature regressed.

## Corrections to the pilot summary

- `notes.md:112-113` says "no drop-resources API and no item-entity spawn
  API". The spawn half was wrong even at `c9d474d`: `native_summon`
  (`native_entities.rs:141`) spawns `minecraft:item` from NBT (the kind is
  loadable, `entity_nbt.rs:2138`). The drop half is now covered properly by
  `native_machine_destroy_block`.
- The in-crate `Game::break_block_as_player` (`game.rs:6189`, private) already
  did a full player break (drops, 2001, game event) at `c9d474d` too. It is not
  the recommended route: it borrows the host's `player`/`held`/`creative`, so
  guests and dedicated servers are wrong. `native_machine_destroy_block` is
  player-free and dimension-safe (acts on the level in place).
- The 1-second delay is still tick-based: `native_clock` (`native_hooks.rs:9130`)
  returns *game* time (`world.game_time`/`day_time`), not wall clock.

## Highest-value next step

In `mixins/accelerateddecay/support.rs::on_level_tick` (`:120-150`), replace

```rust
game.native_set_block(dimension, removal.pos, target, flags::UPDATE_ALL);
```

with `game.native_machine_destroy_block(removal.pos, tool, false)`, keeping the
`native_set_block` write only as the `None` fallback. That single change lands
A1 + A3 + A4 + A5 (loot, 2001 particles/sound, `BLOCK_DESTROY`, arbitrary-fluid
legacy block) and deletes the `water`/`air` special case.

- Prerequisite to decide first: the helper is a *draft* API at this pin
  (`machine_breaking.json` unstamped, no API number, not in `native-hooks.json`).
  If the campaign will not depend on a draft helper, file the same requirement
  as an issue ("a native `Level.destroyBlock(pos, true)` with a tool and an
  optional player") and keep `native_set_block` until it is stamped.
- Fidelity limits to record: no player is passed to the loot context
  (`native_machine_destroy_block` hard-codes `None`), so a shears-Fortune-
  dependent drop is matched only through `tool`; upstream passes the breaking
  player. A2 stays open (no cancelable break can be emitted).
- Evidence the implementer must produce: unit test that the removal path picks
  the new call (a pure "which removal writer" decision in `runtime.rs` is the
  cheapest), then the existing e2e plus a **new** assertion that a felled
  ready leaf yields a drop entity and plays the break event — a drop appears
  where `native_set_block` produced none, so the scenario can fail.

## Reference

Skill: `verification-before-completion` (read-only audit; Beautiful Rust not
needed because no Rust is written or reviewed here).

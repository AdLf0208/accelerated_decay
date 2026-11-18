//! Accelerated Decay for Horizon — the one `impl Hooks` at the crate root.
//!
//! Upstream (`AcceleratedDecay.java`, tag `r/1.21/21.0.0`) schedules one scan a real second after
//! a player fells a `#minecraft:logs` block: a single flood fill over the 26-neighbour cube that
//! breaks every leaf already marked ready to decay (`distance == 7`, not `persistent`). The rules
//! live in [`runtime`], byte-identical to the retired bundle's `runtime.rs` and tested by
//! `cargo test` without the game; this module is only the glue between them and the SDK:
//!
//! | Hook | When it runs | What the glue does |
//! |---|---|---|
//! | [`Hooks::player_destroyed_block`] | a player's break removed a block, before the drops | opens a merge when the block is a `#minecraft:logs` member, in the acting player's dimension |
//! | [`Hooks::server_tick`] | start of every running authoritative tick | steps the engine, reading blocks by dimension, and queues the removals it decides |
//! | [`Hooks::level_tick_post`] | after a level's tick pass, that level in place | writes the queued removals of that dimension (`set_block`, `UPDATE_ALL`) |
//!
//! Reads and writes are split over two hooks for the same reason as in the retired bundle: a
//! `server_tick` host can read any loaded level, while the write belongs to the level that has
//! just ticked.
use std::sync::{Mutex, MutexGuard};

use mod_api::{
    self, BlockUpdateFlags, Hooks, HostSide, LevelTickPostEvent, PlayerDestroyedBlockEvent,
    ServerHost, ServerTickEvent, ServerTickPhase, WorldEpoch,
};

pub mod runtime;

use runtime::{Engine, Reader, Removal, Snapshot};

/// `#minecraft:logs`: the tag the original opens a scan for.
const LOGS: &str = "minecraft:logs";
/// `#minecraft:leaves`: the tag a candidate block must be a member of.
const LEAVES: &str = "minecraft:leaves";

/// Cross-hook state: the engine, the server-tick counter it schedules on, the world epoch the
/// state belongs to, and the removals waiting for their level's pass.
///
/// The macro boxes one `Default` instance for the session, and `Hooks` is `Send + Sync`, so the
/// state is a `Mutex` field and lives exactly as long as the session.
#[derive(Default)]
struct Core {
    /// The open attempt this state belongs to. A different epoch (a reopened or a new world)
    /// drops everything: the anchors of a closed world must never touch a later one's blocks.
    epoch: Option<WorldEpoch>,
    /// Our own running tick count, advanced once per running server tick. (The destroy hook has
    /// no tick in its event, so it stamps a break with the count the last server tick set.)
    tick: i64,
    engine: Engine,
    /// Removals decided by the engine, executed by the next pass of their own level.
    pending: Vec<Removal>,
}

#[derive(Default)]
struct Mod {
    core: Mutex<Core>,
}

impl Mod {
    fn core(&self) -> MutexGuard<'_, Core> {
        self.core
            .lock()
            .expect("accelerateddecay state lock poisoned")
    }
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// A player's break removed `event.state` at `event.pos`: open a felled-tree scan when the
    /// block is a member of the merged `#minecraft:logs` tag. The break itself is untouched.
    fn player_destroyed_block(
        &self,
        host: &mut dyn ServerHost,
        event: &mut PlayerDestroyedBlockEvent,
    ) {
        let Some(player) = host.players().acting_player() else {
            return;
        };
        let Some(dimension) = player.dimension(&*host) else {
            return;
        };
        // Unknown or unavailable tags answer false/None, which is "not a log": the original
        // leaves unsupported trees alone too.
        if host.block_in_tag(event.state.block(), LOGS) != Some(true) {
            return;
        }
        let dimension = dimension.to_string();
        let mut core = self.core();
        let tick = core.tick;
        core.engine.on_log_removed(&dimension, event.pos, tick);
    }

    /// The start of a running authoritative tick: read the neighbourhoods the engine asks for and
    /// queue the leaves it declares ready. The level passes that follow write them.
    fn server_tick(&self, host: &mut dyn ServerHost, event: &mut ServerTickEvent<'_>) {
        // The end of the tick has nothing to do, and a frozen step does not advance the world.
        let ServerTickPhase::ServerTickStart {
            world_running: true,
        } = event.phase
        else {
            return;
        };
        let epoch = event.context.epoch;
        let mut core = self.core();
        if core.epoch != Some(epoch) {
            core.epoch = Some(epoch);
            core.tick = 0;
            core.engine = Engine::new();
            core.pending.clear();
        }
        core.tick += 1;
        let tick = core.tick;
        let mut reader = ServerReader { host };
        let removals = core.engine.step(tick, &mut reader);
        core.pending.extend(removals);
    }

    /// A level finished its tick pass, with that level in place: replace this dimension's queued
    /// leaves with air, or with water when the leaf was waterlogged (upstream sets the fluid's
    /// legacy block). `NEIGHBORS | CLIENTS` is `UPDATE_ALL`, the flags a normal removal uses.
    fn level_tick_post(&self, host: HostSide<'_>, event: &mut LevelTickPostEvent) {
        let HostSide::Server(host) = host else {
            return;
        };
        let Some(dimension) = event.level.dimension().id(&*host) else {
            return;
        };
        let Some(air) = host.block_state_id("minecraft:air") else {
            return;
        };
        // A registry without `minecraft:water` is malformed; fall back to air rather than skip
        // the removal (the leaf must still go).
        let water = host.block_state_id("minecraft:water").unwrap_or(air);
        // Take this level's removals out under the lock, then write outside it: a world write
        // asks the game's other hooks, and no lock of ours is held while they run.
        let mut mine = Vec::new();
        {
            let mut core = self.core();
            let mut i = 0;
            while i < core.pending.len() {
                if core.pending[i].dimension == dimension {
                    mine.push(core.pending.swap_remove(i));
                } else {
                    i += 1;
                }
            }
        }
        let flags = BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS;
        for removal in mine {
            let target = if removal.waterlogged { water } else { air };
            let _ = event
                .level
                .set_block(&mut *host, removal.pos, target, flags);
        }
    }
}

/// The engine's view of the server: a block is read through its dimension's level handle (which
/// reads the level in place *and* parked ones), and leaf membership through the merged server tag.
struct ServerReader<'a> {
    host: &'a mut dyn ServerHost,
}

impl Reader for ServerReader<'_> {
    fn read(&mut self, dimension: &str, pos: [i32; 3]) -> Option<Snapshot> {
        let level = self.host.level(dimension)?;
        let state = level.block_state(self.host, pos)?;
        // One canonical text carries the name and every property, as `/setblock` serializes it:
        // `minecraft:oak_leaves[distance=7,persistent=false,waterlogged=false]`.
        let text = self.host.block_state_string(state)?;
        Some(snapshot(&text))
    }

    fn is_leaves(&mut self, name: &str) -> bool {
        let Some(block) = self.host.block_key(name) else {
            return false;
        };
        self.host.block_in_tag(block, LEAVES).unwrap_or(false)
    }
}

/// The [`Snapshot`] a state's canonical text describes. A block without a property reads the
/// engine's "absent" value for it, exactly as the retired glue's `properties.get(..)` did.
fn snapshot(text: &str) -> Snapshot {
    let (name, properties) = text.split_once('[').unwrap_or((text, ""));
    let properties = properties.strip_suffix(']').unwrap_or(properties);
    let mut snapshot = Snapshot {
        name: name.to_owned(),
        distance: 0,
        persistent: false,
        waterlogged: false,
    };
    for pair in properties.split(',') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        match key {
            "distance" => snapshot.distance = value.parse().unwrap_or(0),
            "persistent" => snapshot.persistent = value == "true",
            "waterlogged" => snapshot.waterlogged = value == "true",
            _ => {}
        }
    }
    snapshot
}

#[cfg(test)]
mod tests {
    use super::snapshot;

    #[test]
    fn a_ready_waterlogged_leaf_reads_every_property() {
        let read = snapshot("minecraft:oak_leaves[distance=7,persistent=false,waterlogged=true]");
        assert_eq!(read.name, "minecraft:oak_leaves");
        assert_eq!(read.distance, 7);
        assert!(!read.persistent);
        assert!(read.waterlogged);
    }

    #[test]
    fn a_state_without_properties_is_never_ready() {
        let read = snapshot("minecraft:stone");
        assert_eq!(read.name, "minecraft:stone");
        assert_eq!(read.distance, 0);
        assert!(!read.persistent);
        assert!(!read.waterlogged);
    }

    #[test]
    fn properties_are_read_in_any_order() {
        let read = snapshot("mymod:leaves[waterlogged=false,persistent=true,distance=3]");
        assert_eq!(read.name, "mymod:leaves");
        assert_eq!(read.distance, 3);
        assert!(read.persistent);
        assert!(!read.waterlogged);
    }
}

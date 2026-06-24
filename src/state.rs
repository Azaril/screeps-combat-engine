//! Combat world state — JS-free value types over `screeps::Position`. The deterministic tick
//! (`resolve.rs`) operates on a `CombatWorld`. Scope: a single 50×50 room (ADR 0006 Part B).

use crate::body::SimBody;
use crate::constants::{FATIGUE_RATE_PLAIN, FATIGUE_RATE_SWAMP};
use screeps::{Position, RoomName};
use std::collections::{HashMap, HashSet};

/// Room terrain — defaults to all-plain. Walls block movement; swamp raises move fatigue.
/// (Roads, which lower fatigue, are structures and arrive with the structures slice.)
#[derive(Clone, Debug, Default)]
pub struct CombatTerrain {
    pub walls: HashSet<(u8, u8)>,
    pub swamps: HashSet<(u8, u8)>,
}

impl CombatTerrain {
    pub fn is_wall(&self, x: u8, y: u8) -> bool {
        self.walls.contains(&(x, y))
    }
    /// Fatigue added per non-MOVE/non-CARRY part for a step onto this tile.
    pub fn fatigue_rate(&self, x: u8, y: u8) -> u32 {
        if self.swamps.contains(&(x, y)) {
            FATIGUE_RATE_SWAMP
        } else {
            FATIGUE_RATE_PLAIN
        }
    }
}

/// A combatant identity (self-play: side 0 vs side 1; NPCs get their own ids later).
pub type PlayerId = u8;
/// Stable per-engagement creep id (minted by the scenario; NOT a game `ObjectId`).
pub type CreepId = u32;
/// Stable per-engagement structure id.
pub type StructureId = u32;

/// Attackable/dismantlable structure kinds modelled so far. (Roads/containers/etc. are follow-ups.)
/// `Tower` tags a [`SimTower`] when it appears as a *damage target*; towers still live in their own
/// [`CombatWorld::towers`] Vec (they also *fire*), but share the structure damage/repair pools.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructureKind {
    Spawn,
    Rampart,
    Wall,
    Tower,
}

/// A passive (non-firing) structure that can be attacked/dismantled/repaired. Ramparts shield
/// co-located targets: rangedMassAttack SKIPS a shielded target (engine `rangedMassAttack.js:38`),
/// while single-target attack/rangedAttack/tower/dismantle REDIRECT to the rampart (`attack.js:33-36`,
/// `rangedAttack.js:33-36`, `towers/attack.js:27-30`, `dismantle.js:27-29`) — ownership-blind, so a
/// creep on a rampart takes 0 until the rampart breaks. A rampart also suppresses melee attack-back
/// for an attacker standing on one (`_damage.js:17`). All modeled in `resolve.rs` (`redirect`).
#[derive(Clone, Debug)]
pub struct SimStructure {
    pub id: StructureId,
    pub kind: StructureKind,
    /// `None` for unowned constructed walls; `Some` for ramparts/spawns.
    pub owner: Option<PlayerId>,
    pub pos: Position,
    pub hits: u32,
    pub hits_max: u32,
}

impl SimStructure {
    pub fn is_alive(&self) -> bool {
        self.hits > 0
    }
}

/// A creep in the sim.
#[derive(Clone, Debug)]
pub struct SimCreep {
    pub id: CreepId,
    pub owner: PlayerId,
    pub pos: Position,
    pub body: SimBody,
    /// Fatigue carried into this tick; the creep may move only when it is 0.
    pub fatigue: u32,
}

impl SimCreep {
    pub fn is_alive(&self) -> bool {
        self.body.is_alive()
    }
}

/// A tower in the sim. Towers fire once per tick for [`crate::constants::TOWER_ENERGY_COST`]
/// energy and resolve in the same two-phase step as creep combat (the drain math). A tower is also
/// a **damage target**: it shares the structure damage/repair pools (keyed by `id`, which must be
/// unique across `structures` *and* `towers`), takes dismantle/attack/RMA, and is repairable.
#[derive(Clone, Debug)]
pub struct SimTower {
    /// Unique across both `structures` and `towers` (it participates in the structure pools).
    pub id: StructureId,
    pub owner: PlayerId,
    pub pos: Position,
    pub energy: u32,
    pub hits: u32,
    pub hits_max: u32,
}

impl SimTower {
    pub fn is_alive(&self) -> bool {
        self.hits > 0
    }
}

/// One room's combat state for a tick.
#[derive(Clone, Debug, Default)]
pub struct CombatWorld {
    pub tick: u32,
    /// The default/common room terrain — also the terrain of a single-room scenario. Per-room
    /// overrides for a multi-room scenario live in [`rooms`](CombatWorld::rooms); read terrain via
    /// [`terrain_for`](CombatWorld::terrain_for), which falls back to this for any room without an
    /// override. (S1: keeps single-room builders unchanged while the engine becomes room-aware.)
    pub terrain: CombatTerrain,
    /// Per-room terrain overrides for multi-room scenarios (N-room sim, ADR 0023). A room absent here
    /// uses [`terrain`](CombatWorld::terrain).
    pub rooms: HashMap<RoomName, CombatTerrain>,
    pub creeps: Vec<SimCreep>,
    pub towers: Vec<SimTower>,
    pub structures: Vec<SimStructure>,
    /// Owner whose controller is in safe mode this tick (all *hostile* combat zeroed), if any.
    pub safe_mode_owner: Option<PlayerId>,
}

impl CombatWorld {
    pub fn living_creeps(&self) -> impl Iterator<Item = &SimCreep> {
        self.creeps.iter().filter(|c| c.is_alive())
    }

    /// Terrain for `room` — the per-room override if one exists, else the default [`terrain`](CombatWorld::terrain).
    /// All movement/fatigue/wall checks go through this so the engine is multi-room-correct (ADR 0023 S1).
    pub fn terrain_for(&self, room: RoomName) -> &CombatTerrain {
        self.rooms.get(&room).unwrap_or(&self.terrain)
    }

    /// Mutable per-room terrain override for `room` (creating an empty one if absent) — used by the
    /// multi-room ScenarioBuilder to give distinct rooms distinct terrain.
    pub fn terrain_mut(&mut self, room: RoomName) -> &mut CombatTerrain {
        self.rooms.entry(room).or_default()
    }
    // (Removed `rampart_at(x, y)` in S2 — it was dead code keyed by `(x,y)` only, the same single-room
    // conflation S2 fixes elsewhere. The resolver's Position-keyed `rampart_tiles` is the single source
    // of truth for rampart-shield/redirect logic.)
}

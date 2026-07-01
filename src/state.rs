//! Combat world state — JS-free value types over `screeps::Position`. The deterministic tick
//! (`resolve.rs`) operates on a `CombatWorld`. Scope: a single 50×50 room (ADR 0006 Part B).
//!
//! The movement half of the world (tick, terrain, creeps, edge-exit exemptions) lives in the shared
//! kernel's [`MovementState`] (ADR 0033); `CombatWorld` **composes** it and adds the combat overlay
//! (towers/structures/controllers/safe-mode). The creep + terrain + id value types are the kernel's
//! ([`SimCreep`], [`SimTerrain`], [`CreepId`]/[`PlayerId`]/[`StructureId`]) — re-exported here so
//! combat call sites keep referring to `crate::state::…`.

use screeps::{Position, RoomName};

// The movement-layer value types now come from the kernel; re-export so combat call sites (and the
// agent/eval crates) keep using `crate::state::{SimCreep, SimTerrain, CreepId, …}` unchanged.
pub use screeps_sim_core::{CreepId, MovementState, PlayerId, SimCreep, SimTerrain, StructureId};

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

/// A room controller in the sim — the de-claim target for `attackController` (derelict-room salvage /
/// neutralizing an enemy's controller). `downgrade_ticks` is the de-claim countdown: each
/// `attackController` (CLAIM parts × [`crate::constants::CONTROLLER_ATTACK_PER_PART`]) reduces it; at 0 the controller
/// goes neutral (`owner = None`). A neutral/unowned controller (`owner == None`) is not a de-claim
/// target. (A minimal model — enough to simulate a declaim mission; the full upgrade-block / reservation
/// mechanics are not modeled.)
#[derive(Clone, Debug)]
pub struct SimController {
    pub pos: Position,
    pub owner: Option<PlayerId>,
    pub downgrade_ticks: u32,
}

impl SimController {
    /// A de-claim target: owned by someone (a neutral controller can't be attacked further).
    pub fn is_claimed(&self) -> bool {
        self.owner.is_some()
    }
}

impl SimTower {
    pub fn is_alive(&self) -> bool {
        self.hits > 0
    }
}

/// One room's combat state for a tick: the shared movement world plus the combat overlay.
#[derive(Clone, Debug, Default)]
pub struct CombatWorld {
    /// The shared movement/world kernel state: `tick`, terrain, per-room terrain overrides, creeps,
    /// and the NPC edge-exit exemption set (ADR 0033). The combat tick calls
    /// [`screeps_sim_core::resolve_movement`] over this at its movement point.
    pub movement: MovementState,
    pub towers: Vec<SimTower>,
    pub structures: Vec<SimStructure>,
    /// Room controllers — the de-claim targets for `attackController` (one per room at most). Empty for
    /// scenarios that don't model controllers (the common combat case).
    pub controllers: Vec<SimController>,
    /// Owner whose controller is in safe mode this tick (all *hostile* combat zeroed), if any.
    pub safe_mode_owner: Option<PlayerId>,
}

impl CombatWorld {
    /// Living creeps — forwards to the movement state.
    pub fn living_creeps(&self) -> impl Iterator<Item = &SimCreep> {
        self.movement.living_creeps()
    }

    /// Terrain for `room` — the per-room override if one exists, else the default terrain. All
    /// movement/fatigue/wall checks go through this so the engine is multi-room-correct (ADR 0023 S1).
    /// Forwards to the movement state.
    pub fn terrain_for(&self, room: RoomName) -> &SimTerrain {
        self.movement.terrain_for(room)
    }

    /// Mutable per-room terrain override for `room` (creating an empty one if absent) — used by the
    /// multi-room ScenarioBuilder to give distinct rooms distinct terrain. Forwards to the movement state.
    pub fn terrain_mut(&mut self, room: RoomName) -> &mut SimTerrain {
        self.movement.terrain_mut(room)
    }
}

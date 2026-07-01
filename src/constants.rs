//! Combat constants transcribed from the Screeps engine (`screeps-common/lib/constants.js`) and
//! the processor intents. Verified firsthand against `C:\code\screeps-engine`; engine source is
//! ground truth over any documentation.
//!
//! The movement kernel [`screeps_sim_core::constants`] holds only pure-movement constants
//! (`BODYPART_HITS`, fatigue rates); the body-COMBAT constants below (per-part action power, creep
//! lifetimes) live HERE in the combat layer alongside the `SimBodyCombat` extension trait
//! (ADR 0033) — the mover never needs them, and `screeps-combat-decision` imports them from here.

/// Creep lifetimes (`CREEP_LIFE_TIME` / `CREEP_CLAIM_LIFE_TIME`).
pub const CREEP_LIFE_TIME: u32 = 1500;
pub const CREEP_CLAIM_LIFE_TIME: u32 = 600;

// ── Per-part action power (unboosted) — a property of a body part ────────────
pub const ATTACK_POWER: u32 = 30; // ATTACK, melee, range 1
pub const RANGED_ATTACK_POWER: u32 = 10; // RANGED_ATTACK, range 3
pub const HEAL_POWER: u32 = 12; // HEAL adjacent, range 1
pub const RANGED_HEAL_POWER: u32 = 4; // HEAL at range, range 3
pub const DISMANTLE_POWER: u32 = 50; // WORK dismantle, range 1

/// `attackController` controller-downgrade per CLAIM part, range 1 (`attackController.js`: each CLAIM
/// part reduces the target controller's downgrade timer; de-claim accrues until it hits 0 → neutral).
pub const CONTROLLER_ATTACK_PER_PART: u32 = 300;

// ── Action ranges (Chebyshev) ───────────────────────────────────────────────
pub const ATTACK_RANGE: u32 = 1;
pub const RANGED_ATTACK_RANGE: u32 = 3;
pub const HEAL_RANGE: u32 = 1;
pub const RANGED_HEAL_RANGE: u32 = 3;
pub const DISMANTLE_RANGE: u32 = 1;

/// `rangedMassAttack` damage rate by Chebyshev distance 0..=3 (engine
/// `RANGED_ATTACK_DISTANCE_RATE`; `rangedMassAttack.js`). Beyond range 3 → 0.
pub const RANGED_MASS_ATTACK_FALLOFF: [f64; 4] = [1.0, 1.0, 0.4, 0.1];

// ── Towers (towers/attack.js, heal.js, repair.js) ───────────────────────────
pub const TOWER_OPTIMAL_RANGE: u32 = 5;
pub const TOWER_FALLOFF_RANGE: u32 = 20;
pub const TOWER_FALLOFF: f64 = 0.75;
pub const TOWER_POWER_ATTACK: f64 = 600.0;
pub const TOWER_POWER_HEAL: f64 = 400.0;
pub const TOWER_POWER_REPAIR: f64 = 800.0;
pub const TOWER_ENERGY_COST: u32 = 10; // energy per shot (drain math)

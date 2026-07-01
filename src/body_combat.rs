//! Body-COMBAT arithmetic as an extension trait on the movement kernel's [`SimBody`].
//!
//! The kernel `screeps-sim-core` is pure MOVEMENT mechanics (per-part hit pools, fatigue, the
//! `effective_power` boost helper); it deliberately does NOT know attack/heal/dismantle power or
//! TOUGH damage reduction — the mover never needs them (ADR 0033: "non-movement concerns belong in
//! a layer, not the kernel"). This trait, implemented for `SimBody` in the combat layer, adds those
//! methods back for combat call sites. Bring [`SimBodyCombat`] into scope to call them on a `SimBody`.
//!
//! The method bodies are the engine's `calcBodyEffectiveness` (`utils.js:623`, via the kernel's
//! public `effective_power`) and `_applyDamage` (`creeps/tick.js:7-29`), moved verbatim from the
//! kernel — same formulas, same constants (now sourced from [`crate::constants`]).

use crate::constants::{
    ATTACK_POWER, DISMANTLE_POWER, HEAL_POWER, RANGED_ATTACK_POWER, RANGED_HEAL_POWER,
};
use screeps::Part;
use screeps_sim_core::{BoostTier, SimBody};

/// Body-combat arithmetic (attack/heal/dismantle power, TOUGH reduction). Lives in the combat layer,
/// not the movement kernel (ADR 0033): the mover never needs it. Bring into scope to call these on a SimBody.
pub trait SimBodyCombat {
    fn attack_power(&self) -> u32;
    fn ranged_attack_power(&self) -> u32;
    fn heal_power(&self) -> u32;
    fn ranged_heal_power(&self) -> u32;
    fn dismantle_power(&self) -> u32;
    fn damage_after_tough(&self, raw: u32) -> u32;
}

impl SimBodyCombat for SimBody {
    fn attack_power(&self) -> u32 {
        self.effective_power(Part::Attack, ATTACK_POWER)
    }
    fn ranged_attack_power(&self) -> u32 {
        self.effective_power(Part::RangedAttack, RANGED_ATTACK_POWER)
    }
    fn heal_power(&self) -> u32 {
        self.effective_power(Part::Heal, HEAL_POWER)
    }
    fn ranged_heal_power(&self) -> u32 {
        self.effective_power(Part::Heal, RANGED_HEAL_POWER)
    }
    fn dismantle_power(&self) -> u32 {
        self.effective_power(Part::Work, DISMANTLE_POWER)
    }

    /// Damage actually inflicted after TOUGH/boost reduction, given `raw` incoming this tick
    /// (engine `_applyDamage`, `creeps/tick.js:7-29`). Pure — does **not** mutate `hits`; the
    /// caller nets damage-then-heal in the resolve pass. Iterates parts front-to-back, each
    /// absorbing up to `part_hits / damage_ratio` "effective" hits; only TOUGH boosts have a
    /// `damage_ratio < 1`, and the accumulated reduction is rounded **once** at the end.
    fn damage_after_tough(&self, raw: u32) -> u32 {
        if raw == 0 {
            return 0;
        }
        // The reduction loop only runs if any part is boosted (engine `_.any(body, i => !!i.boost)`).
        if !self.parts.iter().any(|p| p.boost != BoostTier::None) {
            return raw;
        }
        let mut damage_reduce = 0.0;
        let mut damage_effective = raw as f64;
        for (i, p) in self.parts.iter().enumerate() {
            if damage_effective <= 0.0 {
                break;
            }
            let part_hits = self.part_hits(i) as f64;
            let ratio = if p.part == Part::Tough {
                p.boost.tough_damage_ratio()
            } else {
                1.0
            };
            let effective = part_hits / ratio;
            let absorbed = effective.min(damage_effective);
            damage_reduce += absorbed * (1.0 - ratio);
            damage_effective -= absorbed;
        }
        (raw as f64 - damage_reduce.round()).max(0.0) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use screeps_sim_core::BodyPartDef;

    fn body(parts: &[(Part, BoostTier)]) -> SimBody {
        SimBody::new(
            parts
                .iter()
                .map(|&(p, b)| BodyPartDef::boosted(p, b))
                .collect(),
        )
    }

    #[test]
    fn action_power_unboosted_and_boosted() {
        let five_attack = body(&[(Part::Attack, BoostTier::None); 5]);
        assert_eq!(five_attack.attack_power(), 150); // 5 × 30

        let five_attack_t3 = body(&[(Part::Attack, BoostTier::T3); 5]);
        assert_eq!(five_attack_t3.attack_power(), 600); // 5 × 30 × 4

        let heal = body(&[(Part::Heal, BoostTier::None); 4]);
        assert_eq!(heal.heal_power(), 48); // 4 × 12
        assert_eq!(heal.ranged_heal_power(), 16); // 4 × 4

        let heal_t3 = body(&[(Part::Heal, BoostTier::T3); 4]);
        assert_eq!(heal_t3.heal_power(), 192); // 4 × 12 × 4
        assert_eq!(heal_t3.ranged_heal_power(), 64); // 4 × 4 × 4

        let ranged = body(&[(Part::RangedAttack, BoostTier::None); 7]);
        assert_eq!(ranged.ranged_attack_power(), 70); // 7 × 10

        let work = body(&[(Part::Work, BoostTier::T3); 10]);
        assert_eq!(work.dismantle_power(), 2000); // 10 × 50 × 4
    }

    #[test]
    fn unboosted_takes_full_damage() {
        let b = SimBody::unboosted(&[Part::Tough, Part::Attack]);
        assert_eq!(b.damage_after_tough(100), 100); // no boost → no reduction
    }

    #[test]
    fn tough_reduces_within_capacity() {
        // 10 full XGHO2 TOUGH (T3, ×0.3) + 1 MOVE: 100 raw is well within the ~3333 effective
        // capacity, so it's reduced straight to ×0.3 = 30.
        let mut parts: Vec<(Part, BoostTier)> = vec![(Part::Tough, BoostTier::T3); 10];
        parts.push((Part::Move, BoostTier::None));
        assert_eq!(body(&parts).damage_after_tough(100), 30);
    }

    #[test]
    fn tough_capacity_exceeded_spills_unreduced() {
        // 1 full XGHO2 TOUGH (eff capacity 100/0.3 ≈ 333) + 1 MOVE, raw 500: the first 333
        // effective is reduced (Σ reduce = 333.33 × 0.7 = 233.33 → round 233), the rest hits
        // unreduced. Result = 500 − 233 = 267.
        let b = body(&[(Part::Tough, BoostTier::T3), (Part::Move, BoostTier::None)]);
        assert_eq!(b.damage_after_tough(500), 267);
    }

    #[test]
    fn dead_tough_gives_no_mitigation() {
        // [Tough(T3), Attack] at 100 hits → Tough (front) is dead, Attack (back) full. A dead
        // TOUGH part absorbs nothing, so damage passes unreduced.
        let mut b = body(&[
            (Part::Tough, BoostTier::T3),
            (Part::Attack, BoostTier::None),
        ]);
        b.hits = 100;
        assert_eq!(b.part_hits(0), 0);
        assert_eq!(b.damage_after_tough(50), 50);
    }
}

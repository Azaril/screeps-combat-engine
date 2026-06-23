# screeps-combat-engine

> A deterministic, JS-free Rust port of the Screeps combat tick.

`screeps-combat-engine` models a single 50×50 Screeps room of creeps and structures and resolves
combat **exactly as the real Screeps engine does** — damage, healing, movement, towers, ramparts,
deaths — in microseconds, deterministically, with no Docker, no server, and no JavaScript. It is the
*mechanism* layer of a combat micro-simulator: hand it a world plus per-creep intents and it returns
the resolved next tick, with an optional per-tick recording for introspection. It was extracted from
the [screeps-ibex](https://github.com/Azaril/screeps-ibex) workspace.

## Why it exists

Combat in Screeps is hard to measure on a live server: ticks run concurrently, outcomes are not
reproducible, and a single engagement takes minutes. This crate makes combat a pure function —
identical `(world, intents)` always produce an identical result — so combat code and tactics can be
iterated at `cargo test` speed with exact, seed-reproducible outcomes. A live (e.g. Dockerized)
private server stays the fidelity oracle; this crate is the fast per-change loop.

## Installation

```toml
[dependencies]
screeps-combat-engine = { git = "https://github.com/Azaril/screeps-combat-engine" }
```

It depends only on the value types from [`screeps-game-api`](https://crates.io/crates/screeps-game-api)
(`Part`, `Position`, `RoomName`, …) — no JS interop — so it builds for both host (`cargo test`) and
`wasm32-unknown-unknown`.

## Quick start

```rust
use screeps_combat_engine::{CombatWorld, SimCreep, Intents, CombatAction, resolve_tick};
use screeps_combat_engine::body::SimBody;
use screeps::{Part, Position, RoomCoordinate, RoomName};

let room: RoomName = "W1N1".parse().unwrap();
let at = |x, y| Position::new(RoomCoordinate::new(x).unwrap(), RoomCoordinate::new(y).unwrap(), room);

let mut world = CombatWorld {
    creeps: vec![
        SimCreep { id: 1, owner: 0, pos: at(25, 25), body: SimBody::unboosted(&[Part::Attack; 15]), fatigue: 0 },
        SimCreep { id: 2, owner: 1, pos: at(25, 26), body: SimBody::unboosted(&[Part::Heal; 14]),   fatigue: 0 },
    ],
    ..Default::default()
};

let mut intents = Intents::new();
intents.set(1, vec![CombatAction::Attack(2)]); // creep 1 melee-attacks creep 2
intents.set(2, vec![CombatAction::Heal(2)]);   // creep 2 self-heals

let report = resolve_tick(&mut world, &intents);
// report.outcomes[&2] carries raw/effective damage + heal; report.deaths lists ids that died.
```

Run the conformance tests:

```bash
cargo test -p screeps-combat-engine
cargo check -p screeps-combat-engine --target wasm32-unknown-unknown
```

## What it models

- **Combat math** — per-part 100-hit pools with back-to-front degradation, boost-aware part power,
  and TOUGH/boost damage reduction; ranged-mass-attack distance falloff and tower output falloff.
- **The two-phase tick** — intent accumulation → movement → apply, with damage-then-heal netting and
  deaths.
- **Movement-conflict resolution** — eligibility/fatigue, swaps, the moves/weight tiebreak, obstacle
  and chain blocking, and `pull` for no-MOVE / under-MOVE compositions.
- **Structures** — ramparts, walls, spawns, and towers (which fire heal/repair/attack and are
  themselves targetable and repairable), with rampart damage-redirection and safe mode.
- **Recording** — an optional per-tick replay artifact (state + intents + reason tags + outcomes)
  with a deterministic text scrubber, for "see why it happened" introspection.

Every formula is a hand-port from a local clone of the
[screeps-engine](https://github.com/screeps/engine) source — not from documentation, not
machine-generated. Each cites the engine file and lines it ports in its doc comment, and is pinned by
host conformance tests against hand-computed engine values. The current port matches:

| Source | Pinned version |
|---|---|
| `screeps-engine` | `8097782` — v4.3.2 (2026-06-01) |
| `screeps-common` (constants) | `2fb779b` (2026-04-19) |
| `screeps-game-api` (value types) | `0.23.1` |

> Contributors: when the upstream engine changes, see [`AGENTS.md`](AGENTS.md) for the engine→port
> source map and the re-verification checklist.

## Modules

| Module | What it is |
|---|---|
| [`constants`](src/constants.rs) | Combat constants (powers, ranges, ranged-mass-attack + tower falloff, fatigue) transcribed from the engine. |
| [`body`](src/body.rs) | The body model: per-part 100-hit pools, back-to-front degradation (`_recalc-body`), boost-aware power (`calcBodyEffectiveness`), and the TOUGH/boost damage reduction (`_applyDamage`). |
| [`damage`](src/damage.rs) | Range-dependent formulas: ranged-mass-attack distance falloff and tower output falloff. |
| [`state`](src/state.rs) | The `CombatWorld` / `SimCreep` / `SimTower` value types (JS-free, over `screeps::Position`). |
| [`resolve`](src/resolve.rs) | The two-phase tick: intent priority/exclusion → per-target pooling → damage-then-heal netting → deaths → fatigue regen. |

## Determinism

No RNG, no wall-clock, no network. Outcomes never depend on `HashMap` iteration order — creeps are
processed in `CombatWorld::creeps` order and per-target pools are keyed by creep id. Identical
`(CombatWorld, Intents)` ⇒ identical result, every time — which is what makes seed-based combat gates
buildable.

## Related crates

This is the lowest layer of a four-crate combat stack (engine → decision → agent → eval):

- [screeps-combat-decision](https://github.com/Azaril/screeps-combat-decision) — the tactical seam +
  pure combat decisions, shared by the live bot and the sim so there is one implementation.
- [screeps-combat-agent](https://github.com/Azaril/screeps-combat-agent) — the sim harness: runs real
  decision code over this engine for self-play.
- [screeps-combat-eval](https://github.com/Azaril/screeps-combat-eval) — the policy layer: a
  metric-producing experiment register over the sim.

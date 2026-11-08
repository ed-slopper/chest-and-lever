# Levels

Rows C1, C2, C3, G11, X3. Code: `lac-rs/sim/src/levelgen.rs`; data `lac-rs/assets/content/prefabs.ron`. To see a
level from above: `cargo run -p lac-sim --example map -- <seed> <depth>`.

## From the record

- R44: levels are procedurally generated.
- R45: some structures are hand-built "with procedural generation in mind".
- R47: the first test level was seeded chaos.
- R42: crypts and tombs. R46 (unconfirmed): caves.

## Ours

- Eight levels of 48×12×48 cells, in four themes:

  | Depths | Theme | Materials |
  |---|---|---|
  | 1-2 | the Crypt | brick and cobble |
  | 3-4 | the Caves | rock, dirt, moss and pools |
  | 5-6 | the Tombs | marble and bone |
  | 7-8 | the Deep | obsidian, crystal and lava |

- Rooms and corridors: 8-12 rooms at three floor heights, joined by a spanning tree plus a few loops.
  Corridors climb one tile at a time, so every step up is a jump.
- A room may get one of:
  - a ledge with a chest on top
  - a trench of spikes, water or lava
  - a pool
  - pillars
  - a chasm with a lever bridge, and a step inside to climb out
  - a prefab, with a ring of free floor around it so it can never block the way
- Caves are a cellular automaton, with rolling floors and pools.
- A solver walks the level with the party's own moves. It hands out keys and levers only inside what is already
  reachable, and puts the stairs in the farthest room it can reach. A level with too little reachable is made
  again.
- Depth 8 has an 11×11 hall holding the Warden's vault and the great chest. Opening the chest wins the run.
- A run can be saved and continued: one save per run, deleted when the run ends. The game also saves on each
  new level.

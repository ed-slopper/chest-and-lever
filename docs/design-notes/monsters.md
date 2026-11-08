# Monsters

Rows G8, C4. Code: `lac-rs/sim/src/monster.rs`, `sim.rs` (`monsters_act`); data
`lac-rs/assets/content/monsters.ron`.

## From the record

- R41: mobs like more detailed Minecraft zombies.
- R42: a giant spider, and a ghost-like figure on tombstones.
- R43: in one test spiders spawned 10% of the time; Notch has arachnophobia.

## Ours

- All our own designs:
  - the cellar rat
  - the shambler, a zombie-like walker
  - the cave spider, and the rock beetle that stands in for it in the spider-free mode
  - the grey ooze
  - the tomb wisp, a ghost-like flier
  - the bone archer
  - the ghoul
  - the hollow knight
  - the mimic
  - the Warden of the Deep, who guards the great chest
- Speed is energy: a monster gains its `speed` each tick and acts once for every 12.
- A sleeping monster that sees the party wakes half the time. Awake monsters next to the party attack; archers
  shoot along straight lines; the rest walk a shortest path (breadth-first, fixed move order) or wander. The
  living (not the undead) flee at a quarter of their hit points, half the time.
- Walkers can drop one tile, spiders and beetles can climb one, and wisps fly.
- In the spider-free mode (Options), every spider spawns as a rock beetle.

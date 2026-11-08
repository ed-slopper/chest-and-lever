# Levers, chests, doors and secrets

Rows G6, G7, G13. Code: `lac-rs/sim/src/world.rs` (`Feature`, `LinkEffect`), `sim.rs` (`use_front`,
`open_chest`, `fire_link`), `levelgen.rs`.

## From the record

- Nothing beyond the working title. R72 and R73 are gaps.

## Ours

- A lever sits in a wall, facing out; the party uses it by facing it. Flipping it fires its link, a list of
  effects: toggle a door, raise or lower a bridge, reveal a secret wall, spawn monsters, flood cells, shoot
  darts, or wake the level.
- Pressure plates fire their link once, when stepped on: darts or an ambush.
- Doors:
  - Wooden doors open and close by hand.
  - A locked door takes a key of its kind (iron, bronze or bone). Without one, a rogue can pick it
    (d20 + DEX + level ≥ 15). With no rogue alive, the strongest member can force it (d20 + STR ≥ 16).
  - Portcullises open only by lever.
- Secret walls are wall cells. Using one searches it: the chance of finding it is 20%, plus 5% per level and
  per point of INT bonus, plus 25% for a rogue.
- Chests:
  - Some are locked, and open the same ways as doors.
  - 20% are trapped (needle, gas, fire, alarm, teleport). A living rogue may find and disarm the trap first
    (d20 + DEX + level ≥ 12).
  - 8% of chests from depth 2 down are mimics.
  - Loot comes from `loot.ron`.
- The level generator's solver places every key and lever inside the area the party can already reach, so every
  level can be finished.

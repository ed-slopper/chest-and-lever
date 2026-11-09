# The world: tiles, collision, jumping, falling

Rows E3, G1, G2. Code: `lac-rs/sim/src/world.rs`, `sim.rs` (`step`, `jump`, `long_jump`, `gravity`).

## From the record

- R20: movement is tile-based, first person: one tile per step, 90° turns.
- R21: a jump moves up one tile, with a moment of grace before falling, so a move queued right after it lands
  on the ledge.
- R22: a long jump crosses a one-tile gap.
- R23: collision comes from the level data.
- R40: voxel dungeons.

## Ours

- Cells are stored in 16³ chunks of `Cell` kinds. Air, water and lava are open; the rest is solid. Outside the
  level is bedrock. A tile is a cell, and the party stands in an open cell with a solid one under it.
- The grace is exactly one tick: the tick after a jump runs its action before gravity applies.
- A long jump flies up to two tiles forward and lands where it stops; gravity follows.
- A step up of one tile needs a jump; two tiles can't be climbed.
- Falls of three tiles or more hurt everyone for `(tiles - 2)d4`, unless the party lands in water.
- Lava burns everyone standing in it for `2d6` per tick; spikes do `1d6` on entering.
- Levitation (potions, rings, boots): no falling, no jumping, and the stairs can't be reached.
- Boots of jumping: the jump climbs two tiles and the long jump flies three.
- Doors are two tiles high and fill their doorway: the space above a closed door is shut too, so nobody jumps
  over a door. Doors stand only where the floor is level on both sides.
- Corridors are two tiles high: room to jump, no more.

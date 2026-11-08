# The simulation

Rows F4, F9, E1. Code: `lac-rs/sim/src/sim.rs`, `rng.rs`; tests `lac-rs/sim/tests/`.

## From the record

- R25: runs are seeded; the generator is a Mersenne Twister.
- R27: time stands still while the player gives no input.
- R24: player actions are decoupled from raw input.
- R13: turn-based.

## Ours

- `lac-sim` has no Bevy, no I/O, no clock, no threads and no hash-map iteration. Everything a run is lives in
  `State` (saved, loaded and hashed as RON).
- MT19937 is implemented in the crate (checked against the reference outputs for `init_genrand(5489)` and
  `init_by_array`) instead of the `rand_mt` crate, so the generator's state is plain data in saves and in the
  replay hash.
- The seed is text, trimmed and upper-cased, hashed with FNV-1a 64. Streams: `loot`, `ai`, `combat`, `world`,
  and one `levelgen.<depth>` per level, made fresh when the level is made. So a seed always gives the same
  dungeon, whatever the party did on the way down.
- `Sim::push` queues actions; `Sim::step` takes one; `Sim::tick` runs one. Actions that fail (a step into a
  wall, using nothing) take no time. Dropping, picking up and swapping members are free; everything else takes
  a tick.
- Order in a tick: the party's action, gravity (skipped on the tick of a jump), status (poison, regeneration,
  lava, timers), then monsters by id, then the automap.
- Replays: a `Recording` is seed + options + actions + the expected state hash. `tests/replay.rs` plays every
  file in `tests/runs/` twice, checks both runs match, and compares the hash. `LAC_BLESS=1` rewrites the hashes;
  do that only when a change in behaviour is meant, and say why in the commit. The app writes recordings with
  `--record <file>` and plays them with `--replay <file>`.

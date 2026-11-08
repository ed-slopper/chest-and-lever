# WORK.md: the board

Goal and rules: [`AGENTS.md`](AGENTS.md). Facts are cited by ID from [`notes/reference.md`](notes/reference.md)
(`R21`); the record is Notch's public posts about Levers and Chests, 2024-04 to 2026-10.

Columns:
- **ID**: stable, letter per section (F foundations, E engine, G gameplay, C content, X finishing, L later).
  Split rows get a letter or digit added (`E3` → `E3a`). IDs are never reused.
- **Area**: a person, `unowned`, `any`, or `humans` (a decision, not for agents).
- **Needs**: rows that must be in Done first.
- **Who**: empty = open, or `person/agent (date)`.
- **Notes**: `R..` = the facts the row builds from; `OURS` = the record is silent, the row designs it (tag the code
  `OURS:`); `[est]` = my estimate of the work.

Where things stand (2026-10-19): the board, the rules and the reference are up. There is no code yet: row F1 makes
the Cargo workspace, and almost everything waits on it. Five decisions are for the humans (F5-F8, G9a); none of
them blocks the first engine rows.

## P0 Foundations

| ID | Task | Area | Needs | Who | Notes |
|---|---|---|---|---|---|
| F1 | The Cargo workspace `lac-rs/`: the crate `lac-sim` (`lac-rs/sim/`, no Bevy, no I/O) and the app `lac` (`lac-rs/`, Bevy 0.20, versions pinned in `Cargo.lock`). The app opens a window, clears it to a colour from the sim, quits on Esc; `cargo test -p lac-sim` runs one test. Window title from F5 when decided, `chest-and-lever` until then | unowned | | ed-slopper/heron (2026-11-08) | [est] small. Linux packages in AGENTS.md §15. Keep `lac-sim` free of Bevy for good: it is what F4 replays |
| F2 | CI for the code: `.github/workflows/lac-rs.yml`, run when `lac-rs/**` changes or by hand: `cargo test -p lac-sim` (fast), then `cargo clippy` and `cargo build` for the app (Linux, with the packages Bevy needs, `Swatinem/rust-cache`) | unowned | F1 | | The Guard workflow (F0) already runs on every push |
| F9 | Random numbers in `lac-sim`: one run seed (shown and typed as text, so runs can be shared) split into named streams (`levelgen`, `loot`, `ai`, `combat`, ...), so adding a draw in one system never shifts another. Mersenne Twister (`rand_mt`) as the record uses (R25); tests with fixed seeds and expected values | unowned | F1 | ed-slopper/heron (2026-11-08) | R25, R44 |
| F4 | The replay test: a run = seed + list of actions; step `lac-sim` through it and compare a hash of the whole state with the expected value (`lac-rs/sim/tests/replay.rs`, recordings in `lac-rs/sim/tests/runs/`). The app writes and plays recordings (`--record <file>`, `--replay <file>`) | unowned | F1, F9, E1 | | Makes "deterministic" (AGENTS.md §13) checkable. Every sim row after it keeps it green |
| F3a | Read r/leversandchests (Top of all time, and everything since 2026-01) and add what is new to `notes/reference.md` (trust 3 unless it links a post of Notch's) and `notes/media.md` | any | F3 | | Our tools can't read Reddit: needs a browser (by hand, or an agent driving one) |
| F3b | Keep up with Notch's posts after 2026-10-10 (R3 is the newest): new facts into `notes/reference.md`, new pictures into `notes/media.md`, and a new open row for each fact that changes a done row | any | F3 | | Recurring: split a dated row (`F3b1`, ...) per pass, leave F3b open |
| F6 | The movement model: tile-based, one tile per step and 90° turns, as the record shows (R20, R21) for the game; free movement (R26) only as the later option L2. Confirm, or choose otherwise | humans | | | Agents assume tile-based until this says other |

## P1 Core engine

| ID | Task | Area | Needs | Who | Notes |
|---|---|---|---|---|---|
| E1 | The action queue and the tick in `lac-sim`: an `Action` enum (step forward/back/left/right, turn left/right, jump, long jump, use, attack, wait), a queue the app pushes to, and `Sim::tick` that takes one action and advances the world by one turn. No action, no tick: time stands still (R27) | unowned | F1 | ed-slopper/heron (2026-11-08) | R24, R27; R13 says turn-based |
| E2 | Playback in the app: the sim runs a tick as soon as an action is queued; the app animates the change, and the more actions wait in the queue the faster it animates, so actions can be spammed or watched (R28). The speed curve is ours | unowned | E1, E5 | | R27, R28; OURS: the curve |
| E3 | The grid world in `lac-sim`: tiles in chunks indexed by `IVec3` (solid, air, floor kinds, water), the party's tile and facing, a level struct saved and loaded as RON, a test level built from code | unowned | F1 | ed-slopper/heron (2026-11-08) | R20, R23, R40 |
| E4 | Voxel meshes in the app: one mesh per chunk (greedy meshing, hidden faces culled), a material per tile kind, rebuilt when the sim changes a chunk | unowned | E3 | | R40 |
| E5 | The first-person camera: placed from the party's tile and facing in the sim, a tweened step and a tweened 90° turn as in Legend of Grimrock (R14) | unowned | E1, E3 | | R14, R20; OURS: tween lengths, head bob |
| E6 | Input: keyboard and gamepad mapped to `Action`s (R24), rebindable, bindings saved in a settings file | unowned | E1 | | R24. `leafwing-input-manager` or our own |
| E7 | The palette pipeline: textures stored as palette indices, a palette texture, a material that looks colours up, and palette ranges that cycle over time (water, fire, glow) (R54) | unowned | E4, F8 | | R54 |
| E8 | Light: Bevy's SSAO (R50), a light around the party (ours), sky and fog colours from an artist-made lookup texture (R53) | unowned | E4 | | R50, R53; OURS: the party's light |
| E9 | The retro output: render to a low-resolution target and scale it up with nearest filtering; the LCD-style splash screen (R55) | unowned | F1, F8 | | R55 |
| E10 | Content as data: items, monsters, loot tables and prefabs as RON files, loaded as Bevy assets and handed to `lac-sim` as plain types; hot reload in debug builds | unowned | E3 | | Makes R12's broken combinations a matter of tuning |
| E11 | The debug overlay: seed, tick, queue length, party tile and facing, frame time; a key to toggle it | any | F1 | | |

## P2 Gameplay systems

| ID | Task | Area | Needs | Who | Notes |
|---|---|---|---|---|---|
| G1 | Collision and steps: walls block, floors hold, a step up of one tile needs a jump, a missing floor is a fall (R23) | unowned | E1, E3 | | R21, R23 |
| G2 | The jump and the long jump, exactly as the record: up one tile with one tick of grace before the fall, so a move queued right after it lands on the ledge (R21); a long jump across a one-tile gap (R22); falling and landing | unowned | G1 | | R21, R22, R29; OURS: fall damage |
| G3 | The party of four (R30): character sheets with attributes, hit points, class; front and back row; who acts when | unowned | E1 | | R30, R33; OURS: R71 (classes, attributes) |
| G4 | The inventory: item icons (R34), equipment slots, a pack per character or shared (ours), picking up from and dropping onto tiles | unowned | G3, E10 | | R34; OURS: the layout |
| G5 | Items: weapons, armour, potions, scrolls, wands; unidentified until used or identified; cursed items that won't come off (R12); combinations that break the game on purpose (R12) | unowned | G4 | | R12; OURS: R74 |
| G6 | Levers: on/off, a list of linked targets, and the events a flip fires (open or close a door, raise a bridge, spring a trap, flood a room, wake monsters) | unowned | E3, E10 | | OURS: R72 |
| G7 | Chests: opened, locked (keys), trapped; their loot rolled from the `loot` stream; some are mimics; some items cursed | unowned | G5, F9 | | OURS: R73 |
| G8 | Monsters: entities on tiles that act once per tick; chase, attack, flee; spawned from the `ai` stream (R41-R43) | unowned | E1, G1, F9 | | R41-R43 |
| G9 | Combat: melee and ranged, front and back rows, to-hit, damage, a character's death | unowned | G3, G8, G9a | | OURS: R70, by G9a |
| G10 | Abilities and spells; R36's unconfirmed time control is an idea, not a requirement | unowned | G9 | | OURS: R78 |
| G11 | The run: permadeath, descending levels, the end screen (seed, depth, cause, stats), how a run is won | unowned | G9, C1 | | R12; OURS: R75, R77 |
| G12 | The minimap: an automap of the tiles the party has seen (R35) | unowned | E3, E5 | | R35 |
| G13 | Doors, keys and secret walls | unowned | G6 | | OURS |

## P3 Content

| ID | Task | Area | Needs | Who | Notes |
|---|---|---|---|---|---|
| C1 | The level generator: rooms and corridors for each depth from the `levelgen` stream (R44, R47) | unowned | E3, F9 | | R44, R47 |
| C2 | Prefabs: hand-built rooms in RON, stamped into generated levels (R45); a first handful of rooms | unowned | C1, E10 | | R45 |
| C3 | Themes by depth: crypts and tombs (R42), caves (R46, unconfirmed), more of our own | unowned | C1 | | R42, R46 |
| C4 | The first monsters, our own designs: a zombie-like walker (R41), a giant spider, which a setting can replace (R43, X4), a ghost (R42) | unowned | G8, C9 | | R41-R43 |
| C5 | The first items, with cursed ones | unowned | G5 | | R12 |
| C6 | Four starting characters: our own names, classes and portraits (not Mark, Reverie or Jess) | unowned | G3, F5 | | R32, R33 |
| C7 | Portrait art | unowned | F8 | | Credits in `assets/CREDITS.md` |
| C8 | Tile textures, palette-indexed | unowned | F8, E7 | | Credits in `assets/CREDITS.md` |
| C9 | Voxel models for monsters and items | unowned | F8 | | Credits in `assets/CREDITS.md` |
| C10 | Sound effects and music | unowned | F7 | | Credits in `assets/CREDITS.md` |

## P4 Finishing

| ID | Task | Area | Needs | Who | Notes |
|---|---|---|---|---|---|
| X1 | The HUD: portraits on the right (R30), hit points, the hands or attack buttons, the message line | unowned | G3, E5 | | R30 |
| X2 | Menus: title after the splash (E9), a new run with a seed to type or roll, options with key rebinding (E6), quit | unowned | E9, E6 | | |
| X3 | Save and quit in the middle of a run: one save per run, deleted on death | unowned | G11 | | |
| X4 | Options: a spider-free mode (R43), volume, resolution scale, colour-blind palettes | unowned | X2, C4 | | R43 |
| X5 | Release builds for Windows, Linux and macOS from CI; a store page (itch.io) | unowned | F2, F5, F7 | | |
| X6 | Review every `OURS:` tag in `lac-rs` against `notes/reference.md` after new facts (F3b): replace the ones the record now answers, one new row each | any | F3b | | |

## P5 Later

| ID | Task | Area | Needs | Who | Notes |
|---|---|---|---|---|---|
| L1 | Other clocks besides the action queue: food, light, rest | unowned | G11 | | OURS: R76 |
| L2 | Free movement as an option (R26) | unowned | F6, G2 | | R26 |
| L3 | Modding: content packs (E10's RON files) loaded from a folder; the record floated an engine licence (R10) | unowned | E10 | | R10 |
| L4 | A raytraced voxel renderer, as an experiment (R51, R52) | unowned | E4 | | R51, R52 |
| L5 | A web build (wasm) | unowned | F1 | | |
| L6 | Multiplayer | unowned | G11 | | R59 (unconfirmed) |

## Done

| ID | Task | Section | Who | Date | Commit |
|---|---|---|---|---|---|
| F5 | Our game's name: **Chest and Lever** (window title `Chest and Lever`; crates keep the `lac` names) | P0 | ed-slopper/heron | 2026-11-08 | c2fdf8d |
| F7 | Licences: our code MIT OR Apache-2.0, our assets CC BY 4.0 | P0 | ed-slopper/heron | 2026-11-08 | c2fdf8d |
| F8 | Art direction: render at 480×270, scaled up with nearest filtering; one 64-colour palette (8 ramps of 8); 16 texels per tile edge; all art made by us (generated in code), our own look, not traced from the record's pictures | P0 | ed-slopper/heron | 2026-11-08 | c2fdf8d |
| G9a | Combat model: d20 + bonus against armour class to hit, weapon dice for damage (D&D-like, R16); one Attack action makes the whole party strike, monsters act after every party action | P2 | ed-slopper/heron | 2026-11-08 | c2fdf8d |
| F3 | The reference: notes/reference.md (facts R1-R59, gaps R70-R78), notes/media.md, docs/design-notes/README.md | P0 | ed-slopper/wren | 2026-10-19 | 1ef7013 |
| F0 | The board, the rules and the CI guard: WORK.md, AGENTS.md, CLAUDE.md, README.md, tools/check_board.py, tools/check_assets.py, .github/workflows/guard.yml, the allow-list .gitignore | P0 | ed-slopper/wren | 2026-10-19 | 1ef7013 |

## Changes to the board

- 2026-10-19 ed-slopper/wren: board, rules and reference made, after the structure of ed-slopper/SSX-TRICKY-RUST-REWRITE (rows F0-F9, E1-E11, G1-G13, C1-C10, X1-X6, L1-L6); F0 and F3 claimed.
- 2026-10-19 ed-slopper/wren: F0 and F3 done (1ef7013).
- 2026-11-08 ed-slopper/heron: ed-slopper decided F5 (name Chest and Lever), F7 (MIT/Apache-2.0 + CC BY 4.0), F8 (480×270, 64 colours) and G9a (d20, whole party acts per tick); recorded in Done. F1, F9, E1, E3 claimed.

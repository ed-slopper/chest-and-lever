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

Where things stand (2026-11-08): the game is playable from the title screen to the great chest at depth 8. The
humans decided F5, F7, F8 and G9a; F6 stands as tile-based. Open: research passes (F3a, F3b), balance (G14),
the store page (X5b) and the later rows.

## P0 Foundations

| ID | Task | Area | Needs | Who | Notes |
|---|---|---|---|---|---|
| F3a | Read r/leversandchests (Top of all time, and everything since 2026-01) and add what is new to `notes/reference.md` (trust 3 unless it links a post of Notch's) and `notes/media.md` | any | F3 | | Our tools can't read Reddit: needs a browser (by hand, or an agent driving one). 2026-11-09 heron: the desktop app's built-in browser refuses reddit.com too; try Claude in Chrome or by hand |
| F3b | Keep up with Notch's posts after 2026-10-10 (R3 is the newest): new facts into `notes/reference.md`, new pictures into `notes/media.md`, and a new open row for each fact that changes a done row | any | F3 | | Recurring: split a dated row (`F3b1`, ...) per pass, leave F3b open |
| F6 | The movement model: tile-based, one tile per step and 90° turns, as the record shows (R20, R21) for the game; free movement (R26) only as the later option L2. Confirm, or choose otherwise | humans | | | Agents assume tile-based until this says other |

## P1 Core engine

| ID | Task | Area | Needs | Who | Notes |
|---|---|---|---|---|---|
| E12 | SSAO as R65 describes it: dithered samples, a blur that respects normals and distance, then a light dither on top, in place of Bevy's default SSAO (`lac-rs/src/game.rs`, the camera); keep the options toggle | unowned | E8 | ed-slopper/heron (2026-11-09) | Gap of F3b1: R65 changes E8 |

## P2 Gameplay systems

| ID | Task | Area | Needs | Who | Notes |
|---|---|---|---|---|---|

## P3 Content

| ID | Task | Area | Needs | Who | Notes |
|---|---|---|---|---|---|

## P4 Finishing

| ID | Task | Area | Needs | Who | Notes |
|---|---|---|---|---|---|
| X5a | Release builds for Windows, Linux and macOS from CI (`.github/workflows/release.yml`, on a `v*` tag): zips of the game and its assets on a GitHub release | unowned | F2, F5, F7 | ed-slopper/heron (2026-11-08) | Split from X5; the workflow is in, not yet run on a tag |
| X5b | A store page (itch.io) and the first tagged release | humans | X5a | | Split from X5; needs an account and the decision to publish |
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
| L7 | An overworld between key locations, perhaps seamless with the dungeons (R60) | unowned | G11 | | R60 (trust 3) |

## Done

| ID | Task | Section | Who | Date | Commit |
|---|---|---|---|---|---|
| F1 | The Cargo workspace `lac-rs/`: the crate `lac-sim` (`lac-rs/sim/`, no Bevy, no I/O) and the app `lac` (`lac-rs/`, Bevy 0.20, versions pinned in `Cargo.lock`). The app opens a window, clears it to a colour from the sim, quits on Esc; `cargo test -p lac-sim` runs one test. Window title from F5 when decided, `chest-and-lever` until then | P0 | ed-slopper/heron | 2026-11-08 | 098faef |
| F2 | CI for the code: `.github/workflows/lac-rs.yml`, run when `lac-rs/**` changes or by hand: `cargo test -p lac-sim` (fast), then `cargo clippy` and `cargo build` for the app (Linux, with the packages Bevy needs, `Swatinem/rust-cache`) | P0 | ed-slopper/heron | 2026-11-08 | 098faef |
| F9 | Random numbers in `lac-sim`: one run seed (shown and typed as text, so runs can be shared) split into named streams (`levelgen`, `loot`, `ai`, `combat`, ...), so adding a draw in one system never shifts another. Mersenne Twister (`rand_mt`) as the record uses (R25); tests with fixed seeds and expected values | P0 | ed-slopper/heron | 2026-11-08 | 098faef |
| F4 | The replay test: a run = seed + list of actions; step `lac-sim` through it and compare a hash of the whole state with the expected value (`lac-rs/sim/tests/replay.rs`, recordings in `lac-rs/sim/tests/runs/`). The app writes and plays recordings (`--record <file>`, `--replay <file>`) | P0 | ed-slopper/heron | 2026-11-08 | 098faef |
| E1 | The action queue and the tick in `lac-sim`: an `Action` enum (step forward/back/left/right, turn left/right, jump, long jump, use, attack, wait), a queue the app pushes to, and `Sim::tick` that takes one action and advances the world by one turn. No action, no tick: time stands still (R27) | P1 | ed-slopper/heron | 2026-11-08 | 098faef |
| E2 | Playback in the app: the sim runs a tick as soon as an action is queued; the app animates the change, and the more actions wait in the queue the faster it animates, so actions can be spammed or watched (R28). The speed curve is ours | P1 | ed-slopper/heron | 2026-11-08 | 098faef |
| E3 | The grid world in `lac-sim`: tiles in chunks indexed by `IVec3` (solid, air, floor kinds, water), the party's tile and facing, a level struct saved and loaded as RON, a test level built from code | P1 | ed-slopper/heron | 2026-11-08 | 098faef |
| E4 | Voxel meshes in the app: one mesh per chunk (greedy meshing, hidden faces culled), a material per tile kind, rebuilt when the sim changes a chunk | P1 | ed-slopper/heron | 2026-11-08 | 098faef |
| E5 | The first-person camera: placed from the party's tile and facing in the sim, a tweened step and a tweened 90° turn as in Legend of Grimrock (R14) | P1 | ed-slopper/heron | 2026-11-08 | 098faef |
| E6 | Input: keyboard and gamepad mapped to `Action`s (R24), rebindable, bindings saved in a settings file | P1 | ed-slopper/heron | 2026-11-08 | 098faef |
| E7 | The palette pipeline: textures stored as palette indices, a palette texture, a material that looks colours up, and palette ranges that cycle over time (water, fire, glow) (R54) | P1 | ed-slopper/heron | 2026-11-08 | 098faef |
| E8 | Light: Bevy's SSAO (R50), a light around the party (ours), sky and fog colours from an artist-made lookup texture (R53) | P1 | ed-slopper/heron | 2026-11-08 | 098faef |
| E9 | The retro output: render to a low-resolution target and scale it up with nearest filtering; the LCD-style splash screen (R55) | P1 | ed-slopper/heron | 2026-11-08 | 098faef |
| E10 | Content as data: items, monsters, loot tables and prefabs as RON files, loaded as Bevy assets and handed to `lac-sim` as plain types; hot reload in debug builds | P1 | ed-slopper/heron | 2026-11-08 | 098faef |
| E11 | The debug overlay: seed, tick, queue length, party tile and facing, frame time; a key to toggle it | P1 | ed-slopper/heron | 2026-11-08 | 098faef |
| G1 | Collision and steps: walls block, floors hold, a step up of one tile needs a jump, a missing floor is a fall (R23) | P2 | ed-slopper/heron | 2026-11-08 | 098faef |
| G2 | The jump and the long jump, exactly as the record: up one tile with one tick of grace before the fall, so a move queued right after it lands on the ledge (R21); a long jump across a one-tile gap (R22); falling and landing | P2 | ed-slopper/heron | 2026-11-08 | 098faef |
| G3 | The party of four (R30): character sheets with attributes, hit points, class; front and back row; who acts when | P2 | ed-slopper/heron | 2026-11-08 | 098faef |
| G4 | The inventory: item icons (R34), equipment slots, a pack per character or shared (ours), picking up from and dropping onto tiles | P2 | ed-slopper/heron | 2026-11-08 | 098faef |
| G5 | Items: weapons, armour, potions, scrolls, wands; unidentified until used or identified; cursed items that won't come off (R12); combinations that break the game on purpose (R12) | P2 | ed-slopper/heron | 2026-11-08 | 098faef |
| G6 | Levers: on/off, a list of linked targets, and the events a flip fires (open or close a door, raise a bridge, spring a trap, flood a room, wake monsters) | P2 | ed-slopper/heron | 2026-11-08 | 098faef |
| G7 | Chests: opened, locked (keys), trapped; their loot rolled from the `loot` stream; some are mimics; some items cursed | P2 | ed-slopper/heron | 2026-11-08 | 098faef |
| G8 | Monsters: entities on tiles that act once per tick; chase, attack, flee; spawned from the `ai` stream (R41-R43) | P2 | ed-slopper/heron | 2026-11-08 | 098faef |
| G9 | Combat: melee and ranged, front and back rows, to-hit, damage, a character's death | P2 | ed-slopper/heron | 2026-11-08 | 098faef |
| G10 | Abilities and spells; R36's unconfirmed time control is an idea, not a requirement | P2 | ed-slopper/heron | 2026-11-08 | 098faef |
| G11 | The run: permadeath, descending levels, the end screen (seed, depth, cause, stats), how a run is won | P2 | ed-slopper/heron | 2026-11-08 | 098faef |
| G12 | The minimap: an automap of the tiles the party has seen (R35) | P2 | ed-slopper/heron | 2026-11-08 | 098faef |
| G13 | Doors, keys and secret walls | P2 | ed-slopper/heron | 2026-11-08 | 098faef |
| C1 | The level generator: rooms and corridors for each depth from the `levelgen` stream (R44, R47) | P3 | ed-slopper/heron | 2026-11-08 | 098faef |
| C2 | Prefabs: hand-built rooms in RON, stamped into generated levels (R45); a first handful of rooms | P3 | ed-slopper/heron | 2026-11-08 | 098faef |
| C3 | Themes by depth: crypts and tombs (R42), caves (R46, unconfirmed), more of our own | P3 | ed-slopper/heron | 2026-11-08 | 098faef |
| C4 | The first monsters, our own designs: a zombie-like walker (R41), a giant spider, which a setting can replace (R43, X4), a ghost (R42) | P3 | ed-slopper/heron | 2026-11-08 | 098faef |
| C5 | The first items, with cursed ones | P3 | ed-slopper/heron | 2026-11-08 | 098faef |
| C6 | Four starting characters: our own names, classes and portraits (not Mark, Reverie or Jess) | P3 | ed-slopper/heron | 2026-11-08 | 098faef |
| C7 | Portrait art | P3 | ed-slopper/heron | 2026-11-08 | 098faef |
| C8 | Tile textures, palette-indexed | P3 | ed-slopper/heron | 2026-11-08 | 098faef |
| C9 | Voxel models for monsters and items | P3 | ed-slopper/heron | 2026-11-08 | 098faef |
| C10 | Sound effects and music | P3 | ed-slopper/heron | 2026-11-08 | 098faef |
| X1 | The HUD: portraits on the right (R30), hit points, the hands or attack buttons, the message line | P4 | ed-slopper/heron | 2026-11-08 | 098faef |
| X2 | Menus: title after the splash (E9), a new run with a seed to type or roll, options with key rebinding (E6), quit | P4 | ed-slopper/heron | 2026-11-08 | 098faef |
| X3 | Save and quit in the middle of a run: one save per run, deleted on death | P4 | ed-slopper/heron | 2026-11-08 | 098faef |
| X4 | Options: a spider-free mode (R43), volume, resolution scale, colour-blind palettes | P4 | ed-slopper/heron | 2026-11-08 | 098faef |
| G16 | Uniques at most once per run: one depth per unique from the seed, placed there once; never from drops | P2 | ed-slopper/heron | 2026-11-08 | 098faef |
| F3b1 | Notch's posts and press up to 2026-11-09: nothing newer than R3 (2026-10-10); filled in R65 (how the SSAO is made, 80.lv May 2024), R66 ("not given up yet", 2025-11-11), R67 (AI for tools, unconfirmed); E12 added | P0 | ed-slopper/heron | 2026-11-09 | 1773d14 |
| G14 | Balance: the bot plans around levers and locked doors and stalls far less; the Warden and hollow knight eased; it wins 6/40 (seeds 1-40) and 7/40 (41-80) | P2 | ed-slopper/heron | 2026-11-08 | 87d8011 |
| G15 | Bringing back the fallen: the priest's Raise (level 5) and a scroll of raise dead | P2 | ed-slopper/heron | 2026-11-08 | 098faef |
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
- 2026-11-08 ed-slopper/heron: F1, F2, F4, F9, E1-E11, G1-G13, C1-C10, X1-X4 done (098faef); X5 split into X5a (release workflow, claimed) and X5b (store page, humans); G14 added (balance, a gap of G9); G15 (raise dead) added and done; G16 (uniques once per run) added and done; L7 (overworld, R60) added.
- 2026-11-08 ed-slopper/heron: G14 done (87d8011).
- 2026-11-09 ed-slopper/heron: F3a released (reddit.com refused in the built-in browser); F3b1 split from F3b and done; E12 added (SSAO as R65, a gap of F3b1).

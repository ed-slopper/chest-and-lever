TUTORIAL TO CONTRIBUTE TO PROJECT:

1 Get added to the repo dm me ur github

2 link claude to github and add the repo (settings > connectors > github login and connect the repo)

3 point this link to claude and it will automatically start working on something > https://github.com/ed-slopper/chest-and-lever/blob/main/AGENTS.md


# chest-and-lever

A fan-made recreation, in Rust and Bevy, of **Levers and Chests**: the first-person, tile-based, party dungeon
crawler and old-school roguelike that Markus "Notch" Persson's studio Bitshift Entertainment has shown since
April 2024 but not released.

What we know of it, from Notch's posts and the press, is in [`notes/reference.md`](notes/reference.md): one tile per
step, a jump of one tile up with a moment's grace, time that stands still until you queue an action and speeds up
the more you queue, a party of four, seeded levels mixing generated and hand-built rooms, cursed items, voxel
dungeons with ambient occlusion and palette cycling. What the record doesn't show (combat, what levers and chests
actually do, classes) we design ourselves and mark as ours.

## How the work is organised

| File | What |
|---|---|
| [`AGENTS.md`](AGENTS.md) | The rules: how people and their AI agents pick, claim and finish work |
| [`WORK.md`](WORK.md) | The board: every task by priority, who has it, what's done |
| [`notes/reference.md`](notes/reference.md) | The facts we build from, each with an ID (`R21`) and a source |
| [`notes/media.md`](notes/media.md) | Links to every known picture and clip (the files stay out of the repo) |
| [`docs/design-notes/`](docs/design-notes/) | How each of our systems works: from the record vs our choices |
| `lac-rs/` | The game (made by board row F1): `sim/` the deterministic, Bevy-free rules; the app around it |
| `tools/` | `check_board.py` (the board's format), `check_assets.py` (every asset credited) |

## Play

You need Rust (stable; on Windows the MSVC toolchain) and, on Linux, the packages in `AGENTS.md` section 15.

```
cd lac-rs
cargo run --release
```

Descend eight levels and open the great chest guarded by the Warden of the Deep. Time stands still until you
act. Queue several actions and they play back faster. Default keys:

| Keys | Do |
|---|---|
| W / S / A / D | step forward, back, left, right |
| Q / E (or the arrow keys) | turn |
| Space | jump (up one tile; step right after to land on a ledge) |
| Shift | long jump (across a one-tile gap) |
| F | use the lever, door, chest or fountain in front; search a wall |
| R | the whole party attacks |
| Z | wait |
| G | pick up |
| I | inventory |
| C | spells |
| M | map |
| 1-4 | pick a member |
| Esc | menu (save and quit) |
| T | rest until healed (stops if danger shows) |
| N | mark the map |
| F1 / H | help |
| [ / ] | darker / brighter |

All keys can be rebound in Options. A gamepad works too once it is switched on there (it is off by default,
because virtual controllers from other software sent stray presses).

Other ways to start:

- `--seed <text>`: start a run on that seed at once
- `--record <file>`: write the run's actions to a file
- `--replay <file>`: play such a file back
- `--test-level`: start on the small jumping test level

The rules' tests and replays: `cargo test -p lac-sim`. See any level from above:
`cargo run -p lac-sim --example map -- <seed> <depth>`.

## Status

Playable from start to finish, and all of it is our own work:

- **The rules** (`lac-rs/sim`): deterministic, with replay tests. Hybrid ticks: time passes only while an action
  plays, and turning is free.
- **The dungeon:** eight seeded levels in four themes, with ledges, chasms, lever bridges, secret vaults and
  hand-built rooms.
- **Party and items:** a party of four, items with curses and unidentified looks, resistances that heal past
  100%, set and unique items.
- **Monsters and combat:** monsters with health bars, and d20 combat.
- **The look:** voxel walls carved in relief, palette cycling and SSAO, glowing particles for spells.
- **The interface:** the party as cards with slots you click, a pack grid, a slot picker, map marks, resting,
  options, saving, and sound synthesised in code.

The board ([`WORK.md`](WORK.md)) lists what is done and what is still open.

## Legal

Unofficial fan project, not affiliated with or endorsed by Markus Persson or Bitshift Entertainment. "Levers and
Chests" is their working title and is used here only to say what we recreate; the game's own name is **Chest and Lever** (board row F5). This repository contains none of their pictures, models, textures, sounds or code: game
mechanics are rebuilt from public descriptions, and every asset here is our own or openly licensed (see
`assets/CREDITS.md`). Code: MIT OR Apache-2.0; our assets: CC BY 4.0 (board row F7).

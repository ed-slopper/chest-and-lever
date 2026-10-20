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

## Status

The board and the reference are up; no game code yet. Start at row F1 of [`WORK.md`](WORK.md).

## Legal

Unofficial fan project, not affiliated with or endorsed by Markus Persson or Bitshift Entertainment. "Levers and
Chests" is their working title and is used here only to say what we recreate; the game's own name is still to be
decided (board row F5). This repository contains none of their pictures, models, textures, sounds or code: game
mechanics are rebuilt from public descriptions, and every asset here is our own or openly licensed (see
`assets/CREDITS.md`).

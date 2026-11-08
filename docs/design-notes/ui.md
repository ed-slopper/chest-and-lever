# The screen: HUD, map, menus

Rows X1, X2, X4, G12, E11. Code: `lac-rs/src/hud.rs`, `menu.rs`.

## From the record

- R30: portraits on the right. R35: a minimap. R34: an inventory with item icons. R55: an LCD-style splash.

## Ours

- Left: a status line, the 3D view, and the last six messages.
- Right: one card per member (portrait, name, class, level, row, HP and mana bars, AC, poison), then a 48×48
  automap centred on the party.
- Overlays on the view:
  - the inventory: worn items and the pack, with icons
  - spells
  - pause: resume, save and quit, abandon
  - the map of the whole level
  - debug (F3): seed, tick, queue, tile and facing, depth, monsters, frame time, state hash
- Screens:
  - the splash
  - the title: continue, new run, options, quit
  - a new run: type a seed or roll one
  - options: spider-free mode, sound and music volume, colours, resolution, SSAO, key bindings
  - the end of a run: seed, depth, cause, ticks, kills, gold, chests, levers, and each member's level

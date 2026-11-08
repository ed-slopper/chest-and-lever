# The screen: HUD, map, menus

Rows X1, X2, X4, G12, E11. Code: `lac-rs/src/hud.rs`, `menu.rs`.

## From the record

- R30: portraits on the right. R35: a minimap. R34: an inventory with item icons. R55: an LCD-style splash.

## Ours

- Left: the 3D view, the full height of the window (4:3), with the status line and the last six messages drawn
  over it.
- Right (layout as ed-slopper asked, 2026-11-08; our own art): a stone panel with
  - the party as a 2×2 grid of cards, front row on top: name, class and level, four equipment slots (weapon,
    shield, head, body), the portrait, health and mana bars, HP, mana, AC and poison;
  - the pack as a 6×4 grid of item icons, with gold and who is acting;
  - the automap, centred on the party and turned so forward is up, and the main keys.
- Mouse: click a card to choose who acts; click a pack item to use or wear it (the acting member, or the first
  one alive if they have fallen); right-click to drop it; click a worn item to take it off.
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
  - loading, while the level is made
  - options: spider-free mode, sound and music volume, colours, resolution, SSAO, brightness, key bindings
  - the end of a run: seed, depth, cause, ticks, kills, gold, chests, levers, and each member's level
- In play, `[` and `]` change the brightness, and F1 (or H) shows the help screen.
- Marks on the map (R60): N puts a mark on the party's tile; pressing again cycles four colours, then clears it.
  Marks are saved with the level.
- Health bars (R61): a thin bar floats over a monster once it is hurt, turned to the camera.

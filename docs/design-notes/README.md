# Design notes: how our game works, system by system

One file per system. Each file has two parts, kept apart:

- **From the record**: what `notes/reference.md` shows, by fact ID (`R21`), and how we read it.
- **Ours**: what we decided where the record is silent, why, and the board row that decided it. The same choices
  carry `OURS:` in the code.

Start a system's file when its row starts and list it here. Planned files (empty until their rows run):

| File | System | Rows |
|---|---|---|
| `sim.md` | The deterministic simulation: ticks, the action queue, seeds and random streams, replays | F4, F9, E1 |
| `time.md` | The clock: frozen while idle, speed-up with queued actions (R27, R28) | E1, E2 |
| `world.md` | The tile grid, collision, jumping and falling (R20-R23) | E3, G1, G2 |
| `input.md` | Actions and key bindings (R24) | E6 |
| `rendering.md` | Voxel meshes, palette and palette cycling, AO, sky, low-resolution output (R40, R50-R55) | E4, E7-E9 |
| `party.md` | The four characters, their sheets and the inventory (R30-R34) | G3, G4 |
| `items.md` | Items, identification, curses (R12, R74) | G5 |
| `levers-chests.md` | Levers, links, chests, loot and traps (R72, R73) | G6, G7 |
| `monsters.md` | Monsters and their AI (R41-R43) | G8 |
| `combat.md` | How fights resolve (R70) | G9 |
| `levels.md` | Level generation and prefabs (R44, R45, R47) | C1, C2 |
| `ui.md` | HUD, minimap, menus (R30, R35) | G12, X1, X2 |

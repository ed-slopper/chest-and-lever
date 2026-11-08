# Input

Row E6. Code: `lac-rs/src/settings.rs`, `game.rs` (`input`).

## From the record

- R24: actions are decoupled from raw input, for custom key bindings.

## Ours

- Keys map to `Command`s. Commands that are sim actions are queued; the rest open screens. Bindings are saved in
  `settings.ron` in the user's data folder (`%APPDATA%/chest-and-lever` on Windows,
  `~/.local/share/chest-and-lever` elsewhere) and can be changed in Options > Key bindings.
- Default keys:
  - Move and turn: W/S/A/D step, Q/E turn (the arrow keys and the number pad work too).
  - Actions: Space jump, Shift long jump, F/Enter use, R/Ctrl attack, Z/./Num5 wait, G pick up.
  - Screens: I/Tab inventory, C spells, M map, F3 debug, Esc menu.
  - T rests (waits, fast, until everyone is healed; stops when an awake monster is in sight or someone is hurt);
    N marks the map.
  - 1-4 pick a party member.
- Gamepad: the D-pad steps and turns, the shoulder buttons strafe, A jumps, B long-jumps, X uses, Y attacks,
  Start opens the menu, Select the inventory.

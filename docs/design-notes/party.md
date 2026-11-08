# The party

Rows G3, G4, C6, C7. Code: `lac-rs/sim/src/party.rs`, `lac-rs/src/hud.rs`, `icons.rs`.

## From the record

- R30: a group of four, with portraits on the right of the screen.
- R33 (unconfirmed): a character sheet with attributes and flavour text.
- R34: an inventory with item icons.
- R32: named characters. We use none of their names.

## Ours (R71 is a gap)

- Classes:

  | Class | Hit die | Weapons | Special |
  |---|---|---|---|
  | Fighter | d10 | +2 | |
  | Rogue | d8 | +1 | picks locks, finds traps |
  | Priest | d8 | +0 | 4 mana a level |
  | Mage | d6 | +0 | 5 mana a level |

- Attributes are STR, DEX, CON, INT and WIS, with modifiers as in D&D: (score − 10) / 2.
- The party is Brann (Fighter), Sel (Rogue), Odile (Priest) and Quill (Mage). Members 0 and 1 are the front
  row.
- Experience is shared among the living; reaching level n+1 needs 15 × 2^(n−1).
- Everyone heals 1 HP every 20 ticks; casters regain 1 mana every 8.
- The party shares one pack of 24 items. Gold doesn't take a slot.
- Portraits are drawn in code, by class.
- A fallen member can come back (board row G15): the priest's Raise (level 5, 14 mana) or a scroll of raise
  dead stands every fallen member up at a quarter of their hit points.
- Spells: the mage has Spark (level 1), Fireball (3) and Quicken (5); the priest has Mend (1), Purify (2),
  Rebuke (3) and Raise (5).

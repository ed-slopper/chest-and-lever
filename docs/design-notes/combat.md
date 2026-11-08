# Combat

Row G9. Code: `lac-rs/sim/src/sim.rs` (`attack`, `monster_attack`, `cast`).

## From the record

- R16: the early pitch had "D20s" (superseded as a pitch; we keep the dice).
- R70: how combat resolves is a gap.

## Ours (G9a, decided by ed-slopper on 2026-11-08: d20 dice, the whole party acts each tick)

- To hit: d20 + bonus against armour class (ascending, base 10). A natural 20 always hits and doubles the
  damage; a natural 1 always misses.
- One Attack action:
  - The front row strikes the monster in front.
  - The back row strikes only with reach weapons (at the monster in front) or ranged weapons (at the first
    monster in line, up to 6 tiles).
- The party's to-hit bonus is half the level, plus the STR modifier (DEX for finesse and ranged weapons), plus
  the enchantment, plus the class skill. Damage is the weapon dice, plus the STR modifier (melee only), plus the
  enchantment.
- Monsters attack the front row while it stands. They reach the back row only once the front row has fallen,
  or by shooting.
- Spells hit without a roll.
- A character at 0 HP is dead for the rest of the run; the run ends when all four are.

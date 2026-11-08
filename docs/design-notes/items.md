# Items

Rows G5, C5. Code: `lac-rs/sim/src/item.rs`, `content.rs`; data `lac-rs/assets/content/items.ron`,
`appearances.ron`.

## From the record

- R12: a roguelike "in the old sense", with cursed items and ridiculously broken mechanics.
- R14: modelled on ADOM and NetHack.

## Ours (R74 is a gap)

- Potions, scrolls, wands, rings and amulets get a random look each run, shuffled from the `loot` stream. Their
  kind becomes known once one is used, worn or identified.
- Weapons and armour hide their enchantment and curse until worn or identified. 8% of them, plus the item's own
  chance, come cursed at −1 to −3. A quarter of the rest are +1 or better, more so deeper down.
- A cursed item that is worn can't come off. Remove curse lifts every curse.
- Identify names one to three things.
- Broken on purpose:
  - An enchant scroll on a +5 item makes it evaporate.
  - Boots of jumping make the long jump cross two-tile chasms that are meant to need a lever.
  - Levitation floats over lava moats.
  - A wand of digging cuts through anything but bedrock, secret walls included.
  - Rings of teleportitis jump the party about at random.

## After the design notes (R60)

- Harm has a kind: physical, fire, cold, poison, lightning. Wards (rings, an amulet, boots) give percent
  resistance; it stacks across everything one member wears, and past 100% the harm heals instead. Ring of fire
  ward (60%) plus Emberheart (120%) lets a member bathe in lava. Spiders and slimes poison, wisps and the Warden
  strike with cold, fire traps and lava burn.
- Set items: the ward-iron helm, mail and shield. Two pieces on one member give +2 AC, three give +4, on top of
  their own armour.
- Uniques: Gravecleaver, the Lantern Shield (sees secret walls), Emberheart. Rare (weight 1), always +3, never
  cursed. They are not yet limited to one per run.

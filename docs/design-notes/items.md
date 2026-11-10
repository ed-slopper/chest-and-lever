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
  - Rings of teleportitis jump the party about at random (3% a round). When one fires it names its wearer and
    becomes known. Every teleport, from a trap, a scroll, a wand or a ring, says its cause and how far the party
    went, and the app logs it (ed-slopper: a teleport should never go unexplained).
- Picking up (ed-slopper, 2026-11-09): stepping onto a tile takes what lies there, gold always and the rest
  while the pack has room. Items the player dropped (`Item::dropped`) stay put when walked over; only Pick up
  (G) takes them, and then they are ordinary again. Items that fell because the pack was full are not
  "dropped" and are taken on the next visit. `sim.rs`: `auto_pick_up`; tested by
  `walking_onto_items_picks_them_up_but_not_dropped_ones`.
- Holding (R63, extended by ed-slopper, 2026-11-09): either hand can hold a potion, scroll or wand, picked in
  the slot picker; clicking it on the card uses it. A wand in the main hand takes the weapon's place (the
  weapon goes to the pack) and fires whenever that member attacks, alone (portrait) or with the party.
  Tested by `a_wand_in_the_main_hand_fires_on_attack`.
- The slot picker shows each item's rarity like the pack does: a tinted, glowing frame and the name in the
  tier's colour (`hud.rs`: `picker`).
- Bolts fly (ed-slopper, 2026-11-09): a wand or scroll of fire or lightning, and the Spark spell, put a bolt on
  the tile in front of the party. It then moves one tile for every 30 ticks of the party's time, the length of
  a step, like a monster of normal speed. It waits while the party turns, hits the first monster it reaches or
  that walks into it (lightning goes on through each, hitting each once), and is spent after 8 tiles or at a
  wall. The Fireball spell and arrows still strike at once. `sim.rs`: `bolt`, `advance_missiles`; tested by
  `a_fire_bolt_flies_a_tile_per_step`.

## After the design notes (R60)

- Harm has a kind: physical, fire, cold, poison, lightning. Wards (rings, an amulet, boots) give percent
  resistance; it stacks across everything one member wears, and past 100% the harm heals instead. Ring of fire
  ward (60%) plus Emberheart (120%) lets a member bathe in lava. Spiders and slimes poison, wisps and the Warden
  strike with cold, fire traps and lava burn.
- Set items: the ward-iron helm, mail and shield. Two pieces on one member give +2 AC, three give +4, on top of
  their own armour.
- Uniques: Gravecleaver, the Lantern Shield (sees secret walls), Emberheart. Rare (weight 1), always +3, never
  cursed. Each has one depth per run, picked from the seed, and appears there at most once (board row G16);
  monster drops and fountains never give one.

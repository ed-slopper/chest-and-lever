//! The party of four. REF R30: the player leads four characters, shown as portraits on the right.
//! REF R13: party-based. OURS (R71 is a gap): four classes, five attributes, levels from shared experience,
//! a shared pack, two rows (members 0 and 1 in front, 2 and 3 behind).

use crate::content::{Content, Effect, ItemKind, Slot, Stat};
use crate::dice::Dice;
use crate::item::Item;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Class {
    Fighter,
    Rogue,
    Priest,
    Mage,
}

impl Class {
    pub fn name(self) -> &'static str {
        match self {
            Class::Fighter => "Fighter",
            Class::Rogue => "Rogue",
            Class::Priest => "Priest",
            Class::Mage => "Mage",
        }
    }
    /// Hit die per level. OURS.
    pub fn hit_die(self) -> u16 {
        match self {
            Class::Fighter => 10,
            Class::Rogue => 8,
            Class::Priest => 8,
            Class::Mage => 6,
        }
    }
    /// Spell points per level. OURS.
    pub fn mana_per_level(self) -> i32 {
        match self {
            Class::Fighter | Class::Rogue => 0,
            Class::Priest => 4,
            Class::Mage => 5,
        }
    }
    /// Bonus to hit with weapons. OURS: fighters are trained.
    pub fn weapon_skill(self) -> i32 {
        match self {
            Class::Fighter => 2,
            Class::Rogue => 1,
            _ => 0,
        }
    }
}

/// Spells (board row G10). OURS: R78 is a gap; R36's time control is unconfirmed, so it is not here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Spell {
    /// Mage: a bolt at the first monster in line.
    Spark,
    /// Mage: fire on the first monster in line and everything next to it.
    Fireball,
    /// Mage: the party is hasted for a while.
    Quicken,
    /// Priest: heal the most hurt member.
    Mend,
    /// Priest: cure poison on everyone.
    Purify,
    /// Priest: burn every undead within three tiles.
    Rebuke,
}

impl Spell {
    pub const ALL: [Spell; 6] = [Spell::Spark, Spell::Fireball, Spell::Quicken, Spell::Mend, Spell::Purify, Spell::Rebuke];
    pub fn class(self) -> Class {
        match self {
            Spell::Spark | Spell::Fireball | Spell::Quicken => Class::Mage,
            _ => Class::Priest,
        }
    }
    pub fn cost(self) -> i32 {
        match self {
            Spell::Spark => 2,
            Spell::Fireball => 6,
            Spell::Quicken => 8,
            Spell::Mend => 3,
            Spell::Purify => 3,
            Spell::Rebuke => 5,
        }
    }
    pub fn min_level(self) -> u8 {
        match self {
            Spell::Spark | Spell::Mend => 1,
            Spell::Purify => 2,
            Spell::Fireball | Spell::Rebuke => 3,
            Spell::Quicken => 5,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Spell::Spark => "Spark",
            Spell::Fireball => "Fireball",
            Spell::Quicken => "Quicken",
            Spell::Mend => "Mend",
            Spell::Purify => "Purify",
            Spell::Rebuke => "Rebuke",
        }
    }
    /// The spells of a class, in hotkey order.
    pub fn of(class: Class) -> Vec<Spell> {
        Spell::ALL.into_iter().filter(|s| s.class() == class).collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Attr {
    Str = 0,
    Dex = 1,
    Con = 2,
    Int = 3,
    Wis = 4,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Character {
    pub name: String,
    pub class: Class,
    /// STR, DEX, CON, INT, WIS.
    pub attrs: [i8; 5],
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub level: u8,
    pub xp: u32,
    #[serde(with = "crate::world::pairs")]
    pub equip: BTreeMap<Slot, Item>,
    /// Ticks of poison left.
    pub poison: u16,
    /// Which portrait the app draws.
    pub portrait: u32,
}

/// The modifier of an attribute score, as in D&D: (score - 10) / 2, rounded down.
pub fn modifier(score: i8) -> i32 {
    (score as i32 - 10).div_euclid(2)
}

/// Experience needed to reach `level + 1`. OURS: doubles each level (15, 30, 60, ...; tuned with the bot, G14).
pub fn xp_for_next(level: u8) -> u32 {
    15 * (1u32 << (level.saturating_sub(1)).min(20))
}

impl Character {
    pub fn alive(&self) -> bool {
        self.hp > 0
    }
    pub fn attr(&self, a: Attr) -> i8 {
        self.attrs[a as usize]
    }
    pub fn mod_of(&self, a: Attr) -> i32 {
        modifier(self.attr(a))
    }

    pub fn has_effect(&self, content: &Content, e: &Effect) -> bool {
        self.equip.values().any(|it| match &content.item(&it.def).kind {
            ItemKind::Ring { effect } | ItemKind::Amulet { effect } => effect == e,
            ItemKind::Armor { effect: Some(effect), .. } => effect == e,
            _ => false,
        })
    }

    /// Armour class, ascending: 10 + DEX modifier + armour + enchantments + rings of protection.
    pub fn ac(&self, content: &Content) -> i32 {
        let mut ac = 10 + self.mod_of(Attr::Dex);
        for it in self.equip.values() {
            match &content.item(&it.def).kind {
                ItemKind::Armor { ac: a, effect, .. } => {
                    ac += *a as i32 + it.enchant as i32;
                    if let Some(Effect::Protection(p)) = effect {
                        ac += *p as i32;
                    }
                }
                ItemKind::Ring { effect: Effect::Protection(p) } | ItemKind::Amulet { effect: Effect::Protection(p) } => {
                    ac += *p as i32 + it.enchant as i32
                }
                _ => {}
            }
        }
        ac
    }

    pub fn weapon(&self) -> Option<&Item> {
        self.equip.get(&Slot::Weapon)
    }

    /// (to-hit bonus, damage dice, damage bonus, reach, ranged) of the current weapon, or of bare fists.
    pub fn attack_profile(&self, content: &Content) -> (i32, Dice, i32, bool, bool) {
        let lvl = (self.level as i32) / 2;
        match self.weapon().map(|w| (w, &content.item(&w.def).kind)) {
            Some((w, ItemKind::Weapon { damage, reach, ranged, stat, .. })) => {
                let m = match stat {
                    Stat::Str => self.mod_of(Attr::Str),
                    Stat::Dex => self.mod_of(Attr::Dex),
                };
                let e = w.enchant as i32;
                let dmg_bonus = if *ranged { e } else { self.mod_of(Attr::Str) + e };
                (lvl + m + e + self.class.weapon_skill(), *damage, dmg_bonus, *reach, *ranged)
            }
            _ => (lvl + self.mod_of(Attr::Str), Dice::new(1, 2, 0), self.mod_of(Attr::Str), false, false),
        }
    }
}

/// The four starting characters. OURS (board row C6): our own names, not the record's.
pub fn starting_party() -> Vec<Character> {
    let mk = |name: &str, class: Class, attrs: [i8; 5], portrait: u32, gear: &[&str]| {
        let con = modifier(attrs[Attr::Con as usize]);
        let max_hp = class.hit_die() as i32 + con + 4;
        let caster_attr = match class {
            Class::Mage => modifier(attrs[Attr::Int as usize]),
            Class::Priest => modifier(attrs[Attr::Wis as usize]),
            _ => 0,
        };
        let max_mp = if class.mana_per_level() > 0 { class.mana_per_level() + 2 + caster_attr.max(0) * 2 } else { 0 };
        let mut equip = BTreeMap::new();
        for g in gear {
            let slot = match *g {
                "longsword" | "short_sword" | "spear" | "sling" => Slot::Weapon,
                "chain_mail" | "leather_armor" | "robe" => Slot::Body,
                "buckler" => Slot::Shield,
                _ => continue,
            };
            equip.insert(slot, Item::new(g));
        }
        Character {
            name: name.to_string(),
            class,
            attrs,
            hp: max_hp,
            max_hp,
            mp: max_mp,
            max_mp,
            level: 1,
            xp: 0,
            equip,
            poison: 0,
            portrait,
        }
    };
    vec![
        mk("Brann", Class::Fighter, [16, 12, 15, 9, 10], 0, &["longsword", "chain_mail", "buckler"]),
        mk("Sel", Class::Rogue, [11, 17, 12, 13, 10], 1, &["short_sword", "leather_armor"]),
        mk("Odile", Class::Priest, [12, 10, 13, 11, 16], 2, &["spear", "leather_armor"]),
        mk("Quill", Class::Mage, [8, 13, 11, 17, 12], 3, &["sling", "robe"]),
    ]
}

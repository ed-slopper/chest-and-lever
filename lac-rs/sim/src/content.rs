//! Content as data: items, monsters, loot and prefab rooms, read from RON (board row E10).
//!
//! The app loads the same files as Bevy assets (with hot reload in debug builds) and hands them here as text;
//! tests and replays use the copies built into the crate, so a recording always replays against the content
//! it was made with.

use crate::dice::Dice;
use crate::world::KeyKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What a potion, scroll, wand, ring or amulet does. OURS: R12 asks for cursed items and "ridiculously
/// broken" combinations; R74 is a gap. This list is our design.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    Heal(Dice),
    Mana(Dice),
    Cure,
    Poison(Dice),
    /// +1 strength for good.
    Strength,
    /// The party acts twice for each monster turn, for this many ticks.
    Haste(u16),
    /// No falling, for this many ticks (or while worn).
    Levitate(u16),
    Identify,
    RemoveCurse,
    Teleport,
    /// The whole level appears on the automap.
    Mapping,
    EnchantWeapon,
    EnchantArmor,
    /// A bolt of fire along the line the party faces.
    Fire(Dice),
    /// A bolt of lightning that passes through every monster on the line.
    Lightning(Dice),
    /// Rock in front of the party crumbles.
    Dig,
    /// Monsters in front fall asleep.
    Sleep,
    /// Armour class bonus while worn.
    Protection(i8),
    /// Wearer heals over time.
    Regeneration,
    /// Cursed rings: random teleports.
    Teleportitis,
    /// Cursed: every monster always knows where you are.
    Aggravate,
    /// Boots: jump two tiles up and long-jump two-tile gaps.
    Jumping,
    /// Secret walls show themselves when the party stands next to them.
    SeeSecrets,
    /// No effect at all (a joke the dungeon plays).
    Nothing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Slot {
    Weapon,
    Shield,
    Body,
    Head,
    Feet,
    Ring,
    Amulet,
}

impl Slot {
    pub const ALL: [Slot; 7] = [Slot::Weapon, Slot::Shield, Slot::Body, Slot::Head, Slot::Feet, Slot::Ring, Slot::Amulet];
    pub fn name(self) -> &'static str {
        match self {
            Slot::Weapon => "weapon",
            Slot::Shield => "shield",
            Slot::Body => "body",
            Slot::Head => "head",
            Slot::Feet => "feet",
            Slot::Ring => "ring",
            Slot::Amulet => "amulet",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stat {
    Str,
    Dex,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemKind {
    Weapon {
        damage: Dice,
        /// Usable from the back row against an adjacent foe.
        #[serde(default)]
        reach: bool,
        /// Hits the first monster in line, up to 6 tiles, from either row.
        #[serde(default)]
        ranged: bool,
        #[serde(default)]
        two_handed: bool,
        stat: Stat,
    },
    Armor {
        slot: Slot,
        ac: i8,
        #[serde(default)]
        effect: Option<Effect>,
    },
    Ring { effect: Effect },
    Amulet { effect: Effect },
    Potion { effect: Effect },
    Scroll { effect: Effect },
    Wand { effect: Effect, charges: Dice },
    Key { key: KeyKind },
    Gold { amount: Dice },
    /// The prize in the great chest.
    Crown,
}

impl ItemKind {
    pub fn category(&self) -> &'static str {
        match self {
            ItemKind::Weapon { .. } => "weapon",
            ItemKind::Armor { .. } => "armor",
            ItemKind::Ring { .. } => "ring",
            ItemKind::Amulet { .. } => "amulet",
            ItemKind::Potion { .. } => "potion",
            ItemKind::Scroll { .. } => "scroll",
            ItemKind::Wand { .. } => "wand",
            ItemKind::Key { .. } => "key",
            ItemKind::Gold { .. } => "gold",
            ItemKind::Crown => "crown",
        }
    }
    /// Kinds whose identity is hidden behind a random appearance until known (NetHack-style, R14).
    pub fn has_appearance(&self) -> bool {
        matches!(
            self,
            ItemKind::Potion { .. } | ItemKind::Scroll { .. } | ItemKind::Wand { .. } | ItemKind::Ring { .. } | ItemKind::Amulet { .. }
        )
    }
    pub fn slot(&self) -> Option<Slot> {
        match self {
            ItemKind::Weapon { .. } => Some(Slot::Weapon),
            ItemKind::Armor { slot, .. } => Some(*slot),
            ItemKind::Ring { .. } => Some(Slot::Ring),
            ItemKind::Amulet { .. } => Some(Slot::Amulet),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemDef {
    pub name: String,
    pub kind: ItemKind,
    /// Depths it can be found at, inclusive.
    pub depth: (u8, u8),
    /// How often, against others of its category.
    pub weight: u32,
    /// Percent of the time it comes cursed (with a negative enchantment, if it has one).
    #[serde(default)]
    pub cursed: u32,
    /// Which icon the app draws (`sword`, `potion`, ...).
    pub icon: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnHit {
    Poison,
    DrainMana,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonsterDef {
    pub name: String,
    pub hp: Dice,
    pub ac: i8,
    pub to_hit: i8,
    pub damage: Dice,
    /// Energy per tick; a monster acts each time it has 12. OURS: 12 = once per tick.
    pub speed: u8,
    /// How far it notices the party, in tiles.
    pub sight: u8,
    pub xp: u32,
    pub depth: (u8, u8),
    pub weight: u32,
    #[serde(default)]
    pub flies: bool,
    /// Can step up one tile without jumping (spiders).
    #[serde(default)]
    pub climbs: bool,
    #[serde(default)]
    pub undead: bool,
    /// Replaced in the spider-free mode (R43, board row X4).
    #[serde(default)]
    pub spider: bool,
    /// Shoots along straight lines up to this many tiles.
    #[serde(default)]
    pub ranged: Option<u8>,
    #[serde(default)]
    pub on_hit: Option<OnHit>,
    /// Never spawned at random (mimics, bosses).
    #[serde(default)]
    pub special: bool,
    /// The voxel model the app builds for it.
    pub model: String,
}

/// A loot table: categories by weight.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LootTable {
    pub rolls: Dice,
    pub entries: Vec<(String, u32)>,
}

/// A hand-built room, "made with procedural generation in mind" (R45), stamped into generated levels.
///
/// `layers` go bottom to top; layer 0 is the floor layer (the cells under the party's feet). Characters:
/// `#` wall, `=` floor, `.` air, ` ` leave the room as generated, `w` water, `l` lava, `T` tomb, `C` chest,
/// `c` locked chest, `M` mimic, `F` fountain, `D` door, `P` portcullis (opened by the prefab's levers),
/// `L` lever (a wall cell; it faces the nearest open side), `S` secret wall, `B` bridge cell (air until a
/// lever raises it), `^` spikes, `g` a monster for the depth, `G` a wisp, `k` an iron key, `X` the great chest,
/// `W` the warden, `t` a torch (a wall cell), `i` an item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Prefab {
    pub name: String,
    pub depth: (u8, u8),
    pub weight: u32,
    pub layers: Vec<Vec<String>>,
    /// Only placed where the generator asks for it by name (the warden's vault).
    #[serde(default)]
    pub special: bool,
}

impl Prefab {
    /// (width in x, height in y, depth in z).
    pub fn size(&self) -> (i32, i32, i32) {
        let h = self.layers.len() as i32;
        let d = self.layers.first().map_or(0, |l| l.len()) as i32;
        let w = self
            .layers
            .iter()
            .flat_map(|l| l.iter())
            .map(|r| r.chars().count())
            .max()
            .unwrap_or(0) as i32;
        (w, h, d)
    }
    pub fn at(&self, x: i32, y: i32, z: i32) -> char {
        self.layers
            .get(y as usize)
            .and_then(|l| l.get(z as usize))
            .and_then(|r| r.chars().nth(x as usize))
            .unwrap_or(' ')
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Appearances {
    pub potion: Vec<String>,
    pub scroll: Vec<String>,
    pub wand: Vec<String>,
    pub ring: Vec<String>,
    pub amulet: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Content {
    pub items: BTreeMap<String, ItemDef>,
    pub monsters: BTreeMap<String, MonsterDef>,
    pub loot: BTreeMap<String, LootTable>,
    pub prefabs: Vec<Prefab>,
    pub appearances: Appearances,
}

/// The content files, one per kind, by file name.
pub const FILES: [&str; 5] = ["items.ron", "monsters.ron", "loot.ron", "prefabs.ron", "appearances.ron"];

impl Content {
    /// The content built into the crate (the same files the app ships in `assets/content/`).
    pub fn builtin() -> Content {
        Content::from_texts(&[
            include_str!("../../assets/content/items.ron"),
            include_str!("../../assets/content/monsters.ron"),
            include_str!("../../assets/content/loot.ron"),
            include_str!("../../assets/content/prefabs.ron"),
            include_str!("../../assets/content/appearances.ron"),
        ])
        .expect("built-in content parses")
    }

    /// Parse the five files, in the order of [`FILES`].
    pub fn from_texts(texts: &[&str; 5]) -> Result<Content, String> {
        let at = |i: usize, e: ron::error::SpannedError| format!("{}: {e}", FILES[i]);
        let c = Content {
            items: ron::from_str(texts[0]).map_err(|e| at(0, e))?,
            monsters: ron::from_str(texts[1]).map_err(|e| at(1, e))?,
            loot: ron::from_str(texts[2]).map_err(|e| at(2, e))?,
            prefabs: ron::from_str(texts[3]).map_err(|e| at(3, e))?,
            appearances: ron::from_str(texts[4]).map_err(|e| at(4, e))?,
        };
        c.check()?;
        Ok(c)
    }

    /// Cross-checks: every name a file uses exists.
    pub fn check(&self) -> Result<(), String> {
        for (id, t) in &self.loot {
            for (cat, _) in &t.entries {
                let known = cat.starts_with('#') && self.items.contains_key(&cat[1..])
                    || self.items.values().any(|d| d.kind.category() == cat);
                if !known {
                    return Err(format!("loot.ron: table {id} names {cat}, which no item has"));
                }
            }
        }
        for need in ["chest", "floor"] {
            if !self.loot.contains_key(need) {
                return Err(format!("loot.ron: no table {need}"));
            }
        }
        for need in ["mimic", "warden", "wisp"] {
            if !self.monsters.contains_key(need) {
                return Err(format!("monsters.ron: no monster {need}"));
            }
        }
        for need in ["iron_key", "crown"] {
            if !self.items.contains_key(need) {
                return Err(format!("items.ron: no item {need}"));
            }
        }
        let count = |f: fn(&ItemKind) -> bool| self.items.values().filter(|d| f(&d.kind)).count();
        let a = &self.appearances;
        for (name, have, need) in [
            ("potion", a.potion.len(), count(|k| matches!(k, ItemKind::Potion { .. }))),
            ("scroll", a.scroll.len(), count(|k| matches!(k, ItemKind::Scroll { .. }))),
            ("wand", a.wand.len(), count(|k| matches!(k, ItemKind::Wand { .. }))),
            ("ring", a.ring.len(), count(|k| matches!(k, ItemKind::Ring { .. }))),
            ("amulet", a.amulet.len(), count(|k| matches!(k, ItemKind::Amulet { .. }))),
        ] {
            if have < need {
                return Err(format!("appearances.ron: {have} {name} looks for {need} {name}s"));
            }
        }
        for p in &self.prefabs {
            if p.layers.is_empty() {
                return Err(format!("prefabs.ron: {} has no layers", p.name));
            }
            let d = p.layers[0].len();
            if p.layers.iter().any(|l| l.len() != d) {
                return Err(format!("prefabs.ron: {} has layers of different depth", p.name));
            }
        }
        Ok(())
    }

    pub fn item(&self, id: &str) -> &ItemDef {
        self.items.get(id).unwrap_or_else(|| panic!("no item {id}"))
    }

    pub fn monster(&self, id: &str) -> &MonsterDef {
        self.monsters.get(id).unwrap_or_else(|| panic!("no monster {id}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_content_parses_and_checks() {
        let c = Content::builtin();
        assert!(c.items.len() >= 30);
        assert!(c.monsters.len() >= 8);
        assert!(!c.prefabs.is_empty());
    }
}

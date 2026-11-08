//! Item instances, their names, and what the party knows about them.
//!
//! REF R12: cursed items, as in an old-school roguelike. OURS (R74 is a gap): potions, scrolls, wands, rings
//! and amulets hide behind a random look until one of their kind is used or identified; weapons and armour
//! hide their enchantment and curse until worn or identified; a cursed item that is worn can't come off.

use crate::content::{Content, ItemKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    /// The id in `items.ron`.
    pub def: String,
    #[serde(default)]
    pub enchant: i8,
    #[serde(default)]
    pub cursed: bool,
    /// Enchantment and curse known.
    #[serde(default)]
    pub known: bool,
    #[serde(default)]
    pub charges: i16,
    /// Gold amount, for gold.
    #[serde(default)]
    pub amount: u32,
}

impl Item {
    pub fn new(def: &str) -> Item {
        Item { def: def.to_string(), enchant: 0, cursed: false, known: true, charges: 0, amount: 0 }
    }
}

/// What the party knows this run: which looks are which, and which kinds are identified.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Knowledge {
    /// Item id to the look it has this run.
    #[serde(with = "crate::world::pairs")]
    pub looks: BTreeMap<String, String>,
    pub identified: BTreeSet<String>,
}

impl Knowledge {
    /// Deal the run's looks out from the `loot` stream.
    pub fn deal(content: &Content, r: &mut crate::rng::Mt19937) -> Knowledge {
        let mut k = Knowledge::default();
        let a = &content.appearances;
        for (cat, pool) in [
            ("potion", &a.potion),
            ("scroll", &a.scroll),
            ("wand", &a.wand),
            ("ring", &a.ring),
            ("amulet", &a.amulet),
        ] {
            let mut pool = pool.clone();
            r.shuffle(&mut pool);
            let ids: Vec<&String> = content.items.iter().filter(|(_, d)| d.kind.category() == cat).map(|(id, _)| id).collect();
            for (id, look) in ids.into_iter().zip(pool) {
                k.looks.insert(id.clone(), look);
            }
        }
        k
    }

    pub fn kind_known(&self, content: &Content, def: &str) -> bool {
        !content.item(def).kind.has_appearance() || self.identified.contains(def)
    }

    /// The name the party sees.
    pub fn name(&self, content: &Content, item: &Item) -> String {
        let d = content.item(&item.def);
        let base = if self.kind_known(content, &item.def) {
            d.name.clone()
        } else {
            let look = self.looks.get(&item.def).map(String::as_str).unwrap_or("strange");
            match &d.kind {
                ItemKind::Potion { .. } => format!("{look} potion"),
                ItemKind::Scroll { .. } => format!("scroll labelled {look}"),
                ItemKind::Wand { .. } => format!("{look} wand"),
                ItemKind::Ring { .. } => format!("{look} ring"),
                ItemKind::Amulet { .. } => format!("{look} amulet"),
                _ => d.name.clone(),
            }
        };
        match &d.kind {
            ItemKind::Gold { .. } => format!("{} gold", item.amount),
            ItemKind::Weapon { .. } | ItemKind::Armor { .. } if item.known => {
                let curse = if item.cursed { "cursed " } else { "" };
                format!("{curse}{:+} {base}", item.enchant)
            }
            ItemKind::Wand { .. } if self.kind_known(content, &item.def) && item.known => format!("{base} ({})", item.charges),
            ItemKind::Ring { .. } | ItemKind::Amulet { .. } if item.known && item.cursed => format!("cursed {base}"),
            _ => base,
        }
    }
}

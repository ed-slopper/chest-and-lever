//! The grid world: cells in 16³ chunks, the things that stand on tiles, and the level that holds them.
//!
//! REF R20: the world is a grid of tiles. REF R23: collision comes from the level data itself, so walls
//! and floors are cells, not separate colliders. REF R40: the dungeon is voxels, so a tile is a cell.

use crate::geom::{Dir, IVec3};
use crate::item::Item;
use crate::monster::Monster;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const CHUNK: i32 = 16;
const CHUNK_CELLS: usize = (CHUNK * CHUNK * CHUNK) as usize;

/// What fills one cell. OURS: the kinds of stone and their names.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Cell {
    #[default]
    Air = 0,
    Water,
    Lava,
    Bedrock,
    Stone,
    Brick,
    Cobble,
    Dirt,
    Rock,
    Moss,
    Wood,
    Marble,
    Bone,
    Obsidian,
    Crystal,
}

impl Cell {
    pub const COUNT: usize = 15;
    pub const ALL: [Cell; Cell::COUNT] = [
        Cell::Air,
        Cell::Water,
        Cell::Lava,
        Cell::Bedrock,
        Cell::Stone,
        Cell::Brick,
        Cell::Cobble,
        Cell::Dirt,
        Cell::Rock,
        Cell::Moss,
        Cell::Wood,
        Cell::Marble,
        Cell::Bone,
        Cell::Obsidian,
        Cell::Crystal,
    ];
    pub fn from_u8(v: u8) -> Cell {
        Cell::ALL.get(v as usize).copied().unwrap_or(Cell::Bedrock)
    }
    /// Blocks movement and holds up whoever stands on it.
    pub fn is_solid(self) -> bool {
        !matches!(self, Cell::Air | Cell::Water | Cell::Lava)
    }
    pub fn is_liquid(self) -> bool {
        matches!(self, Cell::Water | Cell::Lava)
    }
    pub fn name(self) -> &'static str {
        match self {
            Cell::Air => "air",
            Cell::Water => "water",
            Cell::Lava => "lava",
            Cell::Bedrock => "bedrock",
            Cell::Stone => "stone",
            Cell::Brick => "brick",
            Cell::Cobble => "cobblestone",
            Cell::Dirt => "dirt",
            Cell::Rock => "rock",
            Cell::Moss => "moss",
            Cell::Wood => "planks",
            Cell::Marble => "marble",
            Cell::Bone => "bone",
            Cell::Obsidian => "obsidian",
            Cell::Crystal => "crystal",
        }
    }
}

/// 16³ cells, saved run-length encoded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chunk {
    #[serde(with = "rle")]
    cells: Vec<u8>,
}

impl Chunk {
    pub fn filled(c: Cell) -> Self {
        Chunk { cells: vec![c as u8; CHUNK_CELLS] }
    }
    fn index(local: IVec3) -> usize {
        (local.x + CHUNK * (local.z + CHUNK * local.y)) as usize
    }
    pub fn get(&self, local: IVec3) -> Cell {
        Cell::from_u8(self.cells[Chunk::index(local)])
    }
    pub fn set(&mut self, local: IVec3, c: Cell) {
        self.cells[Chunk::index(local)] = c as u8;
    }
}

mod rle {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(cells: &[u8], s: S) -> Result<S::Ok, S::Error> {
        let mut runs: Vec<(u16, u8)> = Vec::new();
        for &c in cells {
            match runs.last_mut() {
                Some((n, v)) if *v == c && *n < u16::MAX => *n += 1,
                _ => runs.push((1, c)),
            }
        }
        runs.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let runs: Vec<(u16, u8)> = Vec::deserialize(d)?;
        let mut out = Vec::with_capacity(super::CHUNK_CELLS);
        for (n, v) in runs {
            out.extend(std::iter::repeat_n(v, n as usize));
        }
        if out.len() != super::CHUNK_CELLS {
            return Err(serde::de::Error::custom(format!("chunk has {} cells", out.len())));
        }
        Ok(out)
    }
}

/// Serialise a map with non-string keys as a list of pairs (keeps RON simple and the order fixed).
pub mod pairs {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;

    pub fn serialize<S, K, V>(m: &BTreeMap<K, V>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        K: Serialize + Ord,
        V: Serialize,
    {
        let v: Vec<(&K, &V)> = m.iter().collect();
        v.serialize(s)
    }

    pub fn deserialize<'de, D, K, V>(d: D) -> Result<BTreeMap<K, V>, D::Error>
    where
        D: Deserializer<'de>,
        K: Deserialize<'de> + Ord,
        V: Deserialize<'de>,
    {
        let v: Vec<(K, V)> = Vec::deserialize(d)?;
        Ok(v.into_iter().collect())
    }
}

/// The cells of one level. Anything outside `size` reads as bedrock, so nobody walks off the map.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct World {
    pub size: IVec3,
    #[serde(with = "pairs")]
    chunks: BTreeMap<IVec3, Chunk>,
}

impl World {
    /// A world of `size` filled with `fill`.
    pub fn new(size: IVec3, fill: Cell) -> Self {
        let mut chunks = BTreeMap::new();
        for cy in 0..(size.y + CHUNK - 1) / CHUNK {
            for cz in 0..(size.z + CHUNK - 1) / CHUNK {
                for cx in 0..(size.x + CHUNK - 1) / CHUNK {
                    chunks.insert(IVec3::new(cx, cy, cz), Chunk::filled(fill));
                }
            }
        }
        World { size, chunks }
    }

    pub fn in_bounds(&self, p: IVec3) -> bool {
        p.x >= 0 && p.y >= 0 && p.z >= 0 && p.x < self.size.x && p.y < self.size.y && p.z < self.size.z
    }

    pub fn chunk_of(p: IVec3) -> IVec3 {
        IVec3::new(p.x.div_euclid(CHUNK), p.y.div_euclid(CHUNK), p.z.div_euclid(CHUNK))
    }

    fn local(p: IVec3) -> IVec3 {
        IVec3::new(p.x.rem_euclid(CHUNK), p.y.rem_euclid(CHUNK), p.z.rem_euclid(CHUNK))
    }

    pub fn get(&self, p: IVec3) -> Cell {
        if !self.in_bounds(p) {
            return Cell::Bedrock;
        }
        match self.chunks.get(&World::chunk_of(p)) {
            Some(c) => c.get(World::local(p)),
            None => Cell::Bedrock,
        }
    }

    /// Set a cell; returns false outside the world.
    pub fn set(&mut self, p: IVec3, c: Cell) -> bool {
        if !self.in_bounds(p) {
            return false;
        }
        match self.chunks.get_mut(&World::chunk_of(p)) {
            Some(ch) => {
                ch.set(World::local(p), c);
                true
            }
            None => false,
        }
    }

    pub fn solid(&self, p: IVec3) -> bool {
        self.get(p).is_solid()
    }

    pub fn chunk_keys(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.chunks.keys().copied()
    }
}

/// Locks and the keys that open them. OURS.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum KeyKind {
    Iron,
    Bronze,
    Bone,
}

impl KeyKind {
    pub fn name(self) -> &'static str {
        match self {
            KeyKind::Iron => "iron",
            KeyKind::Bronze => "bronze",
            KeyKind::Bone => "bone",
        }
    }
}

/// What a chest does to whoever opens it carelessly. OURS: R73 is a gap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Trap {
    /// A needle: one character is poisoned.
    Needle,
    /// A cloud of gas: everyone is poisoned a little.
    Gas,
    /// A burst of flame: fire damage to everyone.
    Fire,
    /// A bell: every monster on the level wakes.
    Alarm,
    /// A glyph: the party is thrown somewhere else on the level.
    Teleport,
}

/// Things standing on (or in) a tile. OURS: R72 (levers) and R73 (chests) are gaps; this is our design.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Feature {
    /// A door in an air cell. Portcullises open only by lever.
    Door { open: bool, lock: Option<KeyKind>, portcullis: bool },
    /// A lever set into a wall cell, facing out of the wall towards `face`. Flipping it fires its link.
    Lever { on: bool, face: Dir, link: u32 },
    /// A plate in the floor: stepping on fires its link once.
    Plate { pressed: bool, link: u32 },
    /// A chest in an air cell; it blocks the tile until opened, and still after.
    Chest {
        open: bool,
        lock: Option<KeyKind>,
        trap: Option<Trap>,
        mimic: bool,
        items: Vec<Item>,
        facing: Dir,
        /// The great chest at the bottom of the dungeon: opening it wins the run.
        great: bool,
    },
    /// Stairs down; stepping onto them takes the party a level deeper.
    Stairs { facing: Dir },
    /// Spikes at the bottom of a trench: entering the tile hurts.
    Spikes,
    /// A fountain: drinking does something random.
    Fountain { uses: u8 },
    /// A tombstone: blocks the tile.
    Tomb,
    /// A wall cell that is really a hidden passage: searching opens it.
    SecretWall,
    /// A torch on a wall cell, facing `face` (light only).
    Torch { face: Dir },
}

impl Feature {
    /// Does this feature stop anyone entering its tile?
    pub fn blocks(&self) -> bool {
        match self {
            Feature::Door { open, .. } => !open,
            Feature::Chest { .. } | Feature::Fountain { .. } | Feature::Tomb => true,
            _ => false,
        }
    }
}

/// What a link does when a lever is flipped or a plate pressed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkEffect {
    /// Open a closed door, close an open one.
    ToggleDoor(IVec3),
    /// Swap these cells between air and `cell`: raise or lower a bridge.
    ToggleBridge { cells: Vec<IVec3>, cell: Cell },
    /// Open a secret wall.
    Reveal(IVec3),
    /// Monsters of a kind appear at these tiles.
    Spawn { kind: String, at: Vec<IVec3> },
    /// These cells fill with water.
    Flood(Vec<IVec3>),
    /// Darts fly from the walls: damage to the party.
    Darts,
    /// Every monster on the level wakes.
    Wake,
}

/// One dungeon level: its cells, what stands on them, and who walks them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Level {
    pub depth: u8,
    pub theme: Theme,
    pub world: World,
    #[serde(with = "pairs")]
    pub features: BTreeMap<IVec3, Feature>,
    #[serde(with = "pairs")]
    pub links: BTreeMap<u32, Vec<LinkEffect>>,
    /// Items lying on tiles.
    #[serde(with = "pairs")]
    pub items: BTreeMap<IVec3, Vec<Item>>,
    pub monsters: Vec<Monster>,
    pub next_monster_id: u32,
    pub start: IVec3,
    pub start_facing: Dir,
    /// Columns (x, z) the party has seen, for the automap (R35).
    pub explored: BTreeSet<(i32, i32)>,
    /// The player's marks on the automap (R60), by column.
    #[serde(default, with = "pairs")]
    pub notes: BTreeMap<(i32, i32), u8>,
    /// Bolts in flight (wands, scrolls, Spark). Left out of saves while empty, so runs that fire none keep
    /// their old state hash.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missiles: Vec<Missile>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub next_missile_id: u32,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// A bolt flying down a line, a tile at a time (ed-slopper's ask, 2026-11-09).
/// OURS: it moves one tile per 30 ticks of the party's time (one step), like a monster of normal speed, so it
/// waits while the party turns or stands still; the record says nothing of projectiles.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Missile {
    pub id: u32,
    pub pos: IVec3,
    pub dir: Dir,
    pub dmg: i32,
    pub fire: bool,
    /// Lightning: passes through every monster, hitting each once.
    pub pierce: bool,
    /// Tiles it may still fly.
    pub left: u8,
    /// Ticks saved toward its next tile.
    pub energy: u32,
    /// Monsters it has hurt already (by id), so a piercing bolt hits each once.
    pub hit: Vec<u32>,
}

/// The look and contents of a stretch of depths. REF R42 (crypts, tombs), R46 (caves, unconfirmed); the rest
/// is ours.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Theme {
    Crypt,
    Caves,
    Tombs,
    Deep,
}

impl Theme {
    /// OURS: depths 1-2 crypt, 3-4 caves, 5-6 tombs, 7-8 the deep.
    pub fn for_depth(depth: u8) -> Theme {
        match depth {
            0..=2 => Theme::Crypt,
            3..=4 => Theme::Caves,
            5..=6 => Theme::Tombs,
            _ => Theme::Deep,
        }
    }
    pub fn wall(self) -> Cell {
        match self {
            Theme::Crypt => Cell::Brick,
            Theme::Caves => Cell::Rock,
            Theme::Tombs => Cell::Marble,
            Theme::Deep => Cell::Obsidian,
        }
    }
    pub fn floor(self) -> Cell {
        match self {
            Theme::Crypt => Cell::Cobble,
            Theme::Caves => Cell::Dirt,
            Theme::Tombs => Cell::Bone,
            Theme::Deep => Cell::Stone,
        }
    }
    pub fn accent(self) -> Cell {
        match self {
            Theme::Crypt => Cell::Stone,
            Theme::Caves => Cell::Moss,
            Theme::Tombs => Cell::Stone,
            Theme::Deep => Cell::Crystal,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Theme::Crypt => "the Crypt",
            Theme::Caves => "the Caves",
            Theme::Tombs => "the Tombs",
            Theme::Deep => "the Deep",
        }
    }
}

impl Level {
    pub fn empty(depth: u8, theme: Theme, size: IVec3, fill: Cell) -> Self {
        Level {
            depth,
            theme,
            world: World::new(size, fill),
            features: BTreeMap::new(),
            links: BTreeMap::new(),
            items: BTreeMap::new(),
            monsters: Vec::new(),
            next_monster_id: 1,
            start: IVec3::ZERO,
            start_facing: Dir::North,
            explored: BTreeSet::new(),
            notes: BTreeMap::new(),
            missiles: Vec::new(),
            next_missile_id: 0,
        }
    }

    pub fn monster_at(&self, p: IVec3) -> Option<usize> {
        self.monsters.iter().position(|m| m.pos == p && m.hp > 0)
    }

    /// Can anyone stand in this cell at all (not solid, nothing blocking)? The space over a closed door is shut
    /// too: a door fills its doorway, so nobody jumps over it.
    pub fn open(&self, p: IVec3) -> bool {
        !self.world.solid(p)
            && !self.features.get(&p).is_some_and(|f| f.blocks())
            && !matches!(self.features.get(&p.down()), Some(Feature::Door { open: false, .. }))
    }

    /// Is there something under this cell to stand on? OURS: chests, tombs and fountains hold you up too, so a
    /// long jump that comes down on one leaves the party standing on it.
    pub fn supported(&self, p: IVec3) -> bool {
        let d = p.down();
        self.world.solid(d) || self.features.get(&d).is_some_and(|f| f.blocks() && !matches!(f, Feature::Door { .. }))
    }

    /// The highest standing cell in column (x, z) at or below `from_y`, if any.
    pub fn ground_below(&self, x: i32, z: i32, from_y: i32) -> Option<i32> {
        let mut y = from_y;
        while y > 0 {
            let p = IVec3::new(x, y, z);
            if !self.world.solid(p) && self.world.solid(p.down()) {
                return Some(y);
            }
            y -= 1;
        }
        None
    }

    /// Save as RON. REF: board row E3.
    pub fn to_ron(&self) -> String {
        ron::ser::to_string(self).expect("a level always serialises")
    }

    pub fn from_ron(text: &str) -> Result<Level, String> {
        ron::from_str(text).map_err(|e| e.to_string())
    }

    /// A small level built from code, for tests: a 12×6×12 room of stone with a floor at y = 1, a ledge one
    /// tile high, a one-tile gap and a two-tile-high wall. REF R47: the first test level was for collision and
    /// jumping.
    pub fn test_level() -> Level {
        let size = IVec3::new(12, 6, 12);
        let mut l = Level::empty(1, Theme::Crypt, size, Cell::Stone);
        for x in 1..11 {
            for z in 1..11 {
                for y in 1..5 {
                    l.world.set(IVec3::new(x, y, z), Cell::Air);
                }
            }
        }
        // A ledge one high along z = 3, x 1..=10 (row in front of the start when facing north).
        for x in 1..11 {
            l.world.set(IVec3::new(x, 1, 3), Cell::Brick);
        }
        // A one-wide gap (pit, two deep) along z = 7.
        for x in 1..11 {
            l.world.set(IVec3::new(x, 0, 7), Cell::Air);
        }
        // A two-high wall at x = 9, z 8..=10.
        for z in 8..11 {
            l.world.set(IVec3::new(9, 1, z), Cell::Brick);
            l.world.set(IVec3::new(9, 2, z), Cell::Brick);
        }
        l.start = IVec3::new(5, 1, 5);
        l.start_facing = Dir::North;
        l
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_round_trip() {
        let mut w = World::new(IVec3::new(20, 20, 20), Cell::Stone);
        assert_eq!(w.get(IVec3::new(17, 3, 2)), Cell::Stone);
        w.set(IVec3::new(17, 3, 2), Cell::Water);
        assert_eq!(w.get(IVec3::new(17, 3, 2)), Cell::Water);
        assert_eq!(w.get(IVec3::new(-1, 3, 2)), Cell::Bedrock);
        assert_eq!(w.get(IVec3::new(20, 3, 2)), Cell::Bedrock);
    }

    #[test]
    fn level_saves_and_loads_as_ron() {
        let l = Level::test_level();
        let text = l.to_ron();
        let back = Level::from_ron(&text).unwrap();
        assert_eq!(l, back);
        assert!(text.len() < 4000, "run-length chunks keep levels small, got {}", text.len());
    }
}

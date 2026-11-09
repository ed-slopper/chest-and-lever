//! Level generation (board rows C1-C3). REF R44: levels are procedurally generated from the seed. REF R45:
//! some rooms are hand-built "with procedural generation in mind" and stamped in. REF R47: the first test
//! level was seeded chaos; ours is rooms and corridors with heights you jump between (R21, R22, R29).
//!
//! OURS: everything about the layout. Each depth draws only from its own stream (`levelgen.<depth>`), so a
//! seed always gives the same dungeon whatever happens on the way down. A solver walks the level with the
//! party's own moves (step, drop, jump-then-step, long jump) and places keys and levers only where the party
//! can already reach, so every level can be finished.

use crate::content::{Content, ItemKind};
use crate::geom::{Dir, IVec3};
use crate::item::Item;
use crate::monster::Monster;
use crate::rng::{self, Mt19937};
use crate::sim::{RunOptions, FINAL_DEPTH};
use crate::world::{Cell, Feature, KeyKind, Level, LinkEffect, Theme, Trap};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const SIZE: IVec3 = IVec3::new(48, 12, 48);

#[derive(Clone, Copy, Debug)]
struct Room {
    x0: i32,
    z0: i32,
    w: i32,
    d: i32,
    /// The y the party stands at.
    g: i32,
    /// Air cells above the floor.
    h: i32,
}

impl Room {
    fn contains(&self, x: i32, z: i32) -> bool {
        x >= self.x0 && x < self.x0 + self.w && z >= self.z0 && z < self.z0 + self.d
    }
    fn center(&self) -> (i32, i32) {
        (self.x0 + self.w / 2, self.z0 + self.d / 2)
    }
    fn overlaps(&self, o: &Room, margin: i32) -> bool {
        self.x0 - margin < o.x0 + o.w && o.x0 - margin < self.x0 + self.w && self.z0 - margin < o.z0 + o.d && o.z0 - margin < self.z0 + self.d
    }
}

/// Something that bars the way until a key or a lever deals with it.
#[derive(Clone, Debug)]
enum Gate {
    Locked { at: IVec3, key: KeyKind },
    Portcullis { at: IVec3 },
    Bridge { cells: Vec<IVec3> },
}

struct Gen<'a> {
    c: &'a Content,
    r: Mt19937,
    l: Level,
    rooms: Vec<Room>,
    gates: Vec<Gate>,
    /// Gates already given their key or lever; the solver treats them as passable.
    solved: Vec<bool>,
    /// Bridge cells the solver treats as raised.
    bridges_up: BTreeSet<IVec3>,
    /// Corridor cells (doors and plates go here).
    corridor: BTreeSet<(i32, i32)>,
    options: RunOptions,
    next_link: u32,
    seed: u64,
    /// Uniques already placed on this level.
    placed: BTreeSet<String>,
}

/// Make the level at `depth` of the run with this seed.
pub fn generate(c: &Content, seed: u64, depth: u8, options: RunOptions) -> Level {
    let mut r = rng::stream(seed, &format!("levelgen.{depth}"));
    let theme = Theme::for_depth(depth);
    for attempt in 0..40 {
        let sub = Mt19937::from_key(&[r.next_u32(), r.next_u32(), attempt]);
        let built = if theme == Theme::Caves && attempt < 20 { caves(c, sub, depth, options, seed) } else { rooms(c, sub, depth, options, seed) };
        if let Some(l) = built {
            return l;
        }
    }
    // never seen in tests; keep the run going with the code-built level
    let mut l = Level::test_level();
    l.depth = depth;
    l
}

impl<'a> Gen<'a> {
    fn new(c: &'a Content, r: Mt19937, depth: u8, options: RunOptions, seed: u64) -> Self {
        let theme = Theme::for_depth(depth);
        let mut l = Level::empty(depth, theme, SIZE, theme.wall());
        for x in 0..SIZE.x {
            for z in 0..SIZE.z {
                l.world.set(IVec3::new(x, 0, z), Cell::Bedrock);
            }
        }
        Gen { c, r, l, rooms: Vec::new(), gates: Vec::new(), solved: Vec::new(), bridges_up: BTreeSet::new(), corridor: BTreeSet::new(), options, next_link: 1, seed, placed: BTreeSet::new() }
    }

    /// Roll a loot table for this level; uniques may come only on their own depth, once (board row G16).
    fn loot(&mut self, table: &str, depth: u8) -> Vec<Item> {
        let mut rule = UniqueRule { seed: self.seed, placed: &mut self.placed };
        roll_loot(self.c, table, depth, &mut self.r, Some(&mut rule))
    }

    fn theme(&self) -> Theme {
        self.l.theme
    }

    fn carve_column(&mut self, x: i32, z: i32, g: i32, h: i32) {
        let floor = self.theme().floor();
        self.l.world.set(IVec3::new(x, g - 1, z), floor);
        for y in g..g + h {
            self.l.world.set(IVec3::new(x, y, z), Cell::Air);
        }
    }

    fn room_at(&self, x: i32, z: i32) -> Option<usize> {
        self.rooms.iter().position(|r| r.contains(x, z))
    }

    /// `big`: the second room is an 11×11 hall (the warden's, on the last level).
    fn place_rooms(&mut self, want: usize, base: i32, big: bool) {
        for _ in 0..300 {
            if self.rooms.len() >= want {
                break;
            }
            let hall = big && self.rooms.len() == 1;
            let (w, d) = if hall { (11, 11) } else { (self.r.range(5, 11), self.r.range(5, 11)) };
            let room = Room {
                x0: self.r.range(2, SIZE.x - w - 2),
                z0: self.r.range(2, SIZE.z - d - 2),
                w,
                d,
                g: base + self.r.range(-1, 1),
                h: if hall { 5 } else { self.r.range(4, 6) },
            };
            if self.rooms.iter().any(|o| o.overlaps(&room, 2)) {
                continue;
            }
            self.rooms.push(room);
        }
        for i in 0..self.rooms.len() {
            let rm = self.rooms[i];
            for x in rm.x0..rm.x0 + rm.w {
                for z in rm.z0..rm.z0 + rm.d {
                    self.carve_column(x, z, rm.g, rm.h);
                }
            }
        }
    }

    /// Connect rooms: a spanning tree by distance, plus a couple of loops.
    fn connect(&mut self) {
        let n = self.rooms.len();
        let dist = |a: &Room, b: &Room| {
            let (ax, az) = a.center();
            let (bx, bz) = b.center();
            (ax - bx).abs() + (az - bz).abs()
        };
        let mut inside = vec![false; n];
        inside[0] = true;
        let mut edges = Vec::new();
        for _ in 1..n {
            let mut best: Option<(i32, usize, usize)> = None;
            for i in 0..n {
                if !inside[i] {
                    continue;
                }
                for (j, &taken) in inside.iter().enumerate() {
                    if taken {
                        continue;
                    }
                    let dd = dist(&self.rooms[i], &self.rooms[j]);
                    if best.is_none_or(|b| dd < b.0) {
                        best = Some((dd, i, j));
                    }
                }
            }
            let (_, i, j) = best.unwrap();
            inside[j] = true;
            edges.push((i, j));
        }
        for _ in 0..2 {
            let i = self.r.below(n as u32) as usize;
            let j = self.r.below(n as u32) as usize;
            if i != j {
                edges.push((i, j));
            }
        }
        for (i, j) in edges {
            self.corridor_between(i, j);
        }
    }

    fn corridor_between(&mut self, i: usize, j: usize) {
        let (a, b) = (self.rooms[i], self.rooms[j]);
        let (ax, az) = a.center();
        let (bx, bz) = b.center();
        let mut path = Vec::new();
        let x_first = self.r.chance(50);
        let (mut x, mut z) = (ax, az);
        path.push((x, z));
        let walk = |tx: i32, tz: i32, x: &mut i32, z: &mut i32, path: &mut Vec<(i32, i32)>| {
            while *x != tx {
                *x += (tx - *x).signum();
                path.push((*x, *z));
            }
            while *z != tz {
                *z += (tz - *z).signum();
                path.push((*x, *z));
            }
        };
        if x_first {
            walk(bx, az, &mut x, &mut z, &mut path);
            walk(bx, bz, &mut x, &mut z, &mut path);
        } else {
            walk(ax, bz, &mut x, &mut z, &mut path);
            walk(bx, bz, &mut x, &mut z, &mut path);
        }
        let outside: Vec<(i32, i32)> = path.iter().copied().filter(|&(x, z)| self.room_at(x, z).is_none()).collect();
        let n = outside.len() as i32;
        let diff = b.g - a.g;
        for (k, &(x, z)) in outside.iter().enumerate() {
            // heights change one tile at a time along the corridor, so every step up is a jump (R21)
            // round(a + diff * (k + 1) / (n + 1)), in integers
            let g = a.g + (2 * diff * (k as i32 + 1) + (n + 1)).div_euclid(2 * (n + 1));
            let g = g.clamp(a.g.min(b.g), a.g.max(b.g));
            if self.corridor.contains(&(x, z)) {
                // crossing another corridor: keep the lower floor, open up to both ceilings
                let old = self.l.ground_below(x, z, SIZE.y - 2).unwrap_or(g);
                let low = old.min(g);
                let top = old.max(g) + 2;
                self.carve_column(x, z, low, top - low);
            } else {
                // OURS: corridors are two tiles high: room to jump, and wider than tall on screen
                self.carve_column(x, z, g, 2);
            }
            self.corridor.insert((x, z));
        }
    }

    // ------------------------------------------------------------ room dressing

    fn dress_rooms(&mut self) {
        let depth = self.l.depth;
        let mut chasm_done = depth < 2;
        for i in 1..self.rooms.len() {
            let rm = self.rooms[i];
            if !chasm_done && rm.w >= 7 && rm.d >= 4 && self.r.chance(60) {
                self.chasm(i);
                chasm_done = true;
                continue;
            }
            if self.r.chance(35) && self.prefab(i) {
                continue;
            }
            match self.r.below(100) {
                0..=19 => self.ledge(i),
                20..=34 => self.trench(i),
                35..=44 => self.pool(i),
                45..=59 => self.pillars(i),
                _ => {}
            }
        }
    }

    fn ledge(&mut self, i: usize) {
        let rm = self.rooms[i];
        if rm.w < 5 || rm.d < 5 {
            return;
        }
        let w = self.r.range(2, 3.min(rm.w - 3));
        let d = self.r.range(2, 3.min(rm.d - 3));
        let x0 = self.r.range(rm.x0 + 1, rm.x0 + rm.w - 1 - w);
        let z0 = self.r.range(rm.z0 + 1, rm.z0 + rm.d - 1 - d);
        let accent = self.theme().accent();
        for x in x0..x0 + w {
            for z in z0..z0 + d {
                self.l.world.set(IVec3::new(x, rm.g, z), accent);
            }
        }
        // something worth the jump
        let top = IVec3::new(x0 + self.r.below(w as u32) as i32, rm.g + 1, z0 + self.r.below(d as u32) as i32);
        if self.r.chance(70) {
            self.chest(top);
        } else {
            let items = self.loot("floor", self.l.depth);
            if !items.is_empty() {
                self.l.items.insert(top, items);
            }
        }
    }

    fn trench(&mut self, i: usize) {
        let rm = self.rooms[i];
        let along_x = self.r.chance(50);
        let fill = match self.r.below(10) {
            0..=3 => None,
            4..=6 => Some(Cell::Water),
            _ if self.l.depth >= 7 => Some(Cell::Lava),
            _ => Some(Cell::Water),
        };
        let mut cells = Vec::new();
        if along_x {
            let z = rm.z0 + rm.d / 2;
            for x in rm.x0..rm.x0 + rm.w {
                cells.push(IVec3::new(x, rm.g - 1, z));
            }
        } else {
            let x = rm.x0 + rm.w / 2;
            for z in rm.z0..rm.z0 + rm.d {
                cells.push(IVec3::new(x, rm.g - 1, z));
            }
        }
        let floor = self.theme().floor();
        for p in cells {
            self.l.world.set(p.down(), floor);
            match fill {
                Some(cell) => {
                    self.l.world.set(p, cell);
                }
                None => {
                    self.l.world.set(p, Cell::Air);
                    self.l.features.insert(p, Feature::Spikes);
                }
            }
        }
    }

    fn pool(&mut self, i: usize) {
        let rm = self.rooms[i];
        let lava = self.l.depth >= 7 && self.r.chance(50);
        let w = self.r.range(2, (rm.w - 2).max(2));
        let d = self.r.range(2, (rm.d - 2).max(2));
        let x0 = rm.x0 + (rm.w - w) / 2;
        let z0 = rm.z0 + (rm.d - d) / 2;
        for x in x0..x0 + w {
            for z in z0..z0 + d {
                self.l.world.set(IVec3::new(x, rm.g - 1, z), if lava { Cell::Lava } else { Cell::Water });
            }
        }
    }

    fn pillars(&mut self, i: usize) {
        let rm = self.rooms[i];
        let wall = self.theme().accent();
        let mut x = rm.x0 + 1;
        while x < rm.x0 + rm.w - 1 {
            let mut z = rm.z0 + 1;
            while z < rm.z0 + rm.d - 1 {
                for y in rm.g..rm.g + rm.h {
                    self.l.world.set(IVec3::new(x, y, z), wall);
                }
                z += 3;
            }
            x += 3;
        }
    }

    /// A chasm two tiles wide and two deep across the room, with a lever bridge; a step inside each end lets a
    /// party that falls in climb out.
    fn chasm(&mut self, i: usize) {
        let rm = self.rooms[i];
        let cx = rm.x0 + rm.w / 2 - 1;
        for x in cx..cx + 2 {
            for z in rm.z0..rm.z0 + rm.d {
                self.l.world.set(IVec3::new(x, rm.g - 1, z), Cell::Air);
                self.l.world.set(IVec3::new(x, rm.g - 2, z), Cell::Air);
                self.l.world.set(IVec3::new(x, rm.g - 3, z), self.l.theme.floor());
            }
        }
        // the climbing steps, at opposite ends
        let accent = self.theme().accent();
        self.l.world.set(IVec3::new(cx, rm.g - 2, rm.z0), accent);
        self.l.world.set(IVec3::new(cx + 1, rm.g - 2, rm.z0 + rm.d - 1), accent);
        let zb = rm.z0 + rm.d / 2;
        let cells = vec![IVec3::new(cx, rm.g - 1, zb), IVec3::new(cx + 1, rm.g - 1, zb)];
        self.add_gate(Gate::Bridge { cells });
    }

    fn add_gate(&mut self, g: Gate) {
        self.gates.push(g);
        self.solved.push(false);
    }

    fn chest(&mut self, p: IVec3) {
        let depth = self.l.depth;
        let items = self.loot("chest", depth);
        let lock = if self.r.chance(25) { Some([KeyKind::Iron, KeyKind::Bronze, KeyKind::Bone][self.r.below(3) as usize]) } else { None };
        let trap = if self.r.chance(20) {
            Some([Trap::Needle, Trap::Gas, Trap::Fire, Trap::Alarm, Trap::Teleport][self.r.below(5) as usize])
        } else {
            None
        };
        let mimic = depth >= 2 && self.r.chance(8);
        let facing = Dir::ALL[self.r.below(4) as usize];
        self.l.features.insert(p, Feature::Chest { open: false, lock, trap, mimic, items, facing, great: false });
    }

    /// Stamp a fitting prefab into room `i`, centred, with a ring of floor left free around it.
    fn prefab(&mut self, i: usize) -> bool {
        let rm = self.rooms[i];
        let depth = self.l.depth;
        let fits: Vec<usize> = (0..self.c.prefabs.len())
            .filter(|&k| {
                let p = &self.c.prefabs[k];
                let (w, h, d) = p.size();
                !p.special && depth >= p.depth.0 && depth <= p.depth.1 && w + 2 <= rm.w && d + 2 <= rm.d && h - 1 <= rm.h
            })
            .collect();
        let weights: Vec<u32> = fits.iter().map(|&k| self.c.prefabs[k].weight).collect();
        let Some(pick) = self.r.weighted(&weights) else { return false };
        let k = fits[pick];
        let (w, _, d) = self.c.prefabs[k].size();
        let origin = IVec3::new(rm.x0 + (rm.w - w) / 2, rm.g - 1, rm.z0 + (rm.d - d) / 2);
        self.stamp(k, origin);
        true
    }

    fn stamp(&mut self, k: usize, origin: IVec3) {
        let p = self.c.prefabs[k].clone();
        let (w, h, d) = p.size();
        let theme = self.theme();
        let mut levers = Vec::new();
        let mut torches = Vec::new();
        let mut doors = Vec::new();
        let mut bridge = Vec::new();
        let mut secrets = Vec::new();
        for y in 0..h {
            for z in 0..d {
                for x in 0..w {
                    let ch = p.at(x, y, z);
                    let at = origin + IVec3::new(x, y, z);
                    let cell = match ch {
                        ' ' => continue,
                        '#' | 'L' | 'S' | 't' => theme.wall(),
                        '=' => theme.floor(),
                        'w' => Cell::Water,
                        'l' => Cell::Lava,
                        _ => Cell::Air,
                    };
                    self.l.world.set(at, cell);
                    self.l.features.remove(&at);
                    match ch {
                        'T' => {
                            self.l.features.insert(at, Feature::Tomb);
                        }
                        'C' | 'c' | 'M' => {
                            self.chest(at);
                            if let Some(Feature::Chest { lock, mimic, .. }) = self.l.features.get_mut(&at) {
                                if ch == 'c' {
                                    *lock = Some(KeyKind::Iron);
                                }
                                *mimic = ch == 'M';
                            }
                        }
                        'X' => {
                            self.l.features.insert(at, Feature::Chest { open: false, lock: None, trap: None, mimic: false, items: vec![Item::new("crown")], facing: Dir::South, great: true });
                        }
                        'F' => {
                            self.l.features.insert(at, Feature::Fountain { uses: 3 });
                        }
                        'D' => {
                            self.l.features.insert(at, Feature::Door { open: false, lock: None, portcullis: false });
                        }
                        'P' => {
                            self.l.features.insert(at, Feature::Door { open: false, lock: None, portcullis: true });
                            doors.push(at);
                        }
                        'L' => levers.push(at),
                        'S' => {
                            self.l.features.insert(at, Feature::SecretWall);
                            secrets.push(at);
                        }
                        // facings filled in below, once the neighbours exist
                        't' => torches.push(at),
                        'B' => bridge.push(at),
                        '^' => {
                            self.l.features.insert(at, Feature::Spikes);
                        }
                        'g' => {
                            let kind = pick_monster(self.c, self.l.depth, &mut self.r);
                            spawn(&mut self.l, self.c, &kind, at, &mut self.r, self.options);
                        }
                        'G' => {
                            spawn(&mut self.l, self.c, "wisp", at, &mut self.r, self.options);
                        }
                        'W' => {
                            spawn(&mut self.l, self.c, "warden", at, &mut self.r, self.options);
                        }
                        'k' => {
                            self.l.items.entry(at).or_default().push(Item::new("iron_key"));
                        }
                        'i' => {
                            let items = self.loot("floor", self.l.depth);
                            self.l.items.entry(at).or_default().extend(items);
                        }
                        _ => {}
                    }
                }
            }
        }
        let mut effects = Vec::new();
        for at in &doors {
            effects.push(LinkEffect::ToggleDoor(*at));
        }
        if !bridge.is_empty() {
            effects.push(LinkEffect::ToggleBridge { cells: bridge.clone(), cell: Cell::Wood });
        }
        for at in &secrets {
            effects.push(LinkEffect::Reveal(*at));
        }
        let link = self.next_link;
        let mut used_link = false;
        let open_side = |l: &Level, at: IVec3| Dir::ALL.into_iter().find(|d| !l.world.solid(at + d.delta()));
        for at in torches {
            if let Some(face) = open_side(&self.l, at) {
                self.l.features.insert(at, Feature::Torch { face });
            }
        }
        // a lever faces a side the party can stand on
        let stand_side = |l: &Level, at: IVec3| Dir::ALL.into_iter().find(|d| !l.world.solid(at + d.delta()) && l.world.solid(at + d.delta() + IVec3::DOWN));
        for at in levers {
            if let Some(face) = stand_side(&self.l, at) {
                self.l.features.insert(at, Feature::Lever { on: false, face, link });
                used_link = true;
            }
        }
        if used_link {
            self.l.links.insert(link, effects);
            self.next_link += 1;
        } else if !doors.is_empty() || !bridge.is_empty() {
            // a gated prefab without a lever of its own: the solver places one
            for at in doors {
                self.add_gate(Gate::Portcullis { at });
            }
            if !bridge.is_empty() {
                self.add_gate(Gate::Bridge { cells: bridge });
            }
        }
    }

    // ------------------------------------------------------------ doors

    fn doors(&mut self) {
        let depth = self.l.depth;
        let cells: Vec<(i32, i32)> = self.corridor.iter().copied().collect();
        for (x, z) in cells {
            // a doorway: corridor cell next to a room, with walls on both sides across the corridor
            let next_to_room = Dir::ALL.iter().any(|d| self.room_at(x + d.delta().x, z + d.delta().z).is_some());
            if !next_to_room || self.room_at(x, z).is_some() {
                continue;
            }
            let Some(g) = self.l.ground_below(x, z, SIZE.y - 2) else { continue };
            let p = IVec3::new(x, g, z);
            let walls_ns = self.l.world.solid(p + Dir::North.delta()) && self.l.world.solid(p + Dir::South.delta());
            let walls_ew = self.l.world.solid(p + Dir::East.delta()) && self.l.world.solid(p + Dir::West.delta());
            if !(walls_ns ^ walls_ew) || self.l.features.contains_key(&p) || !self.l.supported(p) {
                continue;
            }
            // only where the floor is level on both sides: a lintel over a door beside a step would leave no room
            // to jump up it
            let along = if walls_ns { IVec3::new(1, 0, 0) } else { IVec3::new(0, 0, 1) };
            let level = [p + along, p - along].iter().all(|q| self.l.ground_below(q.x, q.z, g + 2) == Some(g));
            if !level {
                continue;
            }
            if !self.r.chance(35) {
                continue;
            }
            // a lintel: the doorway is one tile high, so the door fills it
            let above = p.up();
            if !self.l.world.solid(above) {
                let wall = self.theme().wall();
                self.l.world.set(above, wall);
            }
            let roll = self.r.below(100);
            if roll < 20 {
                let key = [KeyKind::Iron, KeyKind::Bronze, KeyKind::Bone][self.r.below(3) as usize];
                self.l.features.insert(p, Feature::Door { open: false, lock: Some(key), portcullis: false });
                self.add_gate(Gate::Locked { at: p, key });
            } else if roll < 32 && depth >= 2 {
                self.l.features.insert(p, Feature::Door { open: false, lock: None, portcullis: true });
                self.add_gate(Gate::Portcullis { at: p });
            } else {
                self.l.features.insert(p, Feature::Door { open: false, lock: None, portcullis: false });
            }
        }
    }

    // ------------------------------------------------------------ the solver

    fn blocked(&self, p: IVec3, passable_doors: &BTreeSet<IVec3>) -> bool {
        if self.l.world.solid(p) || self.bridges_up.contains(&p) {
            return true;
        }
        if self.l.world.get(p) == Cell::Lava {
            return true;
        }
        match self.l.features.get(&p) {
            Some(Feature::Door { open, lock, portcullis }) => !(*open || (lock.is_none() && !portcullis) || passable_doors.contains(&p)),
            Some(f) => f.blocks(),
            None => false,
        }
    }

    fn solid_for(&self, p: IVec3) -> bool {
        self.l.world.solid(p) || self.bridges_up.contains(&p)
    }

    fn fall(&self, mut p: IVec3, doors: &BTreeSet<IVec3>) -> Option<IVec3> {
        while !self.solid_for(p.down()) {
            if p.y <= 1 || self.blocked(p.down(), doors) {
                return None;
            }
            p = p.down();
        }
        Some(p)
    }

    /// Every tile the party can stand on from `start`, with its distance in moves.
    fn reach(&self, start: IVec3) -> BTreeMap<IVec3, u32> {
        let mut doors = BTreeSet::new();
        for (k, g) in self.gates.iter().enumerate() {
            if self.solved[k] {
                match g {
                    Gate::Locked { at, .. } | Gate::Portcullis { at } => {
                        doors.insert(*at);
                    }
                    Gate::Bridge { .. } => {}
                }
            }
        }
        let mut dist = BTreeMap::new();
        let mut q = VecDeque::new();
        dist.insert(start, 0u32);
        q.push_back(start);
        while let Some(p) = q.pop_front() {
            let dp = dist[&p];
            let push = |t: IVec3, dist: &mut BTreeMap<IVec3, u32>, q: &mut VecDeque<IVec3>| {
                if let std::collections::btree_map::Entry::Vacant(e) = dist.entry(t) {
                    e.insert(dp + 1);
                    q.push_back(t);
                }
            };
            for d in Dir::ALL {
                let t = p + d.delta();
                if !self.blocked(t, &doors) {
                    if let Some(l) = self.fall(t, &doors) {
                        push(l, &mut dist, &mut q);
                    }
                    let t2 = t + d.delta();
                    if !self.blocked(t2, &doors)
                        && let Some(l) = self.fall(t2, &doors) {
                            push(l, &mut dist, &mut q);
                        }
                }
                let up = p.up();
                if !self.blocked(up, &doors) {
                    let u = up + d.delta();
                    if !self.blocked(u, &doors) && self.solid_for(u.down()) {
                        push(u, &mut dist, &mut q);
                    }
                }
            }
        }
        dist
    }

    fn near(set: &BTreeMap<IVec3, u32>, p: IVec3) -> bool {
        Dir::ALL.iter().any(|d| {
            let n = p + d.delta();
            set.contains_key(&n) || set.contains_key(&n.up()) || set.contains_key(&n.down())
        })
    }

    /// A wall spot for a lever next to a tile in `set`: (wall cell, facing out of the wall).
    fn lever_spot(&mut self, set: &BTreeMap<IVec3, u32>) -> Option<(IVec3, Dir)> {
        let mut spots = Vec::new();
        for &s in set.keys() {
            for d in Dir::ALL {
                let w = s + d.delta();
                if self.l.world.solid(w) && self.l.world.solid(w.up()) && !self.l.features.contains_key(&w) && w.x > 0 && w.z > 0 && w.x < SIZE.x - 1 && w.z < SIZE.z - 1 && !self.l.features.contains_key(&s) {
                    spots.push((w, d.opposite()));
                }
            }
        }
        self.r.pick(&spots).copied()
    }

    fn floor_spot(&mut self, set: &BTreeMap<IVec3, u32>, avoid_start: Option<IVec3>) -> Option<IVec3> {
        let cells: Vec<IVec3> = set
            .keys()
            .copied()
            .filter(|p| !self.l.features.contains_key(p) && !self.l.items.contains_key(p) && self.l.world.get(*p) == Cell::Air)
            .filter(|p| avoid_start.is_none_or(|s| p.flat_dist(s) >= 3))
            .collect();
        self.r.pick(&cells).copied()
    }

    /// Give every gate the party can get to its key or lever, inside what is already reachable, until nothing
    /// changes. Returns the final reach.
    fn solve(&mut self, start: IVec3) -> BTreeMap<IVec3, u32> {
        loop {
            let reach = self.reach(start);
            let mut changed = false;
            for k in 0..self.gates.len() {
                if self.solved[k] {
                    continue;
                }
                let gate = self.gates[k].clone();
                let frontier = match &gate {
                    Gate::Locked { at, .. } | Gate::Portcullis { at } => Gen::near(&reach, *at),
                    Gate::Bridge { cells } => cells.iter().any(|c| Gen::near(&reach, c.up())),
                };
                if !frontier {
                    continue;
                }
                match gate {
                    Gate::Locked { key, .. } => {
                        let def = match key {
                            KeyKind::Iron => "iron_key",
                            KeyKind::Bronze => "bronze_key",
                            KeyKind::Bone => "bone_key",
                        };
                        if let Some(p) = self.floor_spot(&reach, None) {
                            self.l.items.entry(p).or_default().push(Item::new(def));
                        }
                    }
                    Gate::Portcullis { at } => {
                        if let Some((w, face)) = self.lever_spot(&reach) {
                            let link = self.next_link;
                            self.next_link += 1;
                            self.l.links.insert(link, vec![LinkEffect::ToggleDoor(at)]);
                            self.l.features.insert(w, Feature::Lever { on: false, face, link });
                        }
                    }
                    Gate::Bridge { ref cells } => {
                        if let Some((w, face)) = self.lever_spot(&reach) {
                            let link = self.next_link;
                            self.next_link += 1;
                            self.l.links.insert(link, vec![LinkEffect::ToggleBridge { cells: cells.clone(), cell: Cell::Wood }]);
                            self.l.features.insert(w, Feature::Lever { on: false, face, link });
                            self.bridges_up.extend(cells.iter().copied());
                        }
                    }
                }
                self.solved[k] = true;
                changed = true;
            }
            if !changed {
                return reach;
            }
        }
    }

    // ------------------------------------------------------------ filling the level

    fn populate(&mut self, reach: &BTreeMap<IVec3, u32>, start: IVec3) {
        let depth = self.l.depth as i32;
        let far: BTreeMap<IVec3, u32> = reach.iter().filter(|(p, _)| p.flat_dist(start) >= 6).map(|(p, d)| (*p, *d)).collect();
        let monsters = (5 + depth * 2).min(22);
        for _ in 0..monsters {
            if let Some(p) = self.floor_spot(&far, Some(start))
                && self.l.monster_at(p).is_none() {
                    let kind = pick_monster(self.c, self.l.depth, &mut self.r);
                    spawn(&mut self.l, self.c, &kind, p, &mut self.r, self.options);
                }
        }
        let chests = self.r.range(2, 4);
        for _ in 0..chests {
            // a chest in a room, off the corridors, so it never blocks the way
            let room_cells: BTreeMap<IVec3, u32> = reach
                .iter()
                .filter(|(p, _)| self.room_at(p.x, p.z).is_some() && !self.corridor.contains(&(p.x, p.z)))
                .filter(|(p, _)| Dir::ALL.iter().filter(|d| self.l.world.solid(**p + d.delta())).count() >= 1)
                .map(|(p, d)| (*p, *d))
                .collect();
            if let Some(p) = self.floor_spot(&room_cells, Some(start))
                && self.l.monster_at(p).is_none() {
                    self.chest(p);
                }
        }
        for _ in 0..self.r.range(3, 6) {
            if let Some(p) = self.floor_spot(reach, None) {
                let items = self.loot("floor", self.l.depth);
                if !items.is_empty() {
                    self.l.items.entry(p).or_default().extend(items);
                }
            }
        }
        if self.r.chance(35) {
            let inner: BTreeMap<IVec3, u32> = reach
                .iter()
                .filter(|(p, _)| self.room_at(p.x, p.z).is_some() && Dir::ALL.iter().all(|d| !self.l.world.solid(**p + d.delta())))
                .map(|(p, d)| (*p, *d))
                .collect();
            if let Some(p) = self.floor_spot(&inner, Some(start)) {
                self.l.features.insert(p, Feature::Fountain { uses: 3 });
            }
        }
        // pressure plates in corridors: darts, or an ambush
        let corridor_cells: BTreeMap<IVec3, u32> = reach.iter().filter(|(p, _)| self.corridor.contains(&(p.x, p.z))).map(|(p, d)| (*p, *d)).collect();
        for _ in 0..self.r.range(0, 2) {
            if let Some(p) = self.floor_spot(&corridor_cells, Some(start)) {
                let link = self.next_link;
                self.next_link += 1;
                let effect = if self.r.chance(70) {
                    LinkEffect::Darts
                } else {
                    let kind = pick_monster(self.c, self.l.depth, &mut self.r);
                    let at: Vec<IVec3> = Dir::ALL.iter().map(|d| p + d.delta() * 2).filter(|q| reach.contains_key(q)).collect();
                    LinkEffect::Spawn { kind, at }
                };
                self.l.links.insert(link, vec![effect]);
                self.l.features.insert(p, Feature::Plate { pressed: false, link });
            }
        }
        // torches on room walls
        for i in 0..self.rooms.len() {
            let rm = self.rooms[i];
            for _ in 0..2 {
                let (x, z) = (self.r.range(rm.x0, rm.x0 + rm.w - 1), self.r.range(rm.z0, rm.z0 + rm.d - 1));
                let p = IVec3::new(x, rm.g + 1, z);
                for d in Dir::ALL {
                    let w = p + d.delta();
                    if self.l.world.solid(w) && !self.l.features.contains_key(&w) && !self.l.world.solid(p) && self.room_at(w.x, w.z).is_none() {
                        self.l.features.insert(w, Feature::Torch { face: d.opposite() });
                        break;
                    }
                }
            }
        }
    }

    /// A hidden vault off a room: a 3×3 cell behind a secret wall, with a good chest.
    fn vault(&mut self, reach: &BTreeMap<IVec3, u32>) {
        for _ in 0..30 {
            let i = self.r.range(1, self.rooms.len() as i32 - 1).max(0) as usize;
            let rm = self.rooms[i];
            let d = Dir::ALL[self.r.below(4) as usize];
            let (cx, cz) = rm.center();
            let (ex, ez) = match d {
                Dir::North => (cx, rm.z0 - 1),
                Dir::South => (cx, rm.z0 + rm.d),
                Dir::East => (rm.x0 + rm.w, cz),
                Dir::West => (rm.x0 - 1, cz),
            };
            let wall = IVec3::new(ex, rm.g, ez);
            let inside = IVec3::new(ex, rm.g, ez) - d.delta();
            if !reach.contains_key(&inside) {
                continue;
            }
            let centre = wall + d.delta() * 2;
            let mut ok = self.l.world.solid(wall) && self.l.world.solid(wall.up());
            for dx in -2..=2 {
                for dz in -2..=2 {
                    let q = IVec3::new(centre.x + dx, rm.g, centre.z + dz);
                    if q.x < 1 || q.z < 1 || q.x >= SIZE.x - 1 || q.z >= SIZE.z - 1 {
                        ok = false;
                    }
                    for y in rm.g - 1..rm.g + 3 {
                        if !self.l.world.solid(IVec3::new(q.x, y, q.z)) {
                            ok = false;
                        }
                    }
                }
            }
            if !ok {
                continue;
            }
            for dx in -1..=1 {
                for dz in -1..=1 {
                    self.carve_column(centre.x + dx, centre.z + dz, rm.g, 3);
                }
            }
            self.l.features.insert(wall, Feature::SecretWall);
            let mut items = self.loot("chest", self.l.depth);
            items.extend(self.loot("chest", self.l.depth + 1));
            self.l.features.insert(centre + d.delta(), Feature::Chest { open: false, lock: None, trap: None, mimic: false, items, facing: d.opposite(), great: false });
            return;
        }
    }

    fn finish_start(&mut self, start: IVec3) {
        self.l.start = start;
        let reach = self.reach(start);
        self.l.start_facing = Dir::ALL
            .into_iter()
            .find(|d| reach.contains_key(&(start + d.delta())))
            .unwrap_or(Dir::North);
    }
}

fn rooms(c: &Content, r: Mt19937, depth: u8, options: RunOptions, seed: u64) -> Option<Level> {
    let mut g = Gen::new(c, r, depth, options, seed);
    let want = g.r.range(7, 10) as usize;
    g.place_rooms(want, 4, depth >= FINAL_DEPTH);
    if g.rooms.len() < 5 {
        return None;
    }
    g.connect();
    g.dress_rooms();
    g.doors();
    let (sx, sz) = g.rooms[0].center();
    let start = IVec3::new(sx, g.rooms[0].g, sz);
    // the start must be a plain tile
    g.l.features.remove(&start);
    g.l.items.remove(&start);
    if g.l.world.solid(start) || !g.l.supported(start) || g.l.world.get(start) != Cell::Air {
        return None;
    }
    let reach = g.solve(start);
    let rooms_reached = (0..g.rooms.len())
        .filter(|&i| reach.keys().any(|p| g.room_at(p.x, p.z) == Some(i)))
        .count();
    if rooms_reached * 10 < g.rooms.len() * 7 {
        return None;
    }
    // the far room: stairs, or on the last level the warden's vault
    let mut far_room = None;
    let mut far_d = 0;
    for (p, d) in &reach {
        if let Some(i) = g.room_at(p.x, p.z)
            && i != 0 && *d > far_d {
                far_d = *d;
                far_room = Some(i);
            }
    }
    let far_room = far_room?;
    if depth >= FINAL_DEPTH {
        let vault = c.prefabs.iter().position(|p| p.special && p.name == "warden_vault")?;
        let (w, _, d) = c.prefabs[vault].size();
        // the biggest reached room that holds the vault
        let mut best = None;
        for i in 1..g.rooms.len() {
            let rm = g.rooms[i];
            let reached = reach.keys().any(|p| g.room_at(p.x, p.z) == Some(i));
            if reached && rm.w >= w + 2 && rm.d >= d + 2 && rm.h >= 4 {
                best = Some(i);
            }
        }
        let i = best?;
        let rm = g.rooms[i];
        g.l.features.retain(|p, _| !rm.contains(p.x, p.z) || !matches!(p.y - rm.g, -3..=5));
        g.l.monsters.retain(|m| !rm.contains(m.pos.x, m.pos.z));
        g.l.items.retain(|p, _| !rm.contains(p.x, p.z));
        let origin = IVec3::new(rm.x0 + (rm.w - w) / 2, rm.g - 1, rm.z0 + (rm.d - d) / 2);
        g.stamp(vault, origin);
        let reach = g.solve(start);
        let great = g.l.features.iter().find(|(_, f)| matches!(f, Feature::Chest { great: true, .. })).map(|(p, _)| *p)?;
        if !Dir::ALL.iter().any(|d| reach.contains_key(&(great + d.delta()))) {
            return None;
        }
        g.populate(&reach, start);
    } else {
        let cells: Vec<IVec3> = reach
            .keys()
            .copied()
            .filter(|p| g.room_at(p.x, p.z) == Some(far_room) && !g.l.features.contains_key(p) && g.l.world.get(*p) == Cell::Air)
            .collect();
        let stairs = *g.r.pick(&cells)?;
        g.l.items.remove(&stairs);
        let facing = Dir::ALL[g.r.below(4) as usize];
        g.l.features.insert(stairs, Feature::Stairs { facing });
        if g.r.chance(50) {
            g.vault(&reach);
        }
        g.populate(&reach, start);
    }
    g.finish_start(start);
    Some(g.l)
}

/// Cave levels: cellular automaton caverns with rolling floors. REF R46 (caves, unconfirmed); OURS.
fn caves(c: &Content, r: Mt19937, depth: u8, options: RunOptions, seed: u64) -> Option<Level> {
    let mut g = Gen::new(c, r, depth, options, seed);
    let (w, d) = (SIZE.x as usize, SIZE.z as usize);
    let mut open = vec![false; w * d];
    for z in 2..d - 2 {
        for x in 2..w - 2 {
            open[z * w + x] = g.r.chance(55);
        }
    }
    for _ in 0..4 {
        let mut next = open.clone();
        for z in 2..d - 2 {
            for x in 2..w - 2 {
                let mut n = 0;
                for dz in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if (dx, dz) != (0, 0) && open[(z as i32 + dz) as usize * w + (x as i32 + dx) as usize] {
                            n += 1;
                        }
                    }
                }
                next[z * w + x] = n >= 5 || (open[z * w + x] && n >= 4);
            }
        }
        open = next;
    }
    // heights: a coarse random grid, smoothed, so neighbours rarely differ by more than one
    let coarse = 8usize;
    let cw = w / coarse + 2;
    let grid: Vec<i32> = (0..cw * cw).map(|_| g.r.range(3, 5)).collect();
    let height = |x: usize, z: usize| -> i32 {
        let (gx, gz) = (x / coarse, z / coarse);
        let (fx, fz) = ((x % coarse) as i32, (z % coarse) as i32);
        let a = grid[gz * cw + gx];
        let b = grid[gz * cw + gx + 1];
        let cc = grid[(gz + 1) * cw + gx];
        let dd = grid[(gz + 1) * cw + gx + 1];
        let top = a * (coarse as i32 - fx) + b * fx;
        let bot = cc * (coarse as i32 - fx) + dd * fx;
        let v = top * (coarse as i32 - fz) + bot * fz;
        let s = (coarse * coarse) as i32;
        (v + s / 2) / s
    };
    for z in 0..d {
        for x in 0..w {
            if open[z * w + x] {
                let gy = height(x, z);
                g.carve_column(x as i32, z as i32, gy, 4);
            }
        }
    }
    // pools in the low places, moss on some floors
    for z in 0..d {
        for x in 0..w {
            if !open[z * w + x] {
                continue;
            }
            let gy = height(x, z);
            let p = IVec3::new(x as i32, gy - 1, z as i32);
            if gy == 3 && g.r.chance(18) {
                g.l.world.set(p, Cell::Water);
            } else if g.r.chance(20) {
                g.l.world.set(p, Cell::Moss);
            }
        }
    }
    // start in the open cell nearest a corner, stairs as far as the solver can walk
    let mut start = None;
    'find: for s in 3..(w as i32) {
        for t in 0..=s {
            let (x, z) = (3 + t, 3 + s - t);
            if (x as usize) < w && (z as usize) < d && open[z as usize * w + x as usize] {
                let gy = height(x as usize, z as usize);
                let p = IVec3::new(x, gy, z);
                if g.l.world.get(p) == Cell::Air && g.l.supported(p) {
                    start = Some(p);
                    break 'find;
                }
            }
        }
    }
    let start = start?;
    let reach = g.solve(start);
    if reach.len() < 500 {
        return None;
    }
    let (&stairs, _) = reach.iter().filter(|(p, _)| g.l.world.get(**p) == Cell::Air).max_by_key(|(p, dd)| (**dd, std::cmp::Reverse(**p)))?;
    g.l.features.insert(stairs, Feature::Stairs { facing: Dir::North });
    // a room list for the populate step: the whole cave counts as one room
    g.rooms.push(Room { x0: 0, z0: 0, w: SIZE.x, d: SIZE.z, g: 3, h: 4 });
    g.populate(&reach, start);
    g.finish_start(start);
    Some(g.l)
}

// ---------------------------------------------------------------- shared by the sim

/// A monster kind for this depth, by weight. Never the special ones.
pub fn pick_monster(c: &Content, depth: u8, r: &mut Mt19937) -> String {
    let ids: Vec<&String> = c.monsters.iter().filter(|(_, m)| !m.special && depth >= m.depth.0 && depth <= m.depth.1).map(|(id, _)| id).collect();
    let weights: Vec<u32> = ids.iter().map(|id| c.monsters[*id].weight).collect();
    match r.weighted(&weights) {
        Some(i) => ids[i].clone(),
        None => "shambler".to_string(),
    }
}

/// Put a monster on the level; returns its id. REF R43: spiders become beetles in the spider-free mode.
pub fn spawn(l: &mut Level, c: &Content, kind: &str, pos: IVec3, r: &mut Mt19937, options: RunOptions) -> u32 {
    let mut kind = kind.to_string();
    if options.spider_free && c.monster(&kind).spider && c.monsters.contains_key("beetle") {
        kind = "beetle".into();
    }
    let def = c.monster(&kind);
    let hp = def.hp.roll(r).max(1);
    let id = l.next_monster_id;
    l.next_monster_id += 1;
    l.monsters.push(Monster {
        id,
        kind,
        pos,
        facing: Dir::ALL[r.below(4) as usize],
        hp,
        max_hp: hp,
        energy: r.below(12 * 30) as i32,
        awake: false,
        sleep: 0,
    });
    id
}

/// Where uniques may come from (board row G16): each has one depth per run, chosen from the seed, and is
/// placed there at most once. Loot rolled during play (drops, fountains) gets no rule and never a unique.
pub struct UniqueRule<'a> {
    pub seed: u64,
    pub placed: &'a mut BTreeSet<String>,
}

/// The one depth where a unique can appear in the run with this seed.
pub fn unique_depth(c: &Content, seed: u64, id: &str) -> u8 {
    let d = c.item(id).depth;
    let span = (d.1.saturating_sub(d.0) as u64) + 1;
    d.0 + (rng::fnv1a64(format!("{seed}:{id}").as_bytes()) % span) as u8
}

/// One random item of a category for this depth.
pub fn roll_item(c: &Content, cat: &str, depth: u8, r: &mut Mt19937, uniques: Option<&mut UniqueRule>) -> Option<Item> {
    let ids: Vec<&String> = c
        .items
        .iter()
        .filter(|(_, d)| d.kind.category() == cat && depth >= d.depth.0 && depth <= d.depth.1 && d.weight > 0)
        .filter(|(id, d)| {
            !d.unique || uniques.as_deref().is_some_and(|u| unique_depth(c, u.seed, id) == depth && !u.placed.contains(*id))
        })
        .map(|(id, _)| id)
        .collect();
    let weights: Vec<u32> = ids.iter().map(|id| c.items[*id].weight).collect();
    let id = ids[r.weighted(&weights)?].clone();
    if c.item(&id).unique
        && let Some(u) = uniques {
            u.placed.insert(id.clone());
        }
    Some(make_item(c, &id, depth, r))
}

/// An item of a given id, with its enchantment, curse and charges rolled. OURS: 8% of arms and armour (plus
/// the item's own chance) come cursed at -1 to -3; a quarter of the rest are +1 or better, more so deeper.
pub fn make_item(c: &Content, id: &str, depth: u8, r: &mut Mt19937) -> Item {
    let def = c.item(id);
    let mut it = Item::new(id);
    if def.unique {
        it.enchant = 3;
        it.known = false;
        return it;
    }
    match &def.kind {
        ItemKind::Weapon { .. } | ItemKind::Armor { .. } => {
            it.known = false;
            if r.chance(8 + def.cursed) {
                it.cursed = true;
                it.enchant = -(r.range(1, 3) as i8);
            } else if r.chance(25) {
                it.enchant = r.range(1, 1 + depth as i32 / 3) as i8;
            }
        }
        ItemKind::Ring { .. } | ItemKind::Amulet { .. } => {
            it.known = false;
            it.cursed = r.chance(def.cursed);
        }
        ItemKind::Wand { charges, .. } => {
            it.known = false;
            it.charges = charges.roll(r).max(1) as i16;
        }
        ItemKind::Gold { amount } => {
            it.amount = (amount.roll(r).max(1) * depth.max(1) as i32) as u32;
        }
        _ => {}
    }
    it
}

/// Roll a loot table: `rolls` picks of a category (or `#id` for one item) by weight.
pub fn roll_loot(c: &Content, table: &str, depth: u8, r: &mut Mt19937, mut uniques: Option<&mut UniqueRule>) -> Vec<Item> {
    let Some(t) = c.loot.get(table) else { return Vec::new() };
    let n = t.rolls.roll(r).max(0);
    let weights: Vec<u32> = t.entries.iter().map(|(_, w)| *w).collect();
    let mut out = Vec::new();
    for _ in 0..n {
        let Some(i) = r.weighted(&weights) else { break };
        let cat = &t.entries[i].0;
        if let Some(id) = cat.strip_prefix('#') {
            out.push(make_item(c, id, depth, r));
        } else if let Some(it) = roll_item(c, cat, depth, r, uniques.as_deref_mut()) {
            out.push(it);
        }
    }
    out
}

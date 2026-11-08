//! Monsters on tiles, and how they find their way. REF R41-R43: zombie-like walkers, a giant spider, a
//! ghost-like figure. OURS: everything about how they move and choose (board row G8).

use crate::content::MonsterDef;
use crate::geom::{Dir, IVec3};
use crate::world::Level;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Monster {
    pub id: u32,
    pub kind: String,
    pub pos: IVec3,
    pub facing: Dir,
    pub hp: i32,
    pub max_hp: i32,
    /// OURS: energy-based speed; acts whenever it holds 12.
    pub energy: i32,
    pub awake: bool,
    /// Ticks of magical sleep left.
    #[serde(default)]
    pub sleep: u16,
}

/// Can a monster of this kind stand in `p`?
pub fn can_stand(level: &Level, def: &MonsterDef, p: IVec3) -> bool {
    if !level.open(p) || level.world.get(p) == crate::world::Cell::Lava {
        return false;
    }
    def.flies || level.supported(p)
}

/// The cells a monster could move to from `p`, in a fixed order (north, east, south, west; then up and down
/// for fliers). Walkers may drop one tile; climbers may also go up one.
pub fn moves_from(level: &Level, def: &MonsterDef, p: IVec3) -> Vec<IVec3> {
    let mut out = Vec::with_capacity(6);
    for d in Dir::ALL {
        let t = p + d.delta();
        if def.flies {
            if level.open(t) && level.world.get(t) != crate::world::Cell::Lava {
                out.push(t);
            }
            continue;
        }
        if can_stand(level, def, t) {
            out.push(t);
        } else if level.open(t) && can_stand(level, def, t.down()) {
            // one tile down, off a ledge
            out.push(t.down());
        } else if def.climbs && level.world.solid(t) && level.open(p.up()) && can_stand(level, def, t.up()) {
            out.push(t.up());
        }
    }
    if def.flies {
        for t in [p.up(), p.down()] {
            if level.open(t) && level.world.get(t) != crate::world::Cell::Lava {
                out.push(t);
            }
        }
    }
    out
}

/// The first step on a shortest path from `from` to any tile next to `goal` (same height, 4-neighbour), or
/// None. Breadth-first within `radius` tiles; ties broken by the fixed move order, so it is deterministic.
pub fn step_towards(level: &Level, def: &MonsterDef, from: IVec3, goal: IVec3, radius: i32) -> Option<IVec3> {
    let adjacent = |p: IVec3| p.y == goal.y && p.flat_manhattan(goal) == 1;
    if adjacent(from) {
        return None;
    }
    let mut prev: BTreeMap<IVec3, IVec3> = BTreeMap::new();
    let mut q = VecDeque::new();
    q.push_back(from);
    prev.insert(from, from);
    let mut found = None;
    let mut visited = 0;
    while let Some(p) = q.pop_front() {
        visited += 1;
        if visited > 3000 {
            break;
        }
        if adjacent(p) {
            found = Some(p);
            break;
        }
        for t in moves_from(level, def, p) {
            if t == goal || prev.contains_key(&t) || t.flat_dist(from) > radius {
                continue;
            }
            if level.monster_at(t).is_some() && !adjacent(t) {
                // others in the way: path around them, unless they stand where we want to be anyway
                continue;
            }
            prev.insert(t, p);
            q.push_back(t);
        }
    }
    let mut p = found?;
    while prev[&p] != from {
        p = prev[&p];
    }
    Some(p)
}

/// Is there a clear line from `a` to `b`? Steps through the cells between them (centres), stopping at walls
/// and closed doors.
pub fn line_of_sight(level: &Level, a: IVec3, b: IVec3) -> bool {
    let d = b - a;
    let n = d.x.abs().max(d.y.abs()).max(d.z.abs());
    if n <= 1 {
        return true;
    }
    for i in 1..n {
        // integer rounding of a + d * i / n, so there are no floats in the sim
        let t = |v: i32, o: i32| o + (v * 2 * i + n).div_euclid(2 * n);
        let p = IVec3::new(t(d.x, a.x), t(d.y, a.y), t(d.z, a.z));
        if p == a || p == b {
            continue;
        }
        if !level.open(p) && !level.features.get(&p).is_some_and(|f| matches!(f, crate::world::Feature::Chest { .. } | crate::world::Feature::Tomb | crate::world::Feature::Fountain { .. })) {
            return false;
        }
    }
    true
}

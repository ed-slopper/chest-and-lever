//! Grid coordinates and facings.
//!
//! REF R20: movement is tile-based, so every position in the sim is a whole tile.
//! Axes: x east, y up, z south (so north is -z, as Bevy's forward).

use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Mul, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IVec3 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl IVec3 {
    pub const ZERO: IVec3 = IVec3 { x: 0, y: 0, z: 0 };
    pub const UP: IVec3 = IVec3 { x: 0, y: 1, z: 0 };
    pub const DOWN: IVec3 = IVec3 { x: 0, y: -1, z: 0 };

    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        IVec3 { x, y, z }
    }
    pub fn up(self) -> Self {
        self + IVec3::UP
    }
    pub fn down(self) -> Self {
        self + IVec3::DOWN
    }
    /// Distance on the floor plan, ignoring height (Chebyshev).
    pub fn flat_dist(self, o: IVec3) -> i32 {
        (self.x - o.x).abs().max((self.z - o.z).abs())
    }
    /// Steps on the floor plan (Manhattan, ignoring height).
    pub fn flat_manhattan(self, o: IVec3) -> i32 {
        (self.x - o.x).abs() + (self.z - o.z).abs()
    }
    pub fn manhattan(self, o: IVec3) -> i32 {
        (self.x - o.x).abs() + (self.y - o.y).abs() + (self.z - o.z).abs()
    }
}

impl Add for IVec3 {
    type Output = IVec3;
    fn add(self, o: IVec3) -> IVec3 {
        IVec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl AddAssign for IVec3 {
    fn add_assign(&mut self, o: IVec3) {
        *self = *self + o;
    }
}
impl Sub for IVec3 {
    type Output = IVec3;
    fn sub(self, o: IVec3) -> IVec3 {
        IVec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl Mul<i32> for IVec3 {
    type Output = IVec3;
    fn mul(self, k: i32) -> IVec3 {
        IVec3::new(self.x * k, self.y * k, self.z * k)
    }
}

/// A facing on the floor plan. REF R20: turns are 90 degrees.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Dir {
    #[default]
    North,
    East,
    South,
    West,
}

impl Dir {
    pub const ALL: [Dir; 4] = [Dir::North, Dir::East, Dir::South, Dir::West];

    pub fn delta(self) -> IVec3 {
        match self {
            Dir::North => IVec3::new(0, 0, -1),
            Dir::East => IVec3::new(1, 0, 0),
            Dir::South => IVec3::new(0, 0, 1),
            Dir::West => IVec3::new(-1, 0, 0),
        }
    }
    pub fn cw(self) -> Dir {
        Dir::ALL[(self as usize + 1) % 4]
    }
    pub fn ccw(self) -> Dir {
        Dir::ALL[(self as usize + 3) % 4]
    }
    pub fn opposite(self) -> Dir {
        Dir::ALL[(self as usize + 2) % 4]
    }
    pub fn index(self) -> usize {
        self as usize
    }
    /// The facing that points from `a` to a 4-neighbour `b` on the floor plan, if any.
    pub fn between(a: IVec3, b: IVec3) -> Option<Dir> {
        let d = IVec3::new(b.x - a.x, 0, b.z - a.z);
        Dir::ALL.into_iter().find(|dir| dir.delta() == d)
    }
}

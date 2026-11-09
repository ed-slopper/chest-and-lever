//! Tile textures (board row C8), made in code: 16×16 texels per cell kind, a top and a side each. Every texel
//! has a palette index and a depth: how many sixteenths of a tile it sits back from the face, so mortar, seams
//! and cracks are carved into the wall as voxels rather than painted on (the relief mesher in `lib.rs`).
//! All ours (F8: 16 texels per tile edge, our own look).

use crate::pal::{ix, BLUE, BONE, BROWN, FIRE, GREEN, GREY, VIOLET};
use lac_sim::Cell;

pub const TILE: usize = 16;
pub const TILES_PER_ROW: usize = 16;
/// The deepest a texel is carved, in sixteenths of a tile.
pub const MAX_DEPTH: u8 = 2;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Texel {
    pub idx: u8,
    pub depth: u8,
}

pub type Tex = [[Texel; TILE]; TILE];

/// The atlas slot of a cell's face: top (and bottom) or side.
pub fn tile_of(cell: Cell, side: bool) -> u32 {
    cell as u32 * 2 + side as u32
}

/// A small deterministic hash for texture noise.
fn h(x: i32, y: i32, salt: u32) -> u32 {
    let mut v = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77) ^ salt.wrapping_mul(0xC2B2_AE3D);
    v ^= v >> 15;
    v = v.wrapping_mul(0x2C1B_3C6D);
    v ^= v >> 12;
    v
}

fn noise(x: i32, y: i32, salt: u32, n: u32) -> u32 {
    h(x, y, salt) % n
}

fn fill(f: impl Fn(i32, i32) -> (u8, u8)) -> Tex {
    let mut t = [[Texel::default(); TILE]; TILE];
    for (y, row) in t.iter_mut().enumerate() {
        for (x, v) in row.iter_mut().enumerate() {
            let (idx, depth) = f(x as i32, y as i32);
            *v = Texel { idx, depth: depth.min(MAX_DEPTH) };
        }
    }
    t
}

/// Dressed stone blocks, two courses per tile, joints carved deep.
fn blocks(salt: u32, ramp: u8, base: u8) -> Tex {
    fill(|x, y| {
        let course = y / 8;
        let off = if course % 2 == 0 { 0 } else { 5 };
        let joint = y % 8 == 7 || (x + off) % 10 == 9 || (x + off) % 10 == 9 + 16;
        let block = noise((x + off) / 10, course, salt, 3) as u8;
        if joint {
            return (ix(ramp, base.saturating_sub(1)), 2);
        }
        let n = noise(x, y, salt + 1, 12);
        let mut s = base + block;
        if n == 0 {
            s = s.saturating_sub(1);
        }
        if n == 11 {
            s = (s + 1).min(7);
        }
        // pits and chips in the faces
        let pit = noise(x, y, salt + 2, 37) == 0;
        (ix(ramp, if pit { base.saturating_sub(1) } else { s }), if pit { 1 } else { 0 })
    })
}

fn rough(salt: u32, ramp: u8, base: u8) -> Tex {
    fill(|x, y| {
        let n = noise(x, y, salt, 10);
        let lump = noise(x / 4, y / 4, salt + 7, 3) as u8;
        let mut s = base + lump;
        if n == 0 {
            s = s.saturating_sub(1);
        }
        if n == 9 {
            s += 1;
        }
        let crack = noise(x / 2, y, salt + 3, 19) == 0;
        let hollow = lump == 0 && noise(x / 2, y / 2, salt + 5, 3) == 0;
        if crack {
            (ix(ramp, base.saturating_sub(1)), 2)
        } else {
            (ix(ramp, s.min(7)), hollow as u8)
        }
    })
}

fn brick(salt: u32) -> Tex {
    fill(|x, y| {
        let row = y / 4;
        let off = if row % 2 == 0 { 0 } else { 4 };
        let mortar = y % 4 == 3 || (x + off) % 8 == 7;
        if mortar {
            return (ix(GREY, 1 + noise(x, y, salt, 2) as u8), 2);
        }
        // OURS: damp grey-green stone (ed-slopper's reference), each block a shade of its own
        let b = noise((x + off) / 8, row, salt, 3) as u8;
        let n = noise(x, y, salt + 1, 8);
        let s = 2 + b + if n == 0 { 1 } else { 0 };
        // worn blocks: a few sit back a step
        let worn = noise((x + off) / 8, row, salt + 4, 5) == 0;
        (ix(GREY, s), worn as u8)
    })
}

fn cobble(salt: u32) -> Tex {
    let seeds: Vec<(i32, i32)> = (0..9).map(|i| ((h(i, 1, salt) % 16) as i32, (h(i, 2, salt) % 16) as i32)).collect();
    fill(|x, y| {
        let mut best = (i32::MAX, i32::MAX, 0usize);
        for (k, &(sx, sy)) in seeds.iter().enumerate() {
            for (ox, oy) in [(0, 0), (16, 0), (-16, 0), (0, 16), (0, -16)] {
                let d = (x - sx - ox).pow(2) + (y - sy - oy).pow(2);
                if d < best.0 {
                    best = (d, best.0, k);
                } else if d < best.1 {
                    best.1 = d;
                }
            }
        }
        let gap = (best.1 as f32).sqrt() - (best.0 as f32).sqrt();
        if gap < 1.2 {
            (ix(GREY, 1), 2)
        } else {
            (ix(GREY, 3 + (best.2 % 3) as u8 + if noise(x, y, salt, 9) == 0 { 1 } else { 0 }), 0)
        }
    })
}

fn dirt(salt: u32) -> Tex {
    fill(|x, y| {
        let n = noise(x, y, salt, 12);
        match n {
            0 => (ix(BROWN, 1), 0),
            1..=2 => (ix(BROWN, 4), 0),
            3 => (ix(GREY, 3), 0),
            // hollows in the earth, a few texels across
            _ => (ix(BROWN, 2 + noise(x / 3, y / 3, salt + 9, 2) as u8), (noise(x / 4, y / 4, salt + 11, 5) == 0) as u8),
        }
    })
}

fn moss(salt: u32) -> Tex {
    let mut t = rough(salt, GREY, 2);
    for (y, row) in t.iter_mut().enumerate() {
        for (x, texel) in row.iter_mut().enumerate() {
            let (x, y) = (x as i32, y as i32);
            let m = noise(x / 2, y / 2, salt + 5, 4) != 0 || noise(x, y, salt + 6, 3) == 0;
            if m {
                *texel = Texel { idx: ix(GREEN, 2 + noise(x, y, salt + 8, 4) as u8), depth: 0 };
            }
        }
    }
    t
}

fn planks(salt: u32, vertical: bool) -> Tex {
    fill(|x, y| {
        let (a, b) = if vertical { (x, y) } else { (y, x) };
        let plank = a / 4;
        if a % 4 == 3 {
            return (ix(BROWN, 1), 2);
        }
        if b % 8 == (plank * 3) % 8 && a % 4 == 1 && noise(plank, 0, salt, 2) == 0 {
            return (ix(GREY, 5), 0);
        }
        let grain = noise(a, b / 3, salt + plank as u32, 5) == 0;
        (ix(BROWN, 3 + noise(plank, 1, salt, 2) as u8 - grain as u8), grain as u8)
    })
}

fn marble(salt: u32) -> Tex {
    fill(|x, y| {
        let v = ((x as f32 * 0.6 + (y as f32 * 0.35).sin() * 3.0 + noise(x, y, salt, 3) as f32) as i32).rem_euclid(9);
        if x % 8 == 7 || y % 8 == 7 {
            (ix(BONE, 3), 2)
        } else if v == 0 {
            (ix(GREY, 4), 0)
        } else {
            (ix(BONE, 6 + (noise(x, y, salt + 1, 6) == 0) as u8), 0)
        }
    })
}

fn bone(salt: u32) -> Tex {
    fill(|x, y| {
        let (cx, cy) = (x % 8, y % 8);
        let skull = (2..=5).contains(&cx) && (2..=5).contains(&cy) && noise(x / 8, y / 8, salt, 2) == 0;
        if skull {
            let eye = (cx == 3 || cx == 4) && cy == 3;
            return if eye { (ix(BROWN, 0), 2) } else { (ix(BONE, 6), 0) };
        }
        if cx == 7 || cy == 7 {
            return (ix(BONE, 2), 2);
        }
        (ix(BONE, 4 + noise(x, y, salt + 2, 2) as u8), 1)
    })
}

fn obsidian(salt: u32) -> Tex {
    fill(|x, y| {
        let n = noise(x, y, salt, 30);
        let seam = (x + y) % 11 == 0 && noise(x, 0, salt, 2) == 0;
        if n == 0 {
            (ix(VIOLET, 5), 0)
        } else if seam {
            (ix(VIOLET, 2), 2)
        } else {
            (ix(GREY, noise(x / 3, y / 3, salt + 1, 2) as u8), 0)
        }
    })
}

fn crystal(salt: u32) -> Tex {
    fill(|x, y| {
        let f = ((x + 2 * y) / 3 + noise(x / 4, y / 4, salt, 4) as i32) % 4;
        if noise(x, y, salt + 1, 7) == 0 {
            (ix(VIOLET, 2), 2)
        } else {
            (ix(VIOLET, 4 + f as u8), (f % 2) as u8)
        }
    })
}

fn liquid(ramp: u8, first: u8, len: u8, salt: u32) -> Tex {
    fill(|x, y| {
        let w = ((x as f32 * 0.5 + y as f32 * 0.9 + ((x as f32) * 0.7).sin() * 1.5) as i32 + noise(x / 4, y / 4, salt, 2) as i32).rem_euclid(len as i32);
        (ix(ramp, first + w as u8), 0)
    })
}

/// The texture of a face.
pub fn texture(cell: Cell, side: bool) -> Tex {
    let s = cell as u32 * 101 + side as u32;
    match cell {
        Cell::Air => fill(|_, _| (0, 0)),
        Cell::Water => liquid(BLUE, 2, 6, s),
        Cell::Lava => liquid(FIRE, 1, 7, s),
        Cell::Bedrock => rough(s, GREY, 0),
        Cell::Stone => blocks(s, GREY, 2),
        Cell::Brick => {
            if side {
                brick(s)
            } else {
                cobble(s)
            }
        }
        Cell::Cobble => cobble(s),
        Cell::Dirt => dirt(s),
        Cell::Rock => {
            if side {
                // OURS: cave rock is grey, worn into soft lumps
                rough(s, GREY, 2)
            } else {
                dirt(s)
            }
        }
        Cell::Moss => moss(s),
        Cell::Wood => planks(s, side),
        Cell::Marble => {
            if side {
                blocks(s, BONE, 4)
            } else {
                marble(s)
            }
        }
        Cell::Bone => bone(s),
        Cell::Obsidian => obsidian(s),
        Cell::Crystal => crystal(s),
    }
}

/// Every tile's texture, by atlas slot.
pub fn all_textures() -> Vec<Tex> {
    let mut v = vec![[[Texel::default(); TILE]; TILE]; Cell::COUNT * 2];
    for cell in Cell::ALL {
        for side in [false, true] {
            v[tile_of(cell, side) as usize] = texture(cell, side);
        }
    }
    v
}

/// The atlas as palette indices, one byte per texel: (width, height, bytes).
pub fn atlas_indices() -> (usize, usize, Vec<u8>) {
    let tiles = Cell::COUNT * 2;
    let rows = tiles.div_ceil(TILES_PER_ROW);
    let (w, hgt) = (TILES_PER_ROW * TILE, rows * TILE);
    let mut data = vec![0u8; w * hgt];
    for (t, tex) in all_textures().iter().enumerate() {
        let (ox, oy) = ((t % TILES_PER_ROW) * TILE, (t / TILES_PER_ROW) * TILE);
        for y in 0..TILE {
            for x in 0..TILE {
                data[(oy + y) * w + ox + x] = tex[y][x].idx;
            }
        }
    }
    (w, hgt, data)
}

//! Tile textures (board row C8), made in code: 16×16 palette indices per cell kind, a top and a side each,
//! packed into one atlas. All ours (F8: 16 texels per tile edge, our own look).

use crate::palette::{ix, BLUE, BONE, BROWN, FIRE, GREEN, GREY, RED, VIOLET};
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use lac_sim::Cell;

pub const TILE: usize = 16;
pub const TILES_PER_ROW: usize = 16;

/// The atlas slot of a cell's face: top (and bottom) or side.
pub fn tile_of(cell: Cell, side: bool) -> u32 {
    cell as u32 * 2 + side as u32
}

/// A small deterministic hash for texture noise (the app may use any; this keeps textures the same each run).
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

type Tex = [[u8; TILE]; TILE];

fn fill(f: impl Fn(i32, i32) -> u8) -> Tex {
    let mut t = [[0u8; TILE]; TILE];
    for (y, row) in t.iter_mut().enumerate() {
        for (x, v) in row.iter_mut().enumerate() {
            *v = f(x as i32, y as i32);
        }
    }
    t
}

fn stone(salt: u32, ramp: u8, base: u8) -> Tex {
    fill(|x, y| {
        let n = noise(x, y, salt, 10);
        let block = noise(x / 4, y / 4, salt + 7, 3) as u8;
        let mut s = base + block;
        if n == 0 {
            s = s.saturating_sub(1);
        }
        if n == 9 {
            s += 1;
        }
        // cracks
        if noise(x / 2, y, salt + 3, 23) == 0 {
            s = base.saturating_sub(1);
        }
        ix(ramp, s.min(7))
    })
}

fn brick(salt: u32) -> Tex {
    fill(|x, y| {
        let row = y / 4;
        let off = if row % 2 == 0 { 0 } else { 4 };
        let mortar = y % 4 == 3 || (x + off) % 8 == 7;
        if mortar {
            return ix(GREY, 2 + noise(x, y, salt, 2) as u8);
        }
        let b = noise((x + off) / 8, row, salt, 3) as u8;
        let n = noise(x, y, salt + 1, 8);
        let s = 2 + b + if n == 0 { 1 } else { 0 };
        ix(RED, s)
    })
}

fn cobble(salt: u32) -> Tex {
    // round stones on a dark bed: nearest of a few seeds
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
        let edge = (best.1 as f32).sqrt() - (best.0 as f32).sqrt() < 1.2;
        if edge {
            ix(GREY, 1)
        } else {
            ix(GREY, 3 + (best.2 % 3) as u8 + if noise(x, y, salt, 9) == 0 { 1 } else { 0 })
        }
    })
}

fn dirt(salt: u32) -> Tex {
    fill(|x, y| {
        let n = noise(x, y, salt, 12);
        let s = match n {
            0 => 1,
            1..=2 => 4,
            3 => ix(GREY, 3) % 8,
            _ => 2 + noise(x / 3, y / 3, salt + 9, 2) as u8,
        } as u8;
        if n == 3 {
            ix(GREY, 3)
        } else {
            ix(BROWN, s)
        }
    })
}

fn moss(salt: u32) -> Tex {
    let base = stone(salt, GREY, 2);
    let mut t = base;
    for y in 0..TILE {
        for x in 0..TILE {
            let m = noise(x as i32 / 2, y as i32 / 2, salt + 5, 4) != 0 || noise(x as i32, y as i32, salt + 6, 3) == 0;
            if m {
                t[y][x] = ix(GREEN, 2 + noise(x as i32, y as i32, salt + 8, 4) as u8);
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
            return ix(BROWN, 1);
        }
        // nails and grain
        if b % 8 == (plank * 3) % 8 && a % 4 == 1 && noise(plank, 0, salt, 2) == 0 {
            return ix(GREY, 5);
        }
        let grain = noise(a, b / 3, salt + plank as u32, 5) == 0;
        ix(BROWN, 3 + noise(plank, 1, salt, 2) as u8 - grain as u8)
    })
}

fn marble(salt: u32) -> Tex {
    fill(|x, y| {
        let v = ((x as f32 * 0.6 + (y as f32 * 0.35).sin() * 3.0 + noise(x, y, salt, 3) as f32) as i32).rem_euclid(9);
        if v == 0 {
            ix(GREY, 4)
        } else if x % 8 == 0 || y % 8 == 0 {
            ix(BONE, 4)
        } else {
            ix(BONE, 6 + (noise(x, y, salt + 1, 6) == 0) as u8)
        }
    })
}

fn bone(salt: u32) -> Tex {
    fill(|x, y| {
        // small skulls in a grid of bone tiles
        let (cx, cy) = (x % 8, y % 8);
        let skull = (2..=5).contains(&cx) && (2..=5).contains(&cy) && noise(x / 8, y / 8, salt, 2) == 0;
        if skull {
            let eye = (cx == 3 || cx == 4) && cy == 3;
            return if eye { ix(BROWN, 0) } else { ix(BONE, 6) };
        }
        if cx == 7 || cy == 7 {
            return ix(BONE, 2);
        }
        ix(BONE, 4 + noise(x, y, salt + 2, 2) as u8)
    })
}

fn obsidian(salt: u32) -> Tex {
    fill(|x, y| {
        let n = noise(x, y, salt, 30);
        if n == 0 {
            ix(VIOLET, 5)
        } else if (x + y) % 11 == 0 && noise(x, 0, salt, 2) == 0 {
            ix(VIOLET, 2)
        } else {
            ix(GREY, noise(x / 3, y / 3, salt + 1, 2) as u8)
        }
    })
}

fn crystal(salt: u32) -> Tex {
    fill(|x, y| {
        // facets in the cycling range, so they glimmer
        let f = ((x + 2 * y) / 3 + noise(x / 4, y / 4, salt, 4) as i32) % 4;
        if noise(x, y, salt + 1, 7) == 0 {
            ix(VIOLET, 2)
        } else {
            ix(VIOLET, 4 + f as u8)
        }
    })
}

fn liquid(ramp: u8, first: u8, len: u8, salt: u32) -> Tex {
    fill(|x, y| {
        // diagonal bands through the cycling range: cycling the palette makes them flow
        let w = ((x as f32 * 0.5 + y as f32 * 0.9 + ((x as f32) * 0.7).sin() * 1.5) as i32 + noise(x / 4, y / 4, salt, 2) as i32).rem_euclid(len as i32);
        ix(ramp, first + w as u8)
    })
}

/// The texture of a face.
fn texture(cell: Cell, side: bool) -> Tex {
    let s = cell as u32 * 101 + side as u32;
    match cell {
        Cell::Air => fill(|_, _| 0),
        Cell::Water => liquid(BLUE, 2, 6, s),
        Cell::Lava => liquid(FIRE, 1, 7, s),
        Cell::Bedrock => stone(s, GREY, 0),
        Cell::Stone => stone(s, GREY, 2),
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
                stone(s, BROWN, 1)
            } else {
                dirt(s)
            }
        }
        Cell::Moss => moss(s),
        Cell::Wood => planks(s, side),
        Cell::Marble => marble(s),
        Cell::Bone => bone(s),
        Cell::Obsidian => obsidian(s),
        Cell::Crystal => crystal(s),
    }
}

/// The atlas: every cell kind's top and side, as palette indices in an R8 texture.
pub fn atlas_image() -> Image {
    let tiles = Cell::COUNT * 2;
    let rows = tiles.div_ceil(TILES_PER_ROW);
    let (w, hgt) = (TILES_PER_ROW * TILE, rows * TILE);
    let mut data = vec![0u8; w * hgt];
    for cell in Cell::ALL {
        for side in [false, true] {
            let t = tile_of(cell, side) as usize;
            let (ox, oy) = ((t % TILES_PER_ROW) * TILE, (t / TILES_PER_ROW) * TILE);
            let tex = texture(cell, side);
            for y in 0..TILE {
                for x in 0..TILE {
                    data[(oy + y) * w + ox + x] = tex[y][x];
                }
            }
        }
    }
    let mut img = Image::new(
        Extent3d { width: w as u32, height: hgt as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::R8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = bevy::image::ImageSampler::nearest();
    img
}

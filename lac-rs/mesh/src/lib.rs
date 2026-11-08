//! `lac-mesh`: the tile textures and the voxel mesher of the dungeon (board rows E4, C8). Plain data in, plain
//! vertex arrays out; the app wraps them in Bevy meshes. Always built optimised (see the workspace profile).
//!
//! REF R40: blocky voxel dungeons. OURS: every exposed face of a cell is carved from its texture's depths at 16
//! voxels per tile edge: mortar, seams and cracks sit back from the face, with small side walls, so walls and
//! floors have real relief instead of a flat picture. Liquids are drawn as lowered surfaces.

pub mod textures;

use lac_sim::world::{World, CHUNK};
use lac_sim::IVec3;
use textures::{tile_of, Tex, TILE};

/// The palette's ramps and the index of a shade (the app's `palette` module adds the colours).
pub mod pal {
    pub const GREY: u8 = 0;
    pub const BROWN: u8 = 1;
    pub const RED: u8 = 2;
    pub const GREEN: u8 = 3;
    pub const BLUE: u8 = 4;
    pub const FIRE: u8 = 5;
    pub const VIOLET: u8 = 6;
    pub const BONE: u8 = 7;
    /// The palette index of `shade` (0 dark .. 7 light) in `ramp`.
    pub const fn ix(ramp: u8, shade: u8) -> u8 {
        ramp * 8 + shade
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct V3(f32, f32, f32);

impl std::ops::Add for V3 {
    type Output = V3;
    fn add(self, o: V3) -> V3 {
        V3(self.0 + o.0, self.1 + o.1, self.2 + o.2)
    }
}
impl std::ops::Sub for V3 {
    type Output = V3;
    fn sub(self, o: V3) -> V3 {
        V3(self.0 - o.0, self.1 - o.1, self.2 - o.2)
    }
}
impl std::ops::Mul<f32> for V3 {
    type Output = V3;
    fn mul(self, k: f32) -> V3 {
        V3(self.0 * k, self.1 * k, self.2 * k)
    }
}
impl V3 {
    fn of(v: IVec3) -> V3 {
        V3(v.x as f32, v.y as f32, v.z as f32)
    }
    fn cross(self, o: V3) -> V3 {
        V3(self.1 * o.2 - self.2 * o.1, self.2 * o.0 - self.0 * o.2, self.0 * o.1 - self.1 * o.0)
    }
    fn dot(self, o: V3) -> f32 {
        self.0 * o.0 + self.1 * o.1 + self.2 * o.2
    }
}

/// Vertex arrays: positions, normals, UVs (position on the face in tiles; fract gives the texel), colours
/// (r = atlas tile, g = shade), triangle indices.
#[derive(Default, Debug)]
pub struct MeshData {
    pub pos: Vec<[f32; 3]>,
    pub nrm: Vec<[f32; 3]>,
    pub uv: Vec<[f32; 2]>,
    pub col: Vec<[f32; 4]>,
    pub idx: Vec<u32>,
}

impl MeshData {
    fn quad(&mut self, c: [V3; 4], n: V3, uv: [[f32; 2]; 4], tile: u32, shade: f32) {
        let b = self.pos.len() as u32;
        for (p, t) in c.iter().zip(uv) {
            self.pos.push([p.0, p.1, p.2]);
            self.nrm.push([n.0, n.1, n.2]);
            self.uv.push(t);
            self.col.push([tile as f32, shade, 0.0, 1.0]);
        }
        // counter-clockwise seen from the side the normal points to
        if (c[1] - c[0]).cross(c[2] - c[0]).dot(n) > 0.0 {
            self.idx.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
        } else {
            self.idx.extend_from_slice(&[b, b + 2, b + 1, b, b + 3, b + 2]);
        }
    }
    pub fn is_empty(&self) -> bool {
        self.idx.is_empty()
    }
}

/// OURS: a fixed shade per face direction, for the old-school look: tops brightest, bottoms darkest.
const SHADE: [f32; 6] = [0.80, 0.80, 1.0, 0.55, 0.90, 0.70];
const LIQUID_TOP: f32 = 0.38;
const T: f32 = TILE as f32;

/// A face direction: outward normal n, and the face's u (texel x, rightwards seen from outside) and v (texel y,
/// downwards on walls) axes.
struct Frame {
    n: IVec3,
    u: IVec3,
    v: IVec3,
    shade: f32,
}

fn frames() -> [Frame; 6] {
    let f = |n: (i32, i32, i32), u: (i32, i32, i32), v: (i32, i32, i32), k: usize| Frame {
        n: IVec3::new(n.0, n.1, n.2),
        u: IVec3::new(u.0, u.1, u.2),
        v: IVec3::new(v.0, v.1, v.2),
        shade: SHADE[k],
    };
    [
        f((1, 0, 0), (0, 0, -1), (0, -1, 0), 0),
        f((-1, 0, 0), (0, 0, 1), (0, -1, 0), 1),
        f((0, 1, 0), (1, 0, 0), (0, 0, 1), 2),
        f((0, -1, 0), (1, 0, 0), (0, 0, 1), 3),
        f((0, 0, 1), (1, 0, 0), (0, -1, 0), 4),
        f((0, 0, -1), (-1, 0, 0), (0, -1, 0), 5),
    ]
}

fn shade_of(dir: V3) -> f32 {
    let i = if dir.0 > 0.5 {
        0
    } else if dir.0 < -0.5 {
        1
    } else if dir.1 > 0.5 {
        2
    } else if dir.1 < -0.5 {
        3
    } else if dir.2 > 0.5 {
        4
    } else {
        5
    };
    SHADE[i]
}

/// The mesh of one chunk, in world coordinates (cell (x, y, z) spans x..x+1 and so on).
pub fn chunk_mesh(world: &World, chunk: IVec3, tex: &[Tex]) -> MeshData {
    let mut m = MeshData::default();
    let base = IVec3::new(chunk.x * CHUNK, chunk.y * CHUNK, chunk.z * CHUNK);
    let frames = frames();
    for y in 0..CHUNK {
        for z in 0..CHUNK {
            for x in 0..CHUNK {
                let p = base + IVec3::new(x, y, z);
                let cell = world.get(p);
                if !cell.is_solid() || !world.in_bounds(p) {
                    continue;
                }
                for fr in &frames {
                    let out = p + fr.n;
                    if world.solid(out) || !world.in_bounds(out) {
                        continue;
                    }
                    face(&mut m, world, tex, p, fr);
                }
            }
        }
    }
    liquids(&mut m, world, base);
    m
}

/// How a border of a face meets its neighbour in the face's plane.
#[derive(Clone, Copy, PartialEq)]
enum Border {
    /// The wall goes on: compare with the neighbour's texels.
    Plane,
    /// An outside corner: the face ends in the open; its edge texels stay flush.
    Convex,
    /// An inside corner: another wall rises here; recesses get a side wall against it.
    Concave,
}

fn border(world: &World, p: IVec3, toward: IVec3, n: IVec3) -> Border {
    let nb = p + toward;
    if !world.solid(nb) {
        Border::Convex
    } else if !world.solid(nb + n) {
        Border::Plane
    } else {
        Border::Concave
    }
}

fn face(m: &mut MeshData, world: &World, tex: &[Tex], p: IVec3, fr: &Frame) {
    let side = fr.n.y == 0;
    let tile = tile_of(world.get(p), side);
    let t = &tex[tile as usize];
    let (n, u, v) = (V3::of(fr.n), V3::of(fr.u), V3::of(fr.v));
    // the corner of the face where texel (0, 0) starts
    let origin = V3::of(p) + V3(0.5, 0.5, 0.5) + n * 0.5 - u * 0.5 - v * 0.5;
    let at = |i: f32, j: f32, d: f32| origin + u * (i / T) + v * (j / T) - n * (d / T);
    // depths, with outside corners kept flush
    let b_lo_u = border(world, p, IVec3::ZERO - fr.u, fr.n);
    let b_hi_u = border(world, p, fr.u, fr.n);
    let b_lo_v = border(world, p, IVec3::ZERO - fr.v, fr.n);
    let b_hi_v = border(world, p, fr.v, fr.n);
    let mut d = [[0u8; TILE]; TILE]; // d[i][j]
    for (i, col) in d.iter_mut().enumerate() {
        for (j, v) in col.iter_mut().enumerate() {
            let flush = (i == 0 && b_lo_u == Border::Convex)
                || (i == TILE - 1 && b_hi_u == Border::Convex)
                || (j == 0 && b_lo_v == Border::Convex)
                || (j == TILE - 1 && b_hi_v == Border::Convex);
            // OURS: ceilings stay flat; nobody looks up long enough to pay for their relief
            *v = if flush || fr.n.y < 0 { 0 } else { t[j][i].depth };
        }
    }
    // the faces of each depth, merged into rectangles
    for level in 0..=textures::MAX_DEPTH {
        let mut mask = [[false; TILE]; TILE];
        for i in 0..TILE {
            for j in 0..TILE {
                mask[i][j] = d[i][j] == level;
            }
        }
        let shade = fr.shade * (1.0 - 0.12 * level as f32);
        for i in 0..TILE {
            let mut j = 0;
            while j < TILE {
                if !mask[i][j] {
                    j += 1;
                    continue;
                }
                let mut hgt = 1;
                while j + hgt < TILE && mask[i][j + hgt] {
                    hgt += 1;
                }
                let mut w = 1;
                'grow: while i + w < TILE {
                    for k in 0..hgt {
                        if !mask[i + w][j + k] {
                            break 'grow;
                        }
                    }
                    w += 1;
                }
                for a in 0..w {
                    for k in 0..hgt {
                        mask[i + a][j + k] = false;
                    }
                }
                let (i0, i1, j0, j1, lv) = (i as f32, (i + w) as f32, j as f32, (j + hgt) as f32, level as f32);
                m.quad(
                    [at(i0, j0, lv), at(i1, j0, lv), at(i1, j1, lv), at(i0, j1, lv)],
                    n,
                    [[i0 / T, j0 / T], [i1 / T, j0 / T], [i1 / T, j1 / T], [i0 / T, j1 / T]],
                    tile,
                    shade,
                );
                j += hgt;
            }
        }
    }
    // side walls of the steps: between texels, and against the neighbours at the borders
    // along u: a wall at u = k between column k-1 (left) and column k (right)
    let neighbour_depth = |toward: IVec3, b: Border, i: usize, j: usize| -> u8 {
        match b {
            Border::Plane => {
                let q = p + toward;
                let nt = &tex[tile_of(world.get(q), side) as usize];
                nt[j][i].depth
            }
            _ => 0,
        }
    };
    for k in 0..=TILE {
        // depths left and right of the line u = k, for each j
        let pair = |j: usize| -> (u8, u8) {
            let left = if k == 0 { neighbour_depth(IVec3::ZERO - fr.u, b_lo_u, TILE - 1, j) } else { d[k - 1][j] };
            let right = if k == TILE { neighbour_depth(fr.u, b_hi_u, 0, j) } else { d[k][j] };
            (left, right)
        };
        let mut j = 0;
        while j < TILE {
            let (l, r) = pair(j);
            // at the borders only the side that is ours draws its own wall
            let ours = if k == 0 { r > l } else if k == TILE { l > r } else { l != r };
            if !ours {
                j += 1;
                continue;
            }
            let mut j1 = j + 1;
            while j1 < TILE && pair(j1) == (l, r) {
                j1 += 1;
            }
            let (lo, hi) = (l.min(r) as f32, l.max(r) as f32);
            // the wall faces the deeper side; it shows the colour of the shallower texel
            let (normal, us) = if r > l { (u, (k as f32 - 0.5).max(0.5) / T) } else { (u * -1.0, (k as f32 + 0.5).min(T - 0.5) / T) };
            let kf = k as f32;
            let (a, b) = (j as f32, j1 as f32);
            m.quad([at(kf, a, lo), at(kf, b, lo), at(kf, b, hi), at(kf, a, hi)], normal, [[us, a / T], [us, b / T], [us, b / T], [us, a / T]], tile, shade_of(normal) * 0.85);
            j = j1;
        }
    }
    // along v: a wall at v = k between row k-1 (above) and row k (below)
    for k in 0..=TILE {
        let pair = |i: usize| -> (u8, u8) {
            let above = if k == 0 { neighbour_depth(IVec3::ZERO - fr.v, b_lo_v, i, TILE - 1) } else { d[i][k - 1] };
            let below = if k == TILE { neighbour_depth(fr.v, b_hi_v, i, 0) } else { d[i][k] };
            (above, below)
        };
        let mut i = 0;
        while i < TILE {
            let (a, b) = pair(i);
            let ours = if k == 0 { b > a } else if k == TILE { a > b } else { a != b };
            if !ours {
                i += 1;
                continue;
            }
            let mut i1 = i + 1;
            while i1 < TILE && pair(i1) == (a, b) {
                i1 += 1;
            }
            let (lo, hi) = (a.min(b) as f32, a.max(b) as f32);
            let (normal, vs) = if b > a { (v, (k as f32 - 0.5).max(0.5) / T) } else { (v * -1.0, (k as f32 + 0.5).min(T - 0.5) / T) };
            let kf = k as f32;
            let (s0, s1) = (i as f32, i1 as f32);
            m.quad([at(s0, kf, lo), at(s1, kf, lo), at(s1, kf, hi), at(s0, kf, hi)], normal, [[s0 / T, vs], [s1 / T, vs], [s1 / T, vs], [s0 / T, vs]], tile, shade_of(normal) * 0.85);
            i = i1;
        }
    }
}

fn liquids(m: &mut MeshData, world: &World, base: IVec3) {
    for y in 0..CHUNK {
        for z in 0..CHUNK {
            for x in 0..CHUNK {
                let p = base + IVec3::new(x, y, z);
                let c = world.get(p);
                if !c.is_liquid() || !world.in_bounds(p) {
                    continue;
                }
                let full = world.get(p.up()).is_liquid();
                let top = if full { 1.0 } else { LIQUID_TOP };
                let t = tile_of(c, false);
                let o = V3::of(p);
                if !full {
                    let y0 = o + V3(0.0, top, 0.0);
                    m.quad([y0, y0 + V3(1.0, 0.0, 0.0), y0 + V3(1.0, 0.0, 1.0), y0 + V3(0.0, 0.0, 1.0)], V3(0.0, 1.0, 0.0), [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]], t, 1.0);
                }
                for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let nb = world.get(p + IVec3::new(dx, 0, dz));
                    if nb.is_solid() || nb.is_liquid() {
                        continue;
                    }
                    let nrm = V3(dx as f32, 0.0, dz as f32);
                    let (corner, along) = match (dx, dz) {
                        (1, 0) => (o + V3(1.0, 0.0, 0.0), V3(0.0, 0.0, 1.0)),
                        (-1, 0) => (o, V3(0.0, 0.0, 1.0)),
                        (0, 1) => (o + V3(0.0, 0.0, 1.0), V3(1.0, 0.0, 0.0)),
                        _ => (o, V3(1.0, 0.0, 0.0)),
                    };
                    let up = V3(0.0, top, 0.0);
                    m.quad([corner, corner + along, corner + along + up, corner + up], nrm, [[0.0, top], [1.0, top], [1.0, 0.0], [0.0, 0.0]], tile_of(c, true), 0.85);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_test_level_meshes_with_relief() {
        let l = lac_sim::Level::test_level();
        let tex = textures::all_textures();
        let mut quads = 0;
        for k in l.world.chunk_keys() {
            let m = chunk_mesh(&l.world, k, &tex);
            assert_eq!(m.idx.len() % 6, 0);
            assert!(m.idx.iter().all(|&i| (i as usize) < m.pos.len()));
            quads += m.idx.len() / 6;
        }
        // a flat mesher would make a few hundred quads here; the relief makes many more
        assert!(quads > 2_000, "only {quads} quads");
        assert!(quads < 400_000, "{quads} quads is too many");
    }
}

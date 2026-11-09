//! Voxel models for monsters, things in the dungeon and items on the floor (board row C9). All our own
//! designs, built in code from boxes of palette colours. REF R40: voxel monsters. REF R41: a zombie-like
//! walker; R42: a giant spider and a ghost-like figure (our shambler, cave spider and tomb wisp).

use crate::palette::{self, ix, Vision, BLUE, BONE, BROWN, FIRE, GREEN, GREY, RED, VIOLET};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use std::collections::BTreeMap;

/// Voxels per tile edge for models. OURS: 16, as the textures (F8).
pub const VOX: f32 = 1.0 / 16.0;

/// A box of voxels; 0 is empty, otherwise palette index + 1.
#[derive(Clone)]
pub struct VoxModel {
    pub w: i32,
    pub h: i32,
    pub d: i32,
    v: Vec<u8>,
    /// Indices drawn without shading (eyes, flames).
    pub lit: Vec<u8>,
}

impl VoxModel {
    pub fn new(w: i32, h: i32, d: i32) -> Self {
        VoxModel { w, h, d, v: vec![0; (w * h * d) as usize], lit: Vec::new() }
    }
    fn i(&self, x: i32, y: i32, z: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && z >= 0 && x < self.w && y < self.h && z < self.d).then(|| (x + self.w * (z + self.d * y)) as usize)
    }
    pub fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        self.i(x, y, z).map_or(0, |i| self.v[i])
    }
    pub fn set(&mut self, x: i32, y: i32, z: i32, c: u8) {
        if let Some(i) = self.i(x, y, z) {
            self.v[i] = c + 1;
        }
    }
    pub fn clear(&mut self, x: i32, y: i32, z: i32) {
        if let Some(i) = self.i(x, y, z) {
            self.v[i] = 0;
        }
    }
    /// Fill the box x0..x1, y0..y1, z0..z1 (ends exclusive).
    pub fn fill(&mut self, x0: i32, x1: i32, y0: i32, y1: i32, z0: i32, z1: i32, c: u8) -> &mut Self {
        for y in y0..y1 {
            for z in z0..z1 {
                for x in x0..x1 {
                    self.set(x, y, z, c);
                }
            }
        }
        self
    }
    /// Mirror the left half onto the right (models are built facing -z, symmetric in x).
    pub fn mirror_x(&mut self) -> &mut Self {
        for y in 0..self.h {
            for z in 0..self.d {
                for x in 0..self.w / 2 {
                    let a = self.get(x, y, z);
                    if a != 0 {
                        if let Some(i) = self.i(self.w - 1 - x, y, z) {
                            self.v[i] = a;
                        }
                    }
                }
            }
        }
        self
    }
    /// Noise in a colour: some voxels one shade darker or lighter, so surfaces aren't flat.
    pub fn speckle(&mut self, salt: u32) -> &mut Self {
        for (k, v) in self.v.iter_mut().enumerate() {
            if *v == 0 {
                continue;
            }
            let c = *v - 1;
            let hsh = (k as u32 ^ salt).wrapping_mul(2_654_435_761) >> 27;
            let shade = c % 8;
            let ramp = c / 8;
            let ns = match hsh {
                0..=2 => shade.saturating_sub(1),
                3 => (shade + 1).min(7),
                _ => shade,
            };
            *v = ramp * 8 + ns + 1;
        }
        self
    }

    /// The smoothed outward normal at a voxel corner (from the filled voxels within two of it) and how many of the
    /// eight voxels touching the corner are filled.
    fn corner_shape(&self, c: IVec3) -> (Vec3, u32) {
        let mut g = Vec3::ZERO;
        let mut occ = 0;
        for dz in -2..2 {
            for dy in -2..2 {
                for dx in -2..2 {
                    let v = c + IVec3::new(dx, dy, dz);
                    if self.get(v.x, v.y, v.z) == 0 {
                        continue;
                    }
                    let d = Vec3::new(dx as f32 + 0.5, dy as f32 + 0.5, dz as f32 + 0.5);
                    g += d / d.length_squared();
                    if (-1..1).contains(&dx) && (-1..1).contains(&dy) && (-1..1).contains(&dz) {
                        occ += 1;
                    }
                }
            }
        }
        (-g.normalize_or_zero(), occ)
    }

    /// Fill a ball (an ellipsoid with radii r) centred at c.
    pub fn ball(&mut self, c: Vec3, r: Vec3, col: u8) -> &mut Self {
        for y in (c.y - r.y).floor() as i32..=(c.y + r.y).ceil() as i32 {
            for z in (c.z - r.z).floor() as i32..=(c.z + r.z).ceil() as i32 {
                for x in (c.x - r.x).floor() as i32..=(c.x + r.x).ceil() as i32 {
                    let d = (Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5) - c) / r;
                    if d.length_squared() <= 1.0 {
                        self.set(x, y, z, col);
                    }
                }
            }
        }
        self
    }

    /// A thick line from a to b, `r` voxels in radius.
    pub fn limb(&mut self, a: Vec3, b: Vec3, r: f32, col: u8) -> &mut Self {
        let steps = ((b - a).length() * 2.0).ceil().max(1.0) as i32;
        for i in 0..=steps {
            let p = a.lerp(b, i as f32 / steps as f32);
            self.ball(p, Vec3::splat(r), col);
        }
        self
    }

    /// The mesh, centred on x and z, standing on y = 0, `scale` tiles per voxel times VOX.
    pub fn mesh(&self, vision: Vision, scale: f32) -> Mesh {
        let cols = palette::colours(vision);
        let mut pos = Vec::new();
        let mut nrm = Vec::new();
        let mut col = Vec::new();
        let mut idx = Vec::new();
        let s = VOX * scale;
        let off = Vec3::new(self.w as f32 / 2.0, 0.0, self.d as f32 / 2.0);
        let faces: [(IVec3, [Vec3; 4], f32); 6] = [
            (IVec3::X, [Vec3::new(1., 0., 1.), Vec3::new(1., 0., 0.), Vec3::new(1., 1., 0.), Vec3::new(1., 1., 1.)], 0.8),
            (IVec3::NEG_X, [Vec3::new(0., 0., 0.), Vec3::new(0., 0., 1.), Vec3::new(0., 1., 1.), Vec3::new(0., 1., 0.)], 0.8),
            (IVec3::Y, [Vec3::new(0., 1., 1.), Vec3::new(1., 1., 1.), Vec3::new(1., 1., 0.), Vec3::new(0., 1., 0.)], 1.0),
            (IVec3::NEG_Y, [Vec3::new(0., 0., 0.), Vec3::new(1., 0., 0.), Vec3::new(1., 0., 1.), Vec3::new(0., 0., 1.)], 0.55),
            (IVec3::Z, [Vec3::new(0., 0., 1.), Vec3::new(1., 0., 1.), Vec3::new(1., 1., 1.), Vec3::new(0., 1., 1.)], 0.9),
            (IVec3::NEG_Z, [Vec3::new(1., 0., 0.), Vec3::new(0., 0., 0.), Vec3::new(0., 1., 0.), Vec3::new(1., 1., 0.)], 0.7),
        ];
        for y in 0..self.h {
            for z in 0..self.d {
                for x in 0..self.w {
                    let v = self.get(x, y, z);
                    if v == 0 {
                        continue;
                    }
                    let ci = v - 1;
                    let c = cols[ci as usize];
                    let lit = self.lit.contains(&ci);
                    for (n, corners, shade) in faces.iter() {
                        if self.get(x + n.x, y + n.y, z + n.z) != 0 {
                            continue;
                        }
                        let b = pos.len() as u32;
                        // OURS: soft shading (ed-slopper's reference): each corner's normal leans toward the
                        // rounded shape around it and hollows darken, so the blocky models read as smooth forms
                        let sh = if lit { 1.0 } else { 0.5 + 0.5 * *shade };
                        for k in corners {
                            let corner = IVec3::new(x, y, z) + k.as_ivec3();
                            let (smooth, occ) = self.corner_shape(corner);
                            let fnorm = n.as_vec3();
                            let mut nn = (fnorm * 0.35 + smooth * 0.65).normalize_or(fnorm);
                            if nn.dot(fnorm) < 0.2 {
                                nn = fnorm;
                            }
                            let ao = if lit { 1.0 } else { (1.0 - 0.09 * (occ as f32 - 4.0)).clamp(0.62, 1.08) };
                            let mut lin = Color::srgb(c[0] * sh * ao, c[1] * sh * ao, c[2] * sh * ao).to_linear();
                            if lit {
                                // eyes and flames glow: brighter than white, so they shine and the bloom catches them
                                lin = LinearRgba::rgb(lin.red * 6.0, lin.green * 6.0, lin.blue * 6.0);
                            }
                            let p = (Vec3::new(x as f32, y as f32, z as f32) + *k - off) * s;
                            pos.push(p.to_array());
                            nrm.push(nn.to_array());
                            col.push([lin.red, lin.green, lin.blue, 1.0]);
                        }
                        idx.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
                    }
                }
            }
        }
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nrm)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
            .with_inserted_indices(Indices::U32(idx))
    }
}

// ------------------------------------------------------------------ monsters (all face -z)

fn shambler() -> VoxModel {
    // a stooped walker in rags, arms reaching forward
    let mut m = VoxModel::new(10, 15, 10);
    let skin = ix(GREEN, 3);
    let rag = ix(BROWN, 2);
    m.fill(3, 5, 0, 6, 4, 6, rag); // left leg
    m.fill(3, 5, 0, 1, 3, 6, ix(BROWN, 1)); // foot
    m.fill(2, 5, 6, 11, 3, 7, rag); // torso (left half)
    m.fill(2, 5, 9, 10, 3, 7, ix(BROWN, 3));
    m.fill(1, 2, 8, 10, 1, 5, skin); // shoulder
    m.fill(1, 2, 9, 10, 0, 4, skin); // arm forward
    m.fill(3, 5, 11, 15, 2, 6, skin); // head
    m.mirror_x();
    m.set(3, 13, 2, ix(FIRE, 6)); // eyes
    m.set(6, 13, 2, ix(FIRE, 6));
    m.fill(4, 6, 11, 12, 2, 3, ix(GREEN, 1)); // mouth
    m.clear(7, 6, 3); // tears in the rags
    m.clear(2, 7, 6);
    m.lit = vec![ix(FIRE, 6)];
    m.speckle(1);
    m
}

fn rat() -> VoxModel {
    let mut m = VoxModel::new(6, 5, 12);
    let fur = ix(BROWN, 2);
    m.fill(1, 5, 1, 4, 3, 10, fur);
    m.fill(2, 4, 1, 4, 1, 3, fur); // head
    m.fill(2, 4, 1, 2, 0, 1, ix(BONE, 5)); // nose
    m.fill(1, 2, 4, 5, 2, 3, ix(BONE, 4)); // ears
    m.fill(4, 5, 4, 5, 2, 3, ix(BONE, 4));
    m.fill(1, 2, 0, 1, 3, 4, fur);
    m.fill(4, 5, 0, 1, 3, 4, fur);
    m.fill(1, 2, 0, 1, 8, 9, fur);
    m.fill(4, 5, 0, 1, 8, 9, fur);
    m.fill(3, 4, 1, 2, 10, 12, ix(BONE, 3)); // tail
    m.set(2, 3, 1, ix(RED, 6));
    m.set(3, 3, 1, ix(RED, 6));
    m.lit = vec![ix(RED, 6)];
    m.speckle(2);
    m
}

fn spider(beetle: bool) -> VoxModel {
    let mut m = VoxModel::new(16, 7, 16);
    let (body, legs) = if beetle { (ix(BLUE, 2), ix(GREY, 1)) } else { (ix(GREY, 1), ix(BROWN, 1)) };
    // abdomen and head
    m.fill(5, 11, 2, 7, 7, 14, body);
    m.fill(6, 10, 2, 5, 3, 7, body);
    if beetle {
        m.fill(4, 12, 4, 6, 6, 14, ix(BLUE, 3));
        m.fill(7, 9, 2, 3, 1, 3, ix(GREY, 4)); // horn
    } else {
        m.fill(7, 9, 5, 6, 9, 12, ix(RED, 3)); // a mark on the back
        m.fill(6, 7, 1, 3, 2, 3, ix(BONE, 3)); // fangs
        m.fill(9, 10, 1, 3, 2, 3, ix(BONE, 3));
    }
    // legs: four each side, bent
    for (k, z) in [3, 6, 9, 12].iter().enumerate() {
        let reach = if k == 0 || k == 3 { 1 } else { 0 };
        m.fill(1 + reach, 6, 4, 5, *z, *z + 1, legs);
        m.fill(1 + reach, 2 + reach, 0, 4, *z, *z + 1, legs);
        m.fill(10, 15 - reach, 4, 5, *z, *z + 1, legs);
        m.fill(14 - reach, 15 - reach, 0, 4, *z, *z + 1, legs);
    }
    let eye = if beetle { ix(GREEN, 6) } else { ix(RED, 6) };
    for x in [6, 7, 8, 9] {
        m.set(x, 4, 2, eye);
    }
    m.lit = vec![eye];
    m.speckle(3);
    m
}

/// OURS: the cave spider, a giant of our own design: a heavy, bristled abdomen with a pale chevron, a low head
/// with a cluster of red eyes, and long jointed legs whose knees rise above the body. REF R42: a giant spider.
fn giant_spider() -> VoxModel {
    let mut m = VoxModel::new(30, 17, 30);
    let body = ix(BROWN, 1);
    let dark = ix(GREY, 0);
    let leg = ix(BROWN, 1);
    let joint = ix(GREY, 2);
    // abdomen, cephalothorax, head
    m.ball(Vec3::new(15.0, 8.0, 20.5), Vec3::new(6.5, 5.5, 7.5), body);
    m.ball(Vec3::new(15.0, 6.5, 10.5), Vec3::new(4.6, 3.6, 4.4), dark);
    m.ball(Vec3::new(15.0, 6.0, 6.0), Vec3::new(3.2, 2.8, 2.4), dark);
    // a pale chevron down the abdomen's back
    for k in 0..4 {
        let z = 16 + k * 3;
        for dx in 0..=(3 - k) {
            for x in [15 - dx, 14 + dx] {
                let top = (0..17).rev().find(|&y| m.get(x, y, z - dx) != 0);
                if let Some(y) = top {
                    m.set(x, y, z - dx, ix(BONE, 2));
                }
            }
        }
    }
    // fangs
    m.fill(12, 14, 2, 5, 3, 5, ix(BONE, 3));
    m.fill(16, 18, 2, 5, 3, 5, ix(BONE, 3));
    m.fill(12, 13, 1, 3, 3, 4, ix(BONE, 5));
    m.fill(17, 18, 1, 3, 3, 4, ix(BONE, 5));
    // legs: four each side, hip to a high knee to the floor
    for (k, (hz, kz, fz)) in [(8.0, 4.0, 0.5), (10.0, 9.0, 7.0), (12.0, 15.0, 17.0), (13.0, 21.0, 28.0)].iter().enumerate() {
        for side in [-1.0f32, 1.0] {
            let hip = Vec3::new(15.0 + side * 3.5, 7.0, *hz);
            let knee = Vec3::new(15.0 + side * (9.5 + k as f32 * 0.4), 15.5 - (k % 2) as f32, *kz);
            let foot = Vec3::new(15.0 + side * 13.8, 0.6, *fz);
            m.limb(hip, knee, 1.1, leg);
            m.limb(knee, foot, 0.8, leg);
            m.ball(knee, Vec3::splat(1.3), joint);
        }
    }
    // eyes: a cluster on the head's front
    let eye = ix(RED, 6);
    for (x, y) in [(13, 7), (14, 8), (15, 8), (16, 7), (14, 6), (15, 6)] {
        m.set(x, y, 3, eye);
    }
    m.set(12, 6, 4, eye);
    m.set(17, 6, 4, eye);
    m.lit = vec![eye];
    m.speckle(3);
    m
}

fn ooze() -> VoxModel {
    let mut m = VoxModel::new(12, 8, 12);
    let c = ix(GREY, 4);
    for y in 0..8 {
        let r = [6.0, 6.0, 5.6, 5.0, 4.4, 3.6, 2.6, 1.4][y as usize];
        for z in 0..12 {
            for x in 0..12 {
                let dx = x as f32 - 5.5;
                let dz = z as f32 - 5.5;
                if dx * dx + dz * dz <= r * r {
                    m.set(x, y, z, c);
                }
            }
        }
    }
    m.set(4, 5, 1, ix(BONE, 7));
    m.set(7, 5, 1, ix(BONE, 7));
    m.fill(3, 9, 1, 2, 0, 12, ix(GREY, 2));
    m.lit = vec![ix(BONE, 7)];
    m.speckle(4);
    m
}

fn wisp() -> VoxModel {
    // a hooded drift of pale cloth, narrowing to a tail, with burning eyes
    let mut m = VoxModel::new(10, 15, 8);
    let cloth = ix(VIOLET, 3);
    for y in 0..15 {
        let half = match y {
            0..=1 => 1,
            2..=4 => 2,
            5..=10 => 3,
            11..=13 => 3,
            _ => 2,
        };
        for x in 5 - half..5 + half {
            for z in 4 - (half - 1).max(1)..4 + (half - 1).max(1) {
                m.set(x, y, z, if y > 10 { ix(VIOLET, 2) } else { cloth });
            }
        }
    }
    m.fill(3, 7, 11, 13, 2, 3, ix(GREY, 0)); // the dark of the hood
    m.set(4, 12, 2, ix(VIOLET, 7));
    m.set(5, 12, 2, ix(VIOLET, 7));
    m.fill(1, 3, 7, 8, 3, 5, cloth); // sleeves
    m.fill(7, 9, 7, 8, 3, 5, cloth);
    m.lit = vec![ix(VIOLET, 7)];
    m.speckle(5);
    m
}

fn skeleton(bow: bool) -> VoxModel {
    let mut m = VoxModel::new(10, 15, 8);
    let b = ix(BONE, 5);
    m.fill(3, 4, 0, 6, 4, 5, b);
    m.fill(3, 5, 6, 7, 3, 5, b); // pelvis
    m.fill(4, 5, 7, 11, 4, 5, ix(BONE, 4)); // spine
    for y in [8, 10] {
        m.fill(2, 5, y, y + 1, 3, 5, b); // ribs
    }
    m.fill(1, 2, 7, 11, 3, 4, b); // arm
    m.fill(3, 5, 11, 15, 2, 6, ix(BONE, 6)); // skull
    m.mirror_x();
    m.set(3, 13, 2, ix(GREY, 0));
    m.set(6, 13, 2, ix(GREY, 0));
    m.fill(4, 6, 11, 12, 2, 3, ix(GREY, 1));
    if bow {
        m.fill(0, 1, 5, 13, 1, 2, ix(BROWN, 3));
        m.fill(0, 1, 4, 5, 2, 3, ix(BROWN, 3));
        m.fill(0, 1, 13, 14, 2, 3, ix(BROWN, 3));
    }
    m.speckle(6);
    m
}

fn ghoul() -> VoxModel {
    let mut m = VoxModel::new(10, 13, 10);
    let skin = ix(VIOLET, 2);
    m.fill(3, 5, 0, 5, 5, 7, skin);
    m.fill(2, 5, 5, 9, 3, 8, skin);
    m.fill(1, 2, 5, 9, 2, 4, skin); // long arms
    m.fill(1, 2, 3, 5, 1, 3, ix(BONE, 5)); // claws
    m.fill(3, 5, 8, 12, 1, 5, ix(VIOLET, 3)); // head pushed forward
    m.mirror_x();
    m.set(3, 10, 1, ix(GREEN, 6));
    m.set(6, 10, 1, ix(GREEN, 6));
    m.fill(3, 7, 8, 9, 1, 2, ix(BONE, 6)); // teeth
    m.lit = vec![ix(GREEN, 6)];
    m.speckle(7);
    m
}

fn knight(warden: bool) -> VoxModel {
    let mut m = VoxModel::new(12, 16, 9);
    let (plate, dark) = if warden { (ix(VIOLET, 1), ix(GREY, 0)) } else { (ix(GREY, 3), ix(GREY, 1)) };
    m.fill(3, 5, 0, 6, 3, 6, plate);
    m.fill(3, 5, 0, 1, 2, 6, dark);
    m.fill(2, 6, 6, 11, 2, 7, plate);
    m.fill(2, 6, 8, 9, 2, 7, dark); // belt
    m.fill(1, 2, 7, 11, 3, 6, plate);
    m.fill(0, 2, 10, 12, 2, 7, plate); // pauldron
    m.fill(3, 6, 11, 15, 2, 7, plate); // helm
    m.mirror_x();
    m.fill(4, 8, 13, 14, 2, 3, ix(GREY, 0)); // visor slit
    let eye = if warden { ix(VIOLET, 7) } else { ix(FIRE, 5) };
    m.set(4, 13, 1, eye);
    m.set(7, 13, 1, eye);
    // a sword held down at the right
    m.fill(11, 12, 1, 12, 2, 3, ix(GREY, 5));
    m.fill(10, 12, 9, 10, 1, 4, ix(BROWN, 2));
    if warden {
        // a crown of crystal
        for x in [3, 5, 6, 8] {
            m.fill(x, x + 1, 15, 16, 3, 6, ix(VIOLET, 6));
        }
        m.fill(3, 9, 14, 15, 2, 7, ix(FIRE, 6));
    }
    m.lit = vec![eye, ix(VIOLET, 6)];
    m.speckle(8);
    m
}

fn bat() -> VoxModel {
    // a small body between ragged wings, held up high
    let mut m = VoxModel::new(16, 16, 6);
    let skin = ix(VIOLET, 1);
    let wing = ix(VIOLET, 2);
    m.fill(6, 8, 9, 13, 2, 4, skin); // body (left half)
    m.fill(6, 8, 13, 15, 2, 4, skin); // head
    m.fill(6, 7, 15, 16, 2, 3, skin); // ear
    for (k, x) in (0..6).enumerate() {
        let top = 14 - (k as i32 / 2);
        let bottom = 10 + (k as i32 % 3);
        m.fill(x, x + 1, bottom, top, 2, 3, wing);
    }
    m.fill(1, 6, 13, 14, 2, 3, ix(VIOLET, 3)); // wing bone
    m.mirror_x();
    m.set(6, 14, 1, ix(RED, 6));
    m.set(9, 14, 1, ix(RED, 6));
    m.lit = vec![ix(RED, 6)];
    m.speckle(14);
    m
}

fn crow() -> VoxModel {
    let mut m = VoxModel::new(6, 8, 10);
    let black = ix(GREY, 0);
    m.fill(1, 5, 2, 6, 3, 8, black); // body
    m.fill(2, 4, 5, 8, 1, 4, black); // head
    m.fill(2, 4, 6, 7, 0, 1, ix(BONE, 3)); // beak
    m.fill(2, 4, 3, 5, 8, 10, ix(GREY, 1)); // tail
    m.fill(1, 2, 3, 6, 4, 8, ix(BLUE, 1)); // wings with a sheen
    m.fill(4, 5, 3, 6, 4, 8, ix(BLUE, 1));
    m.fill(2, 3, 0, 2, 5, 6, ix(BONE, 3)); // legs
    m.fill(3, 4, 0, 2, 5, 6, ix(BONE, 3));
    m.set(2, 7, 1, ix(FIRE, 6));
    m.set(3, 7, 1, ix(FIRE, 6));
    m.lit = vec![ix(FIRE, 6)];
    m.speckle(15);
    m
}

fn slime() -> VoxModel {
    let mut m = VoxModel::new(12, 10, 12);
    let c = ix(GREEN, 4);
    for y in 0..10 {
        let r = [6.0, 6.0, 5.8, 5.4, 5.0, 4.4, 3.8, 3.0, 2.2, 1.2][y as usize];
        for z in 0..12 {
            for x in 0..12 {
                let dx = x as f32 - 5.5;
                let dz = z as f32 - 5.5;
                if dx * dx + dz * dz <= r * r {
                    m.set(x, y, z, if (x + y + z) % 5 == 0 { ix(GREEN, 5) } else { c });
                }
            }
        }
    }
    // a wide mouth and two dark eyes on the front
    m.fill(3, 9, 2, 4, 0, 2, ix(GREEN, 0));
    for x in [4, 6, 8] {
        m.set(x, 3, 0, ix(BONE, 6));
    }
    m.fill(3, 5, 6, 8, 1, 2, ix(GREY, 0));
    m.fill(7, 9, 6, 8, 1, 2, ix(GREY, 0));
    m.speckle(16);
    m
}

fn wolf() -> VoxModel {
    let mut m = VoxModel::new(8, 11, 18);
    let fur = ix(GREY, 2);
    let dark = ix(GREY, 1);
    m.fill(2, 6, 4, 8, 4, 15, fur); // body
    m.fill(2, 6, 7, 8, 5, 14, dark); // back
    m.fill(2, 6, 5, 10, 1, 5, fur); // head
    m.fill(3, 5, 5, 7, 0, 2, ix(GREY, 3)); // muzzle
    m.fill(3, 5, 5, 6, 0, 1, ix(GREY, 0)); // nose
    m.fill(2, 3, 10, 11, 2, 3, dark); // ears
    m.fill(5, 6, 10, 11, 2, 3, dark);
    for z in [5, 13] {
        m.fill(2, 3, 0, 4, z, z + 2, fur); // legs
        m.fill(5, 6, 0, 4, z, z + 2, fur);
    }
    m.fill(3, 5, 6, 8, 15, 18, dark); // tail
    m.set(2, 8, 1, ix(RED, 6));
    m.set(5, 8, 1, ix(RED, 6));
    m.fill(3, 5, 4, 5, 1, 2, ix(BONE, 6)); // teeth
    m.lit = vec![ix(RED, 6)];
    m.speckle(17);
    m
}

pub fn chest_model(open: bool, mimic: bool) -> VoxModel {
    let mut m = VoxModel::new(14, 12, 10);
    let wood = ix(BROWN, 3);
    let band = ix(GREY, 1);
    m.fill(0, 14, 0, 7, 1, 10, wood);
    m.fill(0, 14, 0, 7, 3, 4, band);
    m.fill(0, 14, 0, 7, 7, 8, band);
    if open || mimic {
        // the lid stands up behind
        m.fill(0, 14, 7, 12, 9, 10, wood);
        m.fill(1, 13, 1, 7, 2, 9, ix(BROWN, 0)); // the dark inside
        m.fill(0, 14, 7, 12, 8, 9, band);
        if open && !mimic {
            m.fill(4, 10, 6, 7, 4, 7, ix(FIRE, 6)); // a glint of gold
        }
    } else {
        m.fill(0, 14, 7, 9, 1, 10, wood);
        m.fill(0, 14, 7, 9, 3, 4, band);
        m.fill(0, 14, 7, 9, 7, 8, band);
        m.fill(6, 8, 5, 8, 0, 1, ix(FIRE, 5)); // lock
    }
    if mimic {
        for x in (1..13).step_by(2) {
            m.set(x, 7, 1, ix(BONE, 7));
            m.set(x + 1, 6, 1, ix(BONE, 7));
        }
        m.fill(4, 10, 3, 5, 0, 3, ix(RED, 4)); // tongue
        m.set(3, 10, 8, ix(FIRE, 6));
        m.set(10, 10, 8, ix(FIRE, 6));
        m.lit = vec![ix(FIRE, 6)];
    }
    m.speckle(9);
    m
}

pub fn lever_model(on: bool) -> VoxModel {
    // built against a wall at +z, sticking out towards -z
    let mut m = VoxModel::new(6, 12, 6);
    m.fill(1, 5, 2, 10, 5, 6, ix(GREY, 2));
    m.fill(2, 4, 5, 7, 3, 5, ix(GREY, 4));
    if on {
        m.fill(2, 4, 1, 6, 1, 3, ix(BROWN, 3));
        m.fill(2, 4, 0, 2, 0, 1, ix(RED, 5));
    } else {
        m.fill(2, 4, 6, 11, 1, 3, ix(BROWN, 3));
        m.fill(2, 4, 10, 12, 0, 1, ix(RED, 5));
    }
    m.speckle(10);
    m
}

pub fn door_model(portcullis: bool, tall: bool) -> VoxModel {
    // spans the cell in x, thin in z; a tall door is two cells high with a rounded top, to fit its arch
    let h = if tall { 32 } else { 16 };
    let mut m = VoxModel::new(16, h, 2);
    if portcullis {
        for x in (1..16).step_by(3) {
            m.fill(x, x + 1, 0, h, 0, 2, ix(GREY, 3));
        }
        for y in (2..h).step_by(4) {
            m.fill(0, 16, y, y + 1, 0, 2, ix(GREY, 2));
        }
    } else {
        for x in 0..16 {
            let plank = x / 4;
            m.fill(x, x + 1, 0, h, 0, 2, ix(BROWN, 2 + (plank % 2) as u8));
        }
        for y in if tall { [4, 15, 25].as_slice() } else { [3, 12].as_slice() } {
            m.fill(0, 16, *y, *y + 1, 0, 2, ix(GREY, 3));
        }
        m.fill(12, 13, 9, 11, 0, 2, ix(FIRE, 5)); // handle
    }
    if tall {
        arch_cut(&mut m, 2);
    }
    m.speckle(11);
    m
}

/// Clear what lies outside the round top of a two-cell opening (the upper cell's semicircle).
fn arch_cut(m: &mut VoxModel, depth: i32) {
    for y in 16..32 {
        for x in 0..16 {
            let (dx, dy) = (x as f32 + 0.5 - 8.0, y as f32 + 0.5 - 16.0);
            if dx * dx + dy * dy > 64.0 {
                for z in 0..depth {
                    m.clear(x, y, z);
                }
            }
        }
    }
}

/// OURS: the stone arch over a two-high doorway: it fills the upper cell but for a round vault, its stones laid
/// as wedges around the curve, with deep joints (ed-slopper's reference: arched openings in the walls).
pub fn arch_model() -> VoxModel {
    let mut m = VoxModel::new(16, 16, 16);
    for y in 0..16 {
        for x in 0..16 {
            let (dx, dy) = (x as f32 + 0.5 - 8.0, y as f32 + 0.5);
            let r = (dx * dx + dy * dy).sqrt();
            if r <= 8.0 {
                continue;
            }
            // wedge stones round the curve, plain courses beyond
            let ang = dy.atan2(dx);
            let (joint, stone) = if r < 11.5 {
                let k = ang / std::f32::consts::PI * 9.0;
                ((k.fract() < 0.12) || (r > 11.0), k as i32)
            } else {
                (y % 5 == 4 || (x + if (y / 5) % 2 == 0 { 0 } else { 4 }) % 8 == 7, 20 + y / 5 * 3 + x / 8)
            };
            for z in 0..16 {
                let c = if joint { ix(GREY, 1) } else { ix(GREY, 2 + (stone.rem_euclid(2)) as u8) };
                m.set(x, y, z, c);
            }
        }
    }
    m.speckle(17);
    m
}

pub fn stairs_model() -> VoxModel {
    // steps going down, seen at the floor: a dark hole edged with stone steps
    let mut m = VoxModel::new(16, 3, 16);
    m.fill(0, 16, 0, 1, 0, 16, ix(GREY, 0));
    for k in 0..4 {
        m.fill(0, 16, 1, 2, k * 4, k * 4 + 2, ix(GREY, 3 - k.min(2) as u8));
    }
    m.fill(0, 1, 0, 3, 0, 16, ix(GREY, 4));
    m.fill(15, 16, 0, 3, 0, 16, ix(GREY, 4));
    m
}

pub fn plate_model(pressed: bool) -> VoxModel {
    let mut m = VoxModel::new(12, 1, 12);
    m.fill(0, 12, 0, 1, 0, 12, if pressed { ix(GREY, 1) } else { ix(GREY, 3) });
    m.fill(2, 10, 0, 1, 2, 10, ix(GREY, 2));
    m
}

pub fn spikes_model() -> VoxModel {
    let mut m = VoxModel::new(16, 7, 16);
    for (x, z) in [(2, 2), (7, 3), (12, 2), (4, 7), (10, 8), (2, 12), (7, 12), (13, 13)] {
        m.fill(x, x + 2, 0, 4, z, z + 2, ix(GREY, 4));
        m.fill(x, x + 1, 4, 7, z, z + 1, ix(GREY, 6));
    }
    m
}

pub fn fountain_model() -> VoxModel {
    let mut m = VoxModel::new(14, 12, 14);
    let stone = ix(GREY, 4);
    m.fill(0, 14, 0, 4, 0, 14, stone);
    m.fill(2, 12, 2, 4, 2, 12, ix(BLUE, 4));
    m.fill(6, 8, 0, 10, 6, 8, stone);
    m.fill(5, 9, 10, 11, 5, 9, stone);
    m.fill(6, 8, 11, 12, 6, 8, ix(BLUE, 6));
    m.speckle(12);
    m
}

pub fn tomb_model() -> VoxModel {
    let mut m = VoxModel::new(12, 14, 4);
    m.fill(0, 12, 0, 2, 0, 4, ix(GREY, 2));
    m.fill(1, 11, 2, 12, 1, 3, ix(GREY, 3));
    m.fill(3, 9, 12, 14, 1, 3, ix(GREY, 3));
    m.fill(3, 9, 6, 7, 0, 1, ix(GREY, 1)); // a worn name
    m.fill(4, 8, 8, 9, 0, 1, ix(GREY, 1));
    m.speckle(13);
    m
}

pub fn torch_model() -> VoxModel {
    // against a wall at +z: an iron bracket holding a wooden brand, burning at the top
    let mut m = VoxModel::new(5, 13, 6);
    m.fill(1, 4, 0, 8, 1, 4, ix(BROWN, 2)); // the brand
    m.fill(2, 3, 0, 8, 1, 2, ix(BROWN, 3));
    m.fill(0, 5, 3, 5, 0, 5, ix(GREY, 1)); // the bracket's ring
    m.fill(1, 4, 3, 5, 1, 4, ix(BROWN, 2));
    m.fill(2, 3, 2, 5, 4, 6, ix(GREY, 1)); // its arm to the wall
    m.fill(1, 4, 8, 9, 1, 4, ix(FIRE, 1)); // embers
    m.fill(1, 4, 9, 11, 1, 4, ix(FIRE, 5));
    m.fill(2, 3, 9, 12, 2, 3, ix(FIRE, 7));
    m.fill(2, 3, 12, 13, 2, 3, ix(FIRE, 6));
    m.lit = vec![ix(FIRE, 1), ix(FIRE, 5), ix(FIRE, 6), ix(FIRE, 7)];
    m
}

/// A floor item: its icon, extruded one voxel thick and laid flat.
pub fn item_model(icon: &crate::icons::Icon) -> VoxModel {
    let n = icon.size as i32;
    let mut m = VoxModel::new(n, 1, n);
    for y in 0..n {
        for x in 0..n {
            if let Some(c) = icon.px[(y * n + x) as usize] {
                m.set(x, 0, y, c);
            }
        }
    }
    m
}

/// OURS: monsters are built for one-unit tiles; the world is drawn taller and wider, so they grow with it.
pub const MONSTER_SCALE: f32 = 1.35;

/// The model of a monster kind, and its scale.
pub fn monster_model(model: &str) -> (VoxModel, f32) {
    match model {
        "shambler" => (shambler(), 1.0 * MONSTER_SCALE),
        "rat" => (rat(), 1.0 * MONSTER_SCALE),
        "spider" => (giant_spider(), 0.85 * MONSTER_SCALE),
        "beetle" => (spider(true), 1.2 * MONSTER_SCALE),
        "ooze" => (ooze(), 1.2 * MONSTER_SCALE),
        "wisp" => (wisp(), 1.0 * MONSTER_SCALE),
        "archer" => (skeleton(true), 1.0 * MONSTER_SCALE),
        "ghoul" => (ghoul(), 1.0 * MONSTER_SCALE),
        "knight" => (knight(false), 1.05 * MONSTER_SCALE),
        "mimic" => (chest_model(true, true), 1.0 * MONSTER_SCALE),
        "warden" => (knight(true), 1.5 * MONSTER_SCALE),
        "bat" => (bat(), 0.9 * MONSTER_SCALE),
        "crow" => (crow(), 1.0 * MONSTER_SCALE),
        "slime" => (slime(), 1.3 * MONSTER_SCALE),
        "wolf" => (wolf(), 1.15 * MONSTER_SCALE),
        _ => (shambler(), 1.0 * MONSTER_SCALE),
    }
}

/// Meshes made once and shared.
#[derive(Resource, Default)]
pub struct ModelCache {
    pub meshes: BTreeMap<String, Handle<Mesh>>,
    pub material: Handle<StandardMaterial>,
}

impl ModelCache {
    pub fn get(&mut self, key: &str, meshes: &mut Assets<Mesh>, vision: Vision, make: impl FnOnce() -> (VoxModel, f32)) -> Handle<Mesh> {
        if let Some(h) = self.meshes.get(key) {
            return h.clone();
        }
        let (m, scale) = make();
        let h = meshes.add(m.mesh(vision, scale));
        self.meshes.insert(key.to_string(), h.clone());
        h
    }
}

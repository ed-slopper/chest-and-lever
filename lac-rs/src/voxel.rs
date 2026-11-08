//! Voxel meshes for the dungeon (board row E4) and the palette material they use (E7).
//!
//! REF R40: the dungeon is voxels. One mesh per 16³ chunk: faces between solid cells and open ones only
//! (hidden faces culled), merged into rectangles of the same texture (greedy meshing). Liquids get their own
//! surfaces, lowered so a party standing in water sees it about its knees (OURS).

use crate::palette;
use crate::textures::{self, tile_of};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use lac_sim::world::{World, CHUNK};
use lac_sim::IVec3 as SIVec3;

pub type VoxelMaterial = ExtendedMaterial<StandardMaterial, PaletteExt>;

#[derive(ShaderType, Reflect, Debug, Clone, Default)]
pub struct PaletteSettings {
    pub cycles: [Vec4; 4],
    pub params: Vec4,
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct PaletteExt {
    #[uniform(100)]
    pub settings: PaletteSettings,
    #[texture(101)]
    pub atlas: Handle<Image>,
    #[texture(102)]
    pub palette: Handle<Image>,
}

impl MaterialExtension for PaletteExt {
    fn fragment_shader() -> ShaderRef {
        "shaders/palette.wesl".into()
    }
}

pub fn palette_settings() -> PaletteSettings {
    let mut cycles = [Vec4::ZERO; 4];
    for (i, (first, len, speed)) in palette::CYCLES.iter().enumerate() {
        cycles[i] = Vec4::new(*first as f32, *len as f32, *speed, 0.0);
    }
    PaletteSettings { cycles, params: Vec4::new(textures::TILES_PER_ROW as f32, textures::TILE as f32, 600.0, 0.0) }
}

/// The handles every chunk shares.
#[derive(Resource, Clone)]
pub struct VoxelAssets {
    pub material: Handle<VoxelMaterial>,
}

pub fn make_material(images: &mut Assets<Image>, materials: &mut Assets<VoxelMaterial>, vision: palette::Vision) -> VoxelAssets {
    let atlas = images.add(textures::atlas_image());
    let pal = images.add(palette::palette_image(vision));
    let material = materials.add(ExtendedMaterial {
        base: StandardMaterial { base_color: Color::WHITE, perceptual_roughness: 0.92, reflectance: 0.08, ..default() },
        extension: PaletteExt { settings: palette_settings(), atlas, palette: pal },
    });
    VoxelAssets { material }
}

#[derive(Default)]
struct MeshData {
    pos: Vec<[f32; 3]>,
    nrm: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    col: Vec<[f32; 4]>,
    idx: Vec<u32>,
}

impl MeshData {
    /// A quad from `o` spanning `u` and `v` (world vectors), facing `n`; UV in tiles.
    fn quad(&mut self, o: Vec3, u: Vec3, v: Vec3, n: Vec3, uw: f32, vh: f32, tile: u32, shade: f32) {
        let b = self.pos.len() as u32;
        let corners = [o, o + u, o + u + v, o + v];
        let uvs = [[0.0, vh], [uw, vh], [uw, 0.0], [0.0, 0.0]];
        for (c, t) in corners.iter().zip(uvs) {
            self.pos.push(c.to_array());
            self.nrm.push(n.to_array());
            self.uv.push(t);
            self.col.push([tile as f32, shade, 0.0, 1.0]);
        }
        // counter-clockwise seen from the side `n` points to
        if u.cross(v).dot(n) > 0.0 {
            self.idx.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
        } else {
            self.idx.extend_from_slice(&[b, b + 2, b + 1, b, b + 3, b + 2]);
        }
    }

    fn into_mesh(self) -> Option<Mesh> {
        if self.idx.is_empty() {
            return None;
        }
        Some(
            Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.pos)
                .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.nrm)
                .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uv)
                .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.col)
                .with_inserted_indices(Indices::U32(self.idx)),
        )
    }
}

/// OURS: a little fixed shading per face direction, for the old-school look: tops brightest, bottoms darkest.
const SHADE: [f32; 6] = [0.80, 0.80, 1.0, 0.55, 0.90, 0.70];
const LIQUID_TOP: f32 = 0.38;

fn sv(x: i32, y: i32, z: i32) -> SIVec3 {
    SIVec3::new(x, y, z)
}

/// The mesh of one chunk, in world coordinates (cell (x, y, z) spans x..x+1 and so on).
pub fn chunk_mesh(world: &World, chunk: SIVec3) -> Option<Mesh> {
    let mut m = MeshData::default();
    let base = sv(chunk.x * CHUNK, chunk.y * CHUNK, chunk.z * CHUNK);
    let n = CHUNK as usize;
    let cell = |x: i32, y: i32, z: i32| world.get(sv(base.x + x, base.y + y, base.z + z));
    // the six directions: (axis, sign)
    for dir in 0..6usize {
        let axis = dir / 2;
        let pos = dir % 2 == 0;
        let (ua, va) = match axis {
            0 => (2, 1),
            1 => (0, 2),
            _ => (0, 1),
        };
        for d in 0..CHUNK {
            let mut mask: Vec<Option<u32>> = vec![None; n * n];
            for a in 0..CHUNK {
                for b in 0..CHUNK {
                    let mut p = [0i32; 3];
                    p[axis] = d;
                    p[ua] = a;
                    p[va] = b;
                    let c = cell(p[0], p[1], p[2]);
                    if !c.is_solid() {
                        continue;
                    }
                    let mut q = p;
                    q[axis] += if pos { 1 } else { -1 };
                    let nb = cell(q[0], q[1], q[2]);
                    if nb.is_solid() {
                        continue;
                    }
                    // skip faces nobody can see: out of the world
                    let qw = sv(base.x + q[0], base.y + q[1], base.z + q[2]);
                    if !world.in_bounds(qw) {
                        continue;
                    }
                    let side = axis != 1;
                    mask[(a as usize) * n + b as usize] = Some(tile_of(c, side));
                }
            }
            // greedy: grow rectangles of the same tile
            for a in 0..n {
                let mut b = 0;
                while b < n {
                    let Some(t) = mask[a * n + b] else {
                        b += 1;
                        continue;
                    };
                    let mut h = 1;
                    while b + h < n && mask[a * n + b + h] == Some(t) {
                        h += 1;
                    }
                    let mut w = 1;
                    'grow: while a + w < n {
                        for k in 0..h {
                            if mask[(a + w) * n + b + k] != Some(t) {
                                break 'grow;
                            }
                        }
                        w += 1;
                    }
                    for i in 0..w {
                        for k in 0..h {
                            mask[(a + i) * n + b + k] = None;
                        }
                    }
                    let mut o = [0.0f32; 3];
                    o[axis] = (base_axis(base, axis) + d) as f32 + if pos { 1.0 } else { 0.0 };
                    o[ua] = (base_axis(base, ua) + a as i32) as f32;
                    o[va] = (base_axis(base, va) + b as i32) as f32;
                    let mut u = [0.0f32; 3];
                    u[ua] = w as f32;
                    let mut v = [0.0f32; 3];
                    v[va] = h as f32;
                    let mut nn = [0.0f32; 3];
                    nn[axis] = if pos { 1.0 } else { -1.0 };
                    m.quad(Vec3::from(o), Vec3::from(u), Vec3::from(v), Vec3::from(nn), w as f32, h as f32, t, SHADE[dir]);
                    b += h;
                }
            }
        }
    }
    // liquid surfaces: tops (lowered) and sides that face air
    for y in 0..CHUNK {
        for z in 0..CHUNK {
            for x in 0..CHUNK {
                let c = cell(x, y, z);
                if !c.is_liquid() {
                    continue;
                }
                let (wx, wy, wz) = (base.x + x, base.y + y, base.z + z);
                let above = world.get(sv(wx, wy + 1, wz));
                let full = above.is_liquid();
                let top = if full { 1.0 } else { LIQUID_TOP };
                let t = tile_of(c, false);
                let o = Vec3::new(wx as f32, wy as f32, wz as f32);
                if !full {
                    m.quad(o + Vec3::new(0.0, top, 0.0), Vec3::X, Vec3::Z, Vec3::Y, 1.0, 1.0, t, 1.0);
                }
                for (dx, dz, nrm) in [(1, 0, Vec3::X), (-1, 0, Vec3::NEG_X), (0, 1, Vec3::Z), (0, -1, Vec3::NEG_Z)] {
                    let nb = world.get(sv(wx + dx, wy, wz + dz));
                    if nb.is_solid() || nb.is_liquid() {
                        continue;
                    }
                    let (corner, u) = match (dx, dz) {
                        (1, 0) => (o + Vec3::X, Vec3::Z),
                        (-1, 0) => (o, Vec3::Z),
                        (0, 1) => (o + Vec3::Z, Vec3::X),
                        _ => (o, Vec3::X),
                    };
                    m.quad(corner, u, Vec3::Y * top, nrm, 1.0, top, tile_of(c, true), 0.85);
                }
            }
        }
    }
    m.into_mesh()
}

fn base_axis(base: SIVec3, axis: usize) -> i32 {
    match axis {
        0 => base.x,
        1 => base.y,
        _ => base.z,
    }
}

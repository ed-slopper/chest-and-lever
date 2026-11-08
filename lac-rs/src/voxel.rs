//! Voxel meshes for the dungeon (board row E4) and the palette material they use (E7).
//!
//! REF R40: the dungeon is voxels. One mesh per 16³ chunk, made by `lac-mesh`: only faces between solid and
//! open cells, each carved into voxel relief from its texture's depths (mortar and seams sit back), and liquids
//! as lowered surfaces (OURS).

use crate::palette;
use crate::textures;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use lac_sim::world::World;
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

/// The textures with their depths, made once.
fn textures() -> &'static [lac_mesh::textures::Tex] {
    static TEX: std::sync::OnceLock<Vec<lac_mesh::textures::Tex>> = std::sync::OnceLock::new();
    TEX.get_or_init(lac_mesh::textures::all_textures)
}

/// The mesh of one chunk, with its relief carved (see `lac-mesh`).
pub fn chunk_mesh(world: &World, chunk: SIVec3) -> Option<Mesh> {
    let m = lac_mesh::chunk_mesh(world, chunk, textures());
    if m.is_empty() {
        return None;
    }
    Some(
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, m.pos)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, m.nrm)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, m.uv)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, m.col)
            .with_inserted_indices(Indices::U32(m.idx)),
    )
}

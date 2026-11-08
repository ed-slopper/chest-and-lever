//! Tile textures (board row C8) live in `lac-mesh` (`textures.rs`), with the depths the relief mesher carves.
//! Here they become the R8 atlas the palette material reads.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
pub use lac_mesh::textures::{TILE, TILES_PER_ROW};

/// The atlas: every cell kind's top and side, as palette indices in an R8 texture.
pub fn atlas_image() -> Image {
    let (w, h, data) = lac_mesh::textures::atlas_indices();
    let mut img = Image::new(
        Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::R8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = bevy::image::ImageSampler::nearest();
    img
}

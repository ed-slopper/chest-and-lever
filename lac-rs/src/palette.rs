//! The palette (board rows E7, F8). F8 (decided 2026-11-08): one palette of 64 colours, 8 ramps of 8, dark to
//! light. Textures are palette indices; colours are looked up on the GPU, and some ramps cycle over time.
//! REF R54: palette cycling. REF R53: sky (here, fog) colours come from an artist-made lookup strip, not from
//! physics. OURS: the colours themselves and the colour-blind variants (board row X4).

use bevy::prelude::*;

pub const RAMPS: usize = 8;
pub const SHADES: usize = 8;
pub const COLOURS: usize = RAMPS * SHADES;

/// Ramp numbers and indices live in `lac-mesh`, which builds the textures.
pub use lac_mesh::pal::{ix, BLUE, BONE, BROWN, FIRE, GREEN, GREY, RED, VIOLET};

/// Ranges that cycle: (first index, length, steps per second). REF R54. OURS: water flows, lava churns,
/// crystal glimmers.
pub const CYCLES: [(u8, u8, f32); 3] = [(ix(BLUE, 2), 6, 5.0), (ix(FIRE, 1), 7, 7.0), (ix(VIOLET, 4), 4, 3.0)];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Vision {
    #[default]
    Normal,
    /// Reds and greens moved apart for red-green colour blindness.
    RedGreen,
    /// Blues and yellows moved apart for blue-yellow colour blindness.
    BlueYellow,
}

impl Vision {
    pub fn next(self) -> Vision {
        match self {
            Vision::Normal => Vision::RedGreen,
            Vision::RedGreen => Vision::BlueYellow,
            Vision::BlueYellow => Vision::Normal,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Vision::Normal => "normal",
            Vision::RedGreen => "red-green",
            Vision::BlueYellow => "blue-yellow",
        }
    }
}

/// Each ramp as (hue degrees, saturation, the darkest and lightest lightness).
fn ramp_def(ramp: usize, vision: Vision) -> (f32, f32, f32, f32) {
    let base = match ramp {
        0 => (140.0, 0.09, 0.06, 0.78), // grey stone, a little green (ed-slopper's reference: damp, mossy grey)
        1 => (28.0, 0.42, 0.07, 0.70),  // brown earth and wood
        2 => (14.0, 0.36, 0.08, 0.64),  // brick: a dusty, brownish red
        3 => (105.0, 0.40, 0.06, 0.66), // moss green
        4 => (205.0, 0.60, 0.08, 0.74), // water blue
        5 => (22.0, 0.95, 0.12, 0.88),  // fire: red to yellow-white
        6 => (275.0, 0.55, 0.10, 0.82), // violet crystal and magic
        _ => (42.0, 0.30, 0.18, 0.92),  // bone and skin
    };
    match (vision, ramp) {
        // OURS: shift the clashing ramps so they differ in lightness and hue that the eye can tell apart
        (Vision::RedGreen, 2) => (340.0, 0.55, 0.10, 0.62),
        (Vision::RedGreen, 3) => (190.0, 0.45, 0.10, 0.80),
        (Vision::BlueYellow, 4) => (180.0, 0.45, 0.10, 0.82),
        (Vision::BlueYellow, 5) => (350.0, 0.85, 0.12, 0.80),
        _ => base,
    }
}

fn hsl(h: f32, s: f32, l: f32) -> [f32; 3] {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = (h.rem_euclid(360.0)) / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    [r + m, g + m, b + m]
}

/// The 64 colours, sRGB 0..1.
pub fn colours(vision: Vision) -> Vec<[f32; 3]> {
    let mut out = Vec::with_capacity(COLOURS);
    for r in 0..RAMPS {
        let (h, s, l0, l1) = ramp_def(r, vision);
        for k in 0..SHADES {
            let t = k as f32 / (SHADES - 1) as f32;
            // the fire ramp also turns from red to yellow as it brightens
            let hue = if r == 5 { h - 18.0 + t * 38.0 } else { h + (t - 0.5) * 10.0 };
            // ease the light end so ramps keep detail in the darks
            let l = l0 + (l1 - l0) * t.powf(1.15);
            out.push(hsl(hue, s * (1.0 - 0.25 * t), l));
        }
    }
    out
}

/// The palette as a 64×1 texture.
pub fn palette_image(vision: Vision) -> Image {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let mut data = Vec::with_capacity(COLOURS * 4);
    for c in colours(vision) {
        for v in c {
            data.push((v.clamp(0.0, 1.0) * 255.0).round() as u8);
        }
        data.push(255);
    }
    let mut img = Image::new(
        Extent3d { width: COLOURS as u32, height: 1, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    img.sampler = bevy::image::ImageSampler::nearest();
    img
}

/// REF R53: the fog colour of each depth, from an artist strip rather than computed. OURS: the colours, as
/// palette indices, depth 1 at the left.
/// Green-black in the crypt and caves (ed-slopper's reference), blue-black in the tombs, violet in the deep.
pub const FOG_STRIP: [u8; 9] = [ix(GREY, 1), ix(GREY, 1), ix(GREEN, 0), ix(GREEN, 0), ix(BLUE, 0), ix(BLUE, 0), ix(VIOLET, 0), ix(VIOLET, 0), ix(FIRE, 0)];

pub fn fog(vision: Vision, depth: u8) -> Color {
    // depth 1 is the strip's first entry
    let i = FOG_STRIP[(depth.max(1) as usize - 1).min(FOG_STRIP.len() - 1)];
    let c = colours(vision)[i as usize];
    // fog sits a little darker than the darkest shades, so far walls melt into it
    Color::srgb(c[0] * 0.7, c[1] * 0.7, c[2] * 0.7)
}

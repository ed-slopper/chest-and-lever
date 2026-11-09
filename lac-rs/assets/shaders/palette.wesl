// Board row E7: the palette material. Textures hold palette indices; the palette is looked up here, and some
// ranges cycle with time (R54). The rest of the lighting is Bevy's standard PBR, so SSAO (R50), fog and the
// party's light all apply.
//
// Vertex colour: r = atlas tile, g = face shade. UV: position on the face in tiles (fract gives the texel).

import bevy_pbr::render::{
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    mesh_view_bindings::globals,
};
@if(SCREEN_SPACE_AMBIENT_OCCLUSION)
import bevy_pbr::render::mesh_view_bindings::screen_space_ambient_occlusion_texture;

struct PaletteSettings {
    // (first index, length, steps per second, unused)
    cycles: array<vec4<f32>, 4>,
    // (tiles per atlas row, texels per tile edge, glow strength, unused)
    params: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> settings: PaletteSettings;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var atlas: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var palette: texture_2d<f32>;

// OURS: raw GTAO values mapped to black and white, and the least light a corner keeps
const AO_BLACK: f32 = 0.4;
const AO_WHITE: f32 = 0.8;
const AO_FLOOR: f32 = 0.25;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    let tile = u32(in.color.r + 0.5);
    let shade = in.color.g;
    let tpr = u32(settings.params.x);
    let ts = u32(settings.params.y);
    let f = fract(in.uv);
    let tx = min(u32(f.x * f32(ts)), ts - 1u);
    let ty = min(u32(f.y * f32(ts)), ts - 1u);
    let texel = vec2<i32>(i32((tile % tpr) * ts + tx), i32((tile / tpr) * ts + ty));
    var idx = u32(textureLoad(atlas, texel, 0).r * 255.0 + 0.5);

    for (var i = 0u; i < 4u; i = i + 1u) {
        let c = settings.cycles[i];
        let first = u32(c.x);
        let len = u32(c.y);
        if len > 0u && idx >= first && idx < first + len {
            let off = u32(globals.time * c.z) % len;
            idx = first + (idx - first + off) % len;
        }
    }

    let colour = textureLoad(palette, vec2<i32>(i32(idx), 0), 0);
    pbr_input.material.base_color = vec4<f32>(colour.rgb * shade, 1.0);

    // lava and the brightest crystal glow (fire ramp shades 2-7, violet 6-7)
    let glow = (idx >= 42u && idx <= 47u) || (idx >= 54u && idx <= 55u);
    if glow {
        pbr_input.material.emissive = vec4<f32>(colour.rgb * settings.params.z, 1.0);
    }

    // REF R65: the SSAO is sampled with dithering and blurred with respect for depth (Bevy's GTAO and its
    // bilateral denoiser), then an artificial dither is laid on top: here, the occlusion is cut to a few
    // levels through a 4×4 ordered (Bayer) pattern on the screen's pixels, so dark corners read as a dither.
    // OURS: 6 levels and a 4×4 Bayer matrix (the record does not say which dither); the raw occlusion is
    // remapped so open walls (which GTAO leaves a little below 1 at our scale) stay fully lit and only
    // corners and crevices darken, and it is grey, so the stone keeps its colour.
@if(SCREEN_SPACE_AMBIENT_OCCLUSION) {
        let bayer = array<f32, 16>(0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0);
        let px = vec2<u32>(in.position.xy) % 4u;
        let threshold = (bayer[px.y * 4u + px.x] + 0.5) / 16.0;
        let raw = textureLoad(screen_space_ambient_occlusion_texture, vec2<i32>(in.position.xy), 0i).r;
        let ao = smoothstep(AO_BLACK, AO_WHITE, raw);
        let levels = 6.0;
        let q = min(floor(ao * levels + threshold) / levels, 1.0);
        pbr_input.diffuse_occlusion = vec3<f32>(AO_FLOOR + (1.0 - AO_FLOOR) * q);
}

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}

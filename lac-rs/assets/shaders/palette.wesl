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

struct PaletteSettings {
    // (first index, length, steps per second, unused)
    cycles: array<vec4<f32>, 4>,
    // (tiles per atlas row, texels per tile edge, glow strength, unused)
    params: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> settings: PaletteSettings;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var atlas: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var palette: texture_2d<f32>;

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

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}

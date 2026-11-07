// P26 R28: StandardMaterial forward fragment (bevy_pbr pbr.wgsl, forward path) + the GS destination
// alpha test of EnableStencilRead(1) (TEST DATE on, DATM 0): the fragment is dropped where the frame
// alpha has bit 7 set. The frame alpha is the mask camera's image (game_fx::StencilRead): alpha of the
// EnableStencilWrite objects (owner logic flag 0x8000), 0 elsewhere (ClearStencil). PS2 alpha 0x80 =
// 1.0, so bit 7 is set only for alpha >= 1.0 (stored 255 in the Rgba8 mask).
#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    forward_io::VertexOutput,
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
    view_transformations::frag_coord_to_uv,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var mask_texture: texture_2d<f32>;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    let dim = vec2<f32>(textureDimensions(mask_texture));
    let px = clamp(vec2<i32>(frag_coord_to_uv(in.position.xy) * dim), vec2<i32>(0), vec2<i32>(dim) - 1);
    if textureLoad(mask_texture, px, 0).a > 254.5 / 255.0 {
        discard;
    }
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
    var color: vec4<f32>;
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        color = apply_pbr_lighting(pbr_input);
    } else {
        color = pbr_input.material.base_color;
    }
    return main_pass_post_lighting_processing(pbr_input, color);
}

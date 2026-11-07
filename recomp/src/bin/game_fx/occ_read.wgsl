// P33 R3: RenderSpriteNewOcc final draw (game_fx::OccRead): ALPHA 0x98 Cs*Ad / 0x58 Cs*Ad+Cd /
// 0x91 (Cd-Cs)*Ad, Ad = the sample square's alpha spread over the sprite: colour and alpha * the
// mean green of the stencil-mask image over the sample square (1 = centre visible, 0 = hidden).
#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, main_pass_post_lighting_processing},
    forward_io::VertexOutput,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var mask_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var<uniform> rect: vec4<f32>;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    let dim = vec2<i32>(textureDimensions(mask_texture));
    let p0 = clamp(vec2<i32>(floor(rect.xy * vec2<f32>(dim))), vec2<i32>(0), dim - 1);
    let p1 = clamp(vec2<i32>(ceil(rect.zw * vec2<f32>(dim))) - 1, p0, min(dim - 1, p0 + 31));
    var sum = 0.0;
    var n = 0.0;
    for (var y = p0.y; y <= p1.y; y++) {
        for (var x = p0.x; x <= p1.x; x++) {
            sum += textureLoad(mask_texture, vec2<i32>(x, y), 0).g;
            n += 1.0;
        }
    }
    let ad = sum / max(n, 1.0);
    if ad <= 0.0 {
        discard;
    }
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
    let color = main_pass_post_lighting_processing(pbr_input, pbr_input.material.base_color);
    return color * ad;
}

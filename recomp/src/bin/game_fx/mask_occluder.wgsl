// P26 R29: stencil-mask occluder (game_fx::MaskOccluder): the world drawn into the destination-alpha
// mask with its depth and alpha 0, alpha-tested like its StandardMaterial (cut-out holes stay open).
#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
    forward_io::VertexOutput,
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
    return vec4<f32>(0.0);
}

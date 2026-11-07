// P26 R21: display-space frame -> linear (see game_fx::DisplayDecode).
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var screen_sampler: sampler;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let c = textureSampleLevel(screen_texture, screen_sampler, in.uv, 0.0);
    let lo = c.rgb / 12.92;
    let hi = pow((max(c.rgb, vec3(0.0)) + 0.055) / 1.055, vec3(2.4));
    return vec4<f32>(select(hi, lo, c.rgb <= vec3(0.04045)), c.a);
}

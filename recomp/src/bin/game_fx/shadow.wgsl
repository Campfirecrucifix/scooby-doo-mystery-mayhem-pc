// WP-S6: GS ALPHA 0xa1 (Cd - Cs) projected shadow: Cs = mask texel * occurrence colour;
// blended with ReverseSubtract (dst - src) set in ShadowSubMaterial::specialize.
#import bevy_pbr::forward_io::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> color: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var mask_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var mask_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let m = textureSample(mask_texture, mask_sampler, in.uv).r;
    return vec4<f32>(color.rgb * m, 0.0);
}

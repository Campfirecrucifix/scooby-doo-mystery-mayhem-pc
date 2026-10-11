// WP-S6: EFFBEffect_FrameDistortion approximation: sample the grabbed frame (Bevy's opaque-pass
// copy, view_transmission_texture) at the fragment's screen position offset by the distortion
// matrix applied to the view-space normal.
#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::{view, view_transmission_texture, view_transmission_sampler}
#import bevy_pbr::view_transformations::{direction_world_to_view, frag_coord_to_uv}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> uv_matrix: mat4x4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> strength: vec4<f32>;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(direction_world_to_view(normalize(in.world_normal)));
    let o = (uv_matrix * vec4<f32>(n, 0.0)).xy;
    let uv = frag_coord_to_uv(in.position.xy) + vec2<f32>(o.x, -o.y) * strength.x;
    let c = textureSampleLevel(view_transmission_texture, view_transmission_sampler, clamp(uv, vec2(0.001), vec2(0.999)), 0.0);
    return vec4<f32>(c.rgb, 1.0);
}

// WP-S6: GS ALPHA 0xa1 (Cd - Cs) projected shadow: Cs = (mask texel * trunc(colour*128)) >> 7.
// P26 B2: the GS subtracts in frame-buffer (gamma / sRGB-encoded) space with clamping at 0; a
// linear-space ReverseSubtract blend turned the faint 0.05-0.15 shadows into black (and negative
// HDR values into red fringes). Cd is read from the opaque-pass copy (view_transmission_texture,
// like the distortion pass) and the subtraction is done on the sRGB-encoded value.
// P26 R4: overlapping patches stack like the GS (subtract twice): every patch fragment sums the
// Cs of all the frame's occurrences covering its world position (same projection / near-far /
// texture-square clip as gfx::projector), so overlapping draws write the same stacked value.
#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::{view_transmission_texture, view_transmission_sampler}
#import bevy_pbr::view_transformations::frag_coord_to_uv

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> occ: array<vec4<f32>, 48>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var mask_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var mask_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var<uniform> count: vec4<f32>;

fn to_srgb(c: vec3<f32>) -> vec3<f32> {
    let lo = c * 12.92;
    let hi = 1.055 * pow(max(c, vec3(0.0)), vec3(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3(0.0031308));
}
fn to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((max(c, vec3(0.0)) + 0.055) / 1.055, vec3(2.4));
    return select(hi, lo, c <= vec3(0.04045));
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = in.world_position.xyz;
    var cs = 0.0;
    let n = u32(count.x);
    for (var i = 0u; i < n; i++) {
        let r = occ[6u * i];
        let u = occ[6u * i + 1u];
        let a = occ[6u * i + 2u];
        let o = occ[6u * i + 3u];
        let nr = occ[6u * i + 4u];
        let fr = occ[6u * i + 5u];
        if (dot(nr.xyz, p) + nr.w < 0.0 || dot(fr.xyz, p) + fr.w < 0.0) {
            continue;
        }
        let d = p - o.xyz;
        let z = select(dot(d, a.xyz), 1.0, u.w > 0.5);
        let x = dot(d, r.xyz);
        let y = dot(d, u.xyz);
        let s = r.w * z;
        if (s <= 0.0 || abs(x) > s || abs(y) > s) {
            continue;
        }
        let uv = vec2(0.5 - 0.5 * x / s, 0.5 - 0.5 * y / s) * a.w;
        let slot = vec2(f32(i % 4u), f32(i / 4u));
        let m = textureSampleLevel(mask_texture, mask_sampler, (slot + uv) * vec2(0.25, 0.5), 0.0).r;
        // P26 R7: GS MODULATE (texel * RGBA) >> 7 with RGBA = trunc(colour*128) (o.w, VU1 projectormapPRS)
        cs += floor(o.w * round(m * 255.0) / 128.0) / 255.0;
    }
    let sc = frag_coord_to_uv(in.position.xy);
    let d = textureSampleLevel(view_transmission_texture, view_transmission_sampler, sc, 0.0).rgb;
    // P26 R21: count.y = 1 -> display-space frame (bevy_adapter::display_space): Cd is already the
    // frame-buffer value
    if (count.y > 0.5) {
        return vec4<f32>(max(d - vec3(cs), vec3(0.0)), 1.0);
    }
    let g = max(to_srgb(d) - vec3(cs), vec3(0.0));
    return vec4<f32>(to_linear(g), 1.0);
}

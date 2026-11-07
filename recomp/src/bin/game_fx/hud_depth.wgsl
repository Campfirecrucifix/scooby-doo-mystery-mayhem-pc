// P26 R26: StandardMaterial forward fragment (bevy_pbr pbr.wgsl, forward path) + the PS2 16-bit HUD
// Z buffer: the fragment depth is snapped to the Z value the GS would store for the HUD camera
// (EFHudMgr_SP::Update: parallel, near 0.1, far 256; zBufferDepth 16 -> 65535 steps; ZTST GEQUAL).
// Equal snapped depths compare equal; HudDepth::specialize uses a strict Greater test, so the earlier
// draw wins ties (what PCSX2 shows for the compass dial: disk drawn before the ping ring in the port).
#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    forward_io::VertexOutput,
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
}

struct HudOut {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
}

const HUD_NEAR: f32 = 0.1;
const HUD_FAR: f32 = 256.0;
const Z_MAX: f32 = 65535.0;
// the port's HUD camera (game.rs): orthographic, near -1000, far 1000, reverse Z -> ndc = (1000 - d) / 2000
const PORT_NEAR: f32 = -1000.0;
const PORT_FAR: f32 = 1000.0;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> HudOut {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
    var out: HudOut;
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = apply_pbr_lighting(pbr_input);
    } else {
        out.color = pbr_input.material.base_color;
    }
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    // HUD camera at the origin looking down +z: view depth = world z
    let d = in.world_position.z;
    // GS Z (larger = nearer), truncated like the VU1 FTOI
    let zi = floor(clamp((HUD_FAR - d) / (HUD_FAR - HUD_NEAR), 0.0, 1.0) * Z_MAX);
    let dq = HUD_FAR - zi / Z_MAX * (HUD_FAR - HUD_NEAR);
    out.depth = (PORT_FAR - dq) / (PORT_FAR - PORT_NEAR);
    return out;
}

//! WP-C4: render correctness — textures/UVs/colour scale, CPU RW lighting, particle billboards.
#![cfg(feature = "bevy_render")]
use scooby_recomp::engine::gfx::bevy_adapter::*;
use scooby_recomp::engine::particle::{PLDrawBatch, PLPrims};

#[test]
fn ps2_colour_scale() {
    // GS colour 0x80 = 1.0 (saturating above), RwRGBA 255 = 1.0
    assert_eq!(gs_to_linear([0x80, 0x80, 0x80, 0x80]), [1.0; 4]);
    assert_eq!(gs_to_linear([0xff, 0, 0, 0xff])[0], 1.0);
    assert_eq!(gs_to_linear([0xff, 0, 0, 0xff])[3], 1.0);
    assert_eq!(rwrgba_to_linear([255, 255, 255, 255]), [1.0; 4]);
    assert!((srgb_to_linear(0.5) - 0.214).abs() < 1e-3);
}

#[test]
fn rw_lighting_model() {
    let ident: [f32; 16] = [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.];
    let amb = RwLightInst { ty: 2, flags: 1, color: [0.25; 3], radius: 0., minus_cos_angle: 0., at: [0.; 3], pos: [0.; 3] };
    // directional pointing -Y lights an up-facing normal fully
    let dir = RwLightInst { ty: 1, flags: 1, color: [0.5; 3], radius: 0., minus_cos_angle: 0., at: [0., -1., 0.], pos: [0.; 3] };
    let l = RwLights { lights: vec![amb, dir] };
    let lit = 0x20; // rpGEOMETRYLIGHT
    let c = l.LightVertices(&ident, &[[0.; 3], [0.; 3]], &[[0., 1., 0.], [0., -1., 0.]], &[], lit, 1.0, 1.0);
    assert!((c[0][0] - srgb_to_linear(0.75)).abs() < 1e-5);
    assert!((c[1][0] - srgb_to_linear(0.25)).abs() < 1e-5);
    // unlit geometry without prelit = white
    let c = l.LightVertices(&ident, &[[0.; 3]], &[[0., 1., 0.]], &[], 0, 1.0, 1.0);
    assert_eq!(c[0], [1.0; 4]);
}

#[test]
fn particle_billboards_face_camera() {
    use scooby_recomp::engine::particle::PLSprite;
    let s = PLSprite { pos: [1., 2., 3.], size: 0.5, rot_index: 0, rgba: [0x80, 0x40, 0, 0x80] };
    let b = PLDrawBatch { emitter: 0, texture: "glow".into(), blend: 0x29, stencil_read: false, prims: PLPrims::Sprites(vec![s]) };
    let m = particle_batch_mesh(&b, [1., 0., 0.], [0., 1., 0.]);
    let p = m.attribute(bevy_mesh::Mesh::ATTRIBUTE_POSITION).unwrap().as_float3().unwrap();
    assert_eq!(p.len(), 4);
    for v in p {
        assert_eq!(v[2], 3.0, "quad lies in the camera plane");
    }
    // J23: FIX blends ignore source alpha -> Premultiplied with vertex alpha 0 (exact ONE/ONE)
    assert_eq!(particle_alpha_mode(0x29), bevy_material::AlphaMode::Premultiplied);
    assert_eq!(particle_alpha_mode(0x44), bevy_material::AlphaMode::Blend);
}

//! WP-C4: render correctness — textures/UVs/colour scale, CPU RW lighting, particle billboards.
#![cfg(feature = "bevy_render")]
use scooby_recomp::engine::gfx::bevy_adapter::*;
use scooby_recomp::engine::particle::{PLDrawBatch, PLPrims};

#[test]
fn ps2_colour_scale() {
    // GS colour 0x80 = 1.0; colour above 0x80 kept (GS clamps Cf*Ct>>7, not Cf; P26 R40), alpha saturates
    assert_eq!(gs_to_linear([0x80, 0x80, 0x80, 0x80]), [1.0; 4]);
    assert!(gs_to_linear([0xff, 0, 0, 0xff])[0] > 1.0);
    // P26 R41: unclamped only for the pure adds 0x68/0x29; 0x44/0xa1/Add keep Cf <= 1.0
    assert!(gs_to_linear_blend([0xff, 0, 0, 0xff], 0x68)[0] > 1.0);
    assert_eq!(gs_to_linear_blend([0xff, 0, 0, 0xff], 0x44)[0], 1.0);
    assert_eq!(gs_to_linear_blend([0xff, 0, 0, 0xff], 0xa1)[0], 1.0);
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

// P40 R3 (B#9): GS ALPHA 0xa1 = (Cd - Cs) * FIX (FIX 0x80 = 1.0). With the game's hardware subtract
// material the batch is Blend-phase with the plain Cs (texel * vertex colour) and alpha 1 (the
// pipeline blend becomes ReverseSubtract: out = Cd - Cs); without it the old Multiply(1 - Cs).
/// P40 R5 (B#9 colour space): the GS subtracts frame-buffer (display) values. The game runs in
/// display-space mode (bevy_adapter::display_space, on unless GAME_LINEAR_BLEND): colours reach the
/// GPU unconverted, the sRGB main texture stores them as if linear (hardware decode/blend/encode is
/// an identity on the value), ReverseSubtract works on display values, and game_fx::DisplayDecode +
/// the sRGB target round-trip them back. Model that chain with numbers: Cd 0.75 / Cs 0.46 display
/// -> 0.29 (GS); the linear-light path (GAME_LINEAR_BLEND=1, debug only) would give ~0.62.
#[test]
fn p40_gs_subtract_display_space() {
    let enc = |l: f32| if l <= 0.0031308 { l * 12.92 } else { 1.055 * l.powf(1.0 / 2.4) - 0.055 };
    let dec = |c: f32| if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) };
    let q = |v: f32| (v * 255.0).round() / 255.0; // Rgba8 storage
    let (cd, cs) = (0.75f32, 0.46f32);
    let gs = (cd - cs).max(0.0);
    // display-space path: shader outputs display values as "linear"; sRGB attachment encode -> blend
    // reads dec(stored) = value; ReverseSubtract; stored enc(result); DisplayDecode samples it (dec =
    // the value) and outputs dec(value); the sRGB target encodes -> the display value
    let stored = q(enc(cd));
    let blended = (dec(stored) - cs).max(0.0);
    let sampled = dec(q(enc(blended))); // DisplayDecode samples the sRGB texture (hardware decode)
    let decoded = dec(sampled); // ... and decodes once (sRGB -> linear)
    let shown = q(enc(decoded)); // the sRGB target encodes
    assert!((shown - gs).abs() < 2.0 / 255.0, "display-space subtract {shown} vs GS {gs}");
    assert!((shown - 0.29).abs() < 0.01);
    // linear-light path for contrast: values decoded first, subtracted in linear, shown encoded
    let lin = enc((dec(cd) - dec(cs)).max(0.0));
    assert!((lin - 0.62).abs() < 0.02, "linear-light subtract {lin}");
    // the game binary turns display space on by default (GAME_LINEAR_BLEND unset)
    let game_src = include_str!("../../src/bin/game.rs");
    assert!(game_src.contains("set_display_space(std::env::var_os(\"GAME_LINEAR_BLEND\").is_none())"));
}

#[test]
fn p40_gs_subtract_particles() {
    use bevy_mesh::VertexAttributeValues;
    use scooby_recomp::engine::particle::PLSprite;
    let s = PLSprite { pos: [0.; 3], size: 1.0, rot_index: 0, rgba: [0x40, 0x20, 0x80, 0x10] };
    let b = PLDrawBatch { emitter: 0, texture: "broue".into(), blend: 0xa1, stencil_read: false, prims: PLPrims::Sprites(vec![s]) };
    let col = |m: &bevy_mesh::Mesh| match m.attribute(bevy_mesh::Mesh::ATTRIBUTE_COLOR).unwrap() {
        VertexAttributeValues::Float32x4(v) => v[0],
        _ => panic!("colour format"),
    };
    let cs = gs_to_linear_blend([0x40, 0x20, 0x80, 0x10], 0xa1);
    set_sub_blend_hw(true);
    let hw = (particle_alpha_mode(0xa1), col(&particle_batch_mesh(&b, [1., 0., 0.], [0., 1., 0.])));
    set_sub_blend_hw(false);
    let old = (particle_alpha_mode(0xa1), col(&particle_batch_mesh(&b, [1., 0., 0.], [0., 1., 0.])));
    assert_eq!(hw.0, bevy_material::AlphaMode::Blend);
    assert_eq!(hw.1, [cs[0], cs[1], cs[2], 1.0], "plain Cs, alpha ignored (FIX)");
    // GS result for Cd = 0.75 grey: max(Cd - Cs, 0) per channel; ReverseSubtract(One, One) gives exactly that
    let gs: Vec<f32> = (0..3).map(|i| (0.75 - cs[i]).max(0.0)).collect();
    let rev_sub: Vec<f32> = (0..3).map(|i| (0.75 * 1.0 - hw.1[i] * 1.0).clamp(0.0, 1.0)).collect();
    assert_eq!(gs, rev_sub);
    // the old path: Multiply by the inverted colour = Cd * (1 - Cs), lighter than the GS (Cd - Cs) wherever
    // Cd < 1 and 0 < Cs < 1 (Cd - Cd*Cs > Cd - Cs <=> Cs > Cd*Cs; equal at Cs = 1: both 0)
    assert_eq!(old.0, bevy_material::AlphaMode::Multiply);
    assert!((0..3).all(|i| 0.75 * old.1[i] >= gs[i]) && (0..3).any(|i| 0.75 * old.1[i] > gs[i] + 0.03), "old approximation was lighter: {:?} vs {gs:?}", old.1);
}

/// P40 R7: PLEmitter / EFBillboardGroup sprites are expanded by VU1TestCode (EE 0x002846c0) into a
/// 4-vertex strip whose corners are `size` from the centre (c0 = p - h(R cos + U sin), c1 = p + h(R sin
/// - U cos), c2/c3 the opposite ones): a square of side size*sqrt2 turned 45 degrees, not the
/// axis-aligned square of half-side `size` (2x the area) the port draws by default (P40 R8: VU1 shape = GAME_SPRITE_VU1=1).
/// The GS sees the corner XY as 12.4 fixed point in 16 bits (window [0, 4096), XYOFFSET 2048 - w/2).
#[test]
fn p40_vu1_sprite_corners_and_gs_window() {
    use scooby_recomp::engine::camera::DTMatrix;
    use scooby_recomp::engine::gfx::sprite::{SpriteCamera, Vu1SpriteCorners};
    use scooby_recomp::engine::particle::PLSprite;
    let (r, u) = ([1.0f32, 0., 0.], [0.0f32, 1., 0.]);
    let p = [1.0f32, 2., 3.];
    let d = |a: [f32; 3], b: [f32; 3]| ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
    for rot in [0u8, 17, 64, 127] {
        let c = Vu1SpriteCorners(r, u, p, 0.5, rot);
        for k in c {
            assert!((d(k, p) - 0.5).abs() < 1e-5, "corner at the size from the centre");
            assert!((k[2] - 3.0).abs() < 1e-6);
        }
        assert!((d(c[0], c[1]) - 0.5 * 2f32.sqrt()).abs() < 1e-5, "side = size * sqrt2");
        assert!((d(c[0], c[3]) - 1.0).abs() < 1e-5 && (d(c[1], c[2]) - 1.0).abs() < 1e-5, "strip: c0/c3 and c1/c2 opposite");
    }
    // rot 0: a diamond along R / U (c0 = p - R h, c1 = p - U h)
    let c = Vu1SpriteCorners(r, u, p, 0.5, 0);
    assert_eq!(c[0], [0.5, 2., 3.]);
    assert_eq!(c[1], [1., 1.5, 3.]);
    // the mesh uses these corners (screen-right = -LTM.right)
    let s = PLSprite { pos: p, size: 0.5, rot_index: 0, rgba: [0x80; 4] };
    let b = PLDrawBatch { emitter: 0, texture: "t".into(), blend: 0x29, stencil_read: false, prims: PLPrims::Sprites(vec![s]) };
    let m = particle_batch_mesh_shape(&b, [-1., 0., 0.], [0., 1., 0.], true);
    let mp = m.attribute(bevy_mesh::Mesh::ATTRIBUTE_POSITION).unwrap().as_float3().unwrap();
    assert_eq!(mp, &c[..]);
    // P40 R8: default (GAME_SPRITE_VU1 unset) = the old axis-aligned square of half-side `size`
    let m = particle_batch_mesh_shape(&b, [-1., 0., 0.], [0., 1., 0.], false);
    let mp = m.attribute(bevy_mesh::Mesh::ATTRIBUTE_POSITION).unwrap().as_float3().unwrap();
    for v in mp {
        assert!(((v[0] - 1.0).abs() - 0.5).abs() < 1e-6 && ((v[1] - 2.0).abs() - 0.5).abs() < 1e-6);
    }
    // GS window: centre on screen (screen centre -> 2048, 2048), size 1 fits, size 5 at z 1 crosses 4096
    let ltm = DTMatrix { right: r, up: u, at: [0., 0., 1.], pos: [0.; 3] };
    let cam = SpriteCamera { ltm, view_window: [0.5, 0.5], near: 0.1, far: 100.0, width: 640, height: 448, z_scale: 0., z_shift: 0. };
    assert_eq!(cam.GsXY([0., 0., 1.]), [2048., 2048.]);
    assert!(cam.Vu1SpriteVisible([0., 0., 1.]));
    assert!(cam.GsWindowOk(&Vu1SpriteCorners(r, u, [0., 0., 1.], 1.0, 0)));
    assert!(!cam.GsWindowOk(&Vu1SpriteCorners(r, u, [0., 0., 1.], 5.0, 0)));
}

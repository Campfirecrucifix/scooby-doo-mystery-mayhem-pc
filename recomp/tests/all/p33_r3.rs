//! P33 R3: EFSpriteGroup_SP::RenderSpriteNewOcc (0x002780a0) occlusion test geometry: the sample
//! square at the sprite's own depth (GS pass 2) and its screen uv (read back by the sprite draw).
use scooby_recomp::engine::camera::DTMatrix;
use scooby_recomp::engine::gfx::sprite::{OccludedSprite, SpriteCamera};

fn cam() -> SpriteCamera {
    let ltm = DTMatrix { right: [1.0, 0.0, 0.0], up: [0.0, 1.0, 0.0], at: [0.0, 0.0, 1.0], pos: [2.0, 3.0, -5.0] };
    SpriteCamera { ltm, view_window: [0.6, 0.45], near: 0.1, far: 100.0, width: 512, height: 448, z_scale: 0.0, z_shift: 0.0 }
}

fn occ(sample: [u64; 4], cz: f32) -> OccludedSprite {
    OccludedSprite { group: 0, sprite: 0, rect: [0.0; 4], center: [0.0; 2], sample, z: 0.0, recip_z: 1.0 / cz, uv_center: [0.0; 2], texture: None, blend: Some(0x98), rgba: [255; 4], zwrite_off: false, ztest_off: false }
}

#[test]
fn p33_r3_sprite_occ_sample_square() {
    let c = cam();
    let o = occ([254, 222, 257, 225], 12.5);
    // every corner projects back onto its sample pixel at the sprite's camera depth
    let px = [(254.0, 222.0), (257.0, 222.0), (257.0, 225.0), (254.0, 225.0)];
    for (q, (x, y)) in o.SampleQuad(&c).iter().zip(px) {
        let p = c.Transform(*q);
        assert!((p[2] - 12.5).abs() < 1e-4, "depth {p:?}");
        assert!((p[0] / p[2] * 512.0 - x).abs() < 1e-2 && (p[1] / p[2] * 448.0 - y).abs() < 1e-2, "{p:?} vs {x},{y}");
    }
    // screen centre -> on the camera axis
    let m = c.Unproject(256.0, 224.0, 4.0);
    assert!((m[0] - 2.0).abs() < 1e-5 && (m[1] - 3.0).abs() < 1e-5 && (m[2] + 1.0).abs() < 1e-5, "{m:?}");
    // uv: 4:3 view = PS2 pixels / screen size; Hor+ (ow 640) keeps the centre, narrows x
    let uv = o.SampleUv(512.0, 448.0, 512.0);
    assert_eq!(uv, [254.0 / 512.0, 222.0 / 448.0, 257.0 / 512.0, 225.0 / 448.0]);
    let wide = o.SampleUv(512.0, 448.0, 640.0);
    assert!((wide[0] - (0.5 - 2.0 / 640.0)).abs() < 1e-6 && (wide[2] - (0.5 + 1.0 / 640.0)).abs() < 1e-6 && wide[1] == uv[1]);
}

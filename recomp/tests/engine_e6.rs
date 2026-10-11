//! WP-E6: dynamic modules GroundGrip (8) + AnalogMove (5) driven on the real EP1L01 ground.
#![allow(non_snake_case)]
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::pack::*;
use scooby_recomp::engine::physic::dm::analog::*;
use scooby_recomp::engine::physic::{collision_map::*, floor::*, module::*, physic::*, rwm::*};

const ISO: &str = "../Scooby Doo Mystery Mayhem.ISO";

struct W([f32; 2]);
impl PhysicWorld for W {
    fn InputActionGet(&self, _a: u32, _b: u32) -> Option<[f32; 2]> { Some(self.0) }
}

fn ground() -> Option<([f32; 3], [f32; 3])> {
    let fs = IsoFs::open(ISO).ok()?;
    let mut m = EFRessourcesMgr::default();
    m.level_file_name = Some("EP1L01.P2R".into());
    assert_eq!(m.LoadLevel(&fs, "", "EP1/L01", "EN", &mut RawInstanciator, |_, _| {}).unwrap(), 0);
    let mut map = m.objects.iter().filter_map(|o| o.as_raw()).find(|r| r.class_id == 0x24)
        .map(|r| EFCollisionMap::LoadFromRaw(&r.data).unwrap().0).expect("collision map");
    // first object position with a walkable floor below
    for (k, g) in m.game_objects.iter().enumerate() {
        let p = g.glo.pos;
        if p == [0.0; 3] { continue; }
        if let Some((hp, n, _)) = FindFloor(&mut map, [p[0], p[1] + 1.0, p[2]], 20.0, 1 + k as i32) {
            if n[1] > 0.7 { return Some((hp, n)); }
        }
    }
    None
}

#[test]
fn groundgrip_analogmove_on_ep1l01() {
    let Some((pos, n)) = ground() else { eprintln!("ISO missing, skipped"); return };
    eprintln!("floor {pos:?} n {n:?}");
    let mut f = RwMatrix::identity();
    f.pos = pos;
    let mut ph = EFPhysic::new(1, f);
    let mut notes = Vec::new();

    // AnalogMove: xz plane, no camera, scale 2 -> disp = (-x*2, 0, y*2), overwrite velocity
    let w = W([0.5, 0.5]);
    let mut am = EFDMAnalogMove::new();
    let p = [0, 0, 2f32.to_bits(), 2f32.to_bits(), 2f32.to_bits(), 0, 0, 2];
    let mut ctx = DMCtx { phys: &mut ph, frame: 1, paused_frames: 0, prev_paused: 0, world: &w, notify: &mut notes };
    am.SetParams(&mut ctx, Some(&p));
    let mut dp = EFDynamicPhysic::default();
    assert!(am.PreColUpdate(&mut ctx, &mut dp, 1.0 / 60.0));
    assert_eq!(dp.disp, [-1.0, 0.0, 1.0]);
    assert_eq!(ctx.phys.velocity, dp.disp);
    let mut back = [0u32; 8];
    am.GetParams(&ctx, &mut back);
    assert_eq!(back, p);

    // GroundGrip on the real floor normal: full grip -> velocity = disp projected onto the floor,
    // rescaled to |disp|, plus the (clipped) normal part of the incoming velocity
    let mut gg = EFDMGroundGrip::new();
    let gp = [100f32.to_bits(), 0.8f32.to_bits()];
    gg.SetParams(&mut ctx, Some(&gp));
    ctx.phys.velocity = [0.0, -3.0, 0.0];
    let d = EFColDMData { normal: n, other: None, env_friction: Some(1.0), contacts: vec![] };
    gg.ColUpdate(&mut ctx, &EFCollisionStage::seam(&dp), &d, 1.0 / 60.0);
    let v = ctx.phys.velocity;
    let tang = sub(v, scale(n, v[0] * n[0] + v[1] * n[1] + v[2] * n[2]));
    assert!((RwV3dLength(tang) - RwV3dLength(dp.disp)).abs() < 1e-3, "{v:?}");
    assert!(v[0] * n[0] + v[1] * n[1] + v[2] * n[2] <= 1e-4);
    assert_eq!(ctx.phys.floor_cos, 0.8f32.cos());
    // second call same frame: no-op
    ctx.phys.velocity = [9.0; 3];
    gg.ColUpdate(&mut ctx, &EFCollisionStage::seam(&dp), &d, 1.0 / 60.0);
    assert_eq!(ctx.phys.velocity, [9.0; 3]);
}

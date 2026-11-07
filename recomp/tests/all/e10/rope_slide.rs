// WP-E10 EFDMRopeSlide tests: synthetic straight ropes + real EP1L01 splines.
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::pack::*;
use scooby_recomp::engine::physic::dm::rope_slide::*;
use scooby_recomp::engine::physic::module::*;
use scooby_recomp::engine::physic::physic::EFPhysic;
use scooby_recomp::engine::physic::rwm::*;
use scooby_recomp::engine::spline::*;
use crate::common::ISO;

struct W(Vec<EFQuadraticSpline>, Option<[f32; 2]>);
impl PhysicWorld for W {
    fn Splines(&self) -> &[EFQuadraticSpline] { &self.0 }
    fn InputActionGet(&self, _a: u32, _b: u32) -> Option<[f32; 2]> { self.1 }
}

/// straight rope from a to b, one segment, type bit 2
fn rope(a: V3, b: V3, id: u16) -> EFQuadraticSpline {
    let d = sub(b, a);
    let l = RwV3dLength(d);
    let seg = SplineSegment { p: [a, add(a, scale(d, 0.5)), b], length: l, distance: 0.0, b: d, n: [0.0, 0.0, l * l], ..Default::default() };
    EFQuadraticSpline { id, ty: 4, length: l, center: a, extents: [l; 3], segs: vec![seg] }
}

fn step(m: &mut EFDMRopeSlide, ctx: &mut DMCtx, dt: f32) {
    assert!(m.PreColUpdate(ctx, &mut EFDynamicPhysic::default(), dt));
    // stand-in for the physics integration between Pre/PostColUpdate
    ctx.phys.frame.pos = add(ctx.phys.frame.pos, scale(ctx.phys.velocity, dt));
    m.PostColUpdate(ctx, None, dt);
}

#[test]
fn rope_attach_slide_and_end() {
    let w = W(vec![rope([0.0, 10.0, 0.0], [0.0, 5.0, 20.0], 5)], Some([0.0, 0.0]));
    let mut n = vec![];
    let mut p = EFPhysic::new(1, RwMatrix::from_pos([0.0, 9.0, 1.0]));
    let mut m = EFDMRopeSlide::new();
    m.base.notify_bits = !0;
    let mut ctx = DMCtx { phys: &mut p, frame: 0, paused_frames: 0, prev_paused: 0, world: &w, notify: &mut n };
    m.DoAction(&mut ctx, 1, &mut [5, 1]);
    assert_eq!(m.cursor.spline, Some(0));
    assert!((ctx.phys.frame.pos[1] - 10.045).abs() < 1e-5);
    assert!(ctx.phys.frame.at[2] > 0.9 && ctx.phys.frame.at[1] < 0.0);
    assert!(ctx.phys.frame.up[1] > 0.9);
    let z0 = ctx.phys.frame.pos[2];
    step(&mut m, &mut ctx, 0.1);
    assert!(m.speed >= 4.0 && m.speed <= 8.0, "{}", m.speed);
    assert!(ctx.phys.frame.pos[2] > z0, "moved along the rope");
    assert_eq!(ctx.phys.falling_timer, ctx.phys.falling_delay);
    assert_eq!(m.base.state & 1, 1, "hanging at the rest distance");
    for _ in 0..100 {
        step(&mut m, &mut ctx, 0.1);
    }
    assert_eq!(m.base.state & 2, 2, "end of rope without links");
    drop(ctx);
    assert!(n.iter().any(|x| x.code == 6 && x.data[2] & 2 != 0));
}

#[test]
fn rope_links_to_best_next_spline() {
    let a = rope([0.0, 10.0, 0.0], [0.0, 9.0, 4.0], 5);
    let fwd = rope([0.0, 9.0, 4.0], [0.0, 8.0, 10.0], 6);
    let side = rope([0.0, 9.0, 4.0], [6.0, 8.0, 4.0], 7);
    let w = W(vec![a, side, fwd], None);
    let mut n = vec![];
    let mut p = EFPhysic::new(1, RwMatrix::identity());
    let mut m = EFDMRopeSlide::new();
    m.links = SplineLinks { group_of: vec![0, 1, 1], groups: vec![vec![1, 2], vec![]] };
    let mut ctx = DMCtx { phys: &mut p, frame: 0, paused_frames: 0, prev_paused: 0, world: &w, notify: &mut n };
    m.DoAction(&mut ctx, 1, &mut [5, 1]);
    let mut moved = false;
    for _ in 0..30 {
        step(&mut m, &mut ctx, 0.1);
        if m.cursor.spline != Some(0) {
            moved = true;
            break;
        }
    }
    assert!(moved);
    // score = |t.up| t.up + (1 - t.right)^2: right = up x at = (+1,0,0) for at +z, up +y, so
    // the side rope (+x, score ~0) loses to the forward one ((1-0)^2 + small)
    assert_eq!(m.cursor.spline, Some(2));
    assert_eq!(m.base.state & 2, 0);
}

#[test]
fn rope_real_ep1l01_splines() {
    let Ok(fs) = IsoFs::open(ISO) else { eprintln!("ISO missing, skipped"); return };
    let mut r = EFRessourcesMgr::default();
    r.level_file_name = Some("EP1L01.P2R".into());
    assert_eq!(r.LoadLevel(&fs, "", "EP1/L01", "EN", &mut RawInstanciator, |_, _| {}).unwrap(), 0);
    let mut sm = EFSplineMgr::default();
    sm.LoadFromLevel(&r);
    let Some(mut s) = sm.splines.iter().find(|s| s.segs.len() > 1).cloned() else { eprintln!("no splines"); return };
    eprintln!("spline id {} ty {:#x} len {} segs {}", s.id, s.ty, s.length, s.segs.len());
    s.ty = (s.ty | 4) & !8;
    let start = s.GetPoint(0, 0.0);
    let id = s.id as u32;
    let w = W(vec![s], Some([0.0, 1.0]));
    let mut n = vec![];
    let mut p = EFPhysic::new(1, RwMatrix::from_pos(start));
    let mut m = EFDMRopeSlide::new();
    let mut ctx = DMCtx { phys: &mut p, frame: 0, paused_frames: 0, prev_paused: 0, world: &w, notify: &mut n };
    m.DoAction(&mut ctx, 1, &mut [id, 0]);
    let mut last = m.cursor.cur.global;
    for _ in 0..20 {
        step(&mut m, &mut ctx, 1.0 / 30.0);
        let g = m.cursor.cur.global;
        assert!(g >= last - 1e-4 || m.base.state & 2 != 0, "monotonic slide {last} -> {g}");
        last = g;
        assert!(ctx.phys.frame.pos.iter().all(|x| x.is_finite()));
    }
    let d = RwV3dLength(sub(ctx.phys.frame.pos, m.cursor.point));
    assert!(d < 1.0, "hangs near the rope: {d}");
}

use scooby_recomp::engine::camera::*;
use scooby_recomp::engine::particle::*;
use scooby_recomp::engine::spline::*;

#[test]
fn particle_type_emitter_render() {
    let mut m = PLMgr_BC::new();
    m.textures = Some(vec!["glow".to_string()]);
    let t = m.ParticleTypeCreate();
    assert_eq!(t, 0x2d80_0001);
    let ti = m.GetType(t as i32).unwrap();
    {
        let d = &mut m.types[ti].d;
        d.SetLifespan(2.0);
        d.count = 3;
        d.speed = 1.0;
        assert_eq!(d.SetTexture("GLOW", Some(&["glow".to_string()])), 0);
        d.SetColor(0xf, 1.0, 1.0, 1.0, 1.0);
        d.SetScale(1.0, 1.0, 1.0, 1.0);
    }
    let e = m.ParticleEmitterCreate(t as i32, None);
    assert_eq!(e, 0x2e80_0001);
    m.SyncFrames(&PLNoWorld);
    {
        let em = m.emitter_mut(e as i32).unwrap();
        em.frame = Some(PLFrame { attach: PLFrameAttach::Own, local: DTMatrix::default(), ltm: DTMatrix::default() });
        em.Start(0);
    }
    let mut cams = EFGraphicsMgrCameras::new(640, 448);
    m.Update(0.1, false, &mut cams);
    assert_eq!(m.nb_particles, 3);
    let dl = m.Render();
    let n: usize = dl.batches.iter().map(|b| match &b.prims { PLPrims::Sprites(v) => v.len(), PLPrims::Quads(v) => v.len() }).sum();
    assert_eq!(n, 3);
    assert_eq!(dl.batches[0].texture, "glow");
    assert_eq!(m.ParticleEmitterDelete(e as i32), 0);
    assert_eq!(m.ParticleEmitterDelete(e as i32), 2);
    assert_eq!(m.ParticleTypeDelete(t as i32), 0);
}

#[test]
fn spline_sphere_and_nearest() {
    let seg = SplineSegment { p: [[0.0; 3], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]], length: 2.0, b: [2.0, 0.0, 0.0], ..Default::default() };
    let s = EFQuadraticSpline { length: 2.0, segs: vec![seg], ..Default::default() };
    let g = s.GetNearestFrom(0, [1.0, 1.0, 0.0], None);
    assert!((0.0..=1.0).contains(&g), "{g}");
    let mut out = 0.0;
    let _ = s.CheckSphereIntersection([1.0, 0.5, 0.0], 1.0, &mut out);
}

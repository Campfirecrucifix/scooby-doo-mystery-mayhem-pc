//! WP-K2: EFCutScenePlayer loaded from the real EP1L01 pack (EF_CUTSCENE) and played through the
//! runtime: actor animations start, notification 0x16 {id,1}/{id,0} reaches the logic, the book
//! handover is NOT part of the EP1L01 data (see ENGINE_PORT.md WP-K2).
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::runtime::*;
use scooby_recomp::engine::script::*;

const ISO: &str = "../Scooby Doo Mystery Mayhem.ISO";
const BOOK: u32 = 0x2a00_0093;

#[test]
fn ep1l01_cutscene_data() {
    let fs = IsoFs::open(ISO).expect("ISO missing: tests must load real data");
    let res = EngineRuntime::ReadLevel(&fs, "EP1/L01", "EP1L01.P2R").unwrap();
    let p = EFCutScenePlayer::FromLevelObjects(&res.objects).unwrap();
    let sc = p.scenes.as_ref().expect("EF_CUTSCENE loaded");
    assert_eq!(sc.len(), 1);
    assert!((sc[0].duration - 25.333334).abs() < 1e-4);
    assert_eq!(sc[0].actors, vec![(0x2a000006, 0xa), (0x2a000092, 0x3e), (0x2a000091, 0x38), (0x2a00008f, 6)]);
    assert!(p.IsInCutScene(0x2a000092, 0) && !p.IsInCutScene(BOOK, 0) && !p.IsInCutScene(0x2a000092, 1));
}

#[test]
fn ep1l01_runs_cutscene_no_standins() {
    let fs = IsoFs::open(ISO).expect("ISO missing: tests must load real data");
    let (mut e, mut rt) = EngineRuntime::new_with_engine();
    let res = EngineRuntime::ReadLevel(&fs, "EP1/L01", "EP1L01.P2R").unwrap();
    let mut cs = EFCutScenePlayer::FromLevelObjects(&res.objects).unwrap();
    rt.LoadLevel(&mut e, res);
    let mut notified = vec![];
    let mut playing_mid = false;
    let mut ended = None;
    for f in 0..1700u32 {
        rt.Frame(&mut e, 1.0 / 60.0);
        let mut h = RuntimeCutSceneHost::new(&mut e, &rt);
        if f == 10 && std::env::var("K2_NOPLAY").is_err() {
            // what l01_camera does on msg 0x1de (sent by inv_tutorial): PlugNPlayCutScene(0, snd, cam)
            cs.APIPlayCutScene(&mut h, 0, 0xffff_ffff);
        }
        cs.Update(&mut h, 1.0 / 60.0);
        notified.extend(h.notified);
        if f == 600 {
            playing_mid = J2(&rt.particles).anim_is_playing(0x2a000092, 0x3e) != 0 && cs.flags & 1 != 0;
        }
        if ended.is_none() && cs.flags & 1 == 0 && f > 10 {
            ended = Some(f);
        }
    }
    assert_eq!(notified, vec![(0x16, 0, 1), (0x16, 0, 0)]);
    assert!(playing_mid, "Scooby's cut-scene anim 0x3e not playing mid-scene");
    let ended = ended.unwrap();
    // 25.33 s at 60 Hz + one grace frame
    assert!((1528..=1532).contains(&ended), "ended at {ended}");
    assert_eq!(J2(&rt.particles).anim_is_playing(0x2a000092, 0x3e), 0);
    // original behaviour: nothing in the EP1L01 data hands the book over (GB2 bit 0 stays clear)
    let gb2 = e.rt.game_buffers[&2].0;
    assert_eq!(e.mem.r32(gb2) & 1, 0, "book handed over without a stand-in?");
    let _ = BOOK;
}

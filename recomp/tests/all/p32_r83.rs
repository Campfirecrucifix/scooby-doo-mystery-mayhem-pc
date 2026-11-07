//! P32 R83: P33 table B #4 (SetMyCurrentClipping anim part) + first-level probe extensions.
use crate::common::{frame, load_carried, ISO};
use scooby_recomp::engine::iso::IsoFs;

/// EFGameObject::SetMyCurrentClipping (EFGameObject.cpp:1152-1162) = EFAnimationMgr::SetClipMode: word 0x20 ->
/// mode 0x20 (no Evaluate), else 0x40 -> 0x40 (time-only Evaluate), else 0. Unknown uid: no-op.
#[test]
fn p32_r83_set_anim_clip_mode() {
    use scooby_recomp::engine::anim::mgr::EFAnimationMgr;
    use scooby_recomp::engine::host::gameobj::SetAnimClipMode;
    let mut m = std::collections::HashMap::new();
    m.insert(0x2a00_0001u32, EFAnimationMgr::new(0, 0));
    for (w, want) in [(0x60u32, 0x20u32), (0x40, 0x40), (0x4a7, 0x20), (0x400, 0), (0x40 | 0x80, 0x40), (0, 0)] {
        SetAnimClipMode(&mut m, 0x2a00_0001, w);
        assert_eq!(m[&0x2a00_0001].mode, want, "word {w:#x}");
    }
    SetAnimClipMode(&mut m, 0x2a00_0002, 0x20);
    assert!(!m.contains_key(&0x2a00_0002));
}

/// Reachability evidence (prints only): per first level, objects whose clipping params hold a band word with
/// 0x40 (time-only anim, no UpdateHierarchy) or 0x20 (no anim), i.e. what GAME_LOGICCLIP (interactive default) can pick.
#[test]
#[ignore]
fn p32_r83_probe_clip_anim_words() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    for lvl in ["EP1/L01", "EP2/L01A", "EP3/L01", "EP4/L01", "EP6/L01"] {
        let (mut e, mut rt) = load_carried(&fs, lvl, &[]).unwrap();
        for _ in 0..5 {
            frame(&mut e, &mut rt, 0);
        }
        let g = rt.particles.lock().unwrap();
        let has = |b: u32| g.s1.clip.iter().filter(|(_, c)| (0..5).any(|i| (c[2 + 3 * i] | c[3 + 3 * i]) & b != 0)).count();
        println!("P83 {lvl}: {} clip params, 0x40 words {}, 0x20 words {}", g.s1.clip.len(), has(0x40), has(0x20));
    }
}

/// EP6/L02 0x2a0000f8 Play(1, 2) at f1 without an anim mgr: what the object is (prints only)
#[test]
#[ignore]
fn p32_r83_probe_ep6_l02_f8() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = load_carried(&fs, "EP6/L02", &[]).unwrap();
    for _ in 0..3 {
        frame(&mut e, &mut rt, 0);
    }
    let u = 0x2a00_00f8u32;
    let o = e.rt.objects.iter().flatten().find(|o| o.id == u);
    println!("P83 f8 logic obj {} code {:?}", o.is_some(), o.and_then(|o| o.inst.code).map(|k| e.rt.codes[k].def.name.to_string()));
    let g = rt.particles.lock().unwrap();
    println!("P83 f8 obj3d {:x?} anim {} hidden {} enabled {}", g.sub.obj3d.get(&u), g.sub.anim.contains_key(&u), g.world.hidden.contains(&u), g.s1.is_enabled(u));
    let callers: Vec<_> = g.sub.log.anim_play.iter().filter(|x| x.0 == u).collect();
    println!("P83 f8 plays {callers:x?}");
}

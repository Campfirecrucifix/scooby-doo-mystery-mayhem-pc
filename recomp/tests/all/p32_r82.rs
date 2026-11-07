//! P32 R82: sound / animation play calls of each episode's first level vs the C call sites.
use crate::common::{frame, load_carried, ISO};
use scooby_recomp::engine::iso::IsoFs;

/// Probe (ignored, `P82_FRAMES`, default 900): every APIanim::Play / APInewSound::Play the logic makes in the
/// first 900 frames of EP1/L01, EP2/L01A, EP3/L01, EP4/L01, EP6/L01 (idle pad), grouped by object code name,
/// with the failures (anim not started / sound code != 0).
#[test]
#[ignore]
fn p32_r82_probe_first_level_plays() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let n: u32 = std::env::var("P82_FRAMES").ok().and_then(|s| s.parse().ok()).unwrap_or(900);
    let lv = std::env::var("P82_LEVELS").unwrap_or("EP1/L01,EP2/L01A,EP3/L01,EP4/L01,EP6/L01".into());
    for lvl in lv.split(',') {
        let (mut e, mut rt) = load_carried(&fs, lvl, &[]).unwrap();
        let mut ran = 0;
        // P82_ROUTE=1: play the level's p17 route (route_steps, ANIMSND variants) instead of idling
        let route = std::env::var_os("P82_ROUTE").is_some().then(|| {
            use crate::engine_p17::*;
            [routes_ep2::EP2_ROUTE, routes_ep3::EP3_ROUTE, routes_ep4::EP4_ROUTE, routes_ep6::EP6_ROUTE].iter().flat_map(|r| r.iter()).find(|r| r.0 == lvl).map(|r| route_steps(lvl, r.1))
        }).flatten();
        if let Some(st) = route {
            let r = crate::engine_p17::run(&mut e, &mut rt, &st);
            println!("P82 {lvl} route -> {r:?}");
        } else {
            for _ in 0..n {
                ran += 1;
                if frame(&mut e, &mut rt, 0).is_some() { break }
            }
        }
        let name = |u: u32| e.rt.objects.iter().flatten().find(|o| o.id == u).and_then(|o| o.inst.code).map_or(format!("{u:#x}"), |k| e.rt.codes[k].def.name.to_string());
        let g = rt.particles.lock().unwrap();
        let mut anims = std::collections::BTreeMap::<String, std::collections::BTreeMap<(u32, bool), u32>>::new();
        for &(o, a, _, s) in &g.sub.log.anim_play { *anims.entry(format!("{} {o:#x}", name(o))).or_default().entry((a, s)).or_default() += 1 }
        let mut snds = std::collections::BTreeMap::<String, std::collections::BTreeMap<(u32, u32), u32>>::new();
        for &(o, i, r) in &g.sub.log.sound_play { *snds.entry(format!("{} {o:#x}", name(o))).or_default().entry((i, r)).or_default() += 1 }
        println!("P82 == {lvl} ran {ran} frames");
        for (k, v) in &anims { println!("P82 {lvl} anim {k}: {:x?}", v) }
        for (k, v) in &snds { println!("P82 {lvl} snd {k}: {:x?}", v) }
        for &(o, i, r) in g.sub.log.sound_play.iter().filter(|x| x.2 != 0) {
            let refs: Vec<u32> = g.sub.audio.objects.get(&o).map(|m| m.refs.iter().map(|r| r.id).collect()).unwrap_or_default();
            println!("P82 {lvl} FAIL snd {} {o:#x} id {i:#x} code {r}: object sound list {refs:x?}", name(o));
        }
        for &(o, a, f, _) in g.sub.log.anim_play.iter().filter(|x| !x.3) {
            let m = g.sub.anim.get(&o).map(|m| (m.base, m.anims.len(), m.anims.get(a as usize).map(|x| x.is_some())));
            println!("P82 {lvl} FAIL anim {} {o:#x} id {a:#x} flags {f:#x}: mgr (base, n_anims, slot) {m:x?}", name(o))
        }
    }
}

/// P32 R82 (P33 table B #3): APIgameobj::AddChild flag 0x10 (EFPosData.cpp:153-157): the child copies the
/// parent's clipping params and SetMyCurrentClipping(child, parent +0x138); flags 2/4 send the attach messages
/// (0x10000001 to the parent / 0x10000002 to the child, core::apigameobj::AddChild) without disturbing the link.
/// Flag 1 alone leaves the child's clipping alone. (No module passes 2/4/0x10: all 1786 C call sites use 1 or 9.)
#[test]
fn p32_r82_addchild_flag10_inherits_clipping() {
    use scooby_recomp::engine::APIgameobj;
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = load_carried(&fs, "EP1/L01", &[]).unwrap();
    for _ in 0..2 {
        frame(&mut e, &mut rt, 0);
    }
    let ids: Vec<u32> = e.rt.objects.iter().flatten().map(|o| o.id).filter(|&u| u >> 24 == 0x2a && rt.particles.lock().unwrap().world.frames.contains_key(&u)).take(3).collect();
    let [p, c, d] = ids[..] else { panic!("objects {ids:x?}") };
    let mut pc = scooby_recomp::engine::host::gameobj::CLIP_DEFAULT;
    pc[1] = 123.0f32.to_bits();
    {
        let mut g = rt.particles.lock().unwrap();
        g.s1.clip.insert(p, pc);
        g.s1.clip_cur.insert(p, 0x180);
        g.logic = true;
    }
    let before_d = rt.particles.lock().unwrap().s1.clip.get(&d).copied();
    APIgameobj::AddChild(&mut e, p, 0x7fff_ffff, c, 0x10 | 6 | 1);
    APIgameobj::AddChild(&mut e, p, 0x7fff_ffff, d, 1);
    let g = rt.particles.lock().unwrap();
    assert!(g.parented.iter().any(|x| x.0 == c && x.1 == p), "child linked");
    assert_eq!(g.s1.clip.get(&c), Some(&pc), "flag 0x10 copies the parent's clipping params");
    assert_eq!(g.s1.clip_cur.get(&c), Some(&0x180), "flag 0x10: SetMyCurrentClipping(child, parent +0x138)");
    assert_eq!(g.s1.clip.get(&d).copied(), before_d, "flag 1: clipping untouched");
}

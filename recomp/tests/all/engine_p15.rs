//! WP-P15: collectable placement. Sweep every level pack: collectable-code objects (logic code name
//! containing "coll") whose pack frame (EF_GLO POS) is within 1 m of the origin, plus their local
//! words [0x19..0x1b] (Coll_base MyMessageFunction 0xc1 SetTranslation target).
//! `cargo test --release --test engine_p15 -- --ignored --nocapture` (P15_LEVEL=EP2/L02A for one pack).
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::runtime::*;

use crate::common::ISO;

fn packs(fs: &IsoFs) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = fs
        .names()
        .filter(|n| n.to_uppercase().starts_with("LEVEL/") && n.to_uppercase().ends_with(".P2R"))
        .filter_map(|n| {
            let parts: Vec<&str> = n.split('/').collect();
            let file = parts.last()?.to_string();
            // LEVEL/<dir...>/GEN/<file>
            let dir = parts[1..parts.len().saturating_sub(2)].join("/");
            (!dir.is_empty()).then_some((dir, file))
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
#[ignore]
fn p15_collectables_at_origin() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let only = std::env::var("P15_LEVEL").ok();
    let list = packs(&fs);
    println!("P15 {} packs", list.len());
    for (dir, file) in list {
        if only.as_deref().is_some_and(|o| !o.eq_ignore_ascii_case(&dir)) {
            continue;
        }
        let r = std::panic::catch_unwind(|| {
            let (mut e, mut rt) = EngineRuntime::new_with_engine();
            let res = EngineRuntime::ReadLevel(&fs, &dir, &file)?;
            if std::env::var("P15_CS").is_ok() {
                if let Ok(cs) = scooby_recomp::engine::script::EFCutScenePlayer::FromLevelObjects(&res.objects) {
                    for (i, s) in cs.scenes.iter().flatten().enumerate() {
                        println!("P15 cs {i} actors {:x?}", s.actors);
                    }
                }
            }
            rt.LoadLevel(&mut e, res);
            let frames0 = rt.particles.lock().unwrap().world.frames.clone();
            for _ in 0..2 {
                rt.Frame(&mut e, 1.0 / 60.0);
            }
            let frames = rt.particles.lock().unwrap().world.frames.clone();
            if let Ok(w) = std::env::var("P15_NEAR") {
                let c: Vec<f32> = w.split(',').map(|s| s.parse().unwrap()).collect();
                for o in e.rt.objects.iter().flatten() {
                    if let Some(m) = frames.get(&o.id) {
                        let d = ((m.pos[0] - c[0]).powi(2) + (m.pos[2] - c[2]).powi(2)).sqrt();
                        if d < c[3] {
                            let n = o.inst.code.map(|k| e.rt.codes[k].def.name.clone()).unwrap_or_default();
                            println!("P15 near {:#x} {n} {:?}", o.id, m.pos);
                        }
                    }
                }
            }
            if let Ok(w) = std::env::var("P15_FIND") {
                for t in w.split(',') {
                    let t = u32::from_str_radix(t, 16).unwrap();
                    for o in e.rt.objects.iter().flatten() {
                        let l = o.inst.local;
                        if l == 0 { continue }
                        for i in 0..0x100u32 {
                            if e.mem.r32(l + 4 * i) == t {
                                let n = o.inst.code.map(|k| e.rt.codes[k].def.name.clone()).unwrap_or_default();
                                println!("P15 find {t:#x} in {:#x} {n} local+{:#x}", o.id, 4 * i);
                            }
                        }
                    }
                }
            }
            let mut out = vec![];
            let mut n = 0;
            for o in e.rt.objects.iter().flatten() {
                let Some(k) = o.inst.code else { continue };
                let name = e.rt.codes[k].def.name.to_lowercase();
                if !name.contains("coll") {
                    continue;
                }
                n += 1;
                let p = frames.get(&o.id).map(|m| m.pos).unwrap_or([0.0; 3]);
                if p.iter().map(|c| c * c).sum::<f32>() < 1.0 {
                    let l = o.inst.local;
                    let p0 = frames0.get(&o.id).map(|m| m.pos);
                    if l == 0 {
                        out.push(format!("{:#x} {name} pos {p:?} pack {p0:?} (no locals)", o.id));
                        continue;
                    }
                    let t: Vec<f32> = (0..3).map(|i| f32::from_bits(e.mem.r32(l + 0x64 + 4 * i))).collect();
                    out.push(format!("{:#x} {name} pos {p:?} pack {p0:?} local[0]={:x} [1]={:x} [0x19..]={t:?}", o.id, e.mem.r32(l), e.mem.r32(l + 4)));
                } else if std::env::var("P15_ALL").is_ok() {
                    out.push(format!("{:#x} {name} ok {p:?}", o.id));
                }
            }
            Ok::<_, String>((n, out))
        });
        match r {
            Ok(Ok((n, out))) => {
                println!("P15 {dir}: {n} coll objects, {} at origin", out.len());
                for s in out {
                    println!("P15   {s}");
                }
            }
            Ok(Err(e)) => println!("P15 {dir}: load error {e}"),
            Err(_) => println!("P15 {dir}: PANIC"),
        }
    }
}

/// Coll_base MyMessageFunction 0xd5 (falling key / catapult / tome drop): stores its position into
/// local[0x25..0x27] with lwc1/swc1 (MIPS 0x127e4..0x12808 in coll_level2a), which RunBob reads as
/// floats. The port cast the floats to int (Ghidra `(int)` misprint): after 0xd5 the collectable
/// bobbed at the bit pattern of (131, 0, 257) = ~(0, 0.1, 0) (EP2/L02A 0x2a000049 "at the origin").
#[test]
fn p15_d5_keeps_position() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    use scooby_recomp::engine::APIgameobj;
    let (mut e, mut rt) = EngineRuntime::new_with_engine();
    rt.LoadLevel(&mut e, EngineRuntime::ReadLevel(&fs, "EP2/L02A", "EP2L02A.P2R").unwrap());
    for _ in 0..2 {
        rt.Frame(&mut e, 1.0 / 60.0);
    }
    let id = 0x2a000049;
    let before = rt.particles.lock().unwrap().world.frames.get(&id).unwrap().pos;
    rt.particles.lock().unwrap().logic = true;
    APIgameobj::SendMessageTo(&mut e, id, 0, 0xd5, 0, 0);
    rt.particles.lock().unwrap().logic = false;
    for _ in 0..10 {
        rt.Frame(&mut e, 1.0 / 60.0);
    }
    let after = rt.particles.lock().unwrap().world.frames.get(&id).unwrap().pos;
    println!("P15 0xd5 {id:#x}: {before:?} -> {after:?}");
    assert!(before[0].abs() > 10.0, "0x49 starts at its pack position");
    assert!((after[0] - before[0]).abs() < 0.01 && (after[2] - before[2]).abs() < 0.01 && (after[1] - before[1]).abs() < 0.2, "bobs in place: {after:?}");
}

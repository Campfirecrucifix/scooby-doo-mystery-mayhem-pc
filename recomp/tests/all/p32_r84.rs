//! P32 R84: play-feel check of the P33 B#4 anim clip modes (SetMyCurrentClipping 0x20/0x40) under GAME_LOGICCLIP=1.
use crate::common::{frame_stick, load_carried, FRAME_HOOK, ISO};
use scooby_recomp::engine::iso::IsoFs;
use std::collections::HashMap;

/// Probe (ignored; run with GAME_LOGICCLIP=1, `P84_LEVELS`, default EP1/L01,EP3/L01): 3000 frames with the hero
/// walking (stick turned every 150 f) so the camera crosses clip bands. Every anim-mode change 0x20/0x40 -> 0 is
/// a resume; 5 frames later it prints whether the pose moved (no frozen pose) and how far the object frame moved
/// (no snapping). Summary line: resumes / frozen / snaps (> 1.0 unit in 5 frames).
#[test]
#[ignore]
fn p32_r84_probe_animclip_resume() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let lv = std::env::var("P84_LEVELS").unwrap_or("EP1/L01,EP3/L01".into());
    for lvl in lv.split(',') {
        let (mut e, mut rt) = load_carried(&fs, lvl, &[]).unwrap();
        type St = (HashMap<u32, u32>, HashMap<u32, (u32, Option<u64>, [f32; 3])>, [u32; 4], u32);
        let st = std::rc::Rc::new(std::cell::RefCell::new(St::default()));
        let s2 = st.clone();
        FRAME_HOOK.with(|h| {
            let watch = std::env::var("P84_UID").ok().and_then(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok());
            *h.borrow_mut() = Some(Box::new(move |e, rt| {
                if let Some(w) = watch.filter(|_| s2.borrow().3 < 40) {
                    let n = e.rt.objects.iter().flatten().find(|o| o.id == w).and_then(|o| o.inst.code).map_or("?".to_string(), |k| e.rt.codes[k].def.name.to_string());
                    let g = rt.particles.lock().unwrap();
                    println!("P84 watch f{} {w:#x} {n} pos {:.1?} mode {:?}", s2.borrow().3, g.world.frames.get(&w).map(|m| m.pos), g.sub.anim.get(&w).map(|m| m.mode));
                }
                let g = rt.particles.lock().unwrap();
                let mut s = s2.borrow_mut();
                let (modes, pend, cnt, f) = &mut *s;
                *f += 1;
                let sig = |u: u32| -> Option<u64> {
                    let p = g.sub.anim.get(&u)?.GetPose(g.sub.anim_bind.get(&u)?.obj3d)?;
                    Some(p.iter().flat_map(|t| t.f.iter().chain(t.quat.iter())).fold(0u64, |h, x| h.rotate_left(5) ^ x.to_bits() as u64))
                };
                let pos = |u: u32| g.world.frames.get(&u).map_or([0.0; 3], |m| m.pos);
                pend.retain(|&u, (n, s0, p0)| {
                    *n -= 1;
                    if *n > 0 { return true }
                    let p = pos(u);
                    let d = ((p[0] - p0[0]).powi(2) + (p[1] - p0[1]).powi(2) + (p[2] - p0[2]).powi(2)).sqrt();
                    let moved = sig(u) != *s0;
                    if !moved { cnt[1] += 1 }
                    if d > 1.0 { cnt[2] += 1 }
                    println!("P84 f{f} {u:#x} resumed+5 pose_changed {moved} dpos {d:.3} from {p0:.1?} to {p:.1?} anims {}", g.sub.anim_bind.get(&u).map_or(0, |b| b.anims.len()));
                    false
                });
                for (&u, m) in &g.sub.anim {
                    let old = modes.insert(u, m.mode).unwrap_or(m.mode);
                    if old != m.mode {
                        cnt[3] += 1;
                        println!("P84 f{f} {u:#x} mode {old:#x} -> {:#x}", m.mode);
                        if m.mode == 0 {
                            cnt[0] += 1;
                            pend.insert(u, (5, sig(u), pos(u)));
                        }
                    }
                }
            }));
        });
        let dirs = [(128u8, 0u8), (255, 128), (128, 255), (0, 128)];
        for i in 0..3000u32 {
            if frame_stick(&mut e, &mut rt, 0, dirs[(i / 150) as usize % 4]).is_some() { break }
        }
        FRAME_HOOK.with(|h| *h.borrow_mut() = None);
        let s = st.borrow();
        println!("P84 == {lvl}: {} frames, {} mode changes, {} resumes, {} frozen after resume, {} snaps", s.3, s.2[3], s.2[0], s.2[1], s.2[2]);
    }
}

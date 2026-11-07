//! WP-J21: book open/close anims and the book <-> hero hook on EP1L01.
//! TomeDoomFUNC_SetMasterParams (build 802a33e5): leader = goHeroesData[0] -> book hook bone 9
//! (ui32JointHookMasterID 9, slave 0xa); TomeDoomCB_PHYSIC translates the book so that hook bone
//! lands on the hero's position (GetPosition(hero), not a hand bone: the hands are posed by the
//! hero's book anim around that point).
use scooby_recomp::engine::input::{pad, PadState};
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::gfx::api::GfxWorld;
use scooby_recomp::engine::runtime::*;

use crate::common::ISO;
const SHAGGY: u32 = 0x2a00_0091;
const SCOOBY: u32 = 0x2a00_0092;
const BOOK: u32 = 0x2a00_0093;
const GHOST: u32 = 0x2a00_0008;

fn pos(rt: &EngineRuntime, id: u32) -> [f32; 3] {
    let g = rt.particles.lock().unwrap();
    let ph = g.sub.physic.PhysicOf(id);
    g.sub.physic.Physic(ph).unwrap().frame.pos
}

#[test]
fn ep1l01_book_anims_and_hook() {
    use scooby_recomp::engine::core::apigameobj::GetCurrentStateID;
    let fs = IsoFs::open(ISO).expect("ISO missing: tests must load real data");
    let (mut e, mut rt) = EngineRuntime::new_with_engine();
    let res = EngineRuntime::ReadLevel(&fs, "EP1/L01", "EP1L01.P2R").unwrap();
    rt.LoadLevel(&mut e, res);
    let mut book_states: Vec<(u32, u32)> = vec![];
    let mut max_hook_err = 0f32;
    let mut hook_samples = 0;
    let mut captured = None;
    for f in 0..1500u32 {
        if f == 10 {
            rt.particles.lock().unwrap().logic = true;
            e.SendLogicMessageTo(BOOK, 0xe5, 0, 0, 0);
            let gb1 = e.rt.game_buffers[&1].0;
            e.mem.w32(gb1 + 4, 180f32.to_bits());
            rt.particles.lock().unwrap().logic = false;
        }
        if f == 100 {
            let mut g = rt.particles.lock().unwrap();
            let ph = g.sub.physic.PhysicOf(SCOOBY);
            let hp = g.sub.physic.Physic(ph).unwrap().frame;
            let gp = g.sub.physic.PhysicOf(GHOST);
            let t = [hp.pos[0] + hp.at[0] * 3.0, hp.pos[1] + 1.0, hp.pos[2] + hp.at[2] * 3.0];
            if let Some(p) = g.sub.physic.PhysicMut(gp) {
                p.frame.pos = t;
            }
            if let Some(fr) = g.world.frames.get_mut(&GHOST) {
                fr.pos = t;
            }
        }
        {
            let mut g = rt.particles.lock().unwrap();
            let mut p = PadState { connected: true, mode: 7, rx: 128, ry: 128, lx: 128, ly: 128, ..Default::default() };
            if (120..130).contains(&f) {
                p.buttons = pad::SQUARE;
            }
            if f > 145 && f % 2 == 0 {
                let h = e.rt.map.find(BOOK).unwrap();
                let l = e.rt.objects[h].as_ref().unwrap().inst.local;
                p.buttons = match e.mem.r32(l + 0x1e8) {
                    2 => pad::CROSS,
                    4 => pad::CIRCLE,
                    8 => pad::TRIANGLE,
                    _ => 0,
                };
            }
            g.devices.SetPadState(0, p);
        }
        rt.Frame(&mut e, 1.0 / 60.0);
        let bs = GetCurrentStateID(&mut e, BOOK);
        if book_states.last().map(|x| x.1) != Some(bs) {
            book_states.push((f, bs));
        }
        // book held (states 2..4): with both heroes holding it (L+0x1c0) TomeDoomCB_PHYSIC moves the
        // book so the midpoint of hook bones 9/0xa is the midpoint of the two heroes' positions
        if (2..=4).contains(&bs) {
            let (h1, h2) = (pos(&rt, SCOOBY), pos(&rt, SHAGGY));
            let g = rt.particles.lock().unwrap();
            if let (Some(m), Some(sl)) = (g.world.GetObjectFrame(BOOK, 9), g.world.GetObjectFrame(BOOK, 10)) {
                let d: f32 = (0..3).map(|i| ((m.pos[i] + sl.pos[i]) * 0.5 - (h1[i] + h2[i]) * 0.5).powi(2)).sum::<f32>().sqrt();
                if f % 20 == 0 || (bs >= 3 && d > 0.02) {
                    eprintln!("f{f} st {bs} mid err {d} bone9 {:?} bone10 {:?} scooby {h1:?} shaggy {h2:?}", m.pos, sl.pos);
                }
                // checked from the 2nd frame of state 3 on (the hook starts with the open: SetMasterParams)
                if bs >= 3 && f > book_states.iter().find(|s| s.1 == 3).map_or(0, |s| s.0) {
                    max_hook_err = max_hook_err.max(d);
                    hook_samples += 1;
                }
            }
        }
        if captured.is_none() && GetCurrentStateID(&mut e, GHOST) == 0xb {
            captured = Some(f);
        }
        if captured.is_some_and(|c| f > c + 200) {
            break;
        }
    }
    let g = rt.particles.lock().unwrap();
    let mut seen: Vec<(u32, u32, u32, bool)> = vec![];
    for &(o, a, fl, st) in &g.sub.log.anim_play {
        if [BOOK, SCOOBY, SHAGGY].contains(&o) && !seen.iter().any(|s| s.0 == o && s.1 == a) {
            seen.push((o, a, fl, st));
        }
    }
    eprintln!("book states {book_states:x?}\ncaptured {captured:?}\nhook samples {hook_samples} max err {max_hook_err}");
    for s in &seen {
        eprintln!("anim obj {:#x} id {:#x} flags {:#x} started {}", s.0, s.1, s.2, s.3);
    }
    let book: Vec<_> = seen.iter().filter(|s| s.0 == BOOK).collect();
    assert!(!book.is_empty(), "the book never requested an anim");
    let missing: Vec<_> = seen.iter().filter(|s| !s.3).collect();
    assert!(missing.is_empty(), "requested anims that did not start: {missing:x?}");
    assert!(hook_samples > 0, "book never held");
    assert!(max_hook_err < 0.05, "book hook bones 9/10 midpoint off the heroes' midpoint by {max_hook_err}");
    assert!(book_states.iter().any(|s| s.1 == 5), "book never closed (state 5)");
}

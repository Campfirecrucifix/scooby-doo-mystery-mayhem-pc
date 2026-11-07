//! WP-J22: EP1L01 book failure paths. Same set-up as engine_j4 (book handed over, power 180, ghost
//! 3 m in front of Scooby, SQUARE at f120); then J22_MASH=none (no press) or wrong (always a wrong
//! button). Expected from the C (TomeDoomFUNC_DrainMonsterLife / TomeDoomST_OPEN sub 0x53): the
//! monster life recovers, the link breaks (msg 0xd6/100, sound 0x13), book -> state 5 -> 1, ghost not captured.
use scooby_recomp::engine::core::apigameobj::GetCurrentStateID;
use scooby_recomp::engine::input::{pad, PadState};
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::runtime::*;

use crate::common::ISO;
const BOOK: u32 = 0x2a00_0093;
const GHOST: u32 = 0x2a00_0008;
const SCOOBY: u32 = 0x2a00_0092;

fn run(mode: &str) -> (Vec<(u32, u32)>, Vec<(u32, u32)>, f32) {
    let fs = IsoFs::open(ISO).expect("ISO missing: tests must load real data");
    let (mut e, mut rt) = EngineRuntime::new_with_engine();
    let res = EngineRuntime::ReadLevel(&fs, "EP1/L01", "EP1L01.P2R").unwrap();
    rt.LoadLevel(&mut e, res);
    let (mut bs_v, mut gs_v) = (vec![], vec![]);
    let mut last = (0u32, 0u32);
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
        let l = e.rt.map.find(BOOK).map_or(0, |h| e.rt.objects[h].as_ref().unwrap().inst.local);
        {
            let mut g = rt.particles.lock().unwrap();
            let mut p = PadState { connected: true, mode: 7, rx: 128, ry: 128, lx: 128, ly: 128, ..Default::default() };
            if (120..130).contains(&f) {
                p.buttons = pad::SQUARE;
            }
            if l != 0 && f > 145 && f % 2 == 0 && mode == "wrong" {
                p.buttons = if e.mem.r32(l + 0x1e8) == 2 { pad::CIRCLE } else { pad::CROSS };
            }
            g.devices.SetPadState(0, p);
        }
        rt.Frame(&mut e, 1.0 / 60.0);
        let bs = GetCurrentStateID(&mut e, BOOK);
        let gs = GetCurrentStateID(&mut e, GHOST);
        let l = e.rt.map.find(BOOK).map_or(0, |h| e.rt.objects[h].as_ref().unwrap().inst.local);
        if l == 0 {
            continue;
        }
        let sub = e.mem.r32(l + 0x58);
        if bs_v.last().map(|x: &(u32, u32)| x.1) != Some(bs) {
            bs_v.push((f, bs));
        }
        if gs_v.last().map(|x: &(u32, u32)| x.1) != Some(gs) {
            gs_v.push((f, gs));
        }
        if (bs, sub) != last || f % 60 == 0 && bs == 3 {
            last = (bs, sub);
            let gb1 = e.rt.game_buffers[&1].0;
            eprintln!("J22 {mode} f{f} book {bs} sub {sub:#x} btn {:#x} life {} power {} ghost {gs:#x}", e.mem.r32(l + 0x1e8), e.mem.rf(l + 0x70), e.mem.rf(gb1 + 4));
        }
    }
    let gb1 = e.rt.game_buffers[&1].0;
    (bs_v, gs_v, e.mem.rf(gb1 + 4))
}

#[test]
fn ep1l01_book_fail_paths() {
    for mode in ["none", "wrong"] {
        let (b, g, power) = run(mode);
        eprintln!("{mode}: book {b:x?}\nghost {g:x?}\npower {power}");
        // the book opened, latched, then closed again without capturing the ghost
        assert!(b.iter().any(|x| x.1 == 3), "{mode}: book never open");
        assert!(b.iter().any(|x| x.1 == 5), "{mode}: book never closed");
        assert!(!g.iter().any(|x| x.1 == 0xb), "{mode}: ghost captured");
    }
}

/// J22 stuck probe (no-op without J22_LEVEL): replays the v1_all_levels input script and, for frames
/// J22_FROM..J22_TO every 50 frames, prints the leader's state, cut-scene/book flags, look-at, velocity,
/// and the stage ray hit (map mask 3) 0.6 m above the feet along the look-at (1 m).
#[test]
fn j22_stuck_probe() {
    let Ok(level) = std::env::var("J22_LEVEL") else { return };
    let from: u32 = std::env::var("J22_FROM").ok().and_then(|s| s.parse().ok()).unwrap_or(0);
    let to: u32 = std::env::var("J22_TO").ok().and_then(|s| s.parse().ok()).unwrap_or(from + 600);
    let fs = IsoFs::open(ISO).expect("ISO missing");
    let name = level.replace('/', "");
    let (mut e, mut rt) = EngineRuntime::new_with_engine();
    let res = EngineRuntime::ReadLevel(&fs, &level, &format!("{name}.P2R")).unwrap();
    rt.LoadLevel(&mut e, res);
    let scr = e.mem.malloc(0x80);
    let idle: u32 = std::env::var("J22_IDLE").ok().and_then(|s| s.parse().ok()).unwrap_or(u32::MAX);
    for f in 0..to {
        {
            let mut g = rt.particles.lock().unwrap();
            let mut p = PadState { connected: true, mode: 7, rx: 128, ry: 128, lx: 128, ly: 128, ..Default::default() };
            if f >= 100 && f < idle {
                match (f - 100) / 600 % 4 {
                    0 => p.ly = 0,
                    1 => p.lx = 255,
                    2 => p.ly = 255,
                    _ => p.lx = 0,
                }
                if f % 90 < 4 {
                    p.buttons = pad::CROSS;
                } else if f % 90 >= 45 && f % 90 < 49 {
                    p.buttons = pad::SQUARE;
                }
            }
            g.devices.SetPadState(0, p);
        }
        rt.Frame(&mut e, 1.0 / 60.0);
        if f < from || f % 50 != 0 {
            continue;
        }
        let gb0 = e.rt.game_buffers[&0].0;
        let lead = e.mem.r32(gb0 + 8);
        let st = GetCurrentStateID(&mut e, lead);
        let (fr, vel, cs) = {
            let g = rt.particles.lock().unwrap();
            let ph = g.sub.physic.PhysicOf(lead);
            let p = g.sub.physic.Physic(ph).unwrap();
            (p.frame, p.velocity, g.cutscene.as_ref().map_or(0, |c| c.flags))
        };
        let mut rays = vec![];
        for (h, ang) in [(0.2f32, 0.0f32), (0.6, 0.0), (1.2, 0.0), (0.6, 0.5), (0.6, -0.5)] {
            let (c, s_) = (ang.cos(), ang.sin());
            let d = [fr.at[0] * c - fr.at[2] * s_, fr.at[0] * s_ + fr.at[2] * c];
            let w = [fr.pos[0], fr.pos[1] + h, fr.pos[2], d[0] * 3.0, 0.0, d[1] * 3.0].map(f32::to_bits);
            for (i, v) in w.iter().chain(&[0, 0, 3, 0, 0]).enumerate() {
                e.mem.w32(scr + 4 * i as u32, *v);
            }
            let hit = scooby_recomp::engine::runtime::J2(&rt.particles).ray_intersect(&mut e.mem, scr, scr + 0x30);
            let hp = [e.mem.rf(scr + 0x30), e.mem.rf(scr + 0x38)];
            rays.push(if hit == 0 { -1.0 } else { ((hp[0] - fr.pos[0]).powi(2) + (hp[1] - fr.pos[2]).powi(2)).sqrt() });
        }
        // fan of 36 rays (3 m) from the sphere centre (pos + 0.2 at, +0.6): nearest wall distance + bearing
        let c = [fr.pos[0] + fr.at[0] * 0.2, fr.pos[1] + 0.25, fr.pos[2] + fr.at[2] * 0.2];
        let mm = {
            let g = rt.particles.lock().unwrap();
            let ph = g.sub.physic.PhysicOf(lead);
            g.sub.physic.Physic(ph).unwrap().model.masks
        };
        let mut near = (f32::MAX, 0);
        for k in 0..36 {
            let a = k as f32 * 10f32.to_radians();
            let w = [c[0], c[1], c[2], a.cos() * 3.0, 0.0, a.sin() * 3.0].map(f32::to_bits);
            for (i, v) in w.iter().chain(&[0, 0, 1, 0, 0]).enumerate() {
                e.mem.w32(scr + 4 * i as u32, *v);
            }
            if scooby_recomp::engine::runtime::J2(&rt.particles).ray_intersect(&mut e.mem, scr, scr + 0x30) != 0 {
                let d = ((e.mem.rf(scr + 0x30) - c[0]).powi(2) + (e.mem.rf(scr + 0x38) - c[2]).powi(2)).sqrt();
                if d < near.0 {
                    near = (d, k * 10);
                }
            }
        }
        rays.push(near.0);
        rays.push(near.1 as f32);
        if f == to - 1 - (to - 1) % 50 {
            eprintln!("J22MASK {mm:#x?}");
        }
        if f == from - from % 50 + 50 || f == to - 1 - (to - 1) % 50 {
            let g = rt.particles.lock().unwrap();
            let ph = g.sub.physic.PhysicOf(lead);
            let p = g.sub.physic.Physic(ph).unwrap();
            eprintln!("J22PRIM {:?}", p.model.prims);
        }
        let hl = e.rt.map.find(lead).map_or(0, |h| e.rt.objects[h].as_ref().unwrap().inst.local);
        let hs: Vec<String> = [0x158u32, 0x160, 0x16c, 0x170, 0x174].iter().map(|&o| format!("{:#x}", e.mem.r32(hl + o))).collect();
        eprintln!("J22P f{f} lead {lead:#x} st {st:#x} hl {hs:?} cs {cs:#x} gb0.14 {:#x} pos {:.2?} at {:.2?} vel {:.2?} rays(0.2,0.6,1.2,+-30deg) {rays:.2?}", e.mem.r32(gb0 + 0x14), fr.pos, fr.at, vel);
    }
}

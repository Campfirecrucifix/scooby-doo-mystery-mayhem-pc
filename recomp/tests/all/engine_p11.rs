//! WP-P11: Episode 2 scripted play (legacy; R82: ad-hoc only). Ran the per-level step routes of src/bin/routes_ep2.rs through
//! real level switches (game buffers carried as the C EFGameMemoryMgr does) and reports the frame of
//! each level request. Step machinery copied from tests/engine_r7.rs (plus `c UID`: walk-free pickup =
//! leader onto the object for 30 frames). `cargo test --release --test engine_p11 -- --ignored --nocapture`
//! R82: the chain mode is retired (EP2_ROUTE uses steps like `goto` this interpreter lacks; see p17_chains);
//! P11_CARRY=<dump> carries buffers into the ad-hoc run;
//! P11_STEPS="<level>|<steps>" runs one ad-hoc script.
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::runtime::*;
use scooby_recomp::engine::APIgameobj;

use crate::common::{ISO, leader, pos, warp, send, cutscene_update, frame, load_carried, norm, hex, uid, load_dump};
type Eng = scooby_recomp::engine::Engine;

/// Step script, ';'-separated:
/// `f N` frames; `x N` frames pressing X (3 of every 20), re-warping to the last warp target each 60;
/// `p BTN N` frames holding pad bits BTN (hex); `m UID MSG P2 [W..]` SendMessageTo (data words);
/// `warp UID` leader onto the object; `l UID OFF VAL` write local+OFF of UID; `g ID OFF VAL` write
/// game buffer ID +OFF; `dbg UID` print local words; `dm ADDR LEN` dump memory. Runs until a level request (stops the script).
fn run(e: &mut Eng, rt: &mut EngineRuntime, steps: &str) -> Option<(String, u32)> {
    let (e, rt) = (e, rt);
    let mut f = 0u32;
    let mut at: Option<[f32; 3]> = None;
    let mut got = None;
    'outer: for st in steps.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        let a: Vec<&str> = st.split_whitespace().collect();
        let (btn, n) = match a[0] {
            "f" => (0u16, a[1].parse::<u32>().unwrap()),
            "x" => (scooby_recomp::engine::input::pad::CROSS, a[1].parse().unwrap()),
            "p" => (hex(a[1]) as u16, a[2].parse().unwrap()),
            "m" => {
                let d: Vec<u32> = a[4..].iter().map(|w| if w.starts_with('@') { uid(e, w) } else { hex(w) }).collect();
                let u = uid(e, a[1]);
                send(e, rt, u, hex(a[2]), hex(a[3]), &d);
                (0, 0)
            }
            "warp" => {
                let u = uid(e, a[1]);
                at = pos(e, rt, u);
                println!("P11  warp {u:#x} -> {at:?} leader {:#x}", leader(e));
                if let Some(p) = at { warp(e, rt, p) }
                (0, 0)
            }
            "l" => {
                let u = uid(e, a[1]);
                let lo = e.rt.objects.iter().flatten().find(|o| o.id == u).map_or(0, |o| o.inst.local);
                e.mem.w32(lo + hex(a[2]), hex(a[3]));
                (0, 0)
            }
            "g" => {
                let b = e.rt.game_buffers.get(&hex(a[1])).map(|b| b.0).expect("game buffer");
                e.mem.w32(b + hex(a[2]), hex(a[3]));
                (0, 0)
            }
            "ls" => {
                for o in e.rt.objects.iter().flatten() {
                    if let Some(k) = o.inst.code {
                        let n = &e.rt.codes[k].def.name;
                        if n.to_lowercase().contains(a[1]) {
                            println!("P11  obj {:#x} {n} build {} local {:#x}", o.id, e.rt.codes[k].build, o.inst.local);
                        }
                    }
                }
                (0, 0)
            }
            "gd" => {
                let b = e.rt.game_buffers.get(&hex(a[1])).map(|b| b.0).expect("game buffer");
                let w: Vec<String> = (0..hex(a[2]) / 4).map(|i| format!("{:x}", e.mem.r32(b + 4 * i))).collect();
                println!("P11  f{f} gb{} {}", a[1], w.join(" "));
                (0, 0)
            }
            "play" => {
                // EFCutScenePlayer::Play(id) as APIgame::PlayCutScene (no sound): the scripted cut-scene
                rt.particles.lock().unwrap().cutscene_req.push(CutSceneReq::Play(hex(a[1]), 0xffff_ffff, 0x7fff_ffff));
                (0, 0)
            }
            "cs" => {
                let g = rt.particles.lock().unwrap();
                println!("P11  f{f} cutscene {:?}", g.cutscene.as_ref().map(|c| (c.current, c.flags, c.time, c.duration, c.scene(c.current).map(|s| s.actors.clone()))));
                (0, 0)
            }
            "wxyz" => {
                // warp the leader to a world position (re-applied by `x` like `warp`)
                at = Some([a[1].parse().unwrap(), a[2].parse().unwrap(), a[3].parse().unwrap()]);
                warp(e, rt, at.unwrap());
                (0, 0)
            }
            "put" => {
                // move object A onto object B (hot spot onto a vehicle whose spline overrides warps)
                let (a1, b1) = (uid(e, a[1]), uid(e, a[2]));
                if let Some(to) = pos(e, rt, b1) {
                    let p = e.mem.alloca(0x10);
                    for (i, c) in to.iter().enumerate() {
                        e.mem.w32(p + 4 * i as u32, c.to_bits());
                    }
                    rt.particles.lock().unwrap().logic = true;
                    APIgameobj::SetTranslation(e, a1, p);
                    rt.particles.lock().unwrap().logic = false;
                }
                (0, 0)
            }
            "phys" => {
                let u = uid(e, a[1]);
                let g = rt.particles.lock().unwrap();
                let id = g.sub.physic.PhysicOf(u);
                match g.sub.physic.Physic(id) {
                    Some(p) => println!("P11  f{f} phys {u:#x} id {id} flags {:#x} state {:#x} notify {:#x} masks {:x?} prims {:?}", p.flags, p.state, p.notify_bits, p.model.masks, p.model.prims.iter().map(|q| q.as_ref().map(|q| format!("{:?}", q.base))).collect::<Vec<_>>()),
                    None => println!("P11  f{f} phys {u:#x}: no physic"),
                }
                (0, 0)
            }
            "dbg" => {
                let u = uid(e, a[1]);
                let lo = e.rt.objects.iter().flatten().find(|o| o.id == u).map_or(0, |o| o.inst.local);
                let w: Vec<String> = (0..hex(a.get(2).unwrap_or(&"20")) / 4).map(|i| format!("{:x}", e.mem.r32(lo + 4 * i))).collect();
                println!("P11  f{f} {u:#x} local {}", w.join(" "));
                (0, 0)
            }
            "dm" => {
                // dump memory: `dm ADDR LEN` (hex), e.g. a hero's pLocalDataHero (local word 0)
                let w: Vec<String> = (0..hex(a[2]) / 4).map(|i| format!("{:x}", e.mem.r32(hex(a[1]) + 4 * i))).collect();
                println!("P11  f{f} mem {} {}", a[1], w.join(" "));
                (0, 0)
            }
            "pos" => {
                let u = uid(e, a[1]);
                let b = e.mem.alloca(0x10);
                rt.particles.lock().unwrap().logic = true;
                let r = APIgameobj::GetPosition(e, u, 0, b);
                rt.particles.lock().unwrap().logic = false;
                let wf = rt.particles.lock().unwrap().world.frames.get(&u).map(|m| m.pos);
                println!("P11  f{f} pos {u:#x} api {r} {:?} frame {wf:?}", e.mem.rv3(b));
                (0, 0)
            }
            "nt" => {
                let u = uid(e, a[2]);
                println!("P11  f{f} notify {} {u:#x}: {:?}", a[1], e.rt.notify.maps[hex(a[1]) as usize].get(u).map(|l| l.iter().map(|n| (n.owner, n.data.cb)).collect::<Vec<_>>()));
                (0, 0)
            }
            "pl" => {
                let l = leader(e);
                println!("P11  f{f} leader {l:#x} at {:?}", pos(e, rt, l));
                (0, 0)
            }
            "c" => {
                // pickup: leader onto the object, 30 frames (no re-warp)
                let u = uid(e, a[1]);
                if let Some(p) = pos(e, rt, u) { warp(e, rt, p) }
                at = None;
                (0, 30)
            }
            s => panic!("bad step {s}"),
        };
        for i in 0..n {
            let b = if a[0] == "x" && i % 20 >= 3 { 0 } else { btn };
            if a[0] == "x" && i % 60 == 59 {
                // only when pushed away (> 3 m, as engine_r3): an unconditional re-warp puts the
                // leader in the air right before the next X press (X is ignored while falling)
                let l = leader(e);
                if let Some(p) = at.filter(|p| pos(e, rt, l).is_some_and(|q| (0..3).map(|i| (q[i] - p[i]).powi(2)).sum::<f32>() > 9.0)) { warp(e, rt, p) }
            }
            f += 1;
            if let Some(r) = frame(e, rt, b) {
                got = Some((norm(&r.level), f));
                break 'outer;
            }
        }
    }
    got
}

#[test]
#[ignore]
fn p11_ep2_chain() {
    let fs = IsoFs::open(ISO).expect("ISO missing");
    if let Ok(s) = std::env::var("P11_STEPS") {
        let (dir, steps) = s.split_once('|').expect("P11_STEPS=<level>|<steps>");
        let bufs = std::env::var("P11_CARRY").map(|p| load_dump(&p)).unwrap_or_default();
        let (mut e, mut rt) = load_carried(&fs, dir, &bufs).unwrap();
        println!("P11 adhoc {dir}: {:?}", run(&mut e, &mut rt, steps));
        return;
    }
    // R82 (P32): the EP2 chain mode is retired. EP2_ROUTE now uses steps this legacy interpreter does not
    // know (`goto`, ...), so the chain hit `bad step goto` at EP2/L01A, was caught and passed vacuously.
    // The EP2 chain is covered by engine_p17::p17_chains (EP2_ROUTE, full step set) and
    // p21_real_carry_episodes; this test keeps only the P11_STEPS ad-hoc mode (old step set, see `run`).
    println!("P11: chain mode retired (R82): EP2 is covered by p17_chains / p21_real_carry_episodes; set P11_STEPS for ad-hoc runs");
}

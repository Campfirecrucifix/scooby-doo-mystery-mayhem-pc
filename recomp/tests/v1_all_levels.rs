//! WP-V1: run every level pack in the real runtime with a simple input script.
//! One level per process (a panic can poison shared mutexes / a level can hang):
//!   V1_LEVEL=EP1/L01 V1_FRAMES=1800 cargo test --release --test v1_all_levels -- --nocapture
//! prints one `V1RESULT|level|load|frames|nan|fall|stuck|panic|todo|backtrace` line (WP-V2: fall =
//! hero Y more than FALL_DY under its first position; stuck = 600-frame windows under stick input (direction cycles
//! forward/right/back/left per window)
//! where the hero moved less than STUCK_DIST in XZ; WP-Q3: windows mostly spent paused are not counted, and the
//! runner backs out of pause / menus / save prompts (leader SetLogicFlags 0x78) and of hero-locking interaction
//! modes (STILL_BACK frames without movement) with TRIANGLE pulses, START after 300 frames). Without V1_LEVEL the test
//! is a no-op (the full sweep is tools/engineport/v1_sweep.sh -> recomp/VERIFY.md).
use scooby_recomp::engine::input::PadState;
use scooby_recomp::engine::input::pad;
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::runtime::*;
use scooby_recomp::engine::Engine;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex};

const ISO: &str = "../Scooby Doo Mystery Mayhem.ISO";
const FALL_DY: f32 = 50.0;
const STUCK_WIN: u32 = 600;
const STUCK_DIST: f32 = 0.5;
/// WP-Q3: frames without any leader movement before the runner starts backing out (TRIANGLE / START)
const STILL_BACK: u32 = 240;

fn hero_pos(e: &mut Engine, rt: &EngineRuntime) -> Option<[f32; 3]> {
    let gb0 = e.rt.game_buffers.get(&0)?.0;
    let leader = e.mem.r32(gb0 + 8);
    if leader == 0 {
        return None;
    }
    let g = rt.particles.lock().ok()?;
    let ph = g.sub.physic.PhysicOf(leader);
    g.sub.physic.Physic(ph).map(|p| p.frame.pos)
}

/// WP-Q3: the leader is frozen by EFWorld::Pause (SetLogicFlags 0x78: a menu / save prompt / pause panel)
fn leader_paused(e: &Engine) -> bool {
    let Some(gb0) = e.rt.game_buffers.get(&0).map(|b| b.0) else { return false };
    let leader = e.mem.r32(gb0 + 8);
    e.rt.objects.iter().flatten().any(|o| o.id == leader && o.logic_flags & 0x78 != 0)
}

#[test]
fn v1_level() {
    let Ok(level) = std::env::var("V1_LEVEL") else { return };
    let frames: u32 = std::env::var("V1_FRAMES").ok().and_then(|s| s.parse().ok()).unwrap_or(1800);
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let info: Arc<Mutex<(String, String)>> = Arc::default();
    let i2 = info.clone();
    std::panic::set_hook(Box::new(move |p| {
        let msg = p.payload().downcast_ref::<&str>().map(|s| s.to_string())
            .or_else(|| p.payload().downcast_ref::<String>().cloned()).unwrap_or_default();
        let loc = p.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
        let bt = std::backtrace::Backtrace::force_capture().to_string();
        if std::env::var("V1_BT").is_ok() {
            eprintln!("{bt}");
        }
        let top: Vec<&str> = bt.lines().map(str::trim)
            .filter(|l| l.contains("scooby_recomp") && !l.contains("v1_all_levels") && !l.contains("code_fn") && !l.contains("panic"))
            .take(8).collect();
        *i2.lock().unwrap() = (format!("{} @ {}", msg.replace(['|', '\n'], " "), loc), top.join(" <- "));
    }));
    let name = if level.starts_with("GAMEART/") { level[8..].to_string() } else { level.replace('/', "") };
    let mut load = "ok".to_string();
    let mut reached = 0u32;
    let mut nan = String::new();
    let mut fall = String::new();
    // V1_TRACE=a-b: print the hero position for frames a..b
    let trace: Option<(u32, u32)> = std::env::var("V1_TRACE").ok()
        .and_then(|t| t.split_once('-').and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?))));
    let (mut spawn, mut anchor): (Option<[f32; 3]>, Option<[f32; 3]>) = (None, None);
    let (mut stuck_n, mut stuck_first) = (0u32, 0u32);
    // WP-Q3: frames spent paused (consecutive / total / in the current stuck window)
    // V1_BACK=hex,hex,..: button masks cycled while paused (default TRIANGLE, START after 300 f)
    let back: Option<Vec<u16>> = std::env::var("V1_BACK").ok()
        .map(|v| v.split(',').filter_map(|x| u16::from_str_radix(x.trim_start_matches("0x"), 16).ok()).collect());
    // WP-Q3: consecutive frames the leader did not move (puzzle / interaction modes that lock the hero
    // without pausing the world, e.g. EP3L02 hs_piano -> pianopuzzle, left with TRIANGLE)
    let (mut still_run, mut last_pos): (u32, Option<[f32; 3]>) = (0, None);
    let mut still_backouts = 0u32;
    let mut last_req: Option<String> = None;
    let (mut paused_run, mut paused_total, mut paused_win, mut paused_max) = (0u32, 0u32, 0u32, 0u32);
    let r = catch_unwind(AssertUnwindSafe(|| {
        let (mut e, mut rt) = EngineRuntime::new_with_engine();
        let res = match EngineRuntime::ReadLevel(&fs, &level, &format!("{name}.P2R")) {
            Ok(r) => r,
            Err(err) => {
                load = format!("ReadLevel: {err}");
                return;
            }
        };
        load = "load-panic".into();
        rt.LoadLevel(&mut e, res);
        load = "ok".into();
        // V1_MODS=1: list object id -> logic module name
        if std::env::var("V1_MODS").is_ok() {
            for o in e.rt.objects.iter().flatten() {
                let m = o.inst.code.map_or("-", |c| e.rt.codes[c].def.name);
                eprintln!("V1MOD {:#x} {m}", o.id);
            }
        }
        for f in 0..frames {
            {
                let mut g = rt.particles.lock().unwrap();
                let mut p = PadState { connected: true, mode: 7, rx: 128, ry: 128, lx: 128, ly: 128, ..Default::default() };
                if f >= 100 {
                    // stick forward, then right / back / left, one direction per stuck window, so a
                    // wall in front of the spawn doesn't count as stuck
                    match (f - 100) / STUCK_WIN % 4 {
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
                // WP-Q3: back out of menus / save prompts / pause: TRIANGLE pulses, and after 300
                // paused frames alternate with START pulses (pause panels that only close on START)
                let back_run = if paused_run > 0 { paused_run } else { still_run.saturating_sub(STILL_BACK) };
                if back_run > 0 {
                    p.buttons = 0;
                    if back_run % 30 < 4 {
                        p.buttons = match &back {
                            Some(v) => v[(back_run / 30) as usize % v.len()],
                            None if back_run >= 300 && back_run / 30 % 2 == 1 => pad::START,
                            None => pad::TRIANGLE,
                        };
                    }
                }
                g.devices.SetPadState(0, p);
            }
            rt.Frame(&mut e, 1.0 / 60.0);
            reached = f + 1;
            if leader_paused(&e) {
                paused_run += 1;
                paused_total += 1;
                paused_win += 1;
                paused_max = paused_max.max(paused_run);
            } else {
                paused_run = 0;
            }
            // V1_STATES=id,id,..: logic state / lflags of those objects every 300 frames and at pause start
            if let (Ok(ids), true) = (std::env::var("V1_STATES"), paused_run % 300 == 1 || paused_run == 0 && f % 300 == 0) {
                for id in ids.split(',').filter_map(|x| u32::from_str_radix(x.trim_start_matches("0x"), 16).ok()) {
                    if let Some(o) = e.rt.objects.iter().flatten().find(|o| o.id == id) {
                        eprintln!("V1STATE {f} {id:#x} state={:#x} lflags={:#x} iflags={:#x}", o.inst.state, o.logic_flags, o.inst.flags);
                    }
                }
            }
            // level requests are not followed (the runner stays in the level); print each new one
            if let Some(q) = rt.particles.lock().unwrap().game.take_level_request() {
                let k = format!("level={:?} pack={:?}", q.level, q.pack);
                if last_req.as_deref() != Some(&k) {
                    eprintln!("V1LEVELREQ {f} {k}");
                    last_req = Some(k);
                }
            }
            if nan.is_empty() {
                if let Some(p) = hero_pos(&mut e, &rt) {
                    if p.iter().any(|v| !v.is_finite()) {
                        nan = format!("frame {f}");
                    }
                }
            }
            if let Some(p) = hero_pos(&mut e, &rt).filter(|p| p.iter().all(|v| v.is_finite())) {
                if f >= 100 && last_pos.is_some_and(|l| (0..3).all(|i| (l[i] - p[i]).abs() < 1e-3)) {
                    still_run += 1;
                    if still_run == STILL_BACK + 1 {
                        still_backouts += 1;
                    }
                } else {
                    still_run = 0;
                }
                last_pos = Some(p);
                // V1_WALLS=frame,radius: environment collision prims with |normal.y| < 0.7 near the leader
                if let Some((nf, r)) = std::env::var("V1_WALLS").ok().and_then(|v| v.split_once(',').and_then(|(a, b)| Some((a.parse::<u32>().ok()?, b.parse::<f32>().ok()?)))) {
                    if nf == f {
                        let g = rt.particles.lock().unwrap();
                        for (mi, m) in g.sub.physic.maps.iter().enumerate() {
                            for pr in m.Primitives() {
                                let n = m.PrimNormal(pr);
                                if n[1].abs() >= 0.7 { continue; }
                                let vs: Vec<[f32; 3]> = (0..m.PrimCount(pr)).map(|i| m.PrimVertex(pr, i)).collect();
                                let near = vs.iter().any(|v| ((v[0] - p[0]).powi(2) + (v[2] - p[2]).powi(2)).sqrt() < r && v[1] > p[1] - 0.5 && v[1] < p[1] + 3.0)
                                    || (vs.iter().map(|v| v[1]).fold(f32::MAX, f32::min) < p[1] + 1.0 && vs.iter().map(|v| v[1]).fold(f32::MIN, f32::max) > p[1] && {
                                        // plane distance for big quads whose vertices are far away
                                        let d = n[0] * p[0] + n[1] * p[1] + n[2] * p[2] + m.PrimD(pr);
                                        d.abs() < r && vs.iter().map(|v| v[0]).fold(f32::MAX, f32::min) < p[0] + r && vs.iter().map(|v| v[0]).fold(f32::MIN, f32::max) > p[0] - r
                                            && vs.iter().map(|v| v[2]).fold(f32::MAX, f32::min) < p[2] + r && vs.iter().map(|v| v[2]).fold(f32::MIN, f32::max) > p[2] - r
                                    });
                                if near {
                                    let vv: Vec<String> = vs.iter().map(|v| format!("({:.2},{:.2},{:.2})", v[0], v[1], v[2])).collect();
                                    eprintln!("V1WALL map{mi} uid={:#x} n=({:.2},{:.2},{:.2}) {}", m.uid, n[0], n[1], n[2], vv.join(" "));
                                }
                            }
                        }
                    }
                }
                // V1_NEAR=frame,radius: objects (module, state, position) within radius of the leader at that frame
                if let Some((nf, r)) = std::env::var("V1_NEAR").ok().and_then(|v| v.split_once(',').and_then(|(a, b)| Some((a.parse::<u32>().ok()?, b.parse::<f32>().ok()?)))) {
                    if nf == f {
                        let buf = e.mem.malloc(16);
                        let objs: Vec<(u32, &str, u32)> = e.rt.objects.iter().flatten()
                            .map(|o| (o.id, o.inst.code.map_or("-", |c| e.rt.codes[c].def.name), o.inst.state)).collect();
                        for (id, m, st) in objs {
                            e.host.APIgameobj_GetPosition(&mut e.mem, id, 0x7fff_ffff, buf);
                            let q = [e.mem.rf(buf), e.mem.rf(buf + 4), e.mem.rf(buf + 8)];
                            let d = ((q[0] - p[0]).powi(2) + (q[2] - p[2]).powi(2)).sqrt();
                            if d < r {
                                eprintln!("V1NEAR {id:#x} {m} st={st:#x} d={d:.2} {:.2} {:.2} {:.2}", q[0], q[1], q[2]);
                            }
                        }
                        e.mem.free(buf);
                    }
                }
                let s0 = *spawn.get_or_insert(p);
                if trace.is_some_and(|(a, b)| f >= a && f < b) {
                    let gb0 = e.rt.game_buffers.get(&0).unwrap().0;
                    let leader = e.mem.r32(gb0 + 8);
                    let buf = e.mem.malloc(16);
                    e.host.APIgameobj_GetPosition(&mut e.mem, leader, 0x7fff_ffff, buf);
                    let q = [e.mem.rf(buf), e.mem.rf(buf + 4), e.mem.rf(buf + 8)];
                    e.mem.free(buf);
                    // V1_LOCAL=i,j,..: leader logic local-data words (u32 index) as hex
                    if let Ok(l) = std::env::var("V1_LOCAL") {
                        if let Some(o) = e.rt.objects.iter().flatten().find(|o| o.id == leader) {
                            let w: Vec<String> = l.split(',').filter_map(|i| i.parse::<u32>().ok())
                                .map(|i| format!("[{i:#x}]={:#x}", e.mem.r32(o.inst.local + i * 4))).collect();
                            eprintln!("V1LOCAL {f} {}", w.join(" "));
                        }
                    }
                    eprintln!("V1TRACE {f} leader={leader:#x} physic {:.3} {:.3} {:.3} obj {:.3} {:.3} {:.3}", p[0], p[1], p[2], q[0], q[1], q[2]);
                }
                if fall.is_empty() && p[1] < s0[1] - FALL_DY {
                    fall = format!("frame {f} y={:.1} (spawn {:.1})", p[1], s0[1]);
                }
                if f >= 100 && (f - 100) % STUCK_WIN == 0 {
                    if let Some(a) = anchor {
                        // windows mostly spent paused don't measure movement
                        if paused_win < STUCK_WIN / 2 && ((p[0] - a[0]).powi(2) + (p[2] - a[2]).powi(2)).sqrt() < STUCK_DIST {
                            stuck_n += 1;
                            if stuck_first == 0 {
                                stuck_first = f;
                            }
                        }
                    }
                    anchor = Some(p);
                    paused_win = 0;
                }
            }
        }
    }));
    let (pmsg, bt) = if r.is_err() { info.lock().unwrap().clone() } else { Default::default() };
    let todo = pmsg.contains("not yet implemented") || pmsg.contains("not implemented");
    let mut stuck = if stuck_n > 0 { format!("{stuck_n}x from frame {stuck_first}") } else { String::new() };
    if still_backouts > 0 {
        stuck += &format!(" (still-backouts {still_backouts})");
    }
    if paused_total > 0 {
        stuck += &format!(" (paused {paused_total}f, max run {paused_max})");
    }
    println!("V1RESULT|{level}|{load}|{reached}|{nan}|{fall}|{stuck}|{pmsg}|{}|{bt}", if todo { "yes" } else { "" });
}

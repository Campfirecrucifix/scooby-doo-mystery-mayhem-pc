//! WP-T1: headless book-mechanics regression suite (see BOOK_MECHANICS.md "Regression suite").
//! One test per verified capture of the K3..K6 tables plus the J22 fail paths and the orb refill.
//! Driver = the `game --autoplay --give-book` K3/K5 harness without rendering: book msg 0xe5 + power
//! GB1+4 = 180 at f10, each target moved 3 m in front of the leader (APIgameobj::SetTranslation) once
//! the book is closed, SQUARE until the book opens, then the requested button (book L+0x1e8) mashed.
use scooby_recomp::engine::core::apigameobj::GetCurrentStateID;
use scooby_recomp::engine::input::{pad, PadState};
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::runtime::*;
use scooby_recomp::engine::APIgameobj;

use crate::common::ISO;

#[derive(Clone, Copy, PartialEq)]
enum Mash {
    Right,
    None,
    Wrong,
}

struct Case {
    level: &'static str,
    targets: &'static [u32],
    /// (x, y, z, frame): warp the leader (+ partner 1 m beside it)
    warp: Option<([f32; 3], u32)>,
    /// (uid, msg, param, frame)
    msgs: &'static [(u32, u32, u32, u32)],
    /// move launched wisps onto the leader every 20 frames (GAME_WISPGRAB)
    wispgrab: bool,
    mash: Mash,
    frames: u32,
    /// minimum power before (re)placing a target (GAME_K5POWER, default 150)
    min_power: f32,
    /// power GB1+4 set at f10 (180 = full, the Wisp_baseState_Picked_Execute cap)
    power0: f32,
    /// wispgrab also moves the level's own wisp GLOs (not only launched ones)
    all_wisps: bool,
    /// objects moved in front of the leader, in order (empty = `targets`)
    place: &'static [u32],
    /// captured = the book went FINALSUCKING -> ISCLOSING hooked to the target (bosses whose
    /// handler eats 0xd6/0x68 instead of entering Monster_BaseST_CAPTURED); no removal check
    by_book: bool,
    /// check that every target is still CAPTURED (asleep) / killed at the end
    stay: bool,
    /// wispgrab moves the leader onto the wisp (x/z) instead of the wisp onto the leader: an Active
    /// level wisp re-places itself at its home L+0x68 every frame (Wisp_baseFUNC_RunBob)
    to_wisp: bool,
    /// K8 (GAME_K5FLOOR): every 60 frames put the leader back here when it is >1.5 m above it
    floor: Option<[f32; 3]>,
    /// K8 (GAME_K5REFILL): power GB1+4 = 180 at each placement (debug cheat, not game behaviour)
    refill: bool,
}

const BASE: Case = Case { level: "", targets: &[], warp: None, msgs: &[], wispgrab: false, mash: Mash::Right, frames: 1500, min_power: 150.0, power0: 180.0, all_wisps: false, place: &[], by_book: false, stay: true, to_wisp: false, floor: None, refill: false };

#[derive(Debug, Default)]
struct Run {
    /// (frame, book state) on every change
    states: Vec<(u32, u32)>,
    /// per target: frame it reached Monster_BaseST_CAPTURED 0xb or was removed
    /// (removed = killed or still CAPTURED (asleep, go.Sleep(-1)) at the end of the run)
    captured: Vec<Option<u32>>,
    removed: Vec<bool>,
    /// (frame, power GB1+4) on every change
    power: Vec<(u32, f32)>,
    min_life: f32,
    /// book target uids (L+0x60) in hook order
    hooked: Vec<u32>,
    /// WP-R6: first EFEngineController level request (frame, level)
    level: Option<(u32, String)>,
}

fn sendmsg(e: &mut scooby_recomp::engine::Engine, rt: &EngineRuntime, u: u32, m: u32, a: u32) {
    rt.particles.lock().unwrap().logic = true;
    e.SendLogicMessageTo(u, m, a, 0, 0);
    rt.particles.lock().unwrap().logic = false;
}

fn translate(e: &mut scooby_recomp::engine::Engine, id: u32, p: [f32; 3]) {
    let a = e.mem.alloca(0x10);
    for (i, c) in p.iter().enumerate() {
        e.mem.w32(a + 4 * i as u32, c.to_bits());
    }
    APIgameobj::SetTranslation(e, id, a);
}

fn run(c: &Case) -> Run {
    let env = std::env::var("R6_TAIL").ok();
    run_tail(c, env.is_some(), env.and_then(|v| v.parse().ok()).unwrap_or(600))
}

/// WP-T1: R6 tail mode passed explicitly (r6_boss_level_end used to `set_var("R6_TAIL")`, which leaked into
/// every later/parallel run() of the same process, e.g. ep6l05b_ghost_boss). `r6` = stop at the level request.
fn run_tail(c: &Case, r6: bool, tail: u32) -> Run {
    let fs = IsoFs::open(ISO).unwrap_or_else(|_| panic!("ISO missing at {ISO}: the book regression suite needs the real disc"));
    let (mut e, mut rt) = EngineRuntime::new_with_engine();
    let res = EngineRuntime::ReadLevel(&fs, c.level, &format!("{}.P2R", c.level.replace('/', ""))).unwrap();
    let cs = scooby_recomp::engine::script::EFCutScenePlayer::FromLevelObjects(&res.objects).ok();
    rt.LoadLevel(&mut e, res);
    rt.particles.lock().unwrap().cutscene = cs;
    let book = e.rt.objects.iter().flatten().find(|o| o.inst.code.is_some_and(|k| e.rt.codes[k].def.name == "tomedoom")).map(|o| o.id).expect("no tomedoom object");
    let mut r = Run { captured: vec![None; c.targets.len()], removed: vec![false; c.targets.len()], min_life: f32::MAX, ..Default::default() };
    let (mut idx, mut placed, mut seen4): (usize, Option<u32>, bool) = (0, None, false);
    let mut done_at = None;
    let mut moved: Vec<u32> = vec![];
    for f in 0..c.frames {
        let gb1 = e.rt.game_buffers.get(&1).map_or(0, |b| b.0);
        if f == 10 && gb1 != 0 {
            sendmsg(&mut e, &rt, book, 0xe5, 0);
            e.mem.w32(gb1 + 4, c.power0.to_bits());
        }
        let lead = e.rt.game_buffers.get(&0).map_or(0, |b| e.mem.r32(b.0 + 8));
        if let Some((p, wf)) = c.warp {
            if f == wf && lead != 0 {
                translate(&mut e, lead, p);
                let o = e.mem.r32(e.rt.game_buffers[&0].0 + 0xc);
                if o != 0 && o != lead {
                    translate(&mut e, o, [p[0] + 1.0, p[1], p[2]]);
                }
            }
        }
        for &(u, m, a, mf) in c.msgs {
            if mf == f {
                sendmsg(&mut e, &rt, u, m, a);
            }
        }
        let lp = rt.particles.lock().unwrap().world.frames.get(&lead).map(|m| m.pos);
        if c.wispgrab && f % 20 == 0 && f >= 100 {
            if let Some(lp) = lp {
                let ids: Vec<u32> = e.rt.objects.iter().flatten().filter(|o| (c.all_wisps || o.id & 0x0080_0000 != 0) && o.id != book && o.inst.code.is_some_and(|k| e.rt.codes[k].def.name.to_ascii_lowercase().contains("wisp"))).map(|o| o.id).collect();
                // all_wisps: one level wisp per grab, so each pick-up shows as its own power step
                let n = if c.all_wisps { 1 } else { ids.len() };
                let far: Vec<u32> = { let g = rt.particles.lock().unwrap(); ids.into_iter().filter(|id| !moved.contains(id) && g.world.frames.get(id).is_some_and(|p| (p.pos[0] - lp[0]).hypot(p.pos[2] - lp[2]) > 1.0)).take(n).collect() };
                for id in far {
                    if c.all_wisps {
                        moved.push(id);
                    }
                    if c.to_wisp {
                        let wp = rt.particles.lock().unwrap().world.frames.get(&id).map(|m| m.pos).unwrap();
                        translate(&mut e, lead, [wp[0], lp[1], wp[2]]);
                        eprintln!("T1 {} f{f} leader -> wisp {id:#x}", c.level);
                    } else {
                        translate(&mut e, id, [lp[0], lp[1] + 0.5, lp[2]]);
                        eprintln!("T1 {} f{f} wisp {id:#x} -> leader", c.level);
                    }
                }
            }
        }
        let bs = GetCurrentStateID(&mut e, book);
        let l = e.rt.map.find(book).map_or(0, |h| e.rt.objects[h].as_ref().unwrap().inst.local);
        let power = if gb1 != 0 { e.mem.rf(gb1 + 4) } else { 0.0 };
        let cut = rt.particles.lock().unwrap().cutscene.as_ref().is_some_and(|k| k.flags & 1 != 0);
        // K5 target sequencing: next target after a FINALSUCKING -> closed cycle
        if bs == 4 {
            seen4 = true;
        }
        if seen4 && bs == 1 {
            idx += 1;
            placed = None;
            seen4 = false;
        }
        if let (Some(fl), 0) = (c.floor, f % 60) {
            if lp.is_some_and(|p| p[1] > fl[1] + 1.5) {
                translate(&mut e, lead, fl);
            }
        }
        let hooked = l != 0 && e.mem.r32(l + 0x60) != 0;
        let place = if c.place.is_empty() { c.targets } else { c.place };
        // K8: a placement target already captured (group mask 0, e.g. the book hooked a neighbour) is skipped
        if c.floor.is_some() && bs == 1 && place.get(idx).is_some_and(|&t| e.rt.map.find(t).is_some_and(|h| e.rt.obj(h).group_mask == 0)) {
            idx += 1;
            placed = None;
        }
        if let Some(&t) = place.get(idx) {
            if f >= 100 && !cut && lead != 0 && bs != 4 && !hooked && power >= c.min_power && placed.is_none_or(|p| f - p > 900) && e.rt.map.find(t).is_some() {
                let fr = { let p = rt.particles.lock().unwrap(); let ph = p.sub.physic.PhysicOf(lead); p.sub.physic.Physic(ph).map(|x| x.frame) };
                if let Some(fr) = fr {
                    translate(&mut e, t, [fr.pos[0] + fr.at[0] * 3.0, fr.pos[1] + 1.0, fr.pos[2] + fr.at[2] * 3.0]);
                    if c.refill && gb1 != 0 {
                        e.mem.w32(gb1 + 4, 180f32.to_bits());
                    }
                    eprintln!("T1 {} f{f} place {t:#x}", c.level);
                    placed = Some(f);
                }
            }
        }
        let mut p = PadState { connected: true, mode: 7, rx: 128, ry: 128, lx: 128, ly: 128, ..Default::default() };
        if f >= 120 && !cut && f % 60 < 10 && bs == 1 && idx < place.len() {
            p.buttons = pad::SQUARE;
        }
        if l != 0 && f > 145 && f % 2 == 0 && bs == 3 {
            let want = e.mem.r32(l + 0x1e8);
            let want = match c.mash { Mash::Right => want, Mash::None => 0, Mash::Wrong => if want == 2 { 4 } else { 2 } };
            p.buttons |= match want { 2 => pad::CROSS, 4 => pad::CIRCLE, 8 => pad::TRIANGLE, _ => 0 };
        }
        rt.particles.lock().unwrap().devices.SetPadState(0, p);
        rt.Frame(&mut e, 1.0 / 60.0);
        // WP-R6: after the last capture also run the cut-scene player (as bin/game.rs; the capture
        // phase keeps the recorded K3..K6 timings without it): boss end sequences are scripted
        if done_at.is_some() {
            cutscene_update(&mut e, &mut rt);
        }
        if let Some(q) = rt.particles.lock().unwrap().game.take_level_request() {
            eprintln!("T1 {} f{f} level request {:?} pack {:?}", c.level, q.level, q.pack);
            r.level.get_or_insert((f, q.level));
            if r6 {
                break;
            }
        }
        let bs = GetCurrentStateID(&mut e, book);
        let gb1 = e.rt.game_buffers.get(&1).map_or(0, |b| b.0);
        if gb1 == 0 {
            continue;
        }
        if r.states.last().map(|x| x.1) != Some(bs) {
            r.states.push((f, bs));
            eprintln!("T1 {} f{f} book {bs} power {} life {}", c.level, e.mem.rf(gb1 + 4), if l != 0 { e.mem.rf(l + 0x70) } else { 0.0 });
        }
        let pw = e.mem.rf(gb1 + 4);
        if r.power.last().map(|x| x.1) != Some(pw) {
            r.power.push((f, pw));
        }
        let ht = if l != 0 { e.mem.r32(l + 0x60) } else { 0 };
        if ht != 0 && r.hooked.last() != Some(&ht) {
            r.hooked.push(ht);
        }
        if l != 0 && hooked {
            r.min_life = r.min_life.min(e.mem.rf(l + 0x70));
        }
        for (i, &t) in c.targets.iter().enumerate() {
            let gone = e.rt.map.find(t).is_none();
            let by_book = c.by_book && r.hooked.last() == Some(&t) && r.states.ends_with(&[(f, 5)]) && r.states.iter().rev().nth(1).is_some_and(|x| x.1 == 4);
            if r.captured[i].is_none() && (gone || GetCurrentStateID(&mut e, t) == 0xb || by_book) {
                r.captured[i] = Some(f);
                eprintln!("T1 {} f{f} target {t:#x} captured", c.level);
            }
            r.removed[i] = gone || GetCurrentStateID(&mut e, t) == 0xb;
        }
        if done_at.is_none() && r.captured.iter().all(|x| x.is_some()) && !c.targets.is_empty() && c.mash == Mash::Right {
            done_at = Some(f);
        }
        // WP-R6: R6_TAIL = frames to keep running after the last capture (default 600)
        if done_at.is_some_and(|d| f > d + tail) {
            break;
        }
    }
    for &t in c.targets {
        eprintln!("T1 {} end target {t:#x} exists {} state {:#x}", c.level, e.rt.map.find(t).is_some(), GetCurrentStateID(&mut e, t));
    }
    eprintln!("T1 {} final power {:?}", c.level, r.power.last());
    eprintln!("T1 {} hooked {:x?}", c.level, r.hooked);
    eprintln!("T1 {} states {:?} captured {:?} removed {:?} min life {}", c.level, r.states, r.captured, r.removed, r.min_life);
    r
}

/// the full capture cycle: book opens (3), final suck (4), closes (5), put away (1), target captured,
/// power drained below 180 by the mashing
fn assert_capture(c: &Case, r: &Run) {
    let s: Vec<u32> = r.states.iter().map(|x| x.1).collect();
    for w in [[3, 4], [4, 5], [5, 1]] {
        assert!(s.windows(2).any(|x| x == w), "{}: book sequence {s:?} lacks {w:?}", c.level);
    }
    for (i, t) in c.targets.iter().enumerate() {
        assert!(r.captured[i].is_some(), "{}: target {t:#x} not captured; book {:?}", c.level, r.states);
        assert!(!c.stay || r.removed[i], "{}: target {t:#x} not removed / left CAPTURED", c.level);
    }
    assert!(r.min_life <= 0.0, "{}: monster life never drained ({})", c.level, r.min_life);
    let pmin = r.power.iter().map(|x| x.1).fold(f32::MAX, f32::min);
    assert!(pmin < 180.0, "{}: power never drained", c.level);
}

fn capture(c: Case) -> Run {
    let r = run(&c);
    assert_capture(&c, &r);
    r
}

/// the first book cycle (frames of states 2, 3, 4, 5, 1) as recorded on HEAD (+-TOL frames) and the
/// power after the capture
const TOL: u32 = 15;
fn recorded(c: Case, cycle: [u32; 5], power: f32) -> Run {
    let r = capture(c);
    let got: Vec<(u32, u32)> = r.states.iter().copied().skip_while(|x| x.1 != 2).take(5).collect();
    let want: Vec<(u32, u32)> = cycle.iter().zip([2, 3, 4, 5, 1]).map(|(&f, s)| (f, s)).collect();
    let ok = got.len() == 5 && got.iter().zip(&want).all(|(g, w)| g.1 == w.1 && g.0.abs_diff(w.0) <= TOL);
    assert!(ok, "book cycle {got:?}, recorded {want:?}");
    assert_eq!(r.power.last().unwrap().1, power, "power after capture {:?}", r.power);
    r
}

#[test]
fn ep1l01_small_ghost() {
    recorded(Case { level: "EP1/L01", targets: &[0x2a00_0008], ..BASE }, [120, 141, 184, 267, 289], 135.0);
}

#[test]
fn ep2l01b_poltergeist() {
    recorded(Case { level: "EP2/L01B", targets: &[0x2a00_00f1], ..BASE }, [120, 141, 204, 288, 309], 112.5);
}

#[test]
fn ep3l02_dust_devil() {
    recorded(Case { level: "EP3/L02", targets: &[0x2a00_00e4], frames: 2400, min_power: 0.0, ..BASE }, [120, 141, 224, 307, 329], 90.0);
}

#[test]
fn ep6l04b_fire_ghost_after_water() {
    // not catchable while burning (group 0x401): water msg 0xd8 at f60; the fire ghost 0x147 is placed,
    // the book hooks its inner ghost 0x14d
    let r = recorded(Case { level: "EP6/L04B", targets: &[0x2a00_014d], place: &[0x2a00_0147], msgs: &[(0x2a00_0147, 0xd8, 0, 60)], ..BASE }, [120, 141, 204, 288, 309], 112.5);
    assert_eq!(r.hooked, [0x2a00_014d]);
}

#[test]
fn ep1l03b_shadows() {
    recorded(Case { level: "EP1/L03B", targets: &[0x2a00_001c], ..BASE }, [120, 142, 204, 288, 309], 112.5);
}

#[test]
fn ep6l02_zombie() {
    recorded(Case { level: "EP6/L02", targets: &[0x2a00_01a8], ..BASE }, [120, 141, 204, 287, 309], 112.5);
}

#[test]
fn ep6l05b_ghost_boss() {
    // GhostBossLevelFive eats 0xd6/0x68 (-> 0x180 to 0x5a, 0x3a to the HUD): no CAPTURED state;
    // life 195 -> -1 while the power drains 180 -> 0 (win condition = life)
    recorded(Case { level: "EP6/L05B", targets: &[0x2a00_0012], frames: 2400, by_book: true, stay: false, ..BASE }, [120, 141, 304, 388, 409], 0.0);
}

#[test]
fn ep3l05_dust_devil_boss_with_minions() {
    // minion pairs 9e/9f, a0/a1, a2/a3 then the boss; wisps (carrousel timer latent) grabbed for power
    let t = [0x2a00_009e, 0x2a00_009f, 0x2a00_00a0, 0x2a00_00a1, 0x2a00_00a2, 0x2a00_00a3, 0x2a00_009d];
    let r = capture(Case { level: "EP3/L05", targets: &[0x2a00_009e, 0x2a00_009f, 0x2a00_00a0, 0x2a00_00a1, 0x2a00_00a2, 0x2a00_00a3, 0x2a00_009d], wispgrab: true, frames: 20000, stay: false, ..BASE });
    assert_eq!(r.hooked[..7], t, "hook order");
    assert!(r.captured.windows(2).all(|w| w[0] <= w[1]), "capture order {:?}", r.captured);
    assert!(r.power.windows(2).any(|w| w[1].1 > w[0].1 && w[0].0 > 10), "no wisp refill during the fight");
}

#[test]
fn ep1l01_fail_paths() {
    // recorded: link broken (3 -> 5) at f322 without presses, f232 with wrong presses; power untouched
    for (mash, brk) in [(Mash::None, 322), (Mash::Wrong, 232)] {
        let c = Case { level: "EP1/L01", targets: &[0x2a00_0008], mash, frames: 400, ..BASE };
        let r = run(&c);
        let s: Vec<u32> = r.states.iter().map(|x| x.1).collect();
        assert!(s.windows(2).any(|x| x == [3, 5]), "book did not break the link 3->5: {s:?}");
        assert!(!s.contains(&4), "book reached FINALSUCKING without mashing: {s:?}");
        assert!(r.captured[0].is_none() && !r.removed[0], "ghost captured without mashing");
        let f5 = r.states.iter().find(|x| x.1 == 5).unwrap().0;
        assert!(f5.abs_diff(brk) <= TOL, "link broken at f{f5}, recorded f{brk}");
        assert!(r.power.iter().all(|x| x.0 < 10 || x.1 == 180.0), "power changed: {:?}", r.power);
    }
}

#[test]
fn ep1l03a_orb_refill() {
    // Wisp_baseState_Picked_Execute: power GB1+4 += 18/36/54 per wisp size, capped at 180
    let r = run(&Case { level: "EP1/L03A", power0: 10.0, wispgrab: true, all_wisps: true, frames: 600, ..BASE });
    eprintln!("power {:?}", r.power);
    // recorded on HEAD: [(10, 10), (404, 118), (424, 180)] = the wispsourcenew 0x2a0000d6 wisps
    // (2 x 54 in one frame), then 0x2a0000d7 hits the cap. The l03a_wisps GLOs are not picked up by
    // being walked over (no power change).
    let p: Vec<f32> = r.power.iter().map(|x| x.1).skip_while(|&v| v != 10.0).collect();
    assert!(p.len() >= 3, "fewer than 2 pick-ups from power 10: {:?}", r.power);
    for w in p.windows(2) {
        let d = w[1] - w[0];
        assert!(d > 0.0 && (d % 18.0 == 0.0 || w[1] == 180.0), "power step {d} not n x 18 ({:?})", r.power);
    }
    assert_eq!(*p.last().unwrap(), 180.0, "power not refilled to the 180 cap: {:?}", r.power);
}

#[test]
fn ep1l03a_free_wisps() {
    // K10: the placed l03a_wisps GLOs (0x2a0000ac..cd) start Active (Wisp_baseState_Active_Execute:
    // visible, group mask 0x801, collide notify 3 -> Wisp_baseCB_NOTIFY_COLLIDE_OBJ): the hero touching
    // one with power < 180 picks it (+18 small / +36 medium), no button, no source needed. The T1 "no power" was the test
    // moving the wisp, which Wisp_baseFUNC_RunBob puts back at its home every frame.
    let r = run(&Case { level: "EP1/L03A", power0: 10.0, wispgrab: true, all_wisps: true, to_wisp: true, frames: 380, ..BASE });
    let steps: Vec<f32> = r.power.windows(2).map(|w| w[1].1 - w[0].1).filter(|&d| d > 0.0).collect();
    // recorded: 10 -> +18 per wisp from f103 (one +36 medium wisp at f263) -> 180 cap at f283
    assert!(steps.len() >= 8 && steps[1..steps.len() - 1].iter().all(|&d| d == 18.0 || d == 36.0), "free wisps give no power: {:?}", r.power);
    assert_eq!(r.power.last().unwrap().1, 180.0, "{:?}", r.power);
}

fn boss_uid(level: &str, code: &str) -> u32 {
    let fs = IsoFs::open(ISO).unwrap();
    let (mut e, mut rt) = EngineRuntime::new_with_engine();
    let res = EngineRuntime::ReadLevel(&fs, level, &format!("{}.P2R", level.replace('/', ""))).unwrap();
    rt.LoadLevel(&mut e, res);
    let ids: Vec<(u32, String)> = e.rt.objects.iter().flatten().filter_map(|o| Some((o.id, e.rt.codes[o.inst.code?].def.name.to_string()))).filter(|x| x.1.starts_with(code)).collect();
    eprintln!("R6 {level} {code} objects {ids:x?}");
    ids.first().expect("no boss object").0
}

/// WP-R6: boss capture -> level end. Captures the boss with the K-harness, then keeps running
/// (R6_TAIL frames, default 3000 here) until EFEngineController gets a level request.
/// `R6_BOSS=EP3/L05|EP6/L05B|EP4/L04` picks one (default all).
#[test]
#[ignore]
fn r6_boss_level_end() {
    let tail = std::env::var("R6_TAIL").ok().and_then(|v| v.parse().ok()).unwrap_or(3000);
    let only = std::env::var("R6_BOSS").ok();
    let cases = [
        Case { level: "EP3/L05", targets: &[0x2a00_009e, 0x2a00_009f, 0x2a00_00a0, 0x2a00_00a1, 0x2a00_00a2, 0x2a00_00a3, 0x2a00_009d], wispgrab: true, frames: 24000, stay: false, ..BASE },
        Case { level: "EP6/L05B", targets: &[0x2a00_0012], frames: 6000, by_book: true, stay: false, ..BASE },
        // WP-R8: K8 recipe (BOOK_MECHANICS.md K8): floor warp, power 0, refill, zombie sequence then the boss
        Case { level: "EP4/L04", targets: &[0x2a00_00bc], place: &[0x2a00_00b6, 0x2a00_00b8, 0x2a00_00ba, 0x2a00_00b6, 0x2a00_00b8, 0x2a00_00ba, 0x2a00_00b6, 0x2a00_00b8, 0x2a00_00ba, 0x2a00_00bc], warp: Some(([302.0, 0.6, 205.0], 100)), floor: Some([302.0, 0.6, 205.0]), min_power: 0.0, refill: true, wispgrab: true, frames: 12000, stay: false, ..BASE },
    ];
    for c in cases.iter().filter(|c| only.as_deref().is_none_or(|o| o == c.level)) {
        let r = run_tail(c, true, tail);
        println!("R6 {} captured {:?} level {:?}", c.level, r.captured, r.level);
    }
}

/// cut-scene player step (copy of engine_r3's: play/stop/skip requests, then Update)
fn cutscene_update(e: &mut scooby_recomp::engine::Engine, rt: &mut EngineRuntime) {
    use scooby_recomp::engine::script::RuntimeCutSceneHost;
    let (cs, reqs) = {
        let mut p = rt.particles.lock().unwrap();
        (p.cutscene.take(), std::mem::take(&mut p.cutscene_req))
    };
    let Some(mut cs) = cs else { return };
    let mut h = RuntimeCutSceneHost::new(e, rt);
    for q in reqs {
        match q {
            CutSceneReq::Play(id, snd, pos) => cs.Play(&mut h, id, snd, pos),
            CutSceneReq::Stop => cs.Stop(&mut h),
            CutSceneReq::Skip => cs.Skip(&mut h),
        }
    }
    cs.Update(&mut h, 1.0 / 60.0);
    rt.particles.lock().unwrap().cutscene = Some(cs);
}

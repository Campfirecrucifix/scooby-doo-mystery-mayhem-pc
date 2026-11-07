//! WP-R3: level -> level transitions. Probe: every level pack loads headless, the leader
//! (GB0+8) exists and logic frames run; lists the exit candidates (objects whose logic code
//! name contains door/changelevel/changetrack/exit/end) with their local level string (+0x18).
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::runtime::*;

use crate::common::{ISO, leader, frame, carry, norm};

pub const LEVELS: &[&str] = &[
    "EP1/L01", "EP1/L02", "EP1/L03A", "EP1/L03B", "EP1/L04", "EP1/L09", "EP2/L01A", "EP2/L01B", "EP2/L01C", "EP2/L02A", "EP2/L02B", "EP2/L02C",
    "EP2/L03A", "EP2/L04", "EP2/L09", "EP3/L01", "EP3/L02", "EP3/L03", "EP3/L04", "EP3/L05", "EP3/L09", "EP4/L01", "EP4/L02A", "EP4/L02B",
    "EP4/L03A", "EP4/L03B", "EP4/L04", "EP4/L09A", "EP4/L09B", "EP6/L01", "EP6/L02", "EP6/L03", "EP6/L04A", "EP6/L04B", "EP6/L05", "EP6/L05B",
    "EP6/L09", "INTRO", "LOADER", "SPLASH", "STATUS", "CREDITS",
    // the other 24 of the 66 LEVEL/**/*.P2R packs: Extras art gallery pages (LEVE_/ is a duplicate tree)
    "GAMEART/ART01", "GAMEART/ART02", "GAMEART/ART03", "GAMEART/ART04", "GAMEART/ART05", "GAMEART/ART06", "GAMEART/ART07", "GAMEART/ART08",
    "GAMEART/ART09", "GAMEART/ART10", "GAMEART/ART11", "GAMEART/ART12", "GAMEART/ART13", "GAMEART/ART14", "GAMEART/ART15", "GAMEART/ART16",
    "GAMEART/ART17", "GAMEART/ART18", "GAMEART/ART19", "GAMEART/ART20", "GAMEART/ART21", "GAMEART/ART22", "GAMEART/ART23", "GAMEART/ART24",
];

/// local+0x18 as a level string; many exit codes keep a non-pointer there (the R3 "panel/savepanel
/// faults" in LOADER/STATUS/CREDITS were this probe dereferencing such values, not the game)
fn lvl_str(e: &scooby_recomp::engine::Engine, lo: u32) -> String {
    if lo == 0 { return String::new() }
    let p = e.mem.r32(lo + 0x18);
    if p < 0x0010_0000 || p >= 0x8000_0000 { return String::new() }
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| e.mem.cstr_at(p))).unwrap_or_default()
}

/// logic modules whose (ported) code calls APIgame::LoadLevel
pub const EXIT_CODES: &[&str] = &["controller", "democontrol", "dustdevil", "hero", "hs_changelevel", "hs_changetrack", "hs_commun", "hs_door", "inv_control", "shaggy", "inv_race", "keycageleveltwoa", "launch_control", "loader", "panel", "savepanel", "scoreboard", "stpanel", "zombieboss"];

fn pack(dir: &str) -> String {
    format!("{}.P2R", dir.rsplit('/').next().unwrap()).replace("L", if dir.starts_with("EP") { "L" } else { "L" })
}

pub fn load(fs: &IsoFs, dir: &str) -> (scooby_recomp::engine::Engine, EngineRuntime) {
    let (mut e, mut rt) = EngineRuntime::new_with_engine();
    let file = if dir.starts_with("EP") { format!("{}.P2R", dir.replace('/', "")) } else { pack(dir) };
    let res = EngineRuntime::ReadLevel(fs, dir, &file).unwrap();
    rt.LoadLevel(&mut e, res);
    (e, rt)
}

#[test]
#[ignore]
fn r3_probe_exits() {
    let fs = IsoFs::open(ISO).expect("ISO missing");
    let only = std::env::var("R3_LEVEL").ok();
    for dir in LEVELS.iter().filter(|d| only.as_deref().is_none_or(|o| o == **d)) {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| probe_one(&fs, dir)));
        if r.is_err() {
            println!("LEVEL {dir} PANIC");
        }
    }
}

fn probe_one(fs: &IsoFs, dir: &str) {
    {
        let t = std::time::Instant::now();
        let (mut e, mut rt) = load(fs, dir);
        let mut req = None;
        for _ in 0..120 {
            rt.Frame(&mut e, 1.0 / 60.0);
            if let Some(r) = rt.particles.lock().unwrap().game.take_level_request() {
                req.get_or_insert(r);
            }
        }
        let l = e.rt.game_buffers.get(&0).map(|b| e.mem.r32(b.0 + 8)).unwrap_or(0);
        println!("LEVEL {dir} leader {l:#x} objs {} t {:?} req {:?}", e.rt.objects.iter().flatten().count(), t.elapsed(), req.map(|r| r.level));
        for o in e.rt.objects.iter().flatten() {
            let Some(c) = o.inst.code else { continue };
            let n = e.rt.codes[c].def.name.to_lowercase();
            if EXIT_CODES.iter().any(|k| n.starts_with(k)) {
                let lo = o.inst.local;
                let s = lvl_str(&e, lo);
                println!("  {n} {:#x} lvl {:?}", o.id, s.chars().take(20).collect::<String>());
            }
        }
    }
}

type Eng = scooby_recomp::engine::Engine;

fn pos(rt: &EngineRuntime, id: u32) -> Option<[f32; 3]> {
    rt.particles.lock().unwrap().world.frames.get(&id).map(|m| m.pos)
}

fn warp(e: &mut Eng, rt: &EngineRuntime, to: [f32; 3]) {
    use scooby_recomp::engine::APIgameobj;
    let l = leader(e);
    let p = e.mem.alloca(0x10);
    for (i, c) in to.iter().enumerate() {
        e.mem.w32(p + 4 * i as u32, c.to_bits());
    }
    rt.particles.lock().unwrap().logic = true;
    APIgameobj::SetTranslation(e, l, p);
    let o = e.rt.game_buffers.get(&0).map(|b| e.mem.r32(b.0 + 0xc)).unwrap_or(0);
    if o != 0 && o != l {
        e.mem.w32(p, (to[0] + 1.0).to_bits());
        APIgameobj::SetTranslation(e, o, p);
    }
    rt.particles.lock().unwrap().logic = false;
}

fn load_carried(fs: &IsoFs, dir: &str, bufs: &[(u32, Vec<u8>)]) -> Result<(Eng, EngineRuntime), String> {
    let (mut e, mut rt) = EngineRuntime::new_with_engine();
    let file = format!("{}.P2R", dir.rsplit('/').next().filter(|_| !dir.starts_with("EP")).map(str::to_string).unwrap_or_else(|| dir.replace('/', "")));
    let res = EngineRuntime::ReadLevel(fs, dir, &file)?;
    // as bin/game.rs poll_load: the pack's EF_CUTSCENE object -> EFCutScenePlayer
    let cs = scooby_recomp::engine::script::EFCutScenePlayer::FromLevelObjects(&res.objects).ok();
    for (id, b) in bufs {
        let p = scooby_recomp::engine::host::gameobj::GetGameBuffer(&mut e, *id, b.len() as u32, 1);
        for (i, v) in b.iter().enumerate() {
            e.mem.w8(p + i as u32, *v);
        }
    }
    rt.LoadLevel(&mut e, res);
    rt.particles.lock().unwrap().cutscene = cs;
    Ok((e, rt))
}

/// one exit: warp the leader onto `uid`, unlock every GB2+0xc mask, press X; returns the
/// requested level (normalised dir) and the frame it was requested at
fn try_exit(fs: &IsoFs, dir: &str, uid: u32) -> (Option<(String, u32)>, Vec<(u32, Vec<u8>)>) {
    let (mut e, mut rt) = load_carried(fs, dir, &[]).unwrap();
    for _ in 0..60 {
        frame(&mut e, &mut rt, 0);
    }
    // hot spots without a world frame (no mesh): APIgameobj::GetPosition (physic/pos data)
    let Some(p) = pos(&rt, uid).or_else(|| {
        use scooby_recomp::engine::APIgameobj;
        let b = e.mem.alloca(0x10);
        rt.particles.lock().unwrap().logic = true;
        let r = APIgameobj::GetPosition(&mut e, uid, 0, b);
        rt.particles.lock().unwrap().logic = false;
        (r != 0).then(|| e.mem.rv3(b))
    }) else { return (None, vec![]) };
    if let Some(gb) = e.rt.game_buffers.get(&2).map(|b| b.0) {
        e.mem.w32(gb + 0xc, 0xffff_ffff);
    }
    warp(&mut e, &rt, p);
    if std::env::var_os("R3_DEBUG").is_some() {
        for _ in 0..30 { frame(&mut e, &mut rt, 0); }
        let near: Vec<_> = e.rt.objects.iter().flatten().filter_map(|o| { let q = pos(&rt, o.id)?; let d = (0..3).map(|i| (q[i] - p[i]).powi(2)).sum::<f32>().sqrt(); (d < 3.0).then(|| (o.id, d, o.inst.code.map(|c| e.rt.codes[c].def.name.clone()))) }).collect();
        println!("R3DBG door {uid:#x} at {p:?} leader now {:?} near {near:?}", pos(&rt, leader(&e)));
    }
    for f in 0..1800u32 {
        let b = if f % 20 < 3 { scooby_recomp::engine::input::pad::CROSS } else { 0 };
        // level-start scripts (EP1/L01 intro) may put the leader back: re-warp while it is away
        if f % 60 == 59 && pos(&rt, leader(&e)).is_some_and(|q| (0..3).map(|i| (q[i] - p[i]).powi(2)).sum::<f32>() > 9.0) {
            warp(&mut e, &rt, p);
        }
        if let Some(r) = frame(&mut e, &mut rt, b) {
            return (Some((norm(&r.level), f)), carry(&mut e));
        }
    }
    (None, vec![])
}

/// next level starts: load with the carried buffers, 300 frames, leader present
fn starts(fs: &IsoFs, dir: &str, bufs: &[(u32, Vec<u8>)]) -> Result<u32, String> {
    let (mut e, mut rt) = load_carried(fs, dir, bufs)?;
    for _ in 0..300 {
        frame(&mut e, &mut rt, 0);
    }
    let l = leader(&e);
    if l == 0 && dir.starts_with("EP") { Err("no leader".into()) } else { Ok(l) }
}

#[test]
#[ignore]
fn r3_door_chain() {
    let fs = IsoFs::open(ISO).expect("ISO missing");
    let only = std::env::var("R3_LEVEL").ok();
    for dir in LEVELS.iter().filter(|d| d.starts_with("EP") && only.as_deref().is_none_or(|o| o == **d)) {
        let doors = std::panic::catch_unwind(|| {
            let (mut e, mut rt) = load_carried(&fs, dir, &[]).unwrap();
            for _ in 0..60 {
                frame(&mut e, &mut rt, 0);
            }
            let mut v = vec![];
            for o in e.rt.objects.iter().flatten() {
                let Some(c) = o.inst.code else { continue };
                if !e.rt.codes[c].def.name.starts_with("hs_") { continue }
                let lo = o.inst.local;
                let s = lvl_str(&e, lo);
                if s.starts_with("EP") {
                    v.push((o.id, norm(&s)));
                }
            }
            v
        });
        let Ok(doors) = doors else {
            println!("R3 {dir} - - PANIC(load)");
            continue;
        };
        for (uid, want) in doors {
            if std::env::var("R3_DOOR").is_ok_and(|d| u32::from_str_radix(d.trim_start_matches("0x"), 16).ok() != Some(uid)) { continue }
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let (got, bufs) = try_exit(&fs, dir, uid);
                let Some((got, f)) = got else { return format!("no LoadLevel (want {want})") };
                if got != want {
                    return format!("WRONG {got} f{f} (want {want})");
                }
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| starts(&fs, &got, &bufs))) {
                    Ok(Ok(l)) => format!("ok f{f} -> {got} leader {l:#x}"),
                    Ok(Err(m)) => format!("{got} f{f} start FAIL {m}"),
                    Err(_) => format!("{got} f{f} start PANIC"),
                }
            }));
            println!("R3 {dir} {uid:#x} {want} {}", r.unwrap_or_else(|_| "PANIC(exit)".into()));
        }
    }
}

/// bin/game.rs cutscene_update (EFCutScenePlayer requests + Update per frame)
fn cutscene_update(e: &mut Eng, rt: &mut EngineRuntime) {
    use scooby_recomp::engine::runtime::CutSceneReq;
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

/// non-door exits: every exit-code object (EXIT_CODES) of every pack is fired in a fresh load:
/// msg 0x4f (the LoadLevel message of hs_commun/shaggy/keycage/dustdevil/zombieboss/door bases),
/// then warp + X (hot spots without a level string, e.g. hs_changelevel/hs_changetrack);
/// the requested level is loaded with the carried game buffers and run 300 frames.
#[test]
#[ignore]
fn r3_trigger_chain() {
    use scooby_recomp::engine::APIgameobj;
    let fs = IsoFs::open(ISO).expect("ISO missing");
    let only = std::env::var("R3_LEVEL").ok();
    for dir in LEVELS.iter().filter(|d| !d.starts_with("GAMEART") && only.as_deref().is_none_or(|o| o == **d)) {
        let Ok(objs) = std::panic::catch_unwind(|| {
            let (e, _rt) = load_carried(&fs, dir, &[]).unwrap();
            e.rt.objects.iter().flatten().filter_map(|o| {
                let n = e.rt.codes[o.inst.code?].def.name.to_lowercase();
                (EXIT_CODES.iter().any(|k| n.starts_with(k)) && !(n.starts_with("hs_door") && lvl_str(&e, o.inst.local).starts_with("EP"))).then_some((o.id, n))
            }).collect::<Vec<_>>()
        }) else {
            println!("R3T {dir} - PANIC(load)");
            continue;
        };
        for (uid, n) in objs {
            for how in ["msg4f", "warpX"] {
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let (mut e, mut rt) = load_carried(&fs, dir, &[]).unwrap();
                    for _ in 0..60 {
                        frame(&mut e, &mut rt, 0);
                    }
                    let mut got = None;
                    if how == "msg4f" {
                        rt.particles.lock().unwrap().logic = true;
                        APIgameobj::SendMessageTo(&mut e, uid, 0, 0x4f, 0, 0);
                        rt.particles.lock().unwrap().logic = false;
                        for f in 0..600u32 {
                            if let Some(r) = frame(&mut e, &mut rt, 0) {
                                got = Some((norm(&r.level), f));
                                break;
                            }
                        }
                    } else {
                        let lo = e.rt.objects.iter().flatten().find(|o| o.id == uid).map_or(0, |o| o.inst.local);
                        if n.starts_with("hs_door") && lo != 0 && e.mem.r32(lo + 0x18) == 0 {
                            // not in its build's goCustomDataDoor table: ST_FIRSTMOVE leaves L+0x18 = 0 and
                            // the C's X path would LoadLevel(NULL); only reachable by forcing the hot spot
                            return Some("skip: door not in the module table (L+0x18 NULL)".into());
                        }
                        let p = pos(&rt, uid)?;
                        warp(&mut e, &rt, p);
                        for f in 0..900u32 {
                            let b = if f % 20 < 3 { scooby_recomp::engine::input::pad::CROSS } else { 0 };
                            if f % 60 == 59 { warp(&mut e, &rt, p); }
                            if let Some(r) = frame(&mut e, &mut rt, b) {
                                got = Some((norm(&r.level), f));
                                break;
                            }
                        }
                    }
                    let (lvl, f) = got?;
                    let bufs = carry(&mut e);
                    Some(match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| starts(&fs, &lvl, &bufs))) {
                        Ok(Ok(l)) => format!("ok f{f} -> {lvl} leader {l:#x}"),
                        Ok(Err(m)) => format!("{lvl} f{f} start FAIL {m}"),
                        Err(_) => format!("{lvl} f{f} start PANIC"),
                    })
                }));
                match r {
                    Ok(None) => {}
                    Ok(Some(s)) => println!("R3T {dir} {n} {uid:#x} {how} {s}"),
                    Err(_) => println!("R3T {dir} {n} {uid:#x} {how} PANIC(exit)"),
                }
            }
        }
    }
}

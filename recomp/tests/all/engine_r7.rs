//! WP-R7: scripted level-end triggers (the R5/R6 "not fired" transitions). Each case loads a pack
//! headless (cut-scene player running), drives the C trigger chain with a small step script and
//! reports the EFEngineController level request (then loads the target with the carried buffers).
//! `cargo test --release --test engine_r7 -- --ignored --nocapture` (R7_CASE=<name> for one,
//! R7_STEPS="<level>|<steps>" for an ad-hoc script; steps below).
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::runtime::*;
use scooby_recomp::engine::APIgameobj;

use crate::common::{ISO, leader, pos, send, cutscene_update, frame, carry, load_carried, norm, hex, uid};
type Eng = scooby_recomp::engine::Engine;

fn warp(e: &mut Eng, rt: &EngineRuntime, to: [f32; 3]) {
    let l = leader(e);
    let p = e.mem.alloca(0x10);
    for (i, c) in to.iter().enumerate() {
        e.mem.w32(p + 4 * i as u32, c.to_bits());
    }
    rt.particles.lock().unwrap().logic = true;
    APIgameobj::SetTranslation(e, l, p);
    rt.particles.lock().unwrap().logic = false;
}

/// Step script, ';'-separated:
/// `f N` frames; `x N` frames pressing X (3 of every 20), re-warping to the last warp target each 60;
/// `p BTN N` frames holding pad bits BTN (hex); `m UID MSG P2 [W..]` SendMessageTo (data words);
/// `warp UID` leader onto the object; `l UID OFF VAL` write local+OFF of UID; `g ID OFF VAL` write
/// game buffer ID +OFF; `dbg UID` print local words; `dm ADDR LEN` dump memory. Runs until a level request (stops the script).
fn run(fs: &IsoFs, dir: &str, steps: &str) -> Option<(String, u32)> {
    let (mut e, mut rt) = load_carried(fs, dir, &[]).unwrap();
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
                let d: Vec<u32> = a[4..].iter().map(|w| if w.starts_with('@') { uid(&e, w) } else { hex(w) }).collect();
                let u = uid(&e, a[1]);
                send(&mut e, &rt, u, hex(a[2]), hex(a[3]), &d);
                (0, 0)
            }
            "warp" => {
                let u = uid(&e, a[1]);
                at = pos(&mut e, &rt, u);
                println!("R7   warp {u:#x} -> {at:?} leader {:#x}", leader(&e));
                if let Some(p) = at { warp(&mut e, &rt, p) }
                (0, 0)
            }
            "l" => {
                let u = uid(&e, a[1]);
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
                            println!("R7   obj {:#x} {n} build {} local {:#x}", o.id, e.rt.codes[k].build, o.inst.local);
                        }
                    }
                }
                (0, 0)
            }
            "gd" => {
                let b = e.rt.game_buffers.get(&hex(a[1])).map(|b| b.0).expect("game buffer");
                let w: Vec<String> = (0..hex(a[2]) / 4).map(|i| format!("{:x}", e.mem.r32(b + 4 * i))).collect();
                println!("R7   f{f} gb{} {}", a[1], w.join(" "));
                (0, 0)
            }
            "play" => {
                // EFCutScenePlayer::Play(id) as APIgame::PlayCutScene (no sound): the scripted cut-scene
                rt.particles.lock().unwrap().cutscene_req.push(CutSceneReq::Play(hex(a[1]), 0xffff_ffff, 0x7fff_ffff));
                (0, 0)
            }
            "cs" => {
                let g = rt.particles.lock().unwrap();
                println!("R7   f{f} cutscene {:?}", g.cutscene.as_ref().map(|c| (c.current, c.flags, c.time, c.duration, c.scene(c.current).map(|s| s.actors.clone()))));
                (0, 0)
            }
            "wxyz" => {
                // warp the leader to a world position (re-applied by `x` like `warp`)
                at = Some([a[1].parse().unwrap(), a[2].parse().unwrap(), a[3].parse().unwrap()]);
                warp(&mut e, &rt, at.unwrap());
                (0, 0)
            }
            "put" => {
                // move object A onto object B (hot spot onto a vehicle whose spline overrides warps)
                let (a1, b1) = (uid(&e, a[1]), uid(&e, a[2]));
                if let Some(to) = pos(&mut e, &rt, b1) {
                    let p = e.mem.alloca(0x10);
                    for (i, c) in to.iter().enumerate() {
                        e.mem.w32(p + 4 * i as u32, c.to_bits());
                    }
                    rt.particles.lock().unwrap().logic = true;
                    APIgameobj::SetTranslation(&mut e, a1, p);
                    rt.particles.lock().unwrap().logic = false;
                }
                (0, 0)
            }
            "phys" => {
                let u = uid(&e, a[1]);
                let g = rt.particles.lock().unwrap();
                let id = g.sub.physic.PhysicOf(u);
                match g.sub.physic.Physic(id) {
                    Some(p) => println!("R7   f{f} phys {u:#x} id {id} flags {:#x} state {:#x} notify {:#x} masks {:x?} prims {:?}", p.flags, p.state, p.notify_bits, p.model.masks, p.model.prims.iter().map(|q| q.as_ref().map(|q| format!("{:?}", q.base))).collect::<Vec<_>>()),
                    None => println!("R7   f{f} phys {u:#x}: no physic"),
                }
                (0, 0)
            }
            "dbg" => {
                let u = uid(&e, a[1]);
                let lo = e.rt.objects.iter().flatten().find(|o| o.id == u).map_or(0, |o| o.inst.local);
                let w: Vec<String> = (0..hex(a.get(2).unwrap_or(&"20")) / 4).map(|i| format!("{:x}", e.mem.r32(lo + 4 * i))).collect();
                println!("R7   f{f} {u:#x} local {}", w.join(" "));
                (0, 0)
            }
            "dm" => {
                // dump memory: `dm ADDR LEN` (hex), e.g. a hero's pLocalDataHero (local word 0)
                let w: Vec<String> = (0..hex(a[2]) / 4).map(|i| format!("{:x}", e.mem.r32(hex(a[1]) + 4 * i))).collect();
                println!("R7   f{f} mem {} {}", a[1], w.join(" "));
                (0, 0)
            }
            s => panic!("bad step {s}"),
        };
        for i in 0..n {
            let b = if a[0] == "x" && i % 20 >= 3 { 0 } else { btn };
            if a[0] == "x" && i % 60 == 59 {
                if let Some(p) = at { warp(&mut e, &rt, p) }
            }
            f += 1;
            if let Some(r) = frame(&mut e, &mut rt, b) {
                got = Some((norm(&r.level), f));
                break 'outer;
            }
        }
    }
    let (lvl, f) = got?;
    let bufs = carry(&mut e);
    let ok = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let (mut e, mut rt) = load_carried(fs, &lvl, &bufs).ok()?;
        for _ in 0..300 {
            frame(&mut e, &mut rt, 0);
        }
        Some(leader(&e))
    }));
    println!("R7   -> {lvl} at f{f}; target load+300 frames: {:?}", ok.map(|l| l.map(|l| format!("{l:#x}"))).unwrap_or(Some("PANIC".into())));
    Some((lvl, f))
}

/// (name, level, steps, expected next level)
const CASES: &[(&str, &str, &str, &str)] = &[
    // C chain: racecontroller 0xde -> hs 0x2a00001a 0xde RegisterNotify(3); hero 0x4b/4c collides ->
    // hs_communFUNC_SetLoading (L+0x10 = 1, camera 0x2a000037 0x54 P[0x10] = 1 fade) -> 0x4f -> LoadLevel.
    // R8: real path: hot-spot physics are created at load (EFGameObject::InitHotspot); 0xde (as sent
    // by racecontroller) registers the notify, the leader car 0x4c drives into the hot spot.
    ("hs_commun", "EP2/L01C", "f 60; m 2a00001a de 0; warp 2a00001a; f 30; warp 2a00001a; f 200", "EP2/L02A"),
    // KeyCageLevelTwoA X with key flag GB2+8 bit 0x2000000 (GB 2 = GetGameBuffer(2,0x158)) at the action
    // prim (108.7,0.6,103.3) -> 0x1de param 0 to camera 0x2a000025 -> cut-scene 0 (cage, 16 s, no load: C
    // CB_CutScene loads only on the end of cut-scene 3). Cut-scene 3 end -> bLoading + fade -> 0x4f -> L03A.
    ("keycage", "EP2/L02A", "f 60; g 2 8 2000000; wxyz 108.0 0.0 103.35; f 20; x 3; f 1100; play 3; f 1600", "EP2/L03A"),
    // R9: nobody plays cut-scene 3 in EP2/L02A (all 0x1de senders in the pack's modules: keycage X
    // param 0, hs_door_l01 param 1, button 0xfc param 2; no 0x1df / PlayCutScene / PlugNPlayCutScene;
    // the cut-scene player has no message script), so keycage's CB_CutScene(3) load branch is dead
    // here. The real L02A -> L03A chain is Shaggy's: button 0x2a0000fb X (GB2+0xc bits 0x8000000 and
    // 0x4000000 set, else text 0x15c) -> fcback_InputAction_X -> 0x99 MSG_PUSH_BUTTON to the leader
    // (Shaggy 0x2a000032, build 26992d64, leader at load) -> heroInit_Action anim 0x1a -> last frame
    // shaggyCB_ANIM_LF_LOCAL: L+0x38c, camera 0x54 {0x62, [0x10] fade 1} -> 0x4f -> LoadLevel L03A.
    ("shaggy", "EP2/L02A", "f 60; g 2 c c000000; wxyz 104.76 5.7 91.5; f 20; x 3; f 600", "EP2/L03A"),
    // laps -> SetLoading (L+0x6c) ; shortest: gate + 0x4f
    ("launch_control", "EP2/L04", "f 60; l @launch_control 6c 1; m @launch_control 4f 0; f 60", "STATUS"),
    // f69740fd: 7 x 0xdf -> L+0x771c + fade cam 0x2a00000c -> 0x4f
    ("inv_control_ep6l03", "EP6/L03", "f 60; m @inv_control df 0; m @inv_control df 0; m @inv_control df 0; m @inv_control df 0; m @inv_control df 0; m @inv_control df 0; m @inv_control df 0; f 600", "EP6/L04A"),
    // a555cfe4: msg 0x180 (from GhostBossLevel5) -> LoadLevel LOADER, no gate
    ("inv_control_ep6l05b", "EP6/L05B", "f 60; m @inv_control 180 0; f 10", "LOADER"),
    // 3b2c42cd: counter +0x7708 > 3 via 0x1ab/0x1ac/0x1ad/0x1ae (each counts once: L+0x7720..0x772c latches) -> CB_TIMER_END fade -> 0x4f (1st: cut-scene, 2nd: load)
    ("inv_control_ep6l05", "EP6/L05", "f 60; m @inv_control 1ab 0; m @inv_control 1ac 0; m @inv_control 1ad 0; m @inv_control 1ae 0; f 1200", "EP6/L05B"),
    // 4361ff83 (EP4/L01): CB_CUTSCENE end of cut-scene 3 -> L+0xc19c + fade cam 0x0e -> 0x4f -> LOADER
    ("inv_control_ep4l01", "EP4/L01", "f 60; play 3; f 1500", "LOADER"),
    // f0b75891 (EP1/L04): CB_CUT_SCENE on any cut-scene end (notify sender 0) -> +0x772c + fade -> 0x4f -> LOADER (EP1 extro)
    ("inv_control_ep1l04", "EP1/L04", "f 60; play 0; f 2000", "LOADER"),
    // STPanel L+0x54 = 1 then msg 0xdf (sent by Cut.c at cut-scene end) -> LoadNextLevel (SPLASH for GB+0x18 1..4)
    ("status", "STATUS", "f 60; g 0 18 1; l 2a10000c 54 1; m 2a10000c df 0; f 60", "SPLASH"),
    // scoreboard: 0xd1 (score struct) shows the board, X sets +0xdc, fade -> 0x4f -> LEAVING_LEVEL (11 ticks) -> GB+0x7e0
    ("scoreboard", "EP1/L09", "f 60; m @scoreboard d1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0; x 900; f 600", "*"),
    // hs_changelevel 0x2a000226: collide with 0x2a000004 (tag 0x2c8, 0x17e answer 0) -> L+0x14, L+8 = self, fade -> 0x4f
    // R8: real path: the ride spline overrides warps, so the hot spot is put onto the cart 0x2a000004
    // (prim tag 0x2c8) -> NotifyColide -> fade cam 0x248 -> 0x4f -> LoadLevel.
    ("hs_changelevel", "EP3/L03", "f 60; put 2a000226 2a000004; f 1; put 2a000226 2a000004; f 1; put 2a000226 2a000004; f 300", "EP3/L04"),
];

#[test]
#[ignore]
fn r7_transitions() {
    let fs = IsoFs::open(ISO).expect("ISO missing");
    if let Ok(s) = std::env::var("R7_STEPS") {
        let (dir, steps) = s.split_once('|').expect("R7_STEPS=<level>|<steps>");
        println!("R7 adhoc {dir}: {:?}", run(&fs, dir, steps));
        return;
    }
    let only = std::env::var("R7_CASE").ok();
    for (name, dir, steps, want) in CASES.iter().filter(|c| only.as_deref().is_none_or(|o| o == c.0)) {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(&fs, dir, steps)));
        let s = match &r {
            Ok(Some((l, f))) if *want == "*" || l.eq_ignore_ascii_case(want) => format!("FIRED f{f} -> {l}"),
            Ok(Some((l, f))) => format!("WRONG f{f} -> {l} (want {want})"),
            Ok(None) => "not fired".into(),
            Err(_) => "PANIC".into(),
        };
        println!("R7 {name} {dir}: {s}");
    }
}

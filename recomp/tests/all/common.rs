//! WP-T1: helpers shared by the single test binary (tests/all/main.rs).
#![allow(dead_code)]

/// The disc image (hard link in the worktree root). P20 R51: anchored at CARGO_MANIFEST_DIR (recomp/) so a
/// test binary started from another cwd still finds it (was "../Scooby…" relative to the cwd).
pub const ISO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../Scooby Doo Mystery Mayhem.ISO");

/// P20 R51: long ISO-gated tests: false (and a loud SKIP line on stderr) when the ISO is missing, instead of
/// silently passing
pub fn iso_or_skip(test: &str) -> bool {
    let ok = std::path::Path::new(ISO).exists();
    if !ok {
        eprintln!("SKIP {test}: ISO not found at {ISO}");
    }
    ok
}

/// Exclusive lock for tests that touch process-global state (env vars via set_var, static tables/logs,
/// cwd). All test files share one process now, so such tests must hold `let _g = crate::common::serial();`.
/// Poison-tolerant: a failed holder does not fail every later test.
pub fn serial() -> std::sync::MutexGuard<'static, ()> {
    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

// --- WP-T1: level-chain step helpers (were identical copies in engine_p11/p17/r3/r7/r10) ---
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::runtime::*;
use scooby_recomp::engine::APIgameobj;
type Eng = scooby_recomp::engine::Engine;

pub fn leader(e: &Eng) -> u32 {
    e.rt.game_buffers.get(&0).map(|b| e.mem.r32(b.0 + 8)).unwrap_or(0)
}

pub fn pos(e: &mut Eng, rt: &EngineRuntime, id: u32) -> Option<[f32; 3]> {
    if let Some(m) = rt.particles.lock().unwrap().world.frames.get(&id) {
        return Some(m.pos);
    }
    // P32 R90: release the scratch (an unreleased alloca per call overflowed the 256 KB module stack after
    // ~16k frame-less lookups, e.g. bookzg on EP4/L04: "module stack overflow" at f4344)
    let sp = e.mem.sp();
    let b = e.mem.alloca(0x10);
    rt.particles.lock().unwrap().logic = true;
    let r = APIgameobj::GetPosition(e, id, 0, b);
    rt.particles.lock().unwrap().logic = false;
    let v = (r != 0).then(|| e.mem.rv3(b));
    e.mem.set_sp(sp);
    v
}

pub fn warp(e: &mut Eng, rt: &EngineRuntime, to: [f32; 3]) {
    let l = leader(e);
    let sp = e.mem.sp();
    let p = e.mem.alloca(0x10);
    for (i, c) in to.iter().enumerate() {
        e.mem.w32(p + 4 * i as u32, c.to_bits());
    }
    rt.particles.lock().unwrap().logic = true;
    APIgameobj::SetTranslation(e, l, p);
    // the buddy (GB0+0xc) comes along, 1 m aside (as engine_r3): doors need both heroes (GB0+0x34 0x110)
    let o = e.rt.game_buffers.get(&0).map(|b| e.mem.r32(b.0 + 0xc)).unwrap_or(0);
    if o != 0 && o != l {
        e.mem.w32(p, (to[0] + 1.0).to_bits());
        APIgameobj::SetTranslation(e, o, p);
    }
    rt.particles.lock().unwrap().logic = false;
    e.mem.set_sp(sp);
}

pub fn send(e: &mut Eng, rt: &EngineRuntime, uid: u32, m: u32, p2: u32, data: &[u32]) {
    let sp = e.mem.sp();
    let d = if data.is_empty() { 0 } else {
        let p = e.mem.alloca(0x40.max(4 * data.len() as u32));
        for (i, w) in data.iter().enumerate() {
            e.mem.w32(p + 4 * i as u32, *w);
        }
        p
    };
    rt.particles.lock().unwrap().logic = true;
    APIgameobj::SendMessageTo(e, uid, 0, m, p2, d);
    rt.particles.lock().unwrap().logic = false;
    e.mem.set_sp(sp);
}

pub fn cutscene_update(e: &mut Eng, rt: &mut EngineRuntime) {
    use scooby_recomp::engine::script::RuntimeCutSceneHost;
    let (cs, reqs) = {
        let mut p = rt.particles.lock().unwrap();
        // keep the player in place (cloned): IsInCutScene from the 0x16 notify callbacks must see it
        (p.cutscene.clone(), std::mem::take(&mut p.cutscene_req))
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
    cs.Update(&mut h, step_dt());
    rt.particles.lock().unwrap().cutscene = Some(cs);
}

/// logic step of `frame`/`cutscene_update` in Hz. The original runs at 30 fps (ENGINE.INI MaxFPS = 30 ->
/// EFGraphicsMgr_SP installs VSYNCHandler, which lets every 2nd vblank through); routes were tuned at 60.
/// P28 R9: per test thread (was a process-wide AtomicU32: a probe's HZ 30, or another test's load_carried
/// reset to 60, changed the step of routes running in parallel -> ep12_real_carry_ep2 / p17_chains failed
/// at EP2/L01A only in the full gate). Same store/load calls as the atomic.
pub struct ThreadHz;
thread_local! { static HZ_TL: std::cell::Cell<u32> = const { std::cell::Cell::new(60) }; }
impl ThreadHz {
    pub fn store(&self, v: u32, _: std::sync::atomic::Ordering) {
        HZ_TL.with(|c| c.set(v));
    }
    pub fn load(&self, _: std::sync::atomic::Ordering) -> u32 {
        HZ_TL.with(|c| c.get())
    }
}
pub static HZ: ThreadHz = ThreadHz;
pub fn step_dt() -> f32 {
    1.0 / HZ.load(std::sync::atomic::Ordering::Relaxed) as f32
}

pub fn frame(e: &mut Eng, rt: &mut EngineRuntime, buttons: u16) -> Option<scooby_recomp::engine::host::game::LevelRequest> {
    frame_stick(e, rt, buttons, (128, 128))
}

/// P32 R59: optional per-frame hook (per test thread), e.g. a boss state tracer
pub type FrameHook = Box<dyn FnMut(&mut Eng, &mut EngineRuntime)>;
thread_local! { pub static FRAME_HOOK: std::cell::RefCell<Option<FrameHook>> = std::cell::RefCell::new(None); }

/// `frame` with the left stick at `(lx, ly)` (0..255, 128 = centre, y down = 255)
pub fn frame_stick(e: &mut Eng, rt: &mut EngineRuntime, buttons: u16, (lx, ly): (u8, u8)) -> Option<scooby_recomp::engine::host::game::LevelRequest> {
    use scooby_recomp::engine::input::PadState;
    rt.particles.lock().unwrap().devices.SetPadState(0, PadState { connected: true, mode: 7, rx: 128, ry: 128, lx, ly, buttons, ..Default::default() });
    // P27 R8: route sticks are aimed with the camera visible before the frame (engine_p17 steer): the physics
    // converts them back with it (EngineRuntime::stick_cam), so world-space steering is exact
    if (lx, ly) != (128, 128) {
        rt.stick_cam = rt.gfx.cameras.cameras.iter().flatten().next().map(|c| c.ltm);
    }
    let played = rt.StepInputPlayed(e, step_dt());
    publish_nodes(rt, Some(&played));
    rt.FrameAfterInput(e, step_dt());
    cutscene_update(e, rt);
    publish_nodes(rt, None);
    if let Ok(r) = std::env::var("P24_CTRK") { courage_track(e, rt, r.parse().unwrap_or(15.0)) }
    if let Ok(r) = std::env::var("P20_KTRK") { knight_track(e, rt, r.parse().unwrap_or(10)) }
    FRAME_HOOK.with(|h| if let Some(f) = h.borrow_mut().as_mut() { f(e, rt) });
    rt.particles.lock().unwrap().game.take_level_request()
}

/// P24 R40 debug (`P24_CTRK=R`): on every frame the courage (GB1+0) changes, print it, the leader position and
/// every object within R m (xz) of the leader: uid, module, state, position.
/// P20 R171 debug (`P20_KTRK=N`, EP2/L04 Red Knight retune): every N frames and on every GB0+0x14 change print
/// the leader + partner positions, the knight 0x2a0000c4 position/state, GB0+0x14 and courage GB1+0.
fn knight_track(e: &mut Eng, rt: &mut EngineRuntime, every: u32) {
    use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
    static LAST: AtomicU32 = AtomicU32::new(u32::MAX);
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Relaxed);
    let Some(b0) = e.rt.game_buffers.get(&0).map(|b| b.0) else { return };
    let w14 = e.mem.r32(b0 + 0x14);
    if LAST.swap(w14, Relaxed) == w14 && n % every.max(1) != 0 { return }
    let cg = e.rt.game_buffers.get(&1).map_or(0, |b| e.mem.r32(b.0));
    let (l, pa) = (e.mem.r32(b0 + 8), e.mem.r32(b0 + 0xc));
    let r = |p: Option<[f32; 3]>| p.map(|p| p.map(|v| (v * 10.0).round() / 10.0));
    let (hl, hp, k) = (r(pos(e, rt, l)), r(pos(e, rt, pa)), r(pos(e, rt, 0x2a0000c4)));
    let ks = scooby_recomp::engine::core::apigameobj::GetCurrentStateID(e, 0x2a0000c4);
    let lo = e.rt.objects.iter().flatten().find(|o| o.id == 0x2a0000c4).map_or(0, |o| o.inst.local);
    let (act, kd) = if lo != 0 { (e.mem.r32(lo + 40), e.mem.rf(lo + 72)) } else { (0, 0.0) };
    println!("KTRK n{n} w14 {w14:#x} c {:.2} L {hl:?} P {hp:?} K {k:?} st {ks:#x} act {act} kd {kd:.1}", f32::from_bits(cg));
}

fn courage_track(e: &mut Eng, rt: &mut EngineRuntime, r: f32) {
    use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
    static LAST: AtomicU32 = AtomicU32::new(u32::MAX);
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Relaxed);
    let Some(cg) = e.rt.game_buffers.get(&1).map(|b| e.mem.r32(b.0)) else { return };
    if LAST.swap(cg, Relaxed) == cg { return }
    let l = leader(e);
    let Some(h) = pos(e, rt, l) else { return };
    println!("CTRK n{n} courage {} leader {h:?}", f32::from_bits(cg));
    let ids: Vec<(u32, Option<usize>)> = e.rt.objects.iter().flatten().map(|o| (o.id, o.inst.code)).collect();
    for (u, k) in ids {
        if u == l { continue }
        if let Some(p) = pos(e, rt, u) {
            let d = (p[0] - h[0]).hypot(p[2] - h[2]);
            if d <= r {
                let m = k.map(|k| e.rt.codes[k].def.name.to_string());
                println!("CTRK   {u:#x} {m:?} st {} d {d:.1} at {p:?}", scooby_recomp::engine::core::apigameobj::GetCurrentStateID(e, u));
            }
        }
    }
}

/// P21 R69: the headless harness has no renderer, so RuntimeWorld::nodes (published by bin/game.rs
/// draw_objects) stayed empty and every bone-attached coll prim stayed on the root frame (EP6/L01
/// automaticdoor 0x44: its panel prim on joint 3 kept blocking the doorway after the open anim).
/// Same pose + node publish as draw_objects (gfx::render::pose_nodes), for the objects whose physic
/// has a prim on a joint other than 0 / root (GetBone ignores node 0).
/// R70: ON for EP3/EP4/EP6 levels (P21 routes retuned on moving bone prims), OFF for EP1/EP2 and the other
/// tests until P20 retunes EP2/L01C (on there it stops firing); P17_BONES=1 / =0 forces it on / off everywhere.
struct Nodes {
    res: Vec<scooby_recomp::engine::gfx::EF3dObjRes>,
    objs: std::collections::HashMap<u32, scooby_recomp::engine::gfx::render::EF3dObj_SP>,
    on: bool,
    /// R12 route step `nodes UID,..`: extra objects for the current level only (reset on load)
    extra: Vec<u32>,
}

/// bone publish default per level (see `publish_nodes`)
pub fn bones_on(dir: &str) -> bool {
    match std::env::var("P17_BONES").as_deref() {
        Ok("1") => true,
        Ok("0") => false,
        // P20 R104: EP2 on too (game.rs always publishes bones); EP1 still off.
        // TODO(P20): EP2/L04 laps under bones (lap 2 wagon 0x2a800004 hold at 635,294; lap 3 hero frozen at
        // 573.6,251.5, frankenstein 0x2a0000c4?) -> retune, then drop this exception
        _ if dir.starts_with("EP2/L04") => false,
        _ => ["EP2/", "EP3/", "EP4/", "EP6/"].iter().any(|p| dir.starts_with(p)),
    }
}
thread_local! { static NODES: std::cell::RefCell<Option<Nodes>> = const { std::cell::RefCell::new(None) }; }

/// route step `nodes`: publish joints of these objects too, until the next level load
pub fn add_nodes(uids: &[u32]) {
    NODES.with_borrow_mut(|n| if let Some(n) = n.as_mut() { n.extra.extend_from_slice(uids) });
}

/// `only`: re-pose just these objects (P22 R23: objects whose anim the input callbacks played)
fn publish_nodes(rt: &mut EngineRuntime, only: Option<&[u32]>) {
    use scooby_recomp::engine::gfx::render::{pose_nodes, EF3dObj_SP};
    NODES.with_borrow_mut(|n| {
        let Some(n) = n.as_mut().filter(|n| n.on) else { return };
        let mut p = rt.particles.lock().unwrap();
        // P23 R9: the C (EFWorld::Update -> EF3dObj::UpdateHierarchy per game object; GetObjectFrame(bone) =
        // the HAnim node RwFrame) has joint frames for EVERY 3D object, as game.rs draw_objects publishes.
        // P17_NODESET=all does that, but it changes EP2/EP3/EP4/EP6 routes tuned on the origin fallback
        // (TODO retune, then make it the default). Default: joint-prim owners + heroes (P21 R69 / P24 R7)
        // + the objects in AddChild pairs (ParticleShared.parented: EP6/L02 robot grab AddChild(.., 0x2402000,
        // ..) snapped the robot onto the labcart origin = into chest 0x154 and launched both).
        let mut uids: Vec<u32> = p.sub.obj3d.keys().copied().collect();
        if std::env::var("P17_NODESET").as_deref() != Ok("all") {
            uids = p.sub.physic.physics.iter().flatten().filter(|q| q.model.prims.iter().flatten().any(|r| r.base.joint != 0 && r.base.joint != u32::MAX)).map(|q| q.owner).collect();
            uids.extend([0x2a00_0007u32, 0x2a00_0008].into_iter().filter(|u| p.sub.anim_bind.contains_key(u)));
            if std::env::var("P17_NODESET").as_deref() == Ok("par") {
                uids.extend(p.parented.iter().flat_map(|&(c, q, _, _)| [c, q]));
            }
            // P22 R44: with GAME_EMCHILD=1, the parents of bone-attached emitters (EP4/L04 boss gas on HAnim
            // 0x2401000 = head/mouth node 12; without the node it fell back to the boss feet). Off by default
            // (no attached emitters), so the default set is unchanged.
            if p.em_child {
                use scooby_recomp::engine::particle::PLFrameAttach;
                let par: Vec<u32> = p.pl.types.iter().flat_map(|t| t.emitters.iter()).filter_map(|em| match em.frame.as_ref()?.attach {
                    PLFrameAttach::Child { obj, bone } if bone != 0x7fff_ffff => Some(obj),
                    _ => None,
                }).collect();
                uids.extend(par);
            }
            // P17_NODEUIDS=<hex>,<hex>: extra objects (ad hoc probes)
            uids.extend(std::env::var("P17_NODEUIDS").unwrap_or_default().split(',').filter_map(|h| u32::from_str_radix(h, 16).ok()));
            uids.extend(n.extra.iter().copied());
        }
        uids.sort_unstable();
        uids.dedup();
        if let Some(only) = only {
            uids.retain(|u| only.contains(u));
        }
        for uid in uids {
            let Some(&(res_uid, _)) = p.sub.obj3d.get(&uid) else { continue };
            let Some(frame) = p.world.frames.get(&uid).copied() else { continue };
            let Some(c) = n.res.iter().find(|r| r.res.uid == res_uid).and_then(|r| r.clump.as_ref()) else { continue };
            let o = n.objs.entry(uid).or_insert_with(|| {
                let mut o = EF3dObj_SP::Initialize(res_uid, c, 0);
                o.SetVisible(1);
                o
            });
            let ltm = [frame.right[0], frame.right[1], frame.right[2], 0., frame.up[0], frame.up[1], frame.up[2], 0., frame.at[0], frame.at[1], frame.at[2], 0., frame.pos[0], frame.pos[1], frame.pos[2], 1.];
            let pose = p.sub.anim_bind.get(&uid).and_then(|b| p.sub.anim.get(&uid).and_then(|m| m.GetPose(b.obj3d)));
            // P32 R98: name the object in a crash report (crashlog context "obj") while it is posed
            scooby_recomp::crashlog::OBJ.store(uid, std::sync::atomic::Ordering::Relaxed);
            let posed = pose_nodes(o, c, pose.as_deref(), ltm);
            scooby_recomp::crashlog::OBJ.store(0, std::sync::atomic::Ordering::Relaxed);
            if let Some((nodes, ids)) = posed {
                p.world.nodes.insert(uid, nodes);
                p.world.node_ids.insert(uid, ids);
                p.world.node_root.insert(uid, frame);
                // P28 R11: register the clump for the logic-side poser (PlayHAnim during the logic pass)
                if !p.sub.node_poser.clumps.contains_key(&res_uid) {
                    p.sub.node_poser.clumps.insert(res_uid, c.clone());
                }
            }
        }
    });
}

// EFGameMemoryMgr::Terminate at LoadLevel: only persistent buffers survive (WP-P19)
pub fn carry(e: &mut Eng) -> Vec<(u32, Vec<u8>)> {
    scooby_recomp::engine::host::gameobj::CarriedGameBuffers(e)
}

/// P20 R130: ANIMSND (anim sound table, as game.exe) is the headless default; `P17_ANIMSND=0` opts out
/// (old routes tuned without it).
pub fn animsnd_on() -> bool {
    std::env::var("P17_ANIMSND").as_deref() != Ok("0")
}

/// P20 R170: CAMANIM (EFDMCameraFlex anim channels, as game.exe) is the headless default; `P17_CAMANIM=0` opts out.
pub fn camanim_on() -> bool {
    std::env::var("P17_CAMANIM").as_deref() != Ok("0")
}

pub fn load_carried(fs: &IsoFs, dir: &str, bufs: &[(u32, Vec<u8>)]) -> Result<(Eng, EngineRuntime), String> {
    install_crash_handler();
    HZ.store(60, std::sync::atomic::Ordering::Relaxed);
    // P22 R49: the memory card's TransferIcons reads the save icons from the ISO (apiloadsave.rs disc_file: GAME_ISO or
    // cwd-relative paths). Pin it to the test ISO so a run started outside recomp/ writes the same save (without the icons the
    // EP4/L01 checkpoint save took another menu path and the 0x188 respawn went to the level start).
    if std::env::var_os("GAME_ISO").is_none() {
        // SAFETY: tests only read it during memory-card calls on the test thread; the value is the same for every test
        unsafe { std::env::set_var("GAME_ISO", ISO) };
    }
    let (mut e, mut rt) = EngineRuntime::new_with_engine();
    // P20 R121 ANIMSND (see tools/engineport/common.md): anim sound table, game.exe has it on
    rt.particles.lock().unwrap().sub.anim_sound = animsnd_on();
    rt.particles.lock().unwrap().sub.cam_anim = camanim_on();
    let file = format!("{}.P2R", dir.rsplit('/').next().filter(|_| !dir.starts_with("EP")).map(str::to_string).unwrap_or_else(|| dir.replace('/', "")));
    let res = EngineRuntime::ReadLevel(fs, dir, &file)?;
    let cs = scooby_recomp::engine::script::EFCutScenePlayer::FromLevelObjects(&res.objects).ok();
    let mut r3d = vec![];
    for o in &res.objects {
        let Some(r) = o.as_raw() else { continue };
        let mut x = scooby_recomp::engine::gfx::EF3dObjRes::default();
        if r.class_id == 0x5b && x.LoadPropertiesBinary(&r.data).is_ok() {
            r3d.push(x);
        }
    }
    NODES.with_borrow_mut(|n| *n = Some(Nodes { res: r3d, objs: Default::default(), on: bones_on(dir), extra: vec![] }));
    for (id, b) in bufs {
        let p = scooby_recomp::engine::host::gameobj::GetGameBuffer(&mut e, *id, b.len() as u32, 1);
        for (i, v) in b.iter().enumerate() {
            e.mem.w8(p + i as u32, *v);
        }
    }
    rt.LoadLevel(&mut e, res);
    rt.particles.lock().unwrap().cutscene = cs;
    // P22 R26 GAME_LOGICCLIP: atomic clip spheres per EF3dObj resource
    if scooby_recomp::engine::host::gameobj::logic_clip_enabled() {
        NODES.with_borrow(|n| {
            let Some(n) = n.as_ref() else { return };
            let mut p = rt.particles.lock().unwrap();
            for x in &n.res {
                if let Some(c) = &x.clump {
                    p.s1.clip_spheres.insert(x.res.uid, scooby_recomp::engine::gfx::render::clip_spheres(x.res.uid, c));
                }
            }
        });
    }
    Ok((e, rt))
}

pub fn norm(l: &str) -> String {
    let d = l.replace(char::from(0x5c), "/").trim_matches('/').to_string();
    if d.contains('/') || d.len() < 6 || !d.starts_with("EP") { d } else { format!("{}/{}", &d[..3], &d[3..]) }
}

pub fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or_else(|_| panic!("bad hex {s}"))
}

/// object uid: hex, or `@code` = first object whose logic code name starts with `code`
pub fn uid(e: &Eng, s: &str) -> u32 {
    match s.strip_prefix('@') {
        Some(c) => e.rt.objects.iter().flatten().find(|o| o.inst.code.is_some_and(|k| e.rt.codes[k].def.name.to_lowercase().starts_with(c))).unwrap_or_else(|| panic!("no object with code {c}")).id,
        None => hex(s),
    }
}

pub fn load_dump(path: &str) -> Vec<(u32, Vec<u8>)> {
    let d = std::fs::read(path).expect("carry dump");
    let mut v = vec![];
    let mut i = 0;
    while i + 8 <= d.len() {
        let id = u32::from_le_bytes(d[i..i + 4].try_into().unwrap());
        let n = u32::from_le_bytes(d[i + 4..i + 8].try_into().unwrap()) as usize;
        v.push((id, d[i + 8..i + 8 + n].to_vec()));
        i += 8 + n;
    }
    v
}

/// P20 R168 / P32 R73: crash log (scooby_recomp::crashlog): unhandled native exceptions + panics -> exception
/// code, address, module+offset, backtrace, frame/level/object to $SCOOBY_CRASHLOG (default %TEMP%/scooby_crash.log);
/// `P17_CRASHDBG=1` also logs first-chance faults.
pub fn install_crash_handler() {
    scooby_recomp::crashlog::install();
}

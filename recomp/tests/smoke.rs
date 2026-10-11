//! End-to-end smoke test: libraries + the Tome of Doom loaded, constructed and run for 10 s of frames
//! against a host that implements nothing (every engine call returns 0). Checks that the logic runtime,
//! library registration and the book's states/callbacks run without panicking and behave as the
//! original would with such an engine.
use scooby_recomp::engine::{Engine, Host, Mem, Ptr, NULL};
use scooby_recomp::libs;
use scooby_recomp::logic::ModuleDef;
use scooby_recomp::modules::tomedoom;
use scooby_recomp::engine::APIcons;
use scooby_recomp::logic::LResult;
use scooby_recomp::modules::tod_beam;
use scooby_recomp::modules::{generic_fx, wisps, wispsourcenew};
use scooby_recomp::engine::core::apigameobj::GetGameBuffer;
use std::collections::HashMap;

/// the engine's own timer (EFGameLogicMgr creates it first): the game clock the logic reads
const GAME_TIMER: u32 = 0x9000_0000;
use std::sync::{Arc, Mutex};

struct NullHost;
impl Host for NullHost {}

/// Engine stand-in: everything returns 0 as in [`NullHost`], except the matrix getters, which return
/// a stable identity RwMatrix (0x40 bytes: right, flags, up, at, pos) per (object, bone). The real
/// engine never returns NULL from them, and the game dereferences the result without checking.
#[derive(Default)]
struct TestHost {
    mats: HashMap<(u32, u32), Ptr>,
    /// (object, sound) passed to APInewSound::Play
    sounds: Arc<Mutex<Vec<(u32, u32)>>>,
    /// (object, x): that object stands at (x, 0, 0) instead of the origin
    away: Option<(u32, f32)>,
    /// (object, anim id) passed to APIanim::Play / PlayBlend
    anims: Arc<Mutex<Vec<(u32, u32)>>>,
    /// (window, angle) passed to APIgameobj::Rotate (HUD meters)
    rotations: Arc<Mutex<Vec<(u32, f32)>>>,
    /// (window, angle) passed to APIgameobj::SetRotation
    set_rotations: Arc<Mutex<Vec<(u32, f32)>>>,
    /// (window, from texture, to texture) passed to APIgraphics::TexFaderPlay
    faders: Arc<Mutex<Vec<(u32, String, String)>>>,
}
impl TestHost {
    fn mat(&mut self, mem: &mut Mem, obj: u32, bone: u32) -> Ptr {
        *self.mats.entry((obj, bone)).or_insert_with(|| {
            let m = mem.malloc(0x40);
            mem.memset(m, 0, 0x40);
            for (o, v) in [(0x00, 1.0f32), (0x14, 1.0), (0x28, 1.0)] {
                mem.wf(m + o, v);
            }
            mem.w32(m + 0xc, 0x20003); // rwMATRIXTYPEORTHONORMAL | rwMATRIXINTERNALIDENTITY
            m
        })
    }
}
impl Host for TestHost {
    fn APIgameobj_GetLMat(&mut self, mem: &mut Mem, a0: u32) -> i32 {
        self.mat(mem, a0, u32::MAX) as i32
    }
    fn APIgameobj_GetWMat(&mut self, mem: &mut Mem, a0: u32) -> u32 {
        self.mat(mem, a0, u32::MAX - 1)
    }
    fn APIgameobj_GetBoneLMat(&mut self, mem: &mut Mem, a0: u32, a1: u32) -> i32 {
        self.mat(mem, a0, a1) as i32
    }
    fn APIgameobj_GetBoneWMat(&mut self, mem: &mut Mem, a0: u32, a1: u32) -> u32 {
        self.mat(mem, a0, a1 | 0x8000_0000)
    }
    /// every object stands at the origin (except `away`)
    fn APIgameobj_GetPosition(&mut self, mem: &mut Mem, a0: u32, _: u32, a2: Ptr) -> Ptr {
        for o in [0, 4, 8] {
            mem.wf(a2 + o, 0.0);
        }
        if let Some((id, x)) = self.away {
            if id == a0 {
                mem.wf(a2, x);
            }
        }
        a2
    }
    /// every waypoint path has 4 points, (i, 0, 0)
    fn APIextra_WaypointGetNb(&mut self, _: &mut Mem, _: u32) -> u32 {
        4
    }
    fn APIextra_WaypointGetPos(&mut self, mem: &mut Mem, _: u32, a1: u32, a2: Ptr) -> Ptr {
        mem.wf(a2, a1 as f32);
        mem.wf(a2 + 4, 0.0);
        mem.wf(a2 + 8, 0.0);
        a2
    }
    fn APIgameobj_Rotate(&mut self, _: &mut Mem, a0: u32, _: Ptr, a2: f32) -> u32 {
        self.rotations.lock().unwrap().push((a0, a2));
        1
    }
    fn APIgameobj_SetRotation(&mut self, _: &mut Mem, a0: u32, _: Ptr, a2: f32) -> u32 {
        self.set_rotations.lock().unwrap().push((a0, a2));
        1
    }
    fn APIgraphics_TexFaderPlay(&mut self, mem: &mut Mem, a0: u32, a1: Ptr) -> u32 {
        let (f, t) = (mem.r32(a1 + 8), mem.r32(a1 + 0x14));
        let s = |a: Ptr| if a == 0 { String::new() } else { mem.cstr_at(a) };
        self.faders.lock().unwrap().push((a0, s(f), s(t)));
        1
    }
    fn APIanim_Play(&mut self, _: &mut Mem, a0: u32, a1: u32, _: u32, _: f32) -> u32 {
        self.anims.lock().unwrap().push((a0, a1));
        1
    }
    fn APIanim_PlayBlend(&mut self, _: &mut Mem, a0: u32, a1: u32, _: u32, _: f32, _: Ptr) -> u32 {
        self.anims.lock().unwrap().push((a0, a1));
        1
    }
    fn APIanim_PlayBlendConnect(&mut self, _: &mut Mem, a0: u32, a1: u32, _: u32, _: u32, _: Ptr) -> u32 {
        self.anims.lock().unwrap().push((a0, a1));
        1
    }
    fn APInewSound_Play(&mut self, _: &mut Mem, a0: u32, a1: u32, _: u32) -> u32 {
        self.sounds.lock().unwrap().push((a0, a1));
        0
    }
}

/// Stand-in for the book in the beam test: a logic object that records (message, sub) it receives.
static PROBE_RECEIVED: Mutex<Vec<(u32, u32)>> = Mutex::new(Vec::new());
fn probe_msg(_: &mut Engine, msg: u32, sub: u32, _: Ptr) -> LResult<u64> {
    PROBE_RECEIVED.lock().unwrap().push((msg, sub));
    Ok(0)
}
fn probe_ctor(e: &mut Engine, _: Ptr, _: i32) -> LResult<()> {
    APIcons::SetMessageFunction(e, probe_msg);
    Ok(())
}
fn probe_dtor(_: &mut Engine, _: i32) -> LResult<()> {
    Ok(())
}
static PROBE: ModuleDef = ModuleDef { name: "probe", data_size: 0x10, ctor: probe_ctor, dtor: probe_dtor, register: None, init: &[], build_init: None };

/// Loads library `def` and constructs its object, as the level loader does (EFLogicCodeLib::LoadFile,
/// then the lib object runs once).
fn load_lib(e: &mut Engine, def: &'static ModuleDef, build: usize, id: u32) {
    let c = e.load_module(def, build);
    let h = e.add_object(id, Some(c));
    e.run_object(h, 1.0 / 60.0);
}

fn engine_with_libs() -> Engine {
    engine_with_libs_on(Box::new(TestHost::default()))
}

fn engine_with_libs_on(host: Box<dyn Host>) -> Engine {
    let mut e = Engine::new(host);
    load_lib(&mut e, &libs::anim_mngr::MODULE, 0, 0x2a00_1001);
    load_lib(&mut e, &libs::misc_func::MODULE, 0, 0x2a00_1002);
    let b = libs::api_c_functions::build_index("a1d1e457").unwrap();
    load_lib(&mut e, &libs::api_c_functions::MODULE, b, 0x2a00_1003);
    load_lib(&mut e, &libs::api_text::MODULE, 0, 0x2a00_1004);
    // ghost_base registers with the sector clipping lib (uses apilist); levels load both libs
    load_lib(&mut e, &libs::api_list::MODULE, 0, 0x2a00_1005);
    load_lib(&mut e, &libs::api_sector_clipping::MODULE, 0, 0x2a00_1006);
    // movers query paths (build 0: any level image will do for the smoke tests)
    load_lib(&mut e, &libs::api_path_finding::MODULE, 0, 0x2a00_1007);
    e
}

#[test]
fn null_host_constructs_everything() {
    // with an engine that returns 0 everywhere, loading and constructing must still work
    let mut e = engine_with_libs_on(Box::new(NullHost));
    let b = tomedoom::build_index("802a33e5").unwrap();
    let c = e.load_module(&tomedoom::MODULE, b);
    let tome = e.add_object(0x2a00_00f0, Some(c));
    e.run_object(tome, 1.0 / 60.0);
    e.remove_object(tome);
}

/// The beam's whole life: open -> last frame -> hook (0xd6/0x67) -> close; animation event 3 and the
/// closing animation's last frame tell the book (0xd6/0x69, 0xd6/0x68), then the beam kills itself.
#[test]
fn tod_beam_open_hook_close() {
    let mut e = engine_with_libs();
    let b = tod_beam::build_index("01ad949b").unwrap();
    let book_id = tod_beam::builds::BUILDS[b].k[tod_beam::builds::ID_2a000012];
    let pc = e.load_module(&PROBE, 0);
    let probe = e.add_object(book_id, Some(pc));
    e.run_object(probe, 1.0 / 60.0);

    let c = e.load_module(&tod_beam::MODULE, b);
    let beam = e.add_object(0x2a00_00a0, Some(c));
    let dt = 1.0 / 60.0;
    for _ in 0..3 {
        e.run_object(beam, dt); // construct, FIRSTMOVE, ENTRYPOINT -> Opening_In -> Opening_Execute
    }
    e.call_cb(beam, 2, 0); // ANIM_LF: opening animation done
    e.run_object(beam, dt); // -> Hooking_In stored
    e.run_object(beam, dt); // Hooking_In -> Hooking_Execute
    let payload = e.malloc(0x10);
    e.wf(payload + 8, 0.7);
    e.SendLogicMessageTo(0x2a00_00a0, 0xd6, 0x67, payload, NULL); // monster hooked
    e.run_object(beam, dt); // -> Closing_In stored
    e.run_object(beam, dt); // Closing_In -> Closing_Execute
    assert!(e.rt.kill_list.is_empty());
    let ev = e.malloc(8);
    e.w32(ev, 3);
    e.call_cb(beam, 3, ev); // animation event 3
    e.call_cb(beam, 2, 0); // closing animation done
    e.run_object(beam, dt);
    assert_eq!(*PROBE_RECEIVED.lock().unwrap(), vec![(0xd6, 0x69), (0xd6, 0x68)]);
    // KillObject(self) queued the beam; the end of the frame deletes it
    assert_eq!(e.rt.kill_list, vec![beam]);
    e.ProcessKillList();
    assert!(e.rt.map.find(0x2a00_00a0).is_none());
}

#[test]
fn libraries_register_their_tables() {
    let mut e = engine_with_libs();
    for name in ["AnimMngr", "MiscFunc", "ApiCFunctions", "ApiText"] {
        let n = e.cstr(name);
        assert_ne!(scooby_recomp::engine::core::apicons::GetApiCustom(&mut e, n), NULL, "{name} not registered");
    }
    let n = e.cstr("debug");
    assert_eq!(scooby_recomp::engine::core::apicons::GetApiCustom(&mut e, n), NULL);
}

#[test]
fn tome_of_doom_runs() {
    let mut e = engine_with_libs();
    let b = tomedoom::build_index("802a33e5").expect("EP1L01 build");
    let c = e.load_module(&tomedoom::MODULE, b);
    let tome = e.add_object(0x2a00_00f0, Some(c));
    // frame 1 constructs (Run does not run a state on the constructing frame), then states run
    for _ in 0..600 {
        e.run_object(tome, 1.0 / 60.0);
    }
    // every callback the constructor registered. Some take a pointer (CB_DMSTATECHANGE: {dm, ?, bits}),
    // others an id: a zeroed block serves both (as an id it matches nothing)
    let arg = e.malloc(0x100);
    for idx in [6, 7, 0xb, 0xc, 0xf, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17] {
        e.call_cb(tome, idx, arg);
        e.run_object(tome, 1.0 / 60.0);
    }
    // messages the book handles, to its id, with no payload
    for msg in [0x0d, 0x14, 0x15, 0x3c, 0x5b, 0x6e, 0x127] {
        e.SendLogicMessageTo(0x2a00_00f0, msg, 0, NULL, NULL);
        e.run_object(tome, 1.0 / 60.0);
    }
    e.remove_object(tome);
}

/// Stand-in for the book and the HUD in the wisp tests: records (receiver id, message, sub).
static WISP_PROBE_RECEIVED: Mutex<Vec<(u32, u32, u32)>> = Mutex::new(Vec::new());
fn wisp_probe_msg(e: &mut Engine, msg: u32, sub: u32, _: Ptr) -> LResult<u64> {
    let me = scooby_recomp::engine::APIgameobj::GetID(e, 0x2a00_0000, NULL);
    WISP_PROBE_RECEIVED.lock().unwrap().push((me, msg, sub));
    Ok(0)
}
fn wisp_probe_ctor(e: &mut Engine, _: Ptr, _: i32) -> LResult<()> {
    APIcons::SetMessageFunction(e, wisp_probe_msg);
    Ok(())
}
static WISP_PROBE: ModuleDef =
    ModuleDef { name: "wisp_probe", data_size: 0x10, ctor: wisp_probe_ctor, dtor: probe_dtor, register: None, init: &[], build_init: None };

/// Level 1 wisps with the book/HUD probes, the GENERIC_FX latent, hero 0x2a000091, wisps switched on
/// (game buffer 2 bit 0) and the book power at 100. Returns (engine, wisps code, book id, hud id, power ptr).
fn wisp_level(sounds: Arc<Mutex<Vec<(u32, u32)>>>) -> (Engine, usize, u32, u32, Ptr) {
    let mut e = engine_with_libs_on(Box::new(TestHost { sounds, ..Default::default() }));
    let b = wisps::build_index("l01_wisps").unwrap();
    let k = wisps::builds::BUILDS[b].k;
    let (book, hud, fx) = (k[wisps::builds::ID_2a000025], k[wisps::builds::ID_2a100006], k[wisps::builds::ID_2a80016a]);
    let pc = e.load_module(&WISP_PROBE, 0);
    for id in [book, hud] {
        let h = e.add_object(id, Some(pc));
        e.run_object(h, 1.0 / 60.0);
    }
    let gc = e.load_module(&generic_fx::MODULE, 0);
    e.rt.latents.insert(fx, Some(gc));
    let gb0 = GetGameBuffer(&mut e, 0, 0x8f8, 1);
    e.w32(gb0 + 8, 0x2a00_0091);
    let gb2 = GetGameBuffer(&mut e, 2, 0x158, 1);
    e.w32(gb2, 1);
    let gb1 = GetGameBuffer(&mut e, 1, 0x68, 1);
    e.wf(gb1 + 4, 100.0);
    let c = e.load_module(&wisps::MODULE, b);
    (e, c, book, hud, gb1 + 4)
}

/// A small wisp touched by the current hero: +18 power, collect sound on the book, 0x1d to the HUD,
/// its "collected" bit set in game buffer 2 +0x14, then it waits in Dead.
#[test]
fn wisp_touch_refills_book_power() {
    let sounds = Arc::new(Mutex::new(Vec::new()));
    let (mut e, c, book, hud, power) = wisp_level(sounds.clone());
    let dt = 1.0 / 60.0;
    let w = e.add_object(0x2a00_0074, Some(c)); // TmpWispMap record 0: small, bit 1
    for _ in 0..3 {
        e.run_object(w, dt); // construct; FIRSTMOVE (-> Active); ENTRY_POINT (bob)
    }
    let col = e.malloc(0x10);
    e.w32(col + 8, 0x2a00_0091);
    assert_eq!(e.call_cb(w, 2, col), 1, "touch by the hero picks the wisp");
    e.run_object(w, dt); // Active_Execute -> Picked_In stored
    e.run_object(w, dt); // Picked_In (sparkles), Picked_Execute waits for the clock to pass
    assert_eq!(e.rf(power), 100.0);
    e.GetTimer(GAME_TIMER).elapsed = 1.0;
    e.run_object(w, dt);
    assert_eq!(e.rf(power), 118.0);
    assert_eq!(*sounds.lock().unwrap(), vec![(book, 0x17)]);
    let got: Vec<_> = WISP_PROBE_RECEIVED.lock().unwrap().iter().copied().filter(|r| r.0 == hud || r.0 == book).collect();
    assert!(got.contains(&(hud, 0x1d, 0)), "{got:x?}");
    let gb2 = GetGameBuffer(&mut e, 2, 0x158, 1);
    assert_eq!(e.r32(gb2 + 0x14) & 1, 1);
}

/// The open book sucks a wisp (message 0x11e sub 1): hooked, picked within 1 unit of the book,
/// 0.2 s later +36 for a medium wisp, capped at 180, and 0x122 with the size to the book.
#[test]
fn wisp_sucked_by_book_caps_power() {
    let sounds = Arc::new(Mutex::new(Vec::new()));
    let (mut e, c, book, _hud, power) = wisp_level(sounds.clone());
    e.GetTimer(GAME_TIMER).elapsed = 5.0;
    e.wf(power, 170.0);
    let dt = 1.0 / 60.0;
    let w = e.add_object(0x2a00_0088, Some(c)); // record 4: medium (2), bit 0x400
    for _ in 0..3 {
        e.run_object(w, dt);
    }
    e.SendLogicMessageTo(0x2a00_0088, 0x11e, 1, NULL, NULL);
    e.run_object(w, dt); // Active_Execute -> Hooked_In
    e.run_object(w, dt); // Hooked_In -> Hooked_Execute: at the book -> picked
    e.run_object(w, dt); // -> Picked_In (+0.2 s), Picked_Execute waits
    assert_eq!(e.rf(power), 170.0);
    e.GetTimer(GAME_TIMER).elapsed = 5.3;
    e.run_object(w, dt);
    assert_eq!(e.rf(power), 180.0);
    let got = WISP_PROBE_RECEIVED.lock().unwrap().clone();
    assert!(got.contains(&(book, 0x122, 2)), "{got:x?}");
}

/// Stand-in for the wisps a huge wisp carries: records (receiver id, message, sub).
static HW_PROBE_RECEIVED: Mutex<Vec<(u32, u32, u32)>> = Mutex::new(Vec::new());
fn hw_probe_msg(e: &mut Engine, msg: u32, sub: u32, _: Ptr) -> LResult<u64> {
    let me = scooby_recomp::engine::APIgameobj::GetID(e, 0x2a00_0000, NULL);
    HW_PROBE_RECEIVED.lock().unwrap().push((me, msg, sub));
    Ok(0)
}
fn hw_probe_ctor(e: &mut Engine, _: Ptr, _: i32) -> LResult<()> {
    APIcons::SetMessageFunction(e, hw_probe_msg);
    Ok(())
}
static HW_PROBE: ModuleDef =
    ModuleDef { name: "hw_probe", data_size: 0x10, ctor: hw_probe_ctor, dtor: probe_dtor, register: None, init: &[], build_init: None };

/// The huge wisp (EP6L05): launches its three wisps (0x11f/1 then 0x98 each; 0x166 to all three in
/// FIRSTMOVE) and hums (sound 0x2d). The hero touching volume 2 makes the second wisp suckable (0x165)
/// until that animation ends (0x166); with the book power full a touch does nothing; a collected wisp
/// (0x120) is replaced by a new launch (0x11f/1 only).
#[test]
fn huge_wisp_source_releases_and_replaces() {
    let sounds = Arc::new(Mutex::new(Vec::new()));
    let mut e = engine_with_libs_on(Box::new(TestHost { sounds: sounds.clone(), ..Default::default() }));
    let b = wispsourcenew::build_index("1a407d93").unwrap();
    let k = wispsourcenew::builds::BUILDS[b].k;
    let pc = e.load_module(&HW_PROBE, 0);
    e.rt.latents.insert(k[wispsourcenew::builds::ID_2a800069], Some(pc));
    let gb0 = GetGameBuffer(&mut e, 0, 0x8f8, 1);
    e.w32(gb0 + 8, 0x2a00_0091);
    let gb2 = GetGameBuffer(&mut e, 2, 0x158, 1);
    e.w32(gb2, 1);
    let gb1 = GetGameBuffer(&mut e, 1, 0x68, 1);
    e.wf(gb1 + 4, 100.0);
    let c = e.load_module(&wispsourcenew::MODULE, b);
    let src = e.add_object(0x2a00_0300, Some(c));
    let dt = 1.0 / 60.0;
    for _ in 0..4 {
        e.run_object(src, dt); // construct; FIRSTMOVE; BASICMOVE_SET; BASICMOVE_LOOP
    }
    let got = HW_PROBE_RECEIVED.lock().unwrap().clone();
    let kids: Vec<u32> = got.iter().filter(|r| r.1 == 0x11f).map(|r| r.0).collect();
    assert_eq!(kids.len(), 3, "{got:x?}");
    for &w in &kids {
        let mine: Vec<_> = got.iter().filter(|r| r.0 == w).map(|r| (r.1, r.2)).collect();
        assert_eq!(mine, vec![(0x11f, 1), (0x98, 0), (0x166, 0)]);
    }
    assert!(sounds.lock().unwrap().iter().any(|s| s.1 == 0x2d));

    let col = e.malloc(0x10);
    e.w32(col + 4, 2);
    e.w32(col + 8, 0x2a00_0091);
    e.wf(gb1 + 4, 180.0);
    e.call_cb(src, 3, col); // power full: nothing
    e.wf(gb1 + 4, 100.0);
    e.call_cb(src, 3, col);
    e.call_cb(src, 3, col); // volume 2 busy: nothing
    e.call_cb(src, 5, 0); // its animation ends
    let w2 = kids[1];
    let after: Vec<_> = HW_PROBE_RECEIVED.lock().unwrap()[got.len()..].to_vec();
    assert_eq!(after, vec![(w2, 0x165, 0), (w2, 0x166, 0)]);

    e.SendLogicMessageTo(0x2a00_0300, 0x120, w2, NULL, NULL);
    let all = HW_PROBE_RECEIVED.lock().unwrap().clone();
    let new: Vec<_> = all[got.len() + 2..].to_vec();
    assert_eq!(new.len(), 1, "{new:x?}");
    assert_eq!((new[0].1, new[0].2), (0x11f, 1));
    assert!(!kids.contains(&new[0].0));
    e.remove_object(src);
}

/// Stand-in for the book, the HUD and the capture manager in the ghost tests: records
/// (receiver id, message, sub).
static GHOST_PROBE_RECEIVED: Mutex<Vec<(u32, u32, u32)>> = Mutex::new(Vec::new());
fn ghost_probe_msg(e: &mut Engine, msg: u32, sub: u32, _: Ptr) -> LResult<u64> {
    let me = scooby_recomp::engine::APIgameobj::GetID(e, 0x2a00_0000, NULL);
    GHOST_PROBE_RECEIVED.lock().unwrap().push((me, msg, sub));
    Ok(0)
}
fn ghost_probe_ctor(e: &mut Engine, _: Ptr, _: i32) -> LResult<()> {
    APIcons::SetMessageFunction(e, ghost_probe_msg);
    Ok(())
}
static GHOST_PROBE: ModuleDef =
    ModuleDef { name: "ghost_probe", data_size: 0x10, ctor: ghost_probe_ctor, dtor: probe_dtor, register: None, init: &[], build_init: None };

/// Every ghost build constructs its first ghost and runs a second of frames (START -> roaming) with
/// the book closed, the book open (flee), then hooked, without panicking.
#[test]
fn every_ghost_build_runs() {
    use scooby_recomp::modules::ghost;
    for b in 0..ghost::builds::BUILDS.len() {
        let mut e = engine_with_libs();
        let gb0 = GetGameBuffer(&mut e, 0, 0x8f8, 1);
        e.w32(gb0 + 8, 0x2a00_0091);
        let gb1 = GetGameBuffer(&mut e, 1, 0x68, 1);
        e.wf(gb1 + 4, 100.0);
        let c = e.load_module(&ghost::MODULE, b);
        let id = ghost::maps::GO_CUSTOM_DATA_GHOST[b][0];
        let g = e.add_object(id, Some(c));
        for i in 0..180 {
            if i == 60 {
                e.w32(gb0 + 0x14, 1); // book open
            }
            if i == 120 {
                let p = e.malloc(4);
                e.w32(p, 0x2a00_0093);
                e.SendLogicMessageTo(id, 0xd6, 0x65, p, NULL); // hooked by the beam
            }
            e.run_object(g, 1.0 / 60.0);
        }
        e.remove_object(g);
    }
}

/// EP1L01 small ghost and the book: roams with the book closed, flees (state 0x18) when it opens,
/// is hooked by the beam (0xd6/0x65: state 0x15, answers 0xd6/0x66 to the book) and captured
/// (0xd6/0x68: kill mask in game buffer 2, 0x26 to the HUD, state 0xb -> 0x14e to the manager).
#[test]
fn ghost_flees_open_book_and_is_captured() {
    use scooby_recomp::modules::ghost;
    let mut e = engine_with_libs();
    let b = ghost::build_index("757255de").unwrap();
    let k = ghost::builds::BUILDS[b].k;
    let (book, hud, mgr) = (k[ghost::builds::ID_2a000093], k[ghost::builds::ID_2a100006], k[ghost::builds::ID_2a000096]);
    let pc = e.load_module(&GHOST_PROBE, 0);
    for pid in [book, hud, mgr] {
        let h = e.add_object(pid, Some(pc));
        e.run_object(h, 1.0 / 60.0);
    }
    let gb0 = GetGameBuffer(&mut e, 0, 0x8f8, 1);
    e.w32(gb0 + 8, 0x2a00_0091);
    let gb1 = GetGameBuffer(&mut e, 1, 0x68, 1);
    e.wf(gb1 + 4, 100.0);
    let gb2 = GetGameBuffer(&mut e, 2, 0x158, 1);
    let c = e.load_module(&ghost::MODULE, b);
    let rec = ghost::maps::GO_CUSTOM_DATA_GHOST[b];
    let (id, kill_mask) = (rec[0], rec[3]);
    let g = e.add_object(id, Some(c));
    let dt = 1.0 / 60.0;
    for _ in 0..10 {
        e.run_object(g, dt);
    }
    let state = |e: &Engine| e.rt.obj(g).inst.state;
    assert_ne!(state(&e), 0x18);
    e.w32(gb0 + 0x14, 1); // the book opens
    e.run_object(g, dt);
    e.run_object(g, dt);
    assert_eq!(state(&e), 0x18, "flees the open book");

    let p = e.malloc(4);
    e.w32(p, book);
    e.SendLogicMessageTo(id, 0xd6, 0x65, p, NULL);
    e.run_object(g, dt);
    e.run_object(g, dt);
    assert_eq!(state(&e), 0x15, "hooked by the beam");
    assert!(GHOST_PROBE_RECEIVED.lock().unwrap().contains(&(book, 0xd6, 0x66)));

    e.SendLogicMessageTo(id, 0xd6, 0x68, NULL, NULL);
    e.run_object(g, dt);
    e.run_object(g, dt);
    assert_eq!(state(&e), 0xb, "captured");
    assert_ne!(e.r32(gb2 + 4) & kill_mask, 0);
    let got = GHOST_PROBE_RECEIVED.lock().unwrap().clone();
    assert!(got.contains(&(hud, 0x26, 0)), "{got:x?}");
    assert!(got.contains(&(mgr, 0x14e, id)), "{got:x?}");
}

/// Every shadows build constructs its first shadow and runs frames with the book closed, open, then
/// hooked, without panicking.
#[test]
fn every_shadows_build_runs() {
    use scooby_recomp::modules::shadows;
    for b in 0..shadows::builds::BUILDS.len() {
        let mut e = engine_with_libs();
        let gb0 = GetGameBuffer(&mut e, 0, 0x8f8, 1);
        e.w32(gb0 + 8, 0x2a00_0091);
        let gb1 = GetGameBuffer(&mut e, 1, 0x68, 1);
        e.wf(gb1 + 4, 100.0);
        let c = e.load_module(&shadows::MODULE, b);
        let id = shadows::maps::GO_CUSTOM_DATA_SHADOWS[b][0];
        let s = e.add_object(id, Some(c));
        for i in 0..180 {
            if i == 60 {
                e.w32(gb0 + 0x14, 1); // book open
            }
            if i == 120 {
                let p = e.malloc(4);
                e.w32(p, 0x2a00_0093);
                e.SendLogicMessageTo(id, 0xd6, 0x65, p, NULL); // hooked by the beam
            }
            e.run_object(s, 1.0 / 60.0);
        }
        e.remove_object(s);
    }
}

/// EP1 shadow and the book: waits at its post (0xf) with the book closed, goes back to the post
/// (0x17) when it opens, struggles when hooked, and is captured (0xd6/0x68: kill mask in game
/// buffer 2, 0x26 to the HUD).
#[test]
fn shadow_flees_open_book_and_is_captured() {
    use scooby_recomp::modules::shadows;
    let mut e = engine_with_libs();
    let b = shadows::build_index("shadowslevelone").unwrap();
    let k = shadows::builds::BUILDS[b].k;
    let (book, hud, mgr) = (k[shadows::builds::ID_2a000093], k[shadows::builds::ID_2a100006], k[shadows::builds::ID_2a000096]);
    let pc = e.load_module(&GHOST_PROBE, 0);
    for pid in [book, hud, mgr] {
        let h = e.add_object(pid, Some(pc));
        e.run_object(h, 1.0 / 60.0);
    }
    let gb0 = GetGameBuffer(&mut e, 0, 0x8f8, 1);
    e.w32(gb0 + 8, 0x2a00_0091);
    let gb1 = GetGameBuffer(&mut e, 1, 0x68, 1);
    e.wf(gb1 + 4, 100.0);
    let gb2 = GetGameBuffer(&mut e, 2, 0x158, 1);
    let c = e.load_module(&shadows::MODULE, b);
    let rec = shadows::maps::GO_CUSTOM_DATA_SHADOWS[b];
    let (id, kill_mask) = (rec[0], rec[2]);
    let s = e.add_object(id, Some(c));
    let dt = 1.0 / 60.0;
    for _ in 0..10 {
        e.run_object(s, dt);
    }
    let state = |e: &Engine| e.rt.obj(s).inst.state;
    assert_eq!(state(&e), 0xf, "waiting at its post");
    e.w32(gb0 + 0x14, 1); // the book opens
    e.run_object(s, dt);
    assert_eq!(state(&e), 0x17, "backs off from the open book");

    // hooked on the way back (STAY_AWAY_FROM_TOME, 0x18, ignores the hook in the original)
    let p = e.malloc(4);
    e.w32(p, book);
    e.SendLogicMessageTo(id, 0xd6, 0x65, p, NULL);
    e.run_object(s, dt);
    // BEINGSUCKED runs at once with this frame's events: the stand-in host puts the shadow at its post
    // and the hero in range, so it lashes out (bouga, WHAT_IS_NEXT 0x13)
    assert_eq!(state(&e), 0x13, "hooked by the beam, lashing out");
    assert!(GHOST_PROBE_RECEIVED.lock().unwrap().contains(&(book, 0xd6, 0x66)));

    e.SendLogicMessageTo(id, 0xd6, 0x68, NULL, NULL);
    for _ in 0..3 {
        e.run_object(s, dt);
    }
    assert_ne!(e.r32(gb2 + 4) & kill_mask, 0, "remembered as captured");
    let got = GHOST_PROBE_RECEIVED.lock().unwrap().clone();
    assert!(got.contains(&(hud, 0x26, 0)), "{got:x?}");
}

/// Every zombie build constructs its first zombie and runs frames with the book closed, open, then
/// hooked, without panicking.
#[test]
fn every_zombie_build_runs() {
    use scooby_recomp::modules::zombie;
    for b in 0..zombie::builds::BUILDS.len() {
        let mut e = engine_with_libs();
        let gb0 = GetGameBuffer(&mut e, 0, 0x8f8, 1);
        e.w32(gb0 + 8, 0x2a00_0091);
        let gb1 = GetGameBuffer(&mut e, 1, 0x68, 1);
        e.wf(gb1 + 4, 100.0);
        let c = e.load_module(&zombie::MODULE, b);
        let id = zombie::maps::GO_CUSTOM_DATA_ZOMBIE[b][0];
        let s = e.add_object(id, Some(c));
        for i in 0..180 {
            if i == 60 {
                e.w32(gb0 + 0x14, 1); // book open
            }
            if i == 120 {
                let p = e.malloc(4);
                e.w32(p, 0x2a00_0093);
                e.SendLogicMessageTo(id, 0xd6, 0x65, p, NULL); // hooked by the beam
            }
            e.run_object(s, 1.0 / 60.0);
        }
        e.remove_object(s);
    }
}

/// Ep6 L04A wandering zombie and the book: wanders (0x18) with the book closed, mills about
/// (SCARED_NOTHOOKED 0x1b) when it opens, cowers when hooked (SCARED 0x19, the book told 0xd6/0x66),
/// and is captured (0xd6/0x68: kill mask in game buffer 2, 0x26 to the HUD).
#[test]
fn zombie_scared_by_open_book_and_is_captured() {
    use scooby_recomp::modules::zombie;
    // the hero 10 m away: inside the open book's 20 m, outside the 6 m chase range
    let mut e = engine_with_libs_on(Box::new(TestHost { away: Some((0x2a00_0091, 10.0)), ..Default::default() }));
    let b = zombie::build_index("zombielevelfoura_b").unwrap();
    let book = zombie::builds::BUILDS[b].k[zombie::builds::ID_2a000045];
    let hud = 0x2a10_0006;
    let pc = e.load_module(&GHOST_PROBE, 0);
    for pid in [book, hud] {
        let h = e.add_object(pid, Some(pc));
        e.run_object(h, 1.0 / 60.0);
    }
    let gb0 = GetGameBuffer(&mut e, 0, 0x8f8, 1);
    e.w32(gb0 + 8, 0x2a00_0091);
    let gb1 = GetGameBuffer(&mut e, 1, 0x68, 1);
    e.wf(gb1 + 4, 100.0);
    let gb2 = GetGameBuffer(&mut e, 2, 0x158, 1);
    let c = e.load_module(&zombie::MODULE, b);
    let rec = zombie::maps::GO_CUSTOM_DATA_ZOMBIE[b];
    let (id, behaviour, kill_mask) = (rec[0], rec[2], rec[3]);
    assert_eq!(behaviour, 2, "a wanderer");
    let s = e.add_object(id, Some(c));
    let dt = 1.0 / 60.0;
    let state = |e: &Engine| e.rt.obj(s).inst.state;
    for _ in 0..10 {
        e.run_object(s, dt);
    }
    assert_eq!(state(&e), 0x18, "wandering");
    e.w32(gb0 + 0x14, 1); // the book opens
    for _ in 0..2 {
        e.run_object(s, dt);
    }
    assert_eq!(state(&e), 0x1c, "scared by the open book, not hooked");

    let p = e.malloc(4);
    e.w32(p, book);
    e.SendLogicMessageTo(id, 0xd6, 0x65, p, NULL);
    e.run_object(s, dt);
    assert_eq!(state(&e), 0x1a, "hooked by the beam, cowering");
    assert!(GHOST_PROBE_RECEIVED.lock().unwrap().contains(&(book, 0xd6, 0x66)));

    e.SendLogicMessageTo(id, 0xd6, 0x68, NULL, NULL);
    for _ in 0..3 {
        e.run_object(s, dt);
    }
    assert_ne!(e.r32(gb2 + 4) & kill_mask, 0, "remembered as captured");
    let got = GHOST_PROBE_RECEIVED.lock().unwrap().clone();
    assert!(got.contains(&(hud, 0x26, 0)), "{got:x?}");
}

/// Every dust devil build constructs its first dust devil and runs frames with the book closed, open,
/// then hooked and pulled in, without panicking.
#[test]
fn every_dustdevil_build_runs() {
    use scooby_recomp::modules::dustdevil;
    for b in 0..dustdevil::builds::BUILDS.len() {
        let mut e = engine_with_libs();
        let gb0 = GetGameBuffer(&mut e, 0, 0x8f8, 1);
        e.w32(gb0 + 8, 0x2a00_0091);
        let gb1 = GetGameBuffer(&mut e, 1, 0x68, 1);
        e.wf(gb1 + 4, 100.0);
        let c = e.load_module(&dustdevil::MODULE, b);
        let id = dustdevil::maps::GO_CUSTOM_DATA_DUST_DEVIL[b][0];
        let s = e.add_object(id, Some(c));
        for i in 0..240 {
            if i == 60 {
                e.w32(gb0 + 0x14, 1); // book open
            }
            if i == 120 {
                let p = e.malloc(4);
                e.w32(p, 0x2a00_0093);
                e.SendLogicMessageTo(id, 0xd6, 0x65, p, NULL); // hooked by the beam
            }
            if i == 180 {
                let p = e.malloc(4);
                e.wf(p, 0.5); // pull-in blend time (Monster_Base 0x67 reads it)
                e.SendLogicMessageTo(id, 0xd6, 0x67, p, NULL); // pulled in
            }
            e.run_object(s, 1.0 / 60.0);
        }
        e.remove_object(s);
    }
}

/// Ep3 L02 dust devil and the book: roams its path (0x11), flees along it when hooked (0x18, the book
/// told 0xd6/0x66), is pulled in (black hole 5) and captured (kill mask in game buffer 2, 0x26 to the
/// HUD).
#[test]
fn dustdevil_flees_hook_and_is_captured() {
    use scooby_recomp::modules::dustdevil;
    // the hero 20 m away: outside the 10 m chase range
    let mut e = engine_with_libs_on(Box::new(TestHost { away: Some((0x2a00_0091, 20.0)), ..Default::default() }));
    let b = dustdevil::build_index("dustdevilleveltwo").unwrap();
    let book = dustdevil::builds::BUILDS[b].k[dustdevil::builds::ID_2a000041];
    let hud = 0x2a10_0006;
    let pc = e.load_module(&GHOST_PROBE, 0);
    for pid in [book, hud] {
        let h = e.add_object(pid, Some(pc));
        e.run_object(h, 1.0 / 60.0);
    }
    let gb0 = GetGameBuffer(&mut e, 0, 0x8f8, 1);
    e.w32(gb0 + 8, 0x2a00_0091);
    let gb1 = GetGameBuffer(&mut e, 1, 0x68, 1);
    e.wf(gb1 + 4, 100.0);
    let gb2 = GetGameBuffer(&mut e, 2, 0x158, 1);
    let c = e.load_module(&dustdevil::MODULE, b);
    let rec = dustdevil::maps::GO_CUSTOM_DATA_DUST_DEVIL[b];
    let (id, kill_mask) = (rec[0], rec[2]);
    let s = e.add_object(id, Some(c));
    let dt = 1.0 / 60.0;
    let state = |e: &Engine| e.rt.obj(s).inst.state;
    for _ in 0..10 {
        e.run_object(s, dt);
    }
    assert_eq!(state(&e), 0x11, "roaming its path");

    let p = e.malloc(4);
    e.w32(p, book);
    e.SendLogicMessageTo(id, 0xd6, 0x65, p, NULL);
    for _ in 0..2 {
        e.run_object(s, dt);
    }
    assert_eq!(state(&e), 0x18, "hooked by the beam, fleeing along the path");
    assert!(GHOST_PROBE_RECEIVED.lock().unwrap().contains(&(book, 0xd6, 0x66)));

    let p = e.malloc(4);
    e.wf(p, 0.5); // pull-in blend time (Monster_Base 0x67 reads it)
    e.SendLogicMessageTo(id, 0xd6, 0x67, p, NULL);
    e.run_object(s, dt);
    assert_eq!(state(&e), 5, "pulled into the book");

    e.SendLogicMessageTo(id, 0xd6, 0x68, NULL, NULL);
    for _ in 0..3 {
        e.run_object(s, dt);
    }
    assert_ne!(e.r32(gb2 + 4) & kill_mask, 0, "remembered as captured");
    let got = GHOST_PROBE_RECEIVED.lock().unwrap().clone();
    assert!(got.contains(&(hud, 0x26, 0)), "{got:x?}");
}

/// Every poltergeist build constructs its first poltergeist and runs frames with the book closed,
/// open, then hooked and pulled in, without panicking.
#[test]
fn every_poltergeist_build_runs() {
    use scooby_recomp::modules::poltergeist;
    for b in 0..poltergeist::builds::BUILDS.len() {
        let mut e = engine_with_libs();
        let gb0 = GetGameBuffer(&mut e, 0, 0x8f8, 1);
        e.w32(gb0 + 8, 0x2a00_0091);
        let gb1 = GetGameBuffer(&mut e, 1, 0x68, 1);
        e.wf(gb1 + 4, 100.0);
        let c = e.load_module(&poltergeist::MODULE, b);
        let id = poltergeist::maps::GO_CUSTOM_DATA_POLTERGEIST[b][0];
        let s = e.add_object(id, Some(c));
        for i in 0..240 {
            if i == 60 {
                e.w32(gb0 + 0x14, 1); // book open
            }
            if i == 120 {
                let p = e.malloc(4);
                e.w32(p, 0x2a00_0093);
                e.SendLogicMessageTo(id, 0xd6, 0x65, p, NULL); // hooked by the beam
            }
            if i == 180 {
                let p = e.malloc(4);
                e.wf(p, 0.5); // pull-in blend time (Monster_Base 0x67 reads it)
                e.SendLogicMessageTo(id, 0xd6, 0x67, p, NULL); // pulled in
            }
            e.run_object(s, 1.0 / 60.0);
        }
        e.remove_object(s);
    }
}

/// Ep2 L02B (flying, no path) poltergeist and the book: stands (sub-state Standing_Execute in external state 0xf), is
/// scared when hooked (Scared sub-state, the book told 0xd6/0x66), pulled in (black hole 5) and
/// captured (kill mask in game buffer 2, 0x26 to the HUD).
#[test]
fn poltergeist_scared_by_hook_and_is_captured() {
    use scooby_recomp::modules::poltergeist::{self, poltergeist_base::addr};
    let mut e = engine_with_libs_on(Box::new(TestHost { away: Some((0x2a00_0091, 20.0)), ..Default::default() }));
    let b = poltergeist::build_index("fa6a1f63").unwrap();
    let book = poltergeist::builds::BUILDS[b].k[poltergeist::builds::ID_2a00004d];
    let hud = 0x2a10_0006;
    let pc = e.load_module(&GHOST_PROBE, 0);
    for pid in [book, hud] {
        let h = e.add_object(pid, Some(pc));
        e.run_object(h, 1.0 / 60.0);
    }
    let gb0 = GetGameBuffer(&mut e, 0, 0x8f8, 1);
    e.w32(gb0 + 8, 0x2a00_0091);
    let gb1 = GetGameBuffer(&mut e, 1, 0x68, 1);
    e.wf(gb1 + 4, 100.0);
    let gb2 = GetGameBuffer(&mut e, 2, 0x158, 1);
    let c = e.load_module(&poltergeist::MODULE, b);
    let rec = poltergeist::maps::GO_CUSTOM_DATA_POLTERGEIST[b];
    let (id, kill_mask) = (rec[0], rec[2]);
    let s = e.add_object(id, Some(c));
    let dt = 1.0 / 60.0;
    let state = |e: &Engine| e.rt.obj(s).inst.state;
    let sub = |e: &Engine| e.r32(e.rt.obj(s).inst.local + 0x160);
    for _ in 0..10 {
        e.GameLogicUpdate(dt); // pre-logic notification 0x14 drives the poltergeist's range checks
    }
    assert_eq!(state(&e), 0xf, "entry point state");
    assert_eq!(sub(&e), addr::Poltergeist_baseST_Standing_Execute, "standing");

    let p = e.malloc(4);
    e.w32(p, book);
    e.SendLogicMessageTo(id, 0xd6, 0x65, p, NULL);
    for _ in 0..3 {
        e.GameLogicUpdate(dt); // pre-logic notification 0x14 drives the poltergeist's range checks
    }
    assert_eq!(sub(&e), addr::Poltergeist_baseST_Scared_Execute, "hooked by the beam: scared");
    assert!(GHOST_PROBE_RECEIVED.lock().unwrap().contains(&(book, 0xd6, 0x66)));

    let p = e.malloc(4);
    e.wf(p, 0.5);
    e.SendLogicMessageTo(id, 0xd6, 0x67, p, NULL);
    e.GameLogicUpdate(dt); // pre-logic notification 0x14 drives the poltergeist's range checks
    assert_eq!(state(&e), 5, "pulled into the book");

    e.SendLogicMessageTo(id, 0xd6, 0x68, NULL, NULL);
    for _ in 0..3 {
        e.GameLogicUpdate(dt); // pre-logic notification 0x14 drives the poltergeist's range checks
    }
    assert_ne!(e.r32(gb2 + 4) & kill_mask, 0, "remembered as captured");
    let got = GHOST_PROBE_RECEIVED.lock().unwrap().clone();
    assert!(got.contains(&(hud, 0x26, 0)), "{got:x?}");
}

/// Every fire-ghost build constructs its first fire ghost (EP6L05: after its spawn position, msg
/// 0x1b2) and runs frames: searching, the hero's sector entered (0x98), the hero close, water
/// (0xd8), the extinguished anim's end, the relight timer, then the ghost inside captured (0x68).
#[test]
fn every_fireghost_build_runs() {
    use scooby_recomp::modules::fireghost;
    for b in 0..fireghost::builds::BUILDS.len() {
        let mut e = engine_with_libs();
        let gb0 = GetGameBuffer(&mut e, 0, 0x8f8, 1);
        e.w32(gb0 + 8, 0x2a00_0091);
        e.w32(gb0 + 0x24, 1); // sector 1
        let c = e.load_module(&fireghost::MODULE, b);
        let rec = fireghost::maps::GO_CUSTOM_DATA_FIRE_GHOST[b];
        let (id, inner) = (rec[0], rec[4]);
        let pc = e.load_module(&GHOST_PROBE, 0);
        let g = e.add_object(inner, Some(pc));
        e.run_object(g, 1.0 / 60.0);
        let s = e.add_object(id, Some(c));
        let dt = 1.0 / 60.0;
        for i in 0..900 {
            match i {
                10 => {
                    let p = e.malloc(0x10);
                    e.SendLogicMessageTo(id, 0x1b2, 0, p, NULL); // spawn position (EP6L05 type 0x12)
                }
                20 => {
                    e.SendLogicMessageTo(id, 0x98, 0, NULL, NULL);
                }
                200 => {
                    e.SendLogicMessageTo(id, 0xd8, 0, NULL, NULL);
                }
                220 => e.Notify(1, id, 1, 1, false), // anim 1 (extinguished) ended
                800 => {
                    e.SendLogicMessageTo(id, 0x68, 0, NULL, NULL);
                }
                _ => {}
            }
            e.GameLogicUpdate(dt);
        }
        e.remove_object(s);
    }
}

/// EP6L04A fire ghost and water: searches along its path, is put out by water (smoke, the ghost
/// inside swapped in and set to state 0xe), stays out while the hero holds the book open, and
/// relights when the book is closed and the relight timer runs out.
#[test]
fn fireghost_extinguished_by_water_and_relights() {
    use scooby_recomp::modules::fireghost::{self, fireghost_base::addr};
    let mut e = engine_with_libs_on(Box::new(TestHost { away: Some((0x2a00_0091, 50.0)), ..Default::default() }));
    let b = fireghost::build_index("6f63c7c0").unwrap();
    let gb0 = GetGameBuffer(&mut e, 0, 0x8f8, 1);
    e.w32(gb0 + 8, 0x2a00_0091);
    let c = e.load_module(&fireghost::MODULE, b);
    let rec = fireghost::maps::GO_CUSTOM_DATA_FIRE_GHOST[b];
    let (id, inner) = (rec[0], rec[4]);
    let pc = e.load_module(&GHOST_PROBE, 0);
    let g = e.add_object(inner, Some(pc));
    e.run_object(g, 1.0 / 60.0);
    let s = e.add_object(id, Some(c));
    let dt = 1.0 / 60.0;
    let l = |e: &Engine| e.rt.obj(s).inst.local;
    let sub = |e: &Engine| e.r32(l(e) + 0x160);
    for _ in 0..5 {
        e.GameLogicUpdate(dt);
    }
    assert_eq!(e.rt.obj(s).inst.state, 1, "entry point");
    assert_eq!(sub(&e), addr::fireGhostBaseST_SearchingHero_Execute, "searching");
    assert!(GHOST_PROBE_RECEIVED.lock().unwrap().contains(&(inner, 0x1b3, id)), "the ghost inside knows its shell");
    assert_eq!(e.r32(l(&e) + 0x1b8), 1, "burning");

    e.SendLogicMessageTo(id, 0xd8, 0, NULL, NULL); // water
    e.GameLogicUpdate(dt);
    assert_eq!(sub(&e), addr::fireGhostBaseST_Extinguished_Execute, "put out");
    assert_eq!(e.r32(l(&e) + 0x1b8), 0);
    assert_eq!(e.r32(l(&e) + 0x200) as i32, -1, "no fire");
    assert_ne!(e.r32(l(&e) + 0x21c) as i32, -1, "smoke");

    e.Notify(1, id, 1, 1, false); // anim 1 ended: the ghost inside takes our place
    e.w32(gb0 + 0x14, 1); // the hero holds the book open
    for _ in 0..60 {
        e.GameLogicUpdate(dt);
    }
    assert_eq!(e.r32(l(&e) + 0x21c) as i32, -1, "smoke gone after 0.5 s");
    for _ in 0..600 {
        e.GameLogicUpdate(dt);
    }
    assert_eq!(sub(&e), addr::fireGhostBaseST_Extinguished_Execute, "no relight while the book is open");

    e.w32(gb0 + 0x14, 0); // book closed
    for _ in 0..500 {
        e.GameLogicUpdate(dt);
    }
    assert_eq!(sub(&e), addr::fireGhostBaseST_SearchingHero_Execute, "relit");
    assert_eq!(e.r32(l(&e) + 0x1b8), 1, "burning again");
    assert_ne!(e.r32(l(&e) + 0x200) as i32, -1, "fire particles");
}

/// The Ep4 zombie boss fight: with the book open he keeps his distance; hooked (0xd6/0x65) he
/// struggles, breaks free when the book's pull strength (0x125) falls to 200 (phase 1: defensive
/// stomp, wisps dropped, the open book knocked shut), is hooked again and pulled in (0xd6/0x67: the
/// binding latent takes over and kills itself after its anim), then captured (0xd6/0x68).
#[test]
fn zombieboss_struggles_breaks_free_and_is_captured() {
    use scooby_recomp::modules::{zombieboss::{self, addr}, zombieboss_binding};
    // the hero 10 m away: inside the open book's 20 m, outside the 5.5 m attack range (at the
    // origin the boss would gas him: SCARED_FOLLOW_LOOP acts on the 0x20 bit left by FOLLOW_HERO)
    let mut e = engine_with_libs_on(Box::new(TestHost { away: Some((0x2a00_0091, 10.0)), ..Default::default() }));
    let (hero, hud, book, dropper) = (0x2a00_0091, 0x2a10_0006, 0x2a00_000a, 0x2a00_00c3);
    let gb0 = GetGameBuffer(&mut e, 0, 0x8f8, 1);
    e.w32(gb0 + 8, hero);
    let gb2 = GetGameBuffer(&mut e, 2, 0x158, 1);
    let pc = e.load_module(&GHOST_PROBE, 0);
    for id in [hero, hud, book, dropper] {
        let p = e.add_object(id, Some(pc));
        e.run_object(p, 1.0 / 60.0);
    }
    let bc = e.load_module(&zombieboss_binding::MODULE, 0);
    e.rt.latents.insert(0x2a80_00bd, Some(bc));
    let c = e.load_module(&zombieboss::MODULE, 0);
    let id = 0x2a00_0050;
    let s = e.add_object(id, Some(c));
    let dt = 1.0 / 60.0;
    let l = |e: &Engine| e.rt.obj(s).inst.local;
    let sub = |e: &Engine| e.r32(l(e) + 0x178);
    let run = |e: &mut Engine, n: usize| {
        for _ in 0..n {
            e.GameLogicUpdate(dt);
        }
    };

    run(&mut e, 5);
    assert_eq!(e.rt.obj(s).inst.state, 5, "waits for the intro cut-scene");
    let cs = e.malloc(0x10);
    e.w32(cs + 4, 1);
    e.Notify(0x16, id, id, cs, false); // cut-scene start
    e.w32(cs + 4, 0);
    e.Notify(0x16, id, id, cs, false); // cut-scene end: state 0x10 (FIRSTMOVE_SET)
    e.w32(gb0 + 0x14, 1); // the hero holds the book open
    run(&mut e, 10);
    assert_eq!(e.rt.obj(s).inst.state, 0x11, "entry point");
    assert_eq!(sub(&e), addr::ZombieBossST_SCARED_FOLLOW_LOOP, "keeps away from the open book");

    let p = e.malloc(0x10);
    e.w32(p, book);
    e.SendLogicMessageTo(id, 0xd6, 0x65, p, NULL); // hooked
    assert_eq!(sub(&e), addr::ZombieBossST_SCARED_SET);
    assert_eq!(e.r32(l(&e) + 0x1f0), 1, "being pulled");
    run(&mut e, 2);
    assert_eq!(sub(&e), addr::ZombieBossST_SCARED_LOOP);
    e.Notify(1, id, 4, 4, false); // scared anim ended
    run(&mut e, 2);
    assert_eq!(sub(&e), addr::ZombieBossST_SCARED_FOLLOW_LOOP, "struggling");
    assert!(GHOST_PROBE_RECEIVED.lock().unwrap().contains(&(book, 0x125, id)), "asks the book for its pull");

    e.wf(p, 150.0);
    e.SendLogicMessageTo(id, 0x125, 0, p, NULL); // the book's pull strength
    run(&mut e, 2);
    assert_eq!(e.r32(l(&e) + 0x198), 1, "broke free: phase 1");
    assert_eq!(sub(&e), addr::ZombieBossST_DEFENSIVE_ATTACK_LOOP, "stomps");
    let kf = e.malloc(0x10);
    e.w32(kf, 2); // anim 2, key 0
    e.Notify(0xb, id, id, kf, false);
    assert_eq!(e.r32(l(&e) + 0x19c), 1);
    {
        let got = GHOST_PROBE_RECEIVED.lock().unwrap().clone();
        assert!(got.contains(&(dropper, 0xf7, 0)), "wisps dropped {got:x?}");
        assert!(got.contains(&(hero, 0x92, 0)), "the open book knocked shut");
        assert!(got.contains(&(hud, 0x3a, 0)));
    }
    e.Notify(1, id, 2, 2, false); // stomp anim ended
    // DEFENSIVE_ATTACK_LOOP -> FOLLOW_HERO_SET -> FOLLOW_HERO_LOOP (0x4000) -> SCARED_FOLLOW_SET -> LOOP
    run(&mut e, 4);
    assert_eq!(sub(&e), addr::ZombieBossST_SCARED_FOLLOW_LOOP, "the book is still open: keeps away");

    e.SendLogicMessageTo(id, 0xd6, 0x67, p, NULL); // pulled in
    assert_eq!(e.rt.obj(s).inst.state, 5, "pulled in");
    let binding = e.r32(l(&e) + 0x18c);
    assert_ne!(binding, 0, "binding latent launched");
    let bh = e.get_game_object(binding, NULL).expect("binding object");
    run(&mut e, 2);
    assert_eq!(e.rt.obj(bh).inst.state, 1, "binding anim playing");
    e.SendLogicMessageTo(id, 0xd6, 0x68, p, NULL); // captured
    assert_eq!(e.r32(l(&e) + 500), 1, "end movie queued");
    let got = GHOST_PROBE_RECEIVED.lock().unwrap().clone();
    assert!(got.contains(&(hud, 0x2b, 9)) && got.contains(&(hud, 0x1e, 0)), "{got:x?}");
    // Monster_Base ORs the kill mask L+0x40 into game buffer 2 +4, but nothing in zombieboss ever
    // sets L+0x40: the boss is not remembered there (the end movie ends the episode anyway)
    assert_eq!(e.r32(gb2 + 4), 0);
    assert_eq!(e.rt.obj(s).inst.state, 0xb, "captured");
    e.Notify(1, binding, 1, 1, false); // binding anim ended: it kills itself
    run(&mut e, 2);
    e.SendLogicMessageTo(id, 0x4f, 0, NULL, NULL); // load the Ep4 end movie
    assert_ne!(e.r32(gb2 + 0x28) & 0x80000, 0, "movie flag");
}

/// Every mummy build constructs its first mummy and runs frames with the book closed, open, then
/// hooked, pulled in and captured, without panicking.
#[test]
fn every_mummy_build_runs() {
    use scooby_recomp::modules::{mummy, mummy_binding};
    for b in 0..mummy::builds::BUILDS.len() {
        let mut e = engine_with_libs();
        let gb0 = GetGameBuffer(&mut e, 0, 0x8f8, 1);
        e.w32(gb0 + 8, 0x2a00_0091);
        let gb1 = GetGameBuffer(&mut e, 1, 0x68, 1);
        e.wf(gb1 + 4, 100.0);
        let gb2 = GetGameBuffer(&mut e, 2, 0x158, 1);
        let _ = gb2;
        let bc = e.load_module(&mummy_binding::MODULE, 0);
        e.rt.latents.insert(mummy::builds::BUILDS[b].k[mummy::builds::ID_2a80010c], Some(bc));
        let c = e.load_module(&mummy::MODULE, b);
        let id = mummy::maps::GO_CUSTOM_DATA_MUMMY[b][0];
        let book = mummy::builds::BUILDS[b].k[mummy::builds::ID_2a000034];
        let s = e.add_object(id, Some(c));
        for i in 0..300 {
            if i == 60 {
                e.w32(gb0 + 0x14, 1); // book open
            }
            if i == 120 {
                let p = e.malloc(4);
                e.w32(p, book);
                e.SendLogicMessageTo(id, 0xd6, 0x65, p, NULL); // hooked by the beam
            }
            if i == 180 {
                let p = e.malloc(4);
                e.wf(p, 0.5);
                e.SendLogicMessageTo(id, 0xd6, 0x67, p, NULL); // pulled in
            }
            if i == 240 {
                e.SendLogicMessageTo(id, 0xd6, 0x68, NULL, NULL); // captured
            }
            e.GameLogicUpdate(1.0 / 60.0);
        }
        e.remove_object(s);
    }
}

/// Ep2 L09 mummy and the book: hooked (the book told 0xd6/0x66), pulled in (state 5, the bandage
/// latent launched and wrapping it), captured (kill mask in game buffer 2 +4, 0x26 to the HUD).
#[test]
fn mummy_hooked_pulled_in_and_captured() {
    use scooby_recomp::modules::{mummy, mummy_binding};
    let mut e = engine_with_libs_on(Box::new(TestHost { away: Some((0x2a00_0091, 20.0)), ..Default::default() }));
    let b = mummy::build_index("mummylevelnine").unwrap();
    let k = |i: usize| mummy::builds::BUILDS[b].k[i];
    let (hero, hud, book) = (0x2a00_0091, k(mummy::builds::ID_2a100006), k(mummy::builds::ID_2a000034));
    let pc = e.load_module(&GHOST_PROBE, 0);
    for pid in [hero, hud, book] {
        let h = e.add_object(pid, Some(pc));
        e.run_object(h, 1.0 / 60.0);
    }
    let gb0 = GetGameBuffer(&mut e, 0, 0x8f8, 1);
    e.w32(gb0 + 8, hero);
    let gb1 = GetGameBuffer(&mut e, 1, 0x68, 1);
    e.wf(gb1 + 4, 100.0);
    let gb2 = GetGameBuffer(&mut e, 2, 0x158, 1);
    let bc = e.load_module(&mummy_binding::MODULE, 0);
    e.rt.latents.insert(k(mummy::builds::ID_2a80010c), Some(bc));
    let c = e.load_module(&mummy::MODULE, b);
    let rec = mummy::maps::GO_CUSTOM_DATA_MUMMY[b];
    let (id, kill_mask) = (rec[0], rec[3]);
    let s = e.add_object(id, Some(c));
    let state = |e: &Engine| e.rt.obj(s).inst.state;
    let run = |e: &mut Engine, n: usize| {
        for _ in 0..n {
            e.GameLogicUpdate(1.0 / 60.0);
        }
    };
    run(&mut e, 10);

    let p = e.malloc(4);
    e.w32(p, book);
    e.SendLogicMessageTo(id, 0xd6, 0x65, p, NULL);
    run(&mut e, 2);
    assert!(GHOST_PROBE_RECEIVED.lock().unwrap().contains(&(book, 0xd6, 0x66)), "the book is told it hooked us");

    let p = e.malloc(4);
    e.wf(p, 0.5);
    e.SendLogicMessageTo(id, 0xd6, 0x67, p, NULL);
    run(&mut e, 1);
    assert_eq!(state(&e), 5, "pulled into the book");
    let l = e.rt.obj(s).inst.local;
    let binding = e.r32(l + 400);
    assert_ne!(binding, 0, "bandage latent launched");
    let bh = e.get_game_object(binding, NULL).expect("binding object");
    run(&mut e, 2);
    assert_eq!(e.rt.obj(bh).inst.state, 1, "bandages wrapping");

    e.SendLogicMessageTo(id, 0xd6, 0x68, NULL, NULL);
    run(&mut e, 3);
    assert_ne!(e.r32(gb2 + 4) & kill_mask, 0, "remembered as captured");
    let got = GHOST_PROBE_RECEIVED.lock().unwrap().clone();
    assert!(got.contains(&(hud, 0x26, 0)), "{got:x?}");
    e.Notify(1, binding, 1, 1, false); // bandage anim ended: it kills itself
    run(&mut e, 2);
}

/// Ep3 L03 ride mummy: rides 0x2a000009 (0xae), anim 2 (0xa6) whose key frame launches latent
/// 0x2a80000e, anim 3 (0xaa) launching 0x2a80000d, the rotations, then removal (particles freed).
#[test]
fn mummyride_scripted_anims_and_latents() {
    use scooby_recomp::modules::mummyride;
    let mut e = engine_with_libs();
    GetGameBuffer(&mut e, 0, 0x8f8, 1);
    let c = e.load_module(&mummyride::MODULE, 0);
    let id = 0x2a00_0120;
    let s = e.add_object(id, Some(c));
    e.GameLogicUpdate(1.0 / 60.0);
    e.GameLogicUpdate(1.0 / 60.0); // SetNextState takes effect on the next frame
    assert_eq!(e.rt.obj(s).inst.state, 0, "FIRSTMOVE -> BASICMOVE_LOOP");
    for m in [0xae, 0xa6, 0x160, 0x161, 0x188] {
        e.SendLogicMessageTo(id, m, 0, NULL, NULL);
        e.GameLogicUpdate(1.0 / 60.0);
    }
    let kf = e.malloc(12);
    e.w32(kf, 2);
    e.w32(kf + 4, 1);
    e.w32(kf + 8, 0);
    e.Notify(0xb, 0xffff_ffff, id, kf, false); // anim 2 key frame: latent 0x2a80000e
    e.w32(kf + 8, 1);
    e.Notify(0xb, 0xffff_ffff, id, kf, false); // tells it 0x110
    e.SendLogicMessageTo(id, 0xaa, 0, NULL, NULL);
    e.Notify(0xb, 0xffff_ffff, id, kf, false);
    for _ in 0..10 {
        e.GameLogicUpdate(1.0 / 60.0);
    }
    e.remove_object(s);
}

/// Ep6 fire ghost boss: idle until the cut-scene ends, then searches, turns to the hero, throws a
/// fireball (released on key frame 3), is hit by water (0xd8) and then throws four at once.
#[test]
fn fireghostboss_attacks_and_is_hit_by_water() {
    use scooby_recomp::modules::fireghostboss::{self, addr};
    let mut e = engine_with_libs();
    let gb0 = GetGameBuffer(&mut e, 0, 0x8f8, 1);
    let hero = 0x2a00_0091;
    e.w32(gb0 + 8, hero);
    e.add_object(hero, None);
    let c = e.load_module(&fireghostboss::MODULE, 0);
    let id = 0x2a00_00c0;
    let s = e.add_object(id, Some(c));
    let st = |e: &Engine| e.r32(e.rt.obj(s).inst.local);
    for _ in 0..3 {
        e.GameLogicUpdate(1.0 / 60.0);
    }
    assert_eq!(st(&e), addr::fireGhostBossST_BlackHole_Execute);
    let cs = e.malloc(8);
    e.w32(cs + 4, 0); // cut-scene end
    e.Notify(0x16, 0xffff_ffff, id, cs, false);
    e.SendLogicMessageTo(id, 0xc9, 0, NULL, NULL);
    for _ in 0..30 {
        e.GameLogicUpdate(1.0 / 60.0);
    }
    let kf = e.malloc(8);
    e.w32(kf, 3);
    e.w32(kf + 4, 1);
    e.Notify(0xb, 0xffff_ffff, id, kf, false); // release the fireball
    e.SendLogicMessageTo(id, 0xd8, 0x2a00_00ce, NULL, NULL);
    for _ in 0..3 {
        e.GameLogicUpdate(1.0 / 60.0);
    }
    assert_eq!(st(&e), addr::fireGhostBossST_BeingAttack_Execute);
    for _ in 0..60 {
        e.GameLogicUpdate(1.0 / 60.0);
    }
    e.remove_object(s);
}

/// Every scientist build constructs each of its scientists and runs frames: idle, the hero near
/// (spotting / attack), scared (0x1d0 -> asleep, L09 also 0xd8), woken (0x1cf), then hooked by the
/// book, pulled in and captured (state 0xb; the kill mask stays 0, as on the PS2).
#[test]
fn every_scientist_build_runs() {
    use scooby_recomp::modules::scientist::{self, addr, builds::*};
    for b in 0..BUILDS.len() {
        let rec = scientist::maps::GO_CUSTOM_DATA_SCIENTIST[b];
        for r in 0..4 {
            let id = rec[r * 0x16];
            let mut e = engine_with_libs_on(Box::new(TestHost { away: Some((0x2a00_0091, 2.0)), ..Default::default() }));
            let hero = 0x2a00_0091;
            e.add_object(hero, None);
            let gb0 = GetGameBuffer(&mut e, 0, 0x8f8, 1);
            e.w32(gb0 + 8, hero);
            let gb1 = GetGameBuffer(&mut e, 1, 0x68, 1);
            e.wf(gb1 + 4, 100.0);
            let gb2 = GetGameBuffer(&mut e, 2, 0x158, 1);
            let c = e.load_module(&scientist::MODULE, b);
            let s = e.add_object(id, Some(c));
            let st = |e: &Engine| e.r32(e.rt.obj(s).inst.local + 0x160);
            let run = |e: &mut Engine, n: usize| {
                for _ in 0..n {
                    e.GameLogicUpdate(1.0 / 60.0);
                }
            };
            run(&mut e, 120);
            e.SendLogicMessageTo(id, 0x1d0, 0, NULL, NULL);
            run(&mut e, 2);
            assert!(
                [addr::scientist_baseST_Sleep, addr::scientist_baseST_Sleep_In].contains(&st(&e)),
                "{} {id:#x}: asleep ({:#x})",
                BUILDS[b].sha8,
                st(&e)
            );
            e.SendLogicMessageTo(id, 0x1cf, 0, NULL, NULL);
            run(&mut e, 60);
            if BUILDS[b].sha8 == "955aa35f" {
                e.SendLogicMessageTo(id, 0xd8, 0, NULL, NULL);
                run(&mut e, 2);
                assert!(
                    [addr::scientist_baseST_Scared, addr::scientist_baseST_Sleep].contains(&st(&e)),
                    "L09 water scares ({:#x})",
                    st(&e)
                );
            }
            e.w32(gb0 + 0x14, 1); // book open
            run(&mut e, 30);
            let book = e.malloc(4);
            e.w32(book, BUILDS[b].k[ID_2a000043]);
            e.SendLogicMessageTo(id, 0xd6, 0x65, book, NULL);
            run(&mut e, 2);
            let p = e.malloc(4);
            e.wf(p, 0.5);
            e.SendLogicMessageTo(id, 0xd6, 0x67, p, NULL);
            run(&mut e, 2);
            if e.get_game_object(id, NULL).is_some() && e.rt.obj(s).inst.state == 5 {
                e.SendLogicMessageTo(id, 0xd6, 0x68, NULL, NULL);
                run(&mut e, 3);
                assert_eq!(e.rt.obj(s).inst.state, 0xb, "captured");
                // no scientist source writes LM+0x40 (the kill mask): 0 on the PS2 too
                assert_eq!(e.r32(gb2 + 4), 0, "kill mask never set");
            }
            e.remove_object(s);
        }
    }
}

/// Every hero build (Scooby / Shaggy, all levels) constructs and runs: 3 s standing as the leader,
/// then the partner id becomes the leader (switch / follow path), then the hero is removed.
#[test]
fn every_hero_build_runs() {
    use scooby_recomp::modules::hero;
    // level-specific scooby.c code not ported yet (see hero/scooby.rs)
    const SKIP: [&str; 0] = [];
    for b in 0..hero::builds::BUILDS.len() {
        if SKIP.contains(&hero::builds::BUILDS[b].sha8) {
            continue;
        }
        let mut e = engine_with_libs();
        let ct = &hero::ctor_tables::HERO_CTOR[b];
        let gb0 = GetGameBuffer(&mut e, 0, 0x8f8, 1);
        let _gb1 = GetGameBuffer(&mut e, 1, 0x68, 1);
        let me = ct.hero[0];
        e.w32(gb0 + 8, me);
        e.w32(gb0 + 0xc, ct.hero[1]);
        let c = e.load_module(&hero::MODULE, b);
        let h = e.add_object(me, Some(c));
        for i in 0..360 {
            if i == 180 {
                e.w32(gb0 + 8, ct.hero[1]);
            }
            e.GameLogicUpdate(1.0 / 60.0);
        }
        e.remove_object(h);
    }
}

/// Hero movement and animation in every runnable build: standing plays an anim and sits in
/// heroST_Stand_Loop; holding the stick (message 100, bit 2 = heroCB_INPUT_Stick) moves to
/// heroST_Move_Loop with a new (blend) anim; releasing it (bit 1 = heroCB_INPUT_Idle) stands again.
#[test]
fn hero_moves_and_animates() {
    use scooby_recomp::modules::hero;
    use hero::skeleton::addr;
    const SKIP: [&str; 0] = [];
    for b in 0..hero::builds::BUILDS.len() {
        let sha = hero::builds::BUILDS[b].sha8;
        if SKIP.contains(&sha) {
            continue;
        }
        let host = TestHost::default();
        let anims = host.anims.clone();
        let mut e = engine_with_libs_on(Box::new(host));
        let ct = &hero::ctor_tables::HERO_CTOR[b];
        let gb0 = GetGameBuffer(&mut e, 0, 0x8f8, 1);
        let _gb1 = GetGameBuffer(&mut e, 1, 0x68, 1);
        let me = ct.hero[0];
        e.w32(gb0 + 8, me);
        e.w32(gb0 + 0xc, ct.hero[1]);
        let c = e.load_module(&hero::MODULE, b);
        let h = e.add_object(me, Some(c));
        let st = |e: &Engine| e.r32(e.rt.obj(h).inst.local + 0x158);
        let mine = |a: &Arc<Mutex<Vec<(u32, u32)>>>| a.lock().unwrap().len();
        for _ in 0..30 {
            e.GameLogicUpdate(1.0 / 60.0);
        }
        assert_eq!(st(&e), addr::heroST_Stand_Loop, "{sha}: standing");
        let n0 = mine(&anims);
        assert!(n0 > 0, "{sha}: no stand anim played (+0x218 = {:#x})", e.r32(e.rt.obj(h).inst.local + 0x218));
        for _ in 0..30 {
            e.SendMessage(h, 100, 2, NULL);
            e.GameLogicUpdate(1.0 / 60.0);
        }
        assert_eq!(st(&e), addr::heroST_Move_Loop, "{sha}: stick held -> moving");
        let n1 = mine(&anims);
        assert!(n1 > n0, "{sha}: no move anim played");
        for _ in 0..30 {
            e.SendMessage(h, 100, 1, NULL);
            e.GameLogicUpdate(1.0 / 60.0);
        }
        let s = st(&e);
        assert!(
            s == addr::heroST_Stand_Loop || s == addr::heroST_Break_Loop,
            "{sha}: stick released -> stand/break, got {s:#x}"
        );
        assert!(mine(&anims) > n1, "{sha}: no stop anim played");
        e.remove_object(h);
    }
}

/// Hero side of the book in every runnable build: TomeIn plays the open anim and waits for its end,
/// then the holding loop, then TomeOut plays the close anim and returns to standing. The state
/// machine is driven through local+0x154 (next init) and local+0x168 bit 0 (anim ended), the same
/// fields the message handler / heroCB_LASTFRAME write.
#[test]
fn hero_opens_and_closes_book() {
    use scooby_recomp::modules::hero;
    use hero::skeleton::addr;
    const SKIP: [&str; 0] = [];
    for b in 0..hero::builds::BUILDS.len() {
        let sha = hero::builds::BUILDS[b].sha8;
        if SKIP.contains(&sha) {
            continue;
        }
        let host = TestHost::default();
        let anims = host.anims.clone();
        let mut e = engine_with_libs_on(Box::new(host));
        let ct = &hero::ctor_tables::HERO_CTOR[b];
        let gb0 = GetGameBuffer(&mut e, 0, 0x8f8, 1);
        let _gb1 = GetGameBuffer(&mut e, 1, 0x68, 1);
        let me = ct.hero[0];
        e.w32(gb0 + 8, me);
        e.w32(gb0 + 0xc, ct.hero[1]);
        let c = e.load_module(&hero::MODULE, b);
        let h = e.add_object(me, Some(c));
        let n = |a: &Arc<Mutex<Vec<(u32, u32)>>>| a.lock().unwrap().len();
        for _ in 0..10 {
            e.GameLogicUpdate(1.0 / 60.0);
        }
        let l = e.rt.obj(h).inst.local;
        let mut step = |e: &mut Engine, next: Option<u32>, ended: bool| {
            if let Some(s) = next {
                e.w32(l + 0x154, s);
            }
            if ended {
                let v = e.r32(l + 0x168) | 1;
                e.w32(l + 0x168, v);
            }
            e.GameLogicUpdate(1.0 / 60.0);
            e.GameLogicUpdate(1.0 / 60.0);
            e.r32(l + 0x158)
        };
        let a0 = n(&anims);
        assert_eq!(step(&mut e, Some(addr::heroInit_TomeIn), false), addr::heroST_TomeIn_Loop, "{sha}: open");
        // the anim literal of this build (leader/follower slot); some builds have negative ids there
        // (no such anim: PlayAnim returns -1 without playing), checked against the build table
        let leader = e.r32(gb0 + 8) == e.r32(l + 0x17c);
        let kk = &hero::builds::BUILDS[b].k;
        let lit = |slot: usize| kk[if leader { slot } else { slot + 1 }] as i32;
        let played = |e: &Engine, before, slot| {
            if lit(slot) < 1 { e.r32(l + 0x218) == u32::MAX } else { n(&anims) > before }
        };
        assert!(played(&e, a0, hero::builds::heroInit_TomeIn_0_1), "{sha}: no open anim (+0x218 = {:#x})", e.r32(l + 0x218));
        assert_eq!(step(&mut e, None, false), addr::heroST_TomeIn_Loop, "{sha}: waits for the open anim");
        assert_eq!(step(&mut e, None, true), addr::heroST_Tome_Loop, "{sha}: holding");
        let a1 = n(&anims);
        assert_eq!(step(&mut e, Some(addr::heroInit_TomeOut), false), addr::heroST_TomeOut_Loop, "{sha}: close");
        assert!(played(&e, a1, hero::builds::heroInit_TomeOut_0_1), "{sha}: no close anim");
        // heroST_TomeOut_Loop: leader -> stand, follower -> follow-stand
        let want = if leader { addr::heroST_Stand_Loop } else { addr::heroST_FollowStand_Loop };
        assert_eq!(step(&mut e, None, true), want, "{sha}: back to standing (leader {leader})");
        e.remove_object(h);
    }
}

/// HUD meters (panel, every build): the Tome of Doom needle follows the book angle (GB1+4, degrees, only
/// 0..180), the courage needle walks to *GB1 in 0.01 steps of 1.8 degrees, the portrait fades to the
/// courage band's face once, and the tome face fades between its textures. Window handles are each
/// build's own.
#[test]
fn panel_hud_meters_all_builds() {
    use scooby_recomp::modules::panel::{self, builds as pb, image as pi};
    for b in 0..pb::BUILDS.len() {
        let rot = Arc::new(Mutex::new(Vec::new()));
        let fad = Arc::new(Mutex::new(Vec::new()));
        let set = Arc::new(Mutex::new(Vec::new()));
        let mut e = engine_with_libs_on(Box::new(TestHost {
            rotations: rot.clone(),
            faders: fad.clone(),
            set_rotations: set.clone(),
            ..Default::default()
        }));
        let c = e.load_module(&panel::MODULE, b);
        let h = e.add_object(0x2a00_0100, Some(c));
        e.PushLogic(h, 0);
        let data = e.rt.cur_data();
        let sym = |s: usize| data + (pi::SYMS[b].1[s] - 0x10000);
        let k = |i: usize| pb::BUILDS[b].k[i];
        let lb = e.mem.malloc(0x4cc);
        e.mem.memset(lb, 0, 0x4cc);
        e.mem.wf(lb + 8, 1.0);
        let gd = e.mem.malloc(0x18);
        let gb1 = e.mem.malloc(0x68);
        let gb4 = e.mem.malloc(8);
        e.mem.memset(gb1, 0, 0x68);
        e.mem.w32(gd + 4, gb1);
        e.mem.w32(gd + 0x10, gb4);
        e.mem.w32(sym(pi::S_pLocalData_BasePanel), lb);
        e.mem.w32(sym(pi::S_pGlobalData_BasePanel), gd);
        let rad = 0.017453292f32;

        // tome needle
        let win = k(pb::panel_baseFUNC_UpdateTomeMeter_0_1);
        for (ang, want) in [(90.0f32, Some(-90.0 * rad)), (30.0, Some(60.0 * rad)), (200.0, None), (-5.0, None), (180.0, Some(-150.0 * rad))] {
            e.mem.wf(gb1 + 4, ang);
            rot.lock().unwrap().clear();
            panel::panel_baseFUNC_UpdateTomeMeter(&mut e).unwrap();
            let r = rot.lock().unwrap().clone();
            match want {
                Some(w) => {
                    assert_eq!(r.len(), 1, "build {b} angle {ang}");
                    assert_eq!(r[0].0, win);
                    assert!((r[0].1 - w).abs() < 1e-4, "build {b} angle {ang}: {} vs {w}", r[0].1);
                }
                None => assert!(r.is_empty(), "build {b} angle {ang}: {r:?}"),
            }
        }
        assert_eq!(e.mem.rf(lb + 0x88), 180.0);

        // courage needle 1.0 -> 0.5, then a clamp of an out-of-range target
        e.mem.wf(lb + 0x74, 1.0);
        e.mem.wf(gb1, 0.5);
        rot.lock().unwrap().clear();
        for _ in 0..60 {
            panel::panel_baseFUNC_UpdateCoolMeter(&mut e).unwrap();
        }
        let r = rot.lock().unwrap().clone();
        let cw = k(pb::panel_baseFUNC_UpdateCoolMeter_0_1);
        assert!(r.len() >= 49 && r.len() <= 51 && r.iter().all(|x| x.0 == cw), "build {b}: {} steps", r.len());
        let sum: f32 = r.iter().map(|x| x.1).sum();
        assert!((sum + 90.0 * rad).abs() < 0.03, "build {b}: total {sum}");
        assert!((e.mem.rf(lb + 0x74) - 0.5).abs() < 1e-6);
        e.mem.wf(lb + 0x74, 1.0);
        e.mem.wf(gb1, 1.5);
        panel::panel_baseFUNC_UpdateCoolMeter(&mut e).unwrap();
        assert_eq!(e.mem.rf(gb1), 1.0, "build {b}: clamp");

        // portrait: normal -> scared once
        let face = lb + k(pb::panel_baseFUNC_TexFaderControl_0_1);
        e.mem.w32(face, 1);
        e.mem.w32(lb + 0x30, 1);
        e.mem.wf(lb + 0x74, 0.2);
        fad.lock().unwrap().clear();
        panel::panel_baseFUNC_TexFaderControl(&mut e).unwrap();
        panel::panel_baseFUNC_TexFaderControl(&mut e).unwrap();
        let f = fad.lock().unwrap().clone();
        assert_eq!(f, vec![(k(pb::panel_baseFUNC_TexFaderControl_0_5), "Scooby_Shaggy_Normal".into(), "Scooby_Shaggy_Scared".into())], "build {b}");
        assert_eq!(e.mem.r32(lb + k(pb::panel_baseFUNC_TexFaderControl_0_4)), 3);
        assert_eq!(e.mem.r32(lb + 0x30), 0);

        // tome face: normal -> drama masks; a second identical request does nothing new
        fad.lock().unwrap().clear();
        panel::panel_baseFUNC_ToDFaderControl(&mut e, 3).unwrap();
        let normal = e.mem.r32(lb + 0x58);
        e.mem.w32(lb + 0x54, normal);
        panel::panel_baseFUNC_ToDFaderControl(&mut e, 3).unwrap();
        panel::panel_baseFUNC_ToDFaderControl(&mut e, 4).unwrap();
        let f = fad.lock().unwrap().clone();
        let tw = k(pb::panel_baseFUNC_ToDFaderControl_0_1);
        assert_eq!(f.len(), 2, "build {b}: {f:?}");
        assert_eq!(f[1], (tw, "TOD_Normal_Meter".into(), "drama_masks".into()));

        // meter mode change: gauge mode follows *GB4 (0..1 -> 0..180 degrees)
        e.mem.wf(gb4, 0.25);
        rot.lock().unwrap().clear();
        set.lock().unwrap().clear();
        panel::panel_baseFUNC_ToDMeterChangeControl(&mut e, 1).unwrap();
        assert_eq!(set.lock().unwrap().clone(), vec![(k(pb::panel_baseFUNC_ToDMeterChangeControl_0_1), 0.0)]);
        let r = rot.lock().unwrap().clone();
        assert_eq!(r.len(), 1);
        assert!((r[0].1 + 45.0 * rad).abs() < 1e-4, "build {b}: {r:?}");
        e.PopLogic();
    }
}

/// Whole HUD panel object in every build whose in-game path is ported: constructor (windows, input
/// configs, timers), FIRSTMOVE -> IN_GAME_SET -> IN_GAME for 2 s, a tome-meter update message, objective
/// add/complete, pause slide-in (0x29,1) and back (0x29,0), destructor. Records the needle rotations.
#[test]
fn panel_object_runs_in_game() {
    use scooby_recomp::modules::panel::{self, builds as pb, local};
    let mut ran = 0;
    for b in 0..pb::BUILDS.len() {
        let bd = &pb::BUILDS[b];
        let l = &local::LOCAL[b];
        let rot = Arc::new(Mutex::new(Vec::new()));
        let anims = Arc::new(Mutex::new(Vec::new()));
        let mut e = engine_with_libs_on(Box::new(TestHost { rotations: rot.clone(), anims: anims.clone(), ..Default::default() }));
        let gb1 = GetGameBuffer(&mut e, 1, 0x68, 1);
        e.wf(gb1, 1.0); // courage full
        let c = e.load_module(&panel::MODULE, b);
        let h = e.add_object(0x2a00_0100, Some(c));
        for _ in 0..120 {
            e.GameLogicUpdate(1.0 / 60.0);
        }
        assert_eq!(e.rt.obj(h).inst.state, l.next_state, "build {}: panel not in IN_GAME", bd.sha8);
        // level meters: gold nuggets (0x183 = nugget count) and gas (0x17b = +50 degrees, 0x17c refill)
        let before = rot.lock().unwrap().len();
        match bd.sha8 {
            "3ce3c13c" | "a8417f38" => {
                e.SendLogicMessageTo(0x2a00_0100, 0x183, 41, 0, 0);
            }
            "99c250a4" | "af7a3894" | "bcb77619" | "ceb7d472" => {
                let lp = e.mem.malloc(4);
                e.mem.wf(lp, -30.0);
                e.SendLogicMessageTo(0x2a00_0100, 0x17a, 0, lp, 0);
                e.GameLogicUpdate(1.0 / 60.0);
                e.SendLogicMessageTo(0x2a00_0100, 0x17c, 0, 0, 0);
            }
            _ => {}
        }
        if matches!(bd.sha8, "3ce3c13c" | "a8417f38") {
            // SetRotation is not recorded; the nugget meter must at least not panic
        } else if matches!(bd.sha8, "99c250a4" | "af7a3894" | "bcb77619" | "ceb7d472") {
            let r = rot.lock().unwrap();
            assert!(r.len() >= before + 2, "build {}: gas needle did not move", bd.sha8);
            // drained 30 degrees then refilled: -(-30) then -(30) radians
            let a = &r[before..];
            assert!((a[0].1 - 30f32.to_radians()).abs() < 1e-4 && (a[1].1 + 30f32.to_radians()).abs() < 1e-4, "build {}: gas needle {a:?}", bd.sha8);
        }
        e.wf(gb1 + 4, 90.0);
        e.SendLogicMessageTo(0x2a00_0100, 0x1d, 0, 0, 0);
        assert!(!rot.lock().unwrap().is_empty(), "build {}: tome needle did not move", bd.sha8);
        // objectives: add #1 then complete it
        let one = e.mem.malloc(4);
        e.mem.wf(one, 0.0);
        e.SendLogicMessageTo(0x2a00_0100, 0x2a, 1, one, 0);
        e.SendLogicMessageTo(0x2a00_0100, 0x2b, 1, one, 0);
        e.SendLogicMessageTo(0x2a00_0100, 0x29, 1, 0, 0);
        for _ in 0..30 {
            e.GameLogicUpdate(1.0 / 60.0);
        }
        e.SendLogicMessageTo(0x2a00_0100, 0x29, 0, 0, 0);
        for _ in 0..30 {
            e.GameLogicUpdate(1.0 / 60.0);
        }
        assert_eq!(e.rt.obj(h).inst.state, 2, "build {}: not back in game", bd.sha8);
        e.remove_object(h);
        ran += 1;
    }
    eprintln!("panel builds run: {ran}/{}", pb::BUILDS.len());
    assert_eq!(ran, pb::BUILDS.len(), "every panel build must run");
}

/// HUD menus, every build: open the pause menu from IN_GAME, move the stick through every entry,
/// page left/right, confirm on each entry then cancel back, the options stick/confirm/cancel callbacks,
/// close. No panics and the panel stays a live object. Engine-side effects are not asserted.
#[test]
fn panel_menus_all_builds() {
    use scooby_recomp::modules::panel::{self, builds as pb};
    let mut fails = Vec::new();
    for b in 0..pb::BUILDS.len() {
        let sha = pb::BUILDS[b].sha8;
        let r = std::panic::catch_unwind(|| {
            let mut e = engine_with_libs_on(Box::new(TestHost::default()));
            let gb1 = GetGameBuffer(&mut e, 1, 0x68, 1);
            e.wf(gb1, 1.0);
            let c = e.load_module(&panel::MODULE, b);
            let h = e.add_object(0x2a00_0100, Some(c));
            for _ in 0..120 {
                e.GameLogicUpdate(1.0 / 60.0);
            }
            let stick = e.mem.malloc(8);
            let run = |e: &mut Engine, f: fn(&mut Engine, u32) -> scooby_recomp::logic::LResult<u64>, a: u32| {
                e.PushLogic(h, 0);
                f(e, a).unwrap();
                e.PopLogic();
            };
            run(&mut e, panel::panel_baseCB_PAUSE_MENU_OPEN, 0);
            for _ in 0..60 {
                e.GameLogicUpdate(1.0 / 60.0);
            }
            let st = e.rt.obj(h).inst.state;
            assert!(st != 2 && st != 0x22, "{sha}: pause menu did not open (state {st})");
            for (x, y) in [(0.0f32, -0.5f32), (0.0, -0.5), (0.0, -0.5), (0.0, 0.5), (0.5, 0.0), (-0.5, 0.0)] {
                e.mem.wf(stick, x);
                e.mem.wf(stick + 4, y);
                run(&mut e, panel::panel_baseCB_MENU_STICK_UPDOWN, stick);
                run(&mut e, panel::panel_baseCB_MENU_STICK_LEFTRIGHT, stick);
                run(&mut e, panel::panel_baseCB_OPTIONS_STICK_UPDOWN, stick);
                run(&mut e, panel::panel_baseCB_OPTIONS_STICK_LEFTRIGHT, stick);
                e.GameLogicUpdate(1.0 / 60.0);
            }
            run(&mut e, panel::panel_baseCB_OPTIONS_CONFIRM, 0);
            run(&mut e, panel::panel_baseCB_OPTIONS_CANCEL, 0);
            run(&mut e, panel::panel_baseCB_MENU_CONFIRM, 0);
            for _ in 0..30 {
                e.GameLogicUpdate(1.0 / 60.0);
            }
            run(&mut e, panel::panel_baseCB_MENU_CANCEL, 0);
            run(&mut e, panel::panel_baseCB_TRIANGLE, 0);
            run(&mut e, panel::panel_baseCB_PAUSE_MENU_CLOSE, 0);
            for _ in 0..60 {
                e.GameLogicUpdate(1.0 / 60.0);
            }
            e.remove_object(h);
        });
        if let Err(p) = r {
            let m = p.downcast_ref::<String>().cloned().or(p.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
            fails.push(format!("{sha}: {m}"));
        }
    }
    assert!(fails.is_empty(), "{} builds failed:\n{}", fails.len(), fails.join("\n"));
}

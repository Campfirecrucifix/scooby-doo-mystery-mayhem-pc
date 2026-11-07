// P37 R2: the shipped game.exe (feature `ship`) has no console window; dev/test builds keep the console
#![cfg_attr(feature = "ship", windows_subsystem = "windows")]
//! WP-J5: playable windowed build of a level (EP1L01 by default), focused on the book.
//! Feature `game`. The `GamePlugin` owns the engine (`Engine` + `EngineRuntime`) in one
//! resource; the level pack is read on a worker thread (no blocking I/O in systems) and applied
//! when it arrives. The runtime steps `EngineRuntime::Frame` at the PS2 vsync rate (1/60 s raw
//! frame time; EFEngineController::MainLoop 0x001fbef0 then filters it with ComputeDeltaTime:
//! clamp 0.0001..0.05, 10-frame average — done inside the runtime by `e.clock`).
//!
//! Usage: game [LEVEL (default LOADER = disc boot)] [--iso PATH] [--frames N] [--autoplay] [--give-book] [--headless] [--shots DIR]
//!   --frames N    quit after N runtime frames
//!   --autoplay    scripted pad (engine_j4 capture script: open the book next to a ghost, mash)
//!   --give-book   debug: send book msg 0xe5 at frame 10 (implied by --shots)
//!   --headless    no window: render into an offscreen image (needs --shots to be useful)
//!   --route R     EP1L03A bot follows waypoints "x,z[,A];..." (A: X = press X / take a door,
//!                 J = press X while walking (climb), W = wait for a cut-scene); "l03a" = the built-in hand-over route
//!   --aspect A    4:3 | 16:9 | 21:9 | 32:9 | auto (= window). Hor+: the vertical FOV is kept, the
//!                 view widens; the frustum culling widens with it; the HUD stays a centered 4:3
//!   --res WxH     window (or headless target) size
//!   --rt          (feature `rt`) enhanced lighting post-process path, see `rt_camera`
//!   --shots DIR   save book_closed.png / book_open_beam.png / j7_hud_capture.png / ghost_captured.png into DIR
//! Pad: WASD left stick, arrows right stick, Space X, U square, I triangle, O circle,
//!      Q L1, E R1, Z L2, C R2, Enter START, Tab SELECT, numpad 8/2/4/6 d-pad, Esc START (quit = close the window).
//!      Any gamepad maps 1:1 (south X, east circle, north triangle, west square).
#[path = "game_fx/mod.rs"]
mod game_fx;
mod game_input;
mod game_settings;
// WP-P12: EP1 L03A/L03B/L04 routes (WP-P10 file)
#[path = "routes_ep1b.rs"]
mod routes_ep1b;
// WP-P21 R28: EP3 real-play routes (`--route ep3`)
#[path = "routes_ep3b.rs"]
mod routes_ep3b;
use bevy::app::{AppExit, ScheduleRunnerPlugin};
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, RenderTarget, ScalingMode};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::image::Image;
use bevy::light::GlobalAmbientLight;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{TextureFormat, TextureUsages};
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::ExitCondition;
use bevy::winit::WinitPlugin;
use scooby_recomp::engine::Engine;
use scooby_recomp::engine::camera::DTMatrix;
use scooby_recomp::engine::core::apigameobj::GetCurrentStateID;
use scooby_recomp::engine::dt::descriptors::{DT3dObjectDescriptor, EFIDescriptor};
use scooby_recomp::engine::gfx::api::SpritePass;
use scooby_recomp::engine::gfx::bevy_adapter::*;
use scooby_recomp::engine::gfx::hierarchy::Mat4 as RwMat4;
use scooby_recomp::engine::gfx::mgr::EFGraphicsMgr;
use scooby_recomp::engine::gfx::motion_trail::MTStrip;
use scooby_recomp::engine::gfx::render::{EF3dObj_SP, NoTexAnim, OwnerInfo};
use scooby_recomp::engine::gfx::sprite::SpriteDraw;
use scooby_recomp::engine::gfx::{EF3dObjRes, EFStatic3dObj};
use scooby_recomp::engine::input::{PadState, pad};
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::pack::EFRessourcesMgr;
use scooby_recomp::engine::runtime::{EngineRuntime, SCREEN};
use scooby_recomp::engine::audio::bevy_adapter::{update_voice_gains, VoiceHandle, VoiceSide};
use scooby_recomp::engine::audio::spu::{SpuData, SpuVoice};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, TryRecvError};

const BOOK: u32 = 0x2a00_0093;
const GHOST: u32 = 0x2a00_0008;
const SCOOBY: u32 = 0x2a00_0092;
/// PS2 NTSC vsync: MainLoop's raw frame time (timer delta of one field-synced frame)
const STEP: f32 = 1.0 / 60.0;
/// F1: NTSC field period. SetVideoSettings defaults "DisplayMode" to ntsc (SLUS) and
/// EFGraphicsMgr_SP::Flip = RwRasterShowRaster(raster, 0, 1): the flip waits for vsync.
const VSYNC: f32 = 1001.0 / 60000.0;
/// F1: early-run slack so a 60.00 Hz display locks to the 59.94 Hz grid instead of dropping a frame
const VSYNC_SLACK: f32 = 0.002;
/// headless: render-only frames before the runtime starts (pipeline compilation)
const WARMUP: u32 = 400;

/// Muted unless `--sound` is given: no window ever opens with audio by default.
static SOUND: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// SCOOBY_MUTE=1 forces silence even with --sound (QA/agent runs on the user's PC must never be audible).
fn sound_on() -> bool { SOUND.load(std::sync::atomic::Ordering::Relaxed) && !std::env::var("SCOOBY_MUTE").is_ok_and(|v| !v.is_empty() && v != "0") }

#[derive(Clone)]
struct Args {
    level: String,
    iso: String,
    frames: Option<u32>,
    autoplay: bool,
    give_book: bool,
    headless: bool,
    shots: Option<String>,
    route: Option<String>,
    /// WP-G1: None = not given (headless keeps the legacy full-target output), Some(None) = auto
    aspect: Option<Option<f32>>,
    res: Option<(u32, u32)>,
    rt: bool,
}

fn parse_args() -> Args {
    // P24: no level argument = boot like the disc (LOADER: legal/logo FMVs -> INTRO title + main menu)
    let mut a = Args { level: "LOADER".into(), iso: String::new(), frames: None, autoplay: false, give_book: false, headless: false, shots: None, route: None, aspect: None, res: None, rt: false };
    let mut it = scooby_recomp::crashlog::args().into_iter();
    while let Some(s) = it.next() {
        match s.as_str() {
            "--iso" => a.iso = it.next().unwrap_or_default(),
            "--frames" => a.frames = it.next().and_then(|n| n.parse().ok()),
            "--autoplay" => a.autoplay = true,
            "--give-book" => a.give_book = true,
            "--headless" => a.headless = true,
            "--shots" => a.shots = it.next(),
            "--route" => a.route = it.next(),
            "--rt" => a.rt = true,
            "--sound" => SOUND.store(true, std::sync::atomic::Ordering::Relaxed),
            "--aspect" => {
                let v = it.next().unwrap_or_default();
                a.aspect = Some(v.split_once(':').and_then(|(w, h)| Some(w.parse::<f32>().ok()? / h.parse::<f32>().ok()?)).or_else(|| v.parse().ok()));
            }
            "--res" => a.res = it.next().and_then(|v| v.split_once(['x', 'X']).and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))),
            l => {
                let l = l.to_ascii_uppercase();
                a.level = if l.contains('/') || !l.starts_with("EP") { l } else { l.find('L').map(|i| format!("{}/{}", &l[..i], &l[i..])).unwrap_or(l) };
            }
        }
    }
    if a.iso.is_empty() {
        // P34 R4: the disc chosen in the launcher (settings.toml [game] iso)
        let st = std::fs::read_to_string(scooby_recomp::engine::settings::paths().0).ok().and_then(|t| scooby_recomp::engine::settings::Settings::parse(&t).ok());
        if let Some(p) = st.and_then(|s| s.game.iso).filter(|p| p.is_file()) {
            a.iso = p.to_string_lossy().into_owned();
        }
    }
    if a.iso.is_empty() {
        // P24: also next to game.exe and its parents (a double-clicked exe runs with cwd = its folder)
        let mut cand: Vec<std::path::PathBuf> = ["../Scooby Doo Mystery Mayhem.ISO", "Scooby Doo Mystery Mayhem.ISO", "../../Scooby Doo Mystery Mayhem.ISO"].iter().map(Into::into).collect();
        if let Some(d) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
            cand.extend(d.ancestors().take(5).map(|d| d.join("Scooby Doo Mystery Mayhem.ISO")));
        }
        a.iso = cand.iter().find(|p| p.exists()).unwrap_or(&cand[0]).to_string_lossy().into_owned();
    }
    a
}

// ---------------------------------------------------------------------------------------------
// level loading (worker thread)

/// Render-side data of the level, parsed next to the runtime pack on the worker thread.
struct RenderData {
    res: Vec<EF3dObjRes>,
    stat: Option<EFStatic3dObj>,
    /// game objects present in the world at load (not latent templates)
    placed: HashSet<u32>,
    latent: HashSet<u32>,
}

struct Loaded {
    pack: EFRessourcesMgr,
    render: RenderData,
}

fn read_level(a: &Args) -> Result<Loaded, String> {
    // P39 R3: the gallery packs are LEVEL/GAMEART/ARTnn/GEN/ARTnn.P2R (the episode levels join the path: EP1L01)
    let pack = match a.level.split_once('/') {
        Some((g, art)) if g.eq_ignore_ascii_case("GAMEART") => art.to_string(),
        _ => a.level.replace('/', ""),
    };
    read_level_pack(&a.iso, &a.level, &format!("{pack}.P2R"))
}

/// EFRessourcesMgr level load of `dir`/`pack` (start-up and EFEngineController level switch)
fn read_level_pack(iso: &str, dir: &str, pack: &str) -> Result<Loaded, String> {
    let fs = IsoFs::open(iso).map_err(|e| format!("ISO {iso}: {e:?}"))?;
    let pack = EngineRuntime::ReadLevel(&fs, dir, pack)?;
    let (mut res, mut stat) = (Vec::new(), None);
    for o in &pack.objects {
        let Some(r) = o.as_raw() else { continue };
        if r.class_id == 0x5b {
            let mut x = EF3dObjRes::default();
            if x.LoadPropertiesBinary(&r.data).is_ok() {
                res.push(x);
            }
        } else if r.class_id == 0x18 && stat.is_none() {
            let mut s = EFStatic3dObj::default();
            if s.LoadPropertiesBinary(&r.data).is_ok() {
                stat = Some(s);
            }
        }
    }
    let has3d = |g: &scooby_recomp::engine::level::EFGameObject| g.glo.object_descriptors.objects.iter().any(|d| d.as_any().downcast_ref::<EFIDescriptor<DT3dObjectDescriptor>>().is_some());
    let placed = pack.game_objects.iter().filter(|g| !g.IsLatent() && has3d(g)).map(|g| g.uid).collect();
    let latent = pack.game_objects.iter().filter(|g| g.IsLatent()).map(|g| g.uid).collect();
    Ok(Loaded { pack, render: RenderData { res, stat, placed, latent } })
}

#[derive(Resource)]
struct PendingLoad(Mutex<Receiver<Result<Loaded, String>>>);

// ---------------------------------------------------------------------------------------------
// the game resource

/// The whole engine: logic + runtime managers + the render-side EF3dObj instances.
#[derive(Resource)]
struct Game {
    e: Engine,
    rt: EngineRuntime,
    loaded: bool,
    render: Option<RenderData>,
    /// EF3dObj_SP per game object (created on first sight: launched latents appear later)
    objs: HashMap<u32, EF3dObj_SP>,
    /// J8: EF3dObjs of EFHud3DWindows (owner uid, res uid) = GameObject+0x174[n]; draw-list owner
    /// of entry k = HUD_OWNER + k
    hud_objs: Vec<((u32, u32), EF3dObj_SP)>,
    /// P28 B8: last logic step whose deferred TexFaderPlay requests were applied
    fader_step: u32,
    /// P26 B11: EF3dObj_SP::SetBlendingColor state per (game object, EF3dObj res uid), from
    /// APIgraphics::SetGameObjBlendColor (shadow monster alpha 0 = invisible, HUD panel greying)
    blend_col: HashMap<(u32, u32), [f32; 4]>,
    /// P32 R1: FrameDistortion owners the normal pass clips out (clip word bit 4, e.g. the poltergeist):
    /// in the C only EFFBEffect_FrameDistortion_SP::Update draws them, so their draws go to a render
    /// layer no camera shows and only feed draw_distortions
    distort_only: HashSet<u32>,
    /// EFGraphicsMgr the game objects render into (EF3dObj_SP::Render -> DrawCmd list)
    gmgr: EFGraphicsMgr,
    /// objects that had a logic instance at load (gone from the logic map = killed)
    had_logic: HashSet<u32>,
    pad: PadState,
    /// P41 R5: buttons seen since the last logic tick + the step count of that window (host::latch_taps)
    tap: (u16, u32),
    /// P41 R6: (gameplay, mouse camera on) from read_pad for cursor_visibility (host::cursor_state)
    play_mouse: (bool, bool),
    acc: f32,
    /// F1: raw frame time handed to EngineRuntime::Frame (EFTimer::Update: whole vsync periods)
    raw_dt: f32,
    /// F1: a runtime frame ran since the last render_frame (the original renders once per MainLoop pass)
    fresh: bool,
    /// F1: GAME_PROF=1 per-second timing log (updates, logic frames, logic/render ms avg/max)
    prof: Prof,
    steps: u32,
    /// render-only frames left before the runtime runs (headless warm-up / screenshot hold)
    hold: u32,
    autoplay: bool,
    give_book: bool,
    /// J9: the level's book (the object running tomedoom), found at load
    book: u32,
    /// J9: stand-ins for an unported level camera module (EP1L03A l03_camera), see `CamStandIn`
    cam_standin: Option<CamStandIn>,
    max_frames: Option<u32>,
    shots: Option<String>,
    shot_done: HashSet<&'static str>,
    pending_shot: Option<(&'static str, u32)>,
    beam_since: Option<u32>,
    /// path_step ray scratch (0x80 bytes) in the current level's `Mem`; 0 = not allocated yet.
    /// R10: was a process-wide static -> after a level switch it pointed into the NEW Mem (the
    /// address was L01's) and the BFS ray writes overwrote module data (EP1L02 savepanel table).
    path_scr: u32,
    gblog: Vec<u32>,
    route_nm: (usize, usize),
    /// WP-P18: frame the open book started scanning without a target (TomeDoom sub 0x54), 0 = not scanning
    route_scan: u32,
    /// WP-P20: g.steps of the level's first runtime frame (u32::MAX until it ran): the route bot runs on
    /// level-relative frames, so a chained (--route ep1) level gets the same inputs as a direct boot
    lvl_f0: u32,
    locwatch: Vec<u32>,
    /// EP1L01 autoplay: frame after the last cut-scene frame (the script runs relative to it)
    ap_base: u32,
    /// K5: GAME_BOOKTGTS state (target index, placed at frame, FINALSUCKING seen)
    k5: (usize, Option<u32>, bool),
    /// P32 R6: frames the book spent in FINALSUCKING (GAME_FINALDROP)
    finaldrop: u32,
    /// P36: keyed by Voice::serial (one entry per key-on)
    voices: HashMap<u32, [Entity; 2]>,
    streams: StreamLoader,
    audio: bool,
    /// --route waypoints (x, z, action) for the EP1L03A bot
    route: Vec<(f32, f32, char)>,
    /// --route source + GAME_ROUTE_LEVEL (the level the route is for; default: the start level)
    route_src: Option<String>,
    route_level: String,
    /// P41 R3: the level being played ("EP1/L01", "LOADER", ...): gameplay vs front end
    level: String,
    focus_pause: scooby_recomp::engine::input::host::FocusPause,
    /// WP-P20 R12: `--route ep1` in STATUS (EP1 end screen): press cross every 300 level frames (the real
    /// menu input that moves on: STATUS -> SPLASH -> LOADER EP02E/EP02D -> EP2\L01A)
    menu_cross: bool,
    /// route bot state: (waypoint index, last leader pos, best distance, frame of progress, cut-scene seen)
    route_st: (usize, [f32; 3], f32, u32, bool),
    iso: String,
    /// APIgame::LoadLevel request taken from EFEngineController flags & 8 (switch next update)
    level_req: Option<scooby_recomp::engine::host::game::LevelRequest>,
    /// GAME_LOADLEVEL debug hook latch (fire once)
    loadlevel_fired: bool,
    /// game buffer contents carried over a level switch (written into the new level's buffers)
    buf_carry: Vec<(u32, Vec<u8>)>,
    /// WP-P3: FMV being played (blocking MpegPlay in the original: the runtime does not step)
    movie: Option<Movie>,
}

/// WP-P3: a playing PSS movie + its display (fullscreen quad on its own camera) and audio entities.
struct Movie {
    play: scooby_recomp::engine::video::MpegPlay,
    player: scooby_recomp::engine::video::EFVideoPlayer_SP,
    image: Option<Handle<Image>>,
    ents: Vec<Entity>,
    audio_spawned: bool,
    /// P36: wall clock started with the movie's audio (A/V sync master for MpegPlay::UpdateClock)
    clock: Option<std::time::Instant>,
    /// P36 R6: the soundtrack as (left, right) WAV sources + length (s) + gain, kept so it can be
    /// restarted at the picture's position after an output-device reconnect
    snd: Option<(bevy_audio::AudioSource, bevy_audio::AudioSource, f64, f32)>,
    /// reconnect generation the soundtrack is playing on + its player entities
    snd_gen: Option<u32>,
    snd_ents: Vec<Entity>,
}

impl Movie {
    fn new(mut play: scooby_recomp::engine::video::MpegPlay, player: scooby_recomp::engine::video::EFVideoPlayer_SP, blocking: bool) -> Self {
        play.blocking = blocking;
        Movie { play, player, image: None, ents: Vec::new(), audio_spawned: false, clock: None, snd: None, snd_gen: None, snd_ents: Vec::new() }
    }
}

/// file reader for the movie decoder thread (opens the ISO there: no I/O on the frame thread)
fn movie_reader(iso: String) -> impl FnOnce(&str) -> Option<Vec<u8>> + Send + 'static {
    move |n| IsoFs::open(&iso).ok()?.read(&scooby_recomp::engine::video::iso_path(n)).ok()
}

pub struct GamePlugin {
    args: Args,
}

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        let a = self.args.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let la = a.clone();
        std::thread::spawn(move || {
            let _ = tx.send(read_level(&la));
        });
        let (e, rt) = EngineRuntime::new_with_engine();
    // P20 R121 ANIMSND: anim callback sound table on (C behaviour; e.g. EP2/L01A catapult key throw)
    rt.particles.lock().unwrap().sub.anim_sound = anim_sound_on(a.route.as_deref());
    rt.particles.lock().unwrap().sub.cam_anim = std::env::var("GAME_CAMANIM").as_deref() != Ok("0");
    rt.particles.lock().unwrap().em_child = em_child_on(a.route.as_deref());
        app.insert_resource(PendingLoad(Mutex::new(rx)))
            .insert_resource(Game {
                e,
                rt,
                loaded: false,
                render: None,
                objs: HashMap::new(),
                hud_objs: Vec::new(),
                fader_step: 0,
                blend_col: HashMap::new(),
                distort_only: HashSet::new(),
                gmgr: EFGraphicsMgr::new(SCREEN.0, SCREEN.1),
                had_logic: HashSet::new(),
                pad: idle_pad(),
                tap: (0, 0),
                play_mouse: (false, false),
                acc: 0.0,
                raw_dt: STEP,
                fresh: true,
                prof: Prof::default(),
                steps: 0,
                hold: if a.headless { WARMUP } else { 0 },
                autoplay: a.autoplay,
                give_book: a.give_book || (a.shots.is_some() && a.level == "EP1/L01"),
                book: BOOK,
                cam_standin: None,
                max_frames: a.frames,
                shots: a.shots.clone(),
                shot_done: HashSet::new(),
                pending_shot: None,
                beam_since: None,
                path_scr: 0,
                gblog: Vec::new(),
                route_nm: (usize::MAX, 0),
                route_scan: 0,
                locwatch: Vec::new(),
                ap_base: 0,
                k5: (0, None, false),
                finaldrop: 0,
                voices: HashMap::new(),
                streams: StreamLoader::new(a.iso.clone()),
                audio: !a.headless || headless_audio(),
                route: parse_route(level_route(a.route.as_deref(), &a.level.replace(char::from(0x5c), "/"))),
                route_src: a.route.clone(),
                menu_cross: a.route.is_some() && a.level.eq_ignore_ascii_case("STATUS"),
                level: a.level.replace(char::from(0x5c), "/"),
                focus_pause: Default::default(),
                route_level: std::env::var("GAME_ROUTE_LEVEL").unwrap_or_else(|_| a.level.replace(char::from(0x5c), "/")),
                route_st: Default::default(),
                iso: a.iso.clone(),
                level_req: None,
                loadlevel_fired: false,
                buf_carry: carry_load(),
                lvl_f0: u32::MAX,
                // main.cpp (not demo mode): ENGINE.INI BootUpPSSName played before the first level
                movie: (a.level == "LOADER").then(|| {
                    let ini = scooby_recomp::engine::host::game::IniFile::default();
                    let pal = ini.GetIniString("DisplayMode").is_some_and(|v| v.eq_ignore_ascii_case("TRUE") || v == "1");
                    let play = scooby_recomp::engine::video::BootMovie(ini.GetIniString("BootUpPSSName"), pal, movie_reader(a.iso.clone()));
                    Movie::new(play, Default::default(), a.headless)
                }),
            })
            .insert_resource(ViewAspect { aspect: a.aspect.flatten(), active: !a.headless || a.aspect.is_some() || a.res.is_some(), res: a.res, rt: a.rt })
            .insert_resource(game_input::load_controls())
            .init_resource::<game_input::ActivePad>()
            .add_systems(Startup, setup_cameras)
            .add_systems(PreUpdate, game_input::keyscript.after(bevy::input::InputSystems).run_if(|| game_input::keyscript_on()))
            .add_systems(Update, (switch_level, poll_load, read_pad, toggle_fullscreen, cursor_visibility, step_runtime, pad_rumble, movie_present, render_frame, apply_aspect, play_audio, shots_and_exit).chain());
        // P37 R4: GAME_PROF main-world time per update (First..Last); the rest of an update gap is the
        // render world (extract/prepare/queue/submit) + present
        app.add_systems(First, || {
            if STAGES_ON.load(std::sync::atomic::Ordering::Relaxed) {
                *MAIN_T0.lock().unwrap() = Some(std::time::Instant::now());
            }
        })
        .add_systems(Last, || {
            let mut t = MAIN_T0.lock().unwrap().take();
            stage(&mut t, "main_world");
        });
        // P37 R10: GAME_PROF render-world split: one marker between consecutive RenderSystems sets,
        // each records the time since the previous marker (ExtractCommands starts the clock)
        {
            use bevy::render::{Render, RenderApp, RenderSystems as R};
            static RW_T0: Mutex<Option<std::time::Instant>> = Mutex::new(None);
            fn mark(name: &'static str) -> impl FnMut() {
                move || {
                    if STAGES_ON.load(std::sync::atomic::Ordering::Relaxed) {
                        let mut t = *RW_T0.lock().unwrap();
                        stage(&mut t, name);
                        *RW_T0.lock().unwrap() = t;
                    }
                }
            }
            if let Some(ra) = app.get_sub_app_mut(RenderApp) {
                ra.add_systems(Render, (|| *RW_T0.lock().unwrap() = Some(std::time::Instant::now())).after(R::ExtractCommands).before(R::PrepareAssets))
                    .add_systems(Render, mark("rw:prepare_assets").after(R::PrepareMeshes).after(R::PrepareAssets).before(R::CreateViews))
                    .add_systems(Render, mark("rw:views_specialize").after(R::Specialize).before(R::PrepareViews))
                    .add_systems(Render, mark("rw:queue").after(R::Queue).before(R::PhaseSort))
                    .add_systems(Render, mark("rw:sort_prepare").after(R::Prepare).before(R::Render))
                    .add_systems(Render, mark("rw:render").after(R::Render).before(R::Cleanup))
                    .add_systems(Render, mark("rw:cleanup").after(R::PostCleanup));
            }
        }
    }
}

fn idle_pad() -> PadState {
    PadState { connected: true, mode: 7, rx: 128, ry: 128, lx: 128, ly: 128, ..Default::default() }
}

#[derive(Component)]
struct GameCam;
#[derive(Component)]
struct OverlayCam;
/// WP-G1: screen-space cameras kept at the PS2 4:3 inside the (Hor+) game view
#[derive(Component)]
struct HudCam;
/// WP-G1: output aspect policy. `aspect` None = follow the target (auto).
/// WP-G1: overlay (EFSpriteMgr) width in PS2 screen units (640 = 4:3)
#[derive(Resource)]
struct OverlayWidth(f32);
#[derive(Resource)]
struct ViewAspect {
    aspect: Option<f32>,
    /// viewports applied (windowed, or --aspect/--res given)
    active: bool,
    res: Option<(u32, u32)>,
    rt: bool,
}

/// largest `aspect` rect centered in `size` (x, y, w, h)
fn fit_rect(size: UVec2, x0: u32, y0: u32, aspect: f32) -> (UVec2, UVec2) {
    let (w, h) = (size.x as f32, size.y as f32);
    let (fw, fh) = if w / h > aspect { (h * aspect, h) } else { (w, w / aspect) };
    let (fw, fh) = (fw.round().max(1.0) as u32, fh.round().max(1.0) as u32);
    (UVec2::new(x0 + (size.x - fw.min(size.x)) / 2, y0 + (size.y - fh.min(size.y)) / 2), UVec2::new(fw.min(size.x), fh.min(size.y)))
}

/// WP-G1: Hor+ viewports. Game view = the requested aspect fitted in the target (auto = whole
/// target); the HUD / sprite / movie cameras = a centered 4:3 inside it (the PS2 screen is shown
/// at 4:3). The engine frustum is widened by the horizontal growth so side objects are not culled.
fn apply_aspect(world: &mut World) {
    let Some(aspect) = world.get_resource::<ViewAspect>().filter(|v| v.active).map(|v| v.aspect) else { return };
    let size = match world.get_resource::<Target>() {
        Some(t) => world.resource::<Assets<Image>>().get(&t.0).map(|i| i.size()),
        None => world.query::<&Window>().iter(world).next().map(|w| w.physical_size()),
    };
    let Some(size) = size.filter(|s| s.x > 0 && s.y > 0) else { return };
    let (gp, gs) = match aspect {
        Some(a) => fit_rect(size, 0, 0, a),
        None => (UVec2::ZERO, size),
    };
    let (hp, hs) = fit_rect(gs, gp.x, gp.y, 4.0 / 3.0);
    let set = |c: &mut Camera, p: UVec2, s: UVec2| {
        if c.viewport.as_ref().map(|v| (v.physical_position, v.physical_size)) != Some((p, s)) {
            c.viewport = Some(bevy::camera::Viewport { physical_position: p, physical_size: s, ..default() });
        }
    };
    for mut c in world.query_filtered::<&mut Camera, With<GameCam>>().iter_mut(world) {
        set(&mut c, gp, gs);
    }
    for mut c in world.query_filtered::<&mut Camera, With<HudCam>>().iter_mut(world) {
        set(&mut c, hp, hs);
    }
    // EFSpriteMgr overlay: the whole game rect, PS2 units unscaled (640 units = the 4:3 width) so
    // the sprites stay 4:3-correct around the centre; full-screen-wide sprites are widened in draw_sprites
    let view_aspect = gs.x as f32 / gs.y as f32;
    let ow = SCREEN.0 as f32 * view_aspect * 0.75;
    world.insert_resource(OverlayWidth(ow));
    for (mut c, mut p) in world.query_filtered::<(&mut Camera, &mut Projection), With<OverlayCam>>().iter_mut(world) {
        set(&mut c, gp, gs);
        if let Projection::Orthographic(o) = &mut *p {
            if !matches!(o.scaling_mode, ScalingMode::Fixed { width, .. } if width == ow) {
                o.scaling_mode = ScalingMode::Fixed { width: ow, height: SCREEN.1 as f32 };
            }
        }
    }
    // horizontal tan of the Hor+ view vs the camera view window x
    if let Some(mut g) = world.get_resource_mut::<Game>() {
        let mut changed = false;
        for c in g.rt.gfx.cameras.cameras.iter_mut().flatten() {
            let [vx, vy] = c.rw.view_window;
            if vx > 0.0 && vy > 0.0 {
                let s = (vy * view_aspect / vx).max(1.0);
                if (s - f32::from_bits(scooby_recomp::engine::camera::FRUSTUM_HSCALE.load(std::sync::atomic::Ordering::Relaxed))).abs() > 1e-4 {
                    scooby_recomp::engine::camera::set_frustum_hscale(s);
                    changed = true;
                }
            }
        }
        if changed {
            g.rt.gfx.cameras.cameras.iter_mut().flatten().for_each(|c| c.frustum_dirty = 1);
        }
    }
}

/// WP-G1 `--rt`. Bevy Solari is not usable on this renderer (see recomp/GFX_G1.md): it needs
/// meshes with exactly position/normal/uv0/tangent (no vertex colour) and lit deferred
/// StandardMaterials, while the PS2 port draws prelit vertex-coloured unlit/CPU-lit materials.
/// The closest working option: HDR + bloom + FXAA + contrast-adaptive sharpening on the game camera.
#[cfg(feature = "rt")]
fn rt_camera(c: &mut EntityCommands) {
    c.insert((
        bevy::camera::Hdr,
        bevy::post_process::bloom::Bloom { intensity: 0.12, ..bevy::post_process::bloom::Bloom::NATURAL },
        bevy::anti_alias::fxaa::Fxaa::default(),
        bevy::anti_alias::contrast_adaptive_sharpening::ContrastAdaptiveSharpening { enabled: true, sharpening_strength: 0.4, ..default() },
    ));
}
#[derive(Resource)]
struct Target(Handle<Image>);

fn setup_cameras(mut commands: Commands, mut images: ResMut<Assets<Image>>, game: Res<Game>, view: Res<ViewAspect>) {
    let (tw, th) = view.res.unwrap_or_else(|| match view.aspect {
        Some(a) => (((896.0 * a).round() as u32), 896),
        None => (1280, 896),
    });
    let target = (!game.audio).then(|| {
        let mut img = Image::new_target_texture(tw, th, TextureFormat::Rgba8UnormSrgb, None);
        img.texture_descriptor.usage |= TextureUsages::COPY_SRC;
        images.add(img)
    });
    let mut c = commands.spawn((Camera3d::default(), Tonemapping::None, GameCam, Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y)));
    c.insert(Projection::Perspective(PerspectiveProjection { fov: 60f32.to_radians(), near: 0.1, far: 2000.0, ..default() }));
    if let Some(h) = &target {
        c.insert(RenderTarget::Image(h.clone().into()));
    }
    #[cfg(feature = "rt")]
    if view.rt {
        rt_camera(&mut c);
    }
    // P22: GAME_RTGI=1 (feature `solari`) raytraced GI on the game camera
    #[cfg(feature = "solari")]
    let rtgi = scooby_recomp::engine::gfx::rtgi::requested();
    #[cfg(feature = "solari")]
    if rtgi {
        c.insert(scooby_recomp::engine::gfx::rtgi::camera());
    }
    // EFSpriteMgr_BC screen-space batches (640x448 PS2 screen) on a second, orthographic pass
    let mut o = commands.spawn((
        Camera3d::default(),
        Camera { order: 1, clear_color: ClearColorConfig::None, ..default() },
        Tonemapping::None,
        OverlayCam,
        RenderLayers::layer(1),
        Projection::Orthographic(OrthographicProjection { scaling_mode: ScalingMode::Fixed { width: SCREEN.0 as f32, height: SCREEN.1 as f32 }, near: 0.0, far: 100.0, ..OrthographicProjection::default_3d() }),
        Transform::from_xyz(0.0, 0.0, 50.0),
    ));
    if let Some(h) = &target {
        o.insert(RenderTarget::Image(h.clone().into()));
    }
    // cameras sharing a target must share the HDR main texture
    if view.rt && cfg!(feature = "rt") {
        o.insert(bevy::camera::Hdr);
    }
    #[cfg(feature = "solari")]
    if rtgi {
        o.insert(scooby_recomp::engine::gfx::rtgi::companion());
    }
    if let Some(h) = &target {
        commands.insert_resource(Target(h.clone()));
    }
    // J7: the HUD camera of EFHudMgr (view window 1.0 x 0.75 -> x [-0.5,0.5], y [-0.4375,0.4375],
    // parallel) for the EFHud3DWindow clumps: camera at the origin looking down +z, screen-left = +x
    // (EFHud3DWindow_SP::SpecificRender: x = -(cx/w - 0.5), y = -(cy/h - 0.5) * 0.875, z = priority)
    let mut hc = commands.spawn((
        Camera3d::default(),
        Camera { order: 2, clear_color: ClearColorConfig::None, ..default() },
        Tonemapping::None,
        HudCam,
        RenderLayers::layer(2),
        Projection::Orthographic(OrthographicProjection { scaling_mode: ScalingMode::Fixed { width: 1.0, height: 0.875 }, near: -1000.0, far: 1000.0, ..OrthographicProjection::default_3d() }),
        Transform::IDENTITY.looking_to(Vec3::Z, Vec3::Y),
    ));
    if let Some(h) = &target {
        hc.insert(RenderTarget::Image(h.clone().into()));
    }
    if view.rt && cfg!(feature = "rt") {
        hc.insert(bevy::camera::Hdr);
    }
    #[cfg(feature = "solari")]
    if rtgi {
        hc.insert(scooby_recomp::engine::gfx::rtgi::companion());
    }
    // P26 R45: the display-space decode ends the frame on its own camera (order 3, draws nothing,
    // no viewport = the WHOLE target). It used to sit on the HUD camera, whose viewport is the
    // centred 4:3 rect, so in Hor+ the side strips kept raw display values (pale). In 4:3 the HUD
    // viewport is the whole target -> same pixels as before. The movie camera (order 10) stays after it.
    if scooby_recomp::engine::gfx::bevy_adapter::display_space() {
        let mut dc = commands.spawn((
            Camera3d::default(),
            Camera { order: 3, clear_color: ClearColorConfig::None, ..default() },
            Tonemapping::None,
            RenderLayers::layer(30),
            game_fx::DisplayDecode::default(),
        ));
        if let Some(h) = &target {
            dc.insert(RenderTarget::Image(h.clone().into()));
        }
        if view.rt && cfg!(feature = "rt") {
            dc.insert(bevy::camera::Hdr);
        }
        #[cfg(feature = "solari")]
        if rtgi {
            dc.insert(scooby_recomp::engine::gfx::rtgi::companion());
        }
    }
    // P26 R28: PS2 destination-alpha "stencil" (game_fx::StencilRead): the EnableStencilWrite objects
    // (owner logic flag 0x8000) are drawn again by this camera (same view as GameCam, layer
    // STENCIL_LAYER) into an alpha mask cleared to 0 every frame (EFGraphicsMgr_SP::ClearStencil)
    let mut mask = Image::new_target_texture(tw, th, TextureFormat::Rgba8Unorm, None);
    mask.texture_descriptor.usage |= TextureUsages::TEXTURE_BINDING;
    let mask = images.add(mask);
    commands.spawn((
        Camera3d::default(),
        Camera { order: -1, clear_color: ClearColorConfig::Custom(Color::NONE), ..default() },
        Tonemapping::None,
        Msaa::Off,
        StencilCam,
        RenderLayers::layer(STENCIL_LAYER),
        Projection::Perspective(PerspectiveProjection { fov: 60f32.to_radians(), near: 0.1, far: 2000.0, ..default() }),
        RenderTarget::Image(mask.clone().into()),
    ));
    commands.insert_resource(StencilMask(mask));
}

/// P33 R3: this frame draws occluded sprites (their sample squares need the MaskOccluder twins)
#[derive(Resource, Default)]
struct OccSpritesActive(bool);
/// P26 R28: render layer of the stencil-mask camera
const STENCIL_LAYER: usize = 29;
#[derive(Component)]
struct StencilCam;
/// P26 R28: the destination-alpha mask image (alpha >= 1.0 = frame alpha bit 7 set)
#[derive(Resource)]
struct StencilMask(Handle<Image>);
/// P26 R29: occluder twin of an opaque layer-0 entity on the stencil-mask camera (child, same mesh)
#[derive(Component)]
struct MaskTwin;
#[derive(Component)]
struct HasMaskTwin(Entity);
#[derive(Resource, Default)]
struct MaskOccluderMat(HashMap<AssetId<StandardMaterial>, Handle<game_fx::MaskOccluder>>);

/// P26 R29: the GS writes the destination alpha of an EnableStencilWrite object only where its pixels
/// pass the Z test, so world geometry in front of it keeps alpha 0 there. Every opaque entity of the
/// game camera (layer 0, not itself a stencil writer) gets a child twin on STENCIL_LAYER drawn with
/// game_fx::MaskOccluder (depth only, writes alpha 0); twins follow the parent's transform/visibility
/// and die with it. Stencil writers and non-opaque materials hide their twin; all twins hide while
/// the frame has no stencil writer.
fn stencil_occluders(world: &mut World) {
    if !stencil_on() {
        return;
    }
    if !world.contains_resource::<MaskOccluderMat>() {
        world.init_resource::<MaskOccluderMat>();
    }
    let stencil = RenderLayers::layer(STENCIL_LAYER);
    let rows: Vec<(Entity, Handle<Mesh>, AssetId<StandardMaterial>, bool, Option<Entity>)> = world
        .query_filtered::<(Entity, &Mesh3d, &MeshMaterial3d<StandardMaterial>, Option<&RenderLayers>, Option<&HasMaskTwin>), Without<MaskTwin>>()
        .iter(world)
        .filter(|(_, _, _, l, _)| l.is_none_or(|l| l.intersects(&RenderLayers::layer(0))))
        .map(|(e, m, mat, l, t)| (e, m.0.clone(), mat.0.id(), l.is_some_and(|l| l.intersects(&stencil)), t.map(|t| t.0)))
        .collect();
    // no stencil writer this frame: the mask stays clear, skip the occluder pass (twins hidden)
    // P33 R3: the occluded-sprite sample squares (game_fx::OccSquare) need the occluder world too
    let any_writer = rows.iter().any(|r| r.3) || world.get_resource::<OccSpritesActive>().is_some_and(|o| o.0);
    for (e, mesh, mat, is_writer, twin) in rows {
        let opaque = world.resource::<Assets<StandardMaterial>>().get(mat).is_some_and(|m| matches!(m.alpha_mode, AlphaMode::Opaque | AlphaMode::Mask(_)));
        let show = opaque && !is_writer && any_writer;
        match twin {
            Some(t) if world.get_entity(t).is_ok() => {
                let vis = if show { Visibility::Inherited } else { Visibility::Hidden };
                let same = world.get::<Mesh3d>(t).is_some_and(|m| m.0 == mesh);
                let mut tw = world.entity_mut(t);
                tw.insert(vis);
                if !same {
                    tw.insert(Mesh3d(mesh));
                }
            }
            _ if show => {
                let occ = match world.resource::<MaskOccluderMat>().0.get(&mat) {
                    Some(h) => h.clone(),
                    None => {
                        let Some(base) = world.resource::<Assets<StandardMaterial>>().get(mat).cloned() else { continue };
                        let h = world.resource_mut::<Assets<game_fx::MaskOccluder>>().add(game_fx::MaskOccluder { base, extension: game_fx::MaskOccluderExt {} });
                        world.resource_mut::<MaskOccluderMat>().0.insert(mat, h.clone());
                        h
                    }
                };
                let t = world.spawn((MaskTwin, Mesh3d(mesh), MeshMaterial3d(occ), Transform::IDENTITY, Visibility::Inherited, stencil.clone(), ChildOf(e))).id();
                world.entity_mut(e).insert(HasMaskTwin(t));
            }
            _ => {}
        }
    }
}

/// P40 R3 (B#9): particle batches with GS ALPHA 0xa1 (type flag 0x80) draw with game_fx::GsSubtractMaterial
/// (Cd - Cs, ReverseSubtract). ApplyParticles re-inserts the StandardMaterial every frame (as for
/// stencil_particles), so the swap runs every frame; a slot whose batch is no longer 0xa1 drops it.
#[derive(Resource, Default)]
struct SubParticleMats(HashMap<AssetId<StandardMaterial>, Handle<game_fx::GsSubtractMaterial>>);
fn sub_particles(world: &mut World, ps: &[scooby_recomp::engine::particle::PLDrawBatch], bs: &[scooby_recomp::engine::particle::PLDrawBatch]) {
    if !scooby_recomp::engine::gfx::bevy_adapter::sub_blend_hw() {
        return;
    }
    if !world.contains_resource::<SubParticleMats>() {
        world.init_resource::<SubParticleMats>();
    }
    let rows: Vec<(Entity, bool, Option<Handle<StandardMaterial>>)> = world
        .query::<(Entity, &scooby_recomp::engine::gfx::bevy_adapter::ParticleBatchEntity, Option<&MeshMaterial3d<StandardMaterial>>)>()
        .iter(world)
        .map(|(e, id, m)| {
            let b = if id.billboard { bs.get(id.index) } else { ps.get(id.index) };
            (e, b.is_some_and(|b| b.blend & 0xff == 0xa1), m.map(|m| m.0.clone()))
        })
        .collect();
    for (e, sub, sm) in rows {
        match (sub, sm) {
            (true, Some(sm)) => {
                let h = match world.resource::<SubParticleMats>().0.get(&sm.id()) {
                    Some(h) => h.clone(),
                    None => {
                        let Some(base) = world.resource::<Assets<StandardMaterial>>().get(&sm).cloned() else { continue };
                        let h = world.resource_mut::<Assets<game_fx::GsSubtractMaterial>>().add(game_fx::GsSubtractMaterial { base, extension: game_fx::GsSubtract {} });
                        world.resource_mut::<SubParticleMats>().0.insert(sm.id(), h.clone());
                        h
                    }
                };
                world.entity_mut(e).remove::<MeshMaterial3d<StandardMaterial>>().insert(MeshMaterial3d(h));
            }
            (false, Some(_)) => {
                world.entity_mut(e).remove::<MeshMaterial3d<game_fx::GsSubtractMaterial>>();
            }
            _ => {}
        }
    }
}

/// P26 R29: PLEmitter::BeginRender brackets particle types with flag 0x100 with EnableStencilRead(1/0)
/// (PLEmitter.cpp:70/84): those batches get the stencil-read material like the sprite groups.
#[derive(Resource, Default)]
struct StencilParticleMats(HashMap<AssetId<StandardMaterial>, Handle<game_fx::StencilReadMaterial>>);
fn stencil_particles(world: &mut World, ps: &[scooby_recomp::engine::particle::PLDrawBatch], bs: &[scooby_recomp::engine::particle::PLDrawBatch]) {
    if !world.contains_resource::<StencilParticleMats>() {
        world.init_resource::<StencilParticleMats>();
    }
    let mask = world.get_resource::<StencilMask>().map(|m| m.0.clone());
    let rows: Vec<(Entity, bool, Option<Handle<StandardMaterial>>)> = world
        .query::<(Entity, &scooby_recomp::engine::gfx::bevy_adapter::ParticleBatchEntity, Option<&MeshMaterial3d<StandardMaterial>>)>()
        .iter(world)
        .map(|(e, id, m)| {
            let b = if id.billboard { bs.get(id.index) } else { ps.get(id.index) };
            (e, b.is_some_and(|b| b.stencil_read) && stencil_on(), m.map(|m| m.0.clone()))
        })
        .collect();
    for (e, read, sm) in rows {
        match (read, sm, &mask) {
            (true, Some(sm), Some(mask)) => {
                let h = match world.resource::<StencilParticleMats>().0.get(&sm.id()) {
                    Some(h) => h.clone(),
                    None => {
                        let Some(base) = world.resource::<Assets<StandardMaterial>>().get(&sm).cloned() else { continue };
                        let h = world.resource_mut::<Assets<game_fx::StencilReadMaterial>>().add(game_fx::StencilReadMaterial { base, extension: game_fx::StencilRead { mask: mask.clone() } });
                        world.resource_mut::<StencilParticleMats>().0.insert(sm.id(), h.clone());
                        h
                    }
                };
                world.entity_mut(e).remove::<MeshMaterial3d<StandardMaterial>>().insert(MeshMaterial3d(h));
            }
            (false, Some(_), _) => {
                world.entity_mut(e).remove::<MeshMaterial3d<game_fx::StencilReadMaterial>>();
            }
            _ => {}
        }
    }
}

/// P26 R31: EFTextureAnimNode::ApplyText/RestoreText (RpMaterialSetTexture on the material Connect
/// found). deviation: the material is found by texture name at draw time (every material of the clump
/// using the node's original texture, not only the first one per atomic/LOD); same result for the
/// shipped data (one animated material per clump).
struct TexAnimSwap {
    /// (original texture, current frame texture)
    list: Vec<(String, String)>,
    /// swapped (geometry, material, original name)
    saved: Vec<(usize, usize, String)>,
}
impl scooby_recomp::engine::gfx::render::TexAnimSeam for TexAnimSwap {
    fn ApplyText(&mut self, clump: &mut scooby_recomp::engine::rw::clump::RpClump, _lod: u32) {
        if self.list.is_empty() {
            return;
        }
        for (gi, g) in clump.geometries.iter_mut().enumerate() {
            for (mi, m) in g.materials.iter_mut().enumerate() {
                let Some(t) = m.texture.as_mut() else { continue };
                if let Some((_, cur)) = self.list.iter().find(|(o, _)| o.eq_ignore_ascii_case(&t.name)) {
                    self.saved.push((gi, mi, std::mem::replace(&mut t.name, cur.clone())));
                }
            }
        }
    }
    fn RestoreText(&mut self, clump: &mut scooby_recomp::engine::rw::clump::RpClump, _lod: u32) {
        for (gi, mi, n) in self.saved.drain(..) {
            if let Some(t) = clump.geometries[gi].materials[mi].texture.as_mut() {
                t.name = n;
            }
        }
    }
}

/// `GAME_STENCIL=0`: old behaviour (stencil-read sprites drawn everywhere; comparison only)
fn stencil_on() -> bool {
    !std::env::var("GAME_STENCIL").is_ok_and(|v| v == "0")
}

/// P26 R28: the mask camera follows GameCam (view, projection, target size)
fn sync_stencil_cam(world: &mut World) {
    let Some((t, p, size)) = world
        .query_filtered::<(&Transform, &Projection, &Camera), With<GameCam>>()
        .iter(world)
        .next()
        .map(|(t, p, c)| (*t, p.clone(), c.physical_viewport_size()))
    else {
        return;
    };
    if let Some((mut st, mut sp)) = world.query_filtered::<(&mut Transform, &mut Projection), With<StencilCam>>().iter_mut(world).next() {
        *st = t;
        *sp = p;
    }
    let (Some(size), Some(h)) = (size, world.get_resource::<StencilMask>().map(|m| m.0.clone())) else { return };
    let mut images = world.resource_mut::<Assets<Image>>();
    if images.get(&h).is_some_and(|i| i.size() != size) {
        if let Some(mut i) = images.get_mut(&h) {
            i.resize(bevy::render::render_resource::Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 });
        }
    }
}

/// EFEngineController::Run level switch (flags & 8): the level loop ends, the level is torn down
/// (EFEngineController::UnloadLevel: every manager reset, the logic + resources freed) and the
/// requested level pack is loaded like at start-up. The game buffers (GameBuffer save state) persist.
fn switch_level(world: &mut World) {
    let Some(r) = world.resource_mut::<Game>().level_req.take() else { return };
    let dir = r.level.replace(char::from(0x5c), "/").trim_matches('/').to_string();
    let dir = if dir.contains('/') || dir.len() < 6 || !dir.starts_with("EP") { dir } else { format!("{}/{}", &dir[..3], &dir[3..]) };
    let pack = if r.pack.is_empty() { format!("{}.P2R", dir.replace('/', "")) } else { r.pack.clone() };
    eprintln!("game: level switch -> {dir} ({pack})");
    // EFEngineController::LoadLevel: flags & 0x10 -> EFVideoPlayer::Play() before the pack load
    // (the pack is read in parallel here; nothing observable runs until the movie returned)
    if let Some((v, track)) = &r.video {
        let g = world.resource::<Game>();
        let vol = g.rt.particles.lock().unwrap().sub.audio.video_volume;
        let player = scooby_recomp::engine::video::EFVideoPlayer_SP { name: v.clone(), audio_track: *track };
        let mut ini_skip = 0;
        g.rt.particles.lock().unwrap().game.ini.GetIniInteger("VideoSkipButton", &mut ini_skip);
        // P36 R10: EFVideoPlayer_SP::Play passes `_gpoGraphicsMgr[0x44] == 1` (EFGraphicsMgr init: strcmp
        // DisplayMode "PAL" -> 1): the 50-picture skip minimum on a PAL ENGINE.INI (was always NTSC 60)
        let pal = g.rt.particles.lock().unwrap().game.ini.GetIniString("DisplayMode").is_some_and(|v| v == "PAL");
        let play = player.Play(ini_skip, vol, pal, movie_reader(g.iso.clone()));
        eprintln!("game: EFVideoPlayer::Play {v}.PSS (audio track {track}, volume {vol})");
        let blocking = !g.audio;
        world.resource_mut::<Game>().movie = Some(Movie::new(play, player, blocking));
    }
    // teardown of the render side: static world, draw-list / particle / HUD meshes, voices
    let ents: Vec<Entity> = world.query_filtered::<Entity, Or<(With<StaticWorldEntity>, With<Mesh3d>, With<VoiceHandle>)>>().iter(world).collect();
    for e in ents {
        if let Ok(e) = world.get_entity_mut(e) {
            e.despawn();
        }
    }
    world.insert_resource(DrawListState::default());
    world.insert_resource(ParticleDrawState::default());
    world.insert_resource(GfxCache::default());
    let (e, rt) = EngineRuntime::new_with_engine();
    // P20 R121 ANIMSND: anim callback sound table on (C behaviour; e.g. EP2/L01A catapult key throw)
    rt.particles.lock().unwrap().sub.anim_sound = anim_sound_on(world.resource::<Game>().route_src.as_deref());
    rt.particles.lock().unwrap().sub.cam_anim = std::env::var("GAME_CAMANIM").as_deref() != Ok("0");
    rt.particles.lock().unwrap().em_child = em_child_on(world.resource::<Game>().route_src.as_deref());
    let mut g = world.resource_mut::<Game>();
    // EFGameMemoryMgr (EFGameLogicMgr +0x20c) survives EFEngineController::LoadLevel (only
    // KillAllGameBuffers when +0x2cc engine data is pending): carry every persistent buffer
    // (EFGameMemoryMgr::Terminate in EFEngineController::Reset deletes the others; WP-P19)
    let carry: Vec<(u32, Vec<u8>)> = scooby_recomp::engine::host::gameobj::CarriedGameBuffers(&mut g.e);
    // debug (WP-P9): GAME_CARRY_DUMP=prefix writes the carried buffers to <prefix><level>.bin
    // ((id u32, len u32, bytes) LE records, R10_CARRY format); GAME_CARRY=file boots with them
    if let Ok(pre) = std::env::var("GAME_CARRY_DUMP") {
        let mut d = Vec::new();
        for (id, b) in &carry {
            d.extend(id.to_le_bytes());
            d.extend((b.len() as u32).to_le_bytes());
            d.extend(b);
        }
        let path = format!("{pre}{}.bin", dir.replace('/', ""));
        let _ = std::fs::write(&path, d);
        eprintln!("game: carried buffers dumped to {path}");
    }
    g.buf_carry = carry;
    g.e = e;
    g.rt = rt;
    g.loaded = false;
    g.render = None;
    g.objs.clear();
    g.blend_col.clear();
    g.hud_objs.clear();
    g.had_logic.clear();
    g.voices.clear();
    g.cam_standin = None;
    g.book = BOOK;
    g.give_book = false;
    g.beam_since = None;
    g.path_scr = 0;
    g.route = if let Some(r @ ("ep1" | "ep3")) = g.route_src.as_deref() { parse_route(level_route(Some(r), &dir)) } else if dir == g.route_level { parse_route(g.route_src.as_deref()) } else { Vec::new() };
    g.menu_cross = g.route_src.is_some() && dir.eq_ignore_ascii_case("STATUS");
    g.level = dir.clone();
    g.route_st = Default::default();
    g.route_scan = 0;
    g.route_nm = (usize::MAX, 0);
    let iso = g.iso.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(read_level_pack(&iso, &dir, &pack));
    });
    world.insert_resource(PendingLoad(Mutex::new(rx)));
}

fn poll_load(world: &mut World) {
    let Some(p) = world.get_resource::<PendingLoad>() else { return };
    let r = p.0.lock().unwrap().try_recv();
    let l = match r {
        Ok(Ok(l)) => l,
        Ok(Err(m)) => {
            // P37 R1: also into the crash log (a double-clicked game.exe loses stderr: the exit looked silent)
            // P39 R5: and a message box for the player (main, after the app loop): the disc image removed /
            // unreadable mid-game (USB drive pulled) says so; any other failure says the level data is bad
            let g = world.resource::<Game>();
            let iso = std::path::PathBuf::from(&g.iso);
            let player = if !iso.exists() || m.contains("Io(") {
                scooby_recomp::engine::iso::disc_read_message(&iso, &m)
            } else {
                format!("The level {} couldn't be loaded from the game disc image; it may be damaged. Check the image in the launcher (Select game disc) and start the game again.\n\n({})", g.level, m.chars().take(300).collect::<String>())
            };
            scooby_recomp::crashlog::fatal("LEVEL LOAD FAILED", &player, &m);
            world.write_message(AppExit::error());
            world.remove_resource::<PendingLoad>();
            return;
        }
        Err(TryRecvError::Empty) => return,
        Err(TryRecvError::Disconnected) => {
            // P37 R1: the loader thread panicked (its PANIC record is in the crash log): exit with an error
            // instead of waiting forever on a black screen
            scooby_recomp::crashlog::fatal("LEVEL LOAD FAILED", "The level couldn't be loaded (an internal error), so the game had to close. The details are in crash.log next to game.log.", "the level loader thread stopped (see the PANIC record above)");
            world.write_message(AppExit::error());
            world.remove_resource::<PendingLoad>();
            return;
        }
    };
    world.remove_resource::<PendingLoad>();
    world.resource_scope(|world, mut g: Mut<Game>| {
        let Game { e, rt, buf_carry, .. } = &mut *g;
        // EFRessourcesMgr streams the pack's EF_CUTSCENE object into the EFCutScenePlayer singleton
        let cs = scooby_recomp::engine::script::EFCutScenePlayer::FromLevelObjects(&l.pack.objects).map_err(|m| eprintln!("game: cut-scene load failed: {m:?}")).ok();
        // WP-P1: the carried buffers exist before the new level's logic constructors run
        // (GetGameBuffer returns the existing buffer), as in the C++ where they never died
        for (id, b) in buf_carry.iter() {
            let p = scooby_recomp::engine::host::gameobj::GetGameBuffer(e, *id, b.len() as u32, 1);
            for (i, v) in b.iter().enumerate() {
                e.mem.w8(p + i as u32, *v);
            }
        }
        rt.LoadLevel(e, l.pack);
        rt.particles.lock().unwrap().cutscene = cs;
        // P22 R26 GAME_LOGICCLIP: atomic clip spheres per EF3dObj resource
        if scooby_recomp::engine::host::gameobj::logic_clip_enabled() {
            let mut p = rt.particles.lock().unwrap();
            for x in &l.render.res {
                if let Some(c) = &x.clump {
                    p.s1.clip_spheres.insert(x.res.uid, scooby_recomp::engine::gfx::render::clip_spheres(x.res.uid, c));
                }
            }
        }
        if let Some(s) = &l.render.stat {
            spawn_static_world(world, s, &rt.gfx.tex_dict);
            world.insert_resource(RwLights::from_static(&s.lights));
        }
        g.buf_carry.clear();
        g.had_logic = g.e.rt.objects.iter().flatten().map(|o| o.id).collect();
        g.render = Some(l.render);
        g.loaded = true;
        g.lvl_f0 = u32::MAX;
        if let Some(b) = g.e.rt.objects.iter().flatten().find(|o| o.inst.code.is_some_and(|c| g.e.rt.codes[c].def.name == "tomedoom")) {
            g.book = b.id;
        }
        if g.e.rt.map.find(L03A_VELMA).is_some() && g.book == 0x2a00_00ee {
            let cam = g.rt.report.unported.iter().any(|u| u.name == "l03_camera.op2");
            if cam {
                eprintln!("game: l03_camera unported -> J9 stand-in camera/leader (see CamStandIn)");
            }
            g.cam_standin = Some(CamStandIn { cam_unported: cam, ..Default::default() });
        }
        eprintln!("game: level loaded, {} logic objects", g.had_logic.len());
        // debug: GAME_LISTCODE=name lists the objects running a logic code (uid, position)
        list_code(&mut g);
        let un: Vec<String> = g.rt.report.unported.iter().map(|u| format!("{}:{}", u.name, u.sha8)).collect();
        eprintln!("game: unported logic codes: {}", un.join(" "));
        if std::env::var_os("GAME_DEBUG").is_some() {
            for o in g.e.rt.objects.iter().flatten() {
                let m = o.inst.code.map(|c| g.e.rt.codes[c].def.name).unwrap_or("-");
                eprintln!("game: obj {:#x} {m}", o.id);
            }
        }
    });
}

// ---------------------------------------------------------------------------------------------
// input

fn stick(v: f32) -> u8 {
    (128.0 + v.clamp(-1.0, 1.0) * 127.0).round() as u8
}

// WP-I1: bindings / deadzone / gamepads in game_input.rs (controls.toml)
fn read_pad(keys: Res<ButtonInput<KeyCode>>, mut host: game_input::HostPad, mut game: ResMut<Game>, mut prompts: Local<(u32, bool)>) {
    if game.autoplay {
        return;
    }
    // P41 R3 prototype: [game] pc_key_prompts (re-read twice a second: the PC SETTINGS row is live)
    if prompts.0 % 30 == 0 {
        prompts.1 = game_settings::get().game.pc_key_prompts;
    }
    prompts.0 = prompts.0.wrapping_add(1);
    // P41 R3: front end / pause menu / PC SETTINGS = menu (arrows = d-pad); gameplay = a level
    // running with none of those, no movie, no cut scene (pause_on_focus_loss presses START)
    let (paused, cut) = {
        let p = game.rt.particles.lock().unwrap();
        (p.world.paused, p.cutscene.as_ref().is_some_and(|c| c.flags & 1 != 0))
    };
    let page = game_settings::PAGE.lock().unwrap().open;
    let episode = scooby_recomp::engine::input::host::episode_level(&game.level);
    let menu = !episode || paused || page;
    let gameplay = game.loaded && game.movie.is_none() && !menu && !cut;
    game.pad = host.read(menu);
    game.play_mouse = (gameplay, host.mouse_camera());
    {
        // GAME_KEY_PROMPTS=1: test hook (headless screenshots have no keyboard input)
        static FORCE: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| std::env::var("GAME_KEY_PROMPTS").is_ok_and(|v| v == "1"));
        let want = if *FORCE { Some(host.prompt_keys_any()) } else if prompts.1 { host.prompt_keys() } else { None };
        let mut k = scooby_recomp::engine::hud::KEY_PROMPTS.lock().unwrap();
        if *k != want {
            *k = want;
        }
    }
    let step = game.steps;
    // GAME_FOCUSLOSS_AT=N: test hook (headless has no window focus): a focus loss at the first gameplay step >= N, setting on
    static FAKE: std::sync::LazyLock<Option<u32>> = std::sync::LazyLock::new(|| std::env::var("GAME_FOCUSLOSS_AT").ok()?.parse().ok());
    static FIRED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    let fake = FAKE.is_some_and(|n| step >= n) && !FIRED.load(std::sync::atomic::Ordering::Relaxed);
    let lost = host.lost() || fake;
    let on = fake || (lost && game_settings::get().game.pause_on_focus_loss);
    if game.focus_pause.update(on, lost, gameplay, step) {
        if fake && !FIRED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            eprintln!("game: GAME_FOCUSLOSS_AT: focus loss at step {step} -> START");
        }
        game.pad.buttons |= pad::START;
    }
    // P24: Esc = START (pause menu / skip a movie), like PC ports; it used to quit at once and lose
    // progress. Quit = close the window (or the pause menu Quit -> main menu).
    if keys.pressed(KeyCode::Escape) {
        game.pad.buttons |= pad::START;
    }
    // P41 R5: a tap between two 30 Hz logic ticks reaches the next tick (the window resets per tick)
    // P41 R6: no latching while the logic doesn't tick (loading / hold): no START queued into a level
    let g = &mut *game;
    let ticking = g.loaded && g.hold == 0;
    if scooby_recomp::engine::input::host::tap_window(&mut g.tap, g.steps, ticking) {
        scooby_recomp::engine::input::host::latch_taps(&mut g.tap.0, &mut g.pad);
    }
}

/// P24: F11 toggles borderless fullscreen (PC convenience, no game-side effect; not Alt+Enter: Enter = START).
fn toggle_fullscreen(keys: Res<ButtonInput<KeyCode>>, mut windows: Query<&mut Window>) {
    if !keys.just_pressed(KeyCode::F11) {
        return;
    }
    use bevy::window::{MonitorSelection, WindowMode};
    for mut w in &mut windows {
        w.mode = if w.mode == WindowMode::Windowed { WindowMode::BorderlessFullscreen(MonitorSelection::Current) } else { WindowMode::Windowed };
    }
}

/// N2: scePadSetActDirect replacement. EFPlayerDevice_SP::DeviceSpecificUpdate writes the port-0
/// motor values (`motors`: small motor on/off, big motor 0..255) from the vibration envelopes;
/// the options vibration toggle (APIinput EnableVibration) already gates them in SetVibration, so
/// the host only mirrors the motors: big -> strong (low-frequency) motor, small -> weak motor.
/// Bevy rumble requests add up, so each change is Stop + Add (held until the next change).
/// P41: only the pad that drives port 0 rumbles (a second connected pad stays still), nothing
/// while the window is unfocused, and a pad that takes over (or is re-plugged) gets the current motors.
fn pad_rumble(game: Res<Game>, active: Res<game_input::ActivePad>, pads: Query<Entity, With<Gamepad>>, mut last: Local<(Option<Entity>, (f32, f32))>, mut rq: MessageWriter<bevy::input::gamepad::GamepadRumbleRequest>) {
    use bevy::input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest};
    let m = game.rt.particles.lock().unwrap().devices.devices.iter().find(|d| d.port == 0).map_or((false, 0), |d| d.motors);
    let target = active.pad.filter(|e| pads.contains(*e));
    let want = (target, scooby_recomp::engine::input::host::rumble_intensity(m, active.focused));
    if want == *last {
        return;
    }
    if let Some(old) = last.0.filter(|e| pads.contains(*e)) {
        rq.write(GamepadRumbleRequest::Stop { gamepad: old });
    }
    if let (Some(gamepad), (strong, weak)) = want {
        if Some(gamepad) != last.0 {
            rq.write(GamepadRumbleRequest::Stop { gamepad });
        }
        if strong > 0.0 || weak > 0.0 {
            let intensity = GamepadRumbleIntensity { strong_motor: strong, weak_motor: weak };
            rq.write(GamepadRumbleRequest::Add { gamepad, intensity, duration: std::time::Duration::from_secs(3600) });
        }
    }
    *last = want;
}

/// P41: the mouse cursor is hidden in (borderless / exclusive) fullscreen, like the PC ports;
/// windowed keeps the system cursor (the game has no mouse pointer of its own).
fn cursor_visibility(game: Res<Game>, mut windows: Query<(&Window, &mut bevy::window::CursorOptions)>) {
    use bevy::window::CursorGrabMode;
    for (w, mut c) in &mut windows {
        let (play, mouse) = game.play_mouse;
        // P41 R6: windowed + mouse camera: hidden + confined in play, visible + free in menus / unfocused
        let (v, grab) = scooby_recomp::engine::input::host::cursor_state(w.mode == bevy::window::WindowMode::Windowed, play, mouse, w.focused);
        if c.visible != v {
            c.visible = v;
        }
        let m = if grab { CursorGrabMode::Confined } else { CursorGrabMode::None };
        if c.grab_mode != m {
            c.grab_mode = m;
        }
    }
}

/// --autoplay: the engine_j4 capture script (test set-up steps marked).
fn autoplay_pad(g: &mut Game, f: u32) -> PadState {
    let g_book = g.book;
    // the EP1L01 intro cut-scene (inv_tutorial -> l01_camera msg 0x1de) holds the heroes:
    // the capture script starts after it
    if g.rt.particles.lock().unwrap().cutscene.as_ref().is_some_and(|c| c.flags & 1 != 0) {
        g.ap_base = f.saturating_sub(90);
    }
    let r = f - g.ap_base;
    let Game { e, rt, .. } = g;
    if f == 10 {
        // test set-up: a full book (GB1+4 = 180, the Wisp_baseState_Picked_Execute cap)
        let gb1 = e.rt.game_buffers[&1].0;
        e.mem.w32(gb1 + 4, 180f32.to_bits());
    }
    if r == 100 {
        // test set-up: the ghost 3 m in front of the leader
        // K3: GAME_BOOKTGT=hex uid picks the monster (any level), default the EP1L01 smallghost
        let tgt = std::env::var("GAME_BOOKTGT").ok().and_then(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16).ok()).unwrap_or(GHOST);
        let hero = e.rt.game_buffers.get(&0).map(|b| e.mem.r32(b.0 + 8)).filter(|&l| l != 0).unwrap_or(SCOOBY);
        let p = rt.particles.lock().unwrap();
        let ph = p.sub.physic.PhysicOf(hero);
        let hp = p.sub.physic.Physic(ph).map(|x| x.frame);
        drop(p);
        if let Some(hp) = hp {
            let t = [hp.pos[0] + hp.at[0] * 3.0, hp.pos[1] + 1.0, hp.pos[2] + hp.at[2] * 3.0];
            // K4: through APIgameobj::SetTranslation (as GAME_WARP): writing only the physic frame pos
            // left the monster's ground collision behind -> it fell through the floor (EP1L03B shadows)
            let a = e.mem.alloca(0x10);
            for (i, c) in t.iter().enumerate() {
                e.mem.w32(a + 4 * i as u32, c.to_bits());
            }
            scooby_recomp::engine::APIgameobj::SetTranslation(e, tgt, a);
            eprintln!("game: K3 target {tgt:#x} -> {t:?} (hero {hero:#x})");
        }
    }
    let mut p = idle_pad();
    // SQUARE (book) until the book is open (the tutorial texts after the intro may eat presses)
    if r >= 120 && r % 60 < 10 && r < 1200 && GetCurrentStateID(e, g_book) == 1 {
        p.buttons = pad::SQUARE;
    }
    if r > 145 && f % 2 == 0 {
        if let Some(h) = e.rt.map.find(g_book) {
            let l = e.rt.objects[h].as_ref().unwrap().inst.local;
            // J22: GAME_MASH=none (no press) / wrong (always a wrong button): the failure paths
            let want = e.mem.r32(l + 0x1e8);
            let want = match std::env::var("GAME_MASH").as_deref() { Ok("none") => 0, Ok("wrong") => if want == 2 { 4 } else { 2 }, _ => want };
            p.buttons = match want {
                2 => pad::CROSS,
                4 => pad::CIRCLE,
                8 => pad::TRIANGLE,
                _ => 0,
            };
        }
    }
    p
}

// ---------------------------------------------------------------------------------------------
// runtime stepping

fn leader(g: &Game) -> u32 {
    let l = g.e.rt.game_buffers.get(&0).map(|b| g.e.mem.r32(b.0 + 8)).unwrap_or(0);
    if l == 0 { SCOOBY } else { l }
}

fn one_frame(g: &mut Game) {
    let t0 = g.prof.enabled().then(std::time::Instant::now);
    one_frame_inner(g);
    if let Some(t0) = t0 {
        g.prof.frames += 1;
        Prof::add(&mut g.prof.logic, t0.elapsed().as_secs_f64() * 1e3);
    }
}

fn one_frame_inner(g: &mut Game) {
    let f = g.steps;
    if g.lvl_f0 == u32::MAX {
        g.lvl_f0 = f;
    }
    k5_frame(g, f);
    if f == 10 && g.give_book {
        give_book(g);
    }
    // debug: GAME_WARP=x,y,z:frame puts the leader (APIgameobj::SetTranslation) at (x,y,z)
    // (P26 R3: several warps separated by ';')
    if let Some(v) = std::env::var("GAME_WARP").ok().and_then(|v| v.split(';').find_map(|v| {
        let (p, fr) = v.split_once(':')?;
        let v: Vec<f32> = p.split(',').filter_map(|s| s.parse().ok()).collect();
        (fr.parse::<u32>().ok()? == f && v.len() == 3).then_some(v)
    })) {
        use scooby_recomp::engine::APIgameobj;
        let l = leader(g);
        let p = g.e.mem.alloca(0x10);
        for (i, c) in v.iter().enumerate() {
            g.e.mem.w32(p + 4 * i as u32, c.to_bits());
        }
        APIgameobj::SetTranslation(&mut g.e, l, p);
        // and the partner (GB0+0xc) one unit beside it: the team logic regroups separated heroes
        let o = g.e.rt.game_buffers.get(&0).map(|b| g.e.mem.r32(b.0 + 0xc)).unwrap_or(0);
        if o != 0 && o != l {
            g.e.mem.w32(p, (v[0] + 1.0).to_bits());
            APIgameobj::SetTranslation(&mut g.e, o, p);
        }
        eprintln!("game: GAME_WARP leader {l:#x} to {v:?} at frame {f}");
        // debug: GAME_GB2C=hex ORs progression bits into GB2+0xc (door lock masks) at the warp
        if let Some(b) = std::env::var("GAME_GB2C").ok().and_then(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16).ok()) {
            if let Some(gb) = g.e.rt.game_buffers.get(&2).map(|b| b.0) {
                let v = g.e.mem.r32(gb + 0xc) | b;
                g.e.mem.w32(gb + 0xc, v);
            }
        }
    }
    // debug (P26 R6): GAME_HOLD="uid:x,y,z:from-to;..." (hex uid) keeps an object at (x,y,z)
    // (APIgameobj::SetTranslation every frame in [from,to]), like the PCSX2 lane's RAM hold
    if let Ok(v) = std::env::var("GAME_HOLD") {
        for it in v.split(';') {
            let p: Vec<&str> = it.split(':').collect();
            let [u, xyz, r] = p[..] else { continue };
            let Ok(u) = u32::from_str_radix(u.trim_start_matches("0x"), 16) else { continue };
            let c: Vec<f32> = xyz.split(',').filter_map(|s| s.parse().ok()).collect();
            let Some((a, b)) = r.split_once('-').and_then(|(a, b)| Some((a.parse::<u32>().ok()?, b.parse::<u32>().ok()?))) else { continue };
            if c.len() == 3 && (a..=b).contains(&f) {
                use scooby_recomp::engine::APIgameobj;
                let sp = g.e.mem.sp();
                let p = g.e.mem.alloca(0x10);
                for (i, c) in c.iter().enumerate() {
                    g.e.mem.w32(p + 4 * i as u32, c.to_bits());
                }
                APIgameobj::SetTranslation(&mut g.e, u, p);
                g.e.mem.set_sp(sp);
            }
        }
    }
    // debug (K3): GAME_SENDMSG="uid:msg:param:frame,..." (hex uid/msg/param) sends a logic message,
    // e.g. water 0xd8 to a fire ghost or 0xc1 to a mummy, to reach a monster's catchable state
    // P20 R44: GAME_PRIMWATCH=uid[:every] prints the object's coll prim ltm (joint, up row, translation) every n frames
    if let Some((u, n)) = std::env::var("GAME_PRIMWATCH").ok().and_then(|v| { let (u, n) = v.split_once(':').unwrap_or((&v, "10")); Some((u32::from_str_radix(u.trim_start_matches("0x"), 16).ok()?, n.parse::<u32>().ok()?.max(1))) }) {
        if f % n == 0 {
            let s = scooby_recomp::engine::runtime::J2(&g.rt.particles).physic(|m, _| {
                m.by_owner.get(&u).and_then(|&id| m.Physic(id)).map(|p| p.model.prims.iter().flatten().map(|q| format!("j{} up {:?} t {:?}", q.base.joint, q.base.ltm[1], q.base.ltm[3])).collect::<Vec<_>>())
            });
            eprintln!("game: primwatch f{f} {u:#x} {s:?}");
        }
    }
    // P36 R4: GAME_OPTVOL=music,fx,movies: the original Options > Sound sliders (miscHUDFUNC_SetMusicVolume /
    // SetFXVolume / SetMoviesVolume, misc_hud.rs), re-applied every frame (volume tests with GAME_AUDIO_WAV)
    if let Some(v) = std::env::var("GAME_OPTVOL").ok().map(|v| v.split(',').filter_map(|s| s.parse::<f32>().ok()).collect::<Vec<_>>()).filter(|v| v.len() == 3) {
        scooby_recomp::engine::runtime::J2(&g.rt.particles).sound(|s| {
            s.APInewSound_SetVolumeByGroup(0xa8, v[0]);
            s.APInewSound_SetFXVolume(v[1]);
            s.APInewSound_SetVolumeByGroup(0x17, v[1]);
            s.APInewSound_SetVolumeByGroup(0x300, v[2]);
            s.APInewSound_SetVideoVolume(v[2]);
        });
    }
    if let Ok(v) = std::env::var("GAME_SENDMSG") {
        for it in v.split(',') {
            let p: Vec<&str> = it.split(':').collect();
            let h = |s: &str| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok();
            if let [u, m, a, fr] = p[..] {
                if fr.parse::<u32>().ok() == Some(f) {
                    if let (Some(u), Some(m), Some(a)) = (h(u), h(m), h(a)) {
                        let Game { e, rt, .. } = g;
                        rt.particles.lock().unwrap().logic = true;
                        e.SendLogicMessageTo(u, m, a, 0, 0);
                        rt.particles.lock().unwrap().logic = false;
                        eprintln!("game: GAME_SENDMSG {u:#x} msg {m:#x} p {a:#x} at frame {f}");
                    }
                }
            }
        }
    }
    // P8: the K3 book test set-up (power 180, monster moved in front of the hero) is not part of a --route run
    let pad = if g.autoplay && g.route.is_empty() { autoplay_pad(g, f) } else { g.pad };
    let mut pad = padscript(f, pad);
    if g.menu_cross {
        pad = idle_pad();
        if g.route_src.as_deref() == Some("ep1") {
            if lvl_steps(g) % 300 < 6 {
                pad.buttons |= pad::CROSS;
            }
        } else {
            // P21 R65: after EP2+ the savepanel save-status menu needs DOWN then X (the second entry);
            // X alone toggles menu <-> games list (as the p17 harness end menus)
            match lvl_steps(g) % 300 {
                282..288 => pad.ly = 0xff,
                294..300 => pad.buttons |= pad::CROSS,
                _ => {}
            }
        }
    }
    if f == 100 {
        if let Ok(v) = std::env::var("GAME_PROBE") {
            probe_heights(g, &v);
        }
        if let Ok(v) = std::env::var("GAME_FLOOD") {
            flood_dump(g, &v);
        }
        if let Ok(v) = std::env::var("GAME_LEDGES") {
            ledges_dump(g, &v, true);
        }
        if let Ok(v) = std::env::var("GAME_POLYS") {
            ledges_dump(g, &v, false);
        }
        // P7: the wall map (GAME_MAP) for any level
        if let Ok(v) = std::env::var("GAME_MAP") {
            floor_map(g, &v);
        }
    }
    // P20 R43: GAME_LISTCODE_AT=f1,f2,.. repeats the GAME_LISTCODE dump at those frames
    if f == 2 || std::env::var("GAME_LISTCODE_AT").is_ok_and(|v| v.split(',').any(|s| s.parse() == Ok(f))) {
        list_code(g);
    }
    if std::env::var("GAME_DUMP_DOORS").ok().and_then(|v| v.parse::<u32>().ok()) == Some(f) {
        dump_doors(g);
    }
    // P32 R79: GAME_DUMP_ANIM=f1,f2,.. (level-relative frames, every level) T-pose check (skinned objects: pose?, connected HAnim ids)
    if std::env::var("GAME_DUMP_ANIM").is_ok_and(|v| v.split(',').any(|s| s.parse() == Ok(lvl_steps(g)))) {
        dump_anims(g, lvl_steps(g));
    }
    let pad = if g.cam_standin.is_some() { standin_pre_frame(g, pad) } else if !g.route.is_empty() { route_any(g).map_or(pad, |mut p| { p.buttons |= pad.buttons; p }) } else { pad };
    // P34: the PC Settings page of the pause Options menu takes the pad while open
    let mut pad = pad;
    game_settings::page_input(&mut game_settings::PAGE.lock().unwrap(), &mut pad);
    g.rt.particles.lock().unwrap().devices.SetPadState(0, pad);
    let Game { e, rt, .. } = g;
    // P22 R23: EFAnimationMgr::PlayHAnim poses the hierarchy at once: re-pose the objects the input
    // callbacks played before the logic reads their node frames (see EngineRuntime::StepInputPlayed)
    let played = rt.StepInputPlayed(e, g.raw_dt);
    refresh_nodes(g, &played);
    let Game { e, rt, .. } = g;
    rt.FrameAfterInput(e, g.raw_dt);
    if g.cam_standin.is_some() {
        standin_post_frame(g);
    }
    let Game { e, rt, .. } = g;
    cutscene_update(e, rt);
    let Game { rt, .. } = g;
    // P33 R4: the old far >= 500 floor (stand-in from before the camera zone code was ported) is
    // off: GAME_NO_FAR probe of all 31 levels, frames 60..420, every camera far set by
    // Camera_BaseFUNC_SetClipPlane (15..250); only frame 0 has the RwCameraCreate default 10.
    // GAME_FAR_FLOOR=1 restores it (debug).
    for c in rt.gfx.cameras.cameras.iter_mut().flatten() {
        if c.rw.far <= 10.0 && std::env::var_os("GAME_FAR_FLOOR").is_some() {
            c.rw.far = 500.0;
        }
    }
    if std::env::var_os("GAME_DEBUG").is_some() && f % 60 == 0 {
        let l = leader(g);
        let p = g.rt.particles.lock().unwrap();
        let c = g.rt.gfx.cameras.cameras.iter().flatten().next().map(|c| (c.frame_owner, c.ltm.pos, c.ltm.at, c.rw.near, c.rw.far, c.rw.view_window));
        let gb0 = g.e.rt.game_buffers.get(&0).map(|b| b.0).unwrap_or(0);
        let zone = if gb0 != 0 { g.e.mem.r32(gb0 + 0x24) } else { 0 };
        let cs = p.cutscene.as_ref().map(|c| (c.flags, c.current, c.time, c.duration));
        eprintln!("game: f{f} leader {l:#x} at {:?} f0 {:?} zone {zone} cs {cs:?} cam {c:?} objs drawn {}", p.world.frames.get(&l).map(|m| m.pos), p.world.frames.get(&0x2a0000f0).map(|m| m.pos), g.gmgr.draws.len());
        drop(p);
        let st: Vec<_> = [0x2a00_00ef, 0x2a00_00f0, 0x2a00_004a, 0x2a00_0047].iter().map(|&u| GetCurrentStateID(&mut g.e, u)).collect();
        eprintln!("game: f{f} states shaggy/scooby/cam/velma {st:x?} gb0 {:x?} gb1 {:?}", (0..0x40).step_by(4).map(|o| if gb0 != 0 { g.e.mem.r32(gb0 + o) } else { 0 }).collect::<Vec<_>>(), (1..4).map(|n| { let b = gb(g, n); if b != 0 { f32::from_bits(g.e.mem.r32(b)) } else { -1.0 } }).collect::<Vec<_>>());
    }
    g.steps += 1;
    scooby_recomp::engine::gfx::bevy_adapter::SUBBLEND_FRAME.store(g.steps as u32, std::sync::atomic::Ordering::Relaxed);
}

/// P37 R2: GAME_PROF=1 per-pass split of render_frame_inner: (pass, total ms, max ms) since the last prof line
static STAGES: Mutex<Vec<(&'static str, f64, f64)>> = Mutex::new(Vec::new());
static STAGES_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static MAIN_T0: Mutex<Option<std::time::Instant>> = Mutex::new(None);

/// end of render pass `name` (started at `*t`, None = profiling off): accumulate, restart the clock
fn stage(t: &mut Option<std::time::Instant>, name: &'static str) {
    let Some(t0) = *t else { return };
    let now = std::time::Instant::now();
    let ms = (now - t0).as_secs_f64() * 1e3;
    let mut v = STAGES.lock().unwrap();
    match v.iter_mut().find(|x| x.0 == name) {
        Some(x) => {
            x.1 += ms;
            x.2 = x.2.max(ms);
        }
        None => v.push((name, ms, ms)),
    }
    *t = Some(now);
}

/// P37 R10: fold ApplyDrawList's sub-timers (bevy_adapter::DL_SUB) into the stage list as `dl:*`
fn stage_dl_sub() {
    let sub = std::mem::take(&mut *scooby_recomp::engine::gfx::bevy_adapter::DL_SUB.lock().unwrap());
    let mut v = STAGES.lock().unwrap();
    for (name, ms) in sub {
        match v.iter_mut().find(|x| x.0 == name) {
            Some(x) => {
                x.1 += ms;
                x.2 = x.2.max(ms);
            }
            None => v.push((name, ms, ms)),
        }
    }
}

/// F1: GAME_PROF=1 timing log, one line per wall second.
#[derive(Default)]
struct Prof {
    on: Option<bool>,
    since: Option<std::time::Instant>,
    updates: u32,
    frames: u32,
    logic: (f64, f64),
    render: (f64, f64),
    upd_max: f64,
    last: Option<std::time::Instant>,
}

impl Prof {
    fn enabled(&mut self) -> bool {
        *self.on.get_or_insert_with(|| {
            let on = std::env::var("GAME_PROF").is_ok();
            STAGES_ON.store(on, std::sync::atomic::Ordering::Relaxed);
            on
        })
    }
    fn add(slot: &mut (f64, f64), ms: f64) {
        slot.0 += ms;
        slot.1 = slot.1.max(ms);
    }
    /// called once per Bevy update (start of step_runtime)
    fn tick(&mut self, steps: u32) {
        if !self.enabled() {
            return;
        }
        let now = std::time::Instant::now();
        if let Some(l) = self.last {
            self.upd_max = self.upd_max.max((now - l).as_secs_f64() * 1e3);
        }
        self.last = Some(now);
        let since = *self.since.get_or_insert(now);
        if (now - since).as_secs_f64() >= 1.0 {
            let n = self.frames.max(1) as f64;
            let u = self.updates.max(1) as f64;
            eprintln!("prof: f{steps} {:.1} upd/s (max gap {:.1} ms) {:.1} logic/s | logic avg {:.2} max {:.2} ms | render avg {:.2} max {:.2} ms",
                self.updates as f64 / (now - since).as_secs_f64(), self.upd_max, self.frames as f64 / (now - since).as_secs_f64(),
                self.logic.0 / n, self.logic.1, self.render.0 / u, self.render.1);
            let st: Vec<String> = std::mem::take(&mut *STAGES.lock().unwrap()).iter().filter(|x| x.2 >= 0.5).map(|x| format!("{} {:.2}/{:.2}", x.0, x.1 / u, x.2)).collect();
            if !st.is_empty() {
                eprintln!("prof-render: f{steps} (avg/max ms) {}", st.join(" | "));
            }
            *self = Prof { on: Some(true), since: Some(now), last: Some(now), ..Prof::default() };
        }
        self.updates += 1;
    }
}

fn step_runtime(time: Res<Time>, mut game: ResMut<Game>, mut exit: MessageWriter<AppExit>) {
    let g = &mut *game;
    let steps = g.steps;
    g.prof.tick(steps);
    // WP-P3: MpegPlay blocks the engine: only the movie advances (one readMpeg step per update)
    if let Some(mv) = g.movie.as_mut() {
        let pad = padscript(g.steps, g.pad);
        let dt = if g.autoplay || !g.audio { STEP } else { time.delta_secs().min(0.1) };
        match mv.clock {
            Some(c) if g.audio && !g.autoplay => mv.play.UpdateClock(c.elapsed().as_secs_f32(), pad.buttons),
            _ => mv.play.Update(dt, pad.buttons),
        };
        g.steps += 1;
        g.fresh = true;
        if g.max_frames.is_some_and(|n| g.steps >= n) {
            exit.write(AppExit::Success);
        }
        return;
    }
    if !g.loaded {
        return;
    }
    if g.hold > 0 {
        g.hold -= 1;
        g.fresh = true;
        return;
    }
    if g.autoplay || !g.audio {
        // scripted / headless: exactly one PS2 frame per update (deterministic)
        // debug (P27 R5): GAME_HEADLESS30=1 feeds the real-time 2-vsync dt (30 Hz logic) instead
        g.raw_dt = if std::env::var_os("GAME_HEADLESS30").is_some() { 2.0 * VSYNC } else { STEP };
        one_frame(g);
        g.fresh = true;
    } else {
        // F1: EFEngineController::MainLoop runs ONE input+logic+render pass per flip, and the flip
        // waits for vsync (59.94 Hz NTSC). A pass that misses vsyncs is measured by EFTimer::Update
        // as n field periods and goes through ComputeDeltaTime (clamp 0.05, 10-frame average): there
        // is no catch-up loop (the port used to run up to 4 fixed 1/60 steps per update). Frames are
        // due on the 59.94 Hz grid whatever the monitor refresh is.
        // P28 B12: the original's pass takes 2 fields (PCSX2: FPS 30 / VPS 60, EC+0x98 = 30 f/s,
        // dt = 0.03337, P25 R29/R30): logic AND render run once per 2 vsyncs. Frame-counted logic
        // (Ghost_base Waving bob, per-frame lerps, frame timers) ran 2x fast at one pass per vsync.
        // Scripted/headless (STEP) is unchanged (one logic tick per update).
        // P28 R12 (P25 R62): the original has no 60 Hz logic mode (Ghost_base Waving steps once per logic
        // frame with no dt, also at 25 f/s), so the old `GAME_LOGIC60` 1-vsync pacing (bob 2x fast) is gone.
        let period = 2.0 * VSYNC;
        // P39 R5: scooby_recomp::logic::frame::vsync_pace (unit-tested: a long minimize / display sleep is one
        // pass with raw dt <= 0.25 s, ComputeDeltaTime clamps it to 0.05, no catch-up burst)
        let (acc, due) = scooby_recomp::logic::frame::vsync_pace(g.acc, time.delta_secs(), period, VSYNC_SLACK);
        g.acc = acc;
        if let Some(raw) = due {
            g.raw_dt = raw;
            one_frame(g);
            g.fresh = true;
        }
    }
    // WP-S3: EFEngineController::Run flags set by APIgame (QuitGame/APIqa CloseGame: flags & 4 ends the
    // game; LoadLevel: flags & 8). Level switching (teardown + reload) is not implemented here: logged.
    {
        let mut p = g.rt.particles.lock().unwrap();
        // debug: GAME_LOADLEVEL=EP1L02:frame requests a switch like APIgame::LoadLevel
        let forced = std::env::var("GAME_LOADLEVEL").ok()
            .and_then(|v| scooby_recomp::engine::host::game::LevelRequest::forced(&v, g.steps, &mut g.loadlevel_fired));
        if let Some(r) = p.game.take_level_request().or(forced) {
            eprintln!("game: APIgame::LoadLevel request {r:?} at frame {}", g.steps);
            g.level_req = Some(r);
        }
        if p.game.quit_requested() {
            eprintln!("game: APIgame::QuitGame");
            exit.write(AppExit::Success);
        }
    }
    if g.max_frames.is_some_and(|n| g.steps >= n) && g.pending_shot.is_none() {
        eprintln!("game: {} frames done", g.steps);
        exit.write(AppExit::Success);
    }
}

// ---------------------------------------------------------------------------------------------
// rendering

/// P22 R23: node frames of `uids` from their current anim pose (the J6 publish of draw_objects)
fn refresh_nodes(g: &mut Game, uids: &[u32]) {
    let Game { rt, render, objs, .. } = g;
    let Some(rd) = render.as_mut() else { return };
    let mut p = rt.particles.lock().unwrap();
    for &uid in uids {
        let Some(&(res_uid, _)) = p.sub.obj3d.get(&uid) else { continue };
        let Some(frame) = p.world.frames.get(&uid).copied() else { continue };
        let Some(r) = rd.res.iter_mut().find(|r| r.res.uid == res_uid) else { continue };
        let Some(c) = r.clump.as_mut() else { continue };
        let Some(o) = objs.get_mut(&uid) else { continue };
        let ltm: RwMat4 = [
            frame.right[0], frame.right[1], frame.right[2], 0., frame.up[0], frame.up[1], frame.up[2], 0., frame.at[0], frame.at[1], frame.at[2], 0., frame.pos[0], frame.pos[1], frame.pos[2], 1.,
        ];
        let pose = p.sub.anim_bind.get(&uid).and_then(|b| p.sub.anim.get(&uid).and_then(|m| m.GetPose(b.obj3d)));
        if let Some((nodes, ids)) = scooby_recomp::engine::gfx::render::pose_nodes(o, c, pose.as_deref(), ltm) {
            p.world.nodes.insert(uid, nodes);
            p.world.node_ids.insert(uid, ids);
            p.world.node_root.insert(uid, frame);
            // P28 R11: the logic-side poser (anim start inside the logic pass) needs the clump too
            if !p.sub.node_poser.clumps.contains_key(&res_uid) {
                p.sub.node_poser.clumps.insert(res_uid, c.clone());
            }
        }
    }
}

/// EF3dObj frame/pose update + EF3dObj_SP::Render of every visible game object.
fn draw_objects(g: &mut Game) {
    let Game { e, rt, render, objs, hud_objs: hobjs, gmgr, had_logic, steps, fader_step, raw_dt, blend_col, distort_only, .. } = g;
    let hud_objs = hud_3d_objects(&rt.frame.hud);
    let Some(rd) = render.as_mut() else { return };
    gmgr.draws.clear();
    let mut p = rt.particles.lock().unwrap();
    // P28 B8: APIgraphics::TexFaderPlay -> EFTexFader_SP::FadeTo on the target EF3dObj (the panel's
    // meter icon swaps TOD_Normal_Meter <-> drama_masks (costume) / item icons and the courage face
    // textures this way) + EFTexFader_SP::Update(dt) once per logic step (EFWorld::Update). The
    // requests were drained into frame_gfx_deferred but never applied (icons never changed).
    if *fader_step != *steps {
        *fader_step = *steps;
        gmgr.tex_fader.Update(*raw_dt);
        // P26 B11: APIgraphics::SetGameObjBlendColor -> EF3dObj_SP::SetBlendingColor on
        // GetGameObject(obj)->Get3dObject(index) (applied to the EF3dObj below, every frame:
        // the EF3dObj_SPs are created lazily)
        for d in &rt.frame_gfx_deferred {
            let scooby_recomp::engine::gfx::api::GfxDeferred::SetBlendingColor { game_obj, obj3d_index, rgba } = d else { continue };
            let Some(res_uid) = p.sub.obj3d_list.get(game_obj).and_then(|l| l.get(*obj3d_index as usize)).map(|x| x.1) else { continue };
            blend_col.insert((*game_obj, res_uid), *rgba);
        }
        for d in &rt.frame_gfx_deferred {
            // P28 B9/B11: SetGameObjBlendColor -> EF3dObj_SP::SetBlendingColor on the same target
            // (sandwich ingredient windows greyed 0.2/0.75 until collected; shadow silhouette tint)
            use scooby_recomp::engine::gfx::api::GfxDeferred as D;
            let (game_obj, obj3d_index, params, rgba) = match d {
                D::TexFaderPlay { game_obj, obj3d_index, params } => (game_obj, obj3d_index, Some(params), None),
                D::SetBlendingColor { game_obj, obj3d_index, rgba } => (game_obj, obj3d_index, None, Some(*rgba)),
                _ => continue,
            };
            if std::env::var_os("GAME_FADERDBG").is_some() {
                eprintln!("fader f{} {d:x?} list {:x?}", *steps, p.sub.obj3d_list.get(game_obj));
            }
            let Some(res_uid) = p.sub.obj3d_list.get(game_obj).and_then(|l| l.get(*obj3d_index as usize)).map(|x| x.1) else { continue };
            let Some(c) = rd.res.iter_mut().find(|r| r.res.uid == res_uid).and_then(|r| r.clump.as_mut()) else {
                if std::env::var_os("GAME_FADERDBG").is_some() { eprintln!("fader: no clump {res_uid:#x}"); }
                continue;
            };
            // window objects are the HUD EF3dObjs (owner HUD_OWNER + k); else the world object's own
            let (owner, o) = if let Some(k) = hobjs.iter().position(|h| h.0 == (*game_obj, res_uid)) {
                (HUD_OWNER + k as u32, &mut hobjs[k].1)
            } else if hud_objs.iter().any(|h| (h.0, h.1) == (*game_obj, res_uid)) || p.hud3d.contains_key(game_obj) {
                let mut o = EF3dObj_SP::Initialize(res_uid, c, 0);
                o.SetVisible(1);
                hobjs.push(((*game_obj, res_uid), o));
                let k = hobjs.len() - 1;
                (HUD_OWNER + k as u32, &mut hobjs[k].1)
            } else if p.sub.obj3d.get(game_obj).is_some_and(|x| x.0 == res_uid) {
                // requests issued before the object's first draw (Shadows SetClipping at START)
                let cf = p.sub.obj3d_flags.get(game_obj).copied().unwrap_or(0);
                let o = objs.entry(*game_obj).or_insert_with(|| {
                    let mut o = EF3dObj_SP::Initialize(res_uid, c, cf);
                    o.SetVisible(1);
                    o
                });
                (*game_obj, o)
            } else {
                // P28 R12 (B10): the original fades obj+0x174[index] whether or not a HUD window shows it
                // yet; the panel's Init fades the courage face (0xf000628) before its Window3DSetObj, and
                // dropping it left the fader-idle flag +0x30 at 0 forever (portrait never changed). The
                // object's secondary EF3dObjs are drawn only through HUD windows -> create that one now.
                let mut o = EF3dObj_SP::Initialize(res_uid, c, 0);
                o.SetVisible(1);
                hobjs.push(((*game_obj, res_uid), o));
                let k = hobjs.len() - 1;
                (HUD_OWNER + k as u32, &mut hobjs[k].1)
            };
            if std::env::var_os("GAME_FADERDBG").is_some() { eprintln!("fader: -> owner {owner:#x}"); }
            match (params, rgba) {
                (Some(params), _) => {
                    gmgr.tex_fader.FadeTo(owner, o, c, &rt.gfx.tex_dict, params);
                }
                (_, Some([r, g, b, a])) => o.SetBlendingColor(r, g, b, a),
                _ => {}
            }
        }
    }
    let mut uids: Vec<u32> = p.sub.obj3d.keys().copied().collect();
    uids.sort_unstable();
    // P32 R1: EFFBEffectMgr_BC::Update(pass 8) owners (drawn by the effect even when clipped out)
    let fx_owners: HashSet<u32> = p.s1.fb.Distortions(|u| p.world.frames.contains_key(&u)).iter().map(|f| f.0).collect();
    distort_only.clear();
    for uid in uids {
        // latent templates are not in the world; killed objects left the logic map
        if rd.latent.contains(&uid) {
            continue;
        }
        // J6: hidden objects (the book before GrabBook shows it) still update their hierarchy:
        // the logic reads their node frames (TomeDoomCB_PHYSIC hook joints) while hidden
        // K10: + EFWorld::ClipGameObjects clip mask bit 4 (e.g. captured monsters)
        // P26 B3: + APIgameobj::EnableObject(0) (0x001f0400 RpWorldRemoveClump: picked-up collectables)
        // P28 R1: + with GAME_LOGICCLIP the word ClipGameObjects picked (s1.clip_word, only filled then):
        // EFWorld.cpp:881 adds the clump to the world only when (word & 4) == 0 and the object is in view,
        // else RpWorldRemoveClump. A monster frozen far away (0x6a7: logic off, anim off, 4) is not drawn
        // (was drawn in its bind pose: its START state never ran)
        let cc = p.proximity.get(&uid).map_or(scooby_recomp::engine::host::gameobj::CC_DEFAULT, |e| e.2);
        let hidden_other = p.world.hidden.contains(&uid) || !p.s1.is_enabled(uid) || p.s1.clip_word.get(&uid).is_some_and(|w| w & 4 != 0);
        let clip_off = p.s1.clip_render_off(uid, cc);
        let hidden = hidden_other || clip_off;
        let alive = e.rt.map.find(uid).is_some();
        if !alive && (had_logic.contains(&uid) || !rd.placed.contains(&uid)) {
            continue;
        }
        let Some(&(res_uid, _)) = p.sub.obj3d.get(&uid) else { continue };
        let Some(frame) = p.world.frames.get(&uid).copied() else { continue };
        let Some(r) = rd.res.iter_mut().find(|r| r.res.uid == res_uid) else { continue };
        let Some(c) = r.clump.as_mut() else { continue };
        // P26 R16: EF3dObj::Initialize(DTCreationParameters) gets the DT3dObjectDescriptor INSTANCE_FLAG
        let cf = p.sub.obj3d_flags.get(&uid).copied().unwrap_or(0);
        let o = objs.entry(uid).or_insert_with(|| {
            let mut o = EF3dObj_SP::Initialize(res_uid, c, cf);
            o.SetVisible(1);
            o
        });
        let ltm: RwMat4 = [
            frame.right[0], frame.right[1], frame.right[2], 0., frame.up[0], frame.up[1], frame.up[2], 0., frame.at[0], frame.at[1], frame.at[2], 0., frame.pos[0], frame.pos[1], frame.pos[2], 1.,
        ];
        let pose = p.sub.anim_bind.get(&uid).and_then(|b| p.sub.anim.get(&uid).and_then(|m| m.GetPose(b.obj3d)));
        if let Some(pose) = &pose {
            if posedump(*steps) {
                for t in pose.iter() {
                    if t.state & 0xc != 0 || t.f[..3].iter().any(|s| (s - 1.0).abs() > 1e-3) {
                        eprintln!("POSE {uid:#x} id {} st {:#x} f {:?} v34 {:?}", t.id, t.state, t.f, t.v34);
                    }
                }
                eprintln!("POSE {uid:#x} n {} frame {:?}", pose.len(), frame.pos);
            }
        }
        // J6: publish the HAnim node frames (modelling matrix, LTM) for GetObjectFrame/GetBoneLMat
        if let Some((nodes, ids)) = scooby_recomp::engine::gfx::render::pose_nodes(o, c, pose.as_deref(), ltm) {
            if posedump(*steps) {
                for (i, x) in nodes.iter().enumerate() {
                    eprintln!("NODE {uid:#x} {i} l {:?} w {:?} at {:?}", x.0.pos, x.1.pos, x.1.at);
                }
            }
            p.world.nodes.insert(uid, nodes);
            p.world.node_ids.insert(uid, ids);
            p.world.node_root.insert(uid, frame);
            // P28 R11: the logic-side poser (anim start inside the logic pass) needs the clump too
            if !p.sub.node_poser.clumps.contains_key(&res_uid) {
                p.sub.node_poser.clumps.insert(res_uid, c.clone());
            }
        }
        if hidden {
            if hidden_other || !fx_owners.contains(&uid) {
                continue;
            }
            distort_only.insert(uid);
        }
        // EFHud3DWindow objects are drawn by the HUD pass below, not in the world
        if hud_objs.iter().any(|h| h.0 == uid) {
            continue;
        }
        if let Some(b) = blend_col.get(&(uid, res_uid)) {
            o.SetBlendingColor(b[0], b[1], b[2], b[3]);
        }
        // P26 R28: EF3dObj::Render reads the owner's +0x15c logic flags (0x8000 -> EnableStencilWrite)
        let flags_15c = e.rt.map.find(uid).and_then(|h| e.rt.objects[h].as_ref()).map_or(0, |x| x.logic_flags);
        // P26 R31: the EF3dObj +0x60 EFTextureAnimNode list (ApplyText/RestoreText around each atomic)
        // GAME_TEXANIM=0: debug opt-out (draw the base textures, pre-R31 look)
        let list = if std::env::var("GAME_TEXANIM").is_ok_and(|v| v == "0") { Vec::new() } else { p.sub.TexAnimsOf(uid, res_uid) };
        let mut ta = TexAnimSwap { list, saved: Vec::new() };
        o.Render(0, c, &OwnerInfo { uid, ltm, flags_15c, ..Default::default() }, gmgr, &mut ta);
    }
    // J7: EFHud3DWindow_SP::SpecificRender per visible 3D window: frame pos zeroed, translated
    // (POSTCONCAT) to the window centre in HUD camera space, then EF3dObj Render (vtable+0x70)
    // J8: the window object is GameObject+0x174[n] (its own EF3dObj: e.g. meter frame / fill / icon)
    for &(uid, res_uid, pos) in &hud_objs {
        let Some(mut frame) = p.world.frames.get(&uid).copied() else { continue };
        // P28 B10: the window EF3dObj's own frame (Rotate/SetRotation on its 0xf000xxx uid: courage
        // needle, tome meter) is a child of the game object frame: LTM 3x3 = local * parent
        if let Some(l) = p.sub.obj3d_desc.iter().find(|(_, v)| **v == (uid, res_uid)).and_then(|(d, _)| p.world.frames.get(d)) {
            let mul = |r: [f32; 3]| -> [f32; 3] { std::array::from_fn(|j| r[0] * frame.right[j] + r[1] * frame.up[j] + r[2] * frame.at[j]) };
            let (r, u, a) = (mul(l.right), mul(l.up), mul(l.at));
            frame.right = r;
            frame.up = u;
            frame.at = a;
        }
        let Some(c) = rd.res.iter_mut().find(|r| r.res.uid == res_uid).and_then(|r| r.clump.as_mut()) else { continue };
        let k = match hobjs.iter().position(|h| h.0 == (uid, res_uid)) {
            Some(k) => k,
            None => {
                let mut o = EF3dObj_SP::Initialize(res_uid, c, 0);
                o.SetVisible(1);
                hobjs.push(((uid, res_uid), o));
                hobjs.len() - 1
            }
        };
        let o = &mut hobjs[k].1;
        // P32 R1: EFGameObject::SetGameObjectVisible (0x00200100) = EF3dObj_SP::SetVisible on Get3dObject(0),
        // the EF3dObj this window renders (vtable+0x70): rdrmngr UpdateEnemyPoints hides the radar spots
        // that way (a captured ghost's red dot froze on the minimap while the window kept drawing)
        o.SetVisible(!(p.world.hidden.contains(&uid) && p.sub.obj3d.get(&uid).is_some_and(|r| r.0 == res_uid)) as u32);
        let ltm: RwMat4 = [
            frame.right[0], frame.right[1], frame.right[2], 0., frame.up[0], frame.up[1], frame.up[2], 0., frame.at[0], frame.at[1], frame.at[2], 0., pos[0], pos[1], pos[2], 1.,
        ];
        // N2: the window's EF3dObj is driven by the anims whose REFPUID is its descriptor
        // (EFWorld::Update -> UpdateHierarchy of every Get3dObject); unposed, the pause-menu
        // panel parts sat at their rest pose (stray glyph left of the items)
        if let Some(pose) = p.sub.PoseOf(uid, res_uid) {
            scooby_recomp::engine::gfx::render::apply_pose(o, c, &pose, ltm);
        }
        o.SyncFrames(ltm);
        if let Some(b) = blend_col.get(&(uid, res_uid)) {
            o.SetBlendingColor(b[0], b[1], b[2], b[3]);
        }
        o.Render(0, c, &OwnerInfo { uid: HUD_OWNER + k as u32, ltm, ..Default::default() }, gmgr, &mut NoTexAnim);
    }
}

/// J8: draw-list owner ids of the HUD EF3dObjs (HUD_OWNER + index in `Game::hud_objs`)
const HUD_OWNER: u32 = 0xfff0_0000;

/// P26 R26: StandardMaterial -> HUD 16-bit-Z material (game_fx::HudMaterial), per source material
#[derive(Resource, Default)]
struct HudMatCache(HashMap<AssetId<StandardMaterial>, Handle<game_fx::HudMaterial>>);

/// P26 R26: the HUD clump entities (owner >= HUD_OWNER) draw with game_fx::HudMaterial (same
/// StandardMaterial inputs, fragment depth snapped to the PS2 16-bit HUD Z grid, ties -> earlier draw).
/// ApplyDrawList re-inserts the StandardMaterial every frame, so the swap runs every frame.
/// `GAME_HUDZ16=0`: old float-depth HUD (comparison only).
fn hud_z16(world: &mut World) {
    if std::env::var("GAME_HUDZ16").is_ok_and(|v| v == "0") {
        return;
    }
    if !world.contains_resource::<HudMatCache>() {
        world.init_resource::<HudMatCache>();
    }
    let ents: Vec<(Entity, Handle<StandardMaterial>)> = world
        .query::<(Entity, &DrawListEntity, &MeshMaterial3d<StandardMaterial>)>()
        .iter(world)
        .filter(|(_, d, _)| d.owner >= HUD_OWNER)
        .map(|(e, _, m)| (e, m.0.clone()))
        .collect();
    for (e, sm) in ents {
        let h = match world.resource::<HudMatCache>().0.get(&sm.id()) {
            Some(h) => h.clone(),
            None => {
                let Some(base) = world.resource::<Assets<StandardMaterial>>().get(&sm).cloned() else { continue };
                let h = world.resource_mut::<Assets<game_fx::HudMaterial>>().add(game_fx::HudMaterial { base, extension: game_fx::HudDepth {} });
                world.resource_mut::<HudMatCache>().0.insert(sm.id(), h.clone());
                h
            }
        };
        world.entity_mut(e).remove::<MeshMaterial3d<StandardMaterial>>().insert(MeshMaterial3d(h));
    }
}

/// J7: (object uid, EF3dObj res uid, HUD-camera position) of every EFHud3DWindow drawn this frame (in order).
/// P26 R20: z = priority (as before) minus the list-order tie break of `hud_depths`.
fn hud_3d_objects(cmds: &[scooby_recomp::engine::hud::HudDrawCmd]) -> Vec<(u32, u32, [f32; 3])> {
    let (w, h) = (SCREEN.0 as f32, SCREEN.1 as f32);
    cmds.iter()
        .zip(hud_depths(cmds))
        .filter_map(|(c, z)| match c.tex {
            scooby_recomp::engine::hud::HudTex::Object3D(o, res) => {
                let (cx, cy) = ((c.pos[0][0] + c.pos[3][0]) * 0.5, (c.pos[0][1] + c.pos[3][1]) * 0.5);
                Some((o, res, [-(cx / w - 0.5), -(cy / h - 0.5) * 0.875, z]))
            }
            _ => None,
        })
        .collect()
}

/// P26 R20: EFHudMgr_SP::Update (0x00273ab0) renders the window tree with z test + z write ON
/// (RwRenderStateSet 6/8 = 1): 2D / text quads at z = priority (cam+0x90 + priority * cam+0x8c)
/// and the EFHud3DWindow clumps translated to z = priority (EFHud3DWindow_SP::SpecificRender
/// 0x00214560), so the priority decides what covers what across 2D and 3D windows (EP3/L03 gold
/// nugget count, priority 3, over the meter ring clump). Both go to the HUD camera (at z 0 looking
/// +z) at z = priority; equal priorities keep the list order (later = nearer by 2e-5 per command).
fn hud_depths(cmds: &[scooby_recomp::engine::hud::HudDrawCmd]) -> Vec<f32> {
    cmds.iter().enumerate().map(|(n, c)| c.priority as f32 - n as f32 * 2e-5).collect()
}

#[derive(Component)]
struct OverlayMesh;
#[derive(Component)]
struct TrailMesh;
/// WP-S4: one projected-texture pass entity per projector occurrence (cached while unchanged)
#[derive(Component)]
struct ProjectorMesh(scooby_recomp::engine::gfx::projector::ProjectorOccurence);
#[derive(Component)]
struct HudMesh;

/// J7: EFHudMgr_SP::Update output -> meshes on the HUD camera (P26 R20: layer 2 with the clumps, after the
/// sprites: EFGraphicsMgr::Render calls the HUD update after RenderCamera). RwIm2D replacement:
/// each HudDrawCmd is a pixel-space quad strip (v0 v1 v2 v3 -> 0,1,2 2,1,3), RGBA 0..255,
/// RwRenderState as set by EFHudMgr_SP::Update: z test/write off (draw order = list order),
/// SRCALPHA / INVSRCALPHA blend, texture clamp. Object3D (EFHud3DWindow) rects are not drawn here
/// (the HUD 3D clumps are drawn by the 3D pass, see hud_3d_objects).
/// P22: port-side "Global Ray Tracing" row under the original options panel, drawn in the panel's
/// own font and colours (copied from its last text glyph); R toggles, persisted to settings.toml.
#[cfg(feature = "solari")]
fn rtgi_options_row(world: &mut World, hud: &mut Vec<scooby_recomp::engine::hud::HudDrawCmd>, fonts: &[Option<scooby_recomp::engine::hud::EFFont>]) {
    use scooby_recomp::engine::hud::{HudDrawCmd, HudTex};
    if !scooby_recomp::modules::panel::OPTIONS_OPEN.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    let Some((on, restart)) = scooby_recomp::engine::gfx::rtgi::options_row(world) else { return };
    // style of the panel's own rows: unselected label colour (orange), value colour (green)
    let font_cmds: Vec<HudDrawCmd> = hud.iter().filter(|c| matches!(c.tex, HudTex::Font { .. })).cloned().collect();
    let Some(proto) = font_cmds.iter().find(|c| c.colors[0] == [247, 131, 32, 255]).or(font_cmds.last()).cloned() else { return };
    let val_col = font_cmds.iter().find(|c| c.colors[0] == [0, 114, 54, 255]).map_or(proto.colors, |c| c.colors);
    // last panel row (lowest y among the label-coloured glyphs above the help text)
    let last_row_y = font_cmds.iter().filter(|c| c.colors[0] == proto.colors[0] && c.pos[0][1] < SCREEN.1 as f32 * 0.75).map(|c| c.pos[1][1]).fold(0.0f32, f32::max);
    if std::env::var("GAME_RTGI_HUDDUMP").is_ok() {
        for c in hud.iter() {
            if let HudTex::Font { font, tex } = c.tex { eprintln!("hud f{font} t{tex} pos {:?} uv {:?} col {:?} prio {}", c.pos, c.uv[0], c.colors[0], c.priority); }
        }
    }
    let HudTex::Font { font: fi, .. } = proto.tex else { return };
    let Some(f) = fonts.get(fi as usize).and_then(|f| f.as_ref()) else { return };
    let label = "GLOBAL RAY TRACING (R)  ";
    let text = format!("{label}{}{}", if on { "ON" } else { "OFF" }, if restart { " (RESTART)" } else { "" });
    let nlabel = label.chars().count();
    let glyph = |ch: char| f.ascii.get(ch as usize).and_then(|&code| f.Char(code as u32));
    // the panel draws its rows scaled: scale = row glyph height / natural height of that glyph
    // (upper-quartile drawn height of the row glyphs (small caps pull the median down) / median natural height of A..Z)
    let med = |mut v: Vec<f32>| { v.sort_by(f32::total_cmp); v.get(v.len() / 2).copied() };
    let q3 = |mut v: Vec<f32>| { v.sort_by(f32::total_cmp); v.get(v.len() * 3 / 4).copied() };
    let drawn = q3(font_cmds.iter().filter(|c| c.colors[0] == proto.colors[0]).map(|c| c.pos[1][1] - c.pos[0][1]).collect());
    let nat = med(('A'..='Z').filter_map(|ch| glyph(ch)).map(|c| c.sh).filter(|&h| h > 0.0).collect());
    let sc = match (drawn, nat) { (Some(d), Some(n)) => (d / n).clamp(0.25, 2.0), _ => 1.0 };
    let adv = |ch: char| sc * glyph(ch).filter(|_| ch != ' ').map_or(f.screen_space, |c| c.sw + f.screen_spacing);
    let width: f32 = text.chars().map(adv).sum();
    let base = if last_row_y > 0.0 { last_row_y + 36.0 } else { SCREEN.1 as f32 * 0.585 };
    let (mut x, y) = ((SCREEN.0 as f32 - width) * 0.5, base - 13.0 * sc.min(1.0));
    for (i, ch) in text.chars().enumerate() {
        if let Some(c) = glyph(ch).filter(|_| ch != ' ') {
            let (yy, w, h) = (y + c.syoff * sc, c.sw * sc, c.sh * sc);
            hud.push(HudDrawCmd {
                tex: HudTex::Font { font: fi, tex: c.tex as i32 as u32 },
                pos: [[x, yy], [x, yy + h], [x + w, yy], [x + w, yy + h]],
                uv: [[c.u, c.v], [c.u, c.v + c.dv], [c.u + c.du, c.v], [c.u + c.du, c.v + c.dv]],
                colors: if i < nlabel { proto.colors } else { val_col },
                priority: proto.priority,
            });
        }
        x += adv(ch);
    }
}

fn draw_hud(world: &mut World, cmds: &[scooby_recomp::engine::hud::HudDrawCmd], fonts: &[Option<Vec<String>>], dict: &scooby_recomp::engine::rw::image::RwTexDictionary) {
    use scooby_recomp::engine::hud::HudTex;
    let old: Vec<Entity> = world.query_filtered::<Entity, With<HudMesh>>().iter(world).collect();
    for e in old {
        world.despawn(e);
    }
    if !world.contains_resource::<GfxCache>() {
        world.init_resource::<GfxCache>();
    }
    let (w, h) = (SCREEN.0 as f32, SCREEN.1 as f32);
    let depth = hud_depths(cmds);
    for (n, c) in cmds.iter().enumerate() {
        let name = match &c.tex {
            HudTex::None => None,
            HudTex::Texture(t) => Some(t.clone()),
            HudTex::Font { font, tex } => fonts.get(*font as usize).and_then(|f| f.as_ref()).and_then(|f| f.get(*tex as usize)).cloned(),
            HudTex::Object3D(..) => continue,
        };
        // P26 R20: HUD camera space (the EFHud3DWindow_SP mapping); z = priority (hud_depths)
        // in the Transform so the transparent sort and the depth test against the clumps follow the list
        let pos: Vec<[f32; 3]> = c.pos.iter().map(|p| [-(p[0] / w - 0.5), -(p[1] / h - 0.5) * 0.875, 0.0]).collect();
        let col: Vec<[f32; 4]> = c.colors.iter().map(|&k| rwrgba_to_linear(k)).collect();
        let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, c.uv.to_vec());
        m.insert_attribute(Mesh::ATTRIBUTE_COLOR, col);
        m.insert_indices(Indices::U16(vec![0, 1, 2, 2, 1, 3]));
        let img = name.and_then(|t| {
            world.resource_scope(|wd, mut cache: Mut<GfxCache>| {
                let mut images = wd.resource_mut::<Assets<Image>>();
                cache.texture(&t, dict, &mut images)
            })
        });
        let mat = StandardMaterial { base_color_texture: img.map(|i| i.0), unlit: true, alpha_mode: AlphaMode::Blend, cull_mode: None, double_sided: true, ..default() };
        let mh = world.resource_mut::<Assets<Mesh>>().add(m);
        let ma = world.resource_mut::<Assets<StandardMaterial>>().add(mat);
        world.spawn((HudMesh, Mesh3d(mh), MeshMaterial3d(ma), Transform::from_xyz(0.0, 0.0, depth[n]), RenderLayers::layer(2)));
        // P26 R45: untextured HUD quads spanning the whole 4:3 screen (cut-scene letterbox bars:
        // Camera_BaseFUNC_SetWindows, x 0, dims 1.5 = wider than the PS2 frame; colour fades) cover
        // the whole frame on the PS2. The HUD camera is the centred 4:3 rect, so in Hor+ the side
        // strips get the quad's parts outside [0, w] on the full-width overlay camera (PS2 px units).
        // The centre stays the HUD camera's own draw (unchanged); in 4:3 nothing is added.
        let ow = world.get_resource::<OverlayWidth>().map_or(w, |o| o.0);
        let (x0, x1) = c.pos.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p[0]), b.max(p[0])));
        if matches!(c.tex, HudTex::None) && ow > w + 0.5 && x0 <= 0.5 && x1 >= w - 0.5 {
            let (l, r) = (w * 0.5 - ow * 0.5, w * 0.5 + ow * 0.5);
            let left: Vec<[f32; 3]> = c.pos.iter().map(|p| [if p[0] <= x0 { x0.min(l) } else { 0.0 }, p[1], 0.0]).collect();
            let right: Vec<[f32; 3]> = c.pos.iter().map(|p| [if p[0] >= x1 { x1.max(r) } else { w }, p[1], 0.0]).collect();
            for side in [left, right] {
                let pos: Vec<[f32; 3]> = side.iter().map(|p| [p[0] - w * 0.5, h * 0.5 - p[1], 1.0 + n as f32 * 1e-3]).collect();
                let col: Vec<[f32; 4]> = c.colors.iter().map(|&k| rwrgba_to_linear(k)).collect();
                let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
                m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
                m.insert_attribute(Mesh::ATTRIBUTE_UV_0, c.uv.to_vec());
                m.insert_attribute(Mesh::ATTRIBUTE_COLOR, col);
                m.insert_indices(Indices::U16(vec![0, 1, 2, 2, 1, 3]));
                let mat = StandardMaterial { unlit: true, alpha_mode: AlphaMode::Blend, cull_mode: None, double_sided: true, ..default() };
                let mh = world.resource_mut::<Assets<Mesh>>().add(m);
                let ma = world.resource_mut::<Assets<StandardMaterial>>().add(mat);
                world.spawn((HudMesh, Mesh3d(mh), MeshMaterial3d(ma), Transform::IDENTITY, RenderLayers::layer(1)));
            }
        }
    }
}

fn alpha_for_gs(code: Option<u32>) -> AlphaMode {
    match code {
        // J23: 0x68 = Cs*FIX+Cd (FIX 1.0): ONE/ONE ignoring alpha -> Premultiplied + vertex alpha 0
        Some(0x68) => AlphaMode::Premultiplied,
        Some(0x58) => AlphaMode::Add,
        Some(0xa1 | 0x91) => AlphaMode::Multiply,
        Some(0x44) => AlphaMode::Blend,
        _ => AlphaMode::Blend,
    }
}

/// EFSpriteMgr_BC batches (screen space 640x448, PS2 colours 0x80 = 1.0) -> meshes.
/// Rebuilt every frame (the sprite lists are small).
/// J23: RenderNormal batches with the z test on (group flags & 8 clear, e.g. the tod_beam
/// spirits, flags 6) are depth-tested against the world in the C++ (RwIm2D z = cam z_shift +
/// z_scale/z): they are un-projected back to world space with the RW camera (`cam` = LTM +
/// view window, inverse of SpriteCamera::Transform + the RenderBatchNormal projection) and drawn
/// by the 3D camera (layer 0) so the world occludes them. Before they were an overlay on top of
/// everything. z-test-off and occlusion-tested (flags 0x20) groups stay on the overlay.
fn draw_sprites(world: &mut World, sprites: &[(u32, SpritePass, Vec<SpriteDraw>)], dict: &scooby_recomp::engine::rw::image::RwTexDictionary, cam: Option<(&DTMatrix, [f32; 2])>) {
    let old: Vec<Entity> = world.query_filtered::<Entity, With<OverlayMesh>>().iter(world).collect();
    for e in old {
        world.despawn(e);
    }
    if !world.contains_resource::<GfxCache>() {
        world.init_resource::<GfxCache>();
    }
    let (w, h) = (SCREEN.0 as f32, SCREEN.1 as f32);
    let ow = world.get_resource::<OverlayWidth>().map_or(w, |o| o.0).max(w);
    let mut n = 0;
    // P33 R3: sprite occlusion test on (GAME_SPRITE_OCC=0: drawn without it, the pre-R3 behaviour)
    let occ_mask = if std::env::var("GAME_SPRITE_OCC").is_ok_and(|v| v == "0") { None } else { world.get_resource::<StencilMask>().map(|m| m.0.clone()) };
    let any_occ = occ_mask.is_some() && cam.is_some() && sprites.iter().any(|(_, _, ds)| ds.iter().any(|d| matches!(d, SpriteDraw::Occluded(_))));
    world.insert_resource(OccSpritesActive(any_occ));
    if std::env::var_os("GAME_OCC_LOG").is_some() {
        // debug: count of occlusion-tested sprites (logged every 60 draws)
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        if N.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % 60 == 0 {
            let occ: Vec<[u64; 4]> = sprites.iter().flat_map(|(_, _, ds)| ds.iter()).filter_map(|d| if let SpriteDraw::Occluded(o) = d { Some(o.sample) } else { None }).collect();
            eprintln!("game: occ sprites {} {:?}", occ.len(), &occ[..occ.len().min(4)]);
        }
    }
    for (_, _, draws) in sprites {
        for d in draws {
            // P33 R3: occluded sprites (EFSpriteGroup_SP::RenderSpriteNewOcc 0x002780a0): the sample
            // square goes to the stencil-mask camera (game_fx::OccSquare), the sprite reads it back
            // (game_fx::OccRead) and fades with the visible part of the square
            let mut occ_rect: Option<[f32; 4]> = None;
            if let (SpriteDraw::Occluded(o), Some((ltm, vw)), true) = (d, cam, occ_mask.is_some()) {
                let sc = scooby_recomp::engine::gfx::sprite::SpriteCamera { ltm: *ltm, view_window: vw, near: 0.0, far: 0.0, width: w as i32, height: h as i32, z_scale: 0.0, z_shift: 0.0 };
                let q = o.SampleQuad(&sc);
                let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
                m.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![q[0], q[1], q[2], q[0], q[2], q[3]]);
                m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 6]);
                let mh = world.resource_mut::<Assets<Mesh>>().add(m);
                let base = StandardMaterial { unlit: true, fog_enabled: false, alpha_mode: AlphaMode::Blend, cull_mode: None, double_sided: true, ..default() };
                let ma = world.resource_mut::<Assets<game_fx::OccSquare>>().add(game_fx::OccSquare { base, extension: game_fx::OccSquareExt {} });
                world.spawn((OverlayMesh, Mesh3d(mh), MeshMaterial3d(ma), Transform::IDENTITY, RenderLayers::layer(STENCIL_LAYER)));
                occ_rect = Some(o.SampleUv(w, h, ow));
            }
            let mut world_pos: Option<Vec<[f32; 3]>> = None;
            let (tex, blend, verts): (Option<String>, Option<u32>, Vec<([f32; 3], [f32; 2], [f32; 4])>) = match d {
                SpriteDraw::Batch(b) => {
                    if let (Some((ltm, vw)), false) = (cam, b.ztest_off) {
                        // inverse of: px = x/(rz*W) = 0.5cz - 0.5cx/vw0 (py likewise with H, vw1)
                        world_pos = Some(b.verts.iter().map(|v| {
                            let cz = 1.0 / v.recip_z;
                            let cx = (0.5 * cz - v.x / (v.recip_z * w)) * 2.0 * vw[0];
                            let cy = (0.5 * cz - v.y / (v.recip_z * h)) * 2.0 * vw[1];
                            [0, 1, 2].map(|k| ltm.pos[k] + ltm.right[k] * cx + ltm.up[k] * cy + ltm.at[k] * cz)
                        }).collect());
                    }
                    (b.texture.clone(), b.blend, b.verts.iter().map(|v| ([v.x, v.y, v.z], [v.u, v.v], v.rgba)).collect())
                }
                SpriteDraw::Occluded(o) => {
                    let r = o.rect;
                    let c = [o.rgba[0] as f32, o.rgba[1] as f32, o.rgba[2] as f32, o.rgba[3] as f32];
                    let q = [([r[0], r[1], o.z], [0., 0.]), ([r[2], r[1], o.z], [1., 0.]), ([r[2], r[3], o.z], [1., 1.]), ([r[0], r[1], o.z], [0., 0.]), ([r[2], r[3], o.z], [1., 1.]), ([r[0], r[3], o.z], [0., 1.])];
                    (o.texture.clone(), o.blend, q.iter().map(|(p, uv)| (*p, *uv, c)).collect())
                }
            };
            if blend == Some(0xa1) {
                scooby_recomp::engine::gfx::bevy_adapter::subblend_log("sprite", tex.as_deref().unwrap_or("-"));
            }
            if verts.is_empty() {
                continue;
            }
            let layer = if world_pos.is_some() { 0 } else { 1 };
            // WP-G1: screen-wide sprites (fades, cinema bars) span the whole Hor+ view
            let (x0, x1) = verts.iter().fold((f32::MAX, f32::MIN), |(a, b), v| (a.min(v.0[0]), b.max(v.0[0])));
            let xs = if x0 <= 0.5 && x1 >= w - 0.5 { ow / w } else { 1.0 };
            let pos: Vec<[f32; 3]> = world_pos.unwrap_or_else(|| verts.iter().map(|(p, _, _)| [(p[0] - w * 0.5) * xs, h * 0.5 - p[1], -p[2].clamp(0.0, 1.0) * 40.0 - n as f32 * 1e-3]).collect());
            let uv: Vec<[f32; 2]> = verts.iter().map(|v| v.1).collect();
            // J23: RpSkyRenderStateSet(2, 0x80_000000xx): FIX = 0x80 = 1.0, so 0xa8 (Cs*FIX) and
            // 0x68 (Cs*FIX+Cd) ignore the vertex alpha (the beam group colour has alpha 0: it was
            // multiplied away by AlphaMode::Add; 0x68 = Premultiplied + alpha 0 = ONE/ONE); 0xa1 = (Cd-Cs)*FIX -> Multiply by 1-Cs, as in
            // particle_batch_mesh. Only 0x44 uses As.
            // P26 R42: the colour bytes are RwIm2DVertex RwRGBA (RenderBatchNormal writes the
            // SetColor colour*255 bytes into the float r,g,b,a slots of RwIm2DRenderPrimitive);
            // the RW sky pipeline sends 255 -> GS 0x80 (as for the world prelit, P25 R75 GS dump),
            // so 255 = 1.0, never above: c/255 like the HUD Im2D and motion-trail Im3D colours.
            // Supersedes R34 (A 0xff = 2.0) and R40 (colour > 0x80 unclamped) for sprites, which
            // made the EP3/L05 dust devil opaque/over-bright and the snack-box glow 2x vs PCSX2.
            // GAME_SPRITE_GS=1 restores the R41 GS-native reading (A/B).
            let gs_native = std::env::var("GAME_SPRITE_GS").is_ok_and(|v| v == "1");
            let col: Vec<[f32; 4]> = verts.iter().map(|v| {
                let c = v.2;
                let cb = [c[0].clamp(0., 255.) as u8, c[1].clamp(0., 255.) as u8, c[2].clamp(0., 255.) as u8, c[3].clamp(0., 255.) as u8];
                let mut g = if gs_native { scooby_recomp::engine::gfx::bevy_adapter::gs_to_linear_blend(cb, blend.unwrap_or(0)) } else { rwrgba_to_linear(cb) };
                match blend {
                    Some(0xa1) => g = [1.0 - g[0], 1.0 - g[1], 1.0 - g[2], 1.0],
                    // P26 R34: EFSpriteGroup_SP::SetColor stores colour*255 bytes, so alpha 1.0 is
                    // vertex A 0xff = 2.0 on the GS (MODULATE Av = Af*At>>7, As/0x80 in the blend):
                    // the dust-devil dust_blur group (colour a 1.0, mode 3 = 0x44) is twice as
                    // opaque as the texture alpha; gs_to_linear's min(1.0) halved it
                    Some(0x44) | None => g[3] = c[3].clamp(0., 255.) / if gs_native { 128.0 } else { 255.0 },
                    Some(0x68) => g[3] = 0.0,
                    _ => g[3] = 1.0,
                }
                g
            }).collect();
            let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
            m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
            m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
            m.insert_attribute(Mesh::ATTRIBUTE_COLOR, col);
            let img = tex.and_then(|t| {
                world.resource_scope(|wd, mut cache: Mut<GfxCache>| {
                    let mut images = wd.resource_mut::<Assets<Image>>();
                    cache.texture(&t, dict, &mut images)
                })
            });
            // P26 R20: EFSpriteMgr_BC::RenderNormal / RenderTransparent / RenderOccluded (0x002737f0..)
            // force RwRenderStateSet(FOGENABLE 0xe, 0) around the groups: no fog on sprites (the fog
            // colour was mixed into the additive EP4/L04 checkpoint glow quad -> a grey rectangle)
            let mat = StandardMaterial { base_color_texture: img.map(|i| i.0), unlit: true, fog_enabled: false, alpha_mode: alpha_for_gs(blend), cull_mode: None, double_sided: true, ..default() };
            let mh = world.resource_mut::<Assets<Mesh>>().add(m);
            // P26 R28: EFSpriteGroup_SP flag 0x10 -> EnableStencilRead(1) around the group (GS DATE)
            let mask = match d {
                SpriteDraw::Batch(b) if b.stencil_read && stencil_on() => world.get_resource::<StencilMask>().map(|m| m.0.clone()),
                _ => None,
            };
            if let (Some(rect), Some(mask)) = (occ_rect, occ_mask.clone()) {
                let ma = world.resource_mut::<Assets<game_fx::OccReadMaterial>>().add(game_fx::OccReadMaterial { base: mat, extension: game_fx::OccRead { mask, rect: Vec4::from_array(rect) } });
                world.spawn((OverlayMesh, Mesh3d(mh), MeshMaterial3d(ma), Transform::IDENTITY, RenderLayers::layer(layer)));
            } else if let Some(mask) = mask {
                let ma = world.resource_mut::<Assets<game_fx::StencilReadMaterial>>().add(game_fx::StencilReadMaterial { base: mat, extension: game_fx::StencilRead { mask } });
                world.spawn((OverlayMesh, Mesh3d(mh), MeshMaterial3d(ma), Transform::IDENTITY, RenderLayers::layer(layer)));
            } else {
                let ma = world.resource_mut::<Assets<StandardMaterial>>().add(mat);
                world.spawn((OverlayMesh, Mesh3d(mh), MeshMaterial3d(ma), Transform::IDENTITY, RenderLayers::layer(layer)));
            }
            n += 1;
        }
    }
}

/// EFMotionTrailMgr_BC strips: world-space tristrips, additive ONE/ONE, z-write off, no cull;
/// the texture state is untouched in the C++ (whatever was bound) -> untextured here.
fn draw_trails(world: &mut World, trails: &[(u32, MTStrip)]) {
    let old: Vec<Entity> = world.query_filtered::<Entity, With<TrailMesh>>().iter(world).collect();
    for e in old {
        world.despawn(e);
    }
    for (_, s) in trails {
        if s.verts.len() < 3 {
            continue;
        }
        let mut m = Mesh::new(PrimitiveTopology::TriangleStrip, RenderAssetUsages::default());
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, s.verts.iter().map(|v| v.pos).collect::<Vec<_>>());
        m.insert_attribute(Mesh::ATTRIBUTE_COLOR, s.verts.iter().map(|v| rwrgba_to_linear(v.rgba)).collect::<Vec<_>>());
        m.insert_indices(Indices::U16(s.indices.clone()));
        let mat = StandardMaterial { unlit: true, alpha_mode: AlphaMode::Add, cull_mode: None, double_sided: true, ..default() };
        let mh = world.resource_mut::<Assets<Mesh>>().add(m);
        let ma = world.resource_mut::<Assets<StandardMaterial>>().add(mat);
        world.spawn((TrailMesh, Mesh3d(mh), MeshMaterial3d(ma), Transform::IDENTITY));
    }
}

/// WP-S4: EFProjectorMgr_BC::Begin -> occurrences; the BSPMapper pipeline's projected-texture pass
/// over the hit world sectors (gfx::projector). Blend: ALPHA_1 0x68 Cs+Cd (0) -> Add, 0xa1 Cd-Cs (1)
/// -> Multiply by the inverted color (approximation), 0x44 (2) -> Blend; RGBA = color * 128 (GS 1.0).
fn draw_projectors(world: &mut World, occ: &[scooby_recomp::engine::gfx::projector::ProjectorOccurence], stat: Option<&scooby_recomp::engine::rw::world::RpWorld>, dict: &scooby_recomp::engine::rw::image::RwTexDictionary) {
    let old: Vec<(Entity, scooby_recomp::engine::gfx::projector::ProjectorOccurence)> = world.query::<(Entity, &ProjectorMesh)>().iter(world).map(|(e, p)| (e, p.0.clone())).collect();
    let mut keep = Vec::new();
    for (e, o) in old {
        if occ.contains(&o) && !keep.contains(&o) {
            keep.push(o);
        } else {
            world.despawn(e);
        }
    }
    let Some(w) = stat else { return };
    for o in occ.iter().filter(|o| !keep.contains(o)) {
        let (pos, uv) = o.mesh(w);
        if std::env::var_os("GAME_DEBUG").is_some() {
            let n = pos.len().max(1) as f32;
            let c: Vec<f32> = (0..3).map(|i| pos.iter().map(|p| p[i]).sum::<f32>() / n).collect();
            eprintln!("projector patch center {c:?}");
            eprintln!("projector occ pos {:?} at {:?} tex {} blend {} tris {}", o.pos, o.ltm.at, o.texture, o.blend, pos.len() / 3);
        }
        if pos.is_empty() {
            continue;
        }
        let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
        let mat = projector_material(world, o, dict);
        let mh = world.resource_mut::<Assets<Mesh>>().add(m);
        let ma = world.resource_mut::<Assets<StandardMaterial>>().add(mat);
        world.spawn((ProjectorMesh(o.clone()), Mesh3d(mh), MeshMaterial3d(ma), Transform::IDENTITY, RenderLayers::layer(0)));
    }
}

/// projected-texture pass material (BSPMapperPipelineDataUpload ALPHA_1 / RGBA)
fn projector_material(world: &mut World, o: &scooby_recomp::engine::gfx::projector::ProjectorOccurence, dict: &scooby_recomp::engine::rw::image::RwTexDictionary) -> StandardMaterial {
    let img = world.resource_scope(|wd, mut cache: Mut<GfxCache>| {
        let mut images = wd.resource_mut::<Assets<Image>>();
        cache.texture(&o.texture, dict, &mut images)
    });
    let c = o.color;
    let (col, alpha) = match o.blend {
        0 => ([c[0], c[1], c[2], c[3]], AlphaMode::Add),
        1 => ([1.0 - c[0], 1.0 - c[1], 1.0 - c[2], c[3]], AlphaMode::Multiply),
        _ => ([c[0], c[1], c[2], c[3]], AlphaMode::Blend),
    };
    StandardMaterial {
        base_color: scooby_recomp::engine::gfx::bevy_adapter::display_color(col[0], col[1], col[2], col[3]),
        base_color_texture: img.map(|i| i.0),
        unlit: true,
        alpha_mode: alpha,
        cull_mode: None,
        double_sided: true,
        depth_bias: 50.0,
        // P26 R22: no fog on the projected-texture pass: fogged, the additive pass added the fog
        // colour over the whole frustum footprint (EP4/L02A: a pale trapezoid on the water around
        // the spot; PCSX2 shows only the spot ellipse). The mapper pass's PRIM/FGE comes from the
        // VU1 micro code (not in the decompiled source); fog off matches the reference.
        fog_enabled: false,
        ..default()
    }
}

/// WP-S4: DFF mapper pass on the projector receivers (logic flag 0x200, goProjectorMgr receiver
/// list): their drawn meshes (DrawListEntity of the owner, world transform) get the projected
/// texture too; rebuilt every frame (receivers move).
#[derive(Component)]
struct ProjectorRecvMesh;
/// P37 R7: last frame's receiver patches (occurrence, positions, uvs) in spawn order, so an unchanged
/// patch keeps its entity / mesh / material instead of a fresh upload every frame (same geometry, same
/// material inputs -> identical output)
#[derive(Resource, Default)]
struct ProjectorRecvCache(Vec<(scooby_recomp::engine::gfx::projector::ProjectorOccurence, Vec<[f32; 3]>, Vec<[f32; 2]>)>);
fn draw_projector_receivers(world: &mut World, occ: &[scooby_recomp::engine::gfx::projector::ProjectorOccurence], recv: &[u32], dict: &scooby_recomp::engine::rw::image::RwTexDictionary) {
    let patches = projector_receiver_patches(world, occ, recv);
    let same = world.get_resource::<ProjectorRecvCache>().is_some_and(|c| c.0 == patches)
        && world.query_filtered::<Entity, With<ProjectorRecvMesh>>().iter(world).count() == patches.len();
    // debug: GAME_PROJ_NOCACHE=1 rebuilds every frame (A/B check of the cache)
    if same && std::env::var_os("GAME_PROJ_NOCACHE").is_none() {
        return;
    }
    let old: Vec<Entity> = world.query_filtered::<Entity, With<ProjectorRecvMesh>>().iter(world).collect();
    for e in old {
        world.despawn(e);
    }
    for (o, pos, uv) in &patches {
        let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos.clone());
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv.clone());
        let mat = projector_material(world, o, dict);
        let mh = world.resource_mut::<Assets<Mesh>>().add(m);
        let ma = world.resource_mut::<Assets<StandardMaterial>>().add(mat);
        world.spawn((ProjectorRecvMesh, Mesh3d(mh), MeshMaterial3d(ma), Transform::IDENTITY, RenderLayers::layer(0)));
    }
    world.insert_resource(ProjectorRecvCache(patches));
}
fn projector_receiver_patches(world: &mut World, occ: &[scooby_recomp::engine::gfx::projector::ProjectorOccurence], recv: &[u32]) -> Vec<(scooby_recomp::engine::gfx::projector::ProjectorOccurence, Vec<[f32; 3]>, Vec<[f32; 2]>)> {
    let mut out = Vec::new();
    if occ.is_empty() || recv.is_empty() {
        return out;
    }
    let mut tris: Vec<[[f32; 3]; 3]> = Vec::new();
    {
        let mut q = world.query::<(&DrawListEntity, &Mesh3d, &GlobalTransform, &Visibility)>();
        // P37 R7: only the receivers' entities (was: every draw-list entity, filtered below)
        let items: Vec<(u32, Handle<Mesh>, GlobalTransform, Visibility)> = q.iter(world).filter(|(d, ..)| recv.contains(&d.owner)).map(|(d, m, g, v)| (d.owner, m.0.clone(), *g, *v)).collect();
        let meshes = world.resource::<Assets<Mesh>>();
        for (owner, mh, gt, vis) in items {
            let (d, mh, gt, vis) = (DrawListEntity { owner, atomic: 0, material: 0, pass: 0 }, Mesh3d(mh), gt, vis);
            let (gt, vis) = (&gt, &vis);
            if !recv.contains(&d.owner) || *vis == Visibility::Hidden {
                continue;
            }
            let Some(m) = meshes.get(&mh.0) else { continue };
            let Some(bevy_mesh::VertexAttributeValues::Float32x3(p)) = m.attribute(Mesh::ATTRIBUTE_POSITION) else { continue };
            let a = gt.affine();
            let wp: Vec<[f32; 3]> = p.iter().map(|v| a.transform_point3(Vec3::from(*v)).to_array()).collect();
            let idx: Vec<usize> = match m.indices() {
                Some(i) => i.iter().collect(),
                None => (0..wp.len()).collect(),
            };
            for t in idx.chunks_exact(3) {
                tris.push([wp[t[0]], wp[t[1]], wp[t[2]]]);
            }
        }
    }
    for o in occ {
        let (pos, uv) = o.mesh_tris(tris.iter().copied());
        if !pos.is_empty() {
            out.push((o.clone(), pos, uv));
        }
    }
    out
}

/// WP-S4: EFFBEffect_FrameDistortion_SP pass (APPROXIMATION). The original grabs the frame buffer
/// (EFFBEffect_SP::GrabRaster) and redraws the effect object's atomics (LOD == params+8) with a VU1
/// micro code that offsets the frame-buffer UVs (matrix params+0xc); that micro code is not in the
/// decompiled source. Here: the object's drawn meshes are redrawn with a Bevy specular-transmission
/// material, which samples the opaque screen texture behind the mesh with refraction (screen grab +
/// redraw); the uv matrix scale drives the refraction strength (thickness).
#[derive(Component)]
struct DistortionMesh;
fn draw_distortions(world: &mut World, fx: &[(u32, u32, [u32; 16])]) {
    let old: Vec<Entity> = world.query_filtered::<Entity, With<DistortionMesh>>().iter(world).collect();
    for e in old {
        world.despawn(e);
    }
    if fx.is_empty() {
        return;
    }
    // P32 R1: Transform = the DrawCmd ltm ApplyDrawList wrote this frame (GlobalTransform is only
    // propagated in PostUpdate: the copy lagged one frame behind a moving owner)
    let mut q = world.query::<(&DrawListEntity, &Mesh3d, &Transform, &Visibility)>();
    let items: Vec<(u32, Handle<Mesh>, Transform)> =
        q.iter(world).filter(|(_, _, _, v)| **v != Visibility::Hidden).map(|(d, m, t, _)| (d.owner, m.0.clone(), *t)).collect();
    for (obj, _lod, m) in fx {
        let f = |i: usize| f32::from_bits(m[i]);
        let uv_matrix = Mat4::from_cols_array(&std::array::from_fn(|i| f(i)));
        // offset scale: one view-normal unit = 4% of the screen (approximation)
        let ma = world.resource_mut::<Assets<game_fx::DistortMaterial>>().add(game_fx::DistortMaterial { uv_matrix, strength: Vec4::new(0.04, 0.0, 0.0, 0.0) });
        for (_, mh, gt) in items.iter().filter(|i| i.0 == *obj) {
            world.spawn((DistortionMesh, Mesh3d(mh.clone()), MeshMaterial3d(ma.clone()), *gt, RenderLayers::layer(0)));
        }
    }
}

/// WP-S5: EFShadowMgr caster pass (gfx::shadow). Caster bounding spheres = the atomics' +0x1c
/// spheres (`caster_bs`: anim clipping bsphere / SetRenderBSphere / clump rest pose); the
/// shadow raster is the CPU silhouette (APPROXIMATION of the VU1 cast) and is drawn with the
/// projector pass on the world sectors (blend 1). GAME_SHADOW_OFF disables.
#[derive(Component)]
struct ShadowMesh;
fn draw_shadows(world: &mut World, cast: &[(u32, u32, [u32; 11], scooby_recomp::engine::camera::DTMatrix, (u16, u32))], caster_bs: &std::collections::HashMap<u32, Vec<([f32; 3], f32)>>, lights: &[scooby_recomp::engine::gfx::shadow::ShadowLight], stat: Option<&scooby_recomp::engine::rw::world::RpWorld>) {
    use scooby_recomp::engine::gfx::shadow::*;
    let old: Vec<Entity> = world.query_filtered::<Entity, With<ShadowMesh>>().iter(world).collect();
    for e in old {
        world.despawn(e);
    }
    let Some(w) = stat else { return };
    if cast.is_empty() || std::env::var_os("GAME_SHADOW_OFF").is_some() {
        return;
    }
    // P37 R3: GAME_PROF sub-stages of the shadow pass (shadow:*)
    let mut t = STAGES_ON.load(std::sync::atomic::Ordering::Relaxed).then(std::time::Instant::now);
    let mut q = world.query::<(&DrawListEntity, &Mesh3d, &GlobalTransform, &Visibility)>();
    let items: Vec<(u32, Handle<Mesh>, GlobalTransform)> =
        q.iter(world).filter(|(_, _, _, v)| **v != Visibility::Hidden).map(|(d, m, g, _)| (d.owner, m.0.clone(), *g)).collect();
    let mut input = Vec::new();
    let mut tris_of = std::collections::HashMap::new();
    {
        let meshes = world.resource::<Assets<Mesh>>();
        for (uid, fl, prm, frame, clip) in cast {
            let mut spheres = Vec::new();
            let mut tris: Vec<[[f32; 3]; 3]> = Vec::new();
            for (_, mh, gt) in items.iter().filter(|i| i.0 == *uid) {
                let Some(m) = meshes.get(mh) else { continue };
                let Some(bevy_mesh::VertexAttributeValues::Float32x3(p)) = m.attribute(Mesh::ATTRIBUTE_POSITION) else { continue };
                let a = gt.affine();
                let wp: Vec<Vec3> = p.iter().map(|v| a.transform_point3(Vec3::from(*v))).collect();
                if wp.is_empty() {
                    continue;
                }
                let (mn, mx) = wp.iter().fold((Vec3::MAX, Vec3::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
                let c = (mn + mx) * 0.5 - Vec3::from(frame.pos);
                spheres.push((c.to_array(), (mx - mn).length() * 0.5));
                let idx: Vec<usize> = match m.indices() {
                    Some(i) => i.iter().collect(),
                    None => (0..wp.len()).collect(),
                };
                for t in idx.chunks_exact(3) {
                    tris.push([wp[t[0]].to_array(), wp[t[1]].to_array(), wp[t[2]].to_array()]);
                }
            }
            if spheres.is_empty() {
                continue;
            }
            // GAME_SHADOW_AABB: the old stand-in (one sphere per drawn mesh AABB)
            if let Some(bs) = caster_bs.get(uid).filter(|b| !b.is_empty() && std::env::var_os("GAME_SHADOW_AABB").is_none()) {
                if std::env::var_os("GAME_DEBUG").is_some() {
                    eprintln!("shadow caster {uid:x} atomic spheres {bs:?} (aabb {spheres:?})");
                }
                spheres = bs.clone();
            }
            tris_of.insert(*uid, tris);
            input.push((*uid, *fl, *prm, *frame, InitCaster(&spheres), *clip));
        }
    }
    stage(&mut t, "shadow:casters");
    let dbg = std::env::var_os("GAME_DEBUG").is_some();
    if dbg {
        eprintln!("shadow casters {} with meshes {} clip {:x?} lights {:x?}", cast.len(), input.len(), cast.iter().map(|c| (c.0, c.4)).collect::<Vec<_>>(), lights.iter().map(|l| (l.0, l.1, l.3, l.4, l.5, l.2.pos)).collect::<Vec<_>>());
    }
    // P26 R4: all the frame's shadow rasters go into one atlas + one material (game_fx::ShadowSubMaterial:
    // overlapping patches stack like the GS Cd - Cs per patch)
    let (aw, ah) = (4 * game_fx::SHADOW_SLOT as usize, 2 * game_fx::SHADOW_SLOT as usize);
    let mut atlas = vec![0u8; aw * ah * 4];
    let mut occ_u = [Vec4::ZERO; 6 * game_fx::SHADOW_MAX];
    let mut patches = Vec::new();
    // debug: GAME_SHADOW_GAIN=x scales the shadow intensity (visual check of the pass)
    let gain: f32 = std::env::var("GAME_SHADOW_GAIN").ok().and_then(|v| v.parse().ok()).unwrap_or(1.0);
    for (si, s) in ShadowBeginLights(&input, lights).into_iter().take(game_fx::SHADOW_MAX).enumerate() {
        let mask = CastSilhouette(&s.occ, s.size, &tris_of[&s.caster]);
        stage(&mut t, "shadow:silhouette");
        // P37 R3: ShadowSubMaterial projects every fragment from its world position (shadow.wgsl, occ
        // uniforms) and ignores the mesh UVs, so the receiver patch needs no SUBxSUB split for affine UVs
        // (projector::mesh_flat; same clipped polygons, 1/64 of the triangles of a point-light patch)
        let (pos, uv) = s.occ.mesh_flat(w);
        stage(&mut t, "shadow:receiver");
        if dbg {
            eprintln!("shadow {:x} par {} pos {:?} vw {} col {} cover {} patch {}", s.caster, s.occ.parallel, s.occ.pos, s.occ.view_window[0], s.occ.color[0], mask.iter().filter(|&&a| a > 0).count(), pos.len() / 3);
            let (lo, hi) = pos.iter().fold(([f32::MAX; 3], [f32::MIN; 3]), |(mut l, mut h), p| { for i in 0..3 { l[i] = l[i].min(p[i]); h[i] = h[i].max(p[i]); } (l, h) });
            let (ul, uh) = uv.iter().fold(([f32::MAX; 2], [f32::MIN; 2]), |(mut l, mut h), p| { for i in 0..2 { l[i] = l[i].min(p[i]); h[i] = h[i].max(p[i]); } (l, h) });
            let n = s.size as usize;
            let (mut ml, mut mh) = ([n; 2], [0usize; 2]);
            for (i, &a) in mask.iter().enumerate() { if a > 0 { let (x, y) = (i % n, i / n); ml = [ml[0].min(x), ml[1].min(y)]; mh = [mh[0].max(x), mh[1].max(y)]; } }
            eprintln!("shadow uv {ul:?}..{uh:?} mask px {ml:?}..{mh:?} of {n}");
            eprintln!("shadow patch {lo:?}..{hi:?} light {:?} at {:?} near {:?}", s.occ.ltm.pos, s.occ.ltm.at, s.occ.near);
            // GAME_SHADOW_DUMP=<dir>: write each shadow raster as <dir>/<caster>_<n>.pgm (overwritten every frame)
            if let Some(d) = std::env::var_os("GAME_SHADOW_DUMP") {
                let k = (s.occ.ltm.pos[0] * 10.0) as i32;
                let mut f = format!("P5 {n} {n} 255\n").into_bytes();
                f.extend_from_slice(&mask);
                let _ = std::fs::write(std::path::Path::new(&d).join(format!("{:08x}_{k}.pgm", s.caster)), f);
            }
        }
        // P26 B2 z-fighting: the receiver patch is the world sector triangles themselves; the
        // constant depth bias alone is too small at a distance (float depth): lift each triangle
        // 1 cm along its normal towards the light (the side the shadow is seen from)
        let mut pos = pos;
        let lp = Vec3::from(s.occ.ltm.pos);
        for t in pos.chunks_exact_mut(3) {
            let (a, b, c) = (Vec3::from(t[0]), Vec3::from(t[1]), Vec3::from(t[2]));
            let mut n = (b - a).cross(c - a).normalize_or_zero();
            if n.dot(lp - a) < 0.0 {
                n = -n;
            }
            for v in t.iter_mut() {
                *v = (Vec3::from(*v) + n * 0.01).to_array();
            }
        }
        // shadow raster (cleared black, caster drawn 0x80ffffff = white) -> projected with
        // RGBA = colour*128 and ALPHA_1 0xa1: Cd - tex*colour (game_fx::ShadowSubMaterial)
        let mask: Vec<u8> = if std::env::var_os("GAME_SHADOW_SOLID").is_some() { vec![255; mask.len()] } else { mask };
        let n = s.size as usize;
        let (ox, oy) = ((si % 4) * game_fx::SHADOW_SLOT as usize, (si / 4) * game_fx::SHADOW_SLOT as usize);
        for y in 0..n {
            for x in 0..n {
                let a = mask[y * n + x];
                atlas[((oy + y) * aw + ox + x) * 4..][..4].copy_from_slice(&[a, a, a, 255]);
            }
        }
        let o = &s.occ;
        // P26 R7 (VU1 projectormapPRS, tools/vu1dis.py): every vertex's RGBAQ = FTOI0(colour*128), constant
        let (m, c) = (o.ltm, (o.color[0] * gain * 128.0).clamp(0.0, 255.0).trunc());
        occ_u[6 * si] = Vec4::new(m.right[0], m.right[1], m.right[2], o.view_window[0]);
        occ_u[6 * si + 1] = Vec4::new(m.up[0], m.up[1], m.up[2], if o.parallel { 1.0 } else { 0.0 });
        occ_u[6 * si + 2] = Vec4::new(m.at[0], m.at[1], m.at[2], s.size as f32 / game_fx::SHADOW_SLOT as f32);
        occ_u[6 * si + 3] = Vec4::new(m.pos[0], m.pos[1], m.pos[2], c);
        occ_u[6 * si + 4] = Vec4::from_array(o.near);
        occ_u[6 * si + 5] = Vec4::from_array(o.far_plane);
        patches.push((pos, uv));
        stage(&mut t, "shadow:atlas");
    }
    if patches.is_empty() {
        return;
    }
    let img = Image::new(
        bevy::render::render_resource::Extent3d { width: aw as u32, height: ah as u32, depth_or_array_layers: 1 },
        bevy::render::render_resource::TextureDimension::D2,
        atlas,
        bevy::render::render_resource::TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    let ih = world.resource_mut::<Assets<Image>>().add(img);
    let mat = game_fx::ShadowSubMaterial { occ: occ_u, count: Vec4::new(patches.len() as f32, scooby_recomp::engine::gfx::bevy_adapter::display_space() as u32 as f32, 0.0, 0.0), mask: ih };
    let ma = world.resource_mut::<Assets<game_fx::ShadowSubMaterial>>().add(mat);
    for (pos, uv) in patches {
        if pos.is_empty() {
            continue;
        }
        let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
        let mh = world.resource_mut::<Assets<Mesh>>().add(m);
        world.spawn((ShadowMesh, Mesh3d(mh), MeshMaterial3d(ma.clone()), Transform::IDENTITY, RenderLayers::layer(0)));
    }
    stage(&mut t, "shadow:upload");
}

fn render_frame(world: &mut World) {
    if !world.resource::<Game>().loaded {
        return;
    }
    // F1: EFGraphicsMgr::Render runs once per MainLoop pass: rebuild the draw list only after a new
    // runtime frame (on a >60 Hz display the port re-ran the whole draw-list build every update)
    if !std::mem::take(&mut world.resource_mut::<Game>().fresh) {
        return;
    }
    let t0 = std::time::Instant::now();
    render_frame_inner(world);
    let mut g = world.resource_mut::<Game>();
    if g.prof.enabled() {
        Prof::add(&mut g.prof.render, t0.elapsed().as_secs_f64() * 1e3);
    }
}

fn render_frame_inner(world: &mut World) {
    world.resource_scope(|world, mut game: Mut<Game>| {
        let g = &mut *game;
        let mut t = STAGES_ON.load(std::sync::atomic::Ordering::Relaxed).then(std::time::Instant::now);
        draw_objects(g);
        stage(&mut t, "draw_objects");
        // P28 R12 (B10): EFTexFader_SP::FadeAtomic posts EFNotificationMgr::Notify(0x10, owner, owner, arg, 0)
        // (immediate) when a fade with flags & 1 ends; the queue was never drained, so the panel's
        // NOTIFY_TEXFADER never ran: courage face +0x45c never took the target +0x460 and the fader-idle
        // flag +0x30 stayed 0 (portrait frozen after the first fade). HUD window objects map back to
        // their game object (owner HUD_OWNER + k).
        for (ty, o, _, arg) in std::mem::take(&mut g.gmgr.notifications) {
            let uid = if o >= HUD_OWNER { g.hud_objs.get((o - HUD_OWNER) as usize).map_or(0, |h| h.0.0) } else { o };
            if let Some(h) = g.e.get_game_object(uid, 0) {
                let id = g.e.rt.obj(h).id;
                g.e.Notify(ty, id, id, arg, false);
            }
        }
        let rd = g.render.as_ref().unwrap();
        let p = g.rt.particles.lock().unwrap();
        let obj3d = p.sub.obj3d.clone();
        drop(p);
        let hres: Vec<u32> = g.hud_objs.iter().map(|h| h.0.1).collect();
        let look = |o: u32| {
            let res = if o >= HUD_OWNER { *hres.get((o - HUD_OWNER) as usize)? } else { obj3d.get(&o)?.0 };
            rd.res.iter().find(|r| r.res.uid == res).and_then(|r| r.clump.as_ref()).map(|c| (res, c))
        };
        // WP-S4: SetLogicFlags 0x100 -> EF3dObj::LightObject: rpGEOMETRYLIGHT on/off for the owner
        {
            // EFGameObject::Initialize (EFGameObject.cpp:358): +0x15c & 0x100 clear -> LightObject(0) on
            // every 3d object (prelit only) until a SetLogicFlags 0x100 transition (`lit`) changes it.
            // Without a transition the current flag bit equals the Initialize-time one.
            let lit = g.rt.particles.lock().unwrap().s1.lit.clone();
            let unlit: std::collections::HashSet<u32> = g.e.rt.objects.iter().flatten().filter(|o| o.logic_flags & 0x100 == 0).map(|o| o.id).collect();
            for d in g.gmgr.draws.iter_mut() {
                let on = match lit.get(&d.owner) {
                    Some(&on) => Some(on),
                    None if unlit.contains(&d.owner) => Some(false),
                    None => None,
                };
                if let Some(on) = on {
                    d.geometry_flags = if on { d.geometry_flags | 0x20 } else { d.geometry_flags & !0x20 };
                }
            }
        }
        stage(&mut t, "notify_litflags");
        // P26 R16: RW lights = the level's static lights + the visible dynamic world lights
        if let Some(st) = rd.stat.as_ref() {
            // P26 R17: the static lights as the light manager has them (logic-recoloured, EFStatic3dObj::ReadLights)
            let all = RwLights::from_static_rt(&st.lights, &g.rt.lights.lights).with_dynamic(&g.rt.lights.lights);
            // world sectors: the rpLIGHTLIGHTWORLD ones (RpWorldSectorPS2AllLightingSetupFunc), atomics: rpLIGHTLIGHTATOMICS
            relight_static_world(world, st, &all.only(2));
            world.insert_resource(all.only(1));
            world.insert_resource(RwHudLights { from_owner: HUD_OWNER, lights: RwLights::from_hud(&g.rt.lights.lights) });
        }
        stage(&mut t, "lights");
        scooby_recomp::engine::gfx::bevy_adapter::DL_PROF.store(t.is_some(), std::sync::atomic::Ordering::Relaxed);
        ApplyDrawList(world, &g.gmgr.draws, &look, &g.rt.gfx.tex_dict);
        stage(&mut t, "draw_list");
        if t.is_some() {
            stage_dl_sub();
        }
        // P26 R28: EnableStencilWrite owners (logic flag 0x8000) also go to the stencil-mask camera
        let sw: HashSet<u32> = g.gmgr.draws.iter().filter(|d| d.stencil_write).map(|d| d.owner).collect();
        // J7: the EFHud3DWindow clumps go to the HUD camera layer
        let mut q = world.query::<(&DrawListEntity, &mut RenderLayers)>();
        for (d, mut l) in q.iter_mut(world) {
            let mut want = RenderLayers::layer(if d.owner >= HUD_OWNER { 2 } else if g.distort_only.contains(&d.owner) { 31 } else { 0 });
            if stencil_on() && want.intersects(&RenderLayers::layer(0)) && sw.contains(&d.owner) {
                want = want.with(STENCIL_LAYER);
            }
            // P37 R4: assign only on a change (DerefMut marks the component changed every frame)
            if *l != want {
                *l = want;
            }
        }
        let missing: Vec<(Entity, u32)> = world.query_filtered::<(Entity, &DrawListEntity), Without<RenderLayers>>().iter(world).map(|(e, d)| (e, d.owner)).collect();
        for (e, o) in missing {
            let l = RenderLayers::layer(if o >= HUD_OWNER { 2 } else if g.distort_only.contains(&o) { 31 } else { 0 });
            let l = if stencil_on() && o < HUD_OWNER && !g.distort_only.contains(&o) && sw.contains(&o) { l.with(STENCIL_LAYER) } else { l };
            world.entity_mut(e).insert(l);
        }
        stage(&mut t, "layers");
        hud_z16(world);
        stencil_occluders(world);
        stage(&mut t, "z16_stencil");
        // camera 0 (EFGraphicsMgr::RenderCamera) -> Bevy camera
        if let Some(c) = g.rt.gfx.cameras.cameras.iter().flatten().next() {
            let mut t = rw_camera_transform(&c.ltm);
            if std::env::var_os("GAME_CAMLOG").is_some() && g.steps % 30 == 0 {
                eprintln!("game: CAMLOG f{} cam {:?} hero {:?}", g.steps, c.ltm, pos_of(g, leader(g)));
            }
            // debug: GAME_CLOSEUP="dx,dy,dz" views the book from book pos + (dx,dy,dz) (world)
            if let Some(v) = std::env::var("GAME_CLOSEUP").ok() {
                let d: Vec<f32> = v.split(',').filter_map(|x| x.parse().ok()).collect();
                let target = std::env::var("GAME_CLOSEUP_OBJ").ok().and_then(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16).ok()).unwrap_or(g.book);
                // GAME_CLOSEUP_POS="x,y,z": absolute target point instead of an object
                let abs: Option<[f32; 3]> = std::env::var("GAME_CLOSEUP_POS").ok().and_then(|v| {
                    let p: Vec<f32> = v.split(',').filter_map(|x| x.parse().ok()).collect();
                    (p.len() == 3).then(|| [p[0], p[1], p[2]])
                });
                let bp = abs.or_else(|| g.rt.particles.lock().unwrap().world.frames.get(&target).map(|m| m.pos));
                if let (Some(b), 3) = (bp, d.len()) {
                    t = rw_camera_transform(&look_at([b[0] + d[0], b[1] + d[1], b[2] + d[2]], b));
                }
            }
            let proj = rw_camera_projection(&c.rw);
            let mut q = world.query_filtered::<(&mut Transform, &mut Projection), With<GameCam>>();
            if let Some((mut tr, mut pr)) = q.iter_mut(world).next() {
                *tr = t;
                *pr = proj;
            }
            sync_stencil_cam(world);
            // EFGraphicsMgr_SP::SetRenderState(camera 0): camera fog -> DistanceFog on the game camera
            g.gmgr.SetRenderState(c);
            if let Some(e) = world.query_filtered::<Entity, With<GameCam>>().iter(world).next() {
                ApplyFog(world, e, &g.gmgr.fog, &c.rw);
            }
            let ps: Vec<_> = g.rt.frame.particles.iter().map(|b| b.1.clone()).collect();
            let bs: Vec<_> = g.rt.frame.billboards.iter().map(|b| b.1.clone()).collect();
            ApplyParticles(world, &ps, &bs, &c.ltm, &g.rt.gfx.tex_dict);
            stencil_particles(world, &ps, &bs);
            sub_particles(world, &ps, &bs);
        }
        stage(&mut t, "camera_particles");
        let rwcam = g.rt.gfx.cameras.cameras.iter().flatten().next().map(|c| (c.ltm.clone(), c.rw.view_window));
        draw_sprites(world, &g.rt.frame.sprites, &g.rt.gfx.tex_dict, rwcam.as_ref().map(|(l, v)| (l, *v)));
        draw_trails(world, &g.rt.frame.trails);
        stage(&mut t, "sprites_trails");
        let occ = {
            let mut p = g.rt.particles.lock().unwrap();
            // debug: GAME_PROJ_FORCE forces every projector visible (visual check of the pass)
            if std::env::var_os("GAME_PROJ_FORCE").is_some() {
                p.s1.projectors.projectors.values_mut().for_each(|pr| pr.flags |= 1);
            }
            (scooby_recomp::engine::gfx::projector::ProjectorMgrBegin(&p.s1.projectors, |h| p.world.frames.get(&h).copied()), p.s1.proj_receivers.clone())
        };
        let (occ, recv) = occ;
        stage(&mut t, "proj:begin");
        draw_projectors(world, &occ, rd.stat.as_ref().and_then(|s| s.world.as_ref()), &g.rt.gfx.tex_dict);
        stage(&mut t, "proj:world");
        draw_projector_receivers(world, &occ, &recv, &g.rt.gfx.tex_dict);
        stage(&mut t, "projectors");
        // EFShadowMgr::Begin(lights, viewport): the caster's +0x114 / +0x110 words of the rendering camera
        // (the first camera, as everywhere in the renderer); constructor value 0 = not in the frustum
        let slot = g.rt.gfx.cameras.cameras.iter().flatten().next().map(|c| (c.uid & 0xfff) as usize).unwrap_or(0);
        // EFShadowMgr_SP::InitCaster sums the atomics' +0x1c bounding spheres (geometry bsphere, frame-local);
        // per caster the clump's rest-pose atomic spheres (render::clip_spheres, cached per resource)
        let mut caster_bs: std::collections::HashMap<u32, Vec<([f32; 3], f32)>> = Default::default();
        let cast = {
            let mut p = g.rt.particles.lock().unwrap();
            for u in p.s1.shadow_casters.clone() {
                // atomic +0x1c as last set by EFAnimNode::SetCollPrimData / SetRenderBSphere (one sphere on every atomic)
                if let Some(&(c, r)) = p.s1.anim_bsphere.get(&u).or(p.s1.render_bsphere.get(&u)) {
                    caster_bs.insert(u, vec![(c, r)]);
                    continue;
                }
                let Some(&(res_uid, _)) = p.sub.obj3d.get(&u) else { continue };
                if !p.s1.clip_spheres.contains_key(&res_uid) {
                    let Some(c) = rd.res.iter().find(|r| r.res.uid == res_uid).and_then(|r| r.clump.as_ref()) else { continue };
                    p.s1.clip_spheres.insert(res_uid, scooby_recomp::engine::gfx::render::clip_spheres(res_uid, c));
                }
                caster_bs.insert(u, p.s1.clip_spheres[&res_uid].iter().map(|s| ([s[0], s[1], s[2]], s[3])).collect());
            }
            p.s1.shadow_casters.iter().filter_map(|&u| {
                let clip = *p.s1.clip_cam.get(&(u, slot)).unwrap_or(&(0, 0));
                Some((u, *p.s1.caster_flags.get(&u).unwrap_or(&0x400), *p.s1.shadow.get(&u).unwrap_or(&scooby_recomp::engine::host::gameobj::SHADOW_DEFAULT), *p.world.frames.get(&u)?, clip))
            }).collect::<Vec<_>>()
        };
        // EFLightMgr_BC::GetLightList: the manager's lights (static ones from EFStatic3dObj::ReadLights,
        // frame = the stored matrix; dynamic ones from APIlight). P26 R17: read from EngineRuntime::lights
        // (outside the logic step the manager is swapped out of J2: SwapLogicState, the J2 list was empty)
        // with the logic's current flags / visibility (was: the streamed static lights, always visible).
        let lights: Vec<scooby_recomp::engine::gfx::shadow::ShadowLight> = g.rt.lights.lights.iter().filter_map(|x| {
            let r = x.rp.as_ref()?;
            let f = x.frame.or_else(|| rd.stat.as_ref()?.lights.iter().find(|l| l.uid == x.uid).map(|l| {
                let m = l.matrix;
                scooby_recomp::engine::camera::DTMatrix { right: [m[0], m[1], m[2]], up: [m[3], m[4], m[5]], at: [m[6], m[7], m[8]], pos: [m[9], m[10], m[11]] }
            }))?;
            Some((r.ty, r.flags, f, r.radius, x.kind, x.IsVisible()))
        }).collect();
        stage(&mut t, "shadow_prep");
        draw_shadows(world, &cast, &caster_bs, &lights, rd.stat.as_ref().and_then(|s| s.world.as_ref()));
        stage(&mut t, "draw_shadows");
        let fx = {
            let p = g.rt.particles.lock().unwrap();
            p.s1.fb.Distortions(|u| p.world.frames.contains_key(&u))
        };
        // debug: GAME_FBFX_FORCE=<object uid hex> adds a FrameDistortion on that object (uv matrix
        // scale 1.3) for a visual check of the pass
        let mut fx = fx;
        if let Some(u) = std::env::var("GAME_FBFX_FORCE").ok().and_then(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16).ok()) {
            let k = 1.3f32.to_bits();
            fx.push((u, 0, [k, 0, 0, 0, 0, k, 0, 0, 0, 0, k, 0, 0, 0, 0, 0x3f80_0000]));
        }
        draw_distortions(world, &fx);
        stage(&mut t, "distortions");
        let fonts: Vec<Option<Vec<String>>> = g.rt.particles.lock().unwrap().hud.fonts.iter().map(|f| f.as_ref().map(|f| f.textures.clone())).collect();
        let mut hud = g.rt.frame.hud.clone();
        #[cfg(feature = "solari")]
        rtgi_options_row(world, &mut hud, &g.rt.particles.lock().unwrap().hud.fonts);
        // P34: PC Settings entry row / page (Options menu)
        game_settings::draw(&game_settings::PAGE.lock().unwrap(), &mut hud, &g.rt.particles.lock().unwrap().hud.fonts, SCREEN);
        draw_hud(world, &hud, &fonts, &g.rt.gfx.tex_dict);
        stage(&mut t, "hud");
    });
}

// ---------------------------------------------------------------------------------------------
// audio: EFNewAudioMgr voices -> Bevy audio entities (SPU2 replacement, see audio/bevy_adapter)

/// J7: STREAM bank (VAG) loader thread: the IOP streamed STREAM\..\*.VAG from the disc while the
/// voice played (SOUND_PlayStream); here the file is read on a worker thread (no I/O in systems)
/// and the voice starts once its bytes arrived; A3: the ADPCM is decoded incrementally while the
/// voice plays (spu::AdpcmStream via stream::load_stream_adpcm), keeping the SPU loop points.
struct StreamLoader {
    req: std::sync::mpsc::Sender<scooby_recomp::engine::audio::mgr::Voice>,
    done: Mutex<Receiver<(String, Option<(u32, std::sync::Arc<[u8]>)>)>>,
    /// path -> (rate, ADPCM body); None = requested / failed
    cache: HashMap<String, Option<(u32, std::sync::Arc<[u8]>)>>,
}

impl StreamLoader {
    fn new(iso: String) -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<scooby_recomp::engine::audio::mgr::Voice>();
        let (dtx, drx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let Ok(fs) = IsoFs::open(&iso) else { return };
            for v in rx {
                let path = v.stream.clone().unwrap_or_default();
                let r = scooby_recomp::engine::audio::stream::load_stream_adpcm(&v, &mut |p| fs.read(p).ok());
                if dtx.send((path, r)).is_err() {
                    return;
                }
            }
        });
        StreamLoader { req: tx, done: Mutex::new(drx), cache: HashMap::new() }
    }
}

/// A3: one SPU2 voice side as a Bevy audio source (REPLACEMENT for the SPU2 voice: spu::SpuVoice
/// plays the head once then loops from the ADPCM loop start, ADSR over the voice life, start delay
/// once before key-on). Side 0 = left-only, 1 = right-only stereo; the sink volume is VOLL/VOLR.
#[derive(Asset, TypePath, Clone)]
struct SpuClip {
    voice: SpuVoice,
    rate: u32,
    side: u8,
}

struct SpuDec {
    v: SpuVoice,
    rate: std::num::NonZero<u32>,
    side: u8,
    pend: Option<f32>,
}

impl Iterator for SpuDec {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if let Some(x) = self.pend.take() {
            return Some(x);
        }
        let s = self.v.next()? as f32 / 32768.0;
        let (l, r) = if self.side == 0 { (s, 0.0) } else { (0.0, s) };
        self.pend = Some(r);
        Some(l)
    }
}

impl bevy_audio::Source for SpuDec {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> bevy_audio::ChannelCount {
        std::num::NonZero::new(2).unwrap()
    }
    fn sample_rate(&self) -> bevy_audio::SampleRate {
        self.rate
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

impl bevy_audio::Decodable for SpuClip {
    type Decoder = SpuDec;
    fn decoder(&self) -> SpuDec {
        SpuDec { v: self.voice.clone(), rate: std::num::NonZero::new(self.rate.max(1)).unwrap(), side: self.side, pend: None }
    }
}

/// P34 PC category of a voice: interleaved stream = music, other stream = voice, bank = SFX.
/// P36 R5: voices of the original Options "movies" group (0x300: CS*_*.INT in-engine cut-scene
/// streams, interleaved like music) follow the PC "Voice / movies" volume, not Music.
fn pc_cat(group: Option<u32>, stream: bool, interleaved: bool) -> game_settings::Cat {
    use game_settings::Cat;
    match scooby_recomp::engine::audio::device::pc_category(group, stream, interleaved) {
        0 => Cat::Music,
        1 => Cat::Sfx,
        _ => Cat::Voice,
    }
}

/// P36 R2: the decoders of a voice's two sides (A4: interleaved .INT music: L = channel 0, R = 1)
fn spu_sides(v: &scooby_recomp::engine::audio::mgr::Voice, data: SpuData) -> [SpuVoice; 2] {
    [0u8, 1].map(|side| match &data {
        SpuData::Adpcm(d) if scooby_recomp::engine::audio::spu::is_interleaved(d) => SpuVoice::new(SpuData::Interleaved(d.clone(), side), v.sample_rate, v.pitch, v.adsr, v.looped, v.delay),
        _ => SpuVoice::new(data.clone(), v.sample_rate, v.pitch, v.adsr, v.looped, v.delay),
    })
}

/// P36 R2: the capture bus of a voice (data format split, as the P34 PC category before R5)
fn cat_bus(d: &SpuData) -> scooby_recomp::engine::audio::capture::Bus {
    use scooby_recomp::engine::audio::capture::Bus;
    match d {
        SpuData::Adpcm(a) if scooby_recomp::engine::audio::spu::is_interleaved(a) => Bus::Music,
        SpuData::Interleaved(..) => Bus::Music,
        SpuData::Adpcm(_) => Bus::Voice,
        _ => Bus::Sfx,
    }
}

/// P36 R2: `GAME_AUDIO_WAV=path`: silent offline mix of every voice (+ FMV audio) into a WAV, written
/// at exit with per-bus levels (stderr + path.txt). Works headless (no audio device involved).
static CAPTURE: std::sync::Mutex<Option<scooby_recomp::engine::audio::capture::Capture>> = std::sync::Mutex::new(None);
static CAP_STEPS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
/// serials >= this are FMV soundtracks (kept until the movie ends)
const CAP_MOVIE: u32 = 0xffff_0000;

fn capture_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("GAME_AUDIO_WAV").is_some())
}

fn capture_with<R>(f: impl FnOnce(&mut scooby_recomp::engine::audio::capture::Capture) -> R) -> Option<R> {
    if !capture_on() {
        return None;
    }
    let mut c = CAPTURE.lock().unwrap_or_else(|e| e.into_inner());
    Some(f(c.get_or_insert_with(Default::default)))
}

fn capture_finish() {
    let Some(path) = std::env::var_os("GAME_AUDIO_WAV") else { return };
    let Some((bytes, rep)) = capture_with(|c| (c.wav_bytes(), c.report())) else { return };
    let path = std::path::PathBuf::from(path);
    let _ = std::fs::write(&path, bytes);
    let _ = std::fs::write(path.with_extension("txt"), &rep);
    eprint!("{}", rep.lines().take(6).map(|l| format!("{l}\n")).collect::<String>());
    eprintln!("capture: wrote {}", path.display());
}

/// J7/A3: a voice as two entities (left-only / right-only) at unit gain; play_audio sets each
/// sink's volume to Voice::left/right every frame (SPU2 VOLL/VOLR). `k` = PC category volume.
fn spawn_spu_voice(cmd: &mut Commands, src: &mut Assets<SpuClip>, v: &scooby_recomp::engine::audio::mgr::Voice, data: SpuData, k: f32) -> [Entity; 2] {
    let sides = spu_sides(v, data);
    [(0u8, v.left), (1, v.right)].map(|(side, g)| {
        let voice = sides[side as usize].clone();
        let h = src.add(SpuClip { voice, rate: v.sample_rate, side });
        cmd.spawn((bevy_audio::AudioPlayer(h), bevy_audio::PlaybackSettings::DESPAWN.with_speed(scooby_recomp::engine::audio::spu::rodio_safe_speed(v.sample_rate, v.pitch)).with_volume(bevy_audio::Volume::Linear(if sound_on() { (g * k).clamp(0.0, 1.0) } else { 0.0 })), VoiceHandle(v.serial), VoiceSide(side))).id()
    })
}

/// P36 R5: audio device loss / reconnect (engine::audio::device policy) on bevy_audio's output
/// (vendor/bevy_audio patch: public AudioOutput, cpal DeviceNotAvailable/StreamInvalidated flag).
/// `GAME_AUDIO_FAKE_LOSS=N` simulates an unplug at frame N (test hook). True while a stream is open.
/// P36 R6: output-stream generation, +1 per successful reopen (the FMV soundtrack restarts on a new one)
static AUDIO_GEN: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

fn audio_device_watch(world: &mut World, steps: u32) -> bool {
    use scooby_recomp::engine::audio::device::{DeviceAction, DeviceWatch};
    static WATCH: std::sync::Mutex<Option<(std::time::Instant, DeviceWatch)>> = std::sync::Mutex::new(None);
    static FAKE_DONE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    let Some(mut out) = world.get_resource_mut::<bevy_audio::AudioOutput>() else { return true };
    let fake = std::env::var("GAME_AUDIO_FAKE_LOSS").ok().and_then(|v| v.parse::<u32>().ok()).is_some_and(|f| steps >= f)
        && !FAKE_DONE.swap(true, std::sync::atomic::Ordering::Relaxed);
    let mut w = WATCH.lock().unwrap_or_else(|e| e.into_inner());
    let (t0, w) = w.get_or_insert_with(|| (std::time::Instant::now(), DeviceWatch::default()));
    match w.step(t0.elapsed().as_secs_f64(), out.is_available(), out.is_lost() || fake) {
        DeviceAction::Close => {
            eprintln!("audio: output device lost at f{steps}{}; game continues silently, retrying", if fake { " (GAME_AUDIO_FAKE_LOSS)" } else { "" });
            out.close();
        }
        DeviceAction::Reopen => {
            let ok = out.reopen();
            w.reopened(ok);
            if ok {
                AUDIO_GEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                eprintln!("audio: output device reopened at f{steps} (attempt {}, reconnect {})", w.attempts, w.reconnects);
            }
        }
        DeviceAction::None => {}
    }
    // P36 R8: device underruns (late pulls, bevy_audio UnderrunProbe): log each new one, plus a
    // summary every 3600 steps (60 s) once the stream has played
    {
        use std::sync::atomic::Ordering::Relaxed;
        static SEEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        // R12: once per summary step (steps stays 0 for the whole load / boot movies: it flooded the log)
        static LAST: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(u32::MAX);
        let u = &bevy_audio::UNDERRUN;
        let late = u.late.load(Relaxed);
        if late != SEEN.swap(late, Relaxed) || (steps % 3600 == 0 && u.frames.load(Relaxed) > 0 && LAST.swap(steps, Relaxed) != steps) {
            let rate = u.rate.load(Relaxed).max(1);
            eprintln!("audio: playout f{steps}: {:.1} s played, {late} underruns (worst {:.1} ms), buffer {} frames = {:.1} ms",
                u.frames.load(Relaxed) as f64 / rate as f64, u.worst_us.load(Relaxed) as f64 / 1000.0,
                u.buffer_frames.load(Relaxed), u.buffer_frames.load(Relaxed) as f64 * 1000.0 / rate as f64);
        }
    }
    out.is_available()
}

/// P36 R12: `GAME_HEADLESS_AUDIO=1`: headless run with the real audio device (soak test hook)
fn headless_audio() -> bool {
    std::env::var_os("GAME_HEADLESS_AUDIO").is_some_and(|v| v == "1")
}

fn play_audio(world: &mut World) {
    // P36 R2: the offline mix runs in lock-step with the game clock (also during movies / loads)
    if capture_on() {
        let steps = world.resource::<Game>().steps;
        capture_with(|c| c.advance(steps.saturating_sub(CAP_STEPS.swap(steps, std::sync::atomic::Ordering::Relaxed)) as f64 * STEP as f64));
    }
    if !(world.resource::<Game>().audio || capture_on()) || !world.resource::<Game>().loaded {
        return;
    }
    world.resource_scope(|world, mut game: Mut<Game>| {
        let g = &mut *game;
        let sl = &mut g.streams;
        while let Ok((p, r)) = sl.done.lock().unwrap().try_recv() {
            if r.is_none() {
                eprintln!("snd: stream load failed {p}");
            }
            sl.cache.insert(p, r);
        }
        let p = g.rt.particles.lock().unwrap();
        let a = &p.sub.audio;
        // P32 R1 debug: GAME_SNDSTAT=N dumps the sound manager every N frames (silence hunts)
        if let Some(n) = std::env::var("GAME_SNDSTAT").ok().and_then(|v| v.parse::<u32>().ok()).filter(|&n| n > 0) {
            if g.steps % n == 0 {
                let c = |f: &dyn Fn(&scooby_recomp::engine::audio::mgr::Voice) -> bool| a.voices.iter().filter(|v| f(v)).count();
                eprintln!(
                    "snd: stat f{} vol g{} fx{} st{} voices {} paused {} virt {} looped {} spu {}/48 host {} handles {} streams-failed {}",
                    g.steps, a.global_volume, a.fx_volume, a.stream_volume, a.voices.len(), c(&|v| v.paused), c(&|v| v.virt), c(&|v| v.looped),
                    a.spu.iter().filter(|x| **x).count(), g.voices.len(), a.handles.iter().filter(|h| h.used).count(),
                    sl.cache.values().filter(|x| x.is_none()).count()
                );
                for v in a.voices.iter().filter(|v| v.paused || v.looped || v.virt || v.stream.is_some()) {
                    eprintln!("snd:   h{} obj {:#x} ev {:#x} paused {} virt {} looped {} pitch {} l {:.3} r {:.3} stream {:?}", v.handle, v.owner, v.event_id, v.paused, v.virt, v.looped, v.pitch, v.left, v.right, v.stream);
                }
                let gv: Vec<_> = a.group_volume.iter().enumerate().filter(|(_, x)| **x != 1.0).collect();
                if !gv.is_empty() {
                    eprintln!("snd:   group volumes != 1: {gv:?}");
                }
            }
        }
        let mut live = HashSet::new();
        let mut gains = HashMap::new();
        let mut new = Vec::new();
        let mut paused = HashSet::new();
        let mut names: HashMap<u32, String> = HashMap::new();
        let mut cats: HashMap<u32, f32> = HashMap::new();
        let mut grp: HashMap<u32, Option<u32>> = HashMap::new();
        let cap = capture_on();
        let known = |voices: &HashMap<u32, [Entity; 2]>, k: u32| if cap { capture_with(|c| c.has(k)).unwrap_or(false) } else { voices.contains_key(&k) };
        for v in &a.voices {
            if v.virt {
                continue;
            }
            let key = v.serial;
            // P32 R9: SOUND_Pause/Resume hold the SPU voice (and the IOP stream) at its position; the
            // host used to despawn paused voices, so music/dialogue restarted from the top on resume.
            if v.paused {
                if known(&g.voices, key) {
                    live.insert(key);
                    paused.insert(v.serial);
                }
                continue;
            }
            live.insert(key);
            // P26 R23: speed snapped like at spawn (stream voices play at the loaded file's rate)
            let rate = v.stream.as_ref().and_then(|p| sl.cache.get(p)).and_then(|x| x.as_ref()).map_or(v.sample_rate, |x| x.0);
            // P34: PC volume of the voice's category (interleaved stream = music, other stream = voice, bank = SFX)
            let interleaved = v.stream.as_ref().and_then(|p| sl.cache.get(p)).and_then(|x| x.as_ref()).map(|(_, d)| scooby_recomp::engine::audio::spu::is_interleaved(d));
            let k = game_settings::gain(pc_cat(a.voice_group(v), v.stream.is_some(), interleaved.unwrap_or(false)));
            gains.insert(v.serial, (v.left * k, v.right * k, scooby_recomp::engine::audio::spu::rodio_safe_speed(rate, v.pitch)));
            cats.insert(v.serial, k);
            if cap {
                grp.insert(v.serial, a.voice_group(v));
            }
            if known(&g.voices, key) {
                continue;
            }
            if let Some(path) = &v.stream {
                match sl.cache.get(path) {
                    Some(Some((rate, adpcm))) => {
                        let mut v = v.clone();
                        v.sample_rate = *rate;
                        new.push((key, v, SpuData::Adpcm(adpcm.clone())));
                    }
                    Some(None) => {}
                    None => {
                        sl.cache.insert(path.clone(), None);
                        let _ = sl.req.send(v.clone());
                    }
                }
            } else if let Some(Some(pcm)) = a.banks.get(v.bank as usize).and_then(|b| b.as_ref()).and_then(|b| b.pcm.get(v.sample as usize)) {
                if cap {
                    // P36 R4: bank sample name for the GAME_SNDLOG key-on line (SFX coverage checks)
                    if let Some(n) = a.banks.get(v.bank as usize).and_then(|b| b.as_ref()).and_then(|b| b.sample(v.sample)) {
                        names.insert(key, n.name.clone());
                    }
                }
                new.push((key, v.clone(), SpuData::Pcm(pcm.samples.clone().into(), pcm.loop_start)));
            }
        }
        drop(p);
        if cap {
            capture_with(|c| {
                c.retain(&|s| s >= CAP_MOVIE || live.contains(&s));
                for (key, v, data) in &new {
                    let bus = cat_bus(data);
                    for (side, voice) in spu_sides(v, data.clone()).into_iter().enumerate() {
                        c.spawn(*key, side as u8, voice, v.sample_rate, scooby_recomp::engine::audio::spu::rodio_safe_speed(v.sample_rate, v.pitch), bus);
                    }
                    if std::env::var_os("GAME_SNDLOG").is_some() {
                        // P36 R3: body length in seconds (loop period of a looped stream) for seam checks
                        let secs = match data {
                            SpuData::Adpcm(d) if scooby_recomp::engine::audio::spu::is_interleaved(d) => {
                                use scooby_recomp::engine::audio::spu::{INT_HEADER, INT_INTERLEAVE};
                                ((d.len().saturating_sub(INT_HEADER) / (2 * INT_INTERLEAVE)) * INT_INTERLEAVE) as f64 / 16.0 * 28.0 / v.sample_rate.max(1) as f64
                            }
                            SpuData::Adpcm(d) => d.len() as f64 / 16.0 * 28.0 / v.sample_rate.max(1) as f64,
                            SpuData::Pcm(p, _) => p.len() as f64 / v.sample_rate.max(1) as f64,
                            SpuData::Interleaved(..) => 0.0,
                        };
                        eprintln!("capture: key-on f{} serial {key} ev {:#x} {bus:?} stream {:?} looped {} len {secs:.3}s owner {:#x} name {} group {:#x?}", g.steps, v.event_id, v.stream, v.looped, v.owner, names.get(key).map_or("-", |s| s.as_str()), grp.get(key).copied().flatten());
                    }
                }
                for (s, (l, r, sp)) in &gains {
                    c.set(*s, *l, *r, *sp, false);
                }
                for s in &paused {
                    c.set(*s, 0.0, 0.0, 1.0, true);
                }
            });
            if !g.audio {
                return;
            }
        }
        // P36 R5: output device lost / absent: drop the voices keyed on the dead stream (the live
        // mgr voices re-key on the reopened one) and spawn nothing until a stream is open again
        if !audio_device_watch(world, g.steps) {
            for (_, es) in g.voices.drain() {
                for e in es {
                    if let Ok(em) = world.get_entity_mut(e) {
                        em.despawn();
                    }
                }
            }
            return;
        }
        let gone: Vec<_> = g.voices.keys().filter(|k| !live.contains(k)).copied().collect();
        for k in gone {
            if let Some(es) = g.voices.remove(&k) {
                for e in es {
                    if let Ok(em) = world.get_entity_mut(e) {
                        em.despawn();
                    }
                }
            }
        }
        let mut q = bevy::ecs::system::SystemState::<(Commands, ResMut<Assets<SpuClip>>, Query<(&VoiceHandle, &VoiceSide, &mut bevy_audio::AudioSink)>)>::new(world);
        let (mut cmd, mut src, mut sinks) = q.get_mut(world).unwrap();
        // per-frame VOLL/VOLR of the playing voices (3D pan / distance follow the listener)
        update_voice_gains(&mut sinks, &|h| gains.get(&h).map(|g| if sound_on() { *g } else { (0.0, 0.0, g.2) }));
        for (h, _, sink) in sinks.iter_mut() {
            use bevy_audio::AudioSinkPlayback;
            match (paused.contains(&h.0), sink.is_paused()) {
                (true, false) => sink.pause(),
                (false, true) => sink.play(),
                _ => {}
            }
        }
        for (key, v, data) in new {
            let es = spawn_spu_voice(&mut cmd, &mut src, &v, data, cats.get(&key).copied().unwrap_or(1.0));
            g.voices.insert(key, es);
        }
        q.apply(world);
    });
}

// ---------------------------------------------------------------------------------------------
// screenshots / exit

fn shots_and_exit(mut commands: Commands, mut game: ResMut<Game>, target: Option<Res<Target>>, mut exit: MessageWriter<AppExit>) {
    let g = &mut *game;
    if !g.loaded || g.shots.is_none() {
        return;
    }
    let dir = g.shots.clone().unwrap();
    if let Some((name, n)) = g.pending_shot {
        // hold the runtime while new pipelines compile, then capture
        if n == 0 {
            let path = format!("{dir}/{name}.png");
            eprintln!("game: screenshot {path} at frame {}", g.steps);
            if std::env::var_os("GAME_DEBUG").is_some() {
                let p = g.rt.particles.lock().unwrap();
                for (i, (_, w)) in p.hud.slots.iter().enumerate() {
                    if let Some(w) = w {
                        eprintln!("  hud[{i}] t{} id {:#x} fl {:#x} pos {:?} dims {:?} rect {:?} parent {:?} kind {:?}", w.type_byte(), w.id, w.flags, w.pos, w.dims, w.rect, w.parent, if let scooby_recomp::engine::hud::window::EFHudWindowKind::Win3D(o) = &w.kind { o.obj } else { 0 });
                    }
                }
            }
            match &target {
                Some(t) => commands.spawn(Screenshot::image(t.0.clone())).observe(save_to_disk(path)),
                None => commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path)),
            };
            g.shot_done.insert(name);
            g.pending_shot = None;
            g.hold = 30;
        } else {
            g.pending_shot = Some((name, n - 1));
            g.hold = g.hold.max(1);
        }
        return;
    }
    if g.hold > 0 {
        return;
    }
    let book = GetCurrentStateID(&mut g.e, g.book);
    // K3: GAME_BOOKTGT=hex monster uid (any family); captured = Monster_BaseST_CAPTURED or killed
    let tgt = std::env::var("GAME_BOOKTGT").ok().and_then(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16).ok());
    let ghost = GetCurrentStateID(&mut g.e, tgt.unwrap_or(GHOST));
    if let Some(t) = tgt {
        let l = g.e.get_game_object(g.book, 0).map_or(0, |h| g.e.rt.obj(h).inst.local);
        let gb1 = g.e.rt.game_buffers.get(&1).map_or(0, |b| b.0);
        if g.steps % 15 == 0 && l != 0 {
            eprintln!("game: K3 f{} book {book} sub {:#x} btn {:#x} life {} power {} target {:#x} | tgt {t:#x} state {ghost:#x} alive {} pos {:?} gb2 {:#x} grp {:#x} hero {:?}", g.steps, g.e.mem.r32(l + 0x58), g.e.mem.r32(l + 0x1e8), f32::from_bits(g.e.mem.r32(l + 0x70)), f32::from_bits(g.e.mem.r32(gb1 + 4)), g.e.mem.r32(l + 0x60), g.e.rt.map.find(t).is_some(), pos_of(g, t), g.e.rt.game_buffers.get(&2).map_or(0, |b| g.e.mem.r32(b.0)), g.e.rt.map.find(t).map_or(0, |h| g.e.rt.obj(h).group_mask), pos_of(g, leader(g)));
        }
        let gone = g.e.rt.map.find(t).is_none();
        let n = if book == 2 { "k3_opening" } else if book == 4 { "k3_finalsuck" } else if gone || ghost == 0xb { "k3_captured" } else { "" };
        if !n.is_empty() && !g.shot_done.contains(n) && g.steps > 100 {
            g.pending_shot = Some((n, 1));
            g.hold = 1;
            return;
        }
    }
    let beam = g.rt.frame.sprites.iter().any(|s| !s.2.is_empty());
    if beam && g.beam_since.is_none() {
        g.beam_since = Some(g.steps);
    }
    let want: Option<&'static str> = if let Some(n) = shot_at(g.steps, &g.shot_done) {
        Some(n)
    } else if g.steps >= 90 && !g.shot_done.contains("book_closed") {
        Some("book_closed")
    } else if g.beam_since.is_some_and(|s| g.steps >= s + 20) && book == 3 && !g.shot_done.contains("book_open_beam") {
        Some("book_open_beam")
    } else if g.beam_since.is_some_and(|s| g.steps >= s + 60) && book == 3 && !g.shot_done.contains("j7_hud_capture") {
        // J7: mid-capture (beam on, HUD capture text + panels)
        Some("j7_hud_capture")
    } else if (ghost == 0xb) && !g.shot_done.contains("ghost_captured") {
        Some("ghost_captured")
    } else {
        None
    };
    if let Some(n) = want {
        g.pending_shot = Some((n, 60));
        g.hold = 1;
    }
    // J17: only the 4 built-in shots end the run (GAME_SHOT_AT shots counted here made windowed runs exit silently)
    if ["book_closed", "book_open_beam", "j7_hud_capture", "ghost_captured"].iter().all(|n| g.shot_done.contains(n)) && g.hold == 0 {
        exit.write(AppExit::Success);
    }
}

fn main() {
    // P39 R12: a panic on the main thread (Bevy's renderer init in add_plugins: no GPU / driver, surface or
    // device features missing) ended the game with only a crash.log line: show the player why
    if let Err(p) = std::panic::catch_unwind(game_main) {
        let headless = scooby_recomp::crashlog::args().iter().any(|a| a == "--headless");
        scooby_recomp::crashlog::main_panicked(&*p, headless);
    }
}

fn game_main() {
    // P37 R1: crash log in %LOCALAPPDATA%\ScoobyMM\logs (unless SCOOBY_CRASHLOG), session marker, --version
    scooby_recomp::crashlog::install_game();
    if scooby_recomp::crashlog::args().iter().any(|a| a == "--version" || a == "-V") {
        println!("Scooby-Doo! Mystery Mayhem PC port {}", scooby_recomp::crashlog::version());
        scooby_recomp::crashlog::clean_exit();
        return;
    }
    // P37 R3: SCOOBY_CRASH_TEST=1 panics right here (QA: crash-log record + symbolized backtrace check);
    // P39 R8: =render / =thread crash later (see before app.run)
    if std::env::var_os("SCOOBY_CRASH_TEST").is_some_and(|v| v != "render" && v != "thread") {
        panic!("SCOOBY_CRASH_TEST: deliberate crash-reporting test");
    }
    let a = parse_args();
    // P39 R9: the ISO / settings folders (maybe under a personal name outside the profile) are redacted in the logs
    scooby_recomp::crashlog::redact_game_paths(std::path::Path::new(&a.iso));
    // P39 R3: an untested disc release (US / PAL retail) has other module builds: refuse it with a clear message
    // before a game data lookup panics deep inside a level load (SCOOBY_UNTESTED_DISC=1 lets testers try it)
    if let Err(m) = scooby_recomp::engine::settings::check_disc_supported(std::path::Path::new(&a.iso)) {
        eprintln!("game: {m}");
        scooby_recomp::crashlog::note("UNSUPPORTED DISC", &format!("{m} ({})", a.iso));
        if !a.headless {
            scooby_recomp::crashlog::error_box(&m);
        }
        scooby_recomp::crashlog::clean_exit();
        std::process::exit(3);
    }
    // P39 R3: any game data the disc lacks (gamedata::missing) is shown to the player before the panic
    if !a.headless {
        let _ = scooby_recomp::engine::gamedata::FATAL_HOOK.set(scooby_recomp::crashlog::error_box);
    }
    // P34: settings.toml (launcher / in-game PC Settings); CLI flags override it below
    let pcs = game_settings::init(a.headless);
    // P24: the memory card's icon copy (TransferIcons, apiloadsave disc_file) reads the disc through
    // GAME_ISO or cwd-relative default paths; with `--iso` elsewhere / an exe-relative ISO every save
    // failed (DTFopen < 0 -> "save data is corrupted"). Hand it the ISO actually used.
    if std::env::var_os("GAME_ISO").is_none() {
        // SAFETY: start-up, before any other thread exists
        unsafe { std::env::set_var("GAME_ISO", &a.iso) };
    }
    // P37 R2: saves in the per-user %LOCALAPPDATA%\ScoobyMM\saves\memcard1 (windowed play; headless dev
    // runs keep the cwd-relative default so they never touch a player's card), migrating an old
    // exe-/cwd-relative saves/memcard1 once
    if std::env::var_os("GAME_SAVEDIR").is_none() && !a.headless {
        use scooby_recomp::engine::core::apiloadsave::{migrate_card, user_card_dir};
        if let Some(card) = user_card_dir() {
            let mut legacy = vec![std::path::PathBuf::from("saves/memcard1")];
            if let Some(d) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join("saves").join("memcard1"))) {
                legacy.insert(0, d);
            }
            if let Some(src) = migrate_card(&legacy, &card) {
                scooby_recomp::crashlog::note("SAVES MIGRATED", &format!("{} -> {}", src.display(), card.display()));
            }
            // SAFETY: start-up, before any other thread exists
            unsafe { std::env::set_var("GAME_SAVEDIR", &card) };
        }
    }
    scooby_recomp::crashlog::redact_game_paths(std::path::Path::new(&a.iso));
    // P39 R6/R7: one game per memory card (a second game.exe on it would interleave the saves): the player gets
    // a message and the second game closes. Only windowed runs lock (should_lock_card): a headless dev run
    // never holds the lock, so it can never make the player's windowed game report "already running".
    let _card_lock = if scooby_recomp::engine::core::apiloadsave::should_lock_card(a.headless) {
        let card_root = std::env::var_os("GAME_SAVEDIR").map_or_else(|| std::path::PathBuf::from("saves/memcard1"), Into::into);
        match scooby_recomp::engine::core::apiloadsave::lock_card(&card_root) {
            Ok(l) => l,
            Err(m) => {
                scooby_recomp::crashlog::note("ALREADY RUNNING", &m);
                scooby_recomp::crashlog::error_box(&m);
                scooby_recomp::crashlog::clean_exit();
                std::process::exit(1);
            }
        }
    } else {
        None
    };
    // P28 R3 B11: EFWorld::ClipGameObjects (logic clipping, P22 R26/R27) is always on in the original:
    // off-camera / far objects (word 0x400) are frozen, e.g. the EP1 shadow monster while the hero is
    // elsewhere. Default on for real-time play only; headless / --autoplay / --route keep the scripted
    // default (off) so the route timings stay put. `GAME_LOGICCLIP=0` turns it off.
    if std::env::var_os("GAME_LOGICCLIP").is_none() && !a.headless && !a.autoplay && a.route.is_none() {
        // SAFETY: start-up, before any other thread exists
        unsafe { std::env::set_var("GAME_LOGICCLIP", "1") };
    }
    let mut app = App::new();
    let headless = a.headless;
    if a.rt && cfg!(not(feature = "rt")) {
        eprintln!("game: --rt needs the `rt` cargo feature; ignored");
    }
    if a.headless {
        app.add_plugins(DefaultPlugins.set(WindowPlugin { primary_window: None, exit_condition: ExitCondition::DontExit, ..default() }).disable::<WinitPlugin>())
            .add_plugins(ScheduleRunnerPlugin::run_loop(std::time::Duration::from_millis(1)));
        // P36 R12: GAME_HEADLESS_AUDIO=1 (test hook, soak runs): open the real output device without a
        // window, always at zero output volume (never audible; the graph and device stream still run)
        if headless_audio() {
            app.add_plugins(bevy_audio::AudioPlugin { global_volume: bevy_audio::GlobalVolume::new(bevy_audio::Volume::SILENT), ..default() });
            bevy_audio::AddAudioSource::add_audio_source::<SpuClip>(&mut app);
        }
    } else {
        app.add_plugins(DefaultPlugins.set(WindowPlugin { primary_window: Some(Window { title: if a.level == "LOADER" { "Scooby-Doo! Mystery Mayhem".into() } else { format!("Scooby-Doo! Mystery Mayhem - {}", a.level) }, resolution: a.res.or(pcs.graphics.resolution).map(|(w, h)| bevy::window::WindowResolution::new(w, h)).unwrap_or_default(), mode: game_settings::window_mode(pcs.graphics.window_mode), present_mode: game_settings::present_mode(pcs.graphics.vsync), ..default() }), ..default() }))
            .add_plugins(bevy_audio::AudioPlugin { global_volume: bevy_audio::GlobalVolume::new(if sound_on() { bevy_audio::Volume::Linear(1.0) } else { bevy_audio::Volume::SILENT }), ..default() });
        bevy_audio::AddAudioSource::add_audio_source::<SpuClip>(&mut app);
    }
    if let Some(d) = &a.shots {
        let _ = std::fs::create_dir_all(d);
    }
    // P26 R21: GS blending in display space (GAME_LINEAR_BLEND=1 = the old linear-light path)
    scooby_recomp::engine::gfx::bevy_adapter::set_display_space(std::env::var_os("GAME_LINEAR_BLEND").is_none());
    // P40 R3 (B#9): GS subtract 0xa1 as a real ReverseSubtract blend (GAME_SUBBLEND_HW=0: old Multiply approximation)
    scooby_recomp::engine::gfx::bevy_adapter::set_sub_blend_hw(!std::env::var("GAME_SUBBLEND_HW").is_ok_and(|v| v == "0"));
    app.add_plugins(game_fx::GameFxPlugin).add_plugins(game_settings::PcSettingsPlugin);
    #[cfg(feature = "solari")]
    if scooby_recomp::engine::gfx::rtgi::requested() {
        app.add_plugins(scooby_recomp::engine::gfx::rtgi::RtgiPlugin);
    }
    app.add_plugins(GfxAdapterPlugin)
        .insert_resource(GlobalAmbientLight { color: Color::WHITE, brightness: 0.0, ..default() })
        .insert_resource(ClearColor(Color::srgb(0.0, 0.0, 0.0)))
        .add_plugins(GamePlugin { args: a })
        .add_systems(Startup, cpu_light_clustering);
    // P39 R5: a lost / reset GPU device (wgpu DeviceLost: driver reset or update, TDR) or another wgpu error
    // ended the app with only a log line (Bevy's default handler): keep its exit, add the crash.log record and
    // the player message box (shown below, after the loop)
    app.insert_resource(bevy::render::error_handler::RenderErrorHandler(|err, main_world, _| {
        let detail = format!("{:?}: {}", err.ty, err.description);
        let player = if matches!(err.ty, bevy::render::error_handler::ErrorType::DeviceLost) {
            scooby_recomp::crashlog::gpu_lost_message(&err.description)
        } else {
            format!("The graphics driver reported an error, so the game had to close. Update the graphics driver and start the game again; your saved progress on the memory card is kept.\n\n({})", detail.chars().take(300).collect::<String>())
        };
        scooby_recomp::crashlog::fatal("GPU ERROR", &player, &detail);
        main_world.write_message(AppExit::error());
        bevy::render::error_handler::RenderErrorPolicy::StopRendering
    }));
    // P39 R8: SCOOBY_CRASH_TEST=render panics in the render thread (first render frame), =thread in a helper
    // thread (like an audio / stream thread): QA for the crash record + a clean exit instead of a hang
    match std::env::var("SCOOBY_CRASH_TEST").as_deref() {
        Ok("render") => {
            if let Some(r) = app.get_sub_app_mut(bevy::render::RenderApp) {
                r.add_systems(bevy::render::Render, || panic!("SCOOBY_CRASH_TEST=render: deliberate render-thread crash"));
            }
        }
        Ok("thread") => {
            let _ = std::thread::Builder::new().name("crash-test".into()).spawn(|| panic!("SCOOBY_CRASH_TEST=thread: deliberate helper-thread crash"));
        }
        _ => {}
    }
    let code = app.run();
    capture_finish();
    let mut msg = scooby_recomp::crashlog::take_exit_message();
    // P39 R8: a panic outside main (the render thread: Bevy then ends the app with an error) closed the game:
    // tell the player instead of a window that just vanishes
    if msg.is_none() && matches!(code, AppExit::Error(_)) && scooby_recomp::crashlog::panicked() {
        msg = Some(scooby_recomp::crashlog::internal_error_message());
    }
    if let Some(m) = msg
        && !headless
    {
        scooby_recomp::crashlog::error_box(&m);
    }
    scooby_recomp::crashlog::clean_exit();
    if let AppExit::Error(c) = code {
        std::process::exit(c.get() as i32);
    }
}

/// P37 R1: the port lights everything in its own shaders (no Bevy point/spot lights, no clustered decals or
/// light probes), so Bevy's GPU light clustering only adds a per-frame compute pass + a staging-buffer readback.
/// That readback panicked once at LOADER frame 2 (wgpu "Buffer::get_mapped_range: Buffer with '' label is
/// invalid" on a task-pool thread, then a PoisonError in bevy_pbr cluster/gpu.rs on main = crash). CPU
/// clustering of zero objects is free; `GAME_GPU_CLUSTERING=1` restores Bevy's default.
fn cpu_light_clustering(cs: Option<ResMut<bevy::light::cluster::GlobalClusterSettings>>) {
    if let Some(mut cs) = cs
        && std::env::var_os("GAME_GPU_CLUSTERING").is_none()
    {
        cs.gpu_clustering = None;
    }
}

/// EFEngineController main loop: EFCutScenePlayer::Update after the frame. The cut-scene calls the
/// logic made during the frame (APIgame_*CutScene, queued in `cutscene_req`) are applied first.
fn cutscene_update(e: &mut scooby_recomp::engine::Engine, rt: &mut EngineRuntime) {
    use scooby_recomp::engine::runtime::CutSceneReq;
    use scooby_recomp::engine::script::RuntimeCutSceneHost;
    let (cs, reqs) = {
        let mut p = rt.particles.lock().unwrap();
        // keep the player in place (cloned): IsInCutScene from the 0x16 notify callbacks must see it
        (p.cutscene.clone(), std::mem::take(&mut p.cutscene_req))
    };
    let Some(mut cs) = cs else { return };
    let dt = e.clock.dt;
    let mut h = RuntimeCutSceneHost::new(e, rt);
    for q in reqs {
        match q {
            CutSceneReq::Play(id, snd, pos) => {
                eprintln!("game: cut-scene {id} play (sound {snd:#x}, pos {pos:#x}) actors {:x?}", cs.scene(id).map(|s| s.actors.clone()));
                cs.Play(&mut h, id, snd, pos)
            }
            CutSceneReq::Stop => cs.Stop(&mut h),
            CutSceneReq::Skip => cs.Skip(&mut h),
        }
    }
    // F1: MainLoop passes the filtered fStack_4 (ComputeDeltaTime) to EFCutScenePlayer::Update
    cs.Update(&mut h, dt);
    if std::env::var("GAME_CSLOG").is_ok() {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if n % 300 == 0 { eprintln!("game: cs n{n} flags {:#x} time {} dur {} cur {}", cs.flags, cs.time, cs.duration, cs.current); }
    }
    rt.particles.lock().unwrap().cutscene = Some(cs);
}

/// debug: GAME_LISTCODE=name lists the objects running a logic code (uid, position, first local words);
/// WP-P12: printed at load and again at frame 2 (locals are only set once the constructors ran)
fn list_code(g: &mut Game) {
    let Ok(n) = std::env::var("GAME_LISTCODE") else { return };
    let ids: Vec<u32> = g.e.rt.objects.iter().flatten().filter(|o| o.inst.code.is_some_and(|c| g.e.rt.codes[c].def.name.contains(n.as_str()))).map(|o| o.id).collect();
    for id in ids {
        let l = g.e.get_game_object(id, 0).map_or(0, |h| g.e.rt.obj(h).inst.local);
        let w: Vec<String> = (0..std::env::var("GAME_LISTCODE_N").ok().and_then(|v| v.parse().ok()).unwrap_or(9u32)).map(|k| if l == 0 { "-".into() } else { format!("{:#x}", g.e.mem.r32(l + 4 * k)) }).collect();
        let name = g.e.get_game_object(id, 0).and_then(|h| g.e.rt.obj(h).inst.code).map_or(String::new(), |c| g.e.rt.codes[c].def.name.to_string());
        eprintln!("game: code {name} obj {id:#x} at {:?} local {}", pos_of(g, id), w.join(" "));
    }
}

/// --give-book (debug): the EP1L01 data never hands the book over (WP-K2); send the book msg 0xe5
/// Bridge/VelmaNew send in later levels (GB2 |= 0x7e, TomeDoomFUNC_GrabBook(1)).
/// WP-P9: GAME_CARRY=file (see GAME_CARRY_DUMP) = game buffers carried into the first level
fn carry_load() -> Vec<(u32, Vec<u8>)> {
    let Some(d) = std::env::var("GAME_CARRY").ok().and_then(|f| std::fs::read(f).ok()) else { return Vec::new() };
    let mut v = Vec::new();
    let mut i = 0;
    while i + 8 <= d.len() {
        let id = u32::from_le_bytes(d[i..i + 4].try_into().unwrap());
        let n = u32::from_le_bytes(d[i + 4..i + 8].try_into().unwrap()) as usize;
        // P39 R4: a truncated carry file ends the list instead of an out-of-bounds panic
        let Some(b) = d.get(i + 8..(i + 8).saturating_add(n)) else { eprintln!("game: GAME_CARRY truncated at {i}"); break };
        v.push((id, b.to_vec()));
        i += 8 + n;
    }
    v
}

fn give_book(g: &mut Game) {
    let book = g.book;
    let Game { e, rt, .. } = g;
    rt.particles.lock().unwrap().logic = true;
    e.SendLogicMessageTo(book, 0xe5, 0, 0, 0);
    rt.particles.lock().unwrap().logic = false;
    eprintln!("game: --give-book: book msg 0xe5 at frame {}", g.steps);
}


// ---------------------------------------------------------------------------------------------
// J9: stand-ins for the unported EP1L03A camera module (l03_camera 0df12506) and the unported
// coll_level3a collectable. Active only while those logic codes are unported (decided at load).

/// EP1L03A ids (pack data / l03_camera + VelmaNew constants)
const L03A_CAM: u32 = 0x2a00_004a;
const L03A_VELMA: u32 = 0x2a00_0047;
/// collectable whose coll_level3a SetBit is GB2+0xc bit 0x2000000 (TmpMap entry 25, WP-L6)
const L03A_CLUE: u32 = 0x2a00_0048;

#[derive(Default)]
struct CamStandIn {
    /// l03_camera unported: team set-up, follow camera, clip planes and msg 0x1de stand-ins
    cam_unported: bool,
    cam: Option<DTMatrix>,
    /// GB2+8 seen last frame (VelmaNew cut-scene flags 0x400000 / 0x800000)
    gb2_8: u32,
    /// autoplay bot phase
    phase: u32,
    phase_at: u32,
    /// bot wall-follow: (target, best distance, frame of last progress, detour until, detours)
    nav: (u32, f32, u32, u32, u32),
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn norm(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
    [v[0] / l, v[1] / l, v[2] / l]
}

fn look_at(pos: [f32; 3], target: [f32; 3]) -> DTMatrix {
    let at = norm([target[0] - pos[0], target[1] - pos[1], target[2] - pos[2]]);
    let right = norm(cross([0.0, 1.0, 0.0], at));
    DTMatrix { right, up: cross(at, right), at, pos }
}

fn gb(g: &Game, n: u32) -> u32 {
    g.e.rt.game_buffers.get(&n).map(|b| b.0).unwrap_or(0)
}

fn send(g: &mut Game, to: u32, msg: u32, sub: u32) {
    let Game { e, rt, .. } = g;
    rt.particles.lock().unwrap().logic = true;
    e.SendLogicMessageTo(to, msg, sub, 0, 0);
    rt.particles.lock().unwrap().logic = false;
}

/// debug (K5): boss fights driven as a player would.
/// GAME_BOOKTGTS="uid,uid,..." (hex): catch the monsters in order (each placed 3 m in front of the leader when the
/// book is closed; next one after a FINALSUCKING -> closed cycle; re-placed after 900 frames without a capture).
/// GAME_WISPGRAB=1: every 20 frames, launched power wisps (logic code name contains "wisp") are moved onto the
/// leader = the player walking over them (the pick-up itself is the wisp's own collision logic).
fn k5_frame(g: &mut Game, f: u32) {
    use scooby_recomp::engine::APIgameobj;
    let lead = leader(g);
    if std::env::var_os("GAME_WISPGRAB").is_some() && f % 20 == 0 {
        if let Some(lp) = pos_of(g, lead) {
            let ids: Vec<u32> = g.e.rt.objects.iter().flatten().filter(|o| o.inst.code.is_some_and(|c| g.e.rt.codes[c].def.name.to_ascii_lowercase().contains("wisp")) && (o.id & 0x0080_0000) != 0).map(|o| o.id).collect();
            for id in ids {
                let a = g.e.mem.alloca(0x10);
                for (i, c) in [lp[0], lp[1] + 0.5, lp[2]].iter().enumerate() {
                    g.e.mem.w32(a + 4 * i as u32, c.to_bits());
                }
                if pos_of(g, id).is_some_and(|p| (p[0] - lp[0]).hypot(p[2] - lp[2]) > 1.0) {
                    APIgameobj::SetTranslation(&mut g.e, id, a);
                    eprintln!("game: K5 wisp {id:#x} -> leader at f{f}");
                }
            }
        }
    }
    // WP-P14: GAME_GBLOG=1 prints GB2 +0..+0x1c and the GB1+4 book power whenever they change (checked every 10 frames)
    if std::env::var_os("GAME_GBLOG").is_some() && f % 10 == 0 && gb(g, 2) != 0 && gb(g, 1) != 0 {
        let b = gb(g, 2);
        let w: Vec<u32> = (0..8).map(|i| g.e.mem.r32(b + 4 * i)).chain([g.e.mem.r32(gb(g, 1) + 4), g.e.mem.r32(gb(g, 1))]).collect();
        if w != g.gblog {
            eprintln!("game: GB2 f{f} {:x?} power {} courage {}", &w[..8], f32::from_bits(w[8]), f32::from_bits(w[9]));
            g.gblog = w;
        }
    }
    // P28 R12 (B10): GAME_COOLLOG=1 prints per frame courage GB1+0, the panel 0x2a100006 cool-meter words
    // (+0x1c cut-scene, +0x30 fader idle, +0x74 meter, +0x45c face, +0x460 target face) and the angle
    // (deg, atan2(right.y, right.x)) of every rotated 0xf.. HUD part frame (P25 R63 r63cool.py layout).
    if std::env::var_os("GAME_COOLLOG").is_some() && gb(g, 1) != 0 {
        let c = f32::from_bits(g.e.mem.r32(gb(g, 1)));
        let l = g.e.get_game_object(0x2a10_0006, 0).map_or(0, |h| g.e.rt.obj(h).inst.local);
        let w = |o: u32| if l == 0 { 0 } else { g.e.mem.r32(l + o) };
        let parts: Vec<String> = {
            let p = g.rt.particles.lock().unwrap();
            let mut v: Vec<(u32, f32)> = p.world.frames.iter().filter(|(u, m)| *u >> 24 == 0xf && (m.right[0] - 1.0).abs() > 1e-4).map(|(u, m)| (*u, m.right[1].atan2(m.right[0]).to_degrees())).collect();
            v.sort_by_key(|x| x.0);
            v.iter().map(|(u, a)| format!("{u:#x} {a:.1}")).collect()
        };
        eprintln!("game: COOL f{f} courage {c:.3} cs {} idle {} meter {:.3} face {} tgt {} parts {parts:?}", w(0x1c), w(0x30), f32::from_bits(w(0x74)), w(0x45c), w(0x460));
    }
    // WP-P14: GAME_LOCWATCH=uid prints the instance's local words 0..0x90 whenever they change (every 10 frames) (GAME_LOCWATCH_BASE=hex: start offset)
    if let (Some(u), 0) = (std::env::var("GAME_LOCWATCH").ok().and_then(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16).ok()), f % 10) {
        let l = g.e.get_game_object(u, 0).map_or(0, |h| g.e.rt.obj(h).inst.local);
        if l != 0 {
            let b0 = std::env::var("GAME_LOCWATCH_BASE").ok().and_then(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16).ok()).unwrap_or(0); let w: Vec<u32> = (0..0x25).map(|i| g.e.mem.r32(l + b0 + 4 * i)).collect();
            if w != g.locwatch {
                eprintln!("game: LOCWATCH f{f} {u:#x} {w:x?}");
                g.locwatch = w;
            }
        }
    }
    // P27 R17: GAME_PHYSNEAR=frame lists the physic objects whose world bsphere is within 4 m (XZ) of the leader
    if std::env::var("GAME_PHYSNEAR").ok().and_then(|v| v.parse::<u32>().ok()) == Some(f) && gb(g, 0) != 0 {
        let lead = g.e.mem.r32(gb(g, 0) + 8);
        if let Some(a) = pos_of(g, lead) {
            let s = scooby_recomp::engine::runtime::J2(&g.rt.particles).physic(|m, _| {
                m.by_owner.iter().filter_map(|(&u, &id)| m.Physic(id).map(|p| (u, p))).filter(|(_, p)| (p.model.bsphere_world[0] - a[0]).powi(2) + (p.model.bsphere_world[2] - a[2]).powi(2) < 16.0).map(|(u, p)| format!("{u:#x}: flags {:#x} ctx {:#x} state {:#x} bs {:?} r {} prims {:?}", p.flags, p.context, p.state, p.model.bsphere_world, p.model.bsphere_radius, p.model.prims)).collect::<Vec<_>>()
            });
            eprintln!("game: physnear f{f} lead {a:?} {s:#?}");
        }
    }
    // P27 R16 (B6): GAME_TILTLOG=1 prints both heroes (GB0+8 leader, +0xc follower) whenever their frame up vector
    // leaves (0,1,0) by > 0.01, plus a heartbeat every 300 frames
    if std::env::var_os("GAME_TILTLOG").is_some() && gb(g, 0) != 0 {
        let g0 = gb(g, 0);
        for (i, u) in [g.e.mem.r32(g0 + 8), g.e.mem.r32(g0 + 0xc)].into_iter().enumerate() {
            let up = g.rt.particles.lock().unwrap().world.frames.get(&u).map(|m| m.up);
            if let Some(up) = up {
                if up[0].abs() > 0.01 || up[2].abs() > 0.01 || up[1] < 0.9999 || f % 300 == 0 {
                    eprintln!("game: TILT f{f} {} {u:#x} pos {:?} up {:.3} {:.3} {:.3}", ["lead", "foll"][i], pos_of(g, u), up[0], up[1], up[2]);
                }
            }
        }
    }
    // GAME_POSLOG="uid,uid,...": positions every 30 frames (e.g. a fire ghost 0x147 vs its inner ghost 0x14d)
    // (WP-P16: GAME_POSLOG_EVERY=n logs every n frames, default 30)
    if let (Ok(v), 0) = (std::env::var("GAME_POSLOG"), f % std::env::var("GAME_POSLOG_EVERY").ok().and_then(|s| s.parse().ok()).unwrap_or(30)) {
        for u in v.split(',').filter_map(|s| u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()) {
            // P27 R5: + facing yaw (deg, atan2(at.x, at.z), P25 R34 convention)
            let yaw = g.rt.particles.lock().unwrap().world.frames.get(&u).map(|m| m.at[0].atan2(m.at[2]).to_degrees());
            // (no brackets/commas: p20_crawl parses the last 3 numbers of the [pos] list)
            let cam = g.rt.gfx.cameras.cameras.iter().flatten().next().map(|c| format!("{:.3} {:.3} {:.3} cyaw {:.1} cpitch {:.1} vw {:.4}", c.ltm.pos[0], c.ltm.pos[1], c.ltm.pos[2], c.ltm.at[0].atan2(c.ltm.at[2]).to_degrees(), c.ltm.at[1].asin().to_degrees(), c.rw.view_window[0])).unwrap_or_default();
            // P27 R16: + up vector (tilt) and the hero locals anim +0x218 / state fn +0x158
            let up = g.rt.particles.lock().unwrap().world.frames.get(&u).map(|m| m.up).unwrap_or_default();
            let loc = g.e.get_game_object(u, 0).map_or(0, |h| g.e.rt.obj(h).inst.local);
            let (an, st) = if loc != 0 { (g.e.mem.r32(loc + 0x218), g.e.mem.r32(loc + 0x158)) } else { (0, 0) };
            let fl = if loc != 0 { format!(" fl {:x} {:x} {:x} g14 {:x}", g.e.mem.r32(loc + 0x16c), g.e.mem.r32(loc + 0x170), g.e.mem.r32(loc + 0x174), if gb(g, 0) != 0 { g.e.mem.r32(gb(g, 0) + 0x14) } else { 0 }) } else { String::new() };
            // P27 R19: + EFPhysic velocity +0x28 / state +0x34 (P25 PINE probe reads the same fields; no brackets/commas, see p20_crawl)
            let pv = scooby_recomp::engine::runtime::J2(&g.rt.particles).physic(|m, _| m.Physic(m.PhysicOf(u)).map(|p| format!("{} {} {} st{}", p.velocity[0], p.velocity[1], p.velocity[2], p.state))).unwrap_or_default();
            eprintln!("game: K5 pos f{f} {u:#x} {:?} state {:#x} yaw {yaw:?} cam {cam} up {:.3} {:.3} {:.3} an {an:#x} st {st:#x}{fl} pv {pv}", pos_of(g, u), GetCurrentStateID(&mut g.e, u), up[0], up[1], up[2]);
        }
    }
    // K8: GAME_K5FLOOR=x,y,z puts the leader back on the arena floor whenever it is >1.5 m above it
    // (EP4L04: each zombie capture returns the team to the raised start platform, out of wisp reach)
    if let (Ok(v), 0) = (std::env::var("GAME_K5FLOOR"), f % 60) {
        let p: Vec<f32> = v.split(',').filter_map(|s| s.parse().ok()).collect();
        if p.len() == 3 && pos_of(g, lead).is_some_and(|lp| lp[1] > p[1] + 1.5) {
            let a = g.e.mem.alloca(0x10);
            for (i, c) in p.iter().enumerate() {
                g.e.mem.w32(a + 4 * i as u32, c.to_bits());
            }
            APIgameobj::SetTranslation(&mut g.e, lead, a);
            eprintln!("game: K8 leader {lead:#x} back to floor at f{f}");
        }
    }
    // P32 R6: GAME_RADARLOG=n lists every n frames the group-0x400 objects within 100 m of the leader (the radar /
    // book candidates) with their offset to the leader and state
    if let Some(n) = std::env::var("GAME_RADARLOG").ok().and_then(|v| v.parse::<u32>().ok()) {
        if f % n == 0 {
            if let Some(lp) = pos_of(g, lead) {
                let ids: Vec<u32> = g.e.rt.objects.iter().flatten().filter(|o| o.group_mask & 0x400 != 0).map(|o| o.id).collect();
                let names: std::collections::HashMap<u32, String> = g.e.rt.objects.iter().flatten().filter_map(|o| o.inst.code.map(|c| (o.id, g.e.rt.codes[c].def.name.to_string()))).collect();
                let mut s = Vec::new();
                for id in ids {
                    if let Some(p) = pos_of(g, id) {
                        let d = [p[0] - lp[0], p[1] - lp[1], p[2] - lp[2]];
                        if d[0].hypot(d[2]) < 100.0 {
                            s.push(format!("{id:#x} {} st {:#x} d {:.1} {:.1} {:.1}", names.get(&id).map_or("", |s| s.as_str()), GetCurrentStateID(&mut g.e, id), d[0], d[1], d[2]));
                        }
                    }
                }
                eprintln!("game: RADAR f{f} lead {lp:.1?} {s:?}");
            }
        }
    }
    // P32 R7 debug: GAME_GHOSTDBG=n,ID[,ID..] dumps every n frames a ghost's state, position and Ghost_Base locals
    // (+0x17c waypoint target, +0x110 noise pos, +0x19c events, +0x174/0x178 path count/index, +0x83c/0x840/0x848)
    if let Ok(v) = std::env::var("GAME_GHOSTDBG") {
        let mut it = v.split(',');
        let n = it.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(60).max(1);
        if f % n == 0 {
            for id in it.filter_map(|s| u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()) {
                let Some(l) = g.e.rt.objects.iter().flatten().find(|o| o.id == id).map(|o| o.inst.local).filter(|&l| l != 0) else { continue };
                let p = pos_of(g, id).unwrap_or_default();
                let st = GetCurrentStateID(&mut g.e, id);
                let rf = |o: u32| f32::from_bits(g.e.mem.r32(l + o));
                eprintln!(
                    "game: GHOST f{f} {id:#x} st {st:#x} pos {p:.2?} wp [{:.2} {:.2} {:.2}] n {} i {} noise [{:.2} {:.2} {:.2}] ev {:#x} 83c {} 840 {} 844 {} 848 {} 82c {:#x} 828 {:#x}",
                    rf(0x17c), rf(0x180), rf(0x184), g.e.mem.r32(l + 0x174), g.e.mem.r32(l + 0x178), rf(0x110), rf(0x114), rf(0x118),
                    g.e.mem.r32(l + 0x19c), g.e.mem.r32(l + 0x83c), g.e.mem.r32(l + 0x840), rf(0x844), g.e.mem.r32(l + 0x848), g.e.mem.r32(l + 0x82c), g.e.mem.r32(l + 0x828)
                );
            }
        }
    }
    // P32 R6: GAME_FINALDROP=n sends 0xda (drop the book: hero hit) to the book n frames into its first FINALSUCKING (state 4)
    if let Some(n) = std::env::var("GAME_FINALDROP").ok().and_then(|v| v.parse::<u32>().ok()) {
        if GetCurrentStateID(&mut g.e, g.book) == 4 {
            g.finaldrop += 1;
            if n == 0 && gb(g, 0) != 0 {
                eprintln!("game: FINALDROP f{f} book st4 #{} gb0.14 {:#x}", g.finaldrop, g.e.mem.r32(gb(g, 0) + 0x14));
            }
            if g.finaldrop == n {
                eprintln!("game: FINALDROP f{f} 0xda -> book");
                APIgameobj::SendMessageTo(&mut g.e, g.book, 0, 0xda, 0, 0);
            }
        }
    }
    let Ok(v) = std::env::var("GAME_BOOKTGTS") else { return };
    let tg: Vec<u32> = v.split(',').filter_map(|s| u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()).collect();
    let book = GetCurrentStateID(&mut g.e, g.book);
    let l = g.e.get_game_object(g.book, 0).map_or(0, |h| g.e.rt.obj(h).inst.local);
    let gb1 = g.e.rt.game_buffers.get(&1).map_or(0, |b| b.0);
    let (idx, placed, seen4) = g.k5;
    if book == 4 {
        g.k5.2 = true;
    }
    if seen4 && book == 1 {
        eprintln!("game: K5 f{f} target #{idx} {:#x} done (state {:#x})", tg.get(idx).copied().unwrap_or(0), tg.get(idx).map_or(0, |&t| GetCurrentStateID(&mut g.e, t)));
        g.k5 = (idx + 1, None, false);
    }
    let (idx, placed, _) = g.k5;
    let Some(&t) = tg.get(idx) else { return };
    // K8: a target already captured (group mask 0, e.g. a book hooked a neighbour zombie) is skipped
    if book == 1 && g.e.rt.map.find(t).is_some_and(|h| g.e.rt.obj(h).group_mask == 0) {
        eprintln!("game: K8 f{f} skip target #{idx} {t:#x} (group mask 0)");
        g.k5 = (idx + 1, None, false);
        return;
    }
    // keep the autoplay book-open presses going (they stop at r 1200)
    if book == 1 && f.saturating_sub(g.ap_base) >= 1100 {
        g.ap_base = f - 200;
    }
    let hooked = l != 0 && g.e.mem.r32(l + 0x60) != 0;
    // a player waits for wisps before engaging: a link broken at power 0 leaves the target uncaught
    let power = if gb1 != 0 { f32::from_bits(g.e.mem.r32(gb1 + 4)) } else { 0.0 };
    if f >= 150 && book != 4 && !hooked && power >= std::env::var("GAME_K5POWER").ok().and_then(|v| v.parse().ok()).unwrap_or(150.0) && placed.is_none_or(|p| f - p > 900) {
        // 3 m in front of the leader (physic frame `at`, as the K3 set-up): the book scans ahead
        let fr = { let p = g.rt.particles.lock().unwrap(); let ph = p.sub.physic.PhysicOf(lead); p.sub.physic.Physic(ph).map(|x| x.frame) };
        if let Some(fr) = fr {
            let hp = fr.pos;
            let a = g.e.mem.alloca(0x10);
            for (i, c) in [hp[0] + fr.at[0] * 3.0, hp[1] + 1.0, hp[2] + fr.at[2] * 3.0].iter().enumerate() {
                g.e.mem.w32(a + 4 * i as u32, c.to_bits());
            }
            APIgameobj::SetTranslation(&mut g.e, t, a);
            // K8: GAME_K5REFILL tops the book power up (GB1+4 = 180) at each placement (EP4L04 has too
            // few wisps for the harness' unaimed book use; a debug cheat, not game behaviour)
            if std::env::var_os("GAME_K5REFILL").is_some() && gb1 != 0 {
                g.e.mem.w32(gb1 + 4, 180f32.to_bits());
            }
            eprintln!("game: K5 f{f} place target #{idx} {t:#x}");
            g.k5.1 = Some(f);
        }
    }
    if f % 30 == 0 && l != 0 {
        let grp = g.e.rt.map.find(t).map_or(0, |h| g.e.rt.obj(h).group_mask);
        eprintln!("game: K5 f{f} book {book} life {} power {} hooked {:#x} | tgt {t:#x} state {:#x} grp {grp:#x} pos {:?} lead {:?}", f32::from_bits(g.e.mem.r32(l + 0x70)), f32::from_bits(g.e.mem.r32(gb1 + 4)), g.e.mem.r32(l + 0x60), GetCurrentStateID(&mut g.e, t), pos_of(g, t), pos_of(g, lead));
    }
}

fn pos_of(g: &Game, uid: u32) -> Option<[f32; 3]> {
    g.rt.particles.lock().unwrap().world.frames.get(&uid).map(|m| m.pos)
}

/// before Frame: team set-up of Camera_BaseCB_INIT (notify 0x14, first logic update) and the
/// autoplay bot pad.
fn standin_pre_frame(g: &mut Game, pad: PadState) -> PadState {
    let gb0 = gb(g, 0);
    if lvl_steps(g) == 0 && gb0 != 0 && g.cam_standin.as_ref().unwrap().cam_unported {
        // STAND-IN Camera_BaseCB_INIT: GB0+0x30 = 0, zone GB0+0x24 = 1, +0x34 bit 1 off / 0x10 on,
        // then (GB0+8 == 0) Camera_BaseFUNC_MessageHandler(0x47, 2): no goHeroesData entry for 2
        // in l03_camera -> "target not yet supported" default team: +0/+4 = 1, leader 0x2a0000f0,
        // follower 0x2a0000ef, +0x10 = 2; msg 0x7d to the follower, 0x7c to the leader.
        let m = &mut g.e.mem;
        m.w32(gb0 + 0x30, 0);
        m.w32(gb0 + 0x24, 1);
        let v = m.r32(gb0 + 0x34) & !1;
        m.w32(gb0 + 0x34, v | 0x10);
        if m.r32(gb0 + 8) == 0 {
            m.w32(gb0 + 4, 1);
            m.w32(gb0, 1);
            m.w32(gb0 + 8, 0x2a00_00f0);
            m.w32(gb0 + 0xc, 0x2a00_00ef);
            m.w32(gb0 + 0x10, 2);
            send(g, 0x2a00_00ef, 0x7d, 0);
            send(g, 0x2a00_00f0, 0x7c, 0);
        }
    }
    if g.autoplay { bot_pad(g) } else { pad }
}

/// after Frame: follow camera + l03_camera SetClipPlane table, VelmaNew msg 0x1de cut-scenes,
/// coll_level3a clue pick-up.
fn standin_post_frame(g: &mut Game) {
    use scooby_recomp::engine::runtime::CutSceneReq;
    let (gb0, gb2) = (gb(g, 0), gb(g, 2));
    if gb0 == 0 || gb2 == 0 {
        return;
    }
    // STAND-IN l03_camera msg 0x1de handler: PlugNPlayCutScene(arg, *data = sound, camera).
    // VelmaNew sends it to 0x2a00004a (no logic here) with (0, 0x80000002) on the first talk
    // (GB2+8 |= 0x400000) and (1, 0x80000003) at the hand-over (GB2+8 |= 0x800000).
    let cam_unported = g.cam_standin.as_ref().unwrap().cam_unported;
    let f8 = g.e.mem.r32(gb2 + 8);
    let old = std::mem::replace(&mut g.cam_standin.as_mut().unwrap().gb2_8, f8);
    for (bit, id, snd) in [(0x40_0000, 0, 0x8000_0002), (0x80_0000, 1, 0x8000_0003)] {
        if cam_unported && f8 & bit != 0 && old & bit == 0 {
            eprintln!("game: J9 stand-in camera msg 0x1de: cut-scene {id} sound {snd:#x} at frame {}", g.steps);
            g.rt.particles.lock().unwrap().cutscene_req.push(CutSceneReq::Play(id, snd, L03A_CAM));
        }
    }
    // STAND-IN coll_level3a: the clue collectable sets GB2+0xc bit 0x2000000 when the leader takes it
    let lead = g.e.mem.r32(gb0 + 8);
    if g.rt.report.unported.iter().any(|u| u.name == "coll_level3a.op2") && g.e.mem.r32(gb2 + 0xc) & 0x200_0000 == 0 {
        if let (Some(a), Some(b)) = (pos_of(g, lead), pos_of(g, L03A_CLUE)) {
            let d = ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
            if d < 1.2 && (a[1] - b[1]).abs() < 3.5 {
                let v = g.e.mem.r32(gb2 + 0xc) | 0x200_0000;
                g.e.mem.w32(gb2 + 0xc, v);
                g.rt.particles.lock().unwrap().world.hidden.insert(L03A_CLUE);
                eprintln!("game: J9 stand-in coll_level3a: clue {L03A_CLUE:#x} taken at frame {}", g.steps);
            }
        }
    }
    // STAND-IN camera (as EP1L01 had before l01_camera): behind/above the leader, written into
    // the camera owner's frame; the cut-scene animates the camera object itself while it plays.
    if !cam_unported {
        return;
    }
    let playing = g.rt.particles.lock().unwrap().cutscene.as_ref().is_some_and(|c| c.flags & 1 != 0);
    let zone = g.e.mem.r32(gb0 + 0x24);
    let Some(h) = g.rt.particles.lock().unwrap().world.frames.get(&lead).copied() else { return };
    let st = g.cam_standin.as_mut().unwrap();
    if !playing {
        let at = norm([h.at[0], 0.0, h.at[2]]);
        let want = [h.pos[0] - at[0] * 4.5, h.pos[1] + 2.2, h.pos[2] - at[2] * 4.5];
        let cur = st.cam.map(|c| c.pos).unwrap_or(want);
        let k = 0.08;
        let pos = [cur[0] + (want[0] - cur[0]) * k, cur[1] + (want[1] - cur[1]) * k, cur[2] + (want[2] - cur[2]) * k];
        let m = look_at(pos, [h.pos[0], h.pos[1] + 1.0, h.pos[2]]);
        st.cam = Some(m);
        g.rt.particles.lock().unwrap().world.frames.insert(L03A_CAM, m);
    } else {
        st.cam = None;
    }
    // l03_camera Camera_BaseFUNC_SetClipPlane (zone = GB0+0x24, "#ROOM_nn")
    let (near, far) = match zone {
        2 | 7 => (0.025, 25.0),
        3 | 5 => (0.02, 20.0),
        _ => (0.045, 45.0),
    };
    for c in g.rt.gfx.cameras.cameras.iter_mut().flatten() {
        if c.frame_owner == L03A_CAM {
            c.rw.near = near;
            c.rw.far = far;
        }
    }
}

/// --autoplay in EP1L03A: walk to Velma (first talk), take the clue, walk back (hand-over), open
/// the book (SQUARE). Stick is camera-relative (heroes read the camera frame).
fn bot_pad(g: &mut Game) -> PadState {
    let mut p = idle_pad();
    let (gb0, gb2) = (gb(g, 0), gb(g, 2));
    if gb0 == 0 || lvl_steps(g) < 30 {
        return p;
    }
    let lead = g.e.mem.r32(gb0 + 8);
    let (f8, fc) = (g.e.mem.r32(gb2 + 8), g.e.mem.r32(gb2 + 0xc));
    let playing = g.rt.particles.lock().unwrap().cutscene.as_ref().is_some_and(|c| c.flags & 1 != 0);
    let steps = lvl_steps(g);
    let cam = g.rt.gfx.cameras.cameras.iter().flatten().find(|c| c.frame_owner == L03A_CAM).map(|c| c.ltm);
    let target = if f8 & 0x40_0000 == 0 {
        Some(L03A_VELMA)
    } else if fc & 0x200_0000 == 0 {
        Some(L03A_CLUE)
    } else if f8 & 0x80_0000 == 0 {
        Some(L03A_VELMA)
    } else {
        None
    };
    if playing {
        // cut-scene: X skips the text, like a player would
        return p;
    }
    if !g.route.is_empty() {
        if let Some(p) = route_pad(g, lead, cam) {
            return p;
        }
    }
    let st = g.cam_standin.as_mut().unwrap();
    let target = if g.route.is_empty() { target } else { None };
    let Some(t) = target else {
        if st.phase == 0 {
            st.phase = 1;
            st.phase_at = steps;
        }
        let k = steps - st.phase_at;
        if (60..70).contains(&k) {
            p.buttons = pad::SQUARE;
        }
        return p;
    };
    if steps == 40 {
        if let Ok(v) = std::env::var("GAME_MAP") {
            floor_map(g, &v);
        }
    }
    if std::env::var_os("GAME_TP").is_some() && steps == 40 {
        if let Some(d) = pos_of(g, 0x2a00_001f) {
            if let Some(m) = g.rt.particles.lock().unwrap().world.frames.get_mut(&lead) { m.pos = d; }
        }
    }
    if std::env::var_os("GAME_DEBUG").is_some() && (steps == 30 || (1125..1150).contains(&steps)) {
        let s = scooby_recomp::engine::runtime::J2(&g.rt.particles).physic(|m, _| {
            [lead ^ 0x1f].map(|u| m.Physic(m.PhysicOf(u)).map(|p| (p.flags, p.context, p.state, p.model.prims.len(), p.model.prims.iter().flatten().count(), p.model.bsphere_world, p.model.bsphere_radius, p.velocity, p.frame, p.falling_speed, p.pre_list.clone(), p.all_list.clone())))
        });
        eprintln!("game: physic door/lead {s:?}");
        {
            let p = g.rt.particles.lock().unwrap();
            let o = p.sub.obj3d.get(&0x2a00_001f).copied();
            eprintln!("game: door obj3d {o:x?} col {:?}", o.map(|o| p.sub.res_collision.contains_key(&o.0)));
        }
        for u in (0x2a00_0015..=0x2a00_0020).chain([0x2a00_011e, 0x2a00_011f, L03A_CLUE]) {
            eprintln!("game: pos {u:#x} {:?}", pos_of(g, u));
        }
    }
    if std::env::var_os("GAME_DEBUG").is_some() && steps % 30 == 0 {
        let n: Vec<_> = g.rt.particles.lock().unwrap().sub.log.notifications.iter().filter(|n| (0x2a00_0015..=0x2a00_0020).contains(&n.1) || n.1 == lead).cloned().collect();
        eprintln!("game: notes {n:x?}");
        eprintln!("game: bot f{steps} lead {lead:#x} {:?} -> {t:#x} {:?} gb2+8 {f8:#x} +c {fc:#x} other {:?}", pos_of(g, lead), pos_of(g, t), pos_of(g, lead ^ 0x1f));
    }
    let (Some(a), Some(b), Some(c)) = (pos_of(g, lead), pos_of(g, t), cam) else { return p };
    // the target lies in another room: head for the hot-spot door nearest to it first (doors
    // within 40 m, not the one just arrived through)
    static ARRIVE: std::sync::Mutex<Option<([f32; 3], [f32; 3])>> = std::sync::Mutex::new(None);
    let d2 = |p: [f32; 3], q: [f32; 3]| (p[0] - q[0]).powi(2) + (p[2] - q[2]).powi(2);
    let arrive = {
        let mut ar = ARRIVE.lock().unwrap();
        let (last, mut arr) = ar.unwrap_or((a, a));
        if d2(last, a) > 20.0 * 20.0 {
            arr = a;
        }
        *ar = Some((a, arr));
        arr
    };
    let way = (0x2a00_0015..=0x2a00_0020u32)
        .filter_map(|u| pos_of(g, u))
        .filter(|&w| d2(w, a) < 40.0 * 40.0 && d2(w, arrive) > 4.0 * 4.0 && d2(w, b) < d2(a, b))
        .min_by(|&p, &q| (d2(a, p).sqrt() + d2(p, b).sqrt()).total_cmp(&(d2(a, q).sqrt() + d2(q, b).sqrt())));
    let b = if d2(a, b) > 6.0 * 6.0 { way.unwrap_or(b) } else { b };
    let b = path_step(g, a, b, steps).unwrap_or(b);
    let mut d = norm([b[0] - a[0], 0.0, b[2] - a[2]]);
    // bug-algorithm detour: no progress for 40 frames -> walk sideways (alternating) for 50
    {
        let st = g.cam_standin.as_mut().unwrap();
        let dist = ((b[0] - a[0]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
        if st.nav.0 != t {
            st.nav = (t, dist, steps, 0, 0);
        }
        if dist < st.nav.1 - 0.3 {
            st.nav.1 = dist;
            st.nav.2 = steps;
        } else if steps > st.nav.2 + 40 && steps > st.nav.3 {
            st.nav.3 = steps + 50 + 25 * (st.nav.4 % 3);
            st.nav.4 += 1;
            st.nav.2 = steps + 50;
        }
        if steps < st.nav.3 {
            let r = st.nav.4 as f32 * 2.399_963; // golden-angle random walk out of dead ends
            d = [r.cos(), 0.0, r.sin()];
        }
    }
    let fwd = norm([c.at[0], 0.0, c.at[2]]);
    let right = norm(cross([0.0, 1.0, 0.0], fwd));
    // camera right = +x on screen? RW: right vector of a look-at built with up x at points screen-left
    let x = -(d[0] * right[0] + d[2] * right[2]);
    let y = d[0] * fwd[0] + d[2] * fwd[2];
    p.lx = stick(x);
    p.ly = stick(-y);
    // hs_door_l03 hot-spots: in collision, X (action 0x39) takes the door; a player presses X
    // stuck (dialogue text waiting for X, or a hot-spot): X like a player would
    let stuck = g.cam_standin.as_ref().unwrap().nav.2 + 40 < steps && std::env::var_os("GAME_NOX").is_none();
    if (way.is_some_and(|w| d2(a, w) < 2.5 * 2.5) || stuck) && steps % 20 < 2 {
        p.buttons = pad::CROSS;
    }
    p
}

/// --route: "x,z[,A];..." or "l03a" (built-in EP1L03A hand-over route, positions from the pack:
/// door 0x2a00001f (551.7, 0.07) -> arrival near 0x2a00001e, VelmaNew 0x2a000047 (438.7, -2.4),
/// clue 0x2a000048 (413.5, 3.0, -18.0) on its 2 m table, back to VelmaNew).
/// --route bot outside EP1L03A: leader = GB0+8, stick relative to the active camera. During a
/// cut-scene the pad stays idle (the cut-scene plays out like for a player who does not skip).
/// WP-P20: runtime frames since the level's first frame (route bot clock)
fn lvl_steps(g: &Game) -> u32 {
    g.steps.saturating_sub(g.lvl_f0)
}

fn route_any(g: &mut Game) -> Option<PadState> {
    let gb0 = gb(g, 0);
    if gb0 == 0 || lvl_steps(g) < 30 {
        return None;
    }
    if g.rt.particles.lock().unwrap().cutscene.as_ref().is_some_and(|c| c.flags & 1 != 0) {
        return Some(idle_pad());
    }
    // GB0+0x14 bit 0x1000000: a tutorial text holds the team until X (like a player reading it)
    if g.e.mem.r32(gb0 + 0x14) & 0x100_0000 != 0 {
        return Some(PadState { buttons: if lvl_steps(g) % 40 < 2 { pad::CROSS } else { 0 }, ..idle_pad() });
    }
    let lead = g.e.mem.r32(gb0 + 8);
    let cam = g.rt.gfx.cameras.cameras.iter().flatten().next().map(|c| c.ltm);
    route_pad(g, lead, cam)
}

/// P20 R121: ANIMSND is on in game.exe (P22 R12's `--route ep1` exception dropped in P20 R126: the EP1/L04 route
/// is retuned under it). GAME_ANIMSND=0 turns it off.
/// P22 R43: emitter AddChild (gas/glow emitted at their monsters; EP4/L04 boss gas scares) is on in game.exe
/// (C behaviour). GAME_EMCHILD=0 turns it off. P20 R172: also on for `--route` bot runs (P23 R62's `--route ep1`
/// exception dropped: the EP1/L03B route waits for the third west ghost to idle, 'A' step, before its capture).
fn em_child_on(_route: Option<&str>) -> bool {
    std::env::var("GAME_EMCHILD").as_deref() != Ok("0")
}

fn anim_sound_on(_route: Option<&str>) -> bool {
    std::env::var("GAME_ANIMSND").as_deref() != Ok("0")
}

/// P8: `--route ep1` = the built-in route of every Episode 1 level, chained through the level loads
/// (L01 -> L02 -> L03A -> ... -> L04); `--route l02` etc. = one level's route
fn level_route<'a>(r: Option<&'a str>, level: &str) -> Option<&'a str> {
    match (r, level.to_ascii_uppercase().as_str()) {
        (Some("ep1"), "EP1/L01") => Some("l01"),
        (Some("ep1"), "EP1/L02") => Some("l02"),
        (Some("ep1"), "EP1/L03A") => Some("l03a"),
        (Some("ep1"), "EP1/L03B") => Some("l03b"),
        (Some("ep1"), "EP1/L04") => Some("l04"),
        (Some("ep1"), _) => None,
        (Some("ep3"), "EP3/L01") => Some("e3l01"),
        (Some("ep3"), _) => None,
        (Some("ep2"), "EP2/L01A") => Some("ep2l01a"),
        (Some("ep2"), "EP2/L01B") => Some("ep2l01b"),
        (Some("ep2"), _) => None,
        _ => r,
    }
}

fn parse_route(r: Option<&str>) -> Vec<(f32, f32, char)> {
    let r = match r {
        None => return Vec::new(),
        Some("l03a") => routes_ep1b::L03A_ROUTE,
        Some("l03b") => routes_ep1b::L03B_ROUTE,
        Some("l04") => routes_ep1b::L04_ROUTE,
        Some("e3l01") => routes_ep3b::L01_ROUTE,
        Some("ep2l01a") => routes_ep1b::EP2_L01A_ROUTE,
        Some("ep2l01b") => routes_ep1b::EP2_L01B_ROUTE,
        Some("l01") => L01_ROUTE,
        Some("l01cart") => L01CART_ROUTE,
        Some("l02") => L02_ROUTE,
        Some(r) => r,
    };
    // P32 R30: a waypoint written `!x,z[,A]` is a GAME_LOGICCLIP=1-only step (as `iflc` in the p17 routes);
    // without logic clipping it is dropped, so routes without `!` parse exactly as before
    let clip = std::env::var("GAME_LOGICCLIP").is_ok_and(|v| v.trim() == "1");
    r.split(';')
        .filter_map(|w| {
            let w = match w.trim_start().strip_prefix('!') { Some(_) if !clip => return None, Some(c) => c, None => w };
            let mut it = w.split(',');
            let x = it.next()?.trim().parse().ok()?;
            let z = it.next()?.trim().parse().ok()?;
            Some((x, z, it.next().and_then(|a| a.trim().chars().next()).unwrap_or(' ')))
        })
        .collect()
}

// J16: ledge climb onto the clue block (EdgeUp moves the collision prim), clue, back via x 442 to Velma (0x70 handover)
const L03A_TOME: u32 = 0x2a00_00ee;
/// P7: EP1L01 start -> EP1\L02 without GAME_WARP: crate step J (34.75,4) into the north attic, button P,
/// rolling ladder P, map on the 6 m ledge; X door 0x2e to the cart room; Scooby crawls under the table (Y,
/// door 0x31: only the leader warps, hero_misc NextMove needs gb0+0x34&0x10), pushes cart 0x51 east and cart
/// 0x52 (P), walks back west via the lane to within 7.5 m of Shaggy (ST_Unhook_Loop reconnect: needs a clear
/// line, the table blocks it), then doors 0x30 -> 0x32 -> 0x34 (X; 0x34 loads EP1\L02)
const L01_ROUTE: &str = "-153,-2;-146,-1.5;-142.5,0,X;-60,0;-54,3;-54,9,D;-49,9,D;-48,6,D;-48,2,D;-46.5,2,X;47,-2,D;34.5,-2,D;34.6,2.3,D;34.75,4,J;34.75,6.5,D;40,7,D;49.3,7.98,D;49.3,7.98,P;40.3,9.6,D;40.3,9.6,P;38.2,13.5,D;35.75,15.65,D;38.3,11.6,D;38.3,11.6,P;40.5,8.5,D;45,7,D;49.3,5.72,X;-48,2,D;-48,6,D;-49,9,D;-54,9,D;-54,3,D;-56,0,D;-51.5,0,X;121.5,0,D;121.5,2.5,D;123.5,5,D;124,8.3,D;125.7,8.2,Y;131,4.2,D;133,3.75,D;133,3.75,P;133,3.75,P;133,3.75,P;133,4.2,D;138,3.7,D;138,1,D;138,-3,D;127,-4.8,D;125.1,-4.6,D;125.1,-4.6,P;125.1,-4.6,P;125.1,-4.6,P;127,-3.2,D;123,-3,D;121.5,0,D;121.5,2.5,D;123,5,D;121.5,2.5,D;121.5,0,D;123,-3,D;127,-3.2,D;130,-1;141,-1.5;143.1,-1.57,X;232.5,3.25;234.25,3.25,X;312.5,1.7;314.7,1.7,X";
/// P6: EP1L01 cart room -> EP1\L02 (needs GAME_WARP=125.1,0.2,-5.2:1750): X at the cart's push
/// sphere (south side) pushes it 3 loops, then doors 0x30 -> 0x32 -> 0x34 (X; 0x34 loads EP1\L02)
const L01CART_ROUTE: &str = "-153,-2;-146,-1.5;125.1,-4.6;125.1,-4.6,P;125.1,-4.6,P;125.1,-4.6,P;130,-1;141,-1.5;143.1,-1.57,X;232.5,3.25;234.25,3.25,X;312.5,1.7;314.7,1.7,X";
/// P8: EP1L02 (see ENGINE_PORT P8)
// WP-P9/P12: first room (no book: power 0) -> doors 0x14, 0x16, 0x1e -> cupboard 0x2a00011c (X: launches key
// 0x2a0000ab = GB2+0xc 0x40000) -> door 0x19 back -> door 0x18 (lock 0x40000) -> exit 0x1c (EP1L03A, f~5960)
// P32 R42: clip-only `!288,12.8;!283,12.8` on the way back west: under GAME_LOGICCLIP=1 the team reaches the snacks
// (GB2+0x18 0x100/0x200) east of the 209.3,18.26 door at courage 1.0 and Coll_BaseFUNC_CheckEvents refuses them
// (C: `1.0 <= courage` -> state 3), so they are picked up after the 290-330 shadow hits (L03A entry courage 0.2 -> 0.5)
const L02_ROUTE: &str = "33.69,-6.84;37.31,-8.19;38,-9;38,-12,J;38,-14;37,-17.5;28,-17.8;23,-17.5;13,-17.5;8,-15;7.5,-8;6,-7.8,X;112,-5;108,-4.2;108,-3.2,X;207,-12;207,18.26;209.3,18.26,X;290,12;330,11;336.5,10.9,X;336,11.3;290,11;!288,12.8;!283,12.8;282,10.6;281.1,10.6,X;207,10;207,7.26;209,7.26,X;420,-7.75;439,-3;439,-0.6,X";

/// P8: the level's book (code tomedoom; EP1L03A 0x2a0000ee)
fn tome_id(g: &Game) -> u32 {
    g.e.rt.objects.iter().flatten().find(|o| o.inst.code.is_some_and(|c| g.e.rt.codes[c].def.name == "tomedoom")).map_or(L03A_TOME, |o| o.id)
}

/// P8: the level's book targets for the 'B' route action: monster instances (ghost / shadows codes;
/// override GAME_ROUTE_MONSTERS="hex,..") that are not captured (state 0xb)
fn monsters(g: &mut Game) -> Vec<u32> {
    let ids: Vec<u32> = if let Ok(v) = std::env::var("GAME_ROUTE_MONSTERS") {
        v.split(',').filter_map(|s| u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()).collect()
    } else {
        g.e.rt.objects.iter().flatten().filter(|o| o.inst.code.is_some_and(|c| { let n = g.e.rt.codes[c].def.name; n == "ghost" || n == "shadows" || n == "poltergeist" || n.ends_with("ghost") && !n.starts_with("scooby") && !n.starts_with("shaggy") })).map(|o| o.id).collect()
    };
    ids.into_iter().filter(|&u| GetCurrentStateID(&mut g.e, u) != 0xb).collect()
}

/// --route bot: steer to the current waypoint (camera-relative stick); None when the route is done.
fn route_pad(g: &mut Game, lead: u32, cam: Option<DTMatrix>) -> Option<PadState> {
    let tome = tome_id(g);
    let mut p = idle_pad();
    let steps = lvl_steps(g);
    let a = pos_of(g, lead)?;
    let c = cam?;
    // P27 R8: the stick below is aimed with `c` (last frame's camera); the physics converts it back with the
    // same matrix (EngineRuntime::stick_cam) so the autopilot's world-space steering is exact
    g.rt.stick_cam = Some(c);
    let playing = g.rt.particles.lock().unwrap().cutscene.as_ref().is_some_and(|c| c.flags & 1 != 0);
    if std::env::var_os("GAME_DEBUG").is_some() && steps % 30 == 0 {
        { let b2 = gb(g, 2); let b0 = gb(g, 0); let b14 = if b0 == 0 { 0 } else { g.e.mem.r32(b0 + 0x14) }; let b34 = if b0 == 0 { 0 } else { g.e.mem.r32(b0 + 0x34) }; eprintln!("game: route f{steps} gb0.14 {b14:#x} .34 {b34:#x} lead {lead:#x} {a:?} other {:?} gb2 {:#x} {:#x}", pos_of(g, g.e.mem.r32(b0 + 0xc)), g.e.mem.r32(b2 + 8), g.e.mem.r32(b2 + 0xc)); }
        // WP-P9: GAME_WATCH="hex,.." prints state + position of the listed instances
        if let Ok(v) = std::env::var("GAME_WATCH") {
            for u in v.split(',').filter_map(|s| u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()) {
                eprintln!("game: watch f{steps} {u:#x} state {:#x} {:?}", GetCurrentStateID(&mut g.e, u), pos_of(g, u));
                // WP-P21 R29: GAME_WATCHL=n also prints the first n local-data words
                if let Some(n) = std::env::var("GAME_WATCHL").ok().and_then(|v| v.parse::<u32>().ok()) {
                    let l = g.e.get_game_object(u, 0).map_or(0, |h| g.e.rt.obj(h).inst.local);
                    if l != 0 { let (fl, sl, inl) = g.e.get_game_object(u, 0).map_or((0, 0.0, false), |h| (g.e.rt.obj(h).inst.flags, g.e.rt.obj(h).inst.sleep, g.e.rt.logic_list.contains(&h))); eprintln!("game: watchl {u:#x} flags {fl:#x} sleep {sl} listed {inl} {:x?}", (0..n).map(|k| g.e.mem.r32(l + 4 * k)).collect::<Vec<_>>()); }
                }
                // GAME_WATCHSND=hexid: is sound event `id` playing on the watched instance
                if let Some(id) = std::env::var("GAME_WATCHSND").ok().and_then(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16).ok()) {
                    let pl = scooby_recomp::engine::runtime::J2(&g.rt.particles).sound(|s| s.ObjIsSoundPlaying(u, id));
                    eprintln!("game: watchsnd {u:#x} {id:#x} {pl:?}");
                }
            }
        }
        if steps == std::env::var("GAME_PHYSDUMP").ok().and_then(|v| v.parse().ok()).unwrap_or(1200) {
            let s = scooby_recomp::engine::runtime::J2(&g.rt.particles).physic(|m, _| {
                m.by_owner.iter().filter_map(|(&u, &id)| m.Physic(id).map(|p| (u, p))).filter(|(_, p)| (p.model.bsphere_world[0] - a[0]).powi(2) + (p.model.bsphere_world[2] - a[2]).powi(2) < 100.0).map(|(u, p)| format!("{u:#x}: flags {:#x} ctx {:#x} state {:#x} prims {} bs {:?} r {} {:?}", p.flags, p.context, p.state, p.model.prims.len(), p.model.bsphere_world, p.model.bsphere_radius, if u == lead || u == 0x2a00_0052 || std::env::var("GAME_PHYSDUMP_UID").ok().and_then(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16).ok()) == Some(u) { format!("{:?}", p.model.prims) } else { String::new() })).collect::<Vec<_>>()
            });
            eprintln!("game: physic {s:#?}");
            { let p = g.rt.particles.lock().unwrap(); eprintln!("game: world frame 52 {:?} 51 {:?}", p.world.frames.get(&0x2a00_0052), p.world.frames.get(&0x2a00_0051)); }
        }
    }
    // 'B': chase the nearest ghost (they flee the book)
    // WP-P14 'G' = 'B' but the target is the uncaptured monster nearest to the waypoint (x,z), within 25 m
    // (B picks the one nearest to the leader, which can be in another room)
    let (rx, rz, rmax) = match g.route.get(g.route_st.0) {
        Some(&(x, z, 'G')) => (x, z, 25.0f32 * 25.0),
        _ => (a[0], a[2], f32::MAX),
    };
    // WP-P14: a 'B'/'G' waypoint is also done once a monster was captured since the waypoint began
    // (L03B: the tome goes 3 -> 5 on a successful capture, state 4 is never seen)
    let nm = monsters(g).len();
    if g.route_nm.0 != g.route_st.0 {
        g.route_nm = (g.route_st.0, nm);
    }
    let captured = nm < g.route_nm.1;
    let ghost = {
        monsters(g)
            .iter()
            .filter_map(|&u| pos_of(g, u))
            .filter(|p| (p[0] - rx).powi(2) + (p[2] - rz).powi(2) < rmax)
            .min_by(|p, q| ((p[0] - rx).powi(2) + (p[2] - rz).powi(2)).total_cmp(&((q[0] - rx).powi(2) + (q[2] - rz).powi(2))))
            .map(|q| (q[0], q[2]))
    };
    // J19: book state / power trace
    {
        static LAST: std::sync::Mutex<(u32, u32, u32)> = std::sync::Mutex::new((u32::MAX, u32::MAX, 0));
        let l = g.e.get_game_object(tome, 0).map_or(0, |h| g.e.rt.obj(h).inst.local);
        let st = GetCurrentStateID(&mut g.e, tome);
        let sub = if l == 0 { 0 } else { g.e.mem.r32(l + 0x58) };
        let mut last = LAST.lock().unwrap();
        let b1 = gb(g, 1);
        let pw = if b1 == 0 { 0.0 } else { f32::from_bits(g.e.mem.r32(b1 + 4)) };
        if *last != (st, sub, pw.to_bits()) {
            *last = (st, sub, pw.to_bits());
            eprintln!("game: tome f{steps} state {st} sub {sub:#x} power {pw}");
        }
    }
    // WP-P14 'Z' (FAKE, harness teleport): put the leader at (x, y, z) through APIgameobj::SetTranslation (as
    // GAME_WARP) and go on. Used where the headless port blocks a real move (EP1L03B rotating door 0x7f).
    if let Some(&(x, z, 'Z')) = g.route.get(g.route_st.0) {
        let w = g.e.mem.alloca(0x10);
        for (k, c) in [x, a[1] + 0.2, z].iter().enumerate() {
            g.e.mem.w32(w + 4 * k as u32, c.to_bits());
        }
        scooby_recomp::engine::APIgameobj::SetTranslation(&mut g.e, lead, w);
        // R43: warp the follower too (a follower left behind, e.g. on the EP2L01A siege tower, breaks captures)
        let other = { let b0 = gb(g, 0); if b0 == 0 { 0 } else { g.e.mem.r32(b0 + 0xc) } };
        if other != 0 && other != lead {
            g.e.mem.w32(w, (x - 1.0).to_bits());
            scooby_recomp::engine::APIgameobj::SetTranslation(&mut g.e, other, w);
        }
        eprintln!("game: route FAKE warp leader {lead:#x} to ({x},{z}) at frame {steps}");
    }
    // WP-P14 'K' (FAKE progression bits): "mask,0,K" ORs the decimal mask into GB2+0xc (door lock word)
    if let Some(&(m, _, 'K')) = g.route.get(g.route_st.0) {
        let b = gb(g, 2) + 0xc;
        let v = g.e.mem.r32(b) | m as u32;
        g.e.mem.w32(b, v);
        eprintln!("game: route FAKE GB2+0xc |= {:#x} -> {v:#x} at frame {steps}", m as u32);
    }
    // WP-P21 R28 'Q' (FAKE episode-start carry): "mask,off,Q" ORs the decimal mask into GB2+off (EP3/L01
    // `31,0,Q` = TITLE EP3 select GB2+0 0x1f, until a real EP2-end carry exists)
    if let Some(&(m, o, 'Q')) = g.route.get(g.route_st.0) {
        let b = gb(g, 2) + o as u32;
        let v = g.e.mem.r32(b) | m as u32;
        g.e.mem.w32(b, v);
        eprintln!("game: route FAKE GB2+{:#x} |= {:#x} -> {v:#x} at frame {steps}", o as u32, m as u32);
    }
    // WP-P20 'F': "id,power,F" chase instance 0x2a000000+id (EP1L04 fakeghost 0x33: surprised within 1 m it
    // drops wisps, FakeGhostFUNC_DropWisp) until the tome power reaches `power`
    // P20 R172 'A': "uid,dist,A" = walk under monster 0x2a000000+uid (live position) while it idles (state 0xf);
    // done within `dist` m (2D); while it is not idle the bot stands still (state-based wait, as the step-script
    // `until`). The next 'G' then opens the book with the monster in front and in range (EP1/L03B GAME_EMCHILD:
    // opening at 5 m under a ghost hovering 4 m up = no target in range, the ghost flees)
    let atgt = match g.route.get(g.route_st.0) {
        Some(&(id, _, 'A')) => {
            let u = 0x2a00_0000 + id as u32;
            pos_of(g, u).map(|q| (q[0], q[2], GetCurrentStateID(&mut g.e, u) == 0xf))
        }
        _ => None,
    };
    let (ftgt, fpow) = match g.route.get(g.route_st.0) {
        Some(&(id, _, 'F')) => {
            let b1 = gb(g, 1);
            (pos_of(g, 0x2a00_0000 + id as u32), if b1 == 0 { 0.0 } else { f32::from_bits(g.e.mem.r32(b1 + 4)) })
        }
        _ => (None, 0.0),
    };
    // route state: (index, last pos, best dist, frame of progress, cut-scene seen)
    // (P20 R136: GAME_FLOG also lists the monsters within 8 m of the leader: id@distance/state)
    let flog_ms: Vec<String> = if steps % 60 == 0 && std::env::var_os("GAME_FLOG").is_some() {
        let near: Vec<(u32, f32)> = monsters(g).into_iter().filter_map(|u| pos_of(g, u).map(|p| (u, ((p[0] - a[0]).powi(2) + (p[2] - a[2]).powi(2)).sqrt()))).filter(|&(_, d)| d < 8.0).collect();
        near.into_iter().map(|(u, d)| format!("{:#x}@{d:.1}/{:#x}", u, GetCurrentStateID(&mut g.e, u))).collect()
    } else { Vec::new() };
    let (i, last, best, at, seen) = &mut g.route_st;
    if *at == 0 {
        (*last, *best, *at) = (a, f32::MAX, steps);
    }
    let jumped = (a[0] - last[0]).powi(2) + (a[2] - last[2]).powi(2) > 100.0;
    *last = a;
    let &(x, z, act) = g.route.get(*i)?;
    // P20 R142 'H': "N,0,H" = idle N frames (step-script `f N`), "N,1,H" = hold X N frames (`x N`).
    // 'V': "uid,off,V" = press X (30 on / 40 off) until local+off of 0x2a000000+uid is 0 (step-script `until`)
    if act == 'H' || act == 'V' {
        let done = if act == 'H' {
            steps >= *at + x as u32
        } else {
            let l = g.e.get_game_object(0x2a00_0000 + x as u32, 0).map_or(0, |h| g.e.rt.obj(h).inst.local);
            l != 0 && g.e.mem.r32(l + z as u32) == 0
        };
        if done || act == 'V' && steps > *at + 600 {
            eprintln!("game: route wp {i} ({x},{z},{act}) {} at frame {steps}, leader {a:?}", if done { "reached" } else { "STUCK, skipped" });
            *i += 1;
            (*best, *at, *seen) = (f32::MAX, steps, false);
        } else if act == 'H' && z == 1.0 || act == 'V' && (steps - *at) % 70 < 30 {
            p.buttons = pad::CROSS;
        }
        return Some(p);
    }
    let act = if act == 'G' { 'B' } else { act };
    let (x, z) = if act == 'B' { ghost.unwrap_or((x, z)) } else { (x, z) };
    // WP-P20 R10: 'F' chases only while the target is within 10 m of the anchor (the leader position when
    // the F waypoint began: reachable by construction; the fakeghost also roams into rooms the leader cannot reach) and the leader is not stuck
    // (moved < 0.5 m over 240 frames -> 240 frames back to the anchor); otherwise wait at the anchor
    let (x, z) = match (act, ftgt) {
        ('F', Some(q)) => {
            static FST: std::sync::Mutex<(u32, [f32; 3], u32, usize, (f32, f32))> = std::sync::Mutex::new((0, [0.0; 3], 0, usize::MAX, (0.0, 0.0)));
            let mut f = FST.lock().unwrap();
            if f.3 != *i || steps < f.0 {
                (f.3, f.4) = (*i, (a[0], a[2]));
            }
            let anc = f.4;
            if steps < f.0 || steps >= f.0 + 240 {
                let moved = (a[0] - f.1[0]).powi(2) + (a[2] - f.1[2]).powi(2);
                if steps >= f.0 + 240 && steps < f.0 + 480 && moved < 0.25 && steps >= f.2 {
                    f.2 = steps + 240;
                }
                (f.0, f.1) = (steps, a);
            }
            let near = (q[0] - anc.0).powi(2) + (q[2] - anc.1).powi(2) < 100.0;
            // debug (P22 R12): GAME_FLOG=1 traces the 'F' chase every 60 frames
            if steps % 60 == 0 && std::env::var("GAME_FLOG").is_ok() {
                eprintln!("game: F f{steps} leader {:.1},{:.1} tgt {:.1},{:.1} anchor {:.1},{:.1} near {near} pow {fpow} m {flog_ms:?}", a[0], a[2], q[0], q[2], anc.0, anc.1);
            }
            if near && steps >= f.2 { (q[0], q[2]) } else { anc }
        }
        ('A', _) => atgt.map_or((a[0], a[2]), |q| (q.0, q.1)),
        _ => (x, z),
    };
    let d = ((x - a[0]).powi(2) + (z - a[2]).powi(2)).sqrt();
    *seen |= playing;
    // the bot is not run during a cut-scene: a gap in the calls = a cut-scene played meanwhile
    {
        static LASTCALL: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let lc = LASTCALL.swap(steps, std::sync::atomic::Ordering::Relaxed);
        if lc != 0 && steps > lc + 30 {
            *seen = true;
            *at = steps;
        }
    }
    let done = match act {
        'X' => jumped,
        // WP-P20 R28: a 'J' press that ends in a door teleport (EP2/L01A crawl into door 0x125) is done too
        'J' => jumped || d < 0.6,
        // P7 'Y': X at a hot-spot that moves the team a short way (crawl): done once the leader,
        // after having been within 2.5, is 2.8 away again
        'Y' => *best < 2.5 && d > 2.8,
        'W' => *seen && !playing,
        // 'P': press X once at a hot-spot (a button toggles on every press)
        'P' => d < 1.0 && steps % 20 == 2,
        // P7 'O': press circle once wherever the leader is (switch leader, heroCB_INPUT_Switch_Leader)
        'O' => steps % 20 == 2,
        'B' => captured || GetCurrentStateID(&mut g.e, tome) == 4,
        'Z' | 'K' | 'Q' => true,
        'F' => fpow >= g.route[*i].1,
        'A' => atgt.is_some_and(|q| q.2) && d < g.route[*i].1,
        // 'R' (race food): the food collision is ~1 m wide
        'R' => d < 1.0,
        _ => d < 0.6,
    };
    if d < *best - 0.2 || playing {
        // (a cut-scene freezes the team: not stuck)
        (*best, *at) = (d.min(*best), steps);
    }
    let stuck = steps > *at + match act { 'B' => 1800, 'F' | 'A' => 3000, 'P' => 240, _ => 600 };
    if done || stuck {
        eprintln!("game: route wp {i} ({x},{z},{act}) {} at frame {steps}, leader {a:?}", if done { "reached" } else { "STUCK, skipped" });
        *i += 1;
        (*best, *at, *seen) = (f32::MAX, steps, false);
        return Some(p);
    }
    if act == 'A' && !atgt.is_some_and(|q| q.2) {
        return Some(p);
    }
    // plain waypoints: steer along the collision-map BFS path (walls between waypoints)
    // P20 R142 'N': nav-mesh goto (engine::nav::nav_path, as the step-script `goto`)
    let t = if act == 'N' && d > 1.0 {
        nav_step(g, a, [x, a[1], z], steps)
    } else if (act == ' ' || act == 'R' || act == 'B' || act == 'F' || act == 'A') && d > 1.0 { path_step(g, a, [x, a[1], z], steps).unwrap_or([x, 0.0, z]) } else { [x, 0.0, z] };
    let dir = norm([t[0] - a[0], 0.0, t[2] - a[2]]);
    let fwd = norm([c.at[0], 0.0, c.at[2]]);
    let right = norm(cross([0.0, 1.0, 0.0], fwd));
    if d > 0.3 {
        p.lx = stick(-(dir[0] * right[0] + dir[2] * right[2]));
        p.ly = stick(-(dir[0] * fwd[0] + dir[2] * fwd[2]));
    }
    // WP-P21 R31 'R': plain waypoint + X pulses (EP3/L01 food race: X = burp, heroCB_INPUT_Burp, restores speed)
    if ((act == 'X' || act == 'Y') && d < 2.5 || act == 'P' && d < 1.0 || act == 'J' || act == 'R') && steps % 20 < 2 {
        p.buttons = pad::CROSS;
    }
    // 'T' / 'C' / 'S': hold triangle / circle / square while walking
    match act {
        'T' => p.buttons = pad::TRIANGLE,
        'C' => p.buttons = pad::CIRCLE,
        'O' if steps % 20 < 2 => p.buttons = pad::CIRCLE,
        'S' => p.buttons = pad::SQUARE,
        // 'B': book capture: hold square towards (x,z); while the tome is in capture mode
        // (TomeDoom local +0x58 == 0x53) mash the requested button (local +0x1e8: 2 X, 4 O, 8 T)
        'B' => {
            // open the book only when a ghost is close (an open book makes the ghosts flee)
            let near = monsters(g)
                .iter()
                .filter_map(|&u| pos_of(g, u))
                .map(|q| ((q[0] - a[0]).powi(2) + (q[2] - a[2]).powi(2)).sqrt())
                .fold(f32::MAX, f32::min);
            let l = g.e.get_game_object(tome, 0).map_or(0, |h| g.e.rt.obj(h).inst.local);
            // WP-P16: with the book closed (state 1) pulse square: holding it through a give-up
            // (book back to state 1) never re-opens it (the press is edge-triggered)
            let closed = GetCurrentStateID(&mut g.e, tome) == 1;
            // WP-P18: TomeDoomST_OPEN sub 0x54 = open, no monster within 5.7 m in front (ScanForMonster); the
            // monster can flee out of range and the hero cannot walk with the book open (it stays open ~30 s):
            // after 2 s pulse square (heroCB_INPUT_Tome toggles) to close it, walk closer, re-open
            let scanning = GetCurrentStateID(&mut g.e, tome) == 3 && l != 0 && g.e.mem.r32(l + 0x58) == 0x54;
            g.route_scan = if !scanning { 0 } else if g.route_scan == 0 { steps } else { g.route_scan };
            // WP-P20: sub 0x53 (capture) stays set after the capture with the book back in state 1: capture
            // mode only while the tome is open (state 3), else the bot froze the stick and held square
            let capturing = l != 0 && g.e.mem.r32(l + 0x58) == 0x53 && GetCurrentStateID(&mut g.e, tome) == 3;
            if scanning && steps > g.route_scan + 120 {
                p.buttons = if steps % 20 < 10 { pad::SQUARE } else { 0 };
            } else if (near < 5.0 && (!closed || steps % 20 < 10)) || capturing {
                p.buttons = pad::SQUARE;
            }
            let _ = 0;
            let l = g.e.get_game_object(tome, 0).map_or(0, |h| g.e.rt.obj(h).inst.local);
            if std::env::var_os("GAME_DEBUG").is_some() && steps % 30 == 0 && l != 0 {
                eprintln!("game: tome near {near} f{steps} state {} sub {:#x} btn {:#x} life {} power {} target {:#x} ghosts {:?}", GetCurrentStateID(&mut g.e, tome), g.e.mem.r32(l + 0x58), g.e.mem.r32(l + 0x1e8), f32::from_bits(g.e.mem.r32(l + 0x70)), f32::from_bits(g.e.mem.r32(gb(g, 1) + 4)), g.e.mem.r32(l + 0x60), [0x2a00_0029u32, 0x2a00_002c].map(|u| { let gl = g.e.get_game_object(u, 0).map_or(0, |h| g.e.rt.obj(h).inst.local); (GetCurrentStateID(&mut g.e, u), pos_of(g, u), if gl == 0 { [0.0; 6] } else { [0x17c, 0x180, 0x184].map(|o| f32::from_bits(g.e.mem.r32(gl + o))).into_iter().chain([0x178, 0x174, 0x188].map(|o| g.e.mem.r32(gl + o) as f32)).collect::<Vec<_>>().try_into().unwrap() }) }));
            }
            if capturing {
                // GAME_MASH=none (no press) / wrong (always a wrong button): the failure paths
                let mode = std::env::var("GAME_MASH").unwrap_or_default();
                if steps % 4 < 2 && mode != "none" {
                    let want = g.e.mem.r32(l + 0x1e8);
                    let want = if mode == "wrong" { if want == 2 { 4 } else { 2 } } else { want };
                    p.buttons |= match want {
                        2 => pad::CROSS,
                        4 => pad::CIRCLE,
                        8 => pad::TRIANGLE,
                        _ => 0,
                    };
                }
                p.lx = 0x80;
                p.ly = 0x80;
            }
        }
        _ => {}
    }
    Some(p)
}

/// --autoplay harness: grid BFS (0.5 m) over the stage collision map (APIphysic ray tests) from
/// the leader to `goal`; returns the next path point to steer to. Recomputed every 120 frames.
/// P20 R142: next nav_path point >= 1 m ahead of the nearest one (replanned every 240 frames, on a new goal or
/// when the leader moved < 0.5 m in 90 frames), as the P17 step-script `goto`
fn nav_step(g: &mut Game, a: [f32; 3], goal: [f32; 3], steps: u32) -> [f32; 3] {
    static NAV: std::sync::Mutex<(u32, [f32; 3], Vec<[f32; 3]>, u32, [f32; 3])> = std::sync::Mutex::new((0, [0.0; 3], Vec::new(), 0, [0.0; 3]));
    let d2 = |p: [f32; 3], q: [f32; 3]| (p[0] - q[0]).powi(2) + (p[2] - q[2]).powi(2);
    let mut n = NAV.lock().unwrap();
    let mut replan = steps >= n.0 + 240 || steps < n.0 || d2(n.1, goal) > 0.01 || n.2.is_empty();
    if steps >= n.3 + 90 || steps < n.3 {
        replan |= d2(n.4, a) < 0.25;
        (n.3, n.4) = (steps, a);
    }
    if replan {
        if g.path_scr == 0 {
            g.path_scr = g.e.mem.malloc(0x80);
        }
        let j2 = scooby_recomp::engine::runtime::J2(&g.rt.particles);
        n.2 = scooby_recomp::engine::nav::nav_path(&mut g.e.mem, &j2, g.path_scr, a, goal, 15.0);
        (n.0, n.1) = (steps, goal);
    }
    let k = (0..n.2.len()).min_by(|&u, &v| d2(n.2[u], a).total_cmp(&d2(n.2[v], a)));
    k.and_then(|k| n.2[k..].iter().find(|&&q| d2(q, a) > 1.0).copied()).unwrap_or(goal)
}

fn path_step(g: &mut Game, a: [f32; 3], goal: [f32; 3], steps: u32) -> Option<[f32; 3]> {
    static PATH: std::sync::Mutex<(u32, [f32; 3], Vec<[f32; 3]>)> = std::sync::Mutex::new((0, [0.0; 3], Vec::new()));
    let d2 = |p: [f32; 3], q: [f32; 3]| (p[0] - q[0]).powi(2) + (p[2] - q[2]).powi(2);
    let mut pg = PATH.lock().unwrap();
    if steps >= pg.0 + 120 || steps < pg.0 || d2(pg.1, goal) > 1.0 {
        if g.path_scr == 0 {
            g.path_scr = g.e.mem.malloc(0x80);
        }
        let scr = g.path_scr;
        let j2 = scooby_recomp::engine::runtime::J2(&g.rt.particles);
        let mem = &mut g.e.mem;
        let mut hit = |o: [f32; 3], dv: [f32; 3]| {
            let w = [o[0], o[1], o[2], dv[0], dv[1], dv[2]].map(f32::to_bits);
            for (i, v) in w.iter().chain(&[0, 0, 3, 0, 0]).enumerate() {
                mem.w32(scr + 4 * i as u32, *v);
            }
            j2.ray_intersect(mem, scr, scr + 0x30) != 0
        };
        const S: f32 = 0.5;
        const N: i32 = 120; // +-60 m
        let y = a[1] + 0.6;
        let cell = |i: i32, j: i32| [a[0] + (i - N / 2) as f32 * S, y, a[2] + (j - N / 2) as f32 * S];
        let gi = (((goal[0] - a[0]) / S).round() as i32 + N / 2).clamp(0, N - 1);
        let gj = (((goal[2] - a[2]) / S).round() as i32 + N / 2).clamp(0, N - 1);
        let mut prev = vec![-2i32; (N * N) as usize];
        let mut q = std::collections::VecDeque::new();
        let s0 = N / 2 * N + N / 2;
        prev[s0 as usize] = -1;
        q.push_back(s0);
        let mut found = None;
        let mut best = (f32::MAX, s0);
        while let Some(c) = q.pop_front() {
            let (i, j) = (c / N, c % N);
            let p = cell(i, j);
            if d2(p, goal) < best.0 {
                best = (d2(p, goal), c);
            }
            if i == gi && j == gj {
                found = Some(c);
                break;
            }
            for (di, dj) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (ni, nj) = (i + di, j + dj);
                if ni < 0 || nj < 0 || ni >= N || nj >= N || prev[(ni * N + nj) as usize] != -2 {
                    continue;
                }
                let np = cell(ni, nj);
                // blocked sideways (with a body-width margin) or no floor below
                let dv = [(np[0] - p[0]) * 1.8, 0.0, (np[2] - p[2]) * 1.8];
                if hit(p, dv) || !hit(np, [0.0, -1.6, 0.0]) {
                    prev[(ni * N + nj) as usize] = -3;
                    continue;
                }
                prev[(ni * N + nj) as usize] = c;
                q.push_back(ni * N + nj);
            }
        }
        let mut c = found.unwrap_or(best.1);
        let mut path = Vec::new();
        while c >= 0 {
            path.push(cell(c / N, c % N));
            c = prev[c as usize];
        }
        path.reverse();
        *pg = (steps, goal, path);
    }
    // next point >= 1 m ahead of the nearest path point
    let k = (0..pg.2.len()).min_by(|&x, &y| d2(pg.2[x], a).total_cmp(&d2(pg.2[y], a)))?;
    pg.2[k..].iter().find(|&&p| d2(p, a) > 1.0).copied().or(Some(goal))
}

/// debug: GAME_MAP="x0,z0,x1,z1" prints a floor-height map (0.5 m cells, down rays from y=12):
/// '.' no floor, digit = floor height in m (rounded), '|'/'-' wall (x/z ray) at body height above the floor.
fn floor_map(g: &mut Game, v: &str) {
    let n: Vec<f32> = v.split(',').filter_map(|s| s.parse().ok()).collect();
    let [x0, z0, x1, z1] = n[..4] else { return };
    for (u, m) in g.rt.particles.lock().unwrap().world.frames.iter() {
        if (x0..=x1).contains(&m.pos[0]) && (z0..=z1).contains(&m.pos[2]) {
            eprintln!("game: map obj {u:#x} at {:?}", m.pos);
        }
    }
    let scr = g.e.mem.malloc(0x80);
    let j2 = scooby_recomp::engine::runtime::J2(&g.rt.particles);
    let mem = &mut g.e.mem;
    let mut ray = |o: [f32; 3], dv: [f32; 3]| -> Option<f32> {
        let w = [o[0], o[1], o[2], dv[0], dv[1], dv[2]].map(f32::to_bits);
        for (i, v) in w.iter().chain(&map_mask()).enumerate() {
            mem.w32(scr + 4 * i as u32, *v);
        }
        (j2.ray_intersect(mem, scr, scr + 0x30) != 0).then(|| f32::from_bits(mem.r32(scr + 0x34)))
    };
    let mut z = z0;
    while z <= z1 {
        let mut line = format!("{z:7.1} ");
        let mut x = x0;
        while x <= x1 {
            line.push(match ray([x, 12.0, z], [0.0, -14.0, 0.0]) {
                None => '.',
                Some(y) if ray([x - 0.3, y + map_h(), z], [0.6, 0.0, 0.0]).is_some() => '|',
                Some(y) if ray([x, y + map_h(), z - 0.3], [0.0, 0.0, 0.6]).is_some() => '-',
                Some(y) if ray([x + 0.3, y + map_h(), z], [-0.6, 0.0, 0.0]).is_some() => '!',
                Some(y) if ray([x, y + map_h(), z + 0.3], [0.0, 0.0, -0.6]).is_some() => '=',
                Some(y) => char::from_digit(y.round().clamp(0.0, 9.0) as u32, 10).unwrap(),
            });
            x += 0.5;
        }
        eprintln!("game: map {line}");
        z += 0.5;
    }
}

/// debug: GAME_SHOT_AT="name:frame,..." takes extra screenshots at the given runtime frames
/// debug (WP-P1): GAME_PADSCRIPT="down@400,cross@460,start@900:20" holds the named pad button
/// (cross circle square triangle start select up down left right l1 r1 l2 r2) from frame for N frames (default 6).
fn padscript(f: u32, mut p: PadState) -> PadState {
    let Ok(v) = std::env::var("GAME_PADSCRIPT") else { return p };
    for it in v.split(',') {
        let Some((name, at)) = it.trim().split_once('@') else { continue };
        let (at, dur) = at.split_once(':').map(|(a, d)| (a, d.parse().unwrap_or(6))).unwrap_or((at, 6));
        let Ok(at) = at.parse::<u32>() else { continue };
        let bit = match name {
            "cross" => pad::CROSS, "circle" => pad::CIRCLE, "square" => pad::SQUARE, "triangle" => pad::TRIANGLE,
            "start" => pad::START, "select" => pad::SELECT, "up" => pad::UP, "down" => pad::DOWN,
            "left" => pad::LEFT, "right" => pad::RIGHT, "l1" => pad::L1, "r1" => pad::R1, "l2" => pad::L2, "r2" => pad::R2,
            _ => 0,
        };
        if f >= at && f < at + dur {
            p.buttons |= bit;
            // P24: lup/ldown/lleft/lright = left stick full deflection (menu navigation by stick)
            match name {
                "lup" => p.ly = 0,
                "ldown" => p.ly = 0xff,
                "lleft" => p.lx = 0,
                "lright" => p.lx = 0xff,
                _ => {}
            }
        }
    }
    p
}

fn shot_at(steps: u32, done: &std::collections::HashSet<&'static str>) -> Option<&'static str> {
    let v = std::env::var("GAME_SHOT_AT").ok()?;
    for it in v.split(',') {
        let (n, f) = it.split_once(':')?;
        if f.parse::<u32>().ok()? <= steps && !done.contains(n) {
            return Some(Box::leak(n.to_string().into_boxed_str()));
        }
    }
    None
}

/// debug: GAME_POSEDUMP="a-b" dumps poses / node frames for runtime frames a..=b
fn posedump(steps: u32) -> bool {
    let Ok(v) = std::env::var("GAME_POSEDUMP") else { return false };
    let (a, b) = v.split_once('-').unwrap_or((&v, &v));
    a.parse::<u32>().is_ok_and(|a| a <= steps) && b.parse::<u32>().is_ok_and(|b| steps <= b)
}

/// debug: GAME_MAP_MASK="om1,om2,m1,m2" ray masks for GAME_MAP (default 0,0,3,0)
fn map_mask() -> [u32; 5] {
    let v: Vec<u32> = std::env::var("GAME_MAP_MASK").unwrap_or_default().split(',').filter_map(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok()).collect();
    if v.len() == 4 { [v[0], v[1], v[2], v[3], 0] } else { [0, 0, 3, 0, 0] }
}

/// debug: GAME_MAP_H = wall-test height above the floor for GAME_MAP (default 0.8)
fn map_h() -> f32 {
    std::env::var("GAME_MAP_H").ok().and_then(|v| v.parse().ok()).unwrap_or(0.8)
}

// ---------------------------------------------------------------------------------------------
// WP-P3: FMV display + audio

/// MpegPlay display/sound side: a fullscreen textured quad on its own camera (order 10, clears to
/// black like clearGsMem; covers the level passes) showing the current picture, and the stereo
/// stream as two hard-panned voices at `_gfStreamVolume`. When MpegPlay has returned the entities
/// go away and EFVideoPlayer_SP::Play's tail runs.
fn movie_present(world: &mut World) {
    world.resource_scope(|world, mut game: Mut<Game>| {
        let g = &mut *game;
        let Some(mv) = g.movie.as_mut() else { return };
        if mv.play.done {
            for e in mv.ents.drain(..) {
                if let Ok(e) = world.get_entity_mut(e) {
                    e.despawn();
                }
            }
            eprintln!("game: movie end at frame {} (MpegPlay -> {}, {} pictures, {:.2} s video @ {} fps, {:.2} s audio, t {:.2})", g.steps, mv.play.MpgSkip as u32, mv.play.frame_count, mv.play.frame_count as f32 / mv.play.fps.max(1.0), mv.play.fps, mv.play.audio_secs, mv.play.t);
            capture_with(|c| c.retain(&|s| s < CAP_MOVIE));
            if let Some(r) = &g.level_req {
                let mut pack = r.pack.clone();
                mv.player.PlayEnd(mv.play.MpgSkip, &mut pack); // never fires (quirk), pack already loading
            }
            g.movie = None;
            return;
        }
        if mv.image.is_none() {
            let img = Image::new_fill(
                bevy::render::render_resource::Extent3d { width: 720, height: 480, depth_or_array_layers: 1 },
                bevy::render::render_resource::TextureDimension::D2,
                &[0, 0, 0, 255],
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::default(),
            );
            let h = world.resource_mut::<Assets<Image>>().add(img);
            let mat = world.resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial { base_color_texture: Some(h.clone()), unlit: true, ..default() });
            // P36: the picture is a 4:3 quad on a camera over the WHOLE target (no viewport, AutoMin
            // keeps 4:3): the black clear covers the Hor+ side strips. As a HudCam (4:3 viewport) the
            // clear stopped at the viewport and the level loading behind the movie showed at the sides.
            let mesh = world.resource_mut::<Assets<Mesh>>().add(Rectangle::new(SCREEN.0 as f32, SCREEN.0 as f32 * 0.75));
            let target = world.get_resource::<Target>().map(|t| t.0.clone());
            let mut c = world.spawn((
                Camera3d::default(),
                Camera { order: 10, clear_color: ClearColorConfig::Custom(Color::BLACK), ..default() },
                Tonemapping::None,
                RenderLayers::layer(7),
                Projection::Orthographic(OrthographicProjection { scaling_mode: ScalingMode::AutoMin { min_width: SCREEN.0 as f32, min_height: SCREEN.0 as f32 * 0.75 }, near: 0.0, far: 100.0, ..OrthographicProjection::default_3d() }),
                Transform::from_xyz(0.0, 0.0, 50.0),
            ));
            if let Some(t) = target {
                c.insert(RenderTarget::Image(t.into()));
            }
            let cam = c.id();
            let quad = world.spawn((Mesh3d(mesh), MeshMaterial3d(mat), Transform::IDENTITY, RenderLayers::layer(7))).id();
            // P36: opaque black backdrop behind the picture: the camera clear alone left the level
            // that loads behind the movie visible in the 16:9 side strips (INTRO room, story movies)
            let bm = world.resource_mut::<Assets<Mesh>>().add(Rectangle::new(SCREEN.0 as f32 * 16.0, SCREEN.1 as f32 * 16.0));
            let bmat = world.resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial { base_color: Color::BLACK, unlit: true, ..default() });
            let back = world.spawn((Mesh3d(bm), MeshMaterial3d(bmat), Transform::from_xyz(0.0, 0.0, -10.0), RenderLayers::layer(7))).id();
            mv.ents = vec![cam, quad, back];
            mv.image = Some(h);
        }
        if let Some(f) = mv.play.take_new_frame() {
            let data = f.rgba.clone();
            if let Some(mut img) = world.resource_mut::<Assets<Image>>().get_mut(mv.image.as_ref().unwrap()) {
                img.data = Some(data);
            }
        }
        // P36 R2: GAME_AUDIO_WAV: the movie soundtrack goes to the offline mix (Movie bus)
        if !g.audio && capture_on() && !mv.audio_spawned && mv.play.started {
            mv.audio_spawned = true;
            if let Some((rate, pcm)) = mv.play.audio.take() {
                let vol = mv.play.volume * game_settings::gain(game_settings::Cat::Voice);
                capture_with(|c| c.spawn_pcm(CAP_MOVIE, rate, pcm, vol));
            }
        }
        if g.audio && !mv.audio_spawned && mv.play.started {
            mv.audio_spawned = true;
            mv.clock = Some(std::time::Instant::now());
            if let Some((rate, pcm)) = mv.play.audio.take() {
                let side = |side: usize, l: f32, r: f32| {
                    let mono: Vec<i16> = pcm.iter().skip(side).step_by(2).copied().collect();
                    bevy_audio::AudioSource { bytes: scooby_recomp::engine::audio::wav::pcm_to_wav(&mono, rate, l, r).into() }
                };
                let len = pcm.len() as f64 / 2.0 / rate.max(1) as f64;
                mv.snd = Some((side(0, 1.0, 0.0), side(1, 0.0, 1.0), len, mv.play.volume));
            }
        }
        // P36 R6: start the soundtrack once an output stream is open; after a device loss + reopen
        // (new generation) restart it at the movie clock's position so it stays in sync with the picture
        if let (true, Some((sl, sr, len, vol)), Some(c)) = (g.audio, mv.snd.as_ref(), mv.clock) {
            if !g.loaded {
                audio_device_watch(world, g.steps); // boot logos: play_audio only watches once a level is loaded
            }
            let cur = AUDIO_GEN.load(std::sync::atomic::Ordering::Relaxed);
            let avail = world.get_resource::<bevy_audio::AudioOutput>().is_some_and(|o| o.is_available());
            let el = c.elapsed().as_secs_f64();
            if let Some(at) = scooby_recomp::engine::audio::device::movie_audio_restart(mv.snd_gen, cur, avail, el, *len) {
                for e in mv.snd_ents.drain(..) {
                    if let Ok(e) = world.get_entity_mut(e) {
                        e.despawn();
                    }
                }
                if mv.snd_gen.is_some() {
                    eprintln!("audio: movie soundtrack restarted at {at:.2} s after the device reconnect");
                }
                let v = if sound_on() { vol * game_settings::gain(game_settings::Cat::Voice) } else { 0.0 };
                for src in [sl.clone(), sr.clone()] {
                    let h = world.resource_mut::<Assets<bevy_audio::AudioSource>>().add(src);
                    let ps = bevy_audio::PlaybackSettings::ONCE.with_volume(bevy_audio::Volume::Linear(v)).with_start_position(std::time::Duration::from_secs_f64(at));
                    let e = world.spawn((bevy_audio::AudioPlayer(h), ps)).id();
                    mv.snd_ents.push(e);
                    mv.ents.push(e);
                }
                mv.snd_gen = Some(cur);
            }
        }
        // GAME_SHOT_AT also works while a movie plays (shots_and_exit waits for a loaded level)
        if let (Some(dir), Some(n)) = (g.shots.clone(), shot_at(g.steps, &g.shot_done)) {
            let path = format!("{dir}/{n}.png");
            eprintln!("game: screenshot {path} at frame {} (movie picture {})", g.steps, mv.play.frame_count);
            let t = world.get_resource::<Target>().map(|t| t.0.clone());
            let mut cmd = match t {
                Some(t) => world.spawn(Screenshot::image(t)),
                None => world.spawn(Screenshot::primary_window()),
            };
            cmd.observe(save_to_disk(path));
            g.shot_done.insert(n);
        }
    });
}

/// debug (WP-P4): GAME_PROBE="x0,z0,x1,z1" prints the collision floor height (ray down from y+20)
/// on a 1 m grid: rows = z, columns = x, one char per cell ('.' none, digit = height/1 m, '+' > 9).
fn probe_heights(g: &mut Game, v: &str) {
    let n: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
    let (x0, z0, x1, z1) = match n[..] { [a, b, c, d, ..] => (a, b, c, d), _ => return };
    // optional 5th/6th values: grid step (default 1 m) and ray start height (default 20)
    let (step, top) = (n.get(4).copied().unwrap_or(1.0), n.get(5).copied().unwrap_or(20.0));
    // optional 7th value: horizontal ray length along +z from (x, top, z) (wall probe: prints hit dz)
    let horiz = n.get(6).copied();
    let scr = g.e.mem.malloc(0x80);
    let j2 = scooby_recomp::engine::runtime::J2(&g.rt.particles);
    let mem = &mut g.e.mem;
    let mut first = true;
    let mut z = z0;
    while z <= z1 {
        let mut row = String::new();
        let mut x = x0;
        while x <= x1 {
            let w = if let Some(l) = horiz { [x, top, z, 0.0, 0.0, l] } else { [x, top, z, 0.0, -40.0, 0.0] }.map(f32::to_bits);
            for (i, v) in w.iter().chain(&[0, 0, 3, 0, 0]).enumerate() {
                mem.w32(scr + 4 * i as u32, *v);
            }
            if j2.ray_intersect(mem, scr, scr + 0x30) != 0 {
                if first {
                    first = false;
                    eprintln!("probe out {:?}", (0..8).map(|i| f32::from_bits(mem.r32(scr + 0x30 + 4 * i))).collect::<Vec<_>>());
                }
                let y = if horiz.is_some() { f32::from_bits(mem.r32(scr + 0x38)) - z } else { f32::from_bits(mem.r32(scr + 0x34)) };
                row.push(if y > 9.5 { '+' } else if y < -0.5 { '-' } else { char::from(b'0' + y.round() as u8) });
            } else {
                row.push('.');
            }
            x += step;
        }
        eprintln!("probe z {z:6.1} {row}");
        z += step;
    }
}

/// WP-P20 R36 debug: GAME_LEDGES=x0,z0,x1,z1 lists the collision-map polygons with material word +4 bit
/// 0x8000000 (the ledge mask EFDMEdgeDetect tests against the hero edge prim, tag 0x66) inside the box:
/// edge mask (prim short +4), normal and the vertex bounds.
/// WP-P20 R37: GAME_POLYS=x0,z0,x1,z1 = the same dump for every polygon (all material words).
fn ledges_dump(g: &mut Game, v: &str, ledges: bool) {
    use scooby_recomp::engine::physic::collision_map::EFGetPrimitivesInSphereData;
    use scooby_recomp::engine::physic::objcoll_check::EnvPrim;
    let n: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
    let &[x0, z0, x1, z1] = &n[..] else { return };
    let (cx, cz) = ((x0 + x1) / 2.0, (z0 + z1) / 2.0);
    let r = ((x1 - x0).powi(2) + (z1 - z0).powi(2)).sqrt() / 2.0 + 50.0;
    let out = scooby_recomp::engine::runtime::J2(&g.rt.particles).physic(|m, _| {
        let mut d = EFGetPrimitivesInSphereData { center: [cx, 0.0, cz], radius: r, mask1: 0xffff_ffff, mask2: 0xffff_ffff, out: Vec::new(), capacity: 1_000_000 };
        m.query_stamp = m.query_stamp.wrapping_add(1);
        let st = m.query_stamp;
        let mut v = Vec::new();
        for (mi, map) in m.maps.iter_mut().enumerate() {
            d.out.clear();
            map.GetPrimitivesInSphere(&mut d, st);
            for &p in &d.out {
                let e = EnvPrim::from_map(map, p);
                if (ledges && e.mat[1] & 0x800_0000 == 0) || e.verts.is_empty() {
                    continue;
                }
                let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
                for q in &e.verts {
                    for k in 0..3 {
                        lo[k] = lo[k].min(q[k]);
                        hi[k] = hi[k].max(q[k]);
                    }
                }
                if hi[0] < x0 || lo[0] > x1 || hi[2] < z0 || lo[2] > z1 {
                    continue;
                }
                v.push(format!("game: map{mi} {} x {:.1}..{:.1} y {:.1}..{:.1} z {:.1}..{:.1} edgemask {:#x} n [{:.2},{:.2},{:.2}] mat {:x?}", if ledges { "ledge" } else { "poly" }, lo[0], hi[0], lo[1], hi[1], lo[2], hi[2], e.w4, e.normal[0], e.normal[1], e.normal[2], e.mat));
            }
        }
        v
    });
    for l in &out {
        eprintln!("{l}");
    }
    eprintln!("game: GAME_LEDGES {} polys in {x0},{z0}..{x1},{z1}", out.len());
}

/// WP-P20 R29 debug: GAME_FLOOD=x0,z0,x1,z1,step,file dumps the walkable surface layers of every grid cell
/// (down rays from y 30, re-cast below each hit; up to 4 layers) with wall flags towards +x / +z (horizontal
/// rays 0.4 m above the surface, length = step) as lines "x z y wx wz [y wx wz ..]" for tools/p20_flood.py.
fn flood_dump(g: &mut Game, v: &str) {
    let it: Vec<&str> = v.split(',').collect();
    let n: Vec<f32> = it.iter().take(5).filter_map(|x| x.trim().parse().ok()).collect();
    let (Some(file), &[x0, z0, x1, z1, step]) = (it.get(5), &n[..]) else { return };
    let scr = g.e.mem.malloc(0x80);
    let j2 = scooby_recomp::engine::runtime::J2(&g.rt.particles);
    let mem = &mut g.e.mem;
    let mut ray = |o: [f32; 3], d: [f32; 3]| -> Option<f32> {
        let w = [o[0], o[1], o[2], d[0], d[1], d[2]].map(f32::to_bits);
        for (i, v) in w.iter().chain(&[0, 0, 3, 0, 0]).enumerate() {
            mem.w32(scr + 4 * i as u32, *v);
        }
        (j2.ray_intersect(mem, scr, scr + 0x30) != 0).then(|| f32::from_bits(mem.r32(scr + 0x34)))
    };
    let mut out = String::new();
    let mut z = z0;
    while z <= z1 {
        let mut x = x0;
        while x <= x1 {
            let mut line = format!("{x} {z}");
            let mut top = 30.0f32;
            for _ in 0..4 {
                let Some(y) = ray([x, top, z], [0.0, -(top + 20.0), 0.0]) else { break };
                let wx = ray([x, y + 0.4, z], [step, 0.0, 0.0]).is_some() as u8;
                let wz = ray([x, y + 0.4, z], [0.0, 0.0, step]).is_some() as u8;
                line += &format!(" {y:.2} {wx} {wz}");
                top = y - 0.3;
                if top < -20.0 {
                    break;
                }
            }
            out += &line;
            out.push(char::from(10u8));
            x += step;
        }
        z += step;
    }
    let _ = std::fs::write(file, out);
    eprintln!("game: GAME_FLOOD written to {file}");
}

/// debug (P32 R79): one line per skinned object (anim_bind): code name, uid, visible frame pos,
/// pose present, connected HAnim ids (none + no pose = bind/T-pose).
fn dump_anims(g: &Game, f: u32) {
    let p = g.rt.particles.lock().unwrap();
    let mut uids: Vec<u32> = p.sub.anim_bind.keys().copied().collect();
    uids.sort();
    for uid in uids {
        let b = &p.sub.anim_bind[&uid];
        let Some(m) = p.sub.anim.get(&uid) else { continue };
        let on: Vec<String> = (0..m.anims.len() as u32).filter_map(|k| m.anims[k as usize].filter(|&n| m.node(n).parent.is_some() && matches!(m.node(n).kind, scooby_recomp::engine::anim::mgr::NodeKind::HAnim(_))).map(|_| format!("{:#x}", m.base + k))).collect();
        let pose = m.GetPose(b.obj3d).is_some();
        // tree shape: live nodes / (type, state, parent?) of every node connected to anything
        let tree: String = if std::env::var_os("GAME_DUMP_ANIM_TREE").is_some() { m.nodes.iter().enumerate().filter_map(|(i, n)| n.as_ref().filter(|n| n.parent.is_some()).map(|n| format!(" n{i}:t{}s{:x}p{}", n.ty(), n.state(), n.parent.unwrap()))).collect() } else { String::new() };
        let o = g.e.rt.objects.iter().flatten().find(|o| o.id == uid);
        let name = o.and_then(|o| o.inst.code).map(|c| g.e.rt.codes[c].def.name.clone()).unwrap_or_default();
        let (st, sl) = o.map_or((u32::MAX, 0.0), |o| (o.inst.state, o.inst.sleep));
        // GAME_DUMP_ANIM_LOC=hex,hex..: + those local-data words (inside the local block)
        let loc: String = std::env::var("GAME_DUMP_ANIM_LOC").ok().into_iter().flat_map(|v| v.split(',').filter_map(|k| u32::from_str_radix(k, 16).ok()).collect::<Vec<_>>()).filter_map(|k| o.filter(|o| o.inst.local != 0 && k + 4 <= o.inst.local_size).map(|o| format!(" +{k:x}={:#x}", g.e.mem.r32(o.inst.local + k)))).collect();
        let pos = p.world.frames.get(&uid).map(|m| m.pos);
        eprintln!("anim f{f} {name} {uid:#x} pos {pos:?} hid {} st {st:#x} sl {sl} pose {pose} play [{}]{loc}{tree}", p.world.hidden.contains(&uid), on.join(","));
    }
}

/// debug (WP-P4): GAME_DUMP_DOORS=frame lists hs_door* instances: id, pos, lock mask +8, bits +0xc,
/// level name +0x18 (local data after FIRSTMOVE), plus GB2 +8/+0xc.
fn dump_doors(g: &Game) {
    let m = &g.e.mem;
    for o in g.e.rt.objects.iter().flatten() {
        let Some(c) = o.inst.code else { continue };
        let name = &g.e.rt.codes[c].def.name;
        if !name.starts_with("hs_") && !name.starts_with("chkpnt") && std::env::var_os("GAME_DUMP_ALL").is_none() {
            continue;
        }
        let l = o.inst.local;
        let lvl = if name.starts_with("hs_door") && l != 0 && o.inst.local_size >= 0x1c && m.r32(l + 0x18) != 0 { m.cstr_at(m.r32(l + 0x18)) } else { String::new() };
        // P32 R78: only within the local block (EP6L04B fireballbig_expl: 16-byte local at the heap top -> +0x10 read panicked)
        let w = |k: u32| if l != 0 && k + 4 <= o.inst.local_size { m.r32(l + k) } else { 0 };
        eprintln!("door {name} {:#x} pos {:?} tgt {:?} +0 {:#x} +4 {:#x} +8 {:#x} +c {:#x} +10 {:#x} lvl {lvl:?}", o.id, pos_of(g, o.id), pos_of(g, w(0)), w(0), w(4), w(8), w(0xc), w(0x10));
    }
    eprintln!("leaders {:?} {:?}", pos_of(g, 0x2a000092), pos_of(g, 0x2a000091));
    let b2 = gb(g, 2);
    if b2 != 0 {
        eprintln!("GB2 +8 {:#x} +c {:#x}", m.r32(b2 + 8), m.r32(b2 + 0xc));
    }
}

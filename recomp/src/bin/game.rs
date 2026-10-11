//! WP-J5: playable windowed build of a level (EP1L01 by default), focused on the book.
//! Feature `game`. The `GamePlugin` owns the engine (`Engine` + `EngineRuntime`) in one
//! resource; the level pack is read on a worker thread (no blocking I/O in systems) and applied
//! when it arrives. The runtime steps `EngineRuntime::Frame` at the PS2 vsync rate (1/60 s raw
//! frame time; EFEngineController::MainLoop 0x001fbef0 then filters it with ComputeDeltaTime:
//! clamp 0.0001..0.05, 10-frame average — done inside the runtime by `e.clock`).
//!
//! Usage: game [LEVEL] [--iso PATH] [--frames N] [--autoplay] [--give-book] [--headless] [--shots DIR]
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
//!      Q L1, E R1, Z L2, C R2, Enter START, Tab SELECT, numpad 8/2/4/6 d-pad, Esc quit.
//!      Any gamepad maps 1:1 (south X, east circle, north triangle, west square).
#[path = "game_fx/mod.rs"]
mod game_fx;
mod game_input;
// WP-P12: EP1 L03A/L03B/L04 routes (WP-P10 file)
#[path = "routes_ep1b.rs"]
mod routes_ep1b;
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
    let mut a = Args { level: "EP1/L01".into(), iso: String::new(), frames: None, autoplay: false, give_book: false, headless: false, shots: None, route: None, aspect: None, res: None, rt: false };
    let mut it = std::env::args().skip(1);
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
        a.iso = ["../Scooby Doo Mystery Mayhem.ISO", "Scooby Doo Mystery Mayhem.ISO", "../../Scooby Doo Mystery Mayhem.ISO"]
            .iter()
            .find(|p| std::path::Path::new(p).exists())
            .unwrap_or(&"../Scooby Doo Mystery Mayhem.ISO")
            .to_string();
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
    read_level_pack(&a.iso, &a.level, &format!("{}.P2R", a.level.replace('/', "")))
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
    /// N2: last scePadSetActDirect motor state sent to the host pad (small on, big 0..255)
    motors: (bool, u8),
    e: Engine,
    rt: EngineRuntime,
    loaded: bool,
    render: Option<RenderData>,
    /// EF3dObj_SP per game object (created on first sight: launched latents appear later)
    objs: HashMap<u32, EF3dObj_SP>,
    /// J8: EF3dObjs of EFHud3DWindows (owner uid, res uid) = GameObject+0x174[n]; draw-list owner
    /// of entry k = HUD_OWNER + k
    hud_objs: Vec<((u32, u32), EF3dObj_SP)>,
    /// EFGraphicsMgr the game objects render into (EF3dObj_SP::Render -> DrawCmd list)
    gmgr: EFGraphicsMgr,
    /// objects that had a logic instance at load (gone from the logic map = killed)
    had_logic: HashSet<u32>,
    pad: PadState,
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
    locwatch: Vec<u32>,
    /// EP1L01 autoplay: frame after the last cut-scene frame (the script runs relative to it)
    ap_base: u32,
    /// K5: GAME_BOOKTGTS state (target index, placed at frame, FINALSUCKING seen)
    k5: (usize, Option<u32>, bool),
    voices: HashMap<(u32, u32, u32), [Entity; 2]>,
    streams: StreamLoader,
    audio: bool,
    /// --route waypoints (x, z, action) for the EP1L03A bot
    route: Vec<(f32, f32, char)>,
    /// --route source + GAME_ROUTE_LEVEL (the level the route is for; default: the start level)
    route_src: Option<String>,
    route_level: String,
    /// route bot state: (waypoint index, last leader pos, best distance, frame of progress, cut-scene seen)
    route_st: (usize, [f32; 3], f32, u32, bool),
    iso: String,
    /// APIgame::LoadLevel request taken from EFEngineController flags & 8 (switch next update)
    level_req: Option<scooby_recomp::engine::host::game::LevelRequest>,
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
}

impl Movie {
    fn new(mut play: scooby_recomp::engine::video::MpegPlay, player: scooby_recomp::engine::video::EFVideoPlayer_SP, blocking: bool) -> Self {
        play.blocking = blocking;
        Movie { play, player, image: None, ents: Vec::new(), audio_spawned: false }
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
        app.insert_resource(PendingLoad(Mutex::new(rx)))
            .insert_resource(Game {
                motors: (false, 0),
                e,
                rt,
                loaded: false,
                render: None,
                objs: HashMap::new(),
                hud_objs: Vec::new(),
                gmgr: EFGraphicsMgr::new(SCREEN.0, SCREEN.1),
                had_logic: HashSet::new(),
                pad: idle_pad(),
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
                locwatch: Vec::new(),
                ap_base: 0,
                k5: (0, None, false),
                voices: HashMap::new(),
                streams: StreamLoader::new(a.iso.clone()),
                audio: !a.headless,
                route: parse_route(level_route(a.route.as_deref(), &a.level.replace(char::from(0x5c), "/"))),
                route_src: a.route.clone(),
                route_level: std::env::var("GAME_ROUTE_LEVEL").unwrap_or_else(|_| a.level.replace(char::from(0x5c), "/")),
                route_st: Default::default(),
                iso: a.iso.clone(),
                level_req: None,
                buf_carry: carry_load(),
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
            .add_systems(Startup, setup_cameras)
            .add_systems(Update, (switch_level, poll_load, read_pad, step_runtime, pad_rumble, movie_present, render_frame, apply_aspect, play_audio, shots_and_exit).chain());
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
        let play = player.Play(ini_skip, vol, false, movie_reader(g.iso.clone()));
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
    let mut g = world.resource_mut::<Game>();
    // EFGameMemoryMgr (EFGameLogicMgr +0x20c) survives EFEngineController::LoadLevel (only
    // KillAllGameBuffers when +0x2cc engine data is pending): carry every buffer (size = header +4)
    let carry: Vec<(u32, Vec<u8>)> = g.e.rt.game_buffers.iter().map(|(&id, &(p, _rc))| (id, (0..g.e.mem.r32(p - 0x10)).map(|i| g.e.mem.r8(p + i)).collect())).collect();
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
    g.hud_objs.clear();
    g.had_logic.clear();
    g.voices.clear();
    g.cam_standin = None;
    g.book = BOOK;
    g.give_book = false;
    g.beam_since = None;
    g.path_scr = 0;
    g.route = if g.route_src.as_deref() == Some("ep1") { parse_route(level_route(Some("ep1"), &dir)) } else if dir == g.route_level { parse_route(g.route_src.as_deref()) } else { Vec::new() };
    g.route_st = Default::default();
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
            eprintln!("game: level load failed: {m}");
            world.write_message(AppExit::error());
            world.remove_resource::<PendingLoad>();
            return;
        }
        Err(TryRecvError::Empty) => return,
        Err(TryRecvError::Disconnected) => {
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
        if let Some(s) = &l.render.stat {
            spawn_static_world(world, s, &rt.gfx.tex_dict);
            world.insert_resource(RwLights::from_static(&s.lights));
        }
        g.buf_carry.clear();
        g.had_logic = g.e.rt.objects.iter().flatten().map(|o| o.id).collect();
        g.render = Some(l.render);
        g.loaded = true;
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
fn read_pad(keys: Res<ButtonInput<KeyCode>>, mut host: game_input::HostPad, mut game: ResMut<Game>, mut exit: MessageWriter<AppExit>) {
    if game.autoplay {
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
    game.pad = host.read();
}

/// N2: scePadSetActDirect replacement. EFPlayerDevice_SP::DeviceSpecificUpdate writes the port-0
/// motor values (`motors`: small motor on/off, big motor 0..255) from the vibration envelopes;
/// the options vibration toggle (APIinput EnableVibration) already gates them in SetVibration, so
/// the host only mirrors the motors: big -> strong (low-frequency) motor, small -> weak motor.
/// Bevy rumble requests add up, so each change is Stop + Add (held until the next change).
fn pad_rumble(mut game: ResMut<Game>, pads: Query<Entity, With<Gamepad>>, mut rq: MessageWriter<bevy::input::gamepad::GamepadRumbleRequest>) {
    use bevy::input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest};
    let m = game.rt.particles.lock().unwrap().devices.devices.iter().find(|d| d.port == 0).map_or((false, 0), |d| d.motors);
    if m == game.motors {
        return;
    }
    game.motors = m;
    for gamepad in &pads {
        rq.write(GamepadRumbleRequest::Stop { gamepad });
        if m.0 || m.1 != 0 {
            let intensity = GamepadRumbleIntensity { strong_motor: m.1 as f32 / 255.0, weak_motor: if m.0 { 1.0 } else { 0.0 } };
            rq.write(GamepadRumbleRequest::Add { gamepad, intensity, duration: std::time::Duration::from_secs(3600) });
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
    k5_frame(g, f);
    if f == 10 && g.give_book {
        give_book(g);
    }
    // debug: GAME_WARP=x,y,z:frame puts the leader (APIgameobj::SetTranslation) at (x,y,z)
    if let Some(v) = std::env::var("GAME_WARP").ok().and_then(|v| {
        let (p, fr) = v.split_once(':')?;
        let v: Vec<f32> = p.split(',').filter_map(|s| s.parse().ok()).collect();
        (fr.parse::<u32>().ok()? == f && v.len() == 3).then_some(v)
    }) {
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
    // debug (K3): GAME_SENDMSG="uid:msg:param:frame,..." (hex uid/msg/param) sends a logic message,
    // e.g. water 0xd8 to a fire ghost or 0xc1 to a mummy, to reach a monster's catchable state
    if let Ok(v) = std::env::var("GAME_SENDMSG") {
        for it in v.split(',') {
            let p: Vec<&str> = it.split(':').collect();
            let h = |s: &str| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok();
            if let [u, m, a, fr] = p[..] {
                if fr.parse::<u32>().ok() == Some(f) {
                    if let (Some(u), Some(m), Some(a)) = (h(u), h(m), h(a)) {
                        let Game { e, rt, .. } = g;
                        rt.particles.lock().unwrap().logic = Some(&e.rt as *const _);
                        e.SendLogicMessageTo(u, m, a, 0, 0);
                        rt.particles.lock().unwrap().logic = None;
                        eprintln!("game: GAME_SENDMSG {u:#x} msg {m:#x} p {a:#x} at frame {f}");
                    }
                }
            }
        }
    }
    // P8: the K3 book test set-up (power 180, monster moved in front of the hero) is not part of a --route run
    let pad = if g.autoplay && g.route.is_empty() { autoplay_pad(g, f) } else { g.pad };
    let pad = padscript(f, pad);
    if f == 100 {
        if let Ok(v) = std::env::var("GAME_PROBE") {
            probe_heights(g, &v);
        }
        // P7: the wall map (GAME_MAP) for any level
        if let Ok(v) = std::env::var("GAME_MAP") {
            floor_map(g, &v);
        }
    }
    if f == 2 {
        list_code(g);
    }
    if std::env::var("GAME_DUMP_DOORS").ok().and_then(|v| v.parse::<u32>().ok()) == Some(f) {
        dump_doors(g);
    }
    let pad = if g.cam_standin.is_some() { standin_pre_frame(g, pad) } else if !g.route.is_empty() { route_any(g).map_or(pad, |mut p| { p.buttons |= pad.buttons; p }) } else { pad };
    g.rt.particles.lock().unwrap().devices.SetPadState(0, pad);
    let Game { e, rt, .. } = g;
    rt.Frame(e, g.raw_dt);
    if g.cam_standin.is_some() {
        standin_post_frame(g);
    }
    let Game { e, rt, .. } = g;
    cutscene_update(e, rt);
    let Game { rt, .. } = g;
    // STAND-IN: l01_camera Camera_BaseFUNC_SetClipPlane only sets near/far for a non-zero
    // camera zone (GB0+0x24, written by unported room/script data); EP1L01 keeps 0 here, so the
    // RwCameraCreate default far = 10 m would clip the level.
    for c in rt.gfx.cameras.cameras.iter_mut().flatten() {
        if c.rw.far <= 10.0 && std::env::var_os("GAME_NO_FAR").is_none() {
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
        *self.on.get_or_insert_with(|| std::env::var("GAME_PROF").is_ok())
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
        mv.play.Update(dt, pad.buttons);
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
        g.raw_dt = STEP;
        one_frame(g);
        g.fresh = true;
    } else {
        // F1: EFEngineController::MainLoop runs ONE input+logic+render pass per flip, and the flip
        // waits for vsync (59.94 Hz NTSC). A pass that misses vsyncs is measured by EFTimer::Update
        // as n field periods and goes through ComputeDeltaTime (clamp 0.05, 10-frame average): there
        // is no catch-up loop (the port used to run up to 4 fixed 1/60 steps per update). Frames are
        // due on the 59.94 Hz grid whatever the monitor refresh is.
        g.acc = (g.acc + time.delta_secs()).min(0.25);
        if g.acc + VSYNC_SLACK >= VSYNC {
            let n = ((g.acc + VSYNC_SLACK) / VSYNC).floor().max(1.0);
            g.acc = (g.acc - n * VSYNC).max(-VSYNC_SLACK);
            g.raw_dt = n * VSYNC;
            one_frame(g);
            g.fresh = true;
        }
    }
    // WP-S3: EFEngineController::Run flags set by APIgame (QuitGame/APIqa CloseGame: flags & 4 ends the
    // game; LoadLevel: flags & 8). Level switching (teardown + reload) is not implemented here: logged.
    {
        let mut p = g.rt.particles.lock().unwrap();
        // debug: GAME_LOADLEVEL=EP1L02:frame requests a switch like APIgame::LoadLevel
        let forced = std::env::var("GAME_LOADLEVEL").ok().and_then(|v| {
            let (l, f) = v.split_once(':')?;
            (f.parse::<u32>().ok()? == g.steps).then(|| scooby_recomp::engine::host::game::LevelRequest { level: l.to_string(), pack: format!("{}.P2R", l.replace('/', "")), video: None, external_files: Vec::new() })
        });
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

/// EF3dObj frame/pose update + EF3dObj_SP::Render of every visible game object.
fn draw_objects(g: &mut Game) {
    let Game { e, rt, render, objs, hud_objs: hobjs, gmgr, had_logic, steps, .. } = g;
    let hud_objs = hud_3d_objects(&rt.frame.hud);
    let Some(rd) = render.as_mut() else { return };
    gmgr.draws.clear();
    let mut p = rt.particles.lock().unwrap();
    let mut uids: Vec<u32> = p.sub.obj3d.keys().copied().collect();
    uids.sort_unstable();
    for uid in uids {
        // latent templates are not in the world; killed objects left the logic map
        if rd.latent.contains(&uid) {
            continue;
        }
        // J6: hidden objects (the book before GrabBook shows it) still update their hierarchy:
        // the logic reads their node frames (TomeDoomCB_PHYSIC hook joints) while hidden
        // K10: + EFWorld::ClipGameObjects clip mask bit 4 (e.g. captured monsters)
        let hidden = p.world.hidden.contains(&uid) || p.s1.clip_render_off(uid);
        let alive = e.rt.map.find(uid).is_some();
        if !alive && (had_logic.contains(&uid) || !rd.placed.contains(&uid)) {
            continue;
        }
        let Some(&(res_uid, _)) = p.sub.obj3d.get(&uid) else { continue };
        let Some(frame) = p.world.frames.get(&uid).copied() else { continue };
        let Some(r) = rd.res.iter_mut().find(|r| r.res.uid == res_uid) else { continue };
        let Some(c) = r.clump.as_mut() else { continue };
        let o = objs.entry(uid).or_insert_with(|| {
            let mut o = EF3dObj_SP::Initialize(res_uid, c, 0);
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
        if let Some(pose) = pose {
            apply_pose(o, c, &pose, ltm);
        }
        o.SyncFrames(ltm);
        // J6: publish the HAnim node frames (modelling matrix, LTM) for GetObjectFrame/GetBoneLMat
        if let Some(h) = &o.hier.hierarchy {
            let m = |a: &RwMat4| DTMatrix { right: [a[0], a[1], a[2]], up: [a[4], a[5], a[6]], at: [a[8], a[9], a[10]], pos: [a[12], a[13], a[14]] };
            let nodes = h.node_frames.iter().map(|f| f.map_or((m(&ltm), m(&ltm)), |f| (m(&o.frames.local[f]), m(&o.frames.GetLTM(f))))).collect();
            if posedump(*steps) {
                let n: &Vec<(DTMatrix, DTMatrix)> = &nodes;
                for (i, x) in n.iter().enumerate() {
                    eprintln!("NODE {uid:#x} {i} l {:?} w {:?} at {:?}", x.0.pos, x.1.pos, x.1.at);
                }
            }
            p.world.nodes.insert(uid, nodes);
            p.world.node_ids.insert(uid, h.node_ids.clone());
        }
        if hidden {
            continue;
        }
        // EFHud3DWindow objects are drawn by the HUD pass below, not in the world
        if hud_objs.iter().any(|h| h.0 == uid) {
            continue;
        }
        o.Render(0, c, &OwnerInfo { uid, ltm, ..Default::default() }, gmgr, &mut NoTexAnim);
    }
    // J7: EFHud3DWindow_SP::SpecificRender per visible 3D window: frame pos zeroed, translated
    // (POSTCONCAT) to the window centre in HUD camera space, then EF3dObj Render (vtable+0x70)
    // J8: the window object is GameObject+0x174[n] (its own EF3dObj: e.g. meter frame / fill / icon)
    for &(uid, res_uid, pos) in &hud_objs {
        let Some(frame) = p.world.frames.get(&uid).copied() else { continue };
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
        let ltm: RwMat4 = [
            frame.right[0], frame.right[1], frame.right[2], 0., frame.up[0], frame.up[1], frame.up[2], 0., frame.at[0], frame.at[1], frame.at[2], 0., pos[0], pos[1], pos[2], 1.,
        ];
        // N2: the window's EF3dObj is driven by the anims whose REFPUID is its descriptor
        // (EFWorld::Update -> UpdateHierarchy of every Get3dObject); unposed, the pause-menu
        // panel parts sat at their rest pose (stray glyph left of the items)
        if let Some(pose) = p.sub.PoseOf(uid, res_uid) {
            apply_pose(o, c, &pose, ltm);
        }
        o.SyncFrames(ltm);
        o.Render(0, c, &OwnerInfo { uid: HUD_OWNER + k as u32, ltm, ..Default::default() }, gmgr, &mut NoTexAnim);
    }
}

/// J8: draw-list owner ids of the HUD EF3dObjs (HUD_OWNER + index in `Game::hud_objs`)
const HUD_OWNER: u32 = 0xfff0_0000;

/// J7: (object uid, EF3dObj res uid, HUD-camera position) of every EFHud3DWindow drawn this frame (in order).
fn hud_3d_objects(cmds: &[scooby_recomp::engine::hud::HudDrawCmd]) -> Vec<(u32, u32, [f32; 3])> {
    let (w, h) = (SCREEN.0 as f32, SCREEN.1 as f32);
    cmds.iter()
        .filter_map(|c| match c.tex {
            scooby_recomp::engine::hud::HudTex::Object3D(o, res) => {
                let (cx, cy) = ((c.pos[0][0] + c.pos[3][0]) * 0.5, (c.pos[0][1] + c.pos[3][1]) * 0.5);
                Some((o, res, [-(cx / w - 0.5), -(cy / h - 0.5) * 0.875, c.priority as f32]))
            }
            _ => None,
        })
        .collect()
}

/// HAnim pose -> EF3dObj transfo pool 0 -> EF3dObj::UpdateHierarchy (as the viewer: pose
/// transfos carry their hierarchy node index, nodes the anim does not drive keep the clump rest).
fn apply_pose(o: &mut EF3dObj_SP, c: &scooby_recomp::engine::rw::clump::RpClump, pose: &[scooby_recomp::engine::anim::res::DTTransfo], ltm: RwMat4) {
    let n = o.hier.transfos.first().map_or(0, |t| t.len());
    if pose.is_empty() || !pose.iter().all(|t| (t.id as usize) < n && t.id >= 0) {
        return;
    }
    for t in pose {
        o.hier.transfos[0][t.id as usize] = t.clone();
    }
    o.base.flags |= 0x80;
    let flags = o.base.flags;
    let mut owner = ltm;
    o.hier.UpdateHierarchy(flags, &mut o.frames, &mut owner);
    if let Some(h) = &o.hier.hierarchy {
        for (i, f) in h.node_frames.iter().enumerate() {
            if let Some(f) = f {
                if !pose.iter().any(|t| t.id as usize == i) {
                    if let Some(r) = c.frames.get(*f) {
                        o.frames.local[*f] = r.local_matrix();
                    }
                }
            }
        }
    }
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

/// J7: EFHudMgr_SP::Update output -> overlay meshes on the HUD camera (layer 1, after the
/// sprites: EFGraphicsMgr::Render calls the HUD update after RenderCamera). RwIm2D replacement:
/// each HudDrawCmd is a pixel-space quad strip (v0 v1 v2 v3 -> 0,1,2 2,1,3), RGBA 0..255,
/// RwRenderState as set by EFHudMgr_SP::Update: z test/write off (draw order = list order),
/// SRCALPHA / INVSRCALPHA blend, texture clamp. Object3D (EFHud3DWindow) rects are not drawn
/// (the HUD 3D clump render pass is unported).
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
    for (n, c) in cmds.iter().enumerate() {
        let name = match &c.tex {
            HudTex::None => None,
            HudTex::Texture(t) => Some(t.clone()),
            HudTex::Font { font, tex } => fonts.get(*font as usize).and_then(|f| f.as_ref()).and_then(|f| f.get(*tex as usize)).cloned(),
            HudTex::Object3D(..) => continue,
        };
        // in front of the sprites (z <= 0..-40), later commands above earlier ones
        let z = 1.0 + n as f32 * 1e-3;
        let pos: Vec<[f32; 3]> = c.pos.iter().map(|p| [p[0] - w * 0.5, h * 0.5 - p[1], z]).collect();
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
        world.spawn((HudMesh, Mesh3d(mh), MeshMaterial3d(ma), Transform::IDENTITY, RenderLayers::layer(1)));
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
    for (_, _, draws) in sprites {
        for d in draws {
            // occluded sprites: rect drawn without the depth-sample test (approximation)
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
            let col: Vec<[f32; 4]> = verts.iter().map(|v| {
                let c = v.2;
                let mut g = gs_to_linear([c[0].clamp(0., 255.) as u8, c[1].clamp(0., 255.) as u8, c[2].clamp(0., 255.) as u8, c[3].clamp(0., 255.) as u8]);
                match blend {
                    Some(0xa1) => g = [1.0 - g[0], 1.0 - g[1], 1.0 - g[2], 1.0],
                    Some(0x44) | None => {}
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
            let mat = StandardMaterial { base_color_texture: img.map(|i| i.0), unlit: true, alpha_mode: alpha_for_gs(blend), cull_mode: None, double_sided: true, ..default() };
            let mh = world.resource_mut::<Assets<Mesh>>().add(m);
            let ma = world.resource_mut::<Assets<StandardMaterial>>().add(mat);
            world.spawn((OverlayMesh, Mesh3d(mh), MeshMaterial3d(ma), Transform::IDENTITY, RenderLayers::layer(layer)));
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
        base_color: Color::srgba(col[0], col[1], col[2], col[3]),
        base_color_texture: img.map(|i| i.0),
        unlit: true,
        alpha_mode: alpha,
        cull_mode: None,
        double_sided: true,
        depth_bias: 50.0,
        ..default()
    }
}

/// WP-S4: DFF mapper pass on the projector receivers (logic flag 0x200, goProjectorMgr receiver
/// list): their drawn meshes (DrawListEntity of the owner, world transform) get the projected
/// texture too; rebuilt every frame (receivers move).
#[derive(Component)]
struct ProjectorRecvMesh;
fn draw_projector_receivers(world: &mut World, occ: &[scooby_recomp::engine::gfx::projector::ProjectorOccurence], recv: &[u32], dict: &scooby_recomp::engine::rw::image::RwTexDictionary) {
    let old: Vec<Entity> = world.query_filtered::<Entity, With<ProjectorRecvMesh>>().iter(world).collect();
    for e in old {
        world.despawn(e);
    }
    if occ.is_empty() || recv.is_empty() {
        return;
    }
    let mut tris: Vec<[[f32; 3]; 3]> = Vec::new();
    {
        let mut q = world.query::<(&DrawListEntity, &Mesh3d, &GlobalTransform, &Visibility)>();
        let items: Vec<(u32, Handle<Mesh>, GlobalTransform, Visibility)> = q.iter(world).map(|(d, m, g, v)| (d.owner, m.0.clone(), *g, *v)).collect();
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
        if pos.is_empty() {
            continue;
        }
        let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
        let mat = projector_material(world, o, dict);
        let mh = world.resource_mut::<Assets<Mesh>>().add(m);
        let ma = world.resource_mut::<Assets<StandardMaterial>>().add(mat);
        world.spawn((ProjectorRecvMesh, Mesh3d(mh), MeshMaterial3d(ma), Transform::IDENTITY, RenderLayers::layer(0)));
    }
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
    let mut q = world.query::<(&DrawListEntity, &Mesh3d, &GlobalTransform, &Visibility)>();
    let items: Vec<(u32, Handle<Mesh>, GlobalTransform)> =
        q.iter(world).filter(|(_, _, _, v)| **v != Visibility::Hidden).map(|(d, m, g, _)| (d.owner, m.0.clone(), *g)).collect();
    for (obj, _lod, m) in fx {
        let f = |i: usize| f32::from_bits(m[i]);
        let uv_matrix = Mat4::from_cols_array(&std::array::from_fn(|i| f(i)));
        // offset scale: one view-normal unit = 4% of the screen (approximation)
        let ma = world.resource_mut::<Assets<game_fx::DistortMaterial>>().add(game_fx::DistortMaterial { uv_matrix, strength: Vec4::new(0.04, 0.0, 0.0, 0.0) });
        for (_, mh, gt) in items.iter().filter(|i| i.0 == *obj) {
            world.spawn((DistortionMesh, Mesh3d(mh.clone()), MeshMaterial3d(ma.clone()), gt.compute_transform(), RenderLayers::layer(0)));
        }
    }
}

/// WP-S5: EFShadowMgr caster pass (gfx::shadow). Caster bounding spheres come from the drawn
/// meshes of the owner (one sphere per mesh AABB, standing in for the atomic bspheres); the
/// shadow raster is the CPU silhouette (APPROXIMATION of the VU1 cast) and is drawn with the
/// projector pass on the world sectors (blend 1). GAME_SHADOW_OFF disables.
#[derive(Component)]
struct ShadowMesh;
fn draw_shadows(world: &mut World, cast: &[(u32, u32, [u32; 11], scooby_recomp::engine::camera::DTMatrix)], lights: &[scooby_recomp::engine::gfx::shadow::ShadowLight], stat: Option<&scooby_recomp::engine::rw::world::RpWorld>) {
    use scooby_recomp::engine::gfx::shadow::*;
    let old: Vec<Entity> = world.query_filtered::<Entity, With<ShadowMesh>>().iter(world).collect();
    for e in old {
        world.despawn(e);
    }
    let Some(w) = stat else { return };
    if cast.is_empty() || std::env::var_os("GAME_SHADOW_OFF").is_some() {
        return;
    }
    let mut q = world.query::<(&DrawListEntity, &Mesh3d, &GlobalTransform, &Visibility)>();
    let items: Vec<(u32, Handle<Mesh>, GlobalTransform)> =
        q.iter(world).filter(|(_, _, _, v)| **v != Visibility::Hidden).map(|(d, m, g, _)| (d.owner, m.0.clone(), *g)).collect();
    let mut input = Vec::new();
    let mut tris_of = std::collections::HashMap::new();
    {
        let meshes = world.resource::<Assets<Mesh>>();
        for (uid, fl, prm, frame) in cast {
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
            tris_of.insert(*uid, tris);
            input.push((*uid, *fl, *prm, *frame, InitCaster(&spheres)));
        }
    }
    let dbg = std::env::var_os("GAME_DEBUG").is_some();
    if dbg {
        eprintln!("shadow casters {} with meshes {} lights {:x?}", cast.len(), input.len(), lights.iter().map(|l| (l.0, l.1, l.3, l.5)).collect::<Vec<_>>());
    }
    for s in ShadowBeginLights(&input, lights) {
        let mask = CastSilhouette(&s.occ, s.size, &tris_of[&s.caster]);
        let (pos, uv) = s.occ.mesh(w);
        if dbg {
            eprintln!("shadow {:x} par {} pos {:?} vw {} col {} cover {} patch {}", s.caster, s.occ.parallel, s.occ.pos, s.occ.view_window[0], s.occ.color[0], mask.iter().filter(|&&a| a > 0).count(), pos.len() / 3);
            let (lo, hi) = pos.iter().fold(([f32::MAX; 3], [f32::MIN; 3]), |(mut l, mut h), p| { for i in 0..3 { l[i] = l[i].min(p[i]); h[i] = h[i].max(p[i]); } (l, h) });
            let (ul, uh) = uv.iter().fold(([f32::MAX; 2], [f32::MIN; 2]), |(mut l, mut h), p| { for i in 0..2 { l[i] = l[i].min(p[i]); h[i] = h[i].max(p[i]); } (l, h) });
            let n = s.size as usize;
            let (mut ml, mut mh) = ([n; 2], [0usize; 2]);
            for (i, &a) in mask.iter().enumerate() { if a > 0 { let (x, y) = (i % n, i / n); ml = [ml[0].min(x), ml[1].min(y)]; mh = [mh[0].max(x), mh[1].max(y)]; } }
            eprintln!("shadow uv {ul:?}..{uh:?} mask px {ml:?}..{mh:?} of {n}");
            eprintln!("shadow patch {lo:?}..{hi:?} light {:?} at {:?} near {:?}", s.occ.ltm.pos, s.occ.ltm.at, s.occ.near);
        }
        if pos.is_empty() {
            continue;
        }
        // Cd - Cs (Cs = mask * occurrence colour) approximated as Cd * (1 - Cs) (Bevy Multiply)
        let mask: Vec<u8> = if std::env::var_os("GAME_SHADOW_SOLID").is_some() { vec![255; mask.len()] } else { mask };
        // shadow raster (cleared black, caster drawn 0x80ffffff = white) -> projected with
        // RGBA = colour*128 and ALPHA_1 0xa1: Cd - tex*colour (game_fx::ShadowSubMaterial)
        let data: Vec<u8> = mask.iter().flat_map(|&a| [a, a, a, 255]).collect();
        let img = Image::new(
            bevy::render::render_resource::Extent3d { width: s.size, height: s.size, depth_or_array_layers: 1 },
            bevy::render::render_resource::TextureDimension::D2,
            data,
            bevy::render::render_resource::TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        );
        let ih = world.resource_mut::<Assets<Image>>().add(img);
        let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
        // debug: GAME_SHADOW_GAIN=x scales the shadow intensity (visual check of the pass)
        let gain: f32 = std::env::var("GAME_SHADOW_GAIN").ok().and_then(|v| v.parse().ok()).unwrap_or(1.0);
        let c = s.occ.color;
        let mat = game_fx::ShadowSubMaterial { color: LinearRgba::new(c[0] * gain, c[1] * gain, c[2] * gain, 1.0), mask: ih };
        let mh = world.resource_mut::<Assets<Mesh>>().add(m);
        let ma = world.resource_mut::<Assets<game_fx::ShadowSubMaterial>>().add(mat);
        world.spawn((ShadowMesh, Mesh3d(mh), MeshMaterial3d(ma), Transform::IDENTITY, RenderLayers::layer(0)));
    }
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
        draw_objects(g);
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
            let lit = g.rt.particles.lock().unwrap().s1.lit.clone();
            for d in g.gmgr.draws.iter_mut() {
                if let Some(&on) = lit.get(&d.owner) {
                    d.geometry_flags = if on { d.geometry_flags | 0x20 } else { d.geometry_flags & !0x20 };
                }
            }
        }
        ApplyDrawList(world, &g.gmgr.draws, &look, &g.rt.gfx.tex_dict);
        // J7: the EFHud3DWindow clumps go to the HUD camera layer
        let mut q = world.query::<(&DrawListEntity, &mut RenderLayers)>();
        for (d, mut l) in q.iter_mut(world) {
            *l = RenderLayers::layer(if d.owner >= HUD_OWNER { 2 } else { 0 });
        }
        let missing: Vec<(Entity, bool)> = world.query_filtered::<(Entity, &DrawListEntity), Without<RenderLayers>>().iter(world).map(|(e, d)| (e, d.owner >= HUD_OWNER)).collect();
        for (e, hud) in missing {
            world.entity_mut(e).insert(RenderLayers::layer(if hud { 2 } else { 0 }));
        }
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
            let ps: Vec<_> = g.rt.frame.particles.iter().map(|b| b.1.clone()).collect();
            let bs: Vec<_> = g.rt.frame.billboards.iter().map(|b| b.1.clone()).collect();
            ApplyParticles(world, &ps, &bs, &c.ltm, &g.rt.gfx.tex_dict);
        }
        let rwcam = g.rt.gfx.cameras.cameras.iter().flatten().next().map(|c| (c.ltm.clone(), c.rw.view_window));
        draw_sprites(world, &g.rt.frame.sprites, &g.rt.gfx.tex_dict, rwcam.as_ref().map(|(l, v)| (l, *v)));
        draw_trails(world, &g.rt.frame.trails);
        let occ = {
            let mut p = g.rt.particles.lock().unwrap();
            // debug: GAME_PROJ_FORCE forces every projector visible (visual check of the pass)
            if std::env::var_os("GAME_PROJ_FORCE").is_some() {
                p.s1.projectors.projectors.values_mut().for_each(|pr| pr.flags |= 1);
            }
            (scooby_recomp::engine::gfx::projector::ProjectorMgrBegin(&p.s1.projectors, |h| p.world.frames.get(&h).copied()), p.s1.proj_receivers.clone())
        };
        let (occ, recv) = occ;
        draw_projectors(world, &occ, rd.stat.as_ref().and_then(|s| s.world.as_ref()), &g.rt.gfx.tex_dict);
        draw_projector_receivers(world, &occ, &recv, &g.rt.gfx.tex_dict);
        let cast = {
            let p = g.rt.particles.lock().unwrap();
            p.s1.shadow_casters.iter().filter_map(|&u| {
                Some((u, *p.s1.caster_flags.get(&u).unwrap_or(&0x400), *p.s1.shadow.get(&u).unwrap_or(&scooby_recomp::engine::host::gameobj::SHADOW_DEFAULT), *p.world.frames.get(&u)?))
            }).collect::<Vec<_>>()
        };
        // EFLightMgr light list: the level's static lights (EFStatic3dObj::ReadLights: frame =
        // the stored matrix, in the world) + the dynamic ones (APIlight)
        let mut lights: Vec<scooby_recomp::engine::gfx::shadow::ShadowLight> = scooby_recomp::engine::runtime::J2(&g.rt.particles).lights(|l| {
            l.lights.iter().filter_map(|x| { let r = x.rp.as_ref()?; Some((r.ty, r.flags, x.frame?, r.radius, x.kind, x.IsVisible())) }).collect()
        });
        if let Some(st) = rd.stat.as_ref() {
            for l in &st.lights {
                let m = l.matrix;
                let f = scooby_recomp::engine::camera::DTMatrix { right: [m[0], m[1], m[2]], up: [m[3], m[4], m[5]], at: [m[6], m[7], m[8]], pos: [m[9], m[10], m[11]] };
                lights.push((l.light.light_type, l.light.flags, f, l.light.radius, 0x20000, true));
            }
        }
        draw_shadows(world, &cast, &lights, rd.stat.as_ref().and_then(|s| s.world.as_ref()));
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
        let fonts: Vec<Option<Vec<String>>> = g.rt.particles.lock().unwrap().hud.fonts.iter().map(|f| f.as_ref().map(|f| f.textures.clone())).collect();
        draw_hud(world, &g.rt.frame.hud, &fonts, &g.rt.gfx.tex_dict);
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

/// J7/A3: a voice as two entities (left-only / right-only) at unit gain; play_audio sets each
/// sink's volume to Voice::left/right every frame (SPU2 VOLL/VOLR).
fn spawn_spu_voice(cmd: &mut Commands, src: &mut Assets<SpuClip>, v: &scooby_recomp::engine::audio::mgr::Voice, data: SpuData) -> [Entity; 2] {
    let voice = SpuVoice::new(data.clone(), v.sample_rate, v.pitch, v.adsr, v.looped, v.delay);
    [(0u8, v.left), (1, v.right)].map(|(side, g)| {
        // A4: interleaved .INT music: each side plays its own channel (L = 0, R = 1)
        let voice = match &data {
            SpuData::Adpcm(d) if scooby_recomp::engine::audio::spu::is_interleaved(d) => SpuVoice::new(SpuData::Interleaved(d.clone(), side), v.sample_rate, v.pitch, v.adsr, v.looped, v.delay),
            _ => voice.clone(),
        };
        let h = src.add(SpuClip { voice, rate: v.sample_rate, side });
        cmd.spawn((bevy_audio::AudioPlayer(h), bevy_audio::PlaybackSettings::DESPAWN.with_speed(v.pitch).with_volume(bevy_audio::Volume::Linear(g.clamp(0.0, 1.0))), VoiceHandle(v.handle), VoiceSide(side))).id()
    })
}

fn play_audio(world: &mut World) {
    if !world.resource::<Game>().audio || !world.resource::<Game>().loaded {
        return;
    }
    world.resource_scope(|world, mut game: Mut<Game>| {
        let g = &mut *game;
        let sl = &mut g.streams;
        while let Ok((p, r)) = sl.done.lock().unwrap().try_recv() {
            sl.cache.insert(p, r);
        }
        let p = g.rt.particles.lock().unwrap();
        let a = &p.sub.audio;
        let mut live = HashSet::new();
        let mut gains = HashMap::new();
        let mut new = Vec::new();
        for v in &a.voices {
            if v.virt || v.paused {
                continue;
            }
            let key = (v.handle, v.bank, v.sample);
            live.insert(key);
            gains.insert(v.handle, (v.left, v.right));
            if g.voices.contains_key(&key) {
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
                new.push((key, v.clone(), SpuData::Pcm(pcm.samples.clone().into(), pcm.loop_start)));
            }
        }
        drop(p);
        // stream PCM is kept only while a voice uses it (failed loads stay None: not retried)
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
        update_voice_gains(&mut sinks, &|h| gains.get(&h).copied());
        for (key, v, data) in new {
            let es = spawn_spu_voice(&mut cmd, &mut src, &v, data);
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
    let a = parse_args();
    let mut app = App::new();
    if a.rt && cfg!(not(feature = "rt")) {
        eprintln!("game: --rt needs the `rt` cargo feature; ignored");
    }
    if a.headless {
        app.add_plugins(DefaultPlugins.set(WindowPlugin { primary_window: None, exit_condition: ExitCondition::DontExit, ..default() }).disable::<WinitPlugin>())
            .add_plugins(ScheduleRunnerPlugin::run_loop(std::time::Duration::from_millis(1)));
    } else {
        app.add_plugins(DefaultPlugins.set(WindowPlugin { primary_window: Some(Window { title: format!("Scooby-Doo! Mystery Mayhem - {}", a.level), resolution: a.res.map(|(w, h)| bevy::window::WindowResolution::new(w, h)).unwrap_or_default(), ..default() }), ..default() }))
            .add_plugins(bevy_audio::AudioPlugin::default());
        bevy_audio::AddAudioSource::add_audio_source::<SpuClip>(&mut app);
    }
    if let Some(d) = &a.shots {
        let _ = std::fs::create_dir_all(d);
    }
    app.add_plugins(game_fx::GameFxPlugin);
    app.add_plugins(GfxAdapterPlugin)
        .insert_resource(GlobalAmbientLight { color: Color::WHITE, brightness: 0.0, ..default() })
        .insert_resource(ClearColor(Color::srgb(0.0, 0.0, 0.0)))
        .add_plugins(GamePlugin { args: a });
    app.run();
}

/// EFEngineController main loop: EFCutScenePlayer::Update after the frame. The cut-scene calls the
/// logic made during the frame (APIgame_*CutScene, queued in `cutscene_req`) are applied first.
fn cutscene_update(e: &mut scooby_recomp::engine::Engine, rt: &mut EngineRuntime) {
    use scooby_recomp::engine::runtime::CutSceneReq;
    use scooby_recomp::engine::script::RuntimeCutSceneHost;
    let (cs, reqs) = {
        let mut p = rt.particles.lock().unwrap();
        (p.cutscene.take(), std::mem::take(&mut p.cutscene_req))
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
        let w: Vec<String> = (0..9).map(|k| if l == 0 { "-".into() } else { format!("{:#x}", g.e.mem.r32(l + 4 * k)) }).collect();
        eprintln!("game: code {n} obj {id:#x} at {:?} local {}", pos_of(g, id), w.join(" "));
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
        v.push((id, d[i + 8..i + 8 + n].to_vec()));
        i += 8 + n;
    }
    v
}

fn give_book(g: &mut Game) {
    let book = g.book;
    let Game { e, rt, .. } = g;
    rt.particles.lock().unwrap().logic = Some(&e.rt as *const _);
    e.SendLogicMessageTo(book, 0xe5, 0, 0, 0);
    rt.particles.lock().unwrap().logic = None;
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
    rt.particles.lock().unwrap().logic = Some(&e.rt as *const _);
    e.SendLogicMessageTo(to, msg, sub, 0, 0);
    rt.particles.lock().unwrap().logic = None;
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
    if std::env::var_os("GAME_GBLOG").is_some() && f % 10 == 0 {
        let b = gb(g, 2);
        let w: Vec<u32> = (0..8).map(|i| g.e.mem.r32(b + 4 * i)).chain([g.e.mem.r32(gb(g, 1) + 4)]).collect();
        if w != g.gblog {
            eprintln!("game: GB2 f{f} {:x?} power {}", &w[..8], f32::from_bits(w[8]));
            g.gblog = w;
        }
    }
    // WP-P14: GAME_LOCWATCH=uid prints the instance's local words 0..0x90 whenever they change (every 10 frames)
    if let (Some(u), 0) = (std::env::var("GAME_LOCWATCH").ok().and_then(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16).ok()), f % 10) {
        let l = g.e.get_game_object(u, 0).map_or(0, |h| g.e.rt.obj(h).inst.local);
        if l != 0 {
            let w: Vec<u32> = (0..0x25).map(|i| g.e.mem.r32(l + 4 * i)).collect();
            if w != g.locwatch {
                eprintln!("game: LOCWATCH f{f} {u:#x} {w:x?}");
                g.locwatch = w;
            }
        }
    }
    // GAME_POSLOG="uid,uid,...": positions every 30 frames (e.g. a fire ghost 0x147 vs its inner ghost 0x14d)
    if let (Ok(v), 0) = (std::env::var("GAME_POSLOG"), f % 30) {
        for u in v.split(',').filter_map(|s| u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()) {
            eprintln!("game: K5 pos f{f} {u:#x} {:?} state {:#x}", pos_of(g, u), GetCurrentStateID(&mut g.e, u));
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
    if g.steps == 0 && gb0 != 0 && g.cam_standin.as_ref().unwrap().cam_unported {
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
    if gb0 == 0 || g.steps < 30 {
        return p;
    }
    let lead = g.e.mem.r32(gb0 + 8);
    let (f8, fc) = (g.e.mem.r32(gb2 + 8), g.e.mem.r32(gb2 + 0xc));
    let playing = g.rt.particles.lock().unwrap().cutscene.as_ref().is_some_and(|c| c.flags & 1 != 0);
    let steps = g.steps;
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
fn route_any(g: &mut Game) -> Option<PadState> {
    let gb0 = gb(g, 0);
    if gb0 == 0 || g.steps < 30 {
        return None;
    }
    if g.rt.particles.lock().unwrap().cutscene.as_ref().is_some_and(|c| c.flags & 1 != 0) {
        return Some(idle_pad());
    }
    // GB0+0x14 bit 0x1000000: a tutorial text holds the team until X (like a player reading it)
    if g.e.mem.r32(gb0 + 0x14) & 0x100_0000 != 0 {
        return Some(PadState { buttons: if g.steps % 40 < 2 { pad::CROSS } else { 0 }, ..idle_pad() });
    }
    let lead = g.e.mem.r32(gb0 + 8);
    let cam = g.rt.gfx.cameras.cameras.iter().flatten().next().map(|c| c.ltm);
    route_pad(g, lead, cam)
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
        _ => r,
    }
}

fn parse_route(r: Option<&str>) -> Vec<(f32, f32, char)> {
    let r = match r {
        None => return Vec::new(),
        Some("l03a") => routes_ep1b::L03A_ROUTE,
        Some("l03b") => routes_ep1b::L03B_ROUTE,
        Some("l04") => routes_ep1b::L04_ROUTE,
        Some("l01") => L01_ROUTE,
        Some("l01cart") => L01CART_ROUTE,
        Some("l02") => L02_ROUTE,
        Some(r) => r,
    };
    r.split(';')
        .filter_map(|w| {
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
const L02_ROUTE: &str = "33.69,-6.84;37.31,-8.19;38,-9;38,-12,J;38,-14;37,-17.5;28,-17.8;23,-17.5;13,-17.5;8,-15;7.5,-8;6,-7.8,X;112,-5;108,-4.2;108,-3.2,X;207,-12;207,18.26;209.3,18.26,X;290,12;330,11;336.5,10.9,X;336,11.3;290,11;282,10.6;281.1,10.6,X;207,10;207,7.26;209,7.26,X;420,-7.75;439,-3;439,-0.6,X";

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
        g.e.rt.objects.iter().flatten().filter(|o| o.inst.code.is_some_and(|c| { let n = g.e.rt.codes[c].def.name; n == "ghost" || n == "shadows" || n.ends_with("ghost") && !n.starts_with("scooby") && !n.starts_with("shaggy") })).map(|o| o.id).collect()
    };
    ids.into_iter().filter(|&u| GetCurrentStateID(&mut g.e, u) != 0xb).collect()
}

/// --route bot: steer to the current waypoint (camera-relative stick); None when the route is done.
fn route_pad(g: &mut Game, lead: u32, cam: Option<DTMatrix>) -> Option<PadState> {
    let tome = tome_id(g);
    let mut p = idle_pad();
    let steps = g.steps;
    let a = pos_of(g, lead)?;
    let c = cam?;
    let playing = g.rt.particles.lock().unwrap().cutscene.as_ref().is_some_and(|c| c.flags & 1 != 0);
    if std::env::var_os("GAME_DEBUG").is_some() && steps % 30 == 0 {
        { let b2 = gb(g, 2); let b0 = gb(g, 0); let b14 = if b0 == 0 { 0 } else { g.e.mem.r32(b0 + 0x14) }; let b34 = if b0 == 0 { 0 } else { g.e.mem.r32(b0 + 0x34) }; eprintln!("game: route f{steps} gb0.14 {b14:#x} .34 {b34:#x} lead {lead:#x} {a:?} other {:?} gb2 {:#x} {:#x}", pos_of(g, lead ^ 0x03), g.e.mem.r32(b2 + 8), g.e.mem.r32(b2 + 0xc)); }
        // WP-P9: GAME_WATCH="hex,.." prints state + position of the listed instances
        if let Ok(v) = std::env::var("GAME_WATCH") {
            for u in v.split(',').filter_map(|s| u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()) {
                eprintln!("game: watch f{steps} {u:#x} state {:#x} {:?}", GetCurrentStateID(&mut g.e, u), pos_of(g, u));
            }
        }
        if steps == std::env::var("GAME_PHYSDUMP").ok().and_then(|v| v.parse().ok()).unwrap_or(1200) {
            let s = scooby_recomp::engine::runtime::J2(&g.rt.particles).physic(|m, _| {
                m.by_owner.iter().filter_map(|(&u, &id)| m.Physic(id).map(|p| (u, p))).filter(|(_, p)| (p.model.bsphere_world[0] - a[0]).powi(2) + (p.model.bsphere_world[2] - a[2]).powi(2) < 100.0).map(|(u, p)| format!("{u:#x}: flags {:#x} ctx {:#x} state {:#x} prims {} bs {:?} r {} {:?}", p.flags, p.context, p.state, p.model.prims.len(), p.model.bsphere_world, p.model.bsphere_radius, if u == lead || u == 0x2a00_0052 { format!("{:?}", p.model.prims) } else { String::new() })).collect::<Vec<_>>()
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
        eprintln!("game: route FAKE warp leader {lead:#x} to ({x},{z}) at frame {steps}");
    }
    // WP-P14 'K' (FAKE progression bits): "mask,0,K" ORs the decimal mask into GB2+0xc (door lock word)
    if let Some(&(m, _, 'K')) = g.route.get(g.route_st.0) {
        let b = gb(g, 2) + 0xc;
        let v = g.e.mem.r32(b) | m as u32;
        g.e.mem.w32(b, v);
        eprintln!("game: route FAKE GB2+0xc |= {:#x} -> {v:#x} at frame {steps}", m as u32);
    }
    // route state: (index, last pos, best dist, frame of progress, cut-scene seen)
    let (i, last, best, at, seen) = &mut g.route_st;
    if *at == 0 {
        (*last, *best, *at) = (a, f32::MAX, steps);
    }
    let jumped = (a[0] - last[0]).powi(2) + (a[2] - last[2]).powi(2) > 100.0;
    *last = a;
    let &(x, z, act) = g.route.get(*i)?;
    let act = if act == 'G' { 'B' } else { act };
    let (x, z) = if act == 'B' { ghost.unwrap_or((x, z)) } else { (x, z) };
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
        // P7 'Y': X at a hot-spot that moves the team a short way (crawl): done once the leader,
        // after having been within 2.5, is 2.8 away again
        'Y' => *best < 2.5 && d > 2.8,
        'W' => *seen && !playing,
        // 'P': press X once at a hot-spot (a button toggles on every press)
        'P' => d < 1.0 && steps % 20 == 2,
        // P7 'O': press circle once wherever the leader is (switch leader, heroCB_INPUT_Switch_Leader)
        'O' => steps % 20 == 2,
        'B' => captured || GetCurrentStateID(&mut g.e, tome) == 4,
        'Z' | 'K' => true,
        _ => d < 0.6,
    };
    if d < *best - 0.2 || playing {
        // (a cut-scene freezes the team: not stuck)
        (*best, *at) = (d.min(*best), steps);
    }
    let stuck = steps > *at + match act { 'B' => 1800, 'P' => 240, _ => 600 };
    if done || stuck {
        eprintln!("game: route wp {i} ({x},{z},{act}) {} at frame {steps}, leader {a:?}", if done { "reached" } else { "STUCK, skipped" });
        *i += 1;
        (*best, *at, *seen) = (f32::MAX, steps, false);
        return Some(p);
    }
    // plain waypoints: steer along the collision-map BFS path (walls between waypoints)
    let t = if (act == ' ' || act == 'B') && d > 1.0 { path_step(g, a, [x, a[1], z], steps).unwrap_or([x, 0.0, z]) } else { [x, 0.0, z] };
    let dir = norm([t[0] - a[0], 0.0, t[2] - a[2]]);
    let fwd = norm([c.at[0], 0.0, c.at[2]]);
    let right = norm(cross([0.0, 1.0, 0.0], fwd));
    if d > 0.3 {
        p.lx = stick(-(dir[0] * right[0] + dir[2] * right[2]));
        p.ly = stick(-(dir[0] * fwd[0] + dir[2] * fwd[2]));
    }
    if ((act == 'X' || act == 'Y') && d < 2.5 || act == 'P' && d < 1.0 || act == 'J') && steps % 20 < 2 {
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
            if near < 5.0 || (l != 0 && g.e.mem.r32(l + 0x58) == 0x53) {
                p.buttons = pad::SQUARE;
            }
            let _ = 0;
            let l = g.e.get_game_object(tome, 0).map_or(0, |h| g.e.rt.obj(h).inst.local);
            if std::env::var_os("GAME_DEBUG").is_some() && steps % 30 == 0 && l != 0 {
                eprintln!("game: tome near {near} f{steps} state {} sub {:#x} btn {:#x} life {} power {} target {:#x} ghosts {:?}", GetCurrentStateID(&mut g.e, tome), g.e.mem.r32(l + 0x58), g.e.mem.r32(l + 0x1e8), f32::from_bits(g.e.mem.r32(l + 0x70)), f32::from_bits(g.e.mem.r32(gb(g, 1) + 4)), g.e.mem.r32(l + 0x60), [0x2a00_0029u32, 0x2a00_002c].map(|u| { let gl = g.e.get_game_object(u, 0).map_or(0, |h| g.e.rt.obj(h).inst.local); (GetCurrentStateID(&mut g.e, u), pos_of(g, u), if gl == 0 { [0.0; 6] } else { [0x17c, 0x180, 0x184].map(|o| f32::from_bits(g.e.mem.r32(gl + o))).into_iter().chain([0x178, 0x174, 0x188].map(|o| g.e.mem.r32(gl + o) as f32)).collect::<Vec<_>>().try_into().unwrap() }) }));
            }
            if l != 0 && g.e.mem.r32(l + 0x58) == 0x53 {
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
fn path_step(g: &mut Game, a: [f32; 3], goal: [f32; 3], steps: u32) -> Option<[f32; 3]> {
    static PATH: std::sync::Mutex<(u32, [f32; 3], Vec<[f32; 3]>)> = std::sync::Mutex::new((0, [0.0; 3], Vec::new()));
    let d2 = |p: [f32; 3], q: [f32; 3]| (p[0] - q[0]).powi(2) + (p[2] - q[2]).powi(2);
    let mut pg = PATH.lock().unwrap();
    if steps >= pg.0 + 120 || d2(pg.1, goal) > 1.0 {
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
            eprintln!("game: movie end at frame {} (MpegPlay -> {}, {} pictures)", g.steps, mv.play.MpgSkip as u32, mv.play.frame_count);
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
            let mesh = world.resource_mut::<Assets<Mesh>>().add(Rectangle::new(SCREEN.0 as f32, SCREEN.1 as f32));
            let target = world.get_resource::<Target>().map(|t| t.0.clone());
            let mut c = world.spawn((
                Camera3d::default(),
                Camera { order: 10, clear_color: ClearColorConfig::Custom(Color::BLACK), ..default() },
                Tonemapping::None,
                HudCam,
                RenderLayers::layer(7),
                Projection::Orthographic(OrthographicProjection { scaling_mode: ScalingMode::Fixed { width: SCREEN.0 as f32, height: SCREEN.1 as f32 }, near: 0.0, far: 100.0, ..OrthographicProjection::default_3d() }),
                Transform::from_xyz(0.0, 0.0, 50.0),
            ));
            if let Some(t) = target {
                c.insert(RenderTarget::Image(t.into()));
            }
            let cam = c.id();
            let quad = world.spawn((Mesh3d(mesh), MeshMaterial3d(mat), Transform::IDENTITY, RenderLayers::layer(7))).id();
            mv.ents = vec![cam, quad];
            mv.image = Some(h);
        }
        if let Some(f) = mv.play.take_new_frame() {
            let data = f.rgba.clone();
            if let Some(mut img) = world.resource_mut::<Assets<Image>>().get_mut(mv.image.as_ref().unwrap()) {
                img.data = Some(data);
            }
        }
        if g.audio && !mv.audio_spawned && mv.play.started {
            mv.audio_spawned = true;
            if let Some((rate, pcm)) = mv.play.audio.take() {
                let vol = mv.play.volume;
                for (side, l, r) in [(0usize, 1.0f32, 0.0f32), (1, 0.0, 1.0)] {
                    let mono: Vec<i16> = pcm.iter().skip(side).step_by(2).copied().collect();
                    let bytes = scooby_recomp::engine::audio::wav::pcm_to_wav(&mono, rate, l, r);
                    let h = world.resource_mut::<Assets<bevy_audio::AudioSource>>().add(bevy_audio::AudioSource { bytes: bytes.into() });
                    let e = world.spawn((bevy_audio::AudioPlayer(h), bevy_audio::PlaybackSettings::ONCE.with_volume(bevy_audio::Volume::Linear(vol)))).id();
                    mv.ents.push(e);
                }
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
        let lvl = if name.starts_with("hs_door") && l != 0 && m.r32(l + 0x18) != 0 { m.cstr_at(m.r32(l + 0x18)) } else { String::new() };
        let w = |k: u32| if l != 0 { m.r32(l + k) } else { 0 };
        eprintln!("door {name} {:#x} pos {:?} tgt {:?} +0 {:#x} +4 {:#x} +8 {:#x} +c {:#x} +10 {:#x} lvl {lvl:?}", o.id, pos_of(g, o.id), pos_of(g, w(0)), w(0), w(4), w(8), w(0xc), w(0x10));
    }
    eprintln!("leaders {:?} {:?}", pos_of(g, 0x2a000092), pos_of(g, 0x2a000091));
    let b2 = gb(g, 2);
    if b2 != 0 {
        eprintln!("GB2 +8 {:#x} +c {:#x}", m.r32(b2 + 8), m.r32(b2 + 0xc));
    }
}

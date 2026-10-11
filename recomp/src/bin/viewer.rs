//! WP-C3/C4 level viewer (feature `viewer`): loads a level pack from the ISO, places every level
//! object that has an EF3dObj descriptor at its GLO frame, runs EF3dObj_SP::Render (LOD 0) into
//! the EFGraphicsMgr draw list every frame and mirrors it with `bevy_adapter::ApplyDrawList`; the
//! static world (EFStatic3dObj) is spawned with `spawn_static_world`. Atomics are lit on the CPU
//! with the level's static RpLights (`RwLights`, PS2 RW lighting model) — no Bevy lights.
//!
//! Usage: viewer [LEVEL] [--model RESUID] [--anim HANIMUID] [--time SECONDS] [--particles]
//!               [--screenshot out.png] [--iso PATH]
//!   LEVEL: EP1L01 (default) or EP1/L01. --model 0x324 shows one EF_3DOBJ_RES (e.g. the book).
//!   --anim 0x325 plays (looped) the EFHAnimRes with that uid through engine/anim's
//!   EFAnimationMgr on every shown object whose hierarchy has the same node count.
//!   --time: animation time at which the screenshot is taken (default 0.2 s).
//!   --particles runs the EngineRuntime (logic + PLMgr_BC + EFBillboardMgr) next to the viewer and
//!   draws RuntimeFrame::particles/billboards with `ApplyParticles`.
//!   --screenshot renders headless (no window) into an image and saves it, then exits.
//! Free-fly: WASD move, Q/E down/up, right mouse drag look, Shift fast, Esc quit.
use bevy::app::{AppExit, ScheduleRunnerPlugin};
use bevy::camera::RenderTarget;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::image::Image;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::light::GlobalAmbientLight;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::ExitCondition;
use bevy::winit::WinitPlugin;
use scooby_recomp::engine::Engine;
use scooby_recomp::engine::anim::mgr::EFAnimationMgr;
use scooby_recomp::engine::anim::nodes::{EFHAnimNode, state};
use scooby_recomp::engine::anim::res::EFHAnimRes;
use scooby_recomp::engine::anim::{AnimInstanciator, api::APIanim_Play};
use scooby_recomp::engine::camera::DTMatrix;
use scooby_recomp::engine::dt::descriptors::{DT3dObjectDescriptor, EFIDescriptor};
use scooby_recomp::engine::gfx::bevy_adapter::*;
use scooby_recomp::engine::gfx::hierarchy::{IDENTITY, Mat4 as RwMat4};
use scooby_recomp::engine::gfx::mgr::EFGraphicsMgr;
use scooby_recomp::engine::gfx::render::{EF3dObj_SP, NoTexAnim, OwnerInfo};
use scooby_recomp::engine::gfx::{EF3dObjRes, EFStatic3dObj, EFTextureDict};
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::pack::EFRessourcesMgr;
use scooby_recomp::engine::rw::image::RwTexDictionary;
use scooby_recomp::engine::runtime::EngineRuntime;
use std::sync::Arc;

struct Args {
    level: String,
    model: Option<u32>,
    anim: Option<u32>,
    time: f32,
    particles: bool,
    screenshot: Option<String>,
    iso: String,
}

fn hex(s: Option<String>) -> Option<u32> {
    s.and_then(|m| u32::from_str_radix(m.trim_start_matches("0x"), 16).ok())
}

fn parse_args() -> Args {
    let mut a = Args { level: "EP1/L01".into(), model: None, anim: None, time: 0.2, particles: false, screenshot: None, iso: String::new() };
    let mut it = std::env::args().skip(1);
    while let Some(s) = it.next() {
        match s.as_str() {
            "--screenshot" => a.screenshot = it.next(),
            "--model" => a.model = hex(it.next()),
            "--anim" => a.anim = hex(it.next()),
            "--time" => a.time = it.next().and_then(|t| t.parse().ok()).unwrap_or(0.2),
            "--particles" => a.particles = true,
            "--iso" => a.iso = it.next().unwrap_or_default(),
            l => {
                let l = l.to_ascii_uppercase();
                // EP1L01 -> EP1/L01
                a.level = if l.contains('/') { l } else { l.find('L').map(|i| format!("{}/{}", &l[..i], &l[i..])).unwrap_or(l) };
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

/// One placed EF3dObj: owner uid, resource uid, owner LTM.
struct Placed {
    owner: u32,
    res: u32,
    ltm: RwMat4,
}

#[derive(Resource)]
struct Scene {
    dict: RwTexDictionary,
    res: Vec<EF3dObjRes>,
    stat: Option<EFStatic3dObj>,
    lights: RwLights,
    placed: Vec<Placed>,
    start: DTMatrix,
    anim: Option<EFAnimationMgr>,
}

/// Per-frame state: the placed EF3dObj instances and the graphics manager they render into.
#[derive(Resource)]
struct Live {
    dict: RwTexDictionary,
    res: Vec<EF3dObjRes>,
    placed: Vec<Placed>,
    objs: Vec<Option<EF3dObj_SP>>,
    mgr: EFGraphicsMgr,
    anim: Option<EFAnimationMgr>,
    frame: u32,
}

fn frame_ltm(pos: [f32; 3], at: [f32; 3], up: [f32; 3], right: [f32; 3]) -> RwMat4 {
    [right[0], right[1], right[2], 0., up[0], up[1], up[2], 0., at[0], at[1], at[2], 0., pos[0], pos[1], pos[2], 1.]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// RW camera frame at `pos` looking at `target` (+at forward, +right = screen-left, RW convention).
fn look_at(pos: [f32; 3], target: [f32; 3]) -> DTMatrix {
    let d = [target[0] - pos[0], target[1] - pos[1], target[2] - pos[2]];
    let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-6);
    let at = [d[0] / l, d[1] / l, d[2] / l];
    let mut right = cross([0.0, 1.0, 0.0], at);
    let rl = (right[0] * right[0] + right[1] * right[1] + right[2] * right[2]).sqrt().max(1e-6);
    right = [right[0] / rl, right[1] / rl, right[2] / rl];
    let up = cross(at, right);
    DTMatrix { right, up, at, pos }
}

fn puid_of(g: &scooby_recomp::engine::level::EFGameObject) -> Option<u32> {
    g.glo.object_descriptors.objects.iter().find_map(|d| d.as_any().downcast_ref::<EFIDescriptor<DT3dObjectDescriptor>>().map(|x| x.desc.puid as u32))
}

fn load_scene(a: &Args) -> Scene {
    let fs = IsoFs::open(&a.iso).unwrap_or_else(|e| panic!("ISO {}: {e:?}", a.iso));
    let mut m = EFRessourcesMgr::default();
    m.level_file_name = Some(format!("{}.P2R", a.level.replace('/', "")));
    let err = m.LoadLevel(&fs, "", &a.level, "EN", &mut AnimInstanciator::default(), |_, _| {}).expect("LoadLevel");
    assert_eq!(err, 0, "LoadLevel {}", a.level);
    let mut dict = RwTexDictionary::default();
    for (p, b) in &m.side_files {
        if p.to_ascii_uppercase().contains("TEXDIC_") {
            EFTextureDict::LoadPropertiesBinary(b, &mut dict);
        }
    }
    let (mut res, mut stat, mut anim) = (Vec::new(), None, None);
    for o in m.objects.iter_mut() {
        if let (Some(uid), Some(d)) = (a.anim, o.as_dt()) {
            if let Some(h) = d.as_any().downcast_ref::<EFHAnimRes>() {
                if h.base.hdr.uid == uid {
                    // EFGameObject::CreateInstances: manager base 1, node id 1 (first anim descriptor)
                    let mut mgr = EFAnimationMgr::new(1, 0);
                    mgr.owner_uid = 0x2a00_0001;
                    let mut n = EFHAnimNode::new(Arc::new(h.clone()), 1);
                    n.obj = 1;
                    mgr.AddAnim(n, 1);
                    APIanim_Play(Some(&mut mgr), 1, state::LOOP, 0.0);
                    anim = Some(mgr);
                }
            }
        }
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
    if a.anim.is_some() && anim.is_none() {
        panic!("no EFHAnimRes {:#x}", a.anim.unwrap());
    }
    let lights = stat.as_ref().map(|s| RwLights::from_static(&s.lights)).unwrap_or_default();
    let mut placed = Vec::new();
    let mut start = look_at([0.0, 10.0, -20.0], [0.0, 0.0, 0.0]);
    if let Some(uid) = a.model {
        let r = res.iter().find(|r| r.res.uid == uid).unwrap_or_else(|| panic!("no EF_3DOBJ_RES {uid:#x}"));
        let bs = r.clump.as_ref().and_then(|c| c.geometries.first()).and_then(|g| g.morph_targets.first()).map(|m| m.bsphere).unwrap_or([0., 0., 0., 1.]);
        placed.push(Placed { owner: 0x2a00_0001, res: uid, ltm: IDENTITY });
        let c = [bs[0], bs[1], bs[2]];
        let r = bs[3].max(0.1) * 2.2;
        // front view: RW models face +Z (their frame +at)
        start = look_at([c[0] + r * 0.35, c[1] + r * 0.3, c[2] + r], c);
        stat = None;
    } else {
        for g in &m.game_objects {
            if g.IsLatent() {
                continue; // latent objects are not in the world until the logic activates them
            }
            let Some(res_uid) = puid_of(g) else { continue };
            let gl = &g.glo;
            placed.push(Placed { owner: g.uid, res: res_uid, ltm: frame_ltm(gl.pos, gl.look_at, gl.look_up, gl.look_right) });
        }
        // start camera: behind and above Scooby's spawn when present, looking along his look-at
        // EP1L01 GLO 0x2a000092 = Scooby (engine_c3); other levels: first placed object
        let hero = m.game_objects.iter().find(|g| g.uid == 0x2a00_0092);
        if let Some(h) = hero {
            let (p, at) = (h.glo.pos, h.glo.look_at);
            // in front of the hero, looking back at him
            start = look_at([p[0] + at[0] * 6.0, p[1] + 3.0, p[2] + at[2] * 6.0], [p[0], p[1] + 1.0, p[2]]);
            eprintln!("viewer: hero at {p:?} look {at:?}");
        }
    }
    eprintln!("viewer: {} {} objects, {} clumps, world {}, {} atomic lights", a.level, placed.len(), res.len(), stat.is_some(), lights.lights.len());
    Scene { dict, res, stat, lights, placed, start, anim }
}

#[derive(Resource)]
struct Shot {
    path: String,
    image: Handle<Image>,
    frame: u32,
}

#[derive(Component)]
struct FlyCam {
    yaw: f32,
    pitch: f32,
}

/// EngineRuntime + Engine run next to the viewer for --particles (not Send: NonSend resource).
struct Runtime {
    e: Engine,
    rt: EngineRuntime,
}

fn main() {
    let a = parse_args();
    let scene = load_scene(&a);
    let mut app = App::new();
    match &a.screenshot {
        Some(_) => {
            app.add_plugins(
                DefaultPlugins
                    .set(WindowPlugin { primary_window: None, exit_condition: ExitCondition::DontExit, ..default() })
                    .disable::<WinitPlugin>(),
            )
            .add_plugins(ScheduleRunnerPlugin::run_loop(std::time::Duration::from_millis(5)));
        }
        None => {
            app.add_plugins(DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window { title: format!("Scooby-Doo MM viewer - {}", a.level), ..default() }),
                ..default()
            }));
        }
    }
    if a.particles {
        let fs = IsoFs::open(&a.iso).expect("ISO");
        let (mut e, mut rt) = EngineRuntime::new_with_engine();
        let res = EngineRuntime::ReadLevel(&fs, &a.level, &format!("{}.P2R", a.level.replace('/', ""))).expect("ReadLevel");
        rt.LoadLevel(&mut e, res);
        app.insert_non_send_resource(Runtime { e, rt });
    }
    app.add_plugins(GfxAdapterPlugin)
        .insert_resource(GlobalAmbientLight { color: Color::WHITE, brightness: 0.0, ..default() })
        .insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.08)))
        .insert_resource(scene)
        .add_systems(Startup, setup)
        .add_systems(Update, tick);
    if let Some(p) = &a.screenshot {
        // headless: fixed 1/60 s steps; the anim time at capture is --time
        let at = ((a.time * 60.0) as u32).max(1);
        app.insert_resource(Shot { path: p.clone(), image: Handle::default(), frame: 0 }).insert_resource(CaptureAt(at)).add_systems(Update, screenshot.after(tick));
    } else {
        app.add_systems(Update, fly.before(tick));
    }
    app.run();
}

#[derive(Resource)]
struct CaptureAt(u32);

fn setup(world: &mut World) {
    let scene = world.remove_resource::<Scene>().unwrap();
    if let Some(s) = &scene.stat {
        spawn_static_world(world, s, &scene.dict);
    }
    if !scene.lights.lights.is_empty() {
        world.insert_resource(scene.lights.clone());
    } else {
        // no level lights (should not happen): RW default = unlit atomics, Bevy fallback light
        world.spawn((DirectionalLight { illuminance: 4000.0, ..default() }, Transform::from_xyz(3.0, 10.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y)));
        world.insert_resource(GlobalAmbientLight { color: Color::WHITE, brightness: 900.0, ..default() });
    }
    // EF3dObj instances: Initialize, SetVisible (Render runs every frame in `tick`)
    let mut res = scene.res;
    let objs = scene
        .placed
        .iter()
        .map(|p| {
            let r = res.iter_mut().find(|r| r.res.uid == p.res)?;
            let c = r.clump.as_mut()?;
            let mut o = EF3dObj_SP::Initialize(p.res, c, 0);
            o.SetVisible(1);
            Some(o)
        })
        .collect();
    world.insert_resource(Live { dict: scene.dict, res, placed: scene.placed, objs, mgr: EFGraphicsMgr::new(640, 448), anim: scene.anim, frame: 0 });
    let t = rw_camera_transform(&scene.start);
    let (yaw, pitch, _) = t.rotation.to_euler(EulerRot::YXZ);
    let target = world.contains_resource::<Shot>().then(|| {
        let mut img = Image::new_target_texture(1280, 720, TextureFormat::Rgba8UnormSrgb, None);
        img.texture_descriptor.usage |= bevy::render::render_resource::TextureUsages::COPY_SRC;
        let h = world.resource_mut::<Assets<Image>>().add(img);
        world.resource_mut::<Shot>().image = h.clone();
        h
    });
    let mut e = world.spawn((Camera3d::default(), Tonemapping::None, t, FlyCam { yaw, pitch }));
    e.insert(Projection::Perspective(PerspectiveProjection { fov: 60f32.to_radians(), near: 0.1, far: 2000.0, ..default() }));
    if let Some(h) = target {
        e.insert(RenderTarget::Image(h.into()));
    }
}

/// One viewer frame: EFAnimationMgr::Update -> EF3dObj transfo pool 0 -> UpdateHierarchy ->
/// frame LTMs -> EF3dObj_SP::Render -> ApplyDrawList; optional runtime frame -> ApplyParticles.
fn tick(world: &mut World) {
    let headless = world.contains_resource::<Shot>();
    let stop = world.get_resource::<CaptureAt>().map(|c| c.0);
    let dt = if headless { 1.0 / 60.0 } else { world.resource::<Time>().delta_secs().min(0.1) };
    let cam = world.query_filtered::<&Transform, With<Camera3d>>().iter(world).next().copied();
    let mut live = world.remove_resource::<Live>().unwrap();
    live.frame += 1;
    // headless: freeze the scene at the capture frame (the screenshot is read back later)
    let advance = stop.is_none_or(|s| live.frame <= s);
    if advance {
        let pose = live.anim.as_mut().and_then(|m| {
            m.Update(dt);
            m.GetPose(1)
        });
        live.mgr.draws.clear();
        let Live { res, placed, objs, mgr, .. } = &mut live;
        for (p, o) in placed.iter().zip(objs.iter_mut()) {
            let Some(o) = o else { continue };
            let Some(c) = res.iter_mut().find(|r| r.res.uid == p.res).and_then(|r| r.clump.as_mut()) else { continue };
            let mut owner = p.ltm;
            if let Some(pose) = &pose {
                // pose transfos carry their hierarchy node index (DTTransfo +0x44 id); the
                // anim may cover fewer nodes than the hierarchy (unanimated nodes keep rest)
                let n = o.hier.transfos[0].len();
                if !pose.is_empty() && pose.iter().all(|t| (t.id as usize) < n && t.id >= 0) {
                    for t in pose {
                        o.hier.transfos[0][t.id as usize] = t.clone();
                    }
                    o.base.flags |= 0x80;
                    let flags = o.base.flags;
                    let rest = c.frames.iter().map(|f| f.local_matrix()).collect::<Vec<_>>();
                    o.hier.UpdateHierarchy(flags, &mut o.frames, &mut owner);
                    // nodes the anim does not drive keep their clump rest matrix
                    if let Some(h) = &o.hier.hierarchy {
                        for (i, f) in h.node_frames.iter().enumerate() {
                            if let Some(f) = f {
                                if !pose.iter().any(|t| t.id as usize == i) {
                                    if let Some(r) = rest.get(*f) {
                                        o.frames.local[*f] = *r;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            o.SyncFrames(p.ltm);
            o.Render(0, c, &OwnerInfo { uid: p.owner, ltm: p.ltm, ..Default::default() }, mgr, &mut NoTexAnim);
        }
        let look = |o: u32| {
            let p = live.placed.iter().find(|p| p.owner == o)?;
            live.res.iter().find(|r| r.res.uid == p.res).and_then(|r| r.clump.as_ref()).map(|c| (p.res, c))
        };
        ApplyDrawList(world, &live.mgr.draws, &look, &live.dict);
        if live.frame == 1 {
            eprintln!("viewer: {} draw cmds, {} mesh entities", live.mgr.draws.len(), world.resource::<DrawListState>().visible);
        }
        if let (Some(mut r), Some(t)) = (world.remove_non_send_resource::<Runtime>(), cam) {
            r.rt.Frame(&mut r.e, dt);
            // Bevy camera = RW LTM * diag(-1,1,-1): RW right = -X, up = Y, at = -Z
            let (x, y, z) = (t.right().as_vec3(), t.up().as_vec3(), t.back().as_vec3());
            let ltm = DTMatrix { right: (-x).into(), up: y.into(), at: (-z).into(), pos: t.translation.into() };
            let ps: Vec<_> = r.rt.frame.particles.iter().map(|b| b.1.clone()).collect();
            let bs: Vec<_> = r.rt.frame.billboards.iter().map(|b| b.1.clone()).collect();
            ApplyParticles(world, &ps, &bs, &ltm, &r.rt.gfx.tex_dict);
            if live.frame % 60 == 0 {
                eprintln!("viewer: frame {} particle batches {} billboard batches {}", live.frame, ps.len(), bs.len());
            }
            world.insert_non_send_resource(r);
        }
    }
    world.insert_resource(live);
}

fn screenshot(mut commands: Commands, mut shot: ResMut<Shot>, at: Res<CaptureAt>, mut exit: MessageWriter<AppExit>) {
    shot.frame += 1;
    // pipelines compile during the first ~400 frames; capture after both are done
    let f = at.0.max(400);
    if shot.frame == f {
        commands.spawn(Screenshot::image(shot.image.clone())).observe(save_to_disk(shot.path.clone()));
    }
    if shot.frame > f + 60 && std::path::Path::new(&shot.path).exists() {
        exit.write(AppExit::Success);
    }
    if shot.frame > f + 2000 {
        eprintln!("viewer: screenshot timed out");
        exit.write(AppExit::error());
    }
}

fn fly(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    mut q: Query<(&mut Transform, &mut FlyCam)>,
    mut exit: MessageWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
    let Ok((mut t, mut c)) = q.single_mut() else { return };
    if buttons.pressed(MouseButton::Right) {
        c.yaw -= motion.delta.x * 0.003;
        c.pitch = (c.pitch - motion.delta.y * 0.003).clamp(-1.5, 1.5);
    }
    t.rotation = Quat::from_euler(EulerRot::YXZ, c.yaw, c.pitch, 0.0);
    let mut v = Vec3::ZERO;
    for (k, d) in [(KeyCode::KeyW, t.forward().as_vec3()), (KeyCode::KeyS, -t.forward().as_vec3()), (KeyCode::KeyD, t.right().as_vec3()), (KeyCode::KeyA, -t.right().as_vec3()), (KeyCode::KeyE, Vec3::Y), (KeyCode::KeyQ, -Vec3::Y)] {
        if keys.pressed(k) {
            v += d;
        }
    }
    let speed = if keys.pressed(KeyCode::ShiftLeft) { 40.0 } else { 10.0 };
    t.translation += v * speed * time.delta_secs();
}

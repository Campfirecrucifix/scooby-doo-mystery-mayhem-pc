//! WP-J1: engine runtime. Loads real levels, runs frames in MainLoop order and checks that the
//! particle system is fed from the level data / logic and produces particles and draw batches.
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::particle::PLPrims;
use scooby_recomp::engine::runtime::*;
use scooby_recomp::engine::Engine;

use crate::common::ISO;

struct Stats {
    max_particles: i32,
    batches: usize,
    sprites: usize,
    emitting: std::collections::BTreeSet<String>,
}

/// `cam`: pin the view at this position (looking +z, the RwFrame identity orientation) instead of
/// the camera object's frame. Since WP-L1 the l01_camera logic drives the EP1L01 camera (start
/// attractor at [-150.6,4.2,4.3], not rendering until its fade-in), so particle checks pin it.
fn run(e: &mut Engine, rt: &mut EngineRuntime, frames: u32, cam: Option<[f32; 3]>) -> Stats {
    let mut s = Stats { max_particles: 0, batches: 0, sprites: 0, emitting: Default::default() };
    for _ in 0..frames {
        // EngineRuntime::Frame with the camera pinned between SyncCameras and StepRender
        let raw = 1.0 / 60.0;
        // P27 R8: the runtime also syncs the cameras before physics/particles (91d3db55), so detach them from
        // their owner frames for the whole pinned frame (restored below), else the particle clip sees the game camera
        let owners: Vec<u32> = rt.gfx.cameras.cameras.iter().flatten().map(|c| c.frame_owner).collect();
        if let Some(p) = cam {
            for c in rt.gfx.cameras.cameras.iter_mut().flatten() {
                c.ltm.right = [1.0, 0.0, 0.0];
                c.ltm.up = [0.0, 1.0, 0.0];
                c.ltm.at = [0.0, 0.0, 1.0];
                c.ltm.pos = p;
                c.frame_owner = u32::MAX;
            }
        }
        rt.StepInput(e, raw);
        let dt = rt.StepLogic(e, raw);
        rt.StepParticles(e, dt);
        rt.StepGfxApi(dt);
        EngineRuntime::StepFinishWorld(e);
        rt.SyncCameras();
        if let Some(p) = cam {
            for c in rt.gfx.cameras.cameras.iter_mut().flatten() {
                c.ltm.right = [1.0, 0.0, 0.0];
                c.ltm.up = [0.0, 1.0, 0.0];
                c.ltm.at = [0.0, 0.0, 1.0];
                c.ltm.pos = p;
                c.render_flag = 0xffff_ffef;
            }
        }
        rt.StepRender(e, dt);
        rt.StepAudio(dt);
        if cam.is_some() {
            for (c, o) in rt.gfx.cameras.cameras.iter_mut().flatten().zip(owners) {
                c.frame_owner = o;
            }
        }
        let g = rt.particles.lock().unwrap();
        s.max_particles = s.max_particles.max(g.pl.nb_particles);
        for t in &g.pl.types {
            if t.d.nb_active > 0 {
                s.emitting.insert(t.d.name.clone());
            }
        }
        drop(g);
        s.batches += rt.frame.particles.len();
        for (_, b) in &rt.frame.particles {
            s.sprites += match &b.prims {
                PLPrims::Sprites(v) => v.len(),
                PLPrims::Quads(v) => v.len(),
            };
        }
    }
    s
}

fn dump(rt: &EngineRuntime, tag: &str) {
    let g = rt.particles.lock().unwrap();
    for t in &g.pl.types {
        let em: Vec<String> = t.emitters.iter().map(|e| format!("{}@{:08x}", e.name, e.object_uid)).collect();
        eprintln!("{tag}: type {:08x} '{}' tex {:?} active {} life {} rate {} count {} flags {:x} emitters {:?} eflags {:?}", t.d.id, t.d.name, t.d.texture, t.d.nb_active, t.d.lifespan, t.d.rate, t.d.count, t.d.flags, em, t.emitters.iter().map(|e| (e.flags, e.iflags, e.started)).collect::<Vec<_>>());
    }
}

#[test]
fn ep1l01_logic_particles() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = EngineRuntime::new_with_engine();
    let res = EngineRuntime::ReadLevel(&fs, "EP1/L01", "EP1L01.P2R").unwrap();
    rt.LoadLevel(&mut e, res);
    assert!(!rt.gfx.tex_dict.textures.is_empty(), "TXD loaded");
    assert!(rt.objects.len() > 0);
    let mut s = run(&mut e, &mut rt, 10, Some([0.0; 3]));
    // P23 R62: with em_child (default on) the ghost glows are joint to their ghost (EmitterSetJoint SELF), no
    // longer at the world origin, so pin the view 5 units in front of the first TRAINEE_ emitter's frame
    let gp = {
        let g = rt.particles.lock().unwrap();
        g.pl.types.iter().filter(|t| t.d.name.starts_with("TRAINEE_")).flat_map(|t| t.emitters.iter()).find_map(|em| em.frame.map(|f| f.ltm.pos)).unwrap_or([0.0, 0.0, 5.0])
    };
    let s2 = run(&mut e, &mut rt, 110, Some([gp[0], gp[1], gp[2] - 5.0]));
    s.max_particles = s.max_particles.max(s2.max_particles);
    s.batches += s2.batches;
    s.sprites += s2.sprites;
    s.emitting.extend(s2.emitting);
    dump(&rt, "EP1L01");
    eprintln!("EP1L01: max particles {} batches {} sprites {} emitting {:?} passes {}", s.max_particles, s.batches, s.sprites, s.emitting, rt.frame.passes.len());
    let g = rt.particles.lock().unwrap();
    let nb_em: usize = g.pl.types.iter().map(|t| t.emitters.len()).sum();
    assert!(nb_em > 0, "logic created no emitters");
    assert!(s.max_particles > 0, "no particle emitted");
    assert!(s.sprites > 0, "no particle drawn");
    // the ghost glows (GhostGlowInit: texture "glow", TRAINEE_* types) emit from the start
    assert!(s.emitting.iter().any(|n| n.starts_with("TRAINEE_")), "ghost glow did not emit");
    // the wisp (glowing orb) collect effects exist, idle until a wisp is collected
    let collect: Vec<u32> = g.pl.types.iter().filter(|t| t.d.name.starts_with("wispcollect_")).flat_map(|t| t.emitters.iter().map(|e| e.id)).collect();
    assert_eq!(collect.len(), 6, "wispcollect spark/halo small/medium/large emitters");
    drop(g);
    // the emitters clip against the camera (flags 0x80: no update when clipped): put the
    // camera 5 units in front of the first one, as the game camera would be when collecting
    let p = {
        let g = rt.particles.lock().unwrap();
        let (t, i) = g.pl.GetEmitter(collect[0] as i32).expect("collect emitter");
        g.pl.types[t].emitters[i].frame.map(|f| f.ltm.pos).unwrap_or([0.0; 3])
    };
    let _ = p;
    for c in rt.gfx.cameras.cameras.iter_mut().flatten() {
        c.ltm.pos = [p[0], p[1], p[2] - 5.0];
    }
    // collecting: Wisp_base starts them with EmitterStart(id, 1)
    // GenericFX attaches the effect to the collected wisp (EmitterSetJoint) first
    let obj = *rt.particles.lock().unwrap().world.frames.keys().min().unwrap();
    for &id in &collect {
        let mut g = rt.particles.lock().unwrap();
        let ParticleShared { pl, world, .. } = &mut *g;
        pl.APIparticle_EmitterSetJoint(&mut e.mem, id as i32, obj, 0, world);
        pl.APIparticle_EmitterStart(&mut e.mem, id as i32, 1);
    }
    let op = rt.particles.lock().unwrap().world.frames[&obj].pos;
    let s2 = run(&mut e, &mut rt, 30, Some([op[0], op[1], op[2] - 5.0]));
    eprintln!("after collect: emitting {:?}", s2.emitting);
    assert!(s2.emitting.iter().any(|n| n.starts_with("wispcollect_")), "wisp collect effect did not emit");
}

#[test]
fn ep4l01_level_data_emitters() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = EngineRuntime::new_with_engine();
    let res = EngineRuntime::ReadLevel(&fs, "EP4/L01", "EP4L01.P2R").unwrap();
    rt.LoadLevel(&mut e, res);
    {
        let g = rt.particles.lock().unwrap();
        assert!(g.pl.nb_types > 0, "0x29 types loaded");
        assert!(g.pl.nb_emitters > 0, "0x25 emitters loaded");
    }
    dump(&rt, "EP4L01 load");
    let s = run(&mut e, &mut rt, 120, None);
    eprintln!("EP4L01: max particles {} sprites {} emitting {:?}", s.max_particles, s.sprites, s.emitting);
    assert!(s.max_particles > 0);
}

#[test]
fn bevy_plugin_frames() {
    use bevy_app::App;
    if IsoFs::open(ISO).is_err() {
        return;
    }
    let mut app = App::new();
    app.add_plugins(EngineRuntimePlugin);
    app.insert_resource(RuntimeFixedDt(1.0 / 60.0));
    app.insert_resource(spawn_level_load(ISO.into(), "EP1/L01".into(), "EP1L01.P2R".into()));
    let t0 = std::time::Instant::now();
    while !app.world().resource::<EngineRuntime>().level_loaded {
        app.update();
        assert!(t0.elapsed().as_secs() < 120, "level load timed out");
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let f0 = app.world().resource::<EngineRuntime>().frame_count;
    for _ in 0..120 {
        app.update();
    }
    let rt = app.world().resource::<EngineRuntime>();
    assert_eq!(rt.frame_count - f0, 120);
    assert!(rt.frame.passes.iter().any(|p| matches!(p, scooby_recomp::engine::gfx::mgr::RenderPass::Particles)));
}

#[test]
fn libc_rand_sequence() {
    // newlib rand() with seed 1
    let r = LibcRand::default();
    let a = [r.rand(), r.rand()];
    let mut s: u64 = 1;
    let mut nx = || {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((s >> 32) & 0x7fff_ffff) as u32
    };
    assert_eq!(a, [nx(), nx()]);
}

//! WP-L1: the real EP1L01 camera logic (l01_camera, CameraFree follow camera) drives camera object
//! 0x2a000000: after level start it sits behind/above the leader and keeps following while he walks.
use scooby_recomp::engine::input::PadState;
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::runtime::*;
use scooby_recomp::engine::*;

const ISO: &str = "../Scooby Doo Mystery Mayhem.ISO";

/// APIgameobj_GetPosition (object frame LTM) as the logic sees it
fn pos(e: &mut Engine, id: u32) -> [f32; 3] {
    let p = e.alloca(0x10);
    APIgameobj::GetPosition(e, id, 0x7fff_ffff, p);
    e.rv3(p)
}

fn hdist(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

#[test]
fn ep1l01_camera_follows_leader() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = EngineRuntime::new_with_engine();
    let res = EngineRuntime::ReadLevel(&fs, "EP1/L01", "EP1L01.P2R").unwrap();
    rt.LoadLevel(&mut e, res);
    // the camera object: the one bound to the l01_camera code (0x2a000000 is SELF in module ids)
    #[allow(non_snake_case)]
    let CAM = e
        .rt
        .objects
        .iter()
        .flatten()
        .find(|o| o.inst.code.map_or(false, |c| e.rt.codes[c].def.name == "l01_camera"))
        .map(|o| o.id)
        .expect("camera object");
    eprintln!("camera object {CAM:x}");
    let mut states = vec![];
    let mut max_d = 0.0f32;
    let mut cam_start = None;
    for f in 0..400u32 {
        {
            let mut g = rt.particles.lock().unwrap();
            let mut p = PadState { connected: true, mode: 7, rx: 128, ry: 128, lx: 128, ly: 128, ..Default::default() };
            if f >= 150 {
                p.ly = 0;
            }
            g.devices.SetPadState(0, p);
        }
        rt.Frame(&mut e, 1.0 / 60.0);
        let s = scooby_recomp::engine::core::apigameobj::GetCurrentStateID(&mut e, CAM);
        if states.last() != Some(&s) {
            states.push(s);
        }
        let gb0 = e.rt.game_buffers[&0].0;
        let leader = e.mem.r32(gb0 + 8);
        if f % 25 == 0 {
            let (c, l) = (pos(&mut e, CAM), pos(&mut e, leader));
            let code = e.rt.codes.iter().find(|c| c.def.name == "l01_camera").unwrap();
            let d = code.data;
            let g = |a: u32| e.mem.r32(d + a - 0x10000);
            let gf = |a: u32| f32::from_bits(g(a));
            if f==0 { eprintln!("curve {:?}", (0..12).map(|i| gf(0x10180+i*4)).collect::<Vec<_>>()); }
            eprintln!(
                "f{f} cam {c:?} leader {l:?} ss {} flags {:x} dist {} tgt {} haut {} impy {} lfy {:?}",
                g(0x1016c), g(0x10168), gf(0x10334), gf(0x1033c), gf(0x10330), gf(0x102f4),
                [gf(0x10310), gf(0x10314), gf(0x10318)]
            );
        }
        if f == 149 {
            cam_start = Some(pos(&mut e, CAM));
        }
        if f >= 120 {
            let d = hdist(pos(&mut e, CAM), pos(&mut e, leader));
            max_d = max_d.max(d);
        }
    }
    let gb0 = e.rt.game_buffers[&0].0;
    let leader = e.mem.r32(gb0 + 8);
    let (c, l) = (pos(&mut e, CAM), pos(&mut e, leader));
    let moved = hdist(cam_start.unwrap(), c);
    eprintln!("cam {c:?} leader {l:?} states {states:x?} max dist {max_d} cam moved {moved}");
    assert!(states.contains(&2), "camera never reached state 2 (BASICMOVE): {states:x?}");
    assert!(hdist(c, l) < 8.0 && max_d < 10.0, "camera lost the leader");
    assert!(c[1] > l[1], "camera not above the leader");
    assert!(moved > 0.5, "camera did not follow ({moved} m)");
}

//! WP-J4: the leader walks on the EP1L01 map with the left stick held (normal module path:
//! collision coordinator stage env prims + manager access for the DMs), and stays on the floor.
use scooby_recomp::engine::input::PadState;
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::runtime::*;

const ISO: &str = "../Scooby Doo Mystery Mayhem.ISO";
const SHAGGY: u32 = 0x2a00_0091;
const SCOOBY: u32 = 0x2a00_0092;

fn pos(rt: &EngineRuntime, id: u32) -> [f32; 3] {
    let g = rt.particles.lock().unwrap();
    let ph = g.sub.physic.PhysicOf(id);
    g.sub.physic.Physic(ph).unwrap().frame.pos
}

#[test]
fn ep1l01_hero_walks_on_floor() {
    let fs = IsoFs::open(ISO).expect("ISO missing: tests must load real data");
    let (mut e, mut rt) = EngineRuntime::new_with_engine();
    let res = EngineRuntime::ReadLevel(&fs, "EP1/L01", "EP1L01.P2R").unwrap();
    rt.LoadLevel(&mut e, res);
    let mut start = None;
    let mut ys = vec![];
    let mut states = vec![];
    for f in 0..300u32 {
        {
            let mut g = rt.particles.lock().unwrap();
            let mut p = PadState { connected: true, mode: 7, rx: 128, ry: 128, lx: 128, ly: 128, ..Default::default() };
            if f >= 100 {
                p.ly = 255; // left stick fully down (towards the l01_camera start view)
            }
            g.devices.SetPadState(0, p);
        }
        rt.Frame(&mut e, 1.0 / 60.0);
        // l01_camera Camera_BaseCB_INIT: goHeroesData[1] -> Scooby leads
        let gb0 = e.rt.game_buffers[&0].0;
        let leader = e.mem.r32(gb0 + 8);
        if f == 99 {
            assert_eq!(leader, SCOOBY, "camera did not set the leader");
            start = Some(pos(&rt, leader));
        }
        if f >= 100 {
            let p = pos(&rt, leader);
            ys.push(p[1]);
        }
        let s = scooby_recomp::engine::core::apigameobj::GetCurrentStateID(&mut e, SCOOBY);
        if states.last() != Some(&s) {
            states.push(s);
        }
    }
    let a = start.unwrap();
    let b = pos(&rt, SCOOBY);
    let d = ((b[0] - a[0]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
    let (ymin, ymax) = ys.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &y| (lo.min(y), hi.max(y)));
    eprintln!("start {a:?} end {b:?} horiz {d} y [{ymin}, {ymax}] states {states:x?} shaggy {:?}", pos(&rt, SHAGGY));
    assert!(d > 1.0, "leader did not walk ({d} m)");
    assert!(ymax - ymin < 1.0, "leader left the floor (y {ymin}..{ymax})");
}

/// WP-J4 steps 3/4: the leader opens the book next to a ghost; the book latches (TomeDoomFUNC_SET_CAPTURE
/// launches the tod_beam latent, whose ToD_Beam_Create_Sprites goes through APIgraphics_Sprite*), the
/// player mashes the requested buttons and the run goes on until the ghost-captured event
/// (book msg 0xd6/0x68 -> Monster_BaseST_CAPTURED -> msg 0xea to rdrmngr).
#[test]
fn ep1l01_book_captures_ghost() {
    use scooby_recomp::engine::core::apigameobj::GetCurrentStateID;
    use scooby_recomp::engine::input::pad;
    let fs = IsoFs::open(ISO).expect("ISO missing: tests must load real data");
    let (mut e, mut rt) = EngineRuntime::new_with_engine();
    let res = EngineRuntime::ReadLevel(&fs, "EP1/L01", "EP1L01.P2R").unwrap();
    rt.LoadLevel(&mut e, res);
    const BOOK: u32 = 0x2a00_0093;
    const GHOST: u32 = 0x2a00_0008;
    let mut book_states = vec![];
    let mut ghost_states = vec![];
    let mut max_sprites = 0usize;
    let mut beam_seen = false;
    let mut captured_at = None;
    for f in 0..3000u32 {
        if f == 10 {
            // book handover with every monster type enabled (msg 0xe5: GB2 |= 0x7e, GrabBook(1));
            // 0x70 only sets GB2 bit 0, so TomeDoomFUNC_ScanForMonster's mask GB2 & 0x7e is empty
            rt.particles.lock().unwrap().logic = Some(&e.rt as *const _);
            e.SendLogicMessageTo(BOOK, 0xe5, 0, 0, 0);
            // a full book (power GB1+4 = 180, the cap of Wisp_baseState_Picked_Execute) — the player
            // would collect orbs first; every drain step costs power*0.9
            let gb1 = e.rt.game_buffers[&1].0;
            e.mem.w32(gb1 + 4, 180f32.to_bits());
            rt.particles.lock().unwrap().logic = None;
        }
        if f == 100 {
            // test set-up: put the ghost 3 m in front of the leader (Scooby)
            let mut g = rt.particles.lock().unwrap();
            let ph = g.sub.physic.PhysicOf(SCOOBY);
            let hp = g.sub.physic.Physic(ph).unwrap().frame;
            let gp = g.sub.physic.PhysicOf(GHOST);
            let target = [hp.pos[0] + hp.at[0] * 3.0, hp.pos[1] + 1.0, hp.pos[2] + hp.at[2] * 3.0];
            if let Some(p) = g.sub.physic.PhysicMut(gp) {
                p.frame.pos = target;
            }
            if let Some(fr) = g.world.frames.get_mut(&GHOST) {
                fr.pos = target;
            }
        }
        {
            let mut g = rt.particles.lock().unwrap();
            let mut p = PadState { connected: true, mode: 7, rx: 128, ry: 128, lx: 128, ly: 128, ..Default::default() };
            if (120..130).contains(&f) {
                p.buttons = pad::SQUARE;
            }
            // button mashing: the button the book asks for (book L+0x1e8: 2 X, 4 circle, 8 triangle —
            // TomeDoomCB_INPUT_X/CIRCLE/TRIANGLE set L+0x1e0 |= 0x12/0x14/0x18), on alternate frames
            if f > 145 && f % 2 == 0 {
                let h = e.rt.map.find(BOOK).unwrap();
                let l = e.rt.objects[h].as_ref().unwrap().inst.local;
                p.buttons = match e.mem.r32(l + 0x1e8) {
                    2 => pad::CROSS,
                    4 => pad::CIRCLE,
                    8 => pad::TRIANGLE,
                    _ => 0,
                };
            }
            g.devices.SetPadState(0, p);
        }
        rt.Frame(&mut e, 1.0 / 60.0);
        let n: usize = rt.frame.sprites.iter().map(|s| s.2.len()).sum();
        max_sprites = max_sprites.max(n);
        let g = rt.particles.lock().unwrap();
        beam_seen |= !g.gfxapi.sprites.groups.is_empty();
        drop(g);
        let bs = GetCurrentStateID(&mut e, BOOK);
        if book_states.last().map(|x: &(u32, u32)| x.1) != Some(bs) {
            book_states.push((f, bs));
        }
        let gs = GetCurrentStateID(&mut e, GHOST);
        if ghost_states.last().map(|x: &(u32, u32)| x.1) != Some(gs) {
            ghost_states.push((f, gs));
        }
        // Monster_BaseST_CAPTURED (state 0xb) or killed (go.KillObject at its end)
        if gs == 0xb || gs == 0xffff_ffff {
            captured_at = Some(f);
            break;
        }
    }
    eprintln!("book {book_states:x?}\nghost {ghost_states:x?}\nsprites max {max_sprites} groups {beam_seen} captured {captured_at:?}");
    assert!(beam_seen, "tod_beam sprite groups never created");
    assert!(max_sprites > 0, "no beam sprite drawn");
    assert!(captured_at.is_some(), "ghost never captured");
}

//! WP-J3: the Tome of Doom opens in the real runtime (EP1L01).
//!
//! Precondition (1:1 with the logic): heroValidOpenTome needs `*GB2 & 1` (book owned), which only
//! TomeDoomFUNC_GrabBook sets (book message 0x70 / 0xe5). In EP1L01 no ported logic sends 0x70 (it
//! comes from cut-scene/script data, not from a logic module: no EP1L01 module sends 0x70/0xe5 to the
//! book), so the test delivers msg 0x70 to the book the way the engine would (EFGameLogicMgr message,
//! no retriever). Leader/follower come from the ported l01_camera (Camera_BaseCB_INIT -> msg 0x47:
//! goHeroesData[1] = Scooby leader, Shaggy follower). Then SQUARE
//! (heroCB_INPUT_Tome) after the hero's 0.75 s input timer.
use scooby_recomp::engine::input::{pad, PadState};
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::runtime::*;

use crate::common::ISO;
const SHAGGY: u32 = 0x2a00_0091;
const SCOOBY: u32 = 0x2a00_0092;
const BOOK: u32 = 0x2a00_0093;
/// book open anims (EFAnimationMgr ids): Shaggy open 1, Scooby open 5 (BOOK_MECHANICS.md)
const OPEN: [u32; 2] = [1, 5];

#[test]
fn ep1l01_book_opens() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = EngineRuntime::new_with_engine();
    let res = EngineRuntime::ReadLevel(&fs, "EP1/L01", "EP1L01.P2R").unwrap();
    rt.LoadLevel(&mut e, res);
    let mut grabbed_at = None;
    let mut book_state = vec![];
    let mut hero_state: Vec<(u32, u32)> = vec![];
    for f in 0..400u32 {
        if f == 10 {
            // the level script hands the book over (TomeDoomFUNC_MessageHandler 0x70 -> GrabBook)
            rt.particles.lock().unwrap().logic = true;
            e.SendLogicMessageTo(BOOK, 0x70, 0, 0, 0);
            rt.particles.lock().unwrap().logic = false;
            grabbed_at = Some(f);
        }
        {
            let mut g = rt.particles.lock().unwrap();
            let mut p = PadState { connected: true, mode: 7, ..Default::default() };
            // SQUARE (action 0x21, device index 2 = heroCB_INPUT_Tome 0x14) held for a few frames once the 0.75 s input timer has run
            if (120..130).contains(&f) {
                p.buttons = pad::SQUARE;
            }
            if f == 120 {
                for c in &g.devices.input.configs {
                    eprintln!("DBG cfg {} state {:x} active {:x}", c.id, c.state, c.active);
                }
            }
            g.devices.SetPadState(0, p);
        }
        rt.Frame(&mut e, 1.0 / 60.0);
        let s = scooby_recomp::engine::core::apigameobj::GetCurrentStateID(&mut e, BOOK);
        if f % 20 == 0 { let g = rt.particles.lock().unwrap(); let ph = g.sub.physic.PhysicOf(SHAGGY); let p = g.sub.physic.Physic(ph).unwrap(); eprintln!("DBG f{f} pos {:?} vel {:?} flags {:x} prims {}", p.frame.pos, p.velocity, p.flags, p.model.prims.len()); }
        let hs = scooby_recomp::engine::core::apigameobj::GetCurrentStateID(&mut e, SHAGGY);
        if hero_state.last().map(|x: &(u32, u32)| x.1) != Some(hs) { hero_state.push((f, hs)); }
        if book_state.last() != Some(&s) {
            book_state.push(s);
        }
    }
    let g = rt.particles.lock().unwrap();
    let log = &g.sub.log;
    let book_anims: Vec<_> = log.anim_play.iter().filter(|a| a.0 == BOOK).collect();
    let book_sounds: Vec<_> = log.sound_play.iter().filter(|s| s.0 == BOOK).collect();
    let book_notes: Vec<_> = log.notifications.iter().filter(|n| n.1 == BOOK).collect();
    eprintln!("DBG hero states {hero_state:x?}");
    eprintln!("grabbed {grabbed_at:?} book states {book_state:?}");
    eprintln!("book anims {book_anims:x?}\nbook sounds {book_sounds:x?}\nbook notes {book_notes:x?}");
    eprintln!("hero anims {:x?}", log.anim_play.iter().filter(|a| a.0 == SHAGGY).collect::<Vec<_>>());
    // the book went BASICMOVE (1) -> ISOPENING (2) -> OPEN (3)
    assert!(book_state.contains(&2), "book never reached ISOPENING");
    assert!(book_anims.iter().any(|a| OPEN.contains(&(a.1 & 0xff))), "open anim not played");
    assert!(!book_sounds.is_empty(), "no book sound event");
    // LIBAnimMngr::PlayAnim registers anim-end as notify type 1 (APIgameobj::RegisterNotify(SELF, anim, 1, cb));
    // EFAnimNode::Notify code 1 = last frame reached. (BOOK_MECHANICS' "0x11" is not the notify type.)
    assert!(book_notes.iter().any(|n| n.0 == 1), "no anim-end notification for the book");
    assert!(book_state.contains(&3), "TomeDoomCB_ANIM_LF did not move the book to OPEN");
}

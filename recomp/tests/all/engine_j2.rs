//! WP-J2: runtime integration part 2. EP1L01 runs 300 frames with the merged anim / physics /
//! audio / camera / light / input subsystems behind the logic APIs, with a scripted pad (left
//! stick forward, then R1 = Tome of Doom).
use scooby_recomp::engine::input::{pad, PadState};
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::physic::api as papi;
use scooby_recomp::engine::runtime::*;

use crate::common::ISO;
const SHAGGY: u32 = 0x2a00_0091;
const BOOK: u32 = 0x2a00_0093;

#[test]
fn ep1l01_hero_book_300_frames() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = EngineRuntime::new_with_engine();
    let res = EngineRuntime::ReadLevel(&fs, "EP1/L01", "EP1L01.P2R").unwrap();
    rt.LoadLevel(&mut e, res);
    assert!(rt.gfx.cameras.count > 0, "game camera created");
    {
        let g = rt.particles.lock().unwrap();
        assert!(g.sub.audio.banks.iter().flatten().count() >= 3, "GLALL / LOE1L1 / STE1L1 banks");
        let m = g.sub.anim.get(&SHAGGY).expect("hero EFAnimationMgr");
        assert_eq!(m.base, 1, "anim id = OBJECT_DESCRIPTORS index");
        assert!(g.sub.anim.contains_key(&BOOK), "book EFAnimationMgr");
    }
    let start = rt.particles.lock().unwrap().world.frames.clone();
    let mut hero_anims = [0usize; 2];
    for f in 0..300u32 {
        {
            let mut g = rt.particles.lock().unwrap();
            let mut p = PadState { connected: true, mode: 7, ..Default::default() };
            if f < 150 {
                p.ly = 0;
            } else {
                p.buttons = pad::R1;
            }
            g.devices.SetPadState(0, p);
        }
        rt.Frame(&mut e, 1.0 / 60.0);
        if f == 0 {
            hero_anims[0] = rt.particles.lock().unwrap().sub.log.anim_play.iter().filter(|a| a.0 == SHAGGY).count();
        }
    }
    let g = rt.particles.lock().unwrap();
    let log = &g.sub.log;
    hero_anims[1] = log.anim_play.iter().filter(|a| a.0 == SHAGGY).count();
    let hero_phys = g.sub.physic.PhysicOf(SHAGGY);
    let hero = g.sub.physic.Physic(hero_phys).unwrap();
    let moved = start[&SHAGGY].pos != g.world.frames[&SHAGGY].pos;
    eprintln!("hero anims {hero_anims:?} vel {:?} ctx {:x} moved {moved}", hero.velocity, hero.context);
    eprintln!("anim_play {} sound_play {:x?} physics {} dm {} notes {}", log.anim_play.len(), log.sound_play, log.physic_create.len(), log.dm_add.len(), log.notifications.len());
    eprintln!("book anims {} book sounds {}", log.anim_play.iter().filter(|a| a.0 == BOOK).count(), log.sound_play.iter().filter(|s| s.0 == BOOK).count());
    // SELF resolution: the hero / book create their physics and DMs on themselves
    assert!(log.physic_create.iter().any(|p| p.0 == SHAGGY && p.2 != 0));
    assert!(log.physic_create.iter().any(|p| p.0 == BOOK && p.2 != 0));
    assert!(log.dm_add.iter().any(|d| d.0 == SHAGGY && d.2 != 0));
    // context flags (APIgameobj_SetContextFlag) activate the hero DMs: gravity integrates
    // (J3: with EFCollisionSystem the hero now stands on the floor, so no free-fall velocity)
    assert!(hero.context != 0, "hero physics not driven");
    assert!(log.anim_play.iter().filter(|a| a.0 == SHAGGY).all(|a| a.3), "hero anims start");
    // the pad reaches the hero logic through the EFInputContext callbacks
    assert!(hero_anims[1] > hero_anims[0], "input callbacks changed no hero anim");
    assert!(log.sound_play.iter().any(|s| s.2 == 0), "no sound event triggered");
}

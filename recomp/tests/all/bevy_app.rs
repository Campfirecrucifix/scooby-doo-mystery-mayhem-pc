//! The engine inside a Bevy `App`: frames run through the system sets, timers advance with the
//! filtered frame time and notify, objects get entities, and KillObject removes both.
use bevy_app::App;
use scooby_recomp::engine::{APIcons, APIextra, APIgameobj, Engine, Host, NULL};
use scooby_recomp::logic::{LResult, ModuleDef};
use scooby_recomp::libs;
use scooby_recomp::modules::tomedoom;
use scooby_recomp::plugin::{GameObjectId, ScoobyEnginePlugin};
use std::sync::atomic::{AtomicU32, Ordering};

struct NullHost;
impl Host for NullHost {}

static TICKS: AtomicU32 = AtomicU32::new(0);
static TIMER: AtomicU32 = AtomicU32::new(0);
fn tick(e: &mut Engine, _: u32) -> LResult<u64> {
    TICKS.fetch_add(1, Ordering::SeqCst);
    if TICKS.load(Ordering::SeqCst) == 3 {
        APIgameobj::KillObject(e, 0x2a00_0000); // itself
    }
    Ok(0) // 0 = stay registered
}
/// A logic object with a 0.1 s timer calling `tick`
fn ticker_ctor(e: &mut Engine, _: u32, _: i32) -> LResult<()> {
    APIcons::SetExternalFunction(e, 0, tick);
    let t = APIextra::TimerCreate(e);
    APIextra::TimerSetFrequency(e, t, 0.1);
    APIextra::TimerRegisterNotify(e, 0x2a00_0000, t, 0);
    TIMER.store(t, Ordering::SeqCst);
    Ok(())
}
/// as the game's modules do: the timer is unregistered and destroyed with the object (deleting an
/// object leaves its timers' notifications behind; firing one at a deleted object crashes the original)
fn ticker_dtor(e: &mut Engine, _: i32) -> LResult<()> {
    let t = TIMER.load(Ordering::SeqCst);
    APIextra::TimerUnregisterNotify(e, 0x2a00_0000, t);
    APIextra::TimerDestroy(e, t);
    Ok(())
}
static TICKER: ModuleDef =
    ModuleDef { name: "ticker", data_size: 0x10, ctor: ticker_ctor, dtor: ticker_dtor, register: None, init: &[], build_init: None };

fn ids(app: &mut App) -> Vec<u32> {
    let mut q = app.world_mut().query::<&GameObjectId>();
    let mut v: Vec<u32> = q.iter(app.world()).map(|g| g.0).collect();
    v.sort();
    v
}

#[test]
fn timers_notify_and_kill_through_the_frame_loop() {
    // WP-X4: misc_func's preCalcSin is read from the disc; register it (as smoke does) instead of relying on test order
    scooby_recomp::engine::gamedata::set_iso_path(std::path::Path::new(crate::common::ISO));
    let mut app = App::new();
    app.add_plugins(ScoobyEnginePlugin::new(Box::new(NullHost)));
    {
        let mut e = app.world_mut().resource_mut::<Engine>();
        // the level's libraries are loaded and constructed first (EFLogicCodeLib::LoadFile)
        let b = libs::api_c_functions::build_index("a1d1e457").unwrap();
        for (def, b, id) in [
            (&libs::anim_mngr::MODULE, 0, 0x2a00_1001),
            (&libs::misc_func::MODULE, 0, 0x2a00_1002),
            (&libs::api_c_functions::MODULE, b, 0x2a00_1003),
            (&libs::api_text::MODULE, 0, 0x2a00_1004),
        ] {
            let c = e.load_module(def, b);
            let h = e.add_object(id, Some(c));
            e.run_object(h, 1.0 / 60.0);
        }
        let c = e.load_module(&TICKER, 0);
        e.add_object(0x2a00_0500, Some(c));
        let b = tomedoom::build_index("802a33e5").unwrap();
        let c = e.load_module(&tomedoom::MODULE, b);
        e.add_object(0x2a00_00f0, Some(c));
    }
    app.update();
    assert_eq!(ids(&mut app), vec![0x2a00_00f0, 0x2a00_0500, 0x2a00_1001, 0x2a00_1002, 0x2a00_1003, 0x2a00_1004]);
    // without a Time resource every frame is the filter's minimum (0.0001 s, averaged over 10)
    // so drive the clock: 0.05 s frames (the filter's maximum) -> a tick every 2 frames
    for _ in 0..10 {
        app.world_mut().resource_mut::<Engine>().clock.prev = [0.05; 10];
        app.update();
    }
    assert_eq!(TICKS.load(Ordering::SeqCst), 3, "killed after the third tick");
    let e = app.world().resource::<Engine>();
    assert!(e.rt.map.find(0x2a00_0500).is_none());
    assert_eq!(ids(&mut app), vec![0x2a00_00f0, 0x2a00_1001, 0x2a00_1002, 0x2a00_1003, 0x2a00_1004]);
    let _ = NULL;
}

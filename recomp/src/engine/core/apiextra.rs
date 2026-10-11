//! APIextra game timers (EXE.1 EFApi_Extra.cpp:20-180, EFGameLogicMgr timers). A timer belongs to
//! the object whose logic created it, accumulates frame time while that object is not frozen,
//! and with a period > 0 raises notification 7 (payload = timer id) every period.
//! Only TimerPause checks the id; the others dereference an unknown timer (the port panics).
use crate::engine::{Engine, NULL};
use crate::logic::notify::{LogicCallback, NOTIFY_TIMER};

/// +0x00
pub fn TimerReset(e: &mut Engine, a0: u32) -> u32 {
    e.GetTimer(a0).elapsed = 0.0;
    0
}
/// +0x04: seconds accumulated since creation / the last reset or notification
pub fn TimerGetTime(e: &mut Engine, a0: u32) -> f32 {
    e.GetTimer(a0).elapsed
}
/// +0x08: new timer owned by the running object; returns its id
pub fn TimerCreate(e: &mut Engine) -> u32 {
    e.rt.timer_count += 1;
    let owner = e.rt.current_object();
    let id = e.CreateTimer();
    let owner_id = owner.map_or(0, |h| e.rt.obj(h).id);
    let t = e.GetTimer(id);
    t.owner = owner;
    t.owner_id = owner_id;
    id
}
/// +0x0c
pub fn TimerDestroy(e: &mut Engine, a0: u32) -> u32 {
    e.rt.timer_count -= 1;
    e.DestroyTimer(a0);
    0
}
/// +0x10: call callback `a2` of object `a0` every period of timer `a1`
pub fn TimerRegisterNotify(e: &mut Engine, a0: u32, a1: u32, a2: u32) -> u32 {
    // a missing object is stored as NULL in the original and crashes when the timer fires
    let obj = e.get_game_object(a0, NULL).unwrap_or(usize::MAX);
    let t = e.GetTimer(a1);
    let (key, owner) = (t.owner_id, t.id);
    e.NotificationMgr_RegisterNotify(NOTIFY_TIMER, key, owner, LogicCallback { obj, cb: a2 });
    0
}
/// +0x14
pub fn TimerUnregisterNotify(e: &mut Engine, a0: u32, a1: u32) -> u32 {
    let h = e.get_game_object(a0, NULL).expect("TimerUnregisterNotify: no object");
    let key = e.rt.obj(h).id;
    let owner = e.GetTimer(a1).id;
    e.NotificationMgr_UnregisterNotify(NOTIFY_TIMER, key, owner);
    0
}
/// +0x18
pub fn TimerPause(e: &mut Engine, a0: u32, a1: u32) -> u32 {
    if let Some(t) = e.rt.timers.get_mut(a0) {
        t.paused = (a1 != 0) as u32;
    }
    0
}
/// +0x1c
pub fn TimerSetFrequency(e: &mut Engine, a0: u32, a1: f32) -> u32 {
    e.GetTimer(a0).period = a1;
    0
}
/// +0x20: the period (float bits)
pub fn TimerGetFrequency(e: &mut Engine, a0: u32) -> u32 {
    e.GetTimer(a0).period.to_bits()
}

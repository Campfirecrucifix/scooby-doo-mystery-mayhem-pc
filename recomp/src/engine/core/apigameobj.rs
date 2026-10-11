//! APIgameobj slots that belong to the logic runtime (EXE.1 EFApi_GameObj.cpp).
use crate::engine::{Engine, Ptr, NULL};
use crate::logic::notify::LogicCallback;
use crate::logic::{Jump, LResult};

/// slot +0x0: change the RUNNING instance's state and abort the running logic function (longjmp).
pub fn ChangeState(e: &mut Engine, a0: u32) -> LResult<u32> {
    let h = e.rt.current_instance().expect("ChangeState outside logic");
    e.rt.inst(h).state = a0;
    Err(Jump)
}
/// slot +0x4
pub fn SetNextState(e: &mut Engine, a0: u32, a1: u32) -> u32 {
    if let Some(h) = e.get_game_object(a0, NULL) {
        e.rt.inst(h).state = a1;
    }
    0
}
/// slot +0xc
pub fn Sleep(e: &mut Engine, a0: u32, a1: f32) -> u32 {
    if let Some(h) = e.get_game_object(a0, NULL) {
        e.rt.inst(h).sleep = a1;
    }
    0
}
/// slot +0x10 (EFLogicInstance::Break: longjmp without changing state)
pub fn Break(e: &mut Engine) -> LResult<u32> {
    Err(Jump)
}
/// slot +0x18: launch a copy of a latent (template) object; returns the new object's id, 0 if unknown
pub fn LaunchLatant(e: &mut Engine, a0: u32) -> u32 {
    let Some(&code) = e.rt.latents.get(&a0) else { return 0 };
    let id = e.rt.latent_next_id;
    e.rt.latent_next_id += 1;
    let h = e.add_object(id, code);
    e.host.object_launched(&mut e.mem, a0, id);
    e.LaunchLogicNow(h);
    id
}
/// slot +0x28: shared game buffer by id (refcounted, shared by every module)
pub fn GetGameBuffer(e: &mut Engine, a0: u32, a1: u32, a2: u32) -> Ptr {
    e.GetGameBufferMgr(a0, a1, a2)
}
/// slot +0x38: current state of an object's logic, 0xffffffff if there is no such object
pub fn GetCurrentStateID(e: &mut Engine, a0: u32) -> u32 {
    match e.get_game_object(a0, NULL) {
        Some(h) => e.rt.obj(h).inst.state,
        None => 0xffff_ffff,
    }
}
/// slot +0x3c
pub fn SendMessageTo(e: &mut Engine, a0: u32, a1: Ptr, a2: u32, a3: u32, a4: Ptr) -> u32 {
    // debug: GAME_MSGLOG=1 logs every logic message (target, msg, param)
    if std::env::var_os("GAME_MSGLOG").is_some() && a2 != 0x1d {
        eprintln!("msg: -> {a0:#x} msg {a2:#x} p {a3:#x}");
    }
    e.SendLogicMessageTo(a0, a2, a3, a4, a1) as u32
}
/// slot +0x40: id of the object (0x7ffffffe if none)
pub fn GetID(e: &mut Engine, a0: u32, a1: Ptr) -> u32 {
    match e.get_game_object(a0, a1) {
        Some(h) => e.rt.obj(h).id,
        None => 0x7fff_fffe,
    }
}

/// KillObject: queue the object for deletion at the end of the frame (EFWorld kill list; once)
pub fn KillObject(e: &mut Engine, a0: u32) -> u32 {
    if let Some(h) = e.get_game_object(a0, NULL) {
        if !e.rt.kill_list.contains(&h) {
            e.rt.kill_list.push(h);
        }
    }
    0
}
/// RegisterNotify(obj, sender, type, cb): call callback `cb` of `obj` on notification `type`.
/// sender -1 = the object's own events (key = owner = obj id), else events of sub-instance `sender`.
pub fn RegisterNotify(e: &mut Engine, a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
    if let Some(h) = e.get_game_object(a0, NULL) {
        let id = e.rt.obj(h).id;
        let owner = if a1 == 0xffff_ffff { id } else { a1 };
        e.NotificationMgr_RegisterNotify(a2 & 0xff, id, owner, LogicCallback { obj: h, cb: a3 });
        if a2 == 0x11 {
            e.rt.obj_mut(h).notify11 = 1;
        }
    }
    0
}
/// UnregisterNotify(obj, sender, type). Like the original it also sets +0xbc for type 0x11.
pub fn UnregisterNotify(e: &mut Engine, a0: u32, a1: u32, a2: u32) -> u32 {
    if let Some(h) = e.get_game_object(a0, NULL) {
        let id = e.rt.obj(h).id;
        let owner = if a1 == 0xffff_ffff { id } else { a1 };
        e.NotificationMgr_UnregisterNotify(a2 & 0xff, id, owner);
        if a2 == 0x11 {
            e.rt.obj_mut(h).notify11 = 1;
        }
    }
    0
}
/// UnregisterAllNotify(obj)
pub fn UnregisterAllNotify(e: &mut Engine, a0: u32) -> u32 {
    if let Some(h) = e.get_game_object(a0, NULL) {
        let id = e.rt.obj(h).id;
        e.NotificationMgr_UnregisterAllOwnerNotify(id);
    }
    0
}

//! The per-frame part of the engine that drives logic: frame-time filter (EFEngineController),
//! game timers and the logic update (EFGameLogicMgr::Update), and the kill list
//! (EFWorld::Update). EXE.1 Classes/EF/EFEngineController.cpp, EFGameLogicMgr.cpp, EFWorld.cpp,
//! Engine API/EFApi_Extra.cpp.
//!
//! Frame order of the original main loop, which [`crate::plugin`] reproduces with Bevy system sets:
//! 1. ComputeDeltaTime ([`FrameClock::ComputeDeltaTime`]), input (platform: raises input notifications)
//! 2. [`Engine::GameLogicUpdate`]: NotifyAll(0x14), timers, every object's logic, NotifyAll(0x15)
//! 3. cut-scene player (not ported)
//! 4. EFWorld::Update: animation (platform), [`Engine::FlushNotification`], physics and particles
//!    (platform), [`Engine::ProcessKillList`], PrepareRender (platform clipping, then
//!    [`Engine::PrepareRender_Logic`])
//! 5. render, audio (platform)
use super::notify::{LogicCallback, NOTIFY_POST_LOGIC, NOTIFY_PRE_LOGIC, NOTIFY_TIMER};
use super::world::DTMap;
use super::ObjH;
use crate::engine::Engine;

/// EFEngineController's frame-time filter: the average of the last 10 frame times, each clamped
/// to [0.0001, 0.05] s (so logic never sees a step longer than 1/20 s).
#[derive(Clone, Debug)]
pub struct FrameClock {
    /// PrevFrameTimes (0x2ad630, initialised to 0.02 each)
    pub prev: [f32; 10],
    pub cur: usize,
    /// controller +0x2a4: sum of the filtered frame times
    pub total: f32,
    /// controller +0x2b8: this frame's filtered time (APIgame::GetFrameDeltaTime)
    pub dt: f32,
}

impl Default for FrameClock {
    fn default() -> Self {
        FrameClock { prev: [0.02; 10], cur: 0, total: 0.0, dt: 0.0 }
    }
}

impl FrameClock {
    /// EFEngineController::ComputeDeltaTime. The original's optional video-sync correction
    /// (controller flag 0x20) is a display detail and is not reproduced.
    pub fn ComputeDeltaTime(&mut self, raw: f32) -> f32 {
        let c = self.cur;
        self.prev[c] = raw;
        if 0.05 < self.prev[c] {
            self.prev[c] = 0.05;
        }
        if self.prev[c] < 0.0001 {
            self.prev[c] = 0.0001;
        }
        let mut s = 0.0f32;
        for a in self.prev {
            s += a;
        }
        let dt = s / 10.0;
        self.cur = (c + 1) % 10;
        self.total += dt;
        self.dt = dt;
        dt
    }
}

/// EFGameTimer (EFInstance, 0x20 bytes)
#[derive(Clone, Debug)]
pub struct GameTimer {
    /// +8
    pub id: u32,
    /// +0x10: the object whose logic created it (NULL for the manager's own timer). The id is
    /// remembered too: the original reads it through the pointer when the timer fires.
    pub owner: Option<ObjH>,
    pub owner_id: u32,
    /// +0x14: 1 = paused
    pub paused: u32,
    /// +0x18: notification period in seconds (0 = never notifies)
    pub period: f32,
    /// +0x1c: seconds since the last notification / reset
    pub elapsed: f32,
}

impl Engine {
    // ------------------------------------------------------------------ timers (EFGameLogicMgr)
    /// EFGameLogicMgr::CreateTimer: returns the new id
    pub fn CreateTimer(&mut self) -> u32 {
        let id = self.rt.next_timer_id;
        let t = GameTimer { id, owner: None, owner_id: 0, paused: 0, period: 0.0, elapsed: 0.0 };
        self.rt.timers.insert(id, t);
        self.rt.next_timer_id = id.wrapping_add(1);
        id
    }
    /// EFGameLogicMgr::DestroyTimer
    pub fn DestroyTimer(&mut self, id: u32) {
        self.rt.timers.remove(id);
    }
    /// EFGameLogicMgr::GetTimer. The API functions dereference the result without a check (except
    /// TimerPause), so an unknown id is a crash in the original; the port panics.
    #[track_caller]
    pub fn GetTimer(&mut self, id: u32) -> &mut GameTimer {
        self.rt.timers.get_mut(id).unwrap_or_else(|| panic!("no game timer {id:#x}"))
    }

    // ------------------------------------------------------------------ EFGameLogicMgr::Update
    /// EFGameLogicMgr::Update(dt)
    pub fn GameLogicUpdate(&mut self, dt: f32) {
        // EFEngineController frame: EFLoadSaveMgr_BC::Update runs before the world/logic update
        crate::engine::core::apiloadsave::Update(self);
        self.rt.logic_dt = dt;
        self.NotifyAll(NOTIFY_PRE_LOGIC, 0);
        // timers, in map order. As in NotifyAll the successor is read after the notification
        // (a callback may destroy timers); see there.
        let mut r = self.rt.timers.first();
        while r != 0 {
            let t = self.rt.timers.val_ref(r).clone();
            let before = self.rt.timers.chain_next(r);
            let frozen = match t.owner.and_then(|h| self.rt.objects[h].as_ref()) {
                // the original reads the flags of a dead owner from freed memory; treated as clear
                None => false,
                Some(o) => o.flags138 & 0x200 != 0 || o.logic_flags & 0x40 != 0,
            };
            if !frozen && t.paused != 1 {
                let tm = self.rt.timers.val_mut(r);
                tm.elapsed += dt;
                let period = tm.period;
                if 0.0 < period && period <= tm.elapsed {
                    tm.elapsed -= period;
                    // EFInstance::Notify(timer, 7, timer id): key = owner object id, owner = timer id
                    self.Notify(NOTIFY_TIMER, t.owner_id, t.id, t.id, false);
                }
            }
            let m = &self.rt.timers;
            let chain = if m.alive(r) && m.key(r) == t.id {
                m.chain_next(r)
            } else if m.alive(before) {
                before
            } else {
                0
            };
            r = if chain != 0 { chain } else { m.first_after_bucket_of(t.id) };
        }
        if !self.rt.update_all && !self.rt.logic_list.is_empty() {
            // world +0x4140: the objects PrepareRender found unclipped last frame
            let list = self.rt.logic_list.clone();
            for h in list {
                if self.rt.objects[h].is_some() {
                    self.UpdateLogic(h);
                }
            }
        } else {
            // EFAllRetriever over the object map, twice: "update first" objects (logic flag 0x4000)
            // then the rest. The cursor holds the next node before the object runs, as the retriever does.
            for first in [true, false] {
                let mut cur = self.rt.map.first();
                while cur != 0 {
                    let h = self.rt.map.val(cur);
                    cur = self.rt.map.next(cur);
                    let Some(o) = self.rt.objects[h].as_ref() else { continue };
                    if (o.logic_flags & 0x4000 != 0) == first {
                        self.UpdateLogic(h);
                    }
                    if cur != 0 && !self.rt.map.alive(cur) {
                        // the original would continue from a freed node
                        break;
                    }
                }
            }
            self.rt.update_all = false;
        }
        self.NotifyAll(NOTIFY_POST_LOGIC, 0);
    }

    /// EFGameLogicMgr::UpdateLogic
    pub fn UpdateLogic(&mut self, h: ObjH) {
        if self.rt.obj(h).flags & 8 == 0 {
            let t = self.rt.stack.top;
            self.rt.stack.objects[t] = Some(h);
            let dt = self.rt.logic_dt;
            self.Run(h, dt);
        } else {
            self.DestroyObject(h);
        }
    }

    /// EFGameLogicMgr::DestroyObject: objects flagged 8 are deleted on their next update, unless they
    /// have physics (then nothing happens; the object just stays inert).
    pub fn DestroyObject(&mut self, h: ObjH) {
        let id = self.rt.obj(h).id;
        if !self.host.object_has_physic(id) {
            self.RemoveGameObject(id);
            self.Terminate(h);
            self.delete_object(h);
        }
    }

    // ------------------------------------------------------------------ EFWorld
    /// EFWorld::RemoveGameObject: out of the id map and the world's object lists
    pub fn RemoveGameObject(&mut self, id: u32) {
        if let Some(h) = self.rt.map.remove(id) {
            self.rt.logic_list.retain(|&x| x != h);
        }
    }

    /// EFGameObject::Terminate: the logic destructor (always, constructed or not), then the
    /// platform's part (attached emitters, scene instances, shadow caster, projector receiver).
    pub fn Terminate(&mut self, h: ObjH) {
        let t = self.rt.stack.top;
        self.rt.stack.objects[t] = Some(h);
        self.Destructor(h);
        let id = self.rt.obj(h).id;
        self.host.object_terminated(&mut self.mem, id);
    }

    /// `delete` of an EFGameObject (~EFGameObject: UnregisterAllOwnerNotify(id))
    pub fn delete_object(&mut self, h: ObjH) {
        let id = self.rt.obj(h).id;
        self.NotificationMgr_UnregisterAllOwnerNotify(id);
        // ~EFLogicInstance frees the local data
        let local = self.rt.obj(h).inst.local_alloc;
        self.mem.free(local);
        self.rt.objects[h] = None;
        self.rt.kill_list.retain(|&x| x != h);
        self.rt.events.push(super::WorldEvent::Deleted { id, handle: h });
    }

    /// The kill-list part of EFWorld::Update: objects queued by APIgameobj::KillObject, newest
    /// first, including ones queued while this runs.
    pub fn ProcessKillList(&mut self) {
        while let Some(h) = self.rt.kill_list.pop() {
            let id = self.rt.obj(h).id;
            self.host.destroy_physic(&mut self.mem, id);
            self.RemoveGameObject(id);
            self.Terminate(h);
            self.delete_object(h);
        }
        self.rt.world_frame = self.rt.world_frame.wrapping_add(1);
    }

    /// The logic part of EFWorld::PrepareRender: with cameras, the platform passes the objects whose
    /// clip flags lack 0x400 (oldest first); they are the only ones updated next frame. Without
    /// cameras (`None`) the list is left as it was. Then the deferred notifications are flushed.
    pub fn PrepareRender_Logic(&mut self, unclipped: Option<&[u32]>) {
        if let Some(ids) = unclipped {
            self.rt.logic_list = ids.iter().filter_map(|&id| self.rt.map.find(id)).collect();
        }
        self.FlushNotification();
    }

    /// EFGameLogicMgr::LaunchLogicNow: constructs the logic now (it runs its first state next update)
    pub fn LaunchLogicNow(&mut self, h: ObjH) {
        let t = self.rt.stack.top;
        self.rt.stack.objects[t] = Some(h);
        self.Constructor(h);
    }

    /// Registration helper used by the notification APIs (the 8-byte {object, callback} block)
    pub fn logic_callback(&self, h: ObjH, cb: u32) -> LogicCallback {
        LogicCallback { obj: h, cb }
    }
}

/// DTMap<unsigned int, EFGameTimer*> at EFGameLogicMgr + 0x250
pub type TimerMap = DTMap<GameTimer>;

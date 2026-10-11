//! EFNotificationMgr (EXE.1 Classes/EF/EFNotificationMgr.cpp, at EFGameLogicMgr + 0x14).
//!
//! 0x1b notification types, one DTMap(0x25) per type: key -> list of registrations, newest first.
//! A registration is (owner, fn, data); every registration the logic APIs make uses fn =
//! ExecLogicCallback (0x1fd300) and data = {EFGameObject *, callback index}, so the port stores
//! that pair directly ([`LogicCallback`]).
//!
//! Keys/owners: for object events key = owner = the object id; for sub-instances (timers, animation
//! nodes) key = the owning object's id and owner = the sub-instance id (EFInstance::Notify).
//!
//! Payloads are [`Ptr`]s into [`Mem`](crate::engine::Mem) (or plain values, e.g. a timer id). A deferred
//! notification owns its payload: FlushNotification frees it unless the type is 0 or 1, so callers
//! of [`Engine::Notify`] with `deferred = true` must pass a `mem.malloc`'d block (or NULL).
//!
//! Known types (from the callers in EXE.1 and the modules): 7 timer, 0x14 before logic, 0x15 after
//! logic. The others (input, animation, collision, cutscene ...) are raised by the platform layer
//! with the same calls.
use super::world::DTMap;
use super::ObjH;
use crate::engine::{Engine, Ptr};

pub const NOTIFY_TYPES: usize = 0x1b;
/// EFNotifyType 7: periodic game timer (payload = timer id)
pub const NOTIFY_TIMER: u32 = 7;
/// EFNotifyType 0x14: start of EFGameLogicMgr::Update (NotifyAll, payload 0)
pub const NOTIFY_PRE_LOGIC: u32 = 0x14;
/// EFNotifyType 0x15: end of EFGameLogicMgr::Update (NotifyAll, payload 0)
pub const NOTIFY_POST_LOGIC: u32 = 0x15;

/// The 8-byte block the logic APIs allocate for ExecLogicCallback: {EFGameObject *obj, int cb}.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogicCallback {
    pub obj: ObjH,
    pub cb: u32,
}

/// EFNotificationData (pooled, 0x14 bytes): owner, fn, data, next
#[derive(Clone, Copy, Debug)]
pub struct NotifyNode {
    pub owner: u32,
    pub data: LogicCallback,
}

/// EFNotification (deferred queue entry, 0x18 bytes)
#[derive(Clone, Copy, Debug)]
pub struct Deferred {
    pub ty: u8,
    pub key: u32,
    pub owner: u32,
    pub payload: Ptr,
}

pub struct NotificationMgr {
    /// one map per type; the Vec is the registration list, index 0 = list head
    pub maps: Vec<DTMap<Vec<NotifyNode>>>,
    /// +0x1f0/+0x1f4: deferred notifications, FIFO
    pub queue: Vec<Deferred>,
}

impl Default for NotificationMgr {
    fn default() -> Self {
        NotificationMgr { maps: (0..NOTIFY_TYPES).map(|_| DTMap::new(0x25)).collect(), queue: Vec::new() }
    }
}

impl Engine {
    /// ExecLogicCallback (0x1fd300): puts the object in the next logic-stack frame and calls the
    /// callback; the result passes through (sign-extended int).
    pub fn ExecLogicCallback(&mut self, data: LogicCallback, payload: Ptr) -> i64 {
        let t = self.rt.stack.top;
        self.rt.stack.objects[t] = Some(data.obj);
        self.CallCB(data.obj, data.cb, payload) as i32 as i64
    }

    /// EFNotificationMgr::RegisterNotify. Re-registering the same (type, key, owner) replaces the
    /// callback in place (keeping its list position); a new owner is prepended.
    pub fn NotificationMgr_RegisterNotify(&mut self, ty: u32, key: u32, owner: u32, data: LogicCallback) {
        let map = &mut self.rt.notify.maps[(ty & 0xff) as usize];
        let node = NotifyNode { owner, data };
        match map.get_mut(key) {
            None => map.insert(key, vec![node]),
            Some(list) => match list.iter_mut().find(|n| n.owner == owner) {
                Some(n) => n.data = data,
                None => list.insert(0, node),
            },
        }
    }

    /// EFNotificationMgr::UnregisterNotify: removes the first registration of `owner`; the key goes
    /// when its list is empty.
    pub fn NotificationMgr_UnregisterNotify(&mut self, ty: u32, key: u32, owner: u32) {
        let map = &mut self.rt.notify.maps[(ty & 0xff) as usize];
        let Some(list) = map.get_mut(key) else { return };
        if let Some(i) = list.iter().position(|n| n.owner == owner) {
            list.remove(i);
            if list.is_empty() {
                map.remove(key);
            }
        }
    }

    /// EFNotificationMgr::UnregisterAllOwnerNotify: drops `key` and all its registrations in every type
    pub fn NotificationMgr_UnregisterAllOwnerNotify(&mut self, key: u32) {
        for map in self.rt.notify.maps.iter_mut() {
            map.remove(key);
        }
    }

    /// EFNotificationMgr::Notify. Immediate: calls the first registration of `owner` under `key`;
    /// a result >= 1 unregisters it. Deferred: queued until the next FlushNotification.
    pub fn Notify(&mut self, ty: u32, key: u32, owner: u32, payload: Ptr, deferred: bool) {
        if !deferred {
            let map = &self.rt.notify.maps[(ty & 0xff) as usize];
            let Some(list) = map.get(key) else { return };
            let Some(n) = list.iter().find(|n| n.owner == owner).copied() else { return };
            let r = self.ExecLogicCallback(n.data, payload);
            if r < 1 {
                return;
            }
            self.NotificationMgr_UnregisterNotify(ty, key, owner);
        } else {
            self.rt.notify.queue.push(Deferred { ty: ty as u8, key, owner, payload });
        }
    }

    /// EFNotificationMgr::NotifyAll: calls the HEAD registration of every key, in map order; a
    /// result > 0 unregisters that registration.
    ///
    /// The original reads the next map node from the current one after the callback returned,
    /// even if the callback freed it (unregistered its last registration). The port takes the
    /// live successor when the node still exists, else the successor it had before the call.
    pub fn NotifyAll(&mut self, ty: u32, payload: Ptr) {
        let t = (ty & 0xff) as usize;
        let mut r = self.rt.notify.maps[t].first();
        while r != 0 {
            let map = &self.rt.notify.maps[t];
            let key = map.key(r);
            let head = map.val_ref(r)[0];
            let before = map.chain_next(r);
            let res = self.ExecLogicCallback(head.data, payload);
            let map = &self.rt.notify.maps[t];
            let chain = if map.alive(r) && map.key(r) == key {
                map.chain_next(r)
            } else if map.alive(before) {
                before
            } else {
                0
            };
            let next = if chain != 0 { chain } else { map.first_after_bucket_of(key) };
            if 0 < res {
                self.NotificationMgr_UnregisterNotify(ty, key, head.owner);
            }
            r = next;
        }
    }

    /// EFNotificationMgr::FlushNotification: delivers the queued notifications in order (ones
    /// queued meanwhile wait for the next flush) and frees their payloads, except for types 0 and 1.
    pub fn FlushNotification(&mut self) {
        let q = std::mem::take(&mut self.rt.notify.queue);
        for d in q {
            self.Notify(d.ty as u32, d.key, d.owner, d.payload, false);
            if d.ty != 1 && d.ty != 0 {
                self.mem.free(d.payload);
            }
        }
    }

    /// EFNotificationMgr::ClearAllNotification: drops every registration (deferred ones stay queued)
    pub fn ClearAllNotification(&mut self) {
        for map in self.rt.notify.maps.iter_mut() {
            *map = DTMap::new(0x25);
        }
    }

    /// EFInstance::Notify for a game object (EFGameObject + 0x10 = itself): key = owner = its id.
    /// What the platform layer calls to deliver input / animation / collision events to an object.
    pub fn notify_object(&mut self, ty: u32, id: u32, payload: Ptr, deferred: bool) {
        self.Notify(ty, id, id, payload, deferred);
    }
}

//! EFAnimNode / EFHAnimNode runtime (Classes/EF/EFAnimNode.cpp, EFHAnimNode.cpp).
//! Node tree (DTOperator parent/children), DTWorkMemory and EF3dObj transfo locking are replaced
//! by an owned `Vec<DTTransfo>` output per node; notifications are queued as [`AnimEvent`]s
//! (replaces EFNotificationMgr::Notify / EFNewSoundMgr::PlayAnimationSound, wired by the coordinator).
#![allow(non_snake_case)]

use super::res::{DTTransfo, EFHAnimRes};
use std::sync::Arc;

/// EFAnimNode state bits (this+0x34).
pub mod state {
    pub const PLAY_ONCE: u32 = 0x1; // clamp at the end
    pub const LOOP: u32 = 0x2;
    pub const FROZEN: u32 = 0x4;
    pub const FORCE_UPDATE: u32 = 0x100_0000;
    pub const END_REACHED: u32 = 0x200_0000; // notify 1 pending
    pub const STOPPED: u32 = 0x800_0000;
    pub const FINISHED: u32 = 0x2000_0000;
    pub const WRAPPED: u32 = 0x4000_0000;
}

/// EFChangeType_e for ChangeAnimState
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EFChangeType {
    Set = 0,
    Clear = 1,
    Assign = 2,
}

/// Queued EFAnimNode::Notify / sound callback.
#[derive(Debug, Clone, PartialEq)]
pub enum AnimEvent {
    /// Notify(this, code, arg): 0 = stopped, 1 = end/loop reached. `user` = this+0x20 (the
    /// notification owner), `arg` = the payload (this+0x20, or 0 for the callback-wrap notify).
    Notify { code: u32, user: u32, arg: u32 },
    /// Notify 0xb with {user, cb type (+4), cb index (+8)}
    Callback { user: u32, kind: u8, index: u32 },
    /// EFNewSoundMgr::PlayAnimationSound(sound_table[index])
    Sound { sound_id: u32 },
}

/// bone id -> parent joint index lookup (see [`EFHAnimNode::parent_joint`])
#[derive(Clone)]
pub struct ParentJointFn(pub Arc<dyn Fn(u32) -> Option<u32> + Send + Sync>);
impl std::fmt::Debug for ParentJointFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ParentJointFn")
    }
}

/// EFHAnimNode (EFAnimNode base fields + hanim part).
#[derive(Debug, Clone)]
pub struct EFHAnimNode {
    /// this+0x20 user value passed to notifications
    pub user: u32,
    /// this+0x24 exclusivity key = DTAnimHdr+0xd byte (EFAnimNode::Initialize 0x0022d600). P32 R3: was the
    /// channel-set count (+0xc): an additive layer anim (+0xd 1, e.g. wispsourcenew release anims 2/3/4
    /// over idle anim 1, +0xd 0) then REPLACED the idle (FindExclNode) instead of being layered on it
    /// (FindNonExclNode -> BlendLayer additive), so its raw deltas (scale 0) collapsed the skull model
    pub nb_bones: u32,
    /// this+0x30 the resource
    pub res: Arc<EFHAnimRes>,
    /// this+0x34
    pub state: u32,
    /// this+0x38 current time
    pub time: f32,
    /// this+0x3c duration
    pub duration: f32,
    /// this+0x40 speed
    pub speed: f32,
    /// this+0x44 next callback index
    pub cur_cb: u32,
    /// this+0x48 sound id table indexed by callback index (None = no sound table)
    pub sound_table: Option<Vec<u32>>,
    /// output transfos (replaces the DTWorkMemory/EF3dObj-locked DTTransfo array)
    pub transfos: Vec<DTTransfo>,
    pub events: Vec<AnimEvent>,
    /// this+0x2c target EF3dObj (opaque id; set by the descriptor / caller)
    pub obj: u32,
    /// parent node is an EFAnimBlendNode (DTNode parent +0x14 == 4); set by EFAnimationMgr
    pub parent_is_blend: bool,
    /// result of parent EFAnimBlendNode::CanPlaySoundCB(*this) (true when no blend parent);
    /// set by EFAnimationMgr before Update
    pub can_play_sound: bool,
    /// Visibility channel (id 9) result: EF3dObj_SP+0x2c bit 2 + SetVisible(v != 0). Replaced by
    /// this output field; the coordinator applies it to the EF3dObj.
    pub visible: Option<bool>,
    /// ScaleCompensate input (replaces RpHAnimIDGetIndex / RwFrame parent / RpHAnimFrameGetID /
    /// EF3dObj::GetJointIndex on the connected EF3dObj): channel-set bone id -> joint index of the
    /// parent frame. None = the EF3dObj has no hierarchy (obj+0x2c bit 4 clear; C++ then reads
    /// garbage, here nothing is written).
    pub parent_joint: Option<ParentJointFn>,
    /// The node writes into the EF3dObj's own transfo array (EF3dObj+0x50 == data block, i.e. a
    /// root-level node): transfos are indexed by bone id instead of channel-set index.
    pub full_array: bool,
    /// data block +0x10 -> +0x10: the EFAnimCollPrim chain built by UpdateAnimPrimColl
    pub coll: super::coll_blend::CollPrimList,
    /// what UpdateAnimPrimColl reads through the EF3dObj (+0x28 EFPhysic prims, clump records);
    /// set by EFAnimationMgr from its per-object table
    pub coll_ctx: Option<Arc<super::coll_prim::CollCtx>>,
}

impl EFHAnimNode {
    /// EFAnimNode ctor + EFHAnimNode::Initialize: state STOPPED, speed 1, duration from hdr+0x8.
    pub fn new(res: Arc<EFHAnimRes>, user: u32) -> Self {
        let (nb, excl, dur) = res.base.anim.as_ref().map_or((0, 0, 0.0), |a| (a.sets.len() as u32, a.unkd[0] as u32, a.duration));
        EFHAnimNode {
            user,
            nb_bones: excl,
            res,
            state: state::STOPPED,
            time: 0.0,
            duration: dur,
            speed: 1.0,
            cur_cb: 0,
            sound_table: None,
            transfos: vec![DTTransfo::default(); nb as usize],
            events: Vec::new(),
            obj: 0,
            parent_is_blend: false,
            can_play_sound: true,
            visible: None,
            parent_joint: None,
            full_array: false,
            coll: Default::default(),
            coll_ctx: None,
        }
    }

    /// EFHAnimNode::UpdateAnimPrimColl (0x002316d0), see coll_prim.rs.
    pub fn UpdateAnimPrimColl(&mut self) {
        let Some(cp) = self.res.coll_prim.as_ref() else { return };
        super::coll_prim::UpdateAnimPrimColl(&mut self.coll, &cp.cmds, self.time, self.duration, self.state, self.coll_ctx.as_deref());
    }

    /// EFHAnimNode::SetTime (vtable +0x2c): time + SetCurCBCmd
    pub fn SetTime(&mut self, t: f32) {
        self.time = t;
        self.SetCurCBCmd();
    }

    /// EFAnimNode::Notify
    fn Notify(&mut self, code: u32) {
        self.events.push(AnimEvent::Notify { code, user: self.user, arg: if code == 1 { self.user } else { 0 } });
    }

    /// EFAnimNode::ChangeAnimState (no children here: the recursion goes through the blend tree)
    pub fn ChangeAnimState(&mut self, bits: u32, ty: EFChangeType) {
        match ty {
            EFChangeType::Set => self.state |= bits,
            EFChangeType::Clear => self.state &= !bits,
            EFChangeType::Assign => self.state = bits,
        }
    }

    /// EFHAnimNode::Play = EFAnimNode::Play (state = flags; the float arg is ignored) + SetCurCBCmd.
    /// (the EF3dObj+0x2c |= 0x80 "animated" flag is the caller's business here)
    pub fn Play(&mut self, flags: u32, _blend: f32) {
        self.state = flags;
        self.SetCurCBCmd();
    }

    /// EFHAnimNode::Stop (tree detach done by the mgr) = EFAnimNode::Stop + this+0x44 = 0
    pub fn Stop(&mut self) {
        if self.state != state::STOPPED {
            self.state = state::STOPPED;
            self.time = 0.0;
            self.Notify(0);
        }
        self.cur_cb = 0;
    }

    pub fn cb(&self, i: u32) -> (f32, u8, u32) {
        let w = self.res.callback_words(i as usize);
        let t = w.first().map_or(0.0, |x| f32::from_bits(*x));
        let k = w.get(1).map_or(0, |x| *x as u8);
        (t, k, w.get(2).copied().unwrap_or(0))
    }

    /// EFHAnimNode::SetCurCBCmd
    pub fn SetCurCBCmd(&mut self) {
        self.cur_cb = 0;
        if self.res.callbacks.is_empty() {
            return;
        }
        while self.cur_cb < self.res.cb_count && self.cb(self.cur_cb).0 < self.time {
            self.cur_cb += 1;
        }
    }

    /// EFHAnimNode::UpdateHAnim (0x002314a0), 1:1 except the data-block plumbing:
    /// index = channel-set index, or the bone id when the block is the EF3dObj array
    /// (`full_array`); ResetEuler per element (C++: only when the block type has bit 8, which the
    /// EF3dObj type-10 block has; DTWorkMemory blocks are fresh/reset so the result is the same).
    pub fn UpdateHAnim(&mut self) {
        let res = self.res.clone();
        let Some(a) = res.base.anim.as_ref() else { return };
        if self.full_array {
            let need = a.sets.iter().map(|s| s.bone as usize + 1).max().unwrap_or(0);
            if self.transfos.len() < need {
                self.transfos.resize(need, DTTransfo::default());
            }
        }
        for (i, set) in a.sets.iter().enumerate() {
            let idx = if self.full_array { set.bone as usize } else { i };
            let Some(t) = self.transfos.get_mut(idx) else { break };
            t.ResetEuler();
            for ch in &set.channels {
                let k = (ch.id & 0xff_ffff) as usize;
                let v = ch.curve.Evaluate(self.time);
                if k < 9 {
                    t.f[k] = v;
                    t.state |= 1;
                    t.state &= !2;
                } else if k == 9 {
                    self.visible = Some(v != 0.0);
                }
            }
            t.id = set.bone as i32;
            // root flag: bone id 0 and DTAnimHdr+0xf == 1
            if set.bone == 0 && a.flag_f == 1 {
                t.state |= 8;
            }
            if set.flags & 1 == 0 {
                t.state &= !4;
            } else {
                self.ScaleCompensate(set.bone);
            }
        }
    }

    /// EFHAnimNode::ScaleCompensate: transfo[bone].v34 = 1 / scale of the parent joint; state |= 4.
    /// quirk: both the written element and the read parent are indexed by id/joint index, even in
    /// a compact (channel-set indexed) block.
    pub fn ScaleCompensate(&mut self, bone: u32) {
        let Some(pj) = self.parent_joint.as_ref().and_then(|f| (f.0)(bone)) else { return };
        let Some(sc) = self.transfos.get(pj as usize).map(|p| [p.f[0], p.f[1], p.f[2]]) else { return };
        if let Some(t) = self.transfos.get_mut(bone as usize) {
            t.v34 = [1.0 / sc[0], 1.0 / sc[1], 1.0 / sc[2]];
            t.state |= 4;
        }
    }

    /// EFHAnimNode::UpdateAnimCallback
    pub fn UpdateAnimCallback(&mut self) {
        if self.res.callbacks.is_empty() {
            return;
        }
        let count = self.res.cb_count;
        let mut wrapped = self.state & state::WRAPPED != 0;
        loop {
            let i = self.cur_cb;
            if (count <= i || self.time < self.cb(i).0) && !wrapped {
                break;
            }
            if i < count {
                let (_, kind, idx) = self.cb(i);
                match (&self.sound_table, kind) {
                    (Some(tbl), 0) if tbl.get(idx as usize).copied().unwrap_or(0) != 0 => {
                        // parent blend: EFAnimBlendNode::CanPlaySoundCB (precomputed by the mgr)
                        if !self.parent_is_blend || self.can_play_sound {
                            self.events.push(AnimEvent::Sound { sound_id: tbl[idx as usize] });
                        }
                    }
                    _ => self.events.push(AnimEvent::Callback { user: self.user, kind, index: idx }),
                }
                self.cur_cb += 1;
            }
            if self.cur_cb == count && wrapped {
                self.cur_cb = 0;
                wrapped = false;
                if self.state & state::END_REACHED != 0 {
                    // quirk (EFHAnimNode::UpdateAnimCallback): Notify(this, 1, 0): arg 0, owner still this+0x20
                    self.events.push(AnimEvent::Notify { code: 1, user: self.user, arg: 0 });
                    self.state &= !state::END_REACHED;
                }
            }
        }
    }

    /// EFHAnimNode::Update(dt). Finished nodes still re-sample under a blend parent.
    pub fn Update(&mut self, dt: f32) -> u32 {
        use state::*;
        let s = self.state;
        if s & (FINISHED | FORCE_UPDATE | FROZEN) != 0 {
            if s & FINISHED != 0 && s & FORCE_UPDATE == 0 && !self.parent_is_blend {
                return 0;
            }
            self.UpdateHAnim();
            self.ChangeAnimState(FORCE_UPDATE, EFChangeType::Clear);
            self.UpdateAnimPrimColl();
            return 0;
        }
        if s & STOPPED == 0 && self.duration > 0.0 {
            self.time += dt * self.speed;
            self.state &= !WRAPPED;
            if self.duration <= self.time {
                if self.state & LOOP != 0 {
                    while self.duration < self.time {
                        self.time -= self.duration;
                        self.state |= WRAPPED;
                    }
                } else if self.state & PLAY_ONCE != 0 {
                    self.time = self.duration;
                    self.state |= END_REACHED;
                }
            } else if self.time <= 0.0 {
                if self.state & LOOP != 0 {
                    while self.time < 0.0 {
                        self.time += self.duration;
                        self.state |= WRAPPED;
                    }
                } else if self.state & PLAY_ONCE != 0 {
                    self.time = 0.0;
                    self.state |= END_REACHED;
                }
            }
            if self.state & WRAPPED != 0 {
                self.state |= END_REACHED;
            }
        }
        self.UpdateHAnim();
        self.UpdateAnimPrimColl();
        self.UpdateAnimCallback();
        if self.state & PLAY_ONCE != 0
            && ((self.speed < 0.0 && self.time == 0.0) || (0.0 < self.speed && self.time == self.duration))
        {
            self.state |= FINISHED;
        }
        if self.state & END_REACHED != 0 {
            self.Notify(1);
            self.state &= !END_REACHED;
        }
        0
    }

    /// EFAnimationMgr::Evaluate time-only path (mgr this+0x54 == 0x40): advance, no sampling.
    pub fn AdvanceTime(&mut self, dt: f32) {
        use state::*;
        if self.state & STOPPED == 0 && 0.0 < self.duration {
            self.time += dt * self.speed;
            self.state &= !WRAPPED;
            if self.duration <= self.time {
                if self.state & LOOP != 0 {
                    while self.duration < self.time {
                        self.time -= self.duration;
                        self.state |= WRAPPED;
                    }
                } else if self.state & PLAY_ONCE != 0 {
                    self.time = self.duration;
                    self.state |= END_REACHED;
                }
            } else if self.time <= 0.0 {
                if self.state & LOOP != 0 {
                    while self.time < 0.0 {
                        self.time += self.duration;
                        self.state |= WRAPPED;
                    }
                } else if self.state & PLAY_ONCE != 0 {
                    self.time = 0.0;
                    self.state |= END_REACHED;
                }
            }
            if self.state & WRAPPED != 0 {
                self.state |= END_REACHED;
            }
        }
        if self.state & PLAY_ONCE != 0
            && ((self.speed < 0.0 && self.time == 0.0) || (0.0 < self.speed && self.time == self.duration))
        {
            self.state |= FINISHED;
        }
        if self.state & END_REACHED != 0 {
            self.Notify(1);
            self.state &= !END_REACHED;
        }
        if self.state & FINISHED != 0 {
            self.state |= FORCE_UPDATE;
        }
    }

    /// Per-bone (rotation quat xyzw, translation) at the current time.
    pub fn sample(&self) -> Vec<([f32; 4], [f32; 3])> {
        self.transfos.iter().map(|t| (t.rotation_quat(), t.translation())).collect()
    }
}

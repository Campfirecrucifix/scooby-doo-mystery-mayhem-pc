//! EFAnimationMgr + EFAnimBlendNode (Classes/EF/EFAnimationMgr.cpp, EFAnimBlendNode.cpp).
//!
//! The DTNode tree (parent +0x4, first child +0x8, next sibling +0xc; DTNode::AddChild PREPENDS)
//! is an index arena; slot 0 is the manager's root DTNode (this+0x8, type 0, first child at
//! this+0x10). DTWorkMemory / EF3dObj::LockTransfo data blocks (+0x10) are replaced by an owned
//! `Vec<DTTransfo>` per node; the pose of an EF3dObj is read with [`EFAnimationMgr::GetPose`].
#![allow(non_snake_case)]

use super::nodes::{EFChangeType, EFHAnimNode, state};
use super::quat::DTQuaternion;
use super::res::DTTransfo;
use super::coll_blend::{CollPrimList, EFAnimBaseCollPrim};
use super::coll_prim::{CollCtx, GetDefaultCollPrimAttrib};
use std::sync::Arc;
use super::texture_node::EFTextureAnimNode;

pub type NodeId = usize;
const ROOT: NodeId = 0;
/// DT_MAX_FLOAT (DTOperator ctor: this+0x18)
pub const DT_MAX_FLOAT: f32 = f32::MAX;
/// EFBlendParams id meaning "none"
pub const NO_BLEND_ID: u32 = 0xbbff_ffff;

/// EFBlendParams { +0 id, +4 ramp-in time, +8 target weight, +0xc fade-out time, +0x10 sound threshold }
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EFBlendParams {
    pub id: u32,
    pub ramp: f32,
    pub weight: f32,
    pub fade_out: f32,
    pub sound_threshold: f32,
}

/// EFAnimBlendNode (0x80 bytes, DTOperator type 4).
#[derive(Debug, Clone)]
pub struct EFAnimBlendNode {
    /// +0x1c blend id (EFBlendParams.id)
    pub id: u32,
    /// +0x34 EFAnimNode state (STOPPED from the EFAnimNode ctor)
    pub state: u32,
    /// +0x58 blend type: 1 interp from snapshot, 2 cross-fade, 3 ramp-in, 4 hold, 5 fade-out, 6 weight change
    pub ty: u8,
    /// +0x5c weight
    pub w: f32,
    /// +0x60 phase timer
    pub t: f32,
    /// +0x64 cross-fade duration (types 1/2)
    pub dur: f32,
    /// +0x68 ramp-in time
    pub ramp: f32,
    /// +0x6c fade-out time
    pub fade_out: f32,
    /// +0x70 target weight (type 6)
    pub w_target: f32,
    /// +0x74 weight-change time (type 6)
    pub w_time: f32,
    /// +0x78 sound threshold (-1 = always)
    pub sound_threshold: f32,
    /// +0x7c additive blend (second child's DTAnimHdr+0xe == 2)
    pub additive: bool,
    /// +0x4c/+0x50 PrepareInterpTransfo snapshot (type 1), quats precomputed
    pub snapshot: Vec<DTTransfo>,
}

impl EFAnimBlendNode {
    pub fn new(ty: u8, dur: f32) -> Self {
        EFAnimBlendNode {
            id: 0,
            state: state::STOPPED,
            ty,
            w: 0.0,
            t: 0.0,
            dur,
            ramp: 0.0,
            fade_out: 0.0,
            w_target: 0.0,
            w_time: 0.0,
            sound_threshold: -1.0,
            additive: false,
            snapshot: Vec::new(),
        }
    }

    /// The weight computation shared (inlined) by Update, ChangeBlendParams and CanPlaySoundCB.
    /// `cnt` is this+0x18 (remaining time). Mutates the phase like the C++.
    pub fn Weight(&mut self, dt: f32, cnt: f32) -> f32 {
        let mut w = 1.0;
        match self.ty {
            1 | 2 => {
                if 0.0 < self.dur {
                    w = 1.0 - cnt / self.dur;
                }
            }
            3 => {
                self.t += dt;
                if self.ramp <= self.t {
                    self.t = 0.0;
                    self.ty = 4;
                    w = self.w;
                } else {
                    w = self.w * (self.t / self.ramp);
                }
                if cnt < self.fade_out {
                    self.w = w;
                    w = self.begin_fade(cnt);
                }
            }
            4 => {
                w = self.w;
                if cnt < self.fade_out {
                    w = self.begin_fade(cnt);
                }
            }
            5 => {
                self.t += dt;
                if self.fade_out <= self.t {
                    w = 0.0;
                    self.t = self.fade_out;
                } else {
                    w = self.w - self.w * (self.t / self.fade_out);
                }
            }
            6 => {
                self.t += dt;
                if self.t < self.w_time {
                    w = self.w + (self.t / self.w_time) * (self.w_target - self.w);
                } else {
                    self.ty = 4;
                    w = self.w_target;
                    self.w = w;
                    self.t = 0.0;
                    self.w_time = 0.0;
                    self.w_target = 0.0;
                }
                if cnt < self.fade_out {
                    self.w = w;
                    w = self.begin_fade(cnt);
                }
            }
            _ => {}
        }
        w
    }

    fn begin_fade(&mut self, cnt: f32) -> f32 {
        self.ty = 5;
        self.t = self.fade_out - cnt;
        self.w - self.w * (self.t / self.fade_out)
    }

    /// EFAnimBlendNode::ChangeBlendParams
    pub fn ChangeBlendParams(&mut self, p: &EFBlendParams, cnt: &mut f32) {
        if self.ty == 1 || self.ty == 2 {
            return;
        }
        if 0.0 < p.ramp {
            let w = self.Weight(0.0, *cnt);
            self.w = w;
            self.ty = 6;
            self.t = 0.0;
            self.w_target = p.weight;
            self.w_time = p.ramp;
        } else {
            self.w = p.weight;
        }
        if 0.0 < p.fade_out {
            *cnt = p.fade_out;
            self.fade_out = p.fade_out;
        }
    }

    /// EFAnimBlendNode::CanPlaySoundCB(child). `child_is_first`: parent+0x8 == child.
    pub fn CanPlaySoundCB(&mut self, child_is_first: bool, cnt: f32) -> bool {
        if self.sound_threshold < 0.0 || self.ty == 1 || self.ty == 2 {
            return true;
        }
        let w = self.Weight(0.0, cnt);
        if child_is_first { w < self.sound_threshold } else { self.sound_threshold <= w }
    }
}

/// Snapshot copy with quats computed (PrepareInterpTransfo / the EulerToQuatf-on-demand in Update).
fn with_quat(t: &DTTransfo) -> DTTransfo {
    let mut t = *t;
    if t.state & 1 != 0 {
        t.quat = DTTransfo::EulerToQuatf([t.f[6], t.f[7], t.f[8]]);
        t.state &= !1;
    }
    t
}

fn lerp(a: f32, b: f32, w: f32) -> f32 {
    a + w * (b - a)
}

fn qmul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let [ax, ay, az, aw] = a;
    let [bx, by, bz, bw] = b;
    [
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    ]
}

fn qrot(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let r = qmul(qmul(q, [v[0], v[1], v[2], 0.0]), [-q[0], -q[1], -q[2], q[3]]);
    [r[0], r[1], r[2]]
}

/// One bone of EFAnimBlendNode::Update (non-additive, 0 < w < 1).
fn blend_one(a: &mut DTTransfo, b: &mut DTTransfo, w: f32) -> DTTransfo {
    *a = with_quat(a);
    *b = with_quat(b);
    let mut o = *a;
    o.quat = DTQuaternion::QLerp(&a.quat, &b.quat, w);
    for i in 0..6 {
        o.f[i] = lerp(a.f[i], b.f[i], w);
    }
    o.id = a.id;
    if a.state & 4 == 0 && b.state & 4 == 0 {
        o.state = 2;
    } else {
        if a.state & 4 == 0 {
            a.v34 = [1.0; 3];
        }
        if b.state & 4 == 0 {
            b.v34 = [1.0; 3];
        }
        for i in 0..3 {
            o.v34[i] = lerp(a.v34[i], b.v34[i], w);
        }
        o.state = 6;
    }
    o
}

/// EFAnimBlendNode::Update transfo merge (0x7c == 0, work-memory path): both lists sorted by id.
pub fn BlendTransfos(a: &[DTTransfo], b: &[DTTransfo], w: f32) -> Vec<DTTransfo> {
    let (mut a, mut b) = (a.to_vec(), b.to_vec());
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::with_capacity(a.len().max(b.len()));
    while i < a.len() && j < b.len() {
        if a[i].id == b[j].id {
            out.push(if w == 0.0 {
                a[i]
            } else if w == 1.0 {
                b[j]
            } else {
                blend_one(&mut a[i], &mut b[j], w)
            });
            i += 1;
            j += 1;
        } else if (a[i].id as u32) < (b[j].id as u32) {
            out.push(a[i]);
            i += 1;
        } else {
            out.push(b[j]);
            j += 1;
        }
    }
    out.extend_from_slice(if i < a.len() { &a[i..] } else { &b[j..] });
    out
}

/// EFAnimBlendNode::Update additive path (0x7c == 1): `add` applied on `base` with weight w.
pub fn AddTransfos(base: &[DTTransfo], add: &[DTTransfo], w: f32) -> Vec<DTTransfo> {
    let mut out = Vec::with_capacity(base.len());
    let mut j = 0;
    for a in base {
        if j < add.len() && a.id == add[j].id {
            if w == 0.0 {
                out.push(*a);
            } else {
                let a = with_quat(a);
                let b = with_quat(&add[j]);
                let mut o = DTTransfo::default();
                // DTQuaternion::Lerp(identity, add, w) (not normalised) * base
                let qa = DTQuaternion::Lerp(&[0.0, 0.0, 0.0, 1.0], &b.quat, w);
                o.quat = qmul(qa, a.quat);
                let t = qrot(a.quat, [b.f[3] * w, b.f[4] * w, b.f[5] * w]);
                for k in 0..3 {
                    o.f[3 + k] = a.f[3 + k] + t[k];
                    // +0x0 = base + add*w
                    o.f[k] = a.f[k] + b.f[k] * w;
                }
                o.id = a.id;
                o.state = 0;
                if a.state & 4 != 0 {
                    o.v34 = a.v34;
                    o.state |= 4;
                }
                out.push(o);
            }
            j += 1;
        } else {
            out.push(*a);
        }
    }
    out
}

#[derive(Debug, Clone)]
pub enum NodeKind {
    /// manager root DTNode (type 0)
    Root,
    HAnim(Box<EFHAnimNode>),
    /// EFTextureAnimNode (DTOperator type 2)
    Texture(Box<EFTextureAnimNode>),
    Blend(Box<EFAnimBlendNode>),
    /// EFCameraAnimNode (DTOperator type 7)
    Camera(Box<crate::engine::camera::EFCameraAnimNode>),
}

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub parent: Option<NodeId>,
    pub child: Option<NodeId>,
    pub next: Option<NodeId>,
    /// +0x18 DTOperator countdown
    pub cnt: f32,
    /// +0x24 exclusivity key (EFHAnimNode: channel-set count; blend: copied, -1 if mixed)
    pub excl: u32,
    /// +0x2c target EF3dObj id
    pub obj: u32,
    /// blend output (data block +0x10 of a blend node)
    pub out: Vec<DTTransfo>,
    /// blend node collision-prim chain (data block +0x10 -> +0x10, EFAnimBlendNode::BlendPrimColl)
    pub coll: CollPrimList,
    /// type-1 blend: the interpolated-from chain (+0x54)
    pub coll_snap: CollPrimList,
    pub kind: NodeKind,
}

impl TreeNode {
    /// DTOperator type byte +0x14
    pub fn ty(&self) -> u8 {
        match self.kind {
            NodeKind::Root => 0,
            NodeKind::HAnim(_) => 1,
            NodeKind::Texture(_) => 2,
            NodeKind::Blend(_) => 4,
            NodeKind::Camera(_) => 7,
        }
    }
    fn state_mut(&mut self) -> Option<&mut u32> {
        match &mut self.kind {
            NodeKind::Root => None,
            NodeKind::HAnim(h) => Some(&mut h.state),
            NodeKind::Texture(t) => Some(&mut t.state),
            NodeKind::Blend(b) => Some(&mut b.state),
            NodeKind::Camera(c) => Some(&mut c.state),
        }
    }
    pub fn state(&self) -> u32 {
        match &self.kind {
            NodeKind::Root => 0,
            NodeKind::HAnim(h) => h.state,
            NodeKind::Texture(t) => t.state,
            NodeKind::Blend(b) => b.state,
            NodeKind::Camera(c) => c.state,
        }
    }
}

/// EFAnimationMgr
#[derive(Debug, Clone)]
pub struct EFAnimationMgr {
    /// this+0x0 first anim id
    pub base: u32,
    /// this+0x4 anim table (AddAnim)
    pub anims: Vec<Option<NodeId>>,
    pub nodes: Vec<Option<TreeNode>>,
    /// this+0x50: 1 = no update, 2 = inside BeginTree/EndTree
    pub flags: u32,
    /// this+0x54: 0x20 = no update, 0x40 = time-only evaluation (SetClipMode)
    pub mode: u32,
    /// this+0x58 owner game object UID (EFGameObject ctor: mgr+0x58 = this+8)
    pub owner_uid: u32,
    /// this+0x5c when != 0, Update sends gameLogic Notify(0x11, owner, owner, 0, 1)
    pub notify_update: u32,
    /// queued EFNotificationMgr::Notify(0x11, owner) from Update (the coordinator drains it)
    pub notifications: Vec<(u32, u32)>,
    /// per EF3dObj (node +0x2c) what the collision-prim code reads through it (EFPhysic prims,
    /// clump prim records); filled by the coordinator. Absent = no EFPhysic.
    pub coll_ctx: std::collections::HashMap<u32, Arc<CollCtx>>,
}

impl EFAnimationMgr {
    pub fn new(base: u32, count: usize) -> Self {
        let root = TreeNode { parent: None, child: None, next: None, cnt: DT_MAX_FLOAT, excl: 0, obj: 0, out: Vec::new(), coll: Default::default(), coll_snap: Default::default(), kind: NodeKind::Root };
        EFAnimationMgr { base, anims: vec![None; count], nodes: vec![Some(root)], flags: 0, mode: 0, owner_uid: 0, notify_update: 0, notifications: Vec::new(), coll_ctx: Default::default() }
    }

    pub fn node(&self, i: NodeId) -> &TreeNode {
        self.nodes[i].as_ref().expect("live node")
    }
    pub fn node_mut(&mut self, i: NodeId) -> &mut TreeNode {
        self.nodes[i].as_mut().expect("live node")
    }
    pub fn hanim(&self, id: u32) -> Option<&EFHAnimNode> {
        match &self.node(self.anim(id)?).kind {
            NodeKind::HAnim(h) => Some(h),
            _ => None,
        }
    }
    pub fn hanim_mut(&mut self, id: u32) -> Option<&mut EFHAnimNode> {
        let i = self.anim(id)?;
        match &mut self.node_mut(i).kind {
            NodeKind::HAnim(h) => Some(h),
            _ => None,
        }
    }
    fn anim(&self, id: u32) -> Option<NodeId> {
        self.anims.get(id.wrapping_sub(self.base) as usize).copied().flatten()
    }
    fn alloc(&mut self, n: TreeNode) -> NodeId {
        if let Some(i) = self.nodes.iter().position(|x| x.is_none()) {
            self.nodes[i] = Some(n);
            i
        } else {
            self.nodes.push(Some(n));
            self.nodes.len() - 1
        }
    }
    fn first(&self, i: NodeId) -> Option<NodeId> {
        self.node(i).child
    }

    /// DTNode::AddChild (prepends)
    pub fn AddChild(&mut self, parent: NodeId, c: NodeId) {
        if self.node(c).parent.is_some() {
            self.LeaveParent(c);
        }
        let f = self.node(parent).child;
        let n = self.node_mut(c);
        n.parent = Some(parent);
        n.next = f;
        self.node_mut(parent).child = Some(c);
    }

    /// DTNode::LeaveParent
    pub fn LeaveParent(&mut self, c: NodeId) {
        let Some(p) = self.node(c).parent else { return };
        let next = self.node(c).next;
        if self.node(p).child == Some(c) {
            self.node_mut(p).child = next;
        } else {
            let mut k = self.node(p).child;
            while let Some(i) = k {
                if self.node(i).next == Some(c) {
                    self.node_mut(i).next = next;
                    break;
                }
                k = self.node(i).next;
            }
        }
        let n = self.node_mut(c);
        n.parent = None;
        n.next = None;
    }

    /// insert `c` under `parent` first (was_first) or as second child (the C++ re-add dance)
    fn reinsert(&mut self, parent: NodeId, c: NodeId, was_first: bool) {
        match (was_first, self.first(parent)) {
            (false, Some(old)) => {
                self.LeaveParent(old);
                self.AddChild(parent, c);
                self.AddChild(parent, old);
            }
            _ => self.AddChild(parent, c),
        }
    }

    /// EFAnimationMgr::AddAnim
    pub fn AddAnim(&mut self, node: EFHAnimNode, id: u32) -> NodeId {
        let n = TreeNode { parent: None, child: None, next: None, cnt: DT_MAX_FLOAT, excl: node.nb_bones, obj: node.obj, out: Vec::new(), coll: Default::default(), coll_snap: Default::default(), kind: NodeKind::HAnim(Box::new(node)) };
        let i = self.alloc(n);
        let k = id.wrapping_sub(self.base) as usize;
        if k >= self.anims.len() {
            self.anims.resize(k + 1, None);
        }
        self.anims[k] = Some(i);
        i
    }

    /// EFAnimNode::ChangeAnimState with the C++ recursion quirk (recurses into the start node's
    /// next sibling too).
    pub fn ChangeAnimState(&mut self, i: NodeId, bits: u32, ty: EFChangeType, recurse: bool) {
        if let Some(s) = self.node_mut(i).state_mut() {
            match ty {
                EFChangeType::Set => *s |= bits,
                EFChangeType::Clear => *s &= !bits,
                EFChangeType::Assign => *s = bits,
            }
        }
        if recurse {
            if let Some(c) = self.node(i).child {
                self.ChangeAnimState(c, bits, ty, true);
            }
            if let Some(n) = self.node(i).next {
                self.ChangeAnimState(n, bits, ty, true);
            }
        }
    }

    /// vtable +0x24 Stop. Returns true if the node must be deleted (blend nodes).
    fn StopNode(&mut self, i: NodeId) -> bool {
        match self.node(i).ty() {
            1 => {
                self.LeaveParent(i);
                if let NodeKind::HAnim(h) = &mut self.node_mut(i).kind {
                    h.Stop();
                }
                false
            }
            2 => {
                // EFTextureAnimNode::Stop: detach only if not already stopped
                if self.node(i).state() != state::STOPPED {
                    self.LeaveParent(i);
                }
                if let NodeKind::Texture(t) = &mut self.node_mut(i).kind {
                    t.Stop();
                }
                false
            }
            7 => {
                // EFCameraAnimNode::Stop
                self.LeaveParent(i);
                if let NodeKind::Camera(c) = &mut self.node_mut(i).kind {
                    c.Stop();
                }
                false
            }
            4 => {
                // EFAnimBlendNode::Stop
                self.LeaveParent(i);
                self.node_mut(i).out.clear();
                if let NodeKind::Blend(b) = &mut self.node_mut(i).kind {
                    b.snapshot.clear();
                }
                while let Some(c) = self.first(i) {
                    if self.StopNode(c) {
                        self.nodes[c] = None;
                    } else if self.node(c).parent == Some(i) {
                        self.LeaveParent(c);
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn stop_delete(&mut self, i: NodeId) {
        if self.StopNode(i) {
            self.nodes[i] = None;
        }
    }

    /// EFAnimationMgr::Play. HAnim: PlayHAnim(node, flags, -1, time, NULL, true) -- the bool is
    /// register t1, which Play loads with 1 for the type compare and never clobbers (verified in
    /// the disassembly at 0x1e9284/0x1e92c0). Other nodes: Stop if playing, ConnectNode, Play.
    pub fn Play(&mut self, id: u32, flags: u32, time: f32) {
        let Some(i) = self.anim(id) else { return };
        if self.node(i).ty() == 1 {
            self.PlayHAnim(i, flags, u32::MAX, time, None, true);
        } else {
            if self.node(i).state() & state::STOPPED == 0 {
                self.stop_delete(i);
            }
            self.ConnectNode(i);
            match &mut self.node_mut(i).kind {
                NodeKind::Texture(t) => t.Play(flags, time),
                NodeKind::Camera(c) => c.Play(flags, time),
                _ => {}
            }
        }
    }

    /// EFAnimationMgr::ConnectNode
    pub fn ConnectNode(&mut self, i: NodeId) {
        if self.node(i).ty() == 2 {
            self.ConnectTextureAnim(i);
        } else {
            if let Some(e) = self.FindExclNode(i, self.first(ROOT)) {
                self.stop_delete(e);
            }
            self.AddChild(ROOT, i);
        }
    }

    /// EFAnimationMgr::ConnectTextureAnim: stops the exclusive texture node driving the same
    /// material (+0x48; here the connected object id).
    pub fn ConnectTextureAnim(&mut self, i: NodeId) {
        let mut e = self.FindExclNode(i, self.first(ROOT));
        while let Some(k) = e {
            if self.node(k).obj == self.node(i).obj {
                self.stop_delete(k);
                break;
            }
            e = self.FindExclNode(i, self.node(k).next);
        }
        self.AddChild(ROOT, i);
    }

    /// AddAnim for a texture anim node
    pub fn AddTextureAnim(&mut self, node: EFTextureAnimNode, id: u32) -> NodeId {
        let n = TreeNode { parent: None, child: None, next: None, cnt: DT_MAX_FLOAT, excl: node.excl, obj: node.obj, out: Vec::new(), coll: Default::default(), coll_snap: Default::default(), kind: NodeKind::Texture(Box::new(node)) };
        let i = self.alloc(n);
        let k = id.wrapping_sub(self.base) as usize;
        if k >= self.anims.len() {
            self.anims.resize(k + 1, None);
        }
        self.anims[k] = Some(i);
        i
    }

    /// AddAnim for a camera anim node (EFGameObject::CreateInstances, class 0x53)
    pub fn AddCameraAnim(&mut self, node: crate::engine::camera::EFCameraAnimNode, id: u32) -> NodeId {
        let n = TreeNode { parent: None, child: None, next: None, cnt: DT_MAX_FLOAT, excl: node.excl, obj: node.obj, out: Vec::new(), coll: Default::default(), coll_snap: Default::default(), kind: NodeKind::Camera(Box::new(node)) };
        let i = self.alloc(n);
        let k = id.wrapping_sub(self.base) as usize;
        if k >= self.anims.len() {
            self.anims.resize(k + 1, None);
        }
        self.anims[k] = Some(i);
        i
    }

    /// the camera anim node of anim `id` (None for other node types)
    pub fn camera_mut(&mut self, id: u32) -> Option<&mut crate::engine::camera::EFCameraAnimNode> {
        let i = self.anim(id)?;
        match &mut self.node_mut(i).kind {
            NodeKind::Camera(c) => Some(c),
            _ => None,
        }
    }

    /// anim table entry (this+4)[id - this+0]
    pub fn anim_node(&self, id: u32) -> Option<NodeId> {
        self.anim(id)
    }

    /// EFAnimNode fields (+0x24 excl, +0x34 state, +0x38 time, +0x3c duration, +0x40 speed)
    pub fn AnimFields(&self, id: u32) -> Option<(u32, u32, f32, f32, f32)> {
        let i = self.anim(id)?;
        let ex = self.node(i).excl;
        match &self.node(i).kind {
            NodeKind::HAnim(h) => Some((ex, h.state, h.time, h.duration, h.speed)),
            NodeKind::Texture(t) => Some((ex, t.state, t.time, t.duration, t.speed)),
            NodeKind::Camera(c) => Some((ex, c.state, c.time, c.duration, c.speed)),
            _ => None,
        }
    }

    /// vtable +0x2c SetTime (EFHAnimNode::SetTime also re-seeks the callback cursor)
    pub fn SetTime(&mut self, id: u32, t: f32) {
        let Some(i) = self.anim(id) else { return };
        match &mut self.node_mut(i).kind {
            NodeKind::HAnim(h) => h.SetTime(t),
            NodeKind::Texture(x) => x.time = t,
            // EFAnimNode::SetTime (0x001eac60)
            NodeKind::Camera(x) => x.time = t,
            _ => {}
        }
    }

    /// APIanim SetSpeed body: state &= ~FINISHED; speed = s
    pub fn SetSpeed(&mut self, id: u32, s: f32) {
        let Some(i) = self.anim(id) else { return };
        match &mut self.node_mut(i).kind {
            NodeKind::HAnim(h) => {
                h.state &= !state::FINISHED;
                h.speed = s;
            }
            NodeKind::Texture(x) => {
                x.state &= !state::FINISHED;
                x.speed = s;
            }
            NodeKind::Camera(x) => {
                x.state &= !state::FINISHED;
                x.speed = s;
            }
            _ => {}
        }
    }

    /// EFAnimationMgr::PlayBlend
    pub fn PlayBlend(&mut self, id: u32, flags: u32, blend_id: u32, time: f32, p: Option<EFBlendParams>) {
        if let Some(i) = self.anim(id) {
            self.PlayHAnim(i, flags, blend_id, time, p, false);
        }
    }

    /// EFAnimationMgr::PlayHAnim(node, flags, blend id, time, params, interp from cur state)
    pub fn PlayHAnim(&mut self, i: NodeId, mut flags: u32, blend_id: u32, time: f32, p: Option<EFBlendParams>, interp: bool) {
        if self.node(i).state() & state::STOPPED == 0 && flags & 0x10 != 0 {
            flags = flags & !0x10 | 8;
        }
        // ConnectHAnim returns the adjusted flags; quirk: 0 flags (also failure) skip Play.
        let fl = self.ConnectHAnim(i, flags, blend_id, time, p, interp);
        if fl != 0 {
            if let NodeKind::HAnim(h) = &mut self.node_mut(i).kind {
                h.Play(fl, 0.0);
            }
            if self.flags & 2 == 0 {
                self.UpdateHierarchyAnimNode(self.first(ROOT));
            }
        }
    }

    fn new_blend(&mut self, b: EFAnimBlendNode, from: NodeId, excl: u32, cnt: f32) -> NodeId {
        let obj = self.node(from).obj;
        self.alloc(TreeNode { parent: None, child: None, next: None, cnt, excl, obj, out: Vec::new(), coll: Default::default(), coll_snap: Default::default(), kind: NodeKind::Blend(Box::new(b)) })
    }

    /// collision-prim chain (data block +0x10 -> +0x10) of a node
    pub fn CollOutput(&self, i: NodeId) -> CollPrimList {
        match &self.node(i).kind {
            NodeKind::HAnim(h) => h.coll.clone(),
            NodeKind::Blend(_) => self.node(i).coll.clone(),
            NodeKind::Root | NodeKind::Texture(_) | NodeKind::Camera(_) => CollPrimList::default(),
        }
    }

    /// The collision-prim chain EFAnimNode::SetCollPrimData applies for an EF3dObj: the chain of
    /// its root-level node (the node whose data block is EF3dObj+0x50).
    pub fn GetCollPrims(&self, obj: u32) -> Option<CollPrimList> {
        let mut k = self.first(ROOT);
        while let Some(i) = k {
            if self.node(i).obj == obj {
                return Some(self.CollOutput(i));
            }
            k = self.node(i).next;
        }
        None
    }

    /// pose (data block) of a node
    pub fn Output(&self, i: NodeId) -> Vec<DTTransfo> {
        match &self.node(i).kind {
            NodeKind::HAnim(h) => h.transfos.clone(),
            NodeKind::Blend(_) => self.node(i).out.clone(),
            NodeKind::Root | NodeKind::Texture(_) | NodeKind::Camera(_) => Vec::new(),
        }
    }

    /// EFAnimBlendNode(EFAnimNode&, DTAttrib&, DTAttrib&, float) (type 1) + PrepareInterpTransfo
    fn BlendInterp(&mut self, new: NodeId, snap: Vec<DTTransfo>, time: f32) -> NodeId {
        let mut b = EFAnimBlendNode::new(1, time);
        b.snapshot = snap.iter().map(with_quat).collect();
        let excl = self.node(new).excl;
        let k = self.new_blend(b, new, excl, time);
        self.AddChild(k, new);
        k
    }

    /// EFAnimBlendNode(EFAnimNode& old, EFAnimNode& new, float) (type 2)
    fn BlendCross(&mut self, old: NodeId, new: NodeId, time: f32) -> NodeId {
        let excl = self.node(old).excl;
        let k = self.new_blend(EFAnimBlendNode::new(2, time), old, excl, time);
        let mut keep = old;
        if let NodeKind::Blend(ob) = &self.node(old).kind {
            match ob.ty {
                1 => {
                    keep = self.first(old).expect("type 1 child");
                    self.LeaveParent(keep);
                    self.stop_delete(old);
                }
                2 => {
                    keep = self.node(self.first(old).expect("child")).next.expect("2nd child");
                    self.LeaveParent(keep);
                    self.stop_delete(old);
                }
                _ => {}
            }
        }
        self.AddChild(k, new);
        self.AddChild(k, keep);
        k
    }

    /// EFAnimBlendNode(EFAnimNode& old, EFAnimNode& new, EFBlendParams*) (type 3)
    fn BlendLayer(&mut self, old: NodeId, new: NodeId, p: Option<EFBlendParams>) -> NodeId {
        let mut b = EFAnimBlendNode::new(4, 0.0);
        self.LeaveParent(old);
        let excl = if self.node(old).excl == self.node(new).excl {
            if let Some(p) = p.filter(|p| p.sound_threshold != -1.0) {
                b.sound_threshold = p.sound_threshold;
            }
            self.node(new).excl
        } else {
            u32::MAX
        };
        let p = p.unwrap_or(EFBlendParams { id: NO_BLEND_ID, ramp: 0.1, weight: 1.0, fade_out: 0.1, sound_threshold: -1.0 });
        b.id = p.id;
        b.ramp = p.ramp;
        b.w = p.weight;
        b.fade_out = p.fade_out;
        let cnt = if b.fade_out != 0.0 {
            let d = match &self.node(new).kind {
                NodeKind::HAnim(h) => h.duration,
                _ => 0.0, // +0x3c of a blend node is 0
            };
            b.fade_out + d
        } else {
            DT_MAX_FLOAT
        };
        b.ty = 3;
        let k = self.new_blend(b, old, excl, cnt);
        self.AddChild(k, new);
        self.AddChild(k, old);
        self.SetAdditive(k);
        k
    }

    /// the +0x7c computation (ctor type 3 and ConnectHAnim)
    fn SetAdditive(&mut self, k: NodeId) {
        let add = match &self.node(k).kind {
            NodeKind::Blend(b) if b.ty != 1 && b.ty != 2 => {
                let second = self.first(k).and_then(|f| self.node(f).next);
                match second.map(|s| &self.node(s).kind) {
                    Some(NodeKind::HAnim(h)) => h.res.base.anim.as_ref().is_some_and(|a| a.unkd[1] == 2),
                    _ => false,
                }
            }
            _ => false,
        };
        if let NodeKind::Blend(b) = &mut self.node_mut(k).kind {
            b.additive = add;
        }
    }

    /// EFAnimationMgr::ConnectHAnim(node, flags, blend id, time, EFBlendParams*, bool interp).
    /// Returns the adjusted flags (C++ uStack_9064), 0 when the blend target is missing.
    /// Last two args resolved from the callers: Play passes (NULL, true), PlayBlend /
    /// PlayBlendConnect pass (their EFBlendParams*, false) (t1 cleared in the jal delay slot).
    pub fn ConnectHAnim(&mut self, node: NodeId, flags: u32, blend_id: u32, time: f32, p: Option<EFBlendParams>, interp: bool) -> u32 {
        let mut was_first = true;
        let mut parent: Option<NodeId> = None;
        let mut new = node;
        let mut fl = flags;
        let root_first = self.first(ROOT);
        if blend_id == u32::MAX {
            let excl = self.FindExclNode(node, root_first);
            if interp && excl.is_none_or(|e| self.node(e).parent.is_some_and(|p| self.node(p).ty() != 0)) {
                if self.FindNonExclNode(node, self.first(ROOT)).is_none() {
                    fl &= !0x1c;
                } else {
                    new = self.InterpFromCurState(node, &mut fl, time);
                    parent = Some(ROOT);
                }
            } else if let Some(e) = excl {
                let ep = self.node(e).parent.expect("in tree");
                parent = Some(ep);
                was_first = self.first(ep) == Some(e);
                if fl & 0x1c == 0 {
                    self.stop_delete(e);
                } else {
                    self.LeaveParent(e);
                    if fl & 0xc == 0 {
                        new = self.BlendCross(e, node, time);
                        self.ChangeAnimState(new, 0x10, EFChangeType::Set, true);
                    } else {
                        self.UpdateHierarchyAnimNode(Some(e));
                        let snap = self.Output(e);
                        let csnap = self.CollOutput(e);
                        self.stop_delete(e);
                        new = self.BlendInterp(node, snap, time);
                        self.node_mut(new).coll_snap = csnap;
                        self.ChangeAnimState(new, fl & 0xc, EFChangeType::Set, true);
                    }
                }
            } else {
                fl &= !0x1c;
            }
        } else {
            let target = if blend_id & 0xbb00_0000 == 0 { self.anim(blend_id) } else { self.FindBlendNode(root_first, blend_id) };
            let Some(target) = target else { return 0 };
            // C++ dereferences the parent unconditionally
            let Some(tp) = self.node(target).parent else { return 0 };
            parent = Some(tp);
            was_first = self.first(tp) == Some(target);
            self.LeaveParent(target);
            new = self.BlendLayer(target, node, p);
            let s = self.node(target).state() & 0x1c;
            self.ChangeAnimState(new, s, EFChangeType::Set, true);
            fl = fl & !0x1c | s;
        }
        let mut ins = new;
        let parent = match parent {
            Some(p) => p,
            None => match self.FindNonExclNode(node, self.first(ROOT)) {
                Some(ne) => {
                    let pp = self.node(ne).parent.unwrap_or(ROOT);
                    // quirk: was_first stays true here
                    ins = self.BlendLayer(ne, new, p);
                    pp
                }
                None => ROOT,
            },
        };
        self.reinsert(parent, ins, was_first);
        if parent != ROOT && self.node(parent).ty() == 4 {
            self.SetAdditive(parent);
        }
        fl
    }

    /// EFAnimationMgr::FindBlendNode
    pub fn FindBlendNode(&self, n: Option<NodeId>, id: u32) -> Option<NodeId> {
        let i = n?;
        if let NodeKind::Blend(b) = &self.node(i).kind {
            if b.id == id {
                return Some(i);
            }
        }
        self.FindBlendNode(self.node(i).next, id).or_else(|| self.FindBlendNode(self.node(i).child, id))
    }

    fn compatible(&self, a: NodeId, b: NodeId) -> bool {
        let (x, y) = (self.node(a).ty(), self.node(b).ty());
        x == y || ((x == 4 || x == 1) && (y == 4 || y == 1))
    }

    /// EFAnimationMgr::FindExclNode: same +0x24 and same +0x2c, compatible types.
    pub fn FindExclNode(&self, n: NodeId, t: Option<NodeId>) -> Option<NodeId> {
        let t = t?;
        let (a, b) = (self.node(n), self.node(t));
        if a.excl == b.excl && a.obj == b.obj && self.compatible(n, t) {
            return Some(t);
        }
        self.FindExclNode(n, b.child).or_else(|| self.FindExclNode(n, b.next))
    }

    /// EFAnimationMgr::FindNonExclNode: different +0x24, same +0x2c, compatible types.
    pub fn FindNonExclNode(&self, n: NodeId, t: Option<NodeId>) -> Option<NodeId> {
        let t = t?;
        let (a, b) = (self.node(n), self.node(t));
        if a.excl != b.excl && a.obj == b.obj && self.compatible(n, t) {
            return Some(t);
        }
        self.FindNonExclNode(n, b.child).or_else(|| self.FindNonExclNode(n, b.next))
    }

    /// EFAnimationMgr::InterpFromCurState
    pub fn InterpFromCurState(&mut self, node: NodeId, flags: &mut u32, time: f32) -> NodeId {
        self.UpdateHierarchyAnimNode(self.first(ROOT));
        if *flags & 0x10 != 0 {
            *flags = *flags & !0x10 | 8;
        }
        let obj = self.node(node).obj;
        let mut k = self.first(ROOT);
        while let Some(i) = k {
            let t = self.node(i).ty();
            if (t == 4 || t == 1) && self.node(i).obj == obj {
                break;
            }
            k = self.node(i).next;
        }
        // C++ dereferences NULL when no root node matches the object
        let cur = k.expect("InterpFromCurState: no current node for the object");
        let snap = self.Output(cur);
        let csnap = self.CollOutput(cur);
        self.stop_delete(cur);
        let b = self.BlendInterp(node, snap, time);
        self.node_mut(b).coll_snap = csnap;
        self.ChangeAnimState(b, *flags & 0xc, EFChangeType::Set, true);
        b
    }

    /// EFAnimationMgr::ChangeBlendParam
    pub fn ChangeBlendParam(&mut self, p: &EFBlendParams) {
        if let Some(k) = self.FindBlendNode(self.first(ROOT), p.id) {
            let n = self.node_mut(k);
            let mut cnt = n.cnt;
            if let NodeKind::Blend(b) = &mut n.kind {
                b.ChangeBlendParams(p, &mut cnt);
            }
            n.cnt = cnt;
        }
    }

    /// EFAnimationMgr::Stop
    pub fn Stop(&mut self, id: u32) {
        let Some(i) = self.anim(id) else { return };
        if self.node(i).state() & state::STOPPED == 0 {
            if self.node(i).ty() == 1 {
                self.StopHAnim(i);
            } else {
                self.stop_delete(i);
            }
        }
    }

    /// EFAnimationMgr::StopHAnim
    pub fn StopHAnim(&mut self, n: NodeId) {
        let Some(p) = self.node(n).parent else {
            self.stop_delete(n);
            return;
        };
        if self.node(p).ty() == 0 {
            self.stop_delete(n);
            return;
        }
        let bt = match &self.node(p).kind {
            NodeKind::Blend(b) => b.ty,
            _ => 0,
        };
        let (other, dest, was_first);
        if bt == 1 || bt == 2 {
            let gp = self.node(p).parent.expect("blend in tree");
            if bt == 2 && self.first(p) == Some(n) {
                other = self.node(n).next;
                was_first = false; // quirk: always re-added as second child
                other.map(|o| self.LeaveParent(o));
                self.stop_delete(p);
                dest = gp;
            } else {
                if self.node(gp).ty() != 4 {
                    self.stop_delete(p);
                    return;
                }
                let mut o = self.first(gp);
                let ggp = self.node(gp).parent.expect("blend in tree");
                if o == Some(p) {
                    o = self.node(p).next;
                }
                was_first = self.first(ggp) == Some(gp);
                o.map(|o| self.LeaveParent(o));
                self.stop_delete(gp);
                other = o;
                dest = ggp;
            }
        } else {
            let mut o = self.first(p);
            if o == Some(n) {
                o = self.node(n).next;
            }
            dest = self.node(p).parent.expect("blend in tree");
            was_first = self.first(dest) == Some(p);
            o.map(|o| self.LeaveParent(o));
            self.stop_delete(p);
            other = o;
        }
        if let Some(o) = other {
            self.reinsert(dest, o, was_first);
        }
    }

    /// EFAnimationMgr::StopAll
    pub fn StopAll(&mut self) {
        while let Some(i) = self.first(ROOT) {
            self.stop_delete(i);
            if self.first(ROOT) == Some(i) {
                self.LeaveParent(i); // hanim Stop already detached; safety
            }
        }
    }

    /// EFAnimationMgr::BeginTree / EndTree
    pub fn BeginTree(&mut self) {
        self.flags |= 2;
    }
    pub fn EndTree(&mut self) {
        self.flags &= !2;
        self.UpdateHierarchyAnimNode(self.first(ROOT));
    }

    /// EFAnimationMgr::Update (the gameLogic Notify 0x11 on this+0x5c is the caller's business)
    pub fn Update(&mut self, dt: f32) {
        if self.mode != 0x20 && self.flags & 1 == 0 {
            if let Some(f) = self.first(ROOT) {
                self.Evaluate(dt, f);
            }
        }
        if self.notify_update != 0 {
            self.notifications.push((0x11, self.owner_uid));
        }
    }

    /// EFAnimationMgr::Evaluate (recurses first child then next sibling, then updates the node)
    pub fn Evaluate(&mut self, dt: f32, i: NodeId) {
        let c = self.node(i).cnt - dt;
        self.node_mut(i).cnt = c;
        if c < 0.0 {
            let next = self.node(i).next;
            let mut dt2 = dt;
            let surv = self.TimeOut(i, &mut dt2);
            self.stop_delete(i);
            match surv {
                None => {
                    if let Some(n) = next {
                        self.Evaluate(dt, n);
                    }
                }
                Some(s) => {
                    self.ChangeAnimState(s, 0x1c, EFChangeType::Clear, true);
                    self.Evaluate(dt2, s);
                }
            }
            return;
        }
        if let Some(k) = self.node(i).child {
            self.Evaluate(dt, k);
        }
        if let Some(k) = self.node(i).next {
            self.Evaluate(dt, k);
        }
        if self.mode != 0x40 {
            self.UpdateNode(dt, i);
            return;
        }
        // time-only evaluation
        if let NodeKind::HAnim(h) = &mut self.node_mut(i).kind {
            h.AdvanceTime(dt);
        }
    }

    /// vtable +0x10 TimeOut. Blend nodes only (DTOperator::TimeOut returns 0).
    fn TimeOut(&mut self, i: NodeId, dt: &mut f32) -> Option<NodeId> {
        let NodeKind::Blend(b) = &self.node(i).kind else { return None };
        let surv = match b.ty {
            1 => {
                *dt = -self.node(i).cnt;
                self.first(i)
            }
            2 => self.first(i).and_then(|f| self.node(f).next),
            _ => self.first(i),
        }?;
        let p = self.node(i).parent.expect("in tree");
        let was_first = self.first(p) == Some(i);
        self.LeaveParent(i);
        self.LeaveParent(surv);
        self.reinsert(p, surv, was_first);
        let s = self.node(surv).state();
        if self.node(p).ty() != 4 && self.node(surv).ty() != 4 && s & state::FINISHED != 0 {
            self.ChangeAnimState(surv, state::FORCE_UPDATE, EFChangeType::Set, false);
        }
        Some(surv)
    }

    /// vtable +0x20 Update
    fn UpdateNode(&mut self, dt: f32, i: NodeId) {
        let p = self.node(i).parent;
        match self.node(i).ty() {
            1 => {
                let blend_parent = p.filter(|&p| self.node(p).ty() == 4);
                let first = p.is_some_and(|p| self.first(p) == Some(i));
                let can = match blend_parent {
                    Some(bp) => {
                        let n = self.node_mut(bp);
                        let cnt = n.cnt;
                        match &mut n.kind {
                            NodeKind::Blend(b) => b.CanPlaySoundCB(first, cnt),
                            _ => true,
                        }
                    }
                    None => true,
                };
                let ctx = self.coll_ctx.get(&self.node(i).obj).cloned();
                if let NodeKind::HAnim(h) = &mut self.node_mut(i).kind {
                    h.coll_ctx = ctx;
                    h.parent_is_blend = blend_parent.is_some();
                    h.can_play_sound = can;
                    h.Update(dt);
                }
            }
            2 => {
                if let NodeKind::Texture(t) = &mut self.node_mut(i).kind {
                    t.Update(dt);
                }
            }
            4 => self.UpdateBlend(dt, i),
            7 => {
                if let NodeKind::Camera(c) = &mut self.node_mut(i).kind {
                    c.Update(dt);
                }
            }
            _ => {}
        }
    }

    /// EFAnimBlendNode::Update (+ BlendPrimColl at the end)
    fn UpdateBlend(&mut self, dt: f32, i: NodeId) {
        self.UpdateBlendPose(dt, i);
        // EFAnimBlendNode::BlendPrimColl: type 1 blends the snapshot (+0x54) with the only child
        let obj = self.node(i).obj;
        let Some(ctx) = self.coll_ctx.get(&obj).cloned() else { return };
        let f = self.first(i);
        let (a, b) = match &self.node(i).kind {
            NodeKind::Blend(bn) if bn.ty == 1 => (self.node(i).coll_snap.clone(), f.map(|f| self.CollOutput(f)).unwrap_or_default()),
            NodeKind::Blend(_) => {
                let s = f.and_then(|f| self.node(f).next);
                (f.map(|f| self.CollOutput(f)).unwrap_or_default(), s.map(|s| self.CollOutput(s)).unwrap_or_default())
            }
            _ => return,
        };
        let cnt = self.node(i).cnt;
        let mut da = |tag: u32, r: &mut EFAnimBaseCollPrim| GetDefaultCollPrimAttrib(&ctx, tag, r);
        let out = match &mut self.node_mut(i).kind {
            NodeKind::Blend(bn) => bn.BlendPrimColl(true, cnt, &a, &b, &mut da),
            _ => None,
        };
        if let Some((l, _)) = out {
            self.node_mut(i).coll = l;
        }
    }

    fn UpdateBlendPose(&mut self, dt: f32, i: NodeId) {
        let cnt = self.node(i).cnt;
        let (ty, additive) = match &self.node(i).kind {
            NodeKind::Blend(b) => (b.ty, b.additive),
            _ => return,
        };
        let f = self.first(i);
        let (a, b) = if ty == 1 {
            let NodeKind::Blend(bn) = &self.node(i).kind else { return };
            (bn.snapshot.clone(), f.map(|f| self.Output(f)).unwrap_or_default())
        } else {
            let s = f.and_then(|f| self.node(f).next);
            (f.map(|f| self.Output(f)).unwrap_or_default(), s.map(|s| self.Output(s)).unwrap_or_default())
        };
        let w = match &mut self.node_mut(i).kind {
            NodeKind::Blend(bn) => bn.Weight(dt, cnt),
            _ => return,
        };
        self.node_mut(i).out = if additive { AddTransfos(&a, &b, w) } else { BlendTransfos(&a, &b, w) };
    }

    /// EFAnimationMgr::UpdateHierarchyAnimNode (EF3dObj::UpdateHierarchy is the caller's business)
    pub fn UpdateHierarchyAnimNode(&mut self, n: Option<NodeId>) {
        let Some(i) = n else { return };
        let t = self.node(i).ty();
        if t == 4 || t == 1 {
            if let Some(c) = self.node(i).child {
                self.UpdateHierarchyAnimNode(Some(c));
            }
            if let Some(c) = self.node(i).next {
                self.UpdateHierarchyAnimNode(Some(c));
            }
            self.ChangeAnimState(i, state::FORCE_UPDATE, EFChangeType::Set, false);
            self.UpdateNode(0.0, i);
            self.ChangeAnimState(i, state::FORCE_UPDATE, EFChangeType::Clear, false);
        } else if let Some(c) = self.node(i).next {
            self.UpdateHierarchyAnimNode(Some(c));
        }
    }

    /// Pose of an EF3dObj (replaces the root-level data block bound to EF3dObj+0x50, type 10)
    pub fn GetPose(&self, obj: u32) -> Option<Vec<DTTransfo>> {
        let mut k = self.first(ROOT);
        while let Some(i) = k {
            if self.node(i).obj == obj {
                return Some(self.Output(i));
            }
            k = self.node(i).next;
        }
        None
    }
}

//! WP-D4: morph-target animation.
//! - ApMorphAnimation RenderWare plugin (Free Functions/"Ap plugins - ApMorphAnimation
//!   (morph-target animation).cpp", 0x0011a9e0..0x0011bab7): replaced (rule 2) by native structs,
//!   but the per-function behaviour (key cursor walk, slopes, normal renormalisation, atomic
//!   callbacks) follows the C++ step by step.
//! - EFMorphAnimNode (Classes/EF/EFMorphAnimNode.cpp, class 0x51): 1:1.
//!
//! Blob layout (chunk rwID 0x1bad006, read verbatim by ApMorphAnimationStreamRead; every pointer
//! is an offset from the blob start, patched by ConvertOffsetToPointers; 0 = NULL):
//! - ApMorphAnimationEx: +0 ptr atomic table (u32 offsets, one per atomic, 0 = none), +4 u8
//!   atomic count, +8 u16 version, +0xc u8 nb channel sets (EFMorphAnimNode::Initialize: only if
//!   version >= 0x28a8).
//! - ApMorphAnimEx (per atomic): +0 vertex channel, +4 normal channel, +8 (patched, never read).
//! - ApMorphAnimChannelEx<RwV3d>: +0 f32 time, +4 i16 nb keys, +6 i16 current key, +8 u16 nb
//!   vertices, +0xc u16* vertex indices, +0x10 RwV3d* slopes, +0x14 RwV3d* base (normals),
//!   +0x18 RwV3d* current values, +0x1c ApMorphKey* keys.
//! - ApMorphKey<RwV3d> (0xc): +0 f32 time, +4 ApMorphKey* next, +8 RwV3d* values.
#![allow(non_snake_case)]

use crate::engine::error::EngineError;
use crate::engine::rw::stream::RwStream;

pub type V3 = [f32; 3];

/// ApMorphKey<RwV3d>
#[derive(Debug, Clone, PartialEq)]
pub struct ApMorphKey {
    /// +0
    pub time: f32,
    /// +4 next key (index into the channel's key array; the C++ keeps a pointer)
    pub next: usize,
    /// +8
    pub values: Vec<V3>,
}

/// ApMorphAnimChannelEx<RwV3d>
#[derive(Debug, Clone, PartialEq)]
pub struct ApMorphAnimChannelEx {
    /// +0
    pub time: f32,
    /// +4
    pub nb_keys: i16,
    /// +6
    pub cur_key: i16,
    /// +0xc (+8 = len)
    pub indices: Vec<u16>,
    /// +0x10
    pub slopes: Vec<V3>,
    /// +0x14
    pub base: Vec<V3>,
    /// +0x18
    pub cur: Vec<V3>,
    /// +0x1c
    pub keys: Vec<ApMorphKey>,
}

/// ApMorphAnimEx (one per atomic)
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ApMorphAnimEx {
    /// +0
    pub vertices: Option<ApMorphAnimChannelEx>,
    /// +4
    pub normals: Option<ApMorphAnimChannelEx>,
    /// +8 (raw offset, unused by the engine)
    pub extra: u32,
}

/// ApMorphAnimationEx
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ApMorphAnimationEx {
    /// +0 / +4 (u8 count)
    pub atomics: Vec<Option<ApMorphAnimEx>>,
    /// +8
    pub version: u16,
    /// +0xc
    pub nb_sets: u8,
}

/// Morph targets of an RpAtomic/RpGeometry as the plugin sees them (replaces RpAtomic,
/// RpGeometry morph target 0 at geometry+0x5c, and ApAtomicCreateMorphTargets' copies).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MorphAtomic {
    /// geometry+0x5c->+0x14 vertices
    pub verts: Vec<V3>,
    /// geometry+0x5c->+0x18 normals
    pub normals: Vec<V3>,
    /// geometry+0x18 nb morph targets
    pub nb_geom_morph_targets: i32,
    /// ApAtomicGetMorphTarget(atomic, 0/1): (verts, normals)
    pub targets: Vec<(Vec<V3>, Vec<V3>)>,
    /// atomic+0x50/+0x52 interpolator start/end, atomic+0x4c flags
    pub interp: (u16, u16),
    pub interp_flags: u32,
}

/// RpClump: atomics in RpClumpForAllAtomics order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MorphClump {
    pub atomics: Vec<MorphAtomic>,
}

// ------------------------------------------------------------------------------------------
// stream read

fn r_u32(b: &[u8], o: usize) -> Result<u32, EngineError> {
    b.get(o..o + 4).map(|x| u32::from_le_bytes(x.try_into().unwrap())).ok_or_else(|| bad("morph blob truncated"))
}
fn r_u16(b: &[u8], o: usize) -> Result<u16, EngineError> {
    b.get(o..o + 2).map(|x| u16::from_le_bytes(x.try_into().unwrap())).ok_or_else(|| bad("morph blob truncated"))
}
fn r_v3s(b: &[u8], o: u32, n: usize) -> Result<Vec<V3>, EngineError> {
    if o == 0 {
        return Ok(vec![[0.0; 3]; n]);
    }
    (0..n)
        .map(|i| {
            let p = o as usize + i * 12;
            Ok([0, 4, 8].map(|k| f32::from_bits(r_u32(b, p + k).unwrap_or(0))))
                .and_then(|v| if b.len() >= p + 12 { Ok(v) } else { Err(bad("morph blob truncated")) })
        })
        .collect()
}
fn bad(s: &str) -> EngineError {
    EngineError::Format(s.into())
}

/// ConvertOffsetToPointers<RwV3d> + read of one channel at blob offset `o`.
fn read_channel(b: &[u8], o: u32) -> Result<Option<ApMorphAnimChannelEx>, EngineError> {
    if o == 0 {
        return Ok(None);
    }
    let o = o as usize;
    let nb_keys = r_u16(b, o + 4)? as i16;
    let nv = r_u16(b, o + 8)? as usize;
    let idx = r_u32(b, o + 0xc)?;
    let indices = (0..nv).map(|i| if idx == 0 { Ok(0) } else { r_u16(b, idx as usize + i * 2) }).collect::<Result<_, _>>()?;
    let keys_off = r_u32(b, o + 0x1c)?;
    let mut keys = Vec::new();
    for k in 0..(nb_keys.max(0) as usize) {
        let p = keys_off as usize + k * 0xc;
        let next = r_u32(b, p + 4)?;
        keys.push(ApMorphKey {
            time: f32::from_bits(r_u32(b, p)?),
            next: if next == 0 { k } else { (next.wrapping_sub(keys_off) / 0xc) as usize },
            values: r_v3s(b, r_u32(b, p + 8)?, nv)?,
        });
    }
    Ok(Some(ApMorphAnimChannelEx {
        time: f32::from_bits(r_u32(b, o)?),
        nb_keys,
        cur_key: r_u16(b, o + 6)? as i16,
        indices,
        slopes: r_v3s(b, r_u32(b, o + 0x10)?, nv)?,
        base: r_v3s(b, r_u32(b, o + 0x14)?, nv)?,
        cur: r_v3s(b, r_u32(b, o + 0x18)?, nv)?,
        keys,
    }))
}

/// ConvertOffsetToPointers(ApMorphAnimationEx*) applied to a blob.
pub fn ParseMorphBlob(b: &[u8]) -> Result<ApMorphAnimationEx, EngineError> {
    let table = r_u32(b, 0)?;
    let n = *b.get(4).ok_or_else(|| bad("morph blob truncated"))? as usize;
    let mut atomics = Vec::with_capacity(n);
    for i in 0..n {
        let a = if table == 0 { 0 } else { r_u32(b, table as usize + i * 4)? };
        atomics.push(if a == 0 {
            None
        } else {
            let a = a as usize;
            Some(ApMorphAnimEx { vertices: read_channel(b, r_u32(b, a)?)?, normals: read_channel(b, r_u32(b, a + 4)?)?, extra: r_u32(b, a + 8)? })
        });
    }
    Ok(ApMorphAnimationEx { atomics, version: r_u16(b, 8).unwrap_or(0), nb_sets: b.get(0xc).copied().unwrap_or(0) })
}

/// ApMorphAnimationStreamRead (0x0011b3e0): RwStreamFindChunk(0x1bad006); size 0 / missing ->
/// NULL; else MMMalloc + RwStreamRead(size) + ConvertOffsetToPointers(anim, 0).
pub fn ApMorphAnimationStreamRead(s: &mut RwStream) -> Result<Option<ApMorphAnimationEx>, EngineError> {
    let Some(h) = s.FindChunk(0x1bad006) else { return Ok(None) };
    if h.size == 0 {
        return Ok(None);
    }
    let b = s.Read(h.size as usize)?;
    ParseMorphBlob(b).map(Some)
}

// ------------------------------------------------------------------------------------------
// channel runtime

/// ApMorphAnimProcessUpdate<RwV3d> (0x0011b790)
pub fn ApMorphAnimProcessUpdate(c: &mut ApMorphAnimChannelEx, key: usize, recompute: bool, normalize: bool) {
    let nv = c.indices.len();
    let k = &c.keys[key];
    if recompute {
        let nx = &c.keys[k.next];
        let inv = 1.0 / (nx.time - k.time);
        for v in 0..nv {
            for j in 0..3 {
                c.slopes[v][j] = (nx.values[v][j] - k.values[v][j]) * inv;
            }
        }
    }
    let dt = c.time - k.time;
    for v in 0..nv {
        let mut p = [0.0; 3];
        for j in 0..3 {
            p[j] = k.values[v][j] + dt * c.slopes[v][j];
        }
        if normalize {
            for j in 0..3 {
                p[j] += c.base[v][j];
            }
            // RwV3dLength; quirk: no zero-length guard
            let l = 1.0 / (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
            p = p.map(|x| x * l);
        }
        c.cur[v] = p;
    }
}

/// ApMorphAnimInitialize<RwV3d> (0x0011b520): normals keep the current values as base, vertices
/// reset to 0; then ProcessUpdate from key 0 (the cursor itself is not reset).
pub fn ApMorphAnimInitialize(c: Option<&mut ApMorphAnimChannelEx>, normal: bool) {
    let Some(c) = c else { return };
    if normal {
        c.base = c.cur.clone();
    } else {
        c.cur.iter_mut().for_each(|v| *v = [0.0; 3]);
    }
    ApMorphAnimProcessUpdate(c, 0, true, normal);
}

/// ApMorphAnimChannelAddTime<RwV3d> (0x0011b640): moves the key cursor with wrap-around.
pub fn ApMorphAnimChannelAddTime(c: &mut ApMorphAnimChannelEx, dt: f32, normal: bool) {
    c.time += dt;
    let mut k = c.cur_key as usize;
    let mut changed = false;
    if c.time < c.keys[k].time {
        changed = true;
        while c.time < c.keys[k].time {
            c.cur_key -= 1;
            if c.cur_key < 0 {
                c.cur_key = c.nb_keys - 1;
                k = c.cur_key as usize;
                // quirk: adds the last key's time (== duration when key 0 is at t=0)
                c.time += c.keys[k].time;
            } else {
                k = c.cur_key as usize;
            }
        }
    } else if c.keys[c.keys[k].next].time < c.time {
        changed = true;
        while c.keys[c.keys[k].next].time < c.time {
            c.cur_key += 1;
            k = c.cur_key as usize;
            if c.cur_key as i32 == c.nb_keys as u16 as i32 - 1 {
                c.time -= c.keys[k].time;
                k = 0;
                c.cur_key = 0;
            }
        }
    }
    ApMorphAnimProcessUpdate(c, k, changed, normal);
}

/// ApMorphAnimChannelSetTime<RwV3d> (0x0011b9c0)
pub fn ApMorphAnimChannelSetTime(c: &mut ApMorphAnimChannelEx, t: f32, normal: bool) {
    let d = t - c.time;
    ApMorphAnimChannelAddTime(c, d, normal);
}

/// ApMorphAnimationSetTime (0x0011b150)
pub fn ApMorphAnimationSetTime(a: &mut ApMorphAnimationEx, t: f32) -> u32 {
    for e in a.atomics.iter_mut().flatten() {
        if let Some(c) = e.vertices.as_mut() {
            ApMorphAnimChannelSetTime(c, t, false);
        }
        if let Some(c) = e.normals.as_mut() {
            ApMorphAnimChannelSetTime(c, t, true);
        }
    }
    1
}

/// ApMorphAnimationAddTime (0x0011b210)
pub fn ApMorphAnimationAddTime(a: &mut ApMorphAnimationEx, dt: f32) -> u32 {
    for e in a.atomics.iter_mut().flatten() {
        if let Some(c) = e.vertices.as_mut() {
            ApMorphAnimChannelAddTime(c, dt, false);
        }
        if let Some(c) = e.normals.as_mut() {
            ApMorphAnimChannelAddTime(c, dt, true);
        }
    }
    1
}

/// ApMorphAnimationGetDuration (0x0011b480): atomic 0's vertex channel, last key time - first.
/// quirk: no NULL checks (port returns 0.0).
pub fn ApMorphAnimationGetDuration(a: &ApMorphAnimationEx) -> f32 {
    let Some(Some(ApMorphAnimEx { vertices: Some(c), .. })) = a.atomics.first() else { return 0.0 };
    match (c.keys.get((c.nb_keys as i32 - 1) as usize), c.keys.first()) {
        (Some(l), Some(f)) => l.time - f.time,
        _ => 0.0,
    }
}

/// ApMorphAnimationInitCallBack (0x0011aa70)
fn InitCallBack(at: &mut MorphAtomic, anim: Option<&ApMorphAnimEx>) {
    let Some(a) = anim else { return };
    if a.vertices.is_none() {
        return;
    }
    if at.nb_geom_morph_targets < 2 {
        // ApAtomicCreateMorphTargets + copy of the geometry's vertices/normals into targets 0 and 1
        at.targets = vec![(at.verts.clone(), at.normals.clone()); 2];
    }
    // interpolator start/end = 0, flags |= 3 (RwFrameUpdateObjects is the renderer's business)
    at.interp = (0, 0);
    at.interp_flags |= 3;
}

/// ApMorphAnimationAtomicCallBack (0x0011ac00)
fn AtomicCallBack(at: &mut MorphAtomic, anim: Option<&ApMorphAnimEx>, copy_only: i32) {
    let Some(a) = anim else { return };
    let Some(vc) = a.vertices.as_ref() else { return };
    let Some((src, src_n)) = at.targets.first().cloned() else { return };
    if copy_only == 0 {
        if let Some(nc) = a.normals.as_ref() {
            for (k, &i) in nc.indices.iter().enumerate() {
                at.normals[i as usize] = nc.cur[k];
            }
        }
        for (k, &i) in vc.indices.iter().enumerate() {
            let s = src[i as usize];
            let d = vc.cur[k];
            at.verts[i as usize] = [s[0] + d[0], s[1] + d[1], s[2] + d[2]];
        }
    } else {
        // memcpy(verts, target0 verts, nverts*0x18): verts then normals (contiguous in the RW block)
        at.verts = src;
        at.normals = src_n;
    }
}

/// ApMorphAnimationInit (0x0011b2d0)
pub fn ApMorphAnimationInit(clump: &mut MorphClump, a: &mut ApMorphAnimationEx) {
    for e in a.atomics.iter_mut() {
        // quirk: the C++ dereferences every table entry without a NULL check
        if let Some(e) = e.as_mut() {
            ApMorphAnimInitialize(e.vertices.as_mut(), false);
            ApMorphAnimInitialize(e.normals.as_mut(), true);
        }
    }
    for (i, at) in clump.atomics.iter_mut().enumerate() {
        InitCallBack(at, a.atomics.get(i).and_then(|x| x.as_ref()));
    }
    ApMorphAnimationSetTime(a, 0.0);
}

/// ApMorphAnimationSetGeometry (0x0011b390)
pub fn ApMorphAnimationSetGeometry(clump: &mut MorphClump, a: &ApMorphAnimationEx, copy_only: i32) {
    for (i, at) in clump.atomics.iter_mut().enumerate() {
        AtomicCallBack(at, a.atomics.get(i).and_then(|x| x.as_ref()), copy_only);
    }
}

/// ApMorphAnimationInitBlend (0x0011b4c0) / ApMorphAnimationInitBlendCallBack: target 1 =
/// current geometry (every atomic, animated or not).
pub fn ApMorphAnimationInitBlend(clump: &mut MorphClump, _a: &ApMorphAnimationEx) {
    for at in clump.atomics.iter_mut() {
        if at.targets.len() < 2 {
            at.targets.resize(2, (Vec::new(), Vec::new()));
        }
        at.targets[1] = (at.verts.clone(), at.normals.clone());
    }
}

/// ApMorphAnimationBlend (0x0011b4f0) / ApMorphAnimationBlendCallBack (0x0011aec0)
pub fn ApMorphAnimationBlend(clump: &mut MorphClump, a: &ApMorphAnimationEx, w: f32) {
    for (i, at) in clump.atomics.iter_mut().enumerate() {
        let Some(Some(e)) = a.atomics.get(i) else { continue };
        let Some(vc) = e.vertices.as_ref() else { continue };
        if at.targets.len() < 2 {
            continue;
        }
        let (t0, t1) = (at.targets[0].0.clone(), at.targets[1].0.clone());
        for (k, &ix) in vc.indices.iter().enumerate() {
            let s = t0[ix as usize];
            at.verts[ix as usize] = [s[0] + vc.cur[k][0], s[1] + vc.cur[k][1], s[2] + vc.cur[k][2]];
        }
        for (v, b) in at.verts.iter_mut().zip(&t1) {
            for j in 0..3 {
                v[j] = (v[j] - b[j]) * w + b[j];
            }
        }
    }
}

// ------------------------------------------------------------------------------------------
// EFMorphAnimNode

/// EFMorphAnimNode : EFAnimNode (DTOperator type 3, size 0x50).
///
/// Memory model: the C++ node points at the resource's ApMorphAnimationEx (+0x44 = res+0x10) and
/// the owner's clump (EF3dObj+0x30); here the node owns a copy of the animation and the clump is
/// passed in. quirk (not reproduced): nodes sharing one EFMorphAnimRes share channel times.
#[derive(Debug, Clone, Default)]
pub struct EFMorphAnimNode {
    /// +0x20 user id (notify payload)
    pub user: u32,
    /// +0x24 nb channel sets
    pub nb_sets: u32,
    /// +0x34 state flags
    pub state: u32,
    /// +0x38 time
    pub time: f32,
    /// +0x3c duration
    pub duration: f32,
    /// +0x40 speed
    pub speed: f32,
    /// +0x44
    pub anim: ApMorphAnimationEx,
    /// +0x48 blend timer
    pub blend_t: f32,
    /// +0x4c blend duration
    pub blend_dur: f32,
    /// EFAnimNode::Notify(1, user) calls, drained by the caller
    pub notifications: Vec<(u32, u32)>,
}

impl EFMorphAnimNode {
    pub const CLASS_ID: u32 = 0x51;
    pub const CLASS_NAME: &'static str = "EF_MORPH_ANIM_NODE";

    /// ctor (EFAnimNode(3): state STOPPED 0x8000000, speed 1) + Initialize (0x0023b140)
    pub fn new(anim: ApMorphAnimationEx, clump: &mut MorphClump, user: u32) -> Self {
        let mut n = EFMorphAnimNode { user, state: super::nodes::state::STOPPED, speed: 1.0, anim, ..Default::default() };
        n.Initialize(clump);
        n
    }

    /// EFMorphAnimNode::Initialize
    pub fn Initialize(&mut self, clump: &mut MorphClump) -> u32 {
        ApMorphAnimationInit(clump, &mut self.anim);
        self.nb_sets = if self.anim.version < 0x28a8 { 0 } else { self.anim.nb_sets as u32 };
        self.duration = ApMorphAnimationGetDuration(&self.anim);
        1
    }

    /// EFMorphAnimNode::Play (0x0023b1c0)
    pub fn Play(&mut self, clump: &mut MorphClump, flags: u32, f: f32) {
        if self.state & 0x800_0000 == 0 {
            self.time = 0.0;
        } else if flags & 4 != 0 {
            ApMorphAnimationInitBlend(clump, &self.anim);
            self.blend_t = 0.0;
            self.blend_dur = f;
        }
        // EFAnimNode::Play
        self.state = flags;
        ApMorphAnimationSetTime(&mut self.anim, self.time);
        ApMorphAnimationSetGeometry(clump, &self.anim, 0);
    }

    /// shared time-advance block of Update (both branches)
    fn advance(&mut self, d: f32) {
        if self.state & 0x800_0000 != 0 || !(0.0 < self.duration) {
            return;
        }
        self.time += d;
        self.state &= 0xbfff_ffff;
        if self.duration <= self.time {
            if self.state & 2 == 0 {
                if self.state & 1 != 0 {
                    self.time = self.duration;
                    self.state |= 0x200_0000;
                }
            } else {
                while self.duration < self.time {
                    self.time -= self.duration;
                    self.state |= 0x4000_0000;
                }
            }
        } else if self.time <= 0.0 {
            if self.state & 2 == 0 {
                if self.state & 1 != 0 {
                    self.time = 0.0;
                    self.state |= 0x200_0000;
                }
            } else {
                while self.time < 0.0 {
                    self.time += self.duration;
                    self.state |= 0x4000_0000;
                }
            }
        }
        if self.state & 0x4000_0000 != 0 {
            self.state |= 0x200_0000;
        }
    }

    /// EFMorphAnimNode::Update (0x0023b270). Ghidra shows `this` as param_2 (thiscall + float).
    pub fn Update(&mut self, clump: &mut MorphClump, dt: f32) -> u32 {
        if self.state & 0x2000_0000 != 0 {
            return 0;
        }
        if self.state & 4 == 0 {
            self.advance(dt * self.speed);
            if self.state & 1 != 0
                && ((self.speed < 0.0 && self.time == 0.0) || (0.0 < self.speed && self.time == self.duration))
            {
                self.state |= 0x2000_0000;
            }
            if self.state & 0x200_0000 != 0 {
                self.notifications.push((1, self.user));
                self.state &= 0xfdff_ffff;
            }
            if self.state & 0x2000_0000 == 0 {
                ApMorphAnimationAddTime(&mut self.anim, dt * self.speed);
            } else if self.speed < 0.0 {
                ApMorphAnimationSetTime(&mut self.anim, 0.0);
            } else {
                ApMorphAnimationSetTime(&mut self.anim, self.duration);
            }
        } else {
            let t = self.blend_t;
            self.blend_t = t + dt;
            if self.blend_dur < t + dt {
                self.state &= 0xffff_fffb;
            }
            if self.state & 4 != 0 {
                ApMorphAnimationBlend(clump, &self.anim, self.blend_t / self.blend_dur);
                ApMorphAnimationSetGeometry(clump, &self.anim, 0);
                return 0;
            }
            // quirk: speed * (blend_dur - blend_t) * speed, and AddTime gets the absolute time
            self.advance(self.speed * (self.blend_dur - self.blend_t) * self.speed);
            ApMorphAnimationAddTime(&mut self.anim, self.time);
        }
        ApMorphAnimationSetGeometry(clump, &self.anim, 0);
        0
    }

    /// EFMorphAnimNode::Stop (0x0023b780): LeaveParent (caller's tree), SetTime(0), EFAnimNode::Stop.
    pub fn Stop(&mut self) {
        ApMorphAnimationSetTime(&mut self.anim, 0.0);
        if self.state != super::nodes::state::STOPPED {
            self.state = super::nodes::state::STOPPED;
            self.time = 0.0;
            self.notifications.push((0, self.user));
        }
    }

    /// ShallowCopy (0x0023b7c0): new node on the same resource + user id (resource refcount++).
    pub fn ShallowCopy(&self, res_anim: ApMorphAnimationEx) -> Self {
        EFMorphAnimNode { user: self.user, state: super::nodes::state::STOPPED, speed: 1.0, anim: res_anim, ..Default::default() }
    }
}

//! APIphysic_* host functions (Engine API (module interface)/EFApi_Physic.cpp), 1:1.
//! `EFWorld::GetGameObject(uid)+0x28` is `mgr.PhysicOf(uid)`; module handles go through the
//! manager's table. Param blocks are word slices (the logic VM copies them to/from Mem).
//! Unported here: EF3dObj::CreateCollisionPrimitives; JumpOnCoor flags 4/8 (DTQuartic::Solve).
//! WP-E7 added SetSpherePrimaryVolume, Prim*IntAttrib*, Prim{Get,Set}OffsetFromJoint,
//! JumpOnCoor and the CheckRayIntersection result packing (`CheckRayIntersectionOut`).
#![allow(non_snake_case)]

use super::dm::NewModule;
use super::mgr::*;
use super::module::*;
use super::physic::*;
use super::rwm::*;
use super::model::{ObjPrim, PrimExt};
use super::objcoll_prim::{EFObjCollisionPrim, PRIM_SPHERE};

pub fn SetFlags(m: &mut EFPhysicMgr, uid: u32, f: u32) {
    if let Some(p) = m.PhysicMut(m.PhysicOf(uid)) {
        p.flags |= f;
        p.UpdatePhysicFlags();
    }
}
pub fn ResetFlags(m: &mut EFPhysicMgr, uid: u32, f: u32) {
    if let Some(p) = m.PhysicMut(m.PhysicOf(uid)) {
        p.flags &= !f;
        p.UpdatePhysicFlags();
    }
}
pub fn GetFlags(m: &EFPhysicMgr, uid: u32) -> u32 { m.Physic(m.PhysicOf(uid)).map_or(0, |p| p.flags) }
pub fn GetPhysicState(m: &EFPhysicMgr, uid: u32) -> u32 { m.Physic(m.PhysicOf(uid)).map_or(0, |p| p.state) }

macro_rules! setter {
    ($name:ident, $t:ty, |$p:ident, $v:ident| $body:expr) => {
        pub fn $name(m: &mut EFPhysicMgr, uid: u32, $v: $t) {
            if let Some($p) = m.PhysicMut(m.PhysicOf(uid)) {
                $body;
            }
        }
    };
}
setter!(SetFallingSpeed, f32, |p, v| p.falling_speed = v);
setter!(SetFallingDelay, f32, |p, v| p.falling_delay = v);
setter!(SetStateChangeNotifyBits, u32, |p, v| p.notify_bits |= v);
setter!(ResetStateChangeNotifyBits, u32, |p, v| p.notify_bits &= !v);
setter!(SetStateChangeNotifyValue, u32, |p, v| p.notify_bits = v);
setter!(Impulse, V3, |p, v| p.velocity = add(p.velocity, v));
setter!(SetPriority, u32, |p, v| p.priority = v);
setter!(SetWeight, f32, |p, v| p.weight = v);
setter!(SetFriction, f32, |p, v| p.friction = v);
setter!(SetHardness, f32, |p, v| p.hardness = v);
setter!(SetRebound, f32, |p, v| p.rebound = v);
setter!(SetPrimaryPrim, u32, |p, v| p.model.primary = if v == 0xffff_ffff { -1 } else { p.model.PrimTagToPrimIndex(v) as i32 });

/// CreatePhysic(uid, list): only when the object has none. `frame` = the object's root frame.
/// (EF3dObj::CreateCollisionPrimitives is the caller's job.)
pub fn CreatePhysic(m: &mut EFPhysicMgr, uid: u32, list: u32, frame: RwMatrix) -> PhysId {
    if m.PhysicOf(uid) != 0 {
        return 0;
    }
    m.CreatePhysic(uid, frame, list)
}

/// DMAdd(uid, kind) -> handle (0: no object/physic, unknown or unported kind)
pub fn DMAdd(m: &mut EFPhysicMgr, uid: u32, kind: u32) -> u32 {
    let id = m.PhysicOf(uid);
    if id == 0 {
        return 0;
    }
    match NewModule(kind) {
        Some(dm) => m.AddModule(id, dm),
        None => 0,
    }
}

pub fn DMGetParams(m: &mut EFPhysicMgr, h: u32, w: &dyn PhysicWorld) -> Option<Vec<u32>> {
    m.WithModule(h, w, |dm, c| {
        let mut v = vec![0u32; dm.ParamWords().max(1)];
        dm.GetParams(c, &mut v);
        v
    })
}
pub fn DMSetParams(m: &mut EFPhysicMgr, h: u32, p: Option<&[u32]>, w: &dyn PhysicWorld) {
    m.WithModule(h, w, |dm, c| {
        // pad short blocks so a module never indexes past the guest's struct
        let pad;
        let p = match p {
            Some(p) if p.len() < dm.ParamWords() => {
                let mut v = p.to_vec();
                v.resize(dm.ParamWords(), 0);
                pad = v;
                Some(&pad[..])
            }
            o => o,
        };
        dm.SetParams(c, p)
    });
}
pub fn DMDoAction(m: &mut EFPhysicMgr, h: u32, action: u32, p: &mut [u32], w: &dyn PhysicWorld) {
    m.WithModule(h, w, |dm, c| dm.DoAction(c, action, p));
}
pub fn DMGetData(m: &mut EFPhysicMgr, h: u32, w: &dyn PhysicWorld) -> Option<Vec<u32>> {
    m.WithModule(h, w, |dm, c| {
        let mut v = vec![0u32; dm.DataWords().max(1)];
        dm.GetData(c, &mut v);
        v
    })
}
pub fn DMSetContextMask(m: &mut EFPhysicMgr, h: u32, mask: u32) {
    if let Some(d) = m.ModuleMut(h) {
        d.base_mut().context_mask = mask;
    }
}
pub fn DMSetActive(m: &mut EFPhysicMgr, h: u32, a: u32) {
    if let Some(d) = m.ModuleMut(h) {
        let b = d.base_mut();
        if a == 0xffff_ffff {
            if b.type_ & TYPE_PRE != 0 {
                b.flags |= 0x10;
            }
            if b.type_ & TYPE_COL != 0 {
                b.flags |= 0x20;
            }
            if b.type_ & TYPE_POST != 0 {
                b.flags |= 0x40;
            }
            b.flags &= !1;
        } else if a == 1 {
            b.flags |= 1;
        } else {
            b.flags &= 0xffff_ff8e;
            b.state = 0;
        }
    }
}
fn change_state(m: &mut EFPhysicMgr, h: u32, f: impl FnOnce(u32) -> u32) {
    let Some(i) = m.Slot(h) else { return };
    let b = m.slots[i].1.as_mut().unwrap().base_mut();
    let n = f(b.state);
    let ch = b.state ^ n;
    b.state = n;
    if ch & b.notify_bits != 0 {
        let d = [b.handle, b.type_, b.state, ch];
        let pid = b.physic;
        let owner = m.Physic(pid).map_or(0, |p| p.owner);
        m.notify.push(PhysicNotify { owner, code: 6, data: d });
    }
}
pub fn DMSetStateBits(m: &mut EFPhysicMgr, h: u32, bits: u32) { change_state(m, h, |s| s | bits) }
pub fn DMResetStateBits(m: &mut EFPhysicMgr, h: u32, bits: u32) { change_state(m, h, |s| s & !bits) }
pub fn DMGetState(m: &EFPhysicMgr, h: u32) -> u32 { m.Module(h).map_or(0, |d| d.base().state) }
pub fn DMSetStateChangeNotifyBits(m: &mut EFPhysicMgr, h: u32, bits: u32) {
    if let Some(d) = m.ModuleMut(h) {
        d.base_mut().notify_bits |= bits;
    }
}
pub fn DMResetStateChangeNotifyBits(m: &mut EFPhysicMgr, h: u32, bits: u32) {
    if let Some(d) = m.ModuleMut(h) {
        d.base_mut().notify_bits &= !bits;
    }
}
pub fn DMIsActive(m: &EFPhysicMgr, h: u32) -> bool {
    let Some(d) = m.Module(h) else { return false };
    let b = d.base();
    if b.flags & 0x70 != 0 {
        return true;
    }
    b.flags & 1 != 0 && m.Physic(b.physic).map_or(false, |p| b.context_mask & p.context != 0)
}
/// GetDM(uid, type, after) -> handle of the next module of `type`. quirk: the C++ reads
/// `FindModule(..)+0x2c` without a NULL check (low-memory read); here it returns 0.
pub fn GetDM(m: &EFPhysicMgr, uid: u32, ty: u32, after: u32) -> u32 {
    let id = m.PhysicOf(uid);
    if id == 0 {
        return 0;
    }
    let after = if m.Slot(after).is_some() { after } else { 0 };
    m.FindModule(id, ty, after)
}

fn carrier_vel(m: &EFPhysicMgr, uid: u32, w: &dyn PhysicWorld) -> Option<V3> {
    let par = w.GetParentObject(uid)?;
    let p = m.Physic(m.PhysicOf(par))?;
    if p.flags & flags::RELATIVE_VEL != 0 { Some(p.velocity) } else { None }
}
/// GetVelocity: relative to the carrier (parent object's physic with flag 0x400000)
pub fn GetVelocity(m: &EFPhysicMgr, uid: u32, w: &dyn PhysicWorld) -> Option<V3> {
    let p = m.Physic(m.PhysicOf(uid))?;
    let mut v = p.velocity;
    if let Some(c) = carrier_vel(m, uid, w) {
        v = sub(v, c);
    }
    Some(v)
}
/// SetVelocity: + carrier velocity; resets the falling timer to the delay
pub fn SetVelocity(m: &mut EFPhysicMgr, uid: u32, v: V3, w: &dyn PhysicWorld) {
    let c = carrier_vel(m, uid, w);
    if let Some(p) = m.PhysicMut(m.PhysicOf(uid)) {
        p.velocity = match c { Some(c) => add(v, c), None => v };
        p.falling_timer = p.falling_delay;
    }
}
/// GetFloorAttribs -> (on floor && !(state&4), attribs (zeroed without a floor contact))
pub fn GetFloorAttribs(m: &EFPhysicMgr, uid: u32) -> Option<(u32, EnvPrimAttribs)> {
    let p = m.Physic(m.PhysicOf(uid))?;
    let a = p.floor.map_or([0; 8], |f| f.0);
    Some(((p.state & 1 != 0 && p.state & 4 == 0) as u32, a))
}
/// GetFloorNormal -> (1 with a floor contact, normal)
pub fn GetFloorNormal(m: &EFPhysicMgr, uid: u32) -> (u32, V3) {
    match m.Physic(m.PhysicOf(uid)).and_then(|p| p.floor) {
        Some(f) => (1, f.1),
        None => (0, [0.0; 3]),
    }
}
/// CheckRayIntersection: EFPhysicMgr::RayIntersect with the SELF id resolved.
pub fn CheckRayIntersection(m: &mut EFPhysicMgr, mut rin: EFRayIntersectIn, w: &dyn PhysicWorld, cs: &mut dyn CollisionSystem) -> EFRayResult {
    if rin.exclude == 0x2a00_0000 {
        rin.exclude = w.CurrentLogicObject();
    }
    m.RayIntersect(&rin, cs)
}

// ------------------------------------------------------------------ WP-E7

/// SetSpherePrimaryVolume(uid, radius, x, y, z) (0x001f5a20): the model is reset to one
/// EFObjCollisionSphere prim (radius, joint offset) that is also the primary volume.
/// quirk: the new prim's world position is its ctor value (0 here) + offset.
pub fn SetSpherePrimaryVolume(m: &mut EFPhysicMgr, uid: u32, r: f32, x: f32, y: f32, z: f32) {
    let Some(p) = m.PhysicMut(m.PhysicOf(uid)) else { return };
    let root = p.frame.clone();
    let md = &mut p.model;
    // SetMaxPrims(1) + destroy prim 0 + new EFObjCollisionSphere (kind 0, radius 0) on the root frame
    md.SetMaxPrims(1);
    let mut base = EFObjCollisionPrim::new(rows(&root));
    base.kind = PRIM_SPHERE;
    let mut prim = ObjPrim { base, ext: PrimExt::Sphere };
    md.b3c = 1; // EFPhysic+0x90
    prim.base.radius = r;
    prim.base.offset = [x, y, z];
    // quirk: center (ctor FLT_MAX, 0, 0) += offset
    prim.base.center = add(prim.base.center, prim.base.offset);
    md.bsphere_radius = prim.base.radius;
    md.bsphere_center = prim.base.offset;
    md.prims[0] = Some(prim);
}

fn rows(m: &RwMatrix) -> [[f32; 4]; 4] {
    [[m.right[0], m.right[1], m.right[2], 0.0], [m.up[0], m.up[1], m.up[2], 0.0], [m.at[0], m.at[1], m.at[2], 0.0], [m.pos[0], m.pos[1], m.pos[2], 1.0]]
}

/// PrimTagToPrimIndex then the slot (miss = 0xffffffff -> None)
fn prim_mut(m: &mut EFPhysicMgr, uid: u32, tag: u32) -> Option<&mut ObjPrim> {
    let p = m.PhysicMut(m.PhysicOf(uid))?;
    let i = p.model.PrimTagToPrimIndex(tag);
    p.model.prims.get_mut(i as usize)?.as_mut()
}

/// The shared tail of the Prim*IntAttrib* setters on the prim's 4 x 11 attrib words:
/// `cur = base & !lock; cur |= value & lock`. `i` is not range-checked in the C++ (writes past
/// the prim); out-of-range indices are ignored here.
pub fn IntAttribUpdate(w: &mut [[u32; 11]; 4], i: usize, f: impl FnOnce(&mut u32, &mut u32)) {
    if i >= 11 {
        return;
    }
    let [cur, base, value, lock] = w;
    f(&mut value[i], &mut lock[i]);
    cur[i] = base[i] & !lock[i];
    cur[i] |= value[i] & lock[i];
}
fn int_attrib(m: &mut EFPhysicMgr, uid: u32, tag: u32, i: u32, f: impl FnOnce(&mut u32, &mut u32)) {
    let Some(q) = prim_mut(m, uid, tag) else { return };
    // prim words +0xc.. = current[11], base[11], value[11], lock[11]
    let mut w = [[0u32; 11]; 4];
    for (k, x) in q.base.words.iter().enumerate() {
        w[k / 11][k % 11] = *x;
    }
    IntAttribUpdate(&mut w, i as usize, f);
    for (k, x) in q.base.words.iter_mut().enumerate() {
        *x = w[k / 11][k % 11];
    }
}
/// PrimSetIntAttribBits(uid, tag, index, bits) (0x001f6f60)
pub fn PrimSetIntAttribBits(m: &mut EFPhysicMgr, uid: u32, tag: u32, i: u32, bits: u32) {
    int_attrib(m, uid, tag, i, |v, l| {
        *v |= bits;
        *l |= bits;
    })
}
/// PrimResetIntAttribBits (0x001f7130)
pub fn PrimResetIntAttribBits(m: &mut EFPhysicMgr, uid: u32, tag: u32, i: u32, bits: u32) {
    int_attrib(m, uid, tag, i, |v, l| {
        *v &= !bits;
        *l |= bits;
    })
}
/// PrimSetIntAttribValue (0x001f7230): whole word overridden
pub fn PrimSetIntAttribValue(m: &mut EFPhysicMgr, uid: u32, tag: u32, i: u32, val: u32) {
    int_attrib(m, uid, tag, i, |v, l| {
        *v = val;
        *l = 0xffff_ffff;
    })
}
/// PrimUnlockIntAttrib (0x001f7320): bits go back to the base value
pub fn PrimUnlockIntAttrib(m: &mut EFPhysicMgr, uid: u32, tag: u32, i: u32, bits: u32) {
    int_attrib(m, uid, tag, i, |_, l| *l &= !bits)
}
/// PrimGetIntAttrib (0x001f7060): current word (0 without object/physic/prim)
pub fn PrimGetIntAttrib(m: &mut EFPhysicMgr, uid: u32, tag: u32, i: u32) -> u32 {
    let Some(q) = prim_mut(m, uid, tag) else { return 0 };
    if i < 11 { q.base.words[i as usize] } else { 0 }
}
/// PrimGetOffsetFromJoint (0x001f7410): Some(offset) = 0, None = 0xffffffff
pub fn PrimGetOffsetFromJoint(m: &mut EFPhysicMgr, uid: u32, tag: u32) -> Option<V3> {
    prim_mut(m, uid, tag).map(|q| q.base.offset)
}
/// PrimSetOffsetFromJoint (0x001f74f0) -> 0 / 0xffffffff.
/// quirk: the world position is incremented by the new offset (not re-derived).
pub fn PrimSetOffsetFromJoint(m: &mut EFPhysicMgr, uid: u32, tag: u32, o: V3) -> u32 {
    let Some(q) = prim_mut(m, uid, tag) else { return 0xffff_ffff };
    q.base.offset = o;
    q.base.center = add(q.base.center, q.base.offset);
    0
}

/// JumpOnCoorParam (guest struct, 10 words): +0 target, +0xc target velocity, +0x18 max speed,
/// +0x1c flags (1 clamp speed, 2 apex height, 4/8 quartic solves), +0x20 apex height,
/// +0x24 DM handle whose +0x30 vector is the gravity (an EFDMAcceleration).
/// JumpOnCoor (0x001f7940): sets the velocity that reaches the target under that gravity.
/// The gravity is read as the module's GetParams words 0..3 (== module+0x30 for the
/// EFDMAcceleration both callers pass). quirk: an invalid handle reads low memory in the C++;
/// gravity (0,0,0) here. Unported: flags 4/8 (DTQuartic::Solve) keep the flag-2 time.
pub fn JumpOnCoor(m: &mut EFPhysicMgr, uid: u32, prm: &[u32], w: &dyn PhysicWorld) {
    let id = m.PhysicOf(uid);
    let Some(pos) = m.Physic(id).map(|p| p.frame.pos) else { return };
    let g = DMGetParams(m, prm[9], w).map_or([0.0; 3], |v| if v.len() >= 3 { wv(&v, 0) } else { [0.0; 3] });
    let target = wv(prm, 0);
    let tv = wv(prm, 3);
    let flags = prm[7];
    let height = wf(prm, 8);
    let d = sub(target, pos);
    let mut t = 0.01f32;
    if flags & 2 != 0 {
        let l = -RwV3dLength(g);
        let k = 1.0 / l;
        let n = scale(g, k);
        let h = d[2] * n[2] + d[0] * n[0] + d[1] * n[1];
        let vt = tv[2] * n[2] + tv[0] * n[0] + tv[1] * n[1];
        let v0 = (l * -2.0 * height).sqrt();
        let hh = h + height;
        t = v0;
        if 0.0 < hh {
            t = vt + (l * -2.0 * hh).sqrt();
            if t <= v0 {
                t = v0;
            }
        }
        t -= vt;
        t = (t + (t * t + h * 2.0 * l).sqrt()) / -l;
    }
    if flags & 0xc != 0 {
        log::warn!("APIphysic::JumpOnCoor: flags {flags:#x} need DTQuartic::Solve (unported)");
    }
    let half = scale(g, t * 0.5);
    let mut v = sub(add(scale(d, 1.0 / t), tv), half);
    if flags & 1 != 0 {
        let mx = wf(prm, 6);
        let l2 = v[2] * v[2] + v[0] * v[0] + v[1] * v[1];
        if mx * mx < l2 {
            v = scale(v, mx / l2.sqrt());
        }
    }
    if uid != 0 {
        if let Some(p) = m.PhysicMut(id) {
            p.velocity = v;
            p.falling_timer = p.falling_delay;
        }
    }
}

/// CheckRayIntersection(in, out) (0x001f7fa0) with the guest RayIntersectOut packing (15 words):
/// +0 pos, +0xc normal; object hit: +0x18 owner uid, +0x1c prim tag; map hit: +0x18 material
/// word 5 (two u16), +0x1c..+0x3c the 8 material words. None = no hit / callback mode (the
/// C++ leaves `out` untouched).
pub fn CheckRayIntersectionOut(m: &mut EFPhysicMgr, rin: EFRayIntersectIn, w: &dyn PhysicWorld, cs: &mut dyn CollisionSystem) -> Option<Vec<u32>> {
    let r = CheckRayIntersection(m, rin, w, cs);
    let mut o = vec![0u32; 15];
    put_v(&mut o, 0, r.pos);
    put_v(&mut o, 3, r.normal);
    match r.kind {
        1 => {
            let p = m.Physic(r.physic)?;
            o[6] = p.owner;
            o[7] = p.model.prims.get(r.prim).and_then(|q| q.as_ref()).map_or(0, |q| q.base.tag);
            o.truncate(8);
        }
        2 => {
            let (mi, pi) = r.map_prim?;
            let map = m.maps.get(mi)?;
            let mat = map.PrimMaterial(pi);
            o[6] = map.MaterialWord(mat, 5);
            for i in 0..8 {
                o[7 + i] = map.MaterialWord(mat, i);
            }
        }
        _ => return None,
    }
    Some(o)
}

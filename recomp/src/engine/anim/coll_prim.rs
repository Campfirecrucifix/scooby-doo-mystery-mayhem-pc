//! WP-J16: EFHAnimNode::UpdateAnimPrimColl (0x002316d0) + EFHAnimNode::GetCollPrimData
//! (0x00232040) + EF3dObj::GetDefaultCollPrimAttrib(uint, EFAnimCollPrim&) (0x00223d60), 1:1.
//!
//! The anim's collision-prim commands (EFCollisionPrimAnim of the EFHAnimRes) build, every
//! update, the node's EFAnimCollPrim chain (node data block +0x10 -> +0x10; here a
//! `CollPrimList`). The root node's chain is applied to the EFPhysic collision model by
//! EFAnimNode::SetCollPrimData (physic::anim_coll). This is how the anims move the hero's
//! collision volume relative to its frame (ledge climb, crawl, ...).
#![allow(non_snake_case)]

use super::coll_blend::{CollPrimList, EFAnimBaseCollPrim};
use super::res::EFCPAnimCmdPair;
use crate::engine::rw::clump::ApCollisionPrimitive;

/// What the C++ reads through the node's EF3dObj (+0x2c / +0x28): the EFPhysic collision model
/// prim kinds (EFObjCollisionPrim +4, by prim index) and the clump's ApCollisionPrimitive records
/// (EF3dObj::GetDefaultCollPrimAttrib). None on the node = the object has no EFPhysic.
#[derive(Debug, Clone, Default)]
pub struct CollCtx {
    pub kinds: Vec<u32>,
    pub defaults: Vec<ApCollisionPrimitive>,
}

/// EF3dObj::GetDefaultCollPrimAttrib(uint, EFAnimCollPrim&) (0x00223d60).
/// quirk: a missing clump record is a NULL deref in the C++ (nothing written here).
pub fn GetDefaultCollPrimAttrib(ctx: &CollCtx, index: u32, rec: &mut EFAnimBaseCollPrim) {
    let Some(p) = ctx.defaults.get(index as usize) else { return };
    let x = |i: usize| p.extra.get(i).copied().unwrap_or(0);
    let set = |rec: &mut EFAnimBaseCollPrim, off: usize, w: u32| rec.set_f(off, f32::from_bits(w));
    match p.head[0] {
        2 => (0..3).for_each(|i| set(rec, 0x4c + 4 * i, x(i))),
        1 => (0..4).for_each(|i| set(rec, 0x4c + 4 * i, x(i))),
        0 => set(rec, 0x4c, x(0)),
        _ => {}
    }
    for i in 0..3 {
        rec.set_f(0x40 + 4 * i, p.reals[i]);
    }
    for i in 0..11 {
        set(rec, 0x14 + 4 * i, p.attribs[i] as u32);
    }
    rec.flags &= !2;
}

/// EFHAnimNode::GetCollPrimData(uint index, CommandType_e cmd, DTWorkMemory&): the record of
/// (type, index) in the node's chain, created (and linked) when missing. Record type: cmd 8 -> 0
/// (clipping bsphere), cmd 2 -> 1 (physic bsphere), else by the physic prim kind: sphere 0 -> 2,
/// swept sphere 1 -> 4, bbox 2 -> 3. Returns the index in `list.prims`.
pub fn GetCollPrimData(list: &mut CollPrimList, ctx: &CollCtx, index: u32, cmd: u16) -> usize {
    let (ty, size) = match cmd {
        8 => (0u8, 0x24u32),
        2 => (1, 0x24),
        _ => match ctx.kinds.get(index as usize).copied() {
            Some(2) => (3, 0x58),
            Some(1) => (4, 0x5c),
            Some(0) => (2, 0x50),
            // quirk: other kinds keep the previous (uninitialised) type/size registers in the C++
            _ => (2, 0x50),
        },
    };
    if let Some(i) = list.prims.iter().position(|p| p.ty == ty && p.tag == index) {
        return i;
    }
    let mut r = EFAnimBaseCollPrim { ty, tag: index, flags: if cmd == 9 { 2 } else { 0 }, size, body: vec![0; (size as usize - 0x14) / 4] };
    if cmd != 9 && matches!(ty, 2 | 3 | 4) {
        GetDefaultCollPrimAttrib(ctx, index, &mut r);
    }
    list.count += 1;
    if list.prims.is_empty() {
        list.prims.push(r);
        return 0;
    }
    // link: type 1 goes right after the head (after an existing type-1 second record);
    // others are inserted by ascending index after the head / type-1 record
    let mut at = 1;
    if list.prims.get(1).is_some_and(|p| p.ty == 1) {
        at = 2;
    }
    if ty != 1 {
        while at < list.prims.len() && list.prims[at].tag < index {
            at += 1;
        }
    }
    list.prims.insert(at, r);
    at
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/// EFHAnimNode::UpdateAnimPrimColl (0x002316d0). `time`/`duration`/`state` = node +0x38/+0x3c/
/// +0x34; `cmds` = the EFHAnimRes collision-prim commands (sorted by time); `ctx` = None when the
/// EF3dObj has no EFPhysic (only the clipping bsphere record is built then).
pub fn UpdateAnimPrimColl(list: &mut CollPrimList, cmds: &[EFCPAnimCmdPair], time: f32, duration: f32, state: u32, ctx: Option<&CollCtx>) {
    if cmds.is_empty() {
        return;
    }
    let empty = CollCtx::default();
    // the chain is rebuilt every update
    list.count = 0;
    list.prims.clear();
    let f = |c: &EFCPAnimCmdPair, i: usize| f32::from_bits(c.data.as_ref().and_then(|d| d.get(i).copied()).unwrap_or(0));
    let v = |c: &EFCPAnimCmdPair, i: usize| [f(c, i), f(c, i + 1), f(c, i + 2)];
    if let Some(c) = cmds.iter().find(|c| c.cmd == 8) {
        let i = GetCollPrimData(list, ctx.unwrap_or(&empty), c.a as u32, 8);
        let r = &mut list.prims[i];
        r.set_f(0x20, f(c, 3));
        r.set_v(0x14, v(c, 0));
    }
    let Some(ctx) = ctx else { return };
    for c in cmds {
        let t = c.cmd;
        if t != 2 && t != 8 && ctx.kinds.get(c.a as usize).copied() == Some(3) {
            continue; // polygon prims are not animated
        }
        if time < c.time {
            break;
        }
        match t {
            9 => {
                GetCollPrimData(list, ctx, c.a as u32, 9);
            }
            1 | 8 => {}
            0 => {
                let i = GetCollPrimData(list, ctx, c.a as u32, 0);
                // the C++ reads the attrib index as an int word (Ghidra types it float: `(int)*pf`;
                // the data holds 2 / 5 ..., not float bits)
                let w = c.data.as_ref().and_then(|d| d.first().copied()).unwrap_or(0) as i32;
                list.prims[i].set_f((0x14 + w * 4) as usize, f(c, 1));
            }
            _ => {
                let mut end = f(c, 0);
                let open = end <= 0.0;
                if open {
                    end = duration;
                }
                if !(c.time <= time && time <= end) {
                    continue;
                }
                let k = if open { 0.0 } else { (time - c.time) / (end - c.time) };
                let i = GetCollPrimData(list, ctx, c.a as u32, t);
                let r = &mut list.prims[i];
                match t {
                    2 => {
                        r.set_v(0x14, lerp3(v(c, 1), v(c, 5), k));
                        r.set_f(0x20, f(c, 4) + k * (f(c, 8) - f(c, 4)));
                    }
                    3 => {
                        r.set_v(0x40, lerp3(v(c, 1), v(c, 4), k));
                        if state & 0x4000_0000 != 0 {
                            r.flags = 1;
                        }
                    }
                    4 | 6 => r.set_f(0x4c, f(c, 1) + k * (f(c, 2) - f(c, 1))),
                    5 => r.set_v(0x50, lerp3(v(c, 1), v(c, 4), k)),
                    7 => r.set_v(0x4c, lerp3(v(c, 1), v(c, 4), k)),
                    _ => {}
                }
            }
        }
    }
}

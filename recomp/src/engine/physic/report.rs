//! EFCollisionReport (Classes/EF/EFCollisionReport.cpp): one collision event, kept in t-sorted
//! singly linked lists (+0x28 next). Lists here are `Vec<usize>` of indices into a report arena
//! (C++ pointer chains replaced; same ordering semantics).
#![allow(non_snake_case)]

use super::objcoll_prim::V3;

#[derive(Debug, Clone, Copy, Default)]
pub struct EFCollisionReport {
    /// +0x00 this-side EFCollisionStage / +0x10 its primitive index
    pub stage: u32,
    pub prim: u32,
    /// +0x0c other side (EFPhysic**), +0x24 its primitive index
    pub other: u32,
    pub other_prim: u32,
    /// +0x20 t of the hit (sort key)
    pub t: f32,
}

/// Primitive view used by CheckNotify: +0x8 tag, +0xc flags (4 notify this, 8 notify other),
/// +0x28..+0x34 four user words.
#[derive(Debug, Clone, Copy, Default)]
pub struct ReportPrim {
    pub tag: u32,
    pub flags: u32,
    pub user: [u32; 4],
}

/// The 0x38-byte message sent with EFInstance::Notify ids 0x17 then 3:
/// +0 hit flag (1 = sender's primitive collided), +4 tag of the reporting prim, +8 other instance uid,
/// +0xc other prim tag, +0x10 user words, +0x20 pos A, +0x2c pos B.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CollisionNotifyMsg {
    pub hit: u32,
    pub tag: u32,
    pub other_uid: u32,
    pub other_tag: u32,
    pub user: [u32; 4],
    pub p0: V3,
    pub p1: V3,
}

impl EFCollisionReport {
    /// InsertTo(list): insert before the first report with t >= self.t (after equal-less ones).
    pub fn InsertTo(arena: &[EFCollisionReport], me: usize, list: &mut Vec<usize>) {
        let pos = list.iter().position(|&r| !(arena[r].t < arena[me].t)).unwrap_or(list.len());
        list.insert(pos, me);
    }
    /// MergeInto(list): walks `chain` and `list` together; each chain element is inserted at the
    /// cursor after skipping list elements with smaller t (cursor never moves back, as in the C++).
    pub fn MergeInto(arena: &[EFCollisionReport], chain: &[usize], list: &mut Vec<usize>) {
        let mut cur = 0;
        for &me in chain {
            while cur < list.len() && arena[list[cur]].t < arena[me].t {
                cur += 1;
            }
            list.insert(cur, me);
            cur += 1;
        }
    }
    /// CheckNotify(p0, p1) (0x001dc6d0). `a` = this side (prim +0x40[prim], instance uid of
    /// stage +0x7c[prim]->+0xac), `b` = other side (NULL other -> nothing). Returns the
    /// EFInstance::Notify calls in order as (target uid, message id, msg).
    pub fn CheckNotify(a: (u32, ReportPrim), b: Option<(u32, Option<ReportPrim>)>, p0: V3, p1: V3) -> Vec<(u32, u32, CollisionNotifyMsg)> {
        let mut out = vec![];
        let Some((buid, bprim)) = b else { return out };
        let (auid, ap) = a;
        // quirk: a NULL other prim array gives prim NULL, then +0xc is read through NULL (crash); here 0
        let bp = bprim.unwrap_or_default();
        let mut send = |to: u32, m: CollisionNotifyMsg| {
            out.push((to, 0x17, m));
            out.push((to, 3, m));
        };
        if ap.flags & 0xc != 0 {
            if ap.flags & 4 != 0 {
                send(auid, CollisionNotifyMsg { hit: 1, tag: ap.tag, other_uid: buid, other_tag: bp.tag, user: ap.user, p0, p1 });
            }
            if ap.flags & 8 != 0 {
                send(buid, CollisionNotifyMsg { hit: 0, tag: bp.tag, other_uid: auid, other_tag: ap.tag, user: ap.user, p0: p1, p1: p0 });
            }
        }
        if bp.flags & 0xc != 0 {
            if bp.flags & 4 != 0 {
                send(buid, CollisionNotifyMsg { hit: 1, tag: bp.tag, other_uid: auid, other_tag: ap.tag, user: bp.user, p0: p1, p1: p0 });
            }
            if bp.flags & 8 != 0 {
                send(auid, CollisionNotifyMsg { hit: 0, tag: ap.tag, other_uid: buid, other_tag: bp.tag, user: bp.user, p0, p1 });
            }
        }
        out
    }
}

//! WP-D4: EFAnimBlendNode::BlendPrimColl (0x0022fd30) + Blend2CollPrimData (0x002303a0), 1:1.
//!
//! Memory model: the C++ chains EFAnimBaseCollPrim records (+0x10 next) allocated in the
//! DTWorkMemory of the anim tree; here a node's collision-prim output is a `CollPrimList`
//! (Vec in chain order). Records the C++ links without copying (cross-fade: the second child's
//! records, and the first child's when the type is not 1) are cloned; quirk not reproduced: the
//! C++ rewrites those records' +0x10 links, i.e. it re-chains the children's own lists.
//! Words of a fresh DTWorkMemory record that Blend2CollPrimData does not write are uninitialised
//! in the C++; they start as a copy of the first prim here.
#![allow(non_snake_case)]

use super::mgr::EFAnimBlendNode;

/// EFAnimBaseCollPrim (variable size record)
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EFAnimBaseCollPrim {
    /// +0 prim type (0 sphere, 1 swept sphere, 2 bbox, 3/4 per EFCollisionModel kinds)
    pub ty: u8,
    /// +4 prim tag/id (lists are sorted by it)
    pub tag: u32,
    /// +8 flags (bit 1: take the EF3dObj default attributes, EF3dObj::GetDefaultCollPrimAttrib)
    pub flags: u32,
    /// +0xc record size in bytes
    pub size: u32,
    /// +0x14.. record body as raw words (word i = byte offset 0x14 + 4*i)
    pub body: Vec<u32>,
}

impl EFAnimBaseCollPrim {
    pub fn f(&self, off: usize) -> f32 {
        f32::from_bits(self.body.get((off - 0x14) / 4).copied().unwrap_or(0))
    }
    pub fn v(&self, off: usize) -> [f32; 3] {
        [self.f(off), self.f(off + 4), self.f(off + 8)]
    }
    pub fn set_f(&mut self, off: usize, x: f32) {
        let i = (off - 0x14) / 4;
        if self.body.len() <= i {
            self.body.resize(i + 1, 0);
        }
        self.body[i] = x.to_bits();
    }
    pub fn set_v(&mut self, off: usize, v: [f32; 3]) {
        for j in 0..3 {
            self.set_f(off + 4 * j, v[j]);
        }
    }
}

/// A node's collision-prim data block (node+0x10 -> +0x10): +8 count, +0xc chain head.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CollPrimList {
    /// +8
    pub count: u32,
    /// +0xc chain
    pub prims: Vec<EFAnimBaseCollPrim>,
    /// the chain head is the EF3dObj's own list (EF3dObj+0x50)
    pub is_obj_list: bool,
}

impl EFAnimBlendNode {
    /// EFAnimBlendNode::Blend2CollPrimData (0x002303a0). `default_attrib(tag, prim)` =
    /// EF3dObj::GetDefaultCollPrimAttrib (applied to the inputs in place, as in the C++).
    pub fn Blend2CollPrimData(
        &self,
        p1: &mut EFAnimBaseCollPrim,
        p2: &mut EFAnimBaseCollPrim,
        w: f32,
        default_attrib: &mut dyn FnMut(u32, &mut EFAnimBaseCollPrim),
    ) -> EFAnimBaseCollPrim {
        if p1.flags & 2 != 0 {
            default_attrib(p1.tag, p1);
        }
        if p2.flags & 2 != 0 {
            default_attrib(p2.tag, p2);
        }
        if w == 0.0 {
            return p1.clone();
        }
        if w == 1.0 {
            return p2.clone();
        }
        let mut o = p1.clone();
        o.tag = p1.tag;
        o.ty = p1.ty;
        o.size = p1.size;
        o.flags = p1.flags | p2.flags;
        if p1.ty == 0 || p1.ty == 1 {
            // bounding spheres: smallest sphere containing both, grown by the weight
            let c1 = p1.v(0x14);
            let c2 = p2.v(0x14);
            let d = [c2[0] - c1[0], c2[1] - c1[1], c2[2] - c1[2]];
            let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            let (r1, r2) = (p1.f(0x20), p2.f(0x20));
            if len == 0.0 {
                o.set_v(0x14, c1);
                o.set_f(0x20, r1 + w * (r2 - r1));
            } else {
                // quirk: the weight is not used here, the result is the enclosing sphere
                let r = (r2 + len + r1) * 0.5;
                let k = (r - r1) / len;
                o.set_v(0x14, [d[0] * k + c1[0], d[1] * k + c1[1], d[2] * k + c1[2]]);
                o.set_f(0x20, r);
            }
        } else {
            let lerp3 = |a: [f32; 3], b: [f32; 3]| [a[0] + (b[0] - a[0]) * w, a[1] + (b[1] - a[1]) * w, a[2] + (b[2] - a[2]) * w];
            if p1.ty == 3 {
                o.set_v(0x4c, lerp3(p1.v(0x4c), p2.v(0x4c)));
            } else if p1.ty == 4 {
                o.set_v(0x50, lerp3(p1.v(0x50), p2.v(0x50)));
                o.set_f(0x4c, p1.f(0x4c) + w * (p2.f(0x4c) - p1.f(0x4c)));
            } else if p1.ty == 2 {
                o.set_f(0x4c, p1.f(0x4c) + w * (p2.f(0x4c) - p1.f(0x4c)));
            }
            o.set_v(0x40, lerp3(p1.v(0x40), p2.v(0x40)));
            // quirk (disasm 0x00230840: lw/lw/or/sw): words +0x14..+0x40 are OR-ed, not blended
            for i in 0..11 {
                let a = p1.body.get(i).copied().unwrap_or(0);
                let b = p2.body.get(i).copied().unwrap_or(0);
                if o.body.len() <= i {
                    o.body.resize(i + 1, 0);
                }
                o.body[i] = a | b;
            }
        }
        o
    }

    /// EFAnimBlendNode::BlendPrimColl (0x0022fd30), called at the end of Update.
    /// `has_coll` = owner EF3dObj (+0x2c)->+0x10->+0x28 != 0; `cnt` = this+0x18 countdown;
    /// `first`/`second` = the children's lists (type 1: `first` is the snapshot +0x54 and
    /// `second` the only child). Returns the node's new list (None when `has_coll` is 0) and
    /// whether EFAnimNode::SetCollPrimData must be called (head == EF3dObj+0x50).
    pub fn BlendPrimColl(
        &mut self,
        has_coll: bool,
        cnt: f32,
        first: &CollPrimList,
        second: &CollPrimList,
        default_attrib: &mut dyn FnMut(u32, &mut EFAnimBaseCollPrim),
    ) -> Option<(CollPrimList, bool)> {
        if !has_coll {
            return None;
        }
        // iVar5 = old (first child or snapshot), iVar7 = new
        let (a, b) = (first, second);
        let mut out = CollPrimList::default();
        if !self.additive {
            // the weight state machine with dt = 0 (identical to Update's EFAnimBlendNode weight)
            let w = self.Weight(0.0, cnt);
            let mut av: Vec<EFAnimBaseCollPrim> = a.prims.clone();
            let mut bv: Vec<EFAnimBaseCollPrim> = b.prims.clone();
            // quirk: the heads are used without a NULL check (port: empty list -> nothing)
            if av.is_empty() || bv.is_empty() {
                return Some((out, false));
            }
            let (h1, _) = av.split_at_mut(1);
            let (h2, _) = bv.split_at_mut(1);
            let head = self.Blend2CollPrimData(&mut h1[0], &mut h2[0], w, default_attrib);
            out.prims.push(head);
            out.count = 1;
            let (mut ia, mut ib) = (1usize, 1usize);
            let at = |v: &Vec<EFAnimBaseCollPrim>, i: usize| v.get(i).map(|p| p.ty);
            // second slot: type-1 prims (swept spheres) are paired first
            let dest;
            if at(&av, ia) == Some(1) && at(&bv, ib) == Some(1) {
                let (mut x, mut y) = (av[ia].clone(), bv[ib].clone());
                dest = Some(self.Blend2CollPrimData(&mut x, &mut y, w, default_attrib));
                ia += 1;
                ib += 1;
            } else if at(&av, ia) != Some(1) {
                dest = if at(&bv, ib) == Some(1) {
                    ib += 1;
                    Some(bv[ib - 1].clone())
                } else {
                    None
                };
            } else {
                // type 1 blends memcpy the record into work memory, others link it (same data)
                dest = Some(av[ia].clone());
                ia += 1;
            }
            if let Some(d) = dest {
                out.prims.push(d);
                out.count += 1;
            }
            // merge of the remaining chains, both sorted by tag
            loop {
                let (sa, sb) = (av.get(ia).is_some(), bv.get(ib).is_some());
                if !sa && !sb {
                    break;
                }
                if !sa || !sb {
                    if !sa {
                        let n = bv.len() - ib;
                        out.prims.extend(bv.drain(ib..));
                        out.count += n as u32;
                    } else {
                        let n = av.len() - ia;
                        out.prims.extend(av.drain(ia..));
                        out.count += n as u32;
                    }
                    break;
                }
                let p = if av[ia].ty == bv[ib].ty && av[ia].tag == bv[ib].tag {
                    let (mut x, mut y) = (av[ia].clone(), bv[ib].clone());
                    ia += 1;
                    ib += 1;
                    self.Blend2CollPrimData(&mut x, &mut y, w, default_attrib)
                } else if av[ia].tag < bv[ib].tag {
                    ia += 1;
                    av[ia - 1].clone()
                } else {
                    ib += 1;
                    bv[ib - 1].clone()
                };
                out.prims.push(p);
                out.count += 1;
            }
        } else {
            // additive: the node's block takes the first list as is (+8 count, +0xc head)
            out = a.clone();
        }
        let set = out.is_obj_list;
        Some((out, set))
    }
}

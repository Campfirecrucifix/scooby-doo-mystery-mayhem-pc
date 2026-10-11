//! ApiPathFinding custom library (`Game Logic Modules/Globals/Lib/apipathfinding__<sha8>/ApiPathFinding.c`,
//! 18 functions). Registers the 10-slot table "ApiPathFinding" (0x28 bytes): node graphs with a
//! precomputed next-hop table ("solution") per path.
//!
//! All 17 builds share the same code (two Ghidra spellings of GetSolution, same semantics). They differ
//! only in the data image: the path tables (one per level section) and their addresses ([`BUILDS`]).
//! The image (.text bytes 0x10000..FindPath, relocated) is read from the module on the disc image
//! ([`build_init`]).
//!
//! mPathsTable[n] (0x48 each): +0 i16 node count, +2 i16 edge count, +4 nodes (0x10 each: x, y, z,
//! radius), +8 edges (4 each: i16 a, i16 b), +0xc char name[60]. mSolutions[n]: i16[count*count]
//! next node from `from` to `to` at `from*count + to`.
//! Caller's path record (`ctx`): +4 i16 path id, +6 i16 current node, +8 i16 target node,
//! +0xc flag (non-zero: 2D distance on x/z, and y=0 in returned node positions).
//! Local data (`pLocalDataApiCPathFinding`, 0xc): +0 lib object id, +8 last path record used.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]
use super::{lib_build, lib_data};
use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "apipathfinding",
    data_size: 0x11e60,
    ctor: apipathfindingConstructor,
    dtor: apipathfindingDestructor,
    register: Some(apipathfindingRegister),
    init: &[],
    build_init: Some(build_init),
};

/// (sha8, mPathsTable, mSolutions, pLocalDataApiCPathFinding, path count (GetPathID loop bound + 1))
type Build = (&'static str, u32, u32, u32, i32);
pub static BUILDS: [Build; 17] = [
    ("0af91b25", 0x102a0, 0x10a70, 0x10a90, 1),
    ("0e4e4e94", 0x10558, 0x11258, 0x11280, 3),
    ("3fd08701", 0x10138, 0x102b0, 0x102d4, 2),
    ("62b7cd2c", 0x11288, 0x21178, 0x2119c, 2),
    ("7019bc0d", 0x107b0, 0x14d00, 0x14d20, 1),
    ("703f6825", 0x10500, 0x11cf0, 0x11d10, 1),
    ("72f83ec2", 0x10f88, 0x13d28, 0x13d60, 7),
    ("7c1ab564", 0x10c08, 0x14660, 0x14690, 5),
    ("81f35da4", 0x108a0, 0x15f10, 0x15f30, 1),
    ("9304f4cf", 0x10728, 0x12228, 0x12254, 4),
    ("a36cbdf8", 0x101f0, 0x10600, 0x10620, 1),
    ("b059cace", 0x102a0, 0x10a70, 0x10a90, 1),
    ("c8f6649d", 0x10340, 0x10b40, 0x10b64, 2),
    ("ef91e387", 0x11188, 0x18f50, 0x18f7c, 4),
    ("f48040de", 0x102d8, 0x10ba8, 0x10bc8, 1),
    ("fc465d8d", 0x10388, 0x112f0, 0x11310, 1),
    ("fd44884c", 0x107f0, 0x124f0, 0x12518, 3),
];
pub fn build_index(sha8: &str) -> Option<usize> {
    BUILDS.iter().position(|b| b.0 == sha8)
}

/// The data image is the module's .text bytes 0x10000..FindPath (globals live at the start of .text),
/// read from the module on the user's disc image (WP-X1); R_MIPS_32 words become data-relative pointers.
fn build_init(m: &mut Mem, data: Ptr, b: usize) {
    use crate::engine::gamedata::{module_elf, Elf};
    let elf = Elf::parse(module_elf(BUILDS[b].0));
    let ti = elf.index(".text");
    let text = elf.secs[ti].data;
    let end = elf.symtab().iter().find(|s| s.name == "FindPath").expect("apipathfinding: no FindPath").value as usize;
    let img = &text[..end];
    for (i, c) in img.chunks(4).enumerate() {
        let mut w = [0u8; 4];
        w[..c.len()].copy_from_slice(c);
        m.w32(data + i as u32 * 4, u32::from_le_bytes(w));
    }
    for (si, sec) in elf.secs.iter().enumerate() {
        if sec.ty != 9 && sec.ty != 4 {
            continue;
        }
        let syms = elf.syms(sec.link);
        for r in elf.rels(si) {
            let o = r.offset as usize;
            if o >= end {
                continue;
            }
            assert!(r.ty == 2 && syms[r.sym].shndx as usize == ti, "apipathfinding {}: reloc at {o:#x}", BUILDS[b].0);
            let add = r.addend.map_or_else(|| u32::from_le_bytes(img[o..o + 4].try_into().unwrap()), |a| a as u32);
            m.w32(data + o as u32, data + syms[r.sym].value + add);
        }
    }
}

fn bd(e: &Engine) -> &'static Build {
    &BUILDS[lib_build(e, &MODULE)]
}
/// pLocalDataApiCPathFinding
fn local(e: &Engine) -> Ptr {
    let b = bd(e);
    e.r32(lib_data(e, &MODULE, b.3))
}
/// &mPathsTable[id]
fn path(e: &Engine, id: i32) -> Ptr {
    let b = bd(e);
    lib_data(e, &MODULE, b.1).wrapping_add((id as u32).wrapping_mul(0x48))
}
fn set_cur(e: &mut Engine, ctx: Ptr) {
    let l = local(e);
    e.w32(l + 8, ctx);
}
/// path id of the last path record (*(short*)(pLocal[2] + 4))
fn cur_id(e: &Engine) -> i32 {
    let l = local(e);
    e.r16(e.r32(l + 8) + 4) as i16 as i32
}
fn i16at(e: &Engine, a: Ptr) -> i32 {
    e.r16(a) as i16 as i32
}
/// node `i` of path table `p` (address)
fn node(e: &Engine, p: Ptr, i: i32) -> Ptr {
    e.r32(p + 4).wrapping_add((i as u32).wrapping_mul(0x10))
}
fn vec3(e: &Engine, a: Ptr) -> [f32; 3] {
    [e.rf(a), e.rf(a + 4), e.rf(a + 8)]
}
fn zero3(e: &mut Engine, out: Ptr) {
    e.w32(out, 0);
    e.w32(out + 4, 0);
    e.w32(out + 8, 0);
}
/// GetPosition(obj, ALL) into a Rust vector (stack buffer released at once)
fn pos(e: &mut Engine, obj: u32) -> [f32; 3] {
    let sp = e.sp();
    let p = e.alloca(0x10);
    APIgameobj::GetPosition(e, obj, 0x7fff_ffff, p);
    let v = vec3(e, p);
    e.set_sp(sp);
    v
}

/// 0x00014a28 (72f83ec2)
pub fn FindPath(e: &mut Engine, from: Ptr, to: Ptr, out: Ptr, ctx: i32) -> u32 {
    let ctx = ctx as Ptr;
    let a = vec3(e, from);
    let b = vec3(e, to);
    let p = path(e, i16at(e, ctx + 4));
    set_cur(e, ctx);
    let flat = e.r32(ctx + 0xc);
    let n0 = FindNode(e, a, flat);
    let n1 = FindNode(e, b, flat);
    if i16at(e, ctx + 4) == -1 || n0 == -1 || n1 == -1 || GetSolution(e, n0, n1) == -1 {
        zero3(e, out);
        e.w16(ctx + 6, 0xffff);
        e.w16(ctx + 8, 0xffff);
        return 0;
    }
    e.w16(ctx + 8, n1 as u16);
    let nd = node(e, p, n0);
    let v = e.r32(nd);
    e.w32(out, v);
    let y = if e.r32(ctx + 0xc) == 0 { e.r32(nd + 4) } else { 0 };
    e.w32(out + 4, y);
    let z = e.r32(nd + 8);
    e.w32(out + 8, z);
    e.w16(ctx + 6, n0 as u16);
    if n0 == n1 { 2 } else { 1 }
}

/// 0x00014cc0
pub fn NextPathNode(e: &mut Engine, out: Ptr, ctx: i32) -> u32 {
    let ctx = ctx as Ptr;
    let p = path(e, i16at(e, ctx + 4));
    set_cur(e, ctx);
    let cur = i16at(e, ctx + 6);
    let tgt = i16at(e, ctx + 8);
    if cur == -1 || tgt == -1 {
        zero3(e, out);
        return 0;
    }
    if cur == tgt {
        return 2;
    }
    let s = GetSolution(e, cur, tgt);
    e.w16(ctx + 6, s as u16);
    let nd = node(e, p, i16at(e, ctx + 6));
    let v = e.r32(nd);
    e.w32(out, v);
    let y = if e.r32(ctx + 0xc) == 0 { e.r32(nd + 4) } else { 0 };
    e.w32(out + 4, y);
    let z = e.r32(nd + 8);
    e.w32(out + 8, z);
    1
}

/// 0x00014e88 SELF and `obj` in different nodes
pub fn NeedPathFinding(e: &mut Engine, obj: u32, ctx: i32) -> u8 {
    let ctx = ctx as Ptr;
    let flat = e.r32(ctx + 0xc);
    set_cur(e, ctx);
    let o = pos(e, obj);
    let s = pos(e, 0x2a00_0000);
    let ns = FindNode(e, s, flat);
    let no = FindNode(e, o, flat);
    (ns != no) as u8
}

/// 0x00014fa8
pub fn SetPath(e: &mut Engine, name: i32, ctx: i32) -> u8 {
    let ctx = ctx as Ptr;
    set_cur(e, ctx);
    e.w16(ctx + 8, 0xffff);
    e.w16(ctx + 6, 0xffff);
    let id = GetPathID(e, name as Ptr);
    e.w16(ctx + 4, id as u16);
    (id != -1) as u8
}

/// 0x00015068
pub fn IsOnPath(e: &mut Engine, obj: u32, ctx: i32) -> u8 {
    let ctx = ctx as Ptr;
    let flat = e.r32(ctx + 0xc);
    set_cur(e, ctx);
    let o = pos(e, obj);
    (FindNode(e, o, flat) != -1) as u8
}

/// 0x00015138
pub fn IsOnPath_GetNode(e: &mut Engine, obj: u32, out: Ptr, ctx: i32) -> u8 {
    let ctx = ctx as Ptr;
    let flat = e.r32(ctx + 0xc);
    set_cur(e, ctx);
    let o = pos(e, obj);
    let n = FindNode(e, o, flat);
    e.wi32(out, n);
    (e.ri32(out) != -1) as u8
}

/// 0x00015218 quirk: the node table comes from the PREVIOUS path record (read before pLocal[2] = ctx)
pub fn IsTargetInSameNode(e: &mut Engine, at: Ptr, ctx: i32) -> u32 {
    let ctx = ctx as Ptr;
    let a = vec3(e, at);
    let p = path(e, cur_id(e));
    let flat = e.r32(ctx + 0xc);
    set_cur(e, ctx);
    let t = i16at(e, ctx + 8);
    if t == -1 {
        return 0;
    }
    let n = vec3(e, node(e, p, t));
    let d = Distance(e, n, a, flat);
    let r = e.rf(node(e, p, i16at(e, ctx + 8)) + 0xc);
    (d <= r) as u32
}

/// 0x000153c0 up to 10 nodes linked to `n` by an edge
pub fn GetNeighborNodes(e: &mut Engine, n: i32, out: i32, ctx: u32) -> i32 {
    let out = out as Ptr;
    set_cur(e, ctx);
    let p = path(e, cur_id(e));
    let edges = i16at(e, p + 2);
    let tab = e.r32(p + 8);
    let mut k = 0;
    let mut i = 0;
    while k < 10 && i < edges {
        let ea = tab.wrapping_add((i as u32).wrapping_mul(4));
        if i16at(e, ea) == n {
            let v = i16at(e, ea + 2);
            e.wi32(out + k as u32 * 4, v);
            k += 1;
        } else if i16at(e, ea + 2) == n {
            let v = i16at(e, ea);
            e.wi32(out + k as u32 * 4, v);
            k += 1;
        }
        i += 1;
    }
    k
}

/// 0x00015570 up to 10 nodes within `range` of `at`
pub fn GetNeighborNodesRange(e: &mut Engine, range: f32, at: Ptr, out: i32, ctx: i32) -> i32 {
    let (out, ctx) = (out as Ptr, ctx as Ptr);
    let a = vec3(e, at);
    set_cur(e, ctx);
    let p = path(e, cur_id(e));
    let count = i16at(e, p);
    let mut k = 0;
    let mut i = 0;
    while k < 10 && i < count {
        let n = vec3(e, node(e, p, i));
        let flat = e.r32(ctx + 0xc);
        if Distance(e, a, n, flat) <= range {
            e.wi32(out + k as u32 * 4, i);
            k += 1;
        }
        i += 1;
    }
    k
}

/// 0x00015748
pub fn GetNode(e: &mut Engine, n: i32, out: Ptr, ctx: u32) -> u8 {
    set_cur(e, ctx);
    let p = path(e, cur_id(e));
    let count = i16at(e, p);
    if count <= n {
        zero3(e, out);
    } else {
        let nd = node(e, p, n);
        for o in [0, 4, 8] {
            let v = e.r32(nd + o);
            e.w32(out + o, v);
        }
    }
    (count > n) as u8
}

/// 0x00015870 nearest node (< 3000) whose radius contains `a`, -1 if none
fn FindNode(e: &mut Engine, a: [f32; 3], flat: u32) -> i32 {
    let p = path(e, cur_id(e));
    let mut best = -1;
    let mut bd = 3000.0f32;
    let mut i = 0;
    while i < i16at(e, path(e, cur_id(e))) {
        let n = vec3(e, node(e, p, i));
        let d = Distance(e, n, a, flat);
        if d < bd && d <= e.rf(node(e, p, i) + 0xc) {
            best = i;
            bd = d;
        }
        i += 1;
    }
    best
}

/// 0x00015a58 3D, or x/z only when `flat` != 0
fn Distance(e: &mut Engine, a: [f32; 3], b: [f32; 3], flat: u32) -> f32 {
    let s = if flat == 0 {
        (a[0] - b[0]) * (a[0] - b[0]) + (a[1] - b[1]) * (a[1] - b[1]) + (a[2] - b[2]) * (a[2] - b[2])
    } else {
        (a[0] - b[0]) * (a[0] - b[0]) + (a[2] - b[2]) * (a[2] - b[2])
    };
    APImath::sqrtf(e, s)
}

/// 0x00015bd0 next node from `from` towards `to` (i16, -1 = unreachable)
fn GetSolution(e: &mut Engine, from: i32, to: i32) -> i32 {
    let id = cur_id(e);
    let count = i16at(e, path(e, id));
    let b = bd(e);
    let sol = e.r32(lib_data(e, &MODULE, b.2).wrapping_add((id as u32).wrapping_mul(4)));
    i16at(e, sol.wrapping_add((count.wrapping_mul(from).wrapping_add(to) as u32).wrapping_mul(2)))
}

/// 0x00015c88 index of the path named `name` in mPathsTable, -1 if none
fn GetPathID(e: &mut Engine, name: Ptr) -> i32 {
    let n = bd(e).4;
    for i in 0..n {
        let s = path(e, i) + 0xc;
        if StringCmp(e, name, s) {
            return i;
        }
    }
    -1
}

/// 0x00015d48 equality of two strings (lengths capped at 60)
fn StringCmp(e: &Engine, a: Ptr, b: Ptr) -> bool {
    let len = |p: Ptr| {
        let mut i = 0u32;
        loop {
            if 0x3b < i {
                break i;
            }
            let c = e.r8(p + i);
            i += 1;
            if c == 0 {
                break i;
            }
        }
    };
    let la = len(a);
    let lb = len(b);
    if la.wrapping_sub(1) != lb.wrapping_sub(1) {
        return false;
    }
    (0..lb.wrapping_sub(1)).all(|i| e.r8(a + i) == e.r8(b + i))
}

/// 0x00015ec8 (API slot globals are not mirrored: the LIB trait dispatches straight here)
pub fn apipathfindingRegister(e: &mut Engine, api: Ptr) {
    let n = e.cstr("ApiPathFinding");
    APIRegLib::RegisterLib(e, n, 0x28);
}

/// 0x000160a0 local data; own object: clipping distance -1, 0xa0 flags
pub fn apipathfindingConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let slot = lib_data(e, &MODULE, bd(e).3);
    APIcons::GetLocalData(e, slot, 0xc);
    let id = APIgameobj::GetID(e, 0x2a00_0000, 0);
    let l = local(e);
    e.w32(l, id);
    let id = APIgameobj::GetID(e, 0x2a00_0000, 0);
    let p = e.alloca(0x40);
    APIgameobj::GetClippingParams(e, id, p);
    e.w32(p + 4, 0xbf80_0000);
    e.w32(p + 0xc, 0xa0);
    e.w32(p + 8, 0xa0);
    APIgameobj::SetClippingParams(e, id, p, 1);
    Ok(())
}

/// 0x000161a8
pub fn apipathfindingDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    let n = e.cstr("ApiPathFinding");
    APIRegLib::UnRegisterLib(e, n);
    Ok(())
}

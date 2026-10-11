//! APIparticle_* (Engine API/EFApi_Particle.cpp, table 0xbc bytes) and EFBillboardMgr /
//! EFBillboardGroup (Classes/EF/EFBillboard*.cpp). Signatures follow api_gen.rs `Host` with extra
//! engine-service parameters where the C++ reads _gpoWorldMgr. The "pcRam00343d5c = name" debug
//! trace is dropped. Functions declared void in EXE.1 return the callee's v0 (what the module sees).

#![allow(non_snake_case)]

use super::*;
use crate::engine::{Mem, Ptr};

/// EFBillboardGroup (0x30 bytes). The 4 arrays live in module memory (the logic writes them
/// through the returned pointers): one MMMalloc of n*0x18 = pos[n] (V3), colour[n] (u32 RGBA,
/// R in the low byte), rotation[n] (turns, *128 & 0x7f = table index), size[n].
#[derive(Debug, Clone, PartialEq)]
pub struct EFBillboardGroup {
    /// +0 count
    pub count: u32,
    /// +4 / +8 / +0xc / +0x10 array addresses
    pub pos: Ptr,
    pub color: Ptr,
    pub rot: Ptr,
    pub size: Ptr,
    /// +0x18 texture (name)
    pub texture: String,
    /// +0x20 flags (0x80 / 0x200 blend modes as PLType)
    pub flags: u32,
    /// +0x24 render enabled (default 1)
    pub enabled: u32,
    /// +0x28 id 0x5a000000 | n
    pub id: u32,
}

/// EFBillboardMgr (_gpoBillboardMgr): +0 max billboards (500), +4 group list (append at tail);
/// globals 0x344158 id counter, 0x34415c billboards in use.
#[derive(Debug, Clone, Default)]
pub struct EFBillboardMgr {
    pub groups: Vec<EFBillboardGroup>,
    pub id_counter: i32,
    pub in_use: u32,
}

impl EFBillboardMgr {
    /// EFBillboardMgr::Terminate (0x0027a930)
    pub fn Terminate(&mut self, max: &mut u32) {
        self.groups.clear();
        self.in_use = 0;
        *max = 500;
        self.id_counter = 0;
    }
    /// EFBillboardMgr::AddBillboardGroup (0x0027a9c0) + EFBillboardGroup_BC::Init (0x0027adb0).
    /// quirk: on a texture miss the MMMalloc'ed block and group leak (here: the Mem block leaks).
    pub fn AddBillboardGroup(&mut self, mem: &mut Mem, max: u32, n: u32, tex: &str, flags: u32, out: [Ptr; 4], dict: Option<&[String]>) -> u32 {
        if max.wrapping_sub(self.in_use) < n {
            return 0xffff_ffff;
        }
        let base = mem.malloc(n * 0x18);
        let arr = [base, base + n * 0xc, base + n * 0x10, base + n * 0x14];
        for i in 0..4 {
            mem.w32(out[i], arr[i]);
        }
        // EFTextureDict::GetTexture(name) replacement: name lookup in the current dictionary
        let Some(t) = dict.and_then(|d| d.iter().find(|t| t.eq_ignore_ascii_case(tex))) else { return 0xffff_ffff };
        self.id_counter += 1;
        let id = ((self.id_counter as u32) & 0xff_ffff) | 0x5a00_0000;
        self.groups.push(EFBillboardGroup { count: n, pos: arr[0], color: arr[1], rot: arr[2], size: arr[3], texture: t.clone(), flags, enabled: 1, id });
        self.in_use += n;
        id
    }
    /// EFBillboardMgr::RemoveBillboardGroup (0x0027ab10)
    pub fn RemoveBillboardGroup(&mut self, id: u32) -> u32 {
        let Some(i) = self.groups.iter().position(|g| g.id == id) else { return 0xffff_ffff };
        let g = self.groups.remove(i);
        self.in_use -= g.count;
        // MMFree replacement: the Mem bump allocator has no free (block stays allocated)
        0
    }
    /// EFBillboardMgr::EnableBillboardGroupRender (0x0027ac00)
    pub fn EnableBillboardGroupRender(&mut self, id: u32, on: u32) -> u32 {
        match self.groups.iter_mut().find(|g| g.id == id) {
            None => 0xffff_ffff,
            Some(g) => {
                g.enabled = on;
                0
            }
        }
    }
    /// EFBillboardMgr::Render (0x0027a970) + EFBillboardGroup::Render/RenderBatch: sprites read
    /// from module memory. quirk: the original advances the position pointer by 600 RwV3d (not 50)
    /// per 50-billboard batch, so groups > 50 read garbage past the first batch; here the intended
    /// contiguous layout is read.
    pub fn Render(&self, mem: &Mem) -> Vec<PLDrawBatch> {
        let mut out = Vec::new();
        for g in self.groups.iter().filter(|g| g.enabled != 0) {
            let mut v = Vec::with_capacity(g.count as usize);
            for i in 0..g.count {
                let c = mem.r32(g.color + i * 4);
                let rot = f2b(mem.rf(g.rot + i * 4) * 128.0) & 0x7f;
                v.push(PLSprite { pos: mem.rv3(g.pos + i * 0xc), rgba: c.to_le_bytes(), rot_index: rot, size: mem.rf(g.size + i * 4) * 0.75 });
            }
            let blend = if g.flags & 0x80 != 0 {
                0xa1
            } else if g.flags & 0x200 != 0 {
                0x44
            } else {
                0x29
            };
            out.push(PLDrawBatch { emitter: g.id, texture: g.texture.clone(), blend, stencil_read: false, prims: PLPrims::Sprites(v) });
        }
        out
    }
}

/// Manager-side game services the API needs (EFWorld::GetGameObject(0x2a000000) etc.).
impl PLMgr_BC {
    fn ty(&mut self, id: i32) -> Option<&mut PLTypeData> {
        let i = self.GetType(id)?;
        Some(&mut self.types[i].d)
    }

    /// TypeConvertNameToID (0x001f48a0): -1 when not found
    pub fn APIparticle_TypeConvertNameToID(&mut self, mem: &mut Mem, a0: Ptr) -> u32 {
        let n = mem.cstr_at(a0);
        self.GetTypeByName(&n).map(|i| self.types[i].d.id).unwrap_or(0xffff_ffff)
    }
    /// TypeCreate (0x001f48f0)
    pub fn APIparticle_TypeCreate(&mut self, _mem: &mut Mem) -> u32 {
        self.ParticleTypeCreate()
    }
    /// TypeDestroy (0x001f4920)
    pub fn APIparticle_TypeDestroy(&mut self, _mem: &mut Mem, a0: i32) -> u32 {
        self.ParticleTypeDelete(a0)
    }
    /// TypeSetInter (0x001f4620): 1 = no type
    pub fn APIparticle_TypeSetInter(&mut self, _mem: &mut Mem, a0: i32, a1: u32, a2: f32, a3: f32, a4: f32, a5: f32) -> u32 {
        self.ty(a0).map(|t| t.SetInter(a1, a2, a3, a4, a5)).unwrap_or(1)
    }
    /// TypeSetInt (0x001f46c0)
    pub fn APIparticle_TypeSetInt(&mut self, _mem: &mut Mem, a0: i32, a1: u32, a2: i32) -> u32 {
        self.ty(a0).map(|t| t.SetInt(a1, a2 as u32)).unwrap_or(1)
    }
    /// TypeGetInt (0x001f4730)
    pub fn APIparticle_TypeGetInt(&mut self, mem: &mut Mem, a0: i32, a1: u32, a2: Ptr) -> u32 {
        let Some(t) = self.ty(a0) else { return 1 };
        let mut v = mem.r32(a2);
        let r = t.GetInt(a1, &mut v);
        if r == 0 {
            mem.w32(a2, v);
        }
        r
    }
    /// TypeSetFloat (0x001f47a0)
    pub fn APIparticle_TypeSetFloat(&mut self, _mem: &mut Mem, a0: i32, a1: u32, a2: f32) -> u32 {
        self.ty(a0).map(|t| t.SetReal(a1, a2)).unwrap_or(1)
    }
    /// TypeSetVec (0x001f4810)
    pub fn APIparticle_TypeSetVec(&mut self, _mem: &mut Mem, a0: i32, a1: u32, a2: f32, a3: f32, a4: f32) -> u32 {
        self.ty(a0).map(|t| t.SetVec(a1, a2, a3, a4)).unwrap_or(1)
    }
    /// TypeSetTexture (0x001f4c50): PLType::SetTexture (vtbl 0x4c)
    pub fn APIparticle_TypeSetTexture(&mut self, mem: &mut Mem, a0: i32, a1: Ptr) -> u32 {
        let n = mem.cstr_at(a1);
        let dict = self.textures.clone();
        self.ty(a0).map(|t| t.SetTexture(&n, dict.as_deref())).unwrap_or(1)
    }
    /// TypeSetName (0x001f4950): returns true when NOT found
    pub fn APIparticle_TypeSetName(&mut self, mem: &mut Mem, a0: i32, a1: Ptr) -> u8 {
        let n = mem.cstr_at(a1);
        match self.ty(a0) {
            Some(t) => {
                t.SetDisplayName(&n);
                0
            }
            None => 1,
        }
    }
    /// TypeGetName (0x001f49a0): strcpy of the name
    pub fn APIparticle_TypeGetName(&mut self, mem: &mut Mem, a0: i32, a1: Ptr) -> u8 {
        match self.ty(a0).map(|t| t.name.clone()) {
            Some(n) => {
                for (i, b) in n.bytes().chain(std::iter::once(0)).enumerate() {
                    mem.w8(a1 + i as u32, b);
                }
                0
            }
            None => 1,
        }
    }
    /// EmitterConvertNameToID (0x001f49f0)
    pub fn APIparticle_EmitterConvertNameToID(&mut self, mem: &mut Mem, a0: Ptr, a1: u32) -> u32 {
        let n = mem.cstr_at(a0);
        self.GetEmitterByName(&n, a1).map(|(t, e)| self.types[t].emitters[e].id).unwrap_or(0xffff_ffff)
    }
    /// EmitterCreate (0x001f4a30): owner = EFWorld::GetGameObject(0x2a000000) (the manager object).
    pub fn APIparticle_EmitterCreate(&mut self, _mem: &mut Mem, a0: i32, w: &dyn PLWorld) -> u32 {
        let owner = w.GameObjectExists(0x2a00_0000).then_some(0x2a00_0000);
        self.ParticleEmitterCreate(a0, owner)
    }
    /// EmitterDestroy (0x001f4a80)
    pub fn APIparticle_EmitterDestroy(&mut self, _mem: &mut Mem, a0: i32) -> u32 {
        if self.ParticleEmitterDelete(a0) != 2 { 0 } else { 2 }
    }
    /// EmitterSetType (0x001f4ad0)
    pub fn APIparticle_EmitterSetType(&mut self, _mem: &mut Mem, a0: i32, a1: i32) -> u32 {
        self.ParticleEmitterSetType(a0, a1)
    }
    /// EmitterStart (0x001f4b10): 2 = no emitter
    pub fn APIparticle_EmitterStart(&mut self, _mem: &mut Mem, a0: i32, a1: u32) -> u32 {
        match self.emitter_mut(a0) {
            None => 2,
            Some(e) => {
                e.Start(a1);
                0
            }
        }
    }
    /// EmitterStartAt (0x001f4b70): the time is ignored
    pub fn APIparticle_EmitterStartAt(&mut self, mem: &mut Mem, a0: i32, a1: u32, _a2: f32) -> u32 {
        self.APIparticle_EmitterStart(mem, a0, a1)
    }
    /// EmitterStop (0x001f4b90): started = 0 and quirk: clears flag 0x10 (stopped) if set.
    pub fn APIparticle_EmitterStop(&mut self, _mem: &mut Mem, a0: i32) -> u32 {
        match self.emitter_mut(a0) {
            None => 2,
            Some(e) => {
                e.started = false;
                e.flags &= !0x10;
                0
            }
        }
    }
    /// EmitterReset (0x001f4c00)
    pub fn APIparticle_EmitterReset(&mut self, _mem: &mut Mem, a0: i32) -> u32 {
        let Some((t, e)) = self.GetEmitter(a0) else { return 2 };
        self.types[t].emitters[e].Reset(&mut self.pool);
        0
    }
    /// EmitterSetCollisionPlane (0x001f5020)
    pub fn APIparticle_EmitterSetCollisionPlane(&mut self, mem: &mut Mem, a0: i32, a1: u32, a2: Ptr, a3: Ptr, a4: f32) -> u32 {
        let (n, p) = (mem.rv3(a2), mem.rv3(a3));
        match self.emitter_mut(a0) {
            None => 2,
            Some(e) => {
                e.SetCollisionPlane(n, p, a4, a1);
                0
            }
        }
    }
    /// EmitterSetJoint (0x001f4cc0): bone 0x6fffffff = own root frame (EFPosData::CreateFrame);
    /// 0x2a uid = game object frame `bone` (0x40 = no object); else the pos data frame;
    /// 0x20 = no frame. EFPosData flag +0x18 bit 0 (owns frame) cleared.
    pub fn APIparticle_EmitterSetJoint(&mut self, _mem: &mut Mem, a0: i32, a1: u32, a2: u32, w: &dyn PLWorld) -> u32 {
        let Some(e) = self.emitter_mut(a0) else { return 2 };
        if a2 == 0x6fff_ffff {
            // CreateFrame(NULL): fresh identity root frame
            let m = DTMatrix::default();
            e.frame = Some(PLFrame { attach: PLFrameAttach::Own, local: m, ltm: m });
            return 0;
        }
        let (attach, ltm) = if a1 >> 24 == 0x2a {
            if !w.GameObjectExists(a1) {
                return 0x40;
            }
            (PLFrameAttach::ObjectFrame { obj: a1, bone: a2 }, w.GetObjectFrame(a1, a2))
        } else {
            (PLFrameAttach::PosData { uid: a1 }, w.GetObjectPosDataFrame(a1))
        };
        let Some(ltm) = ltm else { return 0x20 };
        e.frame = Some(PLFrame { attach, local: ltm, ltm });
        0
    }
    /// EmitterSetFlags (0x001f4df0): started = !(flags & 0x10)
    pub fn APIparticle_EmitterSetFlags(&mut self, _mem: &mut Mem, a0: i32, a1: u32) -> u32 {
        match self.emitter_mut(a0) {
            None => 2,
            Some(e) => {
                e.flags = a1;
                e.started = a1 & 0x10 == 0;
                0
            }
        }
    }
    /// EmitterGetFlags (0x001f4e70)
    pub fn APIparticle_EmitterGetFlags(&mut self, mem: &mut Mem, a0: i32, a1: Ptr) -> u32 {
        match self.emitter_mut(a0) {
            None => 2,
            Some(e) => {
                let f = e.flags;
                mem.w32(a1, f);
                0
            }
        }
    }
    /// EmitterSetBox (0x001f4f50)
    pub fn APIparticle_EmitterSetBox(&mut self, mem: &mut Mem, a0: i32, a1: Ptr, a2: Ptr) -> u32 {
        let (lo, hi) = (mem.rv3(a1), mem.rv3(a2));
        match self.emitter_mut(a0) {
            None => 2,
            Some(e) => {
                e.SetBox(lo, hi);
                0
            }
        }
    }
    /// EmitterGetBBox (0x001f4ed0): unselected components are left untouched
    pub fn APIparticle_EmitterGetBBox(&mut self, mem: &mut Mem, a0: i32, a1: Ptr, a2: Ptr, a3: u32) -> u32 {
        let Some((t, e)) = self.GetEmitter(a0) else { return 2 };
        let (mut lo, mut hi) = (mem.rv3(a1), mem.rv3(a2));
        let r = self.types[t].emitters[e].GetBBox(&self.pool, &mut lo, &mut hi, a3);
        mem.wv3(a1, lo);
        mem.wv3(a2, hi);
        r
    }
    /// EmitterSetBSphereRadius (0x001f4fc0)
    pub fn APIparticle_EmitterSetBSphereRadius(&mut self, _mem: &mut Mem, a0: i32, a1: f32) -> u32 {
        match self.emitter_mut(a0) {
            None => 2,
            Some(e) => {
                e.bsphere_radius = a1;
                0
            }
        }
    }
    /// EmitterSetMessageHandlerID (0x001f50b0): 0x40 no object, 2 no emitter, 4 already set
    pub fn APIparticle_EmitterSetMessageHandlerID(&mut self, _mem: &mut Mem, a0: u32, a1: u32, w: &dyn PLWorld) -> u32 {
        let obj = w.GameObjectExists(a1);
        let e = self.emitter_mut(a0 as i32);
        if !obj {
            return 0x40;
        }
        match e {
            None => 2,
            Some(e) if e.msg_handler == 0xffff_ffff => {
                e.msg_handler = a1;
                0
            }
            Some(_) => 4,
        }
    }
    /// CreateBillboardGroup (0x001f5140)
    pub fn APIparticle_CreateBillboardGroup(&mut self, mem: &mut Mem, bb: &mut EFBillboardMgr, a0: u32, a1: Ptr, a2: u32, a3: Ptr, a4: Ptr, a5: Ptr, a6: Ptr) -> u32 {
        let tex = mem.cstr_at(a1);
        let dict = self.textures.clone();
        bb.AddBillboardGroup(mem, self.max_billboards, a0, &tex, a2, [a3, a4, a5, a6], dict.as_deref())
    }
    /// RenderBillboardGroup (0x001f5190)
    pub fn APIparticle_RenderBillboardGroup(&mut self, _mem: &mut Mem, bb: &mut EFBillboardMgr, a0: u32, a1: u32) -> u32 {
        bb.EnableBillboardGroupRender(a0, a1)
    }
    /// DestroyBillboardGroup (0x001f51c0)
    pub fn APIparticle_DestroyBillboardGroup(&mut self, _mem: &mut Mem, bb: &mut EFBillboardMgr, a0: u32) -> u32 {
        bb.RemoveBillboardGroup(a0)
    }
    /// SetMaxNumBillboards (0x001f51e0): only once, only while still at the default 500; quirk:
    /// passing 500 returns -1.
    pub fn APIparticle_SetMaxNumBillboards(&mut self, _mem: &mut Mem, a0: u32) -> u32 {
        if a0 == 500 || self.max_billboards != 500 {
            return 0xffff_ffff;
        }
        self.max_billboards = a0;
        0
    }
}

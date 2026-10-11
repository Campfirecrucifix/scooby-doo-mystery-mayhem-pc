//! WP-S1: APIfbeffects (Engine API (module interface)/EFApi_FBEffects.cpp) -> EFFBEffectMgr_BC
//! (0x002326e0..), EFFBEffect_BC, EFFBEffect_FrameDistortion_BC/_SP Set/GetParams. State only:
//! the frame-buffer distortion pipeline (EFFBEffect_FrameDistortion_SP::Update/PipelineUpload,
//! EFFBEffectMgr_BC::Update depth sort) is render work that reads [`EFFBEffectMgr_BC::slots`].
#![allow(non_snake_case)]

use crate::engine::{Mem, Ptr};

/// EFFBEffect_FrameDistortion_SP (0x20 bytes); type 2 is the only effect AddEffect can create
#[derive(Debug, Clone, PartialEq)]
pub struct EFFBEffect {
    /// +4 flags: bit31 enabled, bit30 kept by SetFlags, low bits = render pass mask
    pub flags: u32,
    /// +8 handle
    pub handle: u32,
    /// +0xc param word 0
    pub p0: u32,
    /// +0x10 game object uid (GetGameObject(param+4)+8), ctor 0xffffffff
    pub obj: u32,
    /// +0x14 param word 2
    pub p2: u32,
    /// +0x18 RwMatrix copy of *param+0xc (RW layout, 16 words)
    pub matrix: Option<[u32; 16]>,
    /// +0x1c the caller's matrix pointer (param+0xc)
    pub matrix_ptr: u32,
}

/// EFFBEffectMgr_BC: +4 handle counter, +8..+0x27 eight effect slots (NULL = None)
#[derive(Debug, Clone)]
pub struct EFFBEffectMgr_BC {
    pub counter: u32,
    pub slots: [Option<EFFBEffect>; 8],
}

impl Default for EFFBEffectMgr_BC {
    /// ctor +4 = 1, then Initialize (0x00232780) +4 = 0 at engine init
    fn default() -> Self {
        Self { counter: 0, slots: Default::default() }
    }
}

impl EFFBEffectMgr_BC {
    /// CreateFB (0x00232370) -> AddEffect (0x00232860), returns GetHandle.
    /// quirk: a type other than 2 creates no effect; the original then writes flags/handle
    /// through NULL (crash) — here it consumes a handle, stores an empty slot and returns 0.
    pub fn CreateFB(&mut self, ty: u32, flags: u32) -> u32 {
        let i = self.slots.iter().position(|s| s.is_none()).unwrap_or(7);
        self.counter += 1;
        if ty != 2 {
            return 0;
        }
        let h = self.counter;
        self.slots[i] =
            Some(EFFBEffect { flags, handle: h, p0: 0, obj: 0xffff_ffff, p2: 0, matrix: None, matrix_ptr: 0 });
        h
    }
    fn get(&mut self, h: u32) -> Option<&mut EFFBEffect> {
        self.slots.iter_mut().flatten().find(|e| e.handle == h)
    }
    /// DestroyFB (0x002323b0) -> RemoveEffect: Term, delete, shift the following slots down
    /// (memcpy (7-i)*4), slot 7 = NULL
    pub fn DestroyFB(&mut self, h: u32) {
        if let Some(i) = self.slots.iter().position(|s| s.as_ref().map_or(false, |e| e.handle == h)) {
            for j in i..7 {
                self.slots[j] = self.slots[j + 1].take();
            }
            self.slots[7] = None;
        }
    }
    /// EnableFB (0x002323e0) -> EFFBEffect_BC::Enable: bit31 set/clear
    pub fn EnableFB(&mut self, h: u32, on: u32) {
        if let Some(e) = self.get(h) {
            e.flags = if on != 0 { e.flags | 0x8000_0000 } else { e.flags & 0x7fff_ffff };
        }
    }
    /// SetParamsFB (0x00232420) -> vtbl+0x1c EFFBEffect_FrameDistortion_SP::SetParams (0x002791d0)
    /// `obj_uid` = EFWorld::GetGameObject(param+4)+8 resolver (None: the original reads NULL+8).
    pub fn SetParamsFB(&mut self, m: &Mem, h: u32, ty: u32, p: Ptr, obj_uid: impl FnOnce(u32) -> Option<u32>) {
        let Some(e) = self.get(h) else { return };
        if ty != 2 {
            return;
        }
        e.p0 = m.r32(p);
        e.p2 = m.r32(p + 8);
        e.matrix_ptr = m.r32(p + 0xc);
        if e.matrix_ptr != 0 {
            let mp = e.matrix_ptr;
            e.matrix = Some(std::array::from_fn(|i| m.r32(mp + 4 * i as u32)));
        }
        e.obj = obj_uid(m.r32(p + 4)).unwrap_or(0);
    }
    /// EFFBEffect_FrameDistortion_SP::Update (0x00278f90) render decision: enabled (bit31) and
    /// object set (+0x10 != -1) and object alive -> (object uid, LOD index +0x14 the atomics must
    /// match (ApGetAtomicLod), uv matrix +0x18 or identity). PipelineUpload sends the matrix rows
    /// (3x3 + pos, w=1) and the screen scale 320 / (224|256 by gfx +0x44) to the VU1 micro code
    /// that samples the grabbed frame buffer (EFFBEffect_SP::GrabRaster) at the distorted
    /// screen position; that micro code is not in the decompiled source.
    pub fn Distortions(&self, alive: impl Fn(u32) -> bool) -> Vec<(u32, u32, [u32; 16])> {
        const ID: [u32; 16] =
            [0x3f80_0000, 0, 0, 0, 0, 0x3f80_0000, 0, 0, 0, 0, 0x3f80_0000, 0, 0, 0, 0, 0x3f80_0000];
        self.slots
            .iter()
            .flatten()
            .filter(|e| e.flags & 0x8000_0000 != 0 && e.obj != 0xffff_ffff && alive(e.obj))
            .map(|e| (e.obj, e.p2, e.matrix.unwrap_or(ID)))
            .collect()
    }
    /// GetParamsFB (0x00232480) -> vtbl+0x20 GetParams (0x002792f0); word 3 = +0x1c (the +0x18
    /// store is overwritten in the original)
    pub fn GetParamsFB(&mut self, m: &mut Mem, h: u32, ty: u32, p: Ptr) {
        let Some(e) = self.get(h) else { return };
        if ty != 2 {
            return;
        }
        let w = [e.p0, e.obj, e.p2, e.matrix_ptr];
        for (i, v) in w.iter().enumerate() {
            m.w32(p + 4 * i as u32, *v);
        }
    }
}

//! WP-S1: APIprojector (Engine API (module interface)/EFApi_Projector.cpp) ->
//! EFProjectorMgr_BC (0x0024c040..) / EFProjector_BC (0x0024ccc0..), state only.
//! The projection itself (EFProjector_BC::Begin / EFProjectorMgr_BC::Begin/End/AddOccurence,
//! receivers from logic flag 0x200) is render work: the renderer reads [`EFProjectorMgr_BC::projectors`].
#![allow(non_snake_case)]

use crate::engine::{Mem, Ptr};
use std::collections::BTreeMap;

/// EFProjector_BC (0x5c bytes) fields the API writes (ctor 0x0024ccc0 defaults)
#[derive(Debug, Clone, PartialEq)]
pub struct EFProjector_BC {
    /// +0x28 flags (bit0 = visible), ctor 1
    pub flags: u32,
    /// +0x2c handle
    pub handle: u32,
    /// +0x30 RwTexture*: the texture name looked up in the current TXD (None = NULL)
    pub texture: Option<String>,
    /// +0x34/+0x38 size, ctor 1, 1
    pub size: [f32; 2],
    /// +0x3c reach distance, ctor 4
    pub reach: f32,
    /// +0x40..+0x4c color rgba, ctor 1,1,1,1
    pub color: [f32; 4],
    /// +0x50 blending function
    pub blend: u32,
    /// +0x54 projection type
    pub projection: u32,
}

#[derive(Debug, Clone, Default)]
pub struct EFProjectorMgr_BC {
    /// hash table (+0xc) handle -> projector; BTreeMap keeps a stable render order
    pub projectors: BTreeMap<u32, EFProjector_BC>,
    /// stand-in for the global muiNextValidHandle (EFPosData handle counter, not modelled
    /// elsewhere): projector handles start at 1
    pub next_handle: u32,
    /// texture names of the current EFTextureDict (EFTextureDict::GetTexture -> NULL when absent);
    /// empty = no dictionary known, names are kept as given
    pub textures: Vec<String>,
}

impl EFProjectorMgr_BC {
    /// CreateProjector (0x001f8600) -> EFProjectorMgr_BC::CreateProjector (0x0024c580); returns +0x2c
    pub fn CreateProjector(&mut self) -> u32 {
        self.next_handle += 1;
        let h = self.next_handle;
        self.projectors.insert(
            h,
            EFProjector_BC {
                flags: 1,
                handle: h,
                texture: None,
                size: [1.0, 1.0],
                reach: 4.0,
                color: [1.0; 4],
                blend: 0,
                projection: 0,
            },
        );
        h
    }
    /// DestroyProjector (0x001f8920): unknown handle -> no-op
    pub fn DestroyProjector(&mut self, h: u32) {
        self.projectors.remove(&h);
    }
    /// SetTexture (0x001f8640) -> EFProjector_BC::SetTexture: +0x30 = EFTextureDict::GetTexture(name)
    pub fn SetTexture(&mut self, m: &Mem, h: u32, name: Ptr) {
        let n = m.cstr_at(name);
        let known = self.textures.is_empty() || self.textures.iter().any(|t| t.eq_ignore_ascii_case(&n));
        if let Some(p) = self.projectors.get_mut(&h) {
            p.texture = known.then_some(n);
        }
    }
    pub fn with(&mut self, h: u32, f: impl FnOnce(&mut EFProjector_BC)) {
        if let Some(p) = self.projectors.get_mut(&h) {
            f(p);
        }
    }
}

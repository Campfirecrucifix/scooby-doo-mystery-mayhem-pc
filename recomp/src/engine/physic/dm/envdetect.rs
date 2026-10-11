//! 0x1a EFDMEnvDetect (Classes/EFDM/EFDMEnvDetect.cpp, 0x44 bytes), 1:1 logic.
//! Replacement: the C++ PostColUpdate reads the collision stage's touched env prims
//! (stage+4 prim* array, stage+8 count; prim +0 attr word, +6 u16 vertex count, +0xc plane,
//! +0x1c vertices stride 0xc) and sends EFNotificationMgr notify 0x19 to the owner with
//! {count, EnvDetectEntry[max] (+0 &prim+0xc, +4 attr, +8 nverts, +0xc vertex ptr array)}.
//! The stage seam (module.rs) has no env prim list, so the caller passes the touched prims to
//! `Detect`, and the notification payload is kept in `last` (coordinator forwards it).
#![allow(non_snake_case)]

use crate::engine::physic::module::*;
use crate::engine::physic::rwm::*;

/// One touched env primitive (EFCollisionPrim of the collision map: the fields the EFDM* read).
/// Shared by EnvDetect and Edge; build it from the map with `from_map`.
#[derive(Debug, Clone, Default)]
pub struct EnvPrimSeam {
    /// prim+0 material pointer (here: material word offset)
    pub attr: u32,
    /// material+4 / +8 masks
    pub mask1: u32,
    pub mask2: u32,
    /// prim+0xc plane (normal, d)
    pub plane: [f32; 4],
    /// prim+0x1c vertices (count = prim+6 & 0x7fff)
    pub verts: Vec<V3>,
    /// edge planes after the vertices (one per vertex)
    pub edges: Vec<[f32; 4]>,
}

impl EnvPrimSeam {
    pub fn from_map(map: &crate::engine::physic::collision_map::EFCollisionMap, p: usize) -> Self {
        let n = map.PrimCount(p) & 0x7fff;
        let mat = map.PrimMaterial(p);
        let nr = map.PrimNormal(p);
        EnvPrimSeam {
            attr: mat as u32,
            mask1: map.MaterialWord(mat, 1),
            mask2: map.MaterialWord(mat, 2),
            plane: [nr[0], nr[1], nr[2], map.PrimD(p)],
            verts: (0..n).map(|i| map.PrimVertex(p, i)).collect(),
            edges: (0..n).map(|i| map.PrimEdgePlane(p, i)).collect(),
        }
    }
}

/// Notify 0x19 payload ({count, entries}). count = number of touched prims (NOT clamped), entries
/// clamped to max_prims; count 0 sends a NULL array.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EnvDetectNotify {
    pub target: u32,
    pub count: u32,
    pub entries: Vec<([f32; 4], u32, Vec<V3>)>,
}

pub struct EFDMEnvDetect {
    pub base: EFPhysicModule,
    /// +0x30 user value (stored in the RegisterNotify payload with the game object)
    pub user: u32,
    /// +0x34 owner uid (notify target)
    pub target: u32,
    /// +0x38 max prims reported (+0x40 = array of that many 0x10 entries)
    pub max_prims: u32,
    /// RegisterNotify(0x19, owner, 0x1fd300 cb, {obj, user}) done
    pub registered: bool,
    pub last: Option<EnvDetectNotify>,
}

impl EFDMEnvDetect {
    pub fn new() -> Self {
        EFDMEnvDetect {
            base: EFPhysicModule { type_: 0x4000_001a, flags: 1, ..Default::default() },
            user: 0,
            target: 0,
            max_prims: 0,
            registered: false,
            last: None,
        }
    }
    /// PostColUpdate body with the stage's touched env prims.
    pub fn Detect(&mut self, prims: &[EnvPrimSeam]) {
        let n = prims.len() as u32;
        let m = n.min(self.max_prims) as usize;
        let entries = prims[..if n == 0 { 0 } else { m }].iter().map(|p| (p.plane, p.attr, p.verts.clone())).collect();
        self.last = Some(EnvDetectNotify { target: self.target, count: n, entries });
    }
}

impl EFDM for EFDMEnvDetect {
    fn base(&self) -> &EFPhysicModule { &self.base }
    fn base_mut(&mut self) -> &mut EFPhysicModule { &mut self.base }
    fn ParamWords(&self) -> usize { 2 }
    fn SetParams(&mut self, ctx: &mut DMCtx, p: Option<&[u32]>) {
        if let Some(p) = p {
            self.user = p[0];
            self.target = ctx.phys.owner;
            self.max_prims = p[1];
            self.registered = true;
        }
    }
    fn GetParams(&self, _c: &DMCtx, p: &mut [u32]) {
        p[0] = self.user;
        p[1] = self.max_prims;
    }
    /// Stage without env prim data (seam): reports zero prims, like a stage with count 0.
    fn PostColUpdate(&mut self, _ctx: &mut DMCtx, stage: Option<&EFCollisionStage>, _dt: f32) {
        if stage.is_some() {
            self.Detect(&[]);
        }
    }
}

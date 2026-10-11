//! EF3dObj frame hierarchy + HAnim skinning (Classes/EF/EF3dObj.cpp InitHierarchy 0x00224140,
//! UpdateHierarchy 0x002242b0, InitTransfo; EF3dObj_SP::GetHierarchyCallback 0x00222cd0;
//! DTQuaternion::QuatToMat 0x0010e210).
//!
//! The EF code is 1:1. The RenderWare objects it drives are replaced (not 1:1):
//! - RwFrame tree -> `FrameTree` (per-instance copy of the clump's frame list: local matrices,
//!   parent links, child lists in RW order, LTMs recomputed by `UpdateLTMs` = RwFrameGetLTM /
//!   RwFrameUpdateObjects).
//! - RpHAnimHierarchy -> `RpHAnimHierarchy` (flags, node ids, node -> frame index map);
//!   RpHAnimHierarchyAttach = match node ids against the frames' HAnim ids.
//! - RpSkin palette (RpSkinAtomicSetHAnimHierarchy + the PS2 skin pipeline) -> `SkinMatrices`.
//!
//! Matrices are column-major 4x4 (`Mat4::from_cols_array` order, as `rw::clump::mat_mul`). An RW
//! product `A*B` (row vectors: apply A then B) is `mat_mul(B, A)` here.
#![allow(non_snake_case, non_camel_case_types)]

use crate::engine::anim::res::DTTransfo;
use crate::engine::rw::clump::{mat_mul, RpClump};
use crate::engine::rw::geometry::RpSkin;

pub type Mat4 = [f32; 16];
pub const IDENTITY: Mat4 = [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.];

/// rpHANIMHIERARCHYNOMATRICES
pub const rpHANIMHIERARCHYNOMATRICES: u32 = 2;
/// rpHANIMHIERARCHYUPDATEMODELLINGMATRICES | rpHANIMHIERARCHYUPDATELTMS
pub const rpHANIMHIERARCHYUPDATEFRAMES: u32 = 0x3000;

/// The per-instance RwFrame tree of an EF3dObj (RpClumpClone gives every instance its own frames).
#[derive(Debug, Clone, Default)]
pub struct FrameTree {
    /// RwFrame modelling matrices (frame +0x10)
    pub local: Vec<Mat4>,
    pub parent: Vec<i32>,
    /// child lists in RwFrameForAllChildren order: RwFrameAddChild inserts at the head, so the
    /// children of a streamed frame list are visited in reverse stream order.
    pub children: Vec<Vec<usize>>,
    /// HAnim id of each frame (RpHAnimFrameGetID), -1 when none
    pub hanim_id: Vec<i32>,
    /// frames that own an RpHAnimHierarchy (RpHAnimFrameGetHierarchy != NULL)
    pub has_hierarchy: Vec<bool>,
    /// RwFrameGetLTM results (world = parent LTM chain; frame 0's parent is `root_parent`)
    pub ltm: Vec<Mat4>,
    /// LTM of the frame the clump root is attached to (EF3dObj::Initialize: RwFrameAddChild(the
    /// game object's frame (owner+0x14), clump root)). Set by the caller each frame.
    pub root_parent: Mat4,
}

impl FrameTree {
    pub fn from_clump(c: &RpClump) -> FrameTree {
        let n = c.frames.len();
        let mut t = FrameTree {
            local: c.frames.iter().map(|f| f.local_matrix()).collect(),
            parent: c.frames.iter().map(|f| f.parent).collect(),
            children: vec![Vec::new(); n],
            hanim_id: c.frames.iter().map(|f| f.hanim.as_ref().map(|h| h.id).unwrap_or(-1)).collect(),
            has_hierarchy: c.frames.iter().map(|f| f.hanim.as_ref().is_some_and(|h| !h.nodes.is_empty())).collect(),
            ltm: vec![IDENTITY; n],
            root_parent: IDENTITY,
        };
        for i in 0..n {
            let p = t.parent[i];
            if p >= 0 && (p as usize) < n {
                t.children[p as usize].insert(0, i); // quirk: head insertion
            }
        }
        t.UpdateLTMs();
        t
    }
    /// RwFrameUpdateObjects + RwFrameGetLTM for the whole tree (parents precede children).
    pub fn UpdateLTMs(&mut self) {
        for i in 0..self.local.len() {
            let p = self.parent[i];
            let pl = if p >= 0 && (p as usize) < i { self.ltm[p as usize] } else { self.root_parent };
            self.ltm[i] = mat_mul(&pl, &self.local[i]);
        }
    }
    pub fn GetLTM(&self, f: usize) -> Mat4 {
        self.ltm.get(f).copied().unwrap_or(IDENTITY)
    }
}

/// RpHAnimHierarchy replacement: +0 flags, +4 numNodes, +0x10 pNodeInfo (+0xc pFrame).
#[derive(Debug, Clone, Default)]
pub struct RpHAnimHierarchy {
    /// +0x0
    pub flags: u32,
    /// +0x4
    pub num_nodes: u32,
    /// node ids (pNodeInfo[i].nodeID) and flags (push/pop)
    pub node_ids: Vec<i32>,
    pub node_flags: Vec<u32>,
    /// pNodeInfo[i].pFrame (frame index), filled by RpHAnimHierarchyAttach
    pub node_frames: Vec<Option<usize>>,
    /// frame owning the hierarchy (parentFrame)
    pub parent_frame: usize,
}

impl RpHAnimHierarchy {
    /// RpHAnimHierarchyAttach: node -> frame by HAnim id, searched in the sub-tree of the
    /// hierarchy's frame (frame order).
    pub fn Attach(&mut self, t: &FrameTree) {
        self.node_frames = self.node_ids.iter().map(|&id| t.hanim_id.iter().position(|&h| h == id)).collect();
    }
}

/// EF3dObj_SP::GetHierarchyCallback (RwFrameForAllChildren callback, depth first; stops at the
/// first frame with a hierarchy).
pub fn GetHierarchyCallback(t: &FrameTree, frame: usize, found: &mut Option<usize>) -> bool {
    if t.has_hierarchy[frame] {
        *found = Some(frame);
        return false;
    }
    for &c in &t.children[frame] {
        if !GetHierarchyCallback(t, c, found) {
            break;
        }
    }
    found.is_none()
}

/// The HAnim side of an EF3dObj (this+0x34 hierarchy, this+0x50/+0x54 transfo pools).
#[derive(Debug, Clone, Default)]
pub struct EF3dObjHierarchy {
    /// this+0x34 RpHAnimHierarchy*
    pub hierarchy: Option<RpHAnimHierarchy>,
    /// this+0x50, this+0x54: two arrays of num_nodes DTTransfo (one allocation of 2*n)
    pub transfos: [Vec<DTTransfo>; 2],
}

impl EF3dObjHierarchy {
    /// EF3dObj::InitTransfo(n): new DTTransfo[2n]; pool 0 ids = 0..n (pool 1 keeps -1).
    pub fn InitTransfo(&mut self, n: u32) {
        let mut p0 = vec![DTTransfo::default(); n as usize];
        for (i, t) in p0.iter_mut().enumerate() {
            t.id = i as i32;
        }
        self.transfos = [p0, vec![DTTransfo::default(); n as usize]];
    }

    /// EF3dObj::InitHierarchy. `obj_flags` = EF3dObj this+0x2c (|4 when a hierarchy exists).
    /// Returns 1 when a hierarchy was found.
    pub fn InitHierarchy(&mut self, clump: &RpClump, t: &FrameTree, obj_flags: &mut u32) -> u32 {
        let mut found = None;
        // RwFrameForAllChildren(clump root frame (frame 0), GetHierarchyCallback, this+0x34)
        if !t.local.is_empty() {
            for &c in &t.children[0] {
                if !GetHierarchyCallback(t, c, &mut found) {
                    break;
                }
            }
        }
        let Some(hf) = found else { return 0 };
        let data = clump.frames[hf].hanim.as_ref().unwrap();
        let mut h = RpHAnimHierarchy {
            flags: data.hierarchy_flags,
            num_nodes: data.nodes.len() as u32,
            node_ids: data.nodes.iter().map(|n| n.id).collect(),
            node_flags: data.nodes.iter().map(|n| n.flags).collect(),
            node_frames: Vec::new(),
            parent_frame: hf,
        };
        // for every atomic with a skin: RpSkinAtomicSetHAnimHierarchy(atomic, h) -> implicit:
        // every skinned atomic of this clump uses `hierarchy` (see SkinMatrices).
        *obj_flags |= 4;
        h.Attach(t);
        // the hierarchy's own matrix array is freed: frames drive the skin (NOMATRICES)
        let mut f = h.flags;
        if f & rpHANIMHIERARCHYNOMATRICES == 0 {
            f |= rpHANIMHIERARCHYNOMATRICES;
        }
        h.flags = f | rpHANIMHIERARCHYUPDATEFRAMES;
        let n = h.num_nodes;
        self.hierarchy = Some(h);
        self.InitTransfo(n);
        1
    }

    /// EF3dObj::UpdateHierarchy: writes the DTTransfo pool 0 into the node frames' modelling
    /// matrices (only when this+0x2c & 0x80). A root transfo (state & 8) drives the owner game
    /// object's frame instead (`owner_frame`, owner+0x14) and is reset afterwards.
    /// Call `FrameTree::UpdateLTMs` afterwards (RwFrameUpdateObjects).
    pub fn UpdateHierarchy(&mut self, obj_flags: u32, t: &mut FrameTree, owner_frame: &mut Mat4) {
        if obj_flags & 0x80 == 0 {
            return;
        }
        let Some(h) = &self.hierarchy else { return };
        let n = (h.num_nodes as usize).min(self.transfos[0].len());
        let mut i = 0;
        if n > 0 && self.transfos[0][0].state & 8 != 0 {
            *owner_frame = GetMatrix(&self.transfos[0][0]);
            // DTTransfo::ResetEuler(this_00, 1)
            self.transfos[0][0].ResetEuler();
            i = 1;
        }
        while i < n {
            if let Some(Some(f)) = h.node_frames.get(i) {
                t.local[*f] = GetMatrix(&self.transfos[0][i]);
            }
            i += 1;
        }
    }
}

/// DTTransfo -> RW modelling matrix as built inline by UpdateHierarchy (DTTransfo::GetMatrix):
/// rot = state&1 ? DTMatrix::SetToRotation(euler +0x18) : QuatToMat(quat +0x24);
/// m = Scale(+0x0) * rot (RwMatrixMultiply(m, scale, rot)); state&4: RwMatrixScale(m, +0x34,
/// POSTCONCAT); pos = +0x0c.
/// Layout used here (from this code): f[0..3] scale +0x0, f[3..6] position +0xc, f[6..9] euler +0x18.
pub fn GetMatrix(tr: &DTTransfo) -> Mat4 {
    let rot = if tr.state & 1 == 0 { QuatToMat(tr.quat) } else { SetToRotation([tr.f[6], tr.f[7], tr.f[8]]) };
    let s = [tr.f[0], tr.f[1], tr.f[2]];
    // RW scale*rot: rows scaled -> columns right/up/at scaled by sx/sy/sz
    let mut m = rot;
    for c in 0..3 {
        for r in 0..3 {
            m[c * 4 + r] *= s[c];
        }
    }
    if tr.state & 4 != 0 {
        // POSTCONCAT scale: every row's components scaled (x,y,z)
        for c in 0..3 {
            for r in 0..3 {
                m[c * 4 + r] *= tr.v34[r];
            }
        }
    }
    m[12] = tr.f[3];
    m[13] = tr.f[4];
    m[14] = tr.f[5];
    m
}

/// DTQuaternion::QuatToMat (x, y, z, w), rotation part only (column-major).
pub fn QuatToMat(q: [f32; 4]) -> Mat4 {
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    let (y2, z2) = (y + y, z + z);
    let xx2 = x * (x + x);
    let wx2 = w * (x + x);
    let xz2 = x * z2;
    let yz2 = y * z2;
    let wy2 = w * y2;
    let right = [1.0 - (y * y2 + z * z2), x * y2 + w * z2, xz2 - wy2];
    let up = [x * y2 - w * z2, 1.0 - (xx2 + z * z2), yz2 + wx2];
    let at = [xz2 + wy2, yz2 - wx2, 1.0 - (xx2 + y * y2)];
    [right[0], right[1], right[2], 0., up[0], up[1], up[2], 0., at[0], at[1], at[2], 0., 0., 0., 0., 1.]
}

/// DTMatrix::SetToRotation (same formula as camera::DTMatrix::SetToRotation), column-major.
pub fn SetToRotation(e: [f32; 3]) -> Mat4 {
    let mut m = crate::engine::camera::DTMatrix::default();
    m.SetToRotation(e);
    let (r, u, a) = (m.right, m.up, m.at);
    [r[0], r[1], r[2], 0., u[0], u[1], u[2], 0., a[0], a[1], a[2], 0., 0., 0., 0., 1.]
}

/// An RpSkin inverse bone matrix as streamed (RwMatrix with flag/pad words) -> clean Mat4.
pub fn SkinInverseMatrix(m: &[f32; 16]) -> Mat4 {
    let mut r = *m;
    r[3] = 0.0;
    r[7] = 0.0;
    r[11] = 0.0;
    r[15] = 1.0;
    r
}

/// The skin matrix palette the PS2 skin pipeline uploads (replacement for the RpSkin pipeline):
/// bone i -> inverse(atomic LTM) * LTM(node i frame) * inverseBind(i), i.e. atomic-local skinning.
pub fn SkinMatrices(skin: &RpSkin, h: &RpHAnimHierarchy, t: &FrameTree, atomic_frame: usize) -> Vec<Mat4> {
    let inv_atomic = invert_affine(&t.GetLTM(atomic_frame));
    (0..skin.num_bones as usize)
        .map(|i| {
            let ltm = h.node_frames.get(i).copied().flatten().map(|f| t.GetLTM(f)).unwrap_or(IDENTITY);
            let ib = skin.inverse_matrices.get(i).map(SkinInverseMatrix).unwrap_or(IDENTITY);
            mat_mul(&inv_atomic, &mat_mul(&ltm, &ib))
        })
        .collect()
}

/// RwMatrixInvert (general affine 3x3 + translation).
pub fn invert_affine(m: &Mat4) -> Mat4 {
    let (a, b, c) = (m[0], m[4], m[8]);
    let (d, e, f) = (m[1], m[5], m[9]);
    let (g, h, i) = (m[2], m[6], m[10]);
    let det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
    if det.abs() < 1e-20 {
        return IDENTITY;
    }
    let id = 1.0 / det;
    let r = [
        (e * i - f * h) * id,
        (c * h - b * i) * id,
        (b * f - c * e) * id,
        (f * g - d * i) * id,
        (a * i - c * g) * id,
        (c * d - a * f) * id,
        (d * h - e * g) * id,
        (b * g - a * h) * id,
        (a * e - b * d) * id,
    ];
    // r is row-major inverse; write column-major
    let mut o = IDENTITY;
    for row in 0..3 {
        for col in 0..3 {
            o[col * 4 + row] = r[row * 3 + col];
        }
    }
    let p = [m[12], m[13], m[14]];
    for row in 0..3 {
        o[12 + row] = -(o[row] * p[0] + o[4 + row] * p[1] + o[8 + row] * p[2]);
    }
    o
}

/// RwV3dTransformPoints with one point.
pub fn transform_point(m: &Mat4, p: [f32; 3]) -> [f32; 3] {
    [
        m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12],
        m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13],
        m[2] * p[0] + m[6] * p[1] + m[10] * p[2] + m[14],
    ]
}

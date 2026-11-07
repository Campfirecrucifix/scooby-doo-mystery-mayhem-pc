//! EF3dObj render side (Classes/EF/EF3dObj.cpp Render 0x002235xx, SetClippingBSphere 0x00223fb0,
//! GetClippingBSphere 0x002240e0, Initialize's atomic set-up; Classes/EF/EF3dObj_SP.cpp SetVisible,
//! EmptyRender, LodGetAtomic, SetBlendingColor, BlendMaterialColorCB, RestoreMaterialColorCB).
//!
//! 1:1: the atomic walk order, the render-flag/LOD test, the blend-colour material callbacks
//! (incl. the float->int conversion quirk), the stencil toggles around owners with flag 0x8000,
//! the tex-fader branch, the "shorelineclip" z-write override, texture-anim apply/restore.
//! Replaced: RxPipelineExecute / the atomic render callback (PS2 DFF-mapper pipelines) emit a
//! `DrawCmd` (snapshot of geometry, LTM, material colours/textures, skin palette, blend state)
//! into `EFGraphicsMgr::draws`; the GPU work happens in the Bevy adapter / coordinator.
#![allow(non_snake_case, non_camel_case_types)]

use super::hierarchy::*;
use super::mgr::EFGraphicsMgr;
use super::EF3dObj;
use crate::engine::rw::clump::RpClump;

/// rpATOMICRENDER (atomic +0x2 byte bit 4)
pub const rpATOMICRENDER: u32 = 4;
/// rpGEOMETRYMODULATEMATERIALCOLOR
pub const rpGEOMETRYMODULATEMATERIALCOLOR: u32 = 0x40;

/// The atomic's pipeline (atomic +0x6c), chosen by EF3dObj::Initialize.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AtomicPipeline {
    /// RwEngine default (+0x6c == 0 -> the global atomic pipeline)
    #[default]
    Default,
    /// EFProjectorMgr_SP::GetDFFMapperPipeline
    DFFMapper,
    /// EFProjectorMgr_SP::GetSkinnedDFFMapperPipeline
    SkinnedDFFMapper,
}

/// Atomic render callbacks the game installs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomicRenderCB {
    /// AtomicDefaultRenderCallBack (RW): runs the atomic's pipeline
    Default,
    /// EF3dObj_SP::EmptyRender: returns the atomic, draws nothing
    Empty,
}

/// Per-instance RpAtomic state the render code reads/writes (clump atomics are cloned per
/// instance by RpClumpClone; geometry/materials stay shared).
#[derive(Debug, Clone)]
pub struct AtomicRenderState {
    /// index in `RpClump::atomics`
    pub atomic: usize,
    /// atomic +0x4 frame (index in the clump frame list)
    pub frame: usize,
    /// atomic +0x18 geometry (index in `RpClump::geometries`)
    pub geometry: usize,
    /// atomic +0x2 flags byte (rpATOMICCOLLISIONTEST 1, rpATOMICRENDER 4)
    pub flags: u32,
    /// ApAtomicData plugin +0x4 (LOD in bits 16..19, see ApGetAtomicLod)
    pub ap_flags: u32,
    /// ApAtomicData plugin +0x0 (ApGetAtomicCallBackRender)
    pub callback_render: Option<AtomicRenderCB>,
    /// atomic +0x48 renderCallBack
    pub render_cb: AtomicRenderCB,
    /// atomic +0x6c pipeline
    pub pipeline: AtomicPipeline,
    /// atomic +0x1c bounding sphere (frame-local centre, radius)
    pub bsphere: [f32; 4],
    /// atomic +0x4c interpolator flags (bit 2: sphere needs resync)
    pub interp_flags: u32,
}

impl AtomicRenderState {
    /// ApGetAtomicLod
    pub fn ApGetAtomicLod(&self) -> u32 {
        (self.ap_flags & 0xf0000) >> 16
    }
    /// ApGetAtomicFlags (u16)
    pub fn ApGetAtomicFlags(&self) -> u32 {
        self.ap_flags & 0xffff
    }
    /// ApSetAtomicFlags. quirk: stores flags & 0xffff, wiping the LOD bits.
    pub fn ApSetAtomicFlags(&mut self, f: u32) {
        self.ap_flags = f & 0xffff;
    }
}

/// One material as drawn (snapshot at RxPipelineExecute time).
#[derive(Debug, Clone, PartialEq)]
pub struct DrawMaterial {
    /// RpMaterial colour (+0x4) after blending
    pub color: [u8; 4],
    /// RpMaterial texture name (+0x0), possibly swapped by the tex fader
    pub texture: Option<String>,
    /// EFTexFader material plugin (flags, alpha) (0x69690000 plugin: +0 flags, +4 alpha)
    pub fade: (u32, f32),
}

/// Owner-supplied blend mode (EFWorld::RenderObjectsTransparent, PS2 GS ALPHA register).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendMode {
    /// opaque / default alpha blend (Cs-Cd)*As+Cd
    #[default]
    Normal,
    /// flag 0x10: ALPHA 0x44 (Cs*As + Cd) additive
    Additive,
    /// flag 0x100: ALPHA 0x68 (Cs*FIX + Cd) additive, fixed alpha
    AdditiveFix,
    /// flag 0x200: ALPHA 0xa1 ((Cd-Cs)*FIX) subtractive
    Subtractive,
}

/// The replacement of one RxPipelineExecute(atomic): everything the GPU side needs.
#[derive(Debug, Clone)]
pub struct DrawCmd {
    /// EF3dObj owner uid (EFGameObject uid)
    pub owner: u32,
    pub atomic: usize,
    pub geometry: usize,
    /// world LTM of the atomic's frame
    pub ltm: Mat4,
    /// geometry flags at draw time (|0x40 while blending)
    pub geometry_flags: u32,
    pub materials: Vec<DrawMaterial>,
    pub pipeline: AtomicPipeline,
    /// atomic-local skin palette (SkinnedDFFMapper atomics with a hierarchy)
    pub skin: Option<Vec<Mat4>>,
    /// EnableStencilWrite(1) active
    pub stencil_write: bool,
    /// render state 8 (z write) forced on ("shorelineclip")
    pub zwrite_forced: bool,
    /// tex fader global (uRam003440e4: 0xc0000000 first pass, 0x80000000 second pass, 0 normal)
    pub fade_pass: u32,
    pub blend: BlendMode,
    pub lod: u32,
}

/// Seam to WP-D's EFTextureAnimNode (EF3dObj this+0x60 list): ApplyText before / RestoreText
/// after drawing (they swap material textures). Default: no texture animation.
pub trait TexAnimSeam {
    fn ApplyText(&mut self, _clump: &mut RpClump, _lod: u32) {}
    fn RestoreText(&mut self, _clump: &mut RpClump, _lod: u32) {}
}
pub struct NoTexAnim;
impl TexAnimSeam for NoTexAnim {}

/// What Render reads from the owner game object (EF3dObj this+0x10).
#[derive(Debug, Clone, Default)]
pub struct OwnerInfo {
    /// EFGameObject uid (+0x8)
    pub uid: u32,
    /// EFGameObject +0x15c flags (0x8000 = stencil write, 0x3000 = not rendered)
    pub flags_15c: u32,
    /// descriptor name (+0x38 -> +0x58)
    pub name: String,
    /// the owner frame's LTM (+0x14)
    pub ltm: Mat4,
}

/// EF3dObj_SP (and EF3dObj's render-side fields).
#[derive(Debug, Clone)]
pub struct EF3dObj_SP {
    pub base: EF3dObj,
    /// resource uid of the clump (this+0x28 -> EF3dObjResData)
    pub res_uid: u32,
    /// this+0x30 clump: per-instance atomics, in RW clump list order (RpClumpAddAtomic links at
    /// the head, so streamed atomics are visited in reverse stream order)
    pub atomics: Vec<AtomicRenderState>,
    /// this+0x14 frame tree (EFPosData::CreateFrame(clump root))
    pub frames: FrameTree,
    /// this+0x34 hierarchy, this+0x50/+0x54 transfos
    pub hier: EF3dObjHierarchy,
    /// this+0x38
    pub field_38: u8,
    /// this+0x3c..+0x48 blending colour (r,g,b,a), 1.0 each
    pub blend: [f32; 4],
    /// per-instance blend mode chosen from base.flags by the transparent pass
    pub blend_mode: BlendMode,
}

impl EF3dObj_SP {
    /// EF3dObj_SP::EF3dObj_SP + EF3dObj::Initialize (atomic part): RpClump bound, frame tree
    /// created, InitHierarchy, creation flags, then per atomic: ApSetAtomicCallBackRender(render
    /// callback), render callback = EmptyRender (quirk: the `== 0` check after it never fires),
    /// pipeline = skinned/unskinned DFF mapper. Emitters (PLMgr_BC::LoadEmitters) are WP-I's.
    pub fn Initialize(res_uid: u32, clump: &RpClump, creation_flags: u32) -> EF3dObj_SP {
        let mut o = EF3dObj_SP {
            base: EF3dObj::default(),
            res_uid,
            atomics: Vec::new(),
            frames: FrameTree::from_clump(clump),
            hier: EF3dObjHierarchy::default(),
            field_38: 0,
            blend: [1.0; 4],
            blend_mode: BlendMode::Normal,
        };
        o.base.SetRessource(res_uid);
        let mut flags = o.base.flags;
        o.hier.InitHierarchy(clump, &o.frames, &mut flags);
        o.base.flags = flags;
        o.base.num_nodes = o.hier.hierarchy.as_ref().map(|h| h.num_nodes).unwrap_or(0);
        if creation_flags & 0x68 != 0 {
            o.base.ApplyCreationFlags(creation_flags);
        }
        // EFWorld::RenderObjectsTransparent picks the GS ALPHA mode from these flags (0x10 > 0x100 >
        // 0x200) for every object queued in the transparent buckets (P26 R16: EP6/L03 electric arcs)
        let f = o.base.flags;
        o.blend_mode = if f & 0x10 != 0 {
            BlendMode::Additive
        } else if f & 0x100 != 0 {
            BlendMode::AdditiveFix
        } else if f & 0x200 != 0 {
            BlendMode::Subtractive
        } else {
            BlendMode::Normal
        };
        // P33 R6 debug: GAME_BLEND_LOG=1 reports objects whose flags select the subtractive GS mode 0xa1
        if o.blend_mode == BlendMode::Subtractive && std::env::var_os("GAME_BLEND_LOG").is_some() {
            eprintln!("P33 blend 0xa1 object flags {f:#x} creation {creation_flags:#x}");
        }
        for (i, a) in clump.atomics.iter().enumerate().rev() {
            let g = clump.geometries.get(a.geometry as usize);
            let skinned = g.is_some_and(|g| g.skin.is_some());
            let bs = g.and_then(|g| g.morph_targets.first()).map(|m| m.bsphere).unwrap_or([0.0; 4]);
            o.atomics.push(AtomicRenderState {
                atomic: i,
                frame: a.frame as usize,
                geometry: a.geometry as usize,
                flags: a.flags,
                ap_flags: a.ap_flags.unwrap_or(0) as u32,
                callback_render: Some(AtomicRenderCB::Default),
                render_cb: AtomicRenderCB::Empty,
                pipeline: if skinned { AtomicPipeline::SkinnedDFFMapper } else { AtomicPipeline::DFFMapper },
                bsphere: bs,
                interp_flags: 0,
            });
        }
        o
    }

    /// EF3dObj_SP::SetVisible: sets/clears rpATOMICRENDER on every atomic.
    pub fn SetVisible(&mut self, v: u32) {
        for a in self.atomics.iter_mut() {
            if v == 0 {
                a.flags &= !rpATOMICRENDER;
            } else {
                a.flags |= rpATOMICRENDER;
            }
        }
    }

    /// EF3dObj_SP::LodGetAtomic: first atomic (list order) whose LOD == lod.
    pub fn LodGetAtomic(&self, lod: u32) -> Option<usize> {
        self.atomics.iter().position(|a| a.ApGetAtomicLod() == lod)
    }

    /// EF3dObj_SP::SetBlendingColor: flag 0x400 unless all four are exactly 1.0.
    pub fn SetBlendingColor(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.blend = [r, g, b, a];
        if r == 1.0 && g == 1.0 && b == 1.0 && a == 1.0 {
            self.base.flags &= !0x400;
        } else {
            self.base.flags |= 0x400;
        }
    }

    /// EF3dObj::GetJointIndex with this instance's hierarchy.
    pub fn GetJointIndex(&self, id: u32) -> u32 {
        let ids = self.hier.hierarchy.as_ref().map(|h| h.node_ids.as_slice()).unwrap_or(&[]);
        self.base.GetJointIndex(id, ids)
    }

    /// EF3dObj::SetClippingBSphere: sphere centre (in the space of hierarchy node 0's frame when
    /// the object has a hierarchy) -> world -> each atomic's frame space; radius copied.
    /// quirk: without a hierarchy the C++ calls RwFrameGetLTM(NULL); identity is used here.
    pub fn SetClippingBSphere(&mut self, s: [f32; 4]) {
        for i in 0..self.atomics.len() {
            let node0 = if self.base.flags & 4 != 0 {
                self.hier.hierarchy.as_ref().and_then(|h| h.node_frames.first().copied().flatten())
            } else {
                None
            };
            let m = node0.map(|f| self.frames.GetLTM(f)).unwrap_or(IDENTITY);
            let world = transform_point(&m, [s[0], s[1], s[2]]);
            let inv = invert_affine(&self.frames.GetLTM(self.atomics[i].frame));
            let local = transform_point(&inv, world);
            // _rpAtomicResyncInterpolatedSphere when +0x4c & 2 (no interpolation here)
            self.atomics[i].interp_flags &= !2;
            self.atomics[i].bsphere = [local[0], local[1], local[2], s[3]];
        }
    }

    /// EF3dObj::GetClippingBSphere: the first atomic's sphere (atomic +0x1c).
    pub fn GetClippingBSphere(&mut self) -> Option<[f32; 4]> {
        let a = self.atomics.first_mut()?;
        a.interp_flags &= !2;
        Some(a.bsphere)
    }

    /// EF3dObj::LightObject over this instance's geometries.
    pub fn LightObject(clump: &mut RpClump, on: u32) {
        EF3dObj::LightObject(clump, on != 0);
    }

    /// EF3dObj_SP::BlendMaterialColorCB for every material of a geometry (RpGeometryForAllMaterials
    /// with the global save index uRam00343ed0 reset to 0 by the caller).
    /// quirk: colour * factor goes through the MIPS float->unsigned conversion (values >= 2^31
    /// subtract 2^31) and is truncated to a byte (no clamping: factors > 1 wrap).
    pub fn BlendMaterialColorCB(&self, clump: &mut RpClump, geom: usize, saved: &mut Vec<[u8; 4]>) {
        let Some(g) = clump.geometries.get_mut(geom) else { return };
        for m in g.materials.iter_mut() {
            saved.push(m.color);
            for c in 0..4 {
                let f = m.color[c] as f32 * self.blend[c];
                let v = if f < 2147483648.0 { f as i32 } else { (f - 2147483648.0) as i32 };
                m.color[c] = v as u8;
            }
        }
    }

    /// EF3dObj_SP::RestoreMaterialColorCB
    pub fn RestoreMaterialColorCB(clump: &mut RpClump, geom: usize, saved: &[[u8; 4]]) {
        let Some(g) = clump.geometries.get_mut(geom) else { return };
        for (m, s) in g.materials.iter_mut().zip(saved) {
            m.color = *s;
        }
    }

    /// Owner LTM -> frame tree LTMs (RwFrameUpdateObjects on the owner frame).
    pub fn SyncFrames(&mut self, owner_ltm: Mat4) {
        self.frames.root_parent = owner_ltm;
        self.frames.UpdateLTMs();
    }

    /// The RxPipelineExecute replacement: snapshot this atomic into a DrawCmd.
    pub fn ExecutePipeline(&self, clump: &RpClump, ai: usize, owner: &OwnerInfo, mgr: &EFGraphicsMgr, fade_pass: u32, lod: u32) -> DrawCmd {
        let a = &self.atomics[ai];
        let g = &clump.geometries[a.geometry];
        let skin = match (&g.skin, &self.hier.hierarchy) {
            (Some(s), Some(h)) if a.pipeline == AtomicPipeline::SkinnedDFFMapper => Some(SkinMatrices(s, h, &self.frames, a.frame)),
            _ => None,
        };
        DrawCmd {
            owner: owner.uid,
            atomic: a.atomic,
            geometry: a.geometry,
            ltm: self.frames.GetLTM(a.frame),
            geometry_flags: g.flags,
            materials: g
                .materials
                .iter()
                .enumerate()
                .map(|(mi, m)| DrawMaterial {
                    color: m.color,
                    texture: m.texture.as_ref().map(|t| t.name.clone()),
                    fade: mgr.tex_fader.MaterialPlugin(self.res_uid, a.geometry, mi),
                })
                .collect(),
            pipeline: a.pipeline,
            skin,
            stencil_write: mgr.stencil.write,
            zwrite_forced: mgr.zwrite_forced,
            fade_pass,
            blend: self.blend_mode,
            lod,
        }
    }

    /// EF3dObj::Render(lod): for every atomic with rpATOMICRENDER whose LOD == lod & 7.
    pub fn Render(&mut self, lod: u16, clump: &mut RpClump, owner: &OwnerInfo, mgr: &mut EFGraphicsMgr, texanim: &mut dyn TexAnimSeam) {
        let lod = (lod & 7) as u32;
        for ai in 0..self.atomics.len() {
            let a = &self.atomics[ai];
            if a.flags & rpATOMICRENDER == 0 || a.ApGetAtomicLod() != lod {
                continue;
            }
            let geom = a.geometry;
            let mut saved_flags = 0;
            let mut saved = Vec::new();
            if self.base.flags & 0x400 != 0 {
                if let Some(g) = clump.geometries.get_mut(geom) {
                    saved_flags = g.flags;
                    g.flags |= rpGEOMETRYMODULATEMATERIALCOLOR;
                }
                self.BlendMaterialColorCB(clump, geom, &mut saved);
            }
            if owner.flags_15c & 0x8000 != 0 {
                mgr.EnableStencilWrite(1);
            }
            if self.base.fading == 0 {
                // RwFrameUpdateObjects(atomic frame): LTMs are current (SyncFrames)
                texanim.ApplyText(clump, lod);
                match self.atomics[ai].callback_render {
                    None => {
                        let d = self.ExecutePipeline(clump, ai, owner, mgr, 0, lod);
                        mgr.draws.push(d);
                    }
                    Some(cb) => {
                        let shore = owner.name == "shorelineclip";
                        let prev = mgr.zwrite_forced;
                        if shore {
                            mgr.zwrite_forced = true; // RwRenderStateSet(8 ZWRITEENABLE, 1)
                        }
                        if cb == AtomicRenderCB::Default {
                            let d = self.ExecutePipeline(clump, ai, owner, mgr, 0, lod);
                            mgr.draws.push(d);
                        }
                        if shore {
                            mgr.zwrite_forced = prev;
                        }
                    }
                }
                texanim.RestoreText(clump, lod);
            } else {
                let mut fader = std::mem::take(&mut mgr.tex_fader);
                fader.FadeAtomic(self, ai, lod, clump, owner, mgr);
                mgr.tex_fader = fader;
            }
            if owner.flags_15c & 0x8000 != 0 {
                mgr.EnableStencilWrite(0);
            }
            if self.base.flags & 0x400 != 0 {
                if let Some(g) = clump.geometries.get_mut(geom) {
                    g.flags = saved_flags;
                }
                Self::RestoreMaterialColorCB(clump, geom, &saved);
            }
        }
    }
}

/// EF3dObj pose + frame update of one game object (bin/game.rs draw_objects; P21 R69 moved here so
/// the headless test harness can run it too): applies the anim pose (`apply_pose`), syncs the frames
/// to `ltm` and returns the HAnim node frames (modelling matrix, LTM) + node ids published to
/// RuntimeWorld::nodes / node_ids (PhysicWorld::GetBone: bone-attached coll prims follow them).
pub fn pose_nodes(o: &mut EF3dObj_SP, c: &RpClump, pose: Option<&[crate::engine::anim::res::DTTransfo]>, ltm: Mat4) -> Option<(Vec<(crate::engine::camera::DTMatrix, crate::engine::camera::DTMatrix)>, Vec<i32>)> {
    use crate::engine::camera::DTMatrix;
    if let Some(pose) = pose {
        apply_pose(o, c, pose, ltm);
    }
    o.SyncFrames(ltm);
    let h = o.hier.hierarchy.as_ref()?;
    let m = |a: &Mat4| DTMatrix { right: [a[0], a[1], a[2]], up: [a[4], a[5], a[6]], at: [a[8], a[9], a[10]], pos: [a[12], a[13], a[14]] };
    let nodes = h.node_frames.iter().map(|f| f.map_or((m(&ltm), m(&ltm)), |f| (m(&o.frames.local[f]), m(&o.frames.GetLTM(f))))).collect();
    Some((nodes, h.node_ids.clone()))
}

/// P28 R11: logic-side node posing. EFAnimationMgr::PlayHAnim (0x001e9390) runs UpdateHierarchyAnimNode at
/// once and the hierarchy frames are the EF3dObj's own RwFrames, so a GetObjectFrame(joint) later in the same
/// logic pass sees the new anim's pose (heroInit_EdgeJump reads the leader's hand 0x2500000 right after the
/// leader's EdgeIn played the climb anim). The renderer / headless publisher register each clump here the first
/// time they pose an object; [`NodePoser::Repose`] then re-poses that object from the logic side.
#[derive(Debug, Clone, Default)]
pub struct NodePoser {
    /// resource uid -> clump (rest frames + HAnim hierarchy)
    pub clumps: std::collections::HashMap<u32, RpClump>,
    /// game object uid -> its posing EF3dObj (independent of the renderer's)
    pub objs: std::collections::HashMap<u32, EF3dObj_SP>,
}

impl NodePoser {
    /// Node frames (modelling, LTM) + HAnim ids of `uid` for `pose` at object LTM `ltm`; None without a
    /// registered clump / hierarchy.
    pub fn Repose(&mut self, uid: u32, res_uid: u32, pose: Option<&[crate::engine::anim::res::DTTransfo]>, ltm: Mat4) -> Option<(Vec<(crate::engine::camera::DTMatrix, crate::engine::camera::DTMatrix)>, Vec<i32>)> {
        let c = self.clumps.get(&res_uid)?;
        let o = self.objs.entry(uid).or_insert_with(|| {
            let mut o = EF3dObj_SP::Initialize(res_uid, c, 0);
            o.SetVisible(1);
            o
        });
        pose_nodes(o, c, pose, ltm)
    }
}

/// HAnim pose -> EF3dObj transfo pool 0 -> EF3dObj::UpdateHierarchy (as the viewer: pose
/// transfos carry their hierarchy node index, nodes the anim does not drive keep the clump rest).
pub fn apply_pose(o: &mut EF3dObj_SP, c: &RpClump, pose: &[crate::engine::anim::res::DTTransfo], ltm: Mat4) {
    let n = o.hier.transfos.first().map_or(0, |t| t.len());
    if pose.is_empty() || !pose.iter().all(|t| (t.id as usize) < n && t.id >= 0) {
        return;
    }
    for t in pose {
        o.hier.transfos[0][t.id as usize] = t.clone();
    }
    o.base.flags |= 0x80;
    let flags = o.base.flags;
    let mut owner = ltm;
    o.hier.UpdateHierarchy(flags, &mut o.frames, &mut owner);
    if let Some(h) = &o.hier.hierarchy {
        for (i, f) in h.node_frames.iter().enumerate() {
            if let Some(f) = f {
                if !pose.iter().any(|t| t.id as usize == i) {
                    if let Some(r) = c.frames.get(*f) {
                        o.frames.local[*f] = r.local_matrix();
                    }
                }
            }
        }
    }
}

/// P22 R26 (GAME_LOGICCLIP): the clip spheres EFWorld::ClipFrustumTest reads, one per atomic (atomic +0x1c
/// centre through its frame LTM, radius +0x28), here in object space with the clump at its REST pose
/// (deviation: the C uses the animated atomic frames; the world transform is applied per frame).
pub fn clip_spheres(res_uid: u32, c: &RpClump) -> Vec<[f32; 4]> {
    let mut o = EF3dObj_SP::Initialize(res_uid, c, 0);
    o.SyncFrames(IDENTITY);
    o.atomics
        .iter()
        .map(|a| {
            let p = transform_point(&o.frames.GetLTM(a.frame), [a.bsphere[0], a.bsphere[1], a.bsphere[2]]);
            [p[0], p[1], p[2], a.bsphere[3]]
        })
        .collect()
}

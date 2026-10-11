//! EFGraphicsMgr / EFGraphicsMgr_SP manager logic (Classes/EF/EFGraphicsMgr*.cpp), EFWorld_SP's
//! static-world slot (Classes/EF/EFWorld_SP.cpp) and EFWorld's object render queue
//! (EFWorld::RenderObjects 0x002709a0 / RenderObjectsTransparent 0x00270bc0).
//!
//! 1:1: Render/RenderCamera call order and flag tests, camera slots (via WP-I's
//! `camera::EFGraphicsMgrCameras`), Pause, SetRenderState (fog), the stencil toggles, the
//! transparent depth-sorted 64-bucket queue and its blend modes, SetStatic3dObj.
//! Replaced: GS register packets (stencil = PS2 destination-alpha tricks: FBMSK alpha write,
//! TEST DATE read) become `StencilState`; RwRenderStateSet fog becomes `FogState`; calls into
//! other managers (dome, map, sprites, particles, shadows, projectors, HUD, slideshow, flip) are
//! `RenderPass` values handed to a `RenderSeam` implemented by the coordinator.
#![allow(non_snake_case, non_camel_case_types)]

use super::hierarchy::{invert_affine, transform_point, Mat4};
use super::render::{BlendMode, DrawCmd};
use super::texfader::EFTexFader_SP;
use super::EFStatic3dObj;
use crate::engine::camera::{EFCamera, EFGraphicsMgrCameras};
use crate::engine::rw::image::RwTexDictionary;
use crate::engine::rw::world::RpWorld;

/// PS2 "stencil" (destination alpha) state.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StencilState {
    /// EnableStencilWrite: FBMSK alpha byte unmasked
    pub write: bool,
    /// EnableStencilRead: TEST.DATE on (0x54000 vs 0x50000)
    pub read: bool,
    /// ClearStencil calls this frame
    pub clears: u32,
}

/// RwRenderStateSet(FOGENABLE 0xe, FOGCOLOR 0xf, FOGTYPE 0x10, FOGDENSITY 0x11)
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FogState {
    pub enabled: bool,
    pub color: u32,
    pub fog_type: u32,
    pub density: f32,
}

/// The external calls of EFGraphicsMgr::Render / RenderCamera, in call order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderPass {
    ProjectorBegin,
    ShadowBegin(u32),
    CameraBegin(u32),
    RenderDome(u32),
    RenderMap(u32),
    FBEffects(u32, u32),
    ProjectorEnd,
    ShadowEnd,
    /// EFWorld::RenderObjects(camera slot): opaque objects drawn, transparent ones queued
    RenderObjects(u32),
    SpritesNormal,
    RenderObjectsTransparent(u32),
    SpritesTransparent,
    SpritesOccluded,
    RenderMapTransparent,
    Lines,
    Particles,
    Billboards,
    MotionTrails,
    LensFlare(u32),
    CameraEnd(u32),
    HudUpdate(f32),
    SlideShow,
    Flip,
}

/// Coordinator hook for the passes owned by other managers.
pub trait RenderSeam {
    fn pass(&mut self, mgr: &mut EFGraphicsMgr, p: RenderPass);
}
pub struct NoSeam;
impl RenderSeam for NoSeam {
    fn pass(&mut self, _: &mut EFGraphicsMgr, _: RenderPass) {}
}

/// EFGraphicsMgr (EFGraphicsMgr_BC + _SP + EFGraphicsMgr).
#[derive(Debug, Clone)]
pub struct EFGraphicsMgr {
    /// this+0x4 camera slots (2), this+0x6c count
    pub cameras: EFGraphicsMgrCameras,
    /// this+0x34 running (Pause(p) -> !p)
    pub running: u32,
    /// this+0x5c EFTextureDict (the current RwTexDictionary)
    pub tex_dict: RwTexDictionary,
    /// this+0x68 EFTexFader
    pub tex_fader: EFTexFader_SP,
    pub stencil: StencilState,
    pub fog: FogState,
    /// render state 8 (z write) override ("shorelineclip" objects)
    pub zwrite_forced: bool,
    /// the frame's draw list (RxPipelineExecute replacement)
    pub draws: Vec<DrawCmd>,
    /// queued EFNotificationMgr::Notify(type, a, b, c)
    pub notifications: Vec<(u32, u32, u32, u32)>,
    /// passes issued this frame (debug / tests)
    pub passes: Vec<RenderPass>,
}

impl EFGraphicsMgr {
    /// EFGraphicsMgr::EFGraphicsMgr(EFAppContext) (screen size = this+0x38/+0x3c).
    pub fn new(screen_w: u32, screen_h: u32) -> Self {
        EFGraphicsMgr {
            cameras: EFGraphicsMgrCameras::new(screen_w, screen_h),
            running: 1,
            tex_dict: RwTexDictionary::default(),
            tex_fader: EFTexFader_SP::default(),
            stencil: StencilState::default(),
            fog: FogState::default(),
            zwrite_forced: false,
            draws: Vec::new(),
            notifications: Vec::new(),
            passes: Vec::new(),
        }
    }

    fn issue(&mut self, seam: &mut dyn RenderSeam, p: RenderPass) {
        self.passes.push(p);
        seam.pass(self, p);
    }

    /// EFGraphicsMgr::Render(dt): every active camera (flags & 1) is rendered; HUD updated after
    /// cameras with render flag 0x80; then slideshow and flip. Draw/pass lists are reset here.
    pub fn Render(&mut self, _dt: f32, seam: &mut dyn RenderSeam) {
        self.draws.clear();
        self.passes.clear();
        self.stencil.clears = 0;
        if self.running == 0 {
            return;
        }
        for slot in 0..2u32 {
            let Some((flags, rf)) = self.cameras.cameras[slot as usize].as_ref().map(|c| (c.flags, c.render_flag)) else { continue };
            if flags & 1 != 0 {
                self.RenderCamera(slot, seam);
            }
            if rf & 0x80 != 0 {
                self.issue(seam, RenderPass::HudUpdate(0.033));
            }
        }
        self.issue(seam, RenderPass::SlideShow);
        self.Flip(seam);
    }

    /// EFGraphicsMgr::RenderCamera (EFCamera::Clear is the coordinator's CameraBegin).
    pub fn RenderCamera(&mut self, slot: u32, seam: &mut dyn RenderSeam) {
        let Some(cam) = self.cameras.cameras[slot as usize].clone() else { return };
        let f = cam.render_flag;
        if f == 0 {
            return;
        }
        self.issue(seam, RenderPass::ProjectorBegin);
        self.issue(seam, RenderPass::ShadowBegin(slot));
        self.issue(seam, RenderPass::CameraBegin(slot));
        self.SetRenderState(&cam);
        self.ClearStencil();
        self.EnableStencilWrite(0);
        self.EnableStencilRead(0);
        self.issue(seam, RenderPass::RenderDome(slot));
        if f & 0x40 != 0 {
            self.issue(seam, RenderPass::RenderMap(slot));
        }
        self.issue(seam, RenderPass::FBEffects(4, slot));
        self.issue(seam, RenderPass::ProjectorEnd);
        self.issue(seam, RenderPass::ShadowEnd);
        if f & 0x20 == 0 {
            self.issue(seam, RenderPass::SpritesNormal);
        } else {
            self.issue(seam, RenderPass::RenderObjects(slot));
            self.issue(seam, RenderPass::SpritesNormal);
            self.issue(seam, RenderPass::RenderObjectsTransparent(slot));
        }
        self.issue(seam, RenderPass::SpritesTransparent);
        self.issue(seam, RenderPass::SpritesOccluded);
        self.issue(seam, RenderPass::RenderMapTransparent);
        self.issue(seam, RenderPass::Lines);
        if f & 4 != 0 {
            self.issue(seam, RenderPass::Particles);
            self.issue(seam, RenderPass::Billboards);
        }
        self.issue(seam, RenderPass::MotionTrails);
        self.issue(seam, RenderPass::LensFlare(slot));
        self.issue(seam, RenderPass::FBEffects(8, slot));
        self.issue(seam, RenderPass::CameraEnd(slot));
    }

    /// EFGraphicsMgr::GetCamera(uid): slot uid & 0xfff; -1 -> None.
    pub fn GetCamera(&mut self, uid: u32) -> Option<&mut EFCamera> {
        self.cameras.GetCamera(uid)
    }

    /// EFGraphicsMgr::Terminate (HUD/shadow/projector/lens-flare terminate are the coordinator's).
    pub fn Terminate(&mut self) {
        self.tex_fader.Terminate();
    }

    /// EFGraphicsMgr_SP::Pause: this+0x34 = !p (RpSkySuspend/Resume and the 30 fps VSYNC handler
    /// are platform-only).
    pub fn Pause(&mut self, p: u32) {
        self.running = (p == 0) as u32;
    }

    /// EFGraphicsMgr_SP::SetRenderState(camera): fog from camera +0x244/+0x2c/+0x248/+0x24c.
    pub fn SetRenderState(&mut self, c: &EFCamera) {
        self.fog = if c.fog_enabled == 0 {
            FogState { enabled: false, ..self.fog.clone() }
        } else {
            FogState { enabled: true, color: c.fog_color, fog_type: c.fog_type, density: c.fog_density }
        };
    }

    /// EFGraphicsMgr_SP::Flip
    pub fn Flip(&mut self, seam: &mut dyn RenderSeam) {
        self.issue(seam, RenderPass::Flip);
    }

    /// EFGraphicsMgr_SP::EnableStencilWrite
    pub fn EnableStencilWrite(&mut self, on: u32) {
        self.stencil.write = on != 0;
    }
    /// EFGraphicsMgr_SP::EnableStencilRead
    pub fn EnableStencilRead(&mut self, on: u32) {
        self.stencil.read = on != 0;
    }
    /// EFGraphicsMgr_SP::ClearStencil: destination alpha cleared over the whole frame (the packet
    /// leaves TEST at 0x50000 = read off).
    pub fn ClearStencil(&mut self) {
        self.stencil.clears += 1;
        self.stencil.read = false;
    }
}

/// EFWorld_SP: +0x0 EFStatic3dObj*, +0x4 env-map list.
#[derive(Default)]
pub struct EFWorld_SP {
    pub static3d: Option<EFStatic3dObj>,
    /// EFEnvMap uids (WP-I)
    pub env_maps: Vec<u32>,
}

impl EFWorld_SP {
    /// EFWorld_SP::Terminate: deletes the env maps and the static object (Terminate + delete).
    pub fn Terminate(&mut self) {
        self.env_maps.clear();
        if let Some(mut s) = self.static3d.take() {
            s.Terminate();
        }
    }
    pub fn AddEnvMap(&mut self, uid: u32) {
        self.env_maps.push(uid);
    }
    pub fn GetStatic3dObj(&self) -> Option<&EFStatic3dObj> {
        self.static3d.as_ref()
    }
    /// EFWorld_SP::SetStatic3dObj: Terminate() first (quirk: also deletes the env maps).
    pub fn SetStatic3dObj(&mut self, s: Option<EFStatic3dObj>) {
        self.Terminate();
        self.static3d = s;
    }
    /// EFWorld_SP::GetCurrentWorld: the static object's RpWorld (+0x14).
    pub fn GetCurrentWorld(&self) -> Option<&RpWorld> {
        self.static3d.as_ref()?.world.as_ref()
    }
}

/// What EFWorld::RenderObjects reads from one game object of the world list (+0x4144 / +0x164).
#[derive(Debug, Clone, Default)]
pub struct RenderObjInfo {
    pub uid: u32,
    /// +0x15c (0x3000 = skipped)
    pub flags_15c: u32,
    /// +200 (1 = 3D object)
    pub render_type: u32,
    /// Get3dObject(0) != NULL
    pub has_3dobj: bool,
    /// EF3dObj +0x2c
    pub obj_flags: u32,
    /// EF3dObj +0x58 bucket (0..63; 0x20 by default)
    pub bucket: u32,
    /// EF3dObj frame LTM position
    pub pos: [f32; 3],
    /// +0x110 + camera*8: LOD per camera
    pub lod: [u16; 2],
}

/// EFWorld's transparent queue (+0x1c 64 bucket heads, +0x11c 0x10-byte nodes, +0x411c count).
#[derive(Debug, Clone, Default)]
pub struct EFWorldRenderQueue {
    /// per bucket: (object index, camera-space z), sorted as the C++ inserts them
    pub buckets: Vec<Vec<(usize, f32)>>,
    pub count: u32,
}

impl EFWorldRenderQueue {
    /// EFWorld::RenderObjects: returns the objects to render now (Render(lod) via vtable 0x70)
    /// as (index, lod); objects with flags 0x310 (blend modes) are queued per bucket, sorted by
    /// camera-space z descending (back to front; equal z after existing ones).
    /// `cleared` gets every visited object index (owner +0xc &= ~0x20).
    pub fn RenderObjects(&mut self, objs: &[RenderObjInfo], cam_ltm: &Mat4, slot: u32, cleared: &mut Vec<usize>) -> Vec<(usize, u16)> {
        let inv = invert_affine(cam_ltm);
        self.count = 0;
        self.buckets = vec![Vec::new(); 64];
        let mut now = Vec::new();
        for (i, o) in objs.iter().enumerate() {
            if o.flags_15c & 0x3000 != 0 {
                continue;
            }
            if o.render_type == 1 && o.has_3dobj {
                if o.obj_flags & 0x310 == 0 {
                    now.push((i, o.lod[slot as usize & 1]));
                } else {
                    self.count += 1;
                    let z = transform_point(&inv, o.pos)[2];
                    // quirk: bucket index unchecked in the C++ (0x40 heads)
                    let b = &mut self.buckets[(o.bucket as usize) & 63];
                    let at = b.iter().position(|&(_, d)| z > d).unwrap_or(b.len());
                    b.insert(at, (i, z));
                }
            }
            cleared.push(i);
        }
        now
    }

    /// EFWorld::RenderObjectsTransparent: buckets 0..63 in order with the blend mode from the
    /// object flags (0x10 additive 0x44, else 0x100 additive-fixed 0x68, else 0x200 subtractive
    /// 0xa1); z-write off during the pass. Returns (index, lod, blend).
    pub fn RenderObjectsTransparent(&self, objs: &[RenderObjInfo], slot: u32) -> Vec<(usize, u16, BlendMode)> {
        let mut out = Vec::new();
        let mut mode = BlendMode::Normal; // RpSkyRenderState 2 keeps the last value set
        for b in &self.buckets {
            for &(i, _) in b {
                let f = objs[i].obj_flags;
                if f & 0x10 != 0 {
                    mode = BlendMode::Additive;
                } else if f & 0x100 != 0 {
                    mode = BlendMode::AdditiveFix;
                } else if f & 0x200 != 0 {
                    mode = BlendMode::Subtractive;
                }
                out.push((i, objs[i].lod[slot as usize & 1], mode));
            }
        }
        out
    }
}

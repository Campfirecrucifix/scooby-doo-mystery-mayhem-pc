//! APIgraphics_* (Engine API (module interface)/EFApi_Graphics.cpp, table 0x78 bytes, 30 entries)
//! and the engine services they need. Signatures follow api_gen.rs `Host::APIgraphics_*` plus a
//! `&dyn GfxWorld` where the C++ reads _gpoWorldMgr. Functions declared void return 0 (the v0
//! the module sees is the callee's; only MTCreateSystem's id is used by the logic).
//!
//! Calls that act on managers owned elsewhere (EFTexFader_SP inside EFGraphicsMgr, EF3dObj_SP,
//! EFPaletteMgr, EFLinesMgr) are resolved here (object lookup, parameter block read from Mem)
//! and queued as [`GfxDeferred`] for the coordinator to apply before EFGraphicsMgr's update of
//! that frame (replacement: direct call -> same-frame queue; nothing reads the result back).
//! The "pcRam00343d5c = name" debug trace is dropped.
#![allow(non_snake_case, non_camel_case_types)]

use super::motion_trail::EFMotionTrailMgr_BC;
use super::sprite::{EFSpriteMgr_BC, SpriteCamera, SpriteDraw};
use super::texfader::_sTexFaderParams;
use crate::engine::camera::DTMatrix;
use crate::engine::{Mem, Ptr};

/// The world queries of the sprite / motion-trail / API code (EFWorld, EFGameObject,
/// EF3dObj hierarchy); implemented by the coordinator.
pub trait GfxWorld {
    /// EFWorld::GetGameObject(uid) -> EFGameObject+8 (its real uid; SELF 0x2a000000 resolved to
    /// the current logic object), None = NULL
    fn ResolveGameObject(&self, uid: u32) -> Option<u32>;
    /// EFGameObject::GetObjectFrame(bone) LTM (sprites)
    fn GetObjectFrame(&self, obj: u32, bone: u32) -> Option<DTMatrix>;
    /// EFWorld::GetObjectPosData(uid)->frame LTM (sprites attached to non-0x2a uids)
    fn GetObjectPosDataFrame(&self, uid: u32) -> Option<DTMatrix>;
    /// Motion trails: Get3dObject(0); if its hierarchy flag (+0x2c & 4) is set, node = joint
    /// (or RpHAnimIDGetIndex(joint) when joint & 0xffff0000), LTM of pNodeInfo[node].frame;
    /// None = NULL frame (no hierarchy / index -1)
    fn GetJointLTM(&self, obj: u32, joint: u32) -> Option<DTMatrix>;
    /// EFGameObject::Get3dObject(i)+8 for i in 0..+200 (count); None = no such game object
    fn Get3dObjectUids(&self, obj: u32) -> Option<Vec<u32>> {
        let _ = obj;
        None
    }
}

/// World with nothing in it (tests / before the level is built).
pub struct GfxNoWorld;
impl GfxWorld for GfxNoWorld {
    fn ResolveGameObject(&self, _: u32) -> Option<u32> {
        None
    }
    fn GetObjectFrame(&self, _: u32, _: u32) -> Option<DTMatrix> {
        None
    }
    fn GetObjectPosDataFrame(&self, _: u32) -> Option<DTMatrix> {
        None
    }
    fn GetJointLTM(&self, _: u32, _: u32) -> Option<DTMatrix> {
        None
    }
}

/// Work for managers owned by the coordinator.
#[derive(Debug, Clone)]
pub enum GfxDeferred {
    /// EFTexFader_SP::FadeTo(_gpoGraphicsMgr+0x68, EF3dObj, params); `obj3d` = owner uid of the
    /// EF3dObj (the game object's Get3dObject(0), or the 3d object whose +8 == uid for 0x0f uids)
    TexFaderPlay { game_obj: u32, obj3d_index: u32, params: _sTexFaderParams },
    /// EFTexFader_BC::Pause(EF3dObj, on)
    TexFaderPause { game_obj: u32, obj3d_index: u32, on: u32 },
    /// EF3dObj_SP::SetBlendingColor(r, g, b, a) on GetGameObject(obj)->Get3dObject(index)
    SetBlendingColor { game_obj: u32, obj3d_index: u32, rgba: [f32; 4] },
    /// EF3dObj +0x38 = flag (DoCrappyXboxSpewHackThingy)
    Set3dObjField38 { game_obj: u32, obj3d_index: u32, value: u8 },
    /// EFPaletteMgr_BC::StartTransition(goPaletteMgr, _sPaletteTransitionData*) (unported)
    StartPaletteTransition { data: Ptr },
    /// EFLinesMgr::AddLines(_gpoLinesMgr, _LinesParams*) (unported)
    DrawLines { params: Ptr },
}

/// EFLensFlareFX globals touched by the API (0x1f84aa0 object).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LensFlareState {
    /// bRam01f84ac8
    pub active: bool,
    /// uRam01f84adc..e4
    pub sun_pos: [f32; 3],
    /// _goLensFlareFX (object uid)
    pub sun_object: u32,
    /// EFLensFlareFX::SetParams(param_1 & 0xff, param_2) arguments (body unported)
    pub params: Option<(u32, u32)>,
}

/// State behind the APIgraphics table: goSpriteMgr, goMotionTrailMgr, lens-flare globals,
/// bRam0034406c and the deferred queue.
#[derive(Debug, Clone, Default)]
pub struct EFGraphicsApi {
    pub sprites: EFSpriteMgr_BC,
    pub trails: EFMotionTrailMgr_BC,
    pub lens_flare: LensFlareState,
    /// bRam0034406c (DoStupidXboxShadowHack)
    pub xbox_shadow_hack: bool,
    pub deferred: Vec<GfxDeferred>,
    /// current RwTexDictionary texture names (set by the coordinator from the level TXD)
    pub textures: Vec<String>,
}

/// _sTexFaderParams from module memory: +0 flags, +4 duration, +8 src char*[3], +0x14 dst
/// char*[3], +0x20..+0x2c alphas, +0x30 notify.
pub fn read_sTexFaderParams(mem: &Mem, p: Ptr) -> _sTexFaderParams {
    let s = |a: Ptr| {
        let q = mem.r32(a);
        if q == 0 { None } else { Some(mem.cstr_at(q)) }
    };
    let f = |a: Ptr| f32::from_bits(mem.r32(a));
    _sTexFaderParams {
        flags: mem.r32(p),
        duration: f(p + 4),
        src: [s(p + 8), s(p + 0xc), s(p + 0x10)],
        dst: [s(p + 0x14), s(p + 0x18), s(p + 0x1c)],
        a_from: f(p + 0x20),
        a_to: f(p + 0x24),
        b_from: f(p + 0x28),
        b_to: f(p + 0x2c),
        notify: mem.r32(p + 0x30),
    }
}

impl EFGraphicsApi {
    pub fn new() -> Self {
        Self::default()
    }

    /// EFEngineController level unload: EFSpriteMgr_BC::Terminate + EFMotionTrailMgr_BC::Terminate
    pub fn Terminate(&mut self) {
        self.sprites.Terminate();
        self.trails.Terminate();
        self.deferred.clear();
    }

    /// RenderSeam helpers: SpritesNormal / SpritesTransparent / SpritesOccluded / MotionTrails
    pub fn RenderSprites(&self, pass: SpritePass, w: &dyn GfxWorld, cam: &SpriteCamera) -> Vec<SpriteDraw> {
        match pass {
            SpritePass::Normal => self.sprites.RenderNormal(w, cam),
            SpritePass::Transparent => self.sprites.RenderTransparent(w, cam),
            SpritePass::Occluded => self.sprites.RenderOccluded(w, cam),
        }
    }

    // ---- sprites (0x00235ca0..)
    /// SpriteAddGroup (0x00235ca0): returns the group id
    pub fn APIgraphics_SpriteAddGroup(&mut self, _mem: &mut Mem) -> u32 {
        self.sprites.AddSpriteGroup()
    }
    /// SpriteRemoveGroup (0x00235cd0)
    pub fn APIgraphics_SpriteRemoveGroup(&mut self, _mem: &mut Mem, g: u32) -> u32 {
        self.sprites.RemoveSpriteGroup(g);
        0
    }
    /// SpriteSetGroupFlags (0x00235d00)
    pub fn APIgraphics_SpriteSetGroupFlags(&mut self, _mem: &mut Mem, g: u32, f: u32) -> u32 {
        if let Some(gr) = self.sprites.GetSpriteGroup(g) {
            gr.flags = f;
        }
        0
    }
    /// SpriteGetGroupFlags (0x00235d40)
    pub fn APIgraphics_SpriteGetGroupFlags(&mut self, _mem: &mut Mem, g: u32) -> u32 {
        self.sprites.GetSpriteGroup(g).map(|gr| gr.flags).unwrap_or(0)
    }
    /// SpriteSetGroupTexture (0x00235d80)
    pub fn APIgraphics_SpriteSetGroupTexture(&mut self, mem: &mut Mem, g: u32, name: Ptr) -> u32 {
        let n = mem.cstr_at(name);
        let Self { sprites, textures, .. } = self;
        if let Some(gr) = sprites.GetSpriteGroup(g) {
            gr.SetTexture(&n, Some(textures.as_slice()));
        }
        0
    }
    /// SpriteSetGroupBlendMode (0x00235dd0)
    pub fn APIgraphics_SpriteSetGroupBlendMode(&mut self, _mem: &mut Mem, g: u32, m: u32) -> u32 {
        if let Some(gr) = self.sprites.GetSpriteGroup(g) {
            gr.SetBlendMode(m);
        }
        0
    }
    /// SpriteSetGroupColor (0x00235e20)
    pub fn APIgraphics_SpriteSetGroupColor(&mut self, _mem: &mut Mem, g: u32, r: f32, gg: f32, b: f32, a: f32) -> u32 {
        if let Some(gr) = self.sprites.GetSpriteGroup(g) {
            gr.SetColor([r, gg, b, a]);
        }
        0
    }
    /// SpriteGroupAddSprite (0x00235e90): returns the sprite id (0 if no group)
    pub fn APIgraphics_SpriteGroupAddSprite(&mut self, _mem: &mut Mem, g: u32, obj: u32, bone: u32, size: f32, w: &dyn GfxWorld) -> u32 {
        let EFSpriteMgr_BC { groups, next_sprite, .. } = &mut self.sprites;
        match groups.iter_mut().find(|gr| gr.id == g) {
            Some(gr) => gr.AddSprite(next_sprite, obj, bone, size, w),
            None => 0,
        }
    }
    /// SpriteGroupRemoveSprite (0x00235f00)
    pub fn APIgraphics_SpriteGroupRemoveSprite(&mut self, _mem: &mut Mem, g: u32, s: u32) -> u32 {
        if let Some(gr) = self.sprites.GetSpriteGroup(g) {
            gr.RemoveSprite(s);
        }
        0
    }

    // ---- palette / tex fader
    /// StartPaletteTransition (0x00235fd0) -> deferred (EFPaletteMgr unported)
    pub fn APIgraphics_StartPaletteTransition(&mut self, _mem: &mut Mem, data: Ptr) -> u32 {
        self.deferred.push(GfxDeferred::StartPaletteTransition { data });
        0
    }
    /// shared object lookup of TexFaderPlay/TexFaderPause: uid >> 24 == 0xf -> search the
    /// world object (0x2a000000)'s 3d objects for +8 == uid; else GetGameObject(uid)->Get3dObject(0).
    fn FaderTarget(uid: u32, w: &dyn GfxWorld) -> Option<(u32, u32)> {
        if uid >> 24 == 0xf {
            let wo = w.ResolveGameObject(0x2a00_0000)?;
            let list = w.Get3dObjectUids(wo)?;
            let i = list.iter().position(|&u| u == uid)?;
            Some((wo, i as u32))
        } else {
            let o = w.ResolveGameObject(uid)?;
            Some((o, 0))
        }
    }
    /// TexFaderPlay (0x00236000)
    pub fn APIgraphics_TexFaderPlay(&mut self, mem: &mut Mem, uid: u32, p: Ptr, w: &dyn GfxWorld) -> u32 {
        if let Some((game_obj, obj3d_index)) = Self::FaderTarget(uid, w) {
            let params = read_sTexFaderParams(mem, p);
            self.deferred.push(GfxDeferred::TexFaderPlay { game_obj, obj3d_index, params });
        }
        0
    }
    /// TexFaderPause (0x00236120)
    pub fn APIgraphics_TexFaderPause(&mut self, _mem: &mut Mem, uid: u32, on: u32, w: &dyn GfxWorld) -> u32 {
        if let Some((game_obj, obj3d_index)) = Self::FaderTarget(uid, w) {
            self.deferred.push(GfxDeferred::TexFaderPause { game_obj, obj3d_index, on });
        }
        0
    }

    // ---- motion trails (0x00236240..)
    /// MTCreateSystem (0x00236240): v0 = the new system id
    pub fn APIgraphics_MTCreateSystem(&mut self, _mem: &mut Mem, n_cp: u32, samples: u32, flags: u32) -> u32 {
        self.trails.CreateSystem(n_cp, samples, flags)
    }
    pub fn APIgraphics_MTDestroySystem(&mut self, _mem: &mut Mem, id: u32) -> u32 {
        self.trails.DestroySystem(id);
        0
    }
    pub fn APIgraphics_MTSetFrequency(&mut self, _mem: &mut Mem, id: u32, f: f32) -> u32 {
        self.trails.SetFrequency(id, f);
        0
    }
    pub fn APIgraphics_MTSetLifeSpan(&mut self, _mem: &mut Mem, id: u32, cp: u32, t: f32) -> u32 {
        self.trails.SetLifeSpan(id, cp, t);
        0
    }
    pub fn APIgraphics_MTSetJoint(&mut self, _mem: &mut Mem, id: u32, cp: u32, obj: u32, joint: u32, w: &dyn GfxWorld) -> u32 {
        self.trails.SetJoint(id, cp, obj, joint, w);
        0
    }
    pub fn APIgraphics_MTSetPosition(&mut self, mem: &mut Mem, id: u32, cp: u32, p: Ptr) -> u32 {
        let v = mem.rv3(p);
        self.trails.SetPosition(id, cp, v);
        0
    }
    pub fn APIgraphics_MTSetRGB(&mut self, _mem: &mut Mem, id: u32, cp: u32, r: f32, g: f32, b: f32) -> u32 {
        self.trails.SetRGB(id, cp, r, g, b);
        0
    }
    pub fn APIgraphics_MTStart(&mut self, _mem: &mut Mem, id: u32) -> u32 {
        self.trails.Start(id);
        0
    }
    pub fn APIgraphics_MTStopTracking(&mut self, _mem: &mut Mem, id: u32) -> u32 {
        self.trails.StopTracking(id);
        0
    }
    pub fn APIgraphics_MTStopDraw(&mut self, _mem: &mut Mem, id: u32) -> u32 {
        self.trails.StopDraw(id);
        0
    }

    // ---- lens flare (0x00235f50..)
    pub fn APIgraphics_LensFlareActivate(&mut self, _mem: &mut Mem, on: u8) -> u32 {
        self.lens_flare.active = on != 0;
        0
    }
    pub fn APIgraphics_LensFlareSetSunPosition(&mut self, mem: &mut Mem, p: Ptr) -> u32 {
        self.lens_flare.sun_pos = mem.rv3(p);
        0
    }
    pub fn APIgraphics_LensFlareAttachSunPositionToObject(&mut self, _mem: &mut Mem, obj: u32) -> u32 {
        self.lens_flare.sun_object = obj;
        0
    }
    /// EFLensFlareFX::SetParams body unported: the arguments are stored
    pub fn APIgraphics_LensFlareSetParams(&mut self, _mem: &mut Mem, a: u32, b: u32) -> u32 {
        self.lens_flare.params = Some((a & 0xff, b));
        0
    }

    // ---- misc
    /// SetGameObjBlendColor (0x00236430)
    pub fn APIgraphics_SetGameObjBlendColor(&mut self, _mem: &mut Mem, obj: u32, idx: u32, r: f32, g: f32, b: f32, a: f32, w: &dyn GfxWorld) -> u32 {
        if let Some(o) = w.ResolveGameObject(obj) {
            self.deferred.push(GfxDeferred::SetBlendingColor { game_obj: o, obj3d_index: idx, rgba: [r, g, b, a] });
        }
        0
    }
    /// DrawLines (0x00235c80) -> deferred (EFLinesMgr unported)
    pub fn APIgraphics_DrawLines(&mut self, _mem: &mut Mem, p: Ptr) -> u32 {
        self.deferred.push(GfxDeferred::DrawLines { params: p });
        0
    }
    /// DoCrappyXboxSpewHackThingy (0x002364c0): EF3dObj +0x38 = flag
    pub fn APIgraphics_DoCrappyXboxSpewHackThingy(&mut self, _mem: &mut Mem, obj: u32, idx: u32, v: u8, w: &dyn GfxWorld) -> u32 {
        if let Some(o) = w.ResolveGameObject(obj) {
            self.deferred.push(GfxDeferred::Set3dObjField38 { game_obj: o, obj3d_index: idx, value: v });
        }
        0
    }
    /// DoStupidXboxShadowHack (0x00236520)
    pub fn APIgraphics_DoStupidXboxShadowHack(&mut self, _mem: &mut Mem, v: u8) -> u32 {
        self.xbox_shadow_hack = v != 0;
        0
    }
}

/// Which EFSpriteMgr_BC render function a RenderPass maps to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpritePass {
    Normal,
    Transparent,
    Occluded,
}

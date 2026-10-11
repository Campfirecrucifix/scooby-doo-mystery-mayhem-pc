//! EFSprite / EFSpriteGroup_BC / EFSpriteGroup_SP / EFSpriteMgr_BC (Classes/EF/EFSprite*.cpp),
//! the global `goSpriteMgr`. Manager logic is 1:1 (linked lists -> Vec kept in list order:
//! the group list and each group's sprite list are head-inserted, so index 0 = newest).
//!
//! Replacements (documented per item):
//! - EFPosData frame (+0x14) -> [`SpriteFrame`] (own frame / object frame / pos-data frame); LTMs
//!   are queried from the coordinator's [`GfxWorld`] at render time (RwFrameGetLTM).
//! - RwTexture* (+0x18) -> texture name looked up in the current texture dictionary names.
//! - Rendering: RwIm2DRenderPrimitive / the hand-built GS packets become backend-agnostic draw
//!   data ([`SpriteDraw`]). RenderBatchNormal is 1:1 down to the screen-space vertices (same
//!   projection, clip, sort, quad layout); the occluded path (GS destination-alpha occlusion
//!   query) is reduced to an [`OccludedSprite`] description the adapter must implement.
#![allow(non_snake_case, non_camel_case_types)]

use super::api::GfxWorld;
use crate::engine::camera::DTMatrix;
use crate::engine::particle::mat_mul;

/// EFSpriteGroup_BC::muiNextSpriteID initial value (.data 0x0034390c = 0x4c800000). The 0x4c
/// class byte makes EFWorld::GetObjectPosData route the uid to EFSpriteMgr_BC::GetSprite.
pub const SPRITE_ID_BASE: u32 = 0x4c80_0000;
/// AddSprite "no attachment" bone value: the sprite gets its own root frame.
pub const SPRITE_OWN_FRAME: u32 = 0x6fff_ffff;

/// EFSprite +0x14 frame (EFPosData) replacement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpriteFrame {
    /// EFPosData::CreateFrame(NULL): own frame; `parent` is set when the logic attaches the
    /// sprite with APIgameobj::AddChild (RwFrameAddChild onto object `obj` frame `bone`).
    Own { local: DTMatrix, parent: Option<(u32, u32)> },
    /// EFGameObject::GetObjectFrame(bone) of a 0x2a object (frame pointer shared, flags &= ~1)
    ObjectFrame { obj: u32, bone: u32 },
    /// EFWorld::GetObjectPosData(uid)->frame
    PosData { uid: u32 },
    /// frame pointer NULL (object not found). quirk: the C++ calls RwFrameGetLTM(NULL) when
    /// drawing; the port skips the sprite instead.
    Null,
}

/// EFSprite (0x40 bytes, derives EFPosData)
#[derive(Debug, Clone, PartialEq)]
pub struct EFSprite {
    /// +0x14 frame
    pub frame: SpriteFrame,
    /// +0x2c size (half extent, in view-matrix units)
    pub size: f32,
    /// +0x31 screen-space override flag and +0x34/+0x38 screen x/y (z forced to 1.0). Never set
    /// by any APIgraphics_* path (0 at creation).
    pub screen: Option<[f32; 2]>,
    /// +0x3c id
    pub id: u32,
}

/// Identity RwMatrix (EFPosData::CreateFrame(NULL) creates an identity root frame).
pub const IDENTITY: DTMatrix = DTMatrix { right: [1.0, 0.0, 0.0], up: [0.0, 1.0, 0.0], at: [0.0, 0.0, 1.0], pos: [0.0; 3] };

impl EFSprite {
    /// RwFrameGetLTM(+0x14)
    pub fn GetLTM(&self, w: &dyn GfxWorld) -> Option<DTMatrix> {
        match self.frame {
            SpriteFrame::Own { local, parent: None } => Some(local),
            SpriteFrame::Own { local, parent: Some((o, b)) } => w.GetObjectFrame(o, b).map(|p| mat_mul(&local, &p)),
            SpriteFrame::ObjectFrame { obj, bone } => w.GetObjectFrame(obj, bone),
            SpriteFrame::PosData { uid } => w.GetObjectPosDataFrame(uid),
            SpriteFrame::Null => None,
        }
    }
}

/// EFSpriteGroup_BC + EFSpriteGroup_SP (0x20 bytes)
#[derive(Debug, Clone, PartialEq)]
pub struct EFSpriteGroup_SP {
    /// +0x4 id
    pub id: u32,
    /// +0x8 flags: 1 z-sort (back to front, per 128-batch), 2 transparent pass, 4 z-write off,
    /// 8 z-test off, 0x10 stencil (dest alpha) read, 0x20 occluded pass
    pub flags: u32,
    /// +0xc blend mode 0..3
    pub blend: u32,
    /// +0x10 sprite list (head first) / +0x14 count
    pub sprites: Vec<EFSprite>,
    /// +0x18 texture (None = NULL raster)
    pub texture: Option<String>,
    /// +0x1c..+0x1f RGBA bytes (default 0x80,0x80,0x80,0xff)
    pub rgba: [u8; 4],
}

/// float * 255 -> byte as the C++ does it ((int) cast, low byte kept; >= 2^31 path folded).
fn col_byte(f: f32) -> u8 {
    let v = f * 255.0;
    if v < 2147483648.0 { (v as i32) as u8 } else { ((v - 2147483648.0) as i32) as u8 }
}

impl EFSpriteGroup_SP {
    /// EFSpriteGroup_SP::EFSpriteGroup_SP (0x00277200) / EFSpriteGroup_BC ctor (0x002353b0)
    pub fn new(id: u32) -> Self {
        EFSpriteGroup_SP { id, flags: 0, blend: 0, sprites: Vec::new(), texture: None, rgba: [0x80, 0x80, 0x80, 0xff] }
    }
    /// EFSpriteGroup_BC::AddSprite (0x00235440). `next_id` = EFSpriteGroup_BC::muiNextSpriteID.
    pub fn AddSprite(&mut self, next_id: &mut u32, obj: u32, bone: u32, size: f32, w: &dyn GfxWorld) -> u32 {
        let id = *next_id;
        *next_id = next_id.wrapping_add(1);
        let frame = if bone == SPRITE_OWN_FRAME {
            SpriteFrame::Own { local: IDENTITY, parent: None }
        } else if obj >> 24 == 0x2a {
            // EFWorld::GetGameObject(obj) -> GetObjectFrame(bone); the frame pointer is shared
            match w.ResolveGameObject(obj) {
                Some(o) if w.GetObjectFrame(o, bone).is_some() => SpriteFrame::ObjectFrame { obj: o, bone },
                _ => SpriteFrame::Null,
            }
        } else if w.GetObjectPosDataFrame(obj).is_some() {
            SpriteFrame::PosData { uid: obj }
        } else {
            SpriteFrame::Null
        };
        self.sprites.insert(0, EFSprite { frame, size, screen: None, id });
        id
    }
    /// EFSpriteGroup_BC::GetSprite (0x00235590)
    pub fn GetSprite(&mut self, id: u32) -> Option<&mut EFSprite> {
        self.sprites.iter_mut().find(|s| s.id == id)
    }
    /// EFSpriteGroup_BC::RemoveSprite (0x002355d0)
    pub fn RemoveSprite(&mut self, id: u32) {
        if let Some(i) = self.sprites.iter().position(|s| s.id == id) {
            self.sprites.remove(i);
        }
    }
    /// EFSpriteGroup_BC::RemoveAllSprites (0x00235660)
    pub fn RemoveAllSprites(&mut self) {
        self.sprites.clear();
    }
    /// EFSpriteGroup_BC::SetBlendMode (0x002356c0)
    pub fn SetBlendMode(&mut self, m: u32) {
        self.blend = m;
    }
    /// EFSpriteGroup_SP::SetColor (0x002772b0)
    pub fn SetColor(&mut self, c: [f32; 4]) {
        self.rgba = [col_byte(c[0]), col_byte(c[1]), col_byte(c[2]), col_byte(c[3])];
    }
    /// EFSpriteGroup_SP::SetTexture (0x00277420): RwTexDictionaryGetCurrent +
    /// RwTexDictionaryFindNamedTexture; unchanged when there is no dictionary or no match.
    pub fn SetTexture(&mut self, name: &str, dict: Option<&[String]>) {
        if let Some(t) = dict.and_then(|d| d.iter().find(|t| t.eq_ignore_ascii_case(name))) {
            self.texture = Some(t.clone());
        }
    }

    /// EFSpriteGroup_SP::Render (0x00277470)
    pub fn Render(&self, w: &dyn GfxWorld, cam: &SpriteCamera, out: &mut Vec<SpriteDraw>) {
        if self.flags & 0x20 == 0 {
            self.RenderNormal(w, cam, out);
        } else {
            self.RenderNewOcc(w, cam, out);
        }
    }

    /// EFSpriteGroup_SP::RenderNormal (0x002774b0): render states + batches of 0x80 sprites.
    pub fn RenderNormal(&self, w: &dyn GfxWorld, cam: &SpriteCamera, out: &mut Vec<SpriteDraw>) {
        // RpSkyRenderStateSet(2, ALPHA): modes > 3 leave the previous blend (None)
        let blend = match self.blend {
            0 => Some(0xa8),
            1 => Some(0x68),
            2 => Some(0xa1),
            3 => Some(0x44),
            _ => None,
        };
        let mut rest = self.sprites.as_slice();
        while !rest.is_empty() {
            let n = rest.len().min(0x80);
            let verts = self.RenderBatchNormal(&rest[..n], w, cam);
            rest = &rest[n..];
            if !verts.is_empty() {
                out.push(SpriteDraw::Batch(SpriteBatch {
                    group: self.id,
                    texture: self.texture.clone(),
                    blend,
                    zwrite_off: self.flags & 4 != 0,
                    ztest_off: self.flags & 8 != 0,
                    stencil_read: self.flags & 0x10 != 0,
                    verts,
                }));
            }
        }
        // after: modes 1..3 restore VERTEXALPHA 0, SRCBLEND 2, DESTBLEND 1 (adapter default)
    }

    /// EFSpriteGroup_SP::RenderBatchNormal (0x002776b0), 1:1: camera-space transform, optional
    /// bubble sort (descending z), near/far clip, projection, guard-band clip, 6 Im2D vertices
    /// (2 triangles) per sprite.
    pub fn RenderBatchNormal(&self, batch: &[EFSprite], w: &dyn GfxWorld, cam: &SpriteCamera) -> Vec<SpriteVertex2D> {
        let mut pts: Vec<([f32; 3], f32)> = Vec::with_capacity(batch.len());
        for s in batch {
            // quirk: a NULL frame crashes RwFrameGetLTM in the C++; skipped here
            let Some(ltm) = s.GetLTM(w) else { continue };
            let mut p = cam.Transform(ltm.pos);
            if let Some([x, y]) = s.screen {
                p = [x, y, 1.0];
            }
            pts.push((p, s.size));
        }
        let n = pts.len();
        if self.flags & 1 != 0 && n != 1 {
            for _ in 0..n.saturating_sub(1) {
                for i in 0..n - 1 {
                    if pts[i].0[2] < pts[i + 1].0[2] {
                        pts.swap(i, i + 1);
                    }
                }
            }
        }
        let (wf, hf) = (cam.width as f32, cam.height as f32);
        let gx = 2048.0 - (cam.width / 2) as f32;
        let gy = 2048.0 - (cam.height / 2) as f32;
        let c = self.rgba.map(|b| b as f32);
        let mut v = Vec::new();
        for (p, size) in pts {
            let z = p[2];
            if !(cam.near <= z && z < cam.far) {
                continue;
            }
            let rz = 1.0 / z;
            let sz = cam.z_shift + rz * cam.z_scale;
            let right = rz * wf * (p[0] + size);
            let left = rz * wf * (p[0] - size);
            let top = rz * hf * (p[1] - size);
            let bottom = rz * hf * (p[1] + size);
            if -gx <= left && right <= gx && -gy <= top && bottom <= gy {
                let mk = |x: f32, y: f32, u: f32, vv: f32| SpriteVertex2D { x, y, z: sz, recip_z: rz, u, v: vv, rgba: c };
                v.push(mk(right, top, 1.0, 1.0));
                v.push(mk(left, bottom, 0.0, 0.0));
                v.push(mk(right, bottom, 1.0, 0.0));
                v.push(mk(right, top, 1.0, 1.0));
                v.push(mk(left, top, 0.0, 1.0));
                v.push(mk(left, bottom, 0.0, 0.0));
            }
        }
        v
    }

    /// EFSpriteGroup_SP::RenderNewOcc (0x00278030)
    pub fn RenderNewOcc(&self, w: &dyn GfxWorld, cam: &SpriteCamera, out: &mut Vec<SpriteDraw>) {
        for s in &self.sprites {
            if let Some(o) = self.RenderSpriteNewOcc(s, w, cam) {
                out.push(SpriteDraw::Occluded(o));
            }
        }
    }

    /// EFSpriteGroup_SP::RenderSpriteNewOcc (0x002780a0): the screen geometry is 1:1; the GS
    /// packets (z-test a small square into frame-buffer alpha, then draw the sprite with the
    /// destination-alpha test) are replaced by [`OccludedSprite`].
    pub fn RenderSpriteNewOcc(&self, s: &EFSprite, w: &dyn GfxWorld, cam: &SpriteCamera) -> Option<OccludedSprite> {
        let ltm = s.GetLTM(w)?;
        let p = cam.Transform(ltm.pos);
        let size = s.size;
        let rz = 1.0 / p[2];
        let (wf, hf) = (cam.width as f32, cam.height as f32);
        let left = rz * wf * (p[0] - size);
        let top = rz * hf * (p[1] - size);
        let right = rz * wf * (p[0] + size);
        let bottom = rz * hf * (p[1] + size);
        let cx = rz * p[0] * wf;
        let cy = rz * p[1] * hf;
        // quirk: hard-coded 512x448 screen
        if !(cx <= 512.0 && 0.0 <= cx && cy <= 448.0 && 0.0 <= cy) {
            return None;
        }
        let fx = |f: f32| f as u64; // __fixunssfdi (negative -> 0 here)
        let (mut x0, mut x1) = if 3.0 < right - left { (fx(cx - 1.5), fx(cx + 1.5)) } else { (fx(left), fx(right)) };
        let (mut y0, mut y1) = if 3.0 < bottom - top { (fx(cy - 1.5), fx(cy + 1.5)) } else { (fx(top), fx(bottom)) };
        let (dw, dh) = (x1.wrapping_sub(x0), y1.wrapping_sub(y0));
        // quirk: the square is made with +- the full smaller extent
        if dw < dh {
            y0 = fx(cy - dw as f32);
            y1 = fx(cy + dw as f32);
        } else if dh < dw {
            x0 = fx(cx - dh as f32);
            x1 = fx(cx + dh as f32);
        }
        let blend = match self.blend {
            0 | 3 => Some(0x98),
            1 => Some(0x58),
            2 => Some(0x91),
            _ => None,
        };
        // colour: (rgba & 0xfefefefe) >> 1 (GS 0x80 = 1.0)
        let rgba = self.rgba.map(|b| (b & 0xfe) >> 1);
        Some(OccludedSprite {
            group: self.id,
            sprite: s.id,
            rect: [left, top, right, bottom],
            center: [cx, cy],
            sample: [x0, y0, x1, y1],
            z: cam.z_shift + rz * cam.z_scale,
            recip_z: rz,
            uv_center: [cx / 512.0, cy / 512.0],
            texture: self.texture.clone(),
            blend,
            rgba,
            zwrite_off: self.flags & 4 != 0,
            ztest_off: self.flags & 8 != 0,
        })
    }
}

/// The camera globals RenderBatchNormal reads (ourGlobals): +0x20 view matrix, +0x60 raster
/// (+0xc width, +0x10 height), +0x80/+0x84 near/far clip, +0x8c/+0x90 z-buffer scale/shift.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpriteCamera {
    /// camera LTM (EFCamera frame)
    pub ltm: DTMatrix,
    /// RwCamera view window (x, y)
    pub view_window: [f32; 2],
    pub near: f32,
    pub far: f32,
    pub width: i32,
    pub height: i32,
    pub z_scale: f32,
    pub z_shift: f32,
}

impl SpriteCamera {
    /// RwV3dTransformPoints(p, ourGlobals+0x20) replacement: the RW perspective view matrix
    /// (inverse LTM, then x/y scaled by 0.5/view window and offset by 0.5*z, so x/z and y/z are
    /// in [0,1] across the screen; +right is screen-left, +up is screen-up -> y flipped).
    pub fn Transform(&self, p: [f32; 3]) -> [f32; 3] {
        let m = &self.ltm;
        let d = [p[0] - m.pos[0], p[1] - m.pos[1], p[2] - m.pos[2]];
        let dot = |a: [f32; 3]| a[0] * d[0] + a[1] * d[1] + a[2] * d[2];
        let (cx, cy, cz) = (dot(m.right), dot(m.up), dot(m.at));
        [0.5 * cz - 0.5 * cx / self.view_window[0], 0.5 * cz - 0.5 * cy / self.view_window[1], cz]
    }
}

/// One RwIm2DVertex of a sprite quad (screen pixels; colour bytes as floats, 0x80 = 1.0 on GS).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpriteVertex2D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub recip_z: f32,
    pub u: f32,
    pub v: f32,
    pub rgba: [f32; 4],
}

/// One RwIm2DRenderPrimitive(TRILIST) call.
#[derive(Debug, Clone, PartialEq)]
pub struct SpriteBatch {
    pub group: u32,
    pub texture: Option<String>,
    /// GS ALPHA register code: 0xa8 (mode 0, Cs*FIX), 0x68 (1, additive Cs*FIX+Cd), 0xa1 (2,
    /// subtractive (Cd-Cs)*FIX), 0x44 (3, alpha blend); None = previous state kept
    pub blend: Option<u32>,
    pub zwrite_off: bool,
    pub ztest_off: bool,
    pub stencil_read: bool,
    pub verts: Vec<SpriteVertex2D>,
}

/// One occlusion-tested sprite (flags 0x20). Adapter contract: draw `rect` (uv 0..1) with
/// `rgba`/`texture`/`blend` only where the depth test of the `sample` square at depth `z`
/// against the scene passed (GS: square z-tested into FB alpha, sprite drawn with DATE).
#[derive(Debug, Clone, PartialEq)]
pub struct OccludedSprite {
    pub group: u32,
    pub sprite: u32,
    /// left, top, right, bottom (pixels)
    pub rect: [f32; 4],
    pub center: [f32; 2],
    /// x0, y0, x1, y1 (pixels, integer)
    pub sample: [u64; 4],
    pub z: f32,
    pub recip_z: f32,
    pub uv_center: [f32; 2],
    pub texture: Option<String>,
    /// GS ALPHA: 0x98 (modes 0/3), 0x58 (1), 0x91 (2)
    pub blend: Option<u32>,
    pub rgba: [u8; 4],
    pub zwrite_off: bool,
    pub ztest_off: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SpriteDraw {
    Batch(SpriteBatch),
    Occluded(OccludedSprite),
}

/// EFSpriteMgr_BC (goSpriteMgr)
#[derive(Debug, Clone, PartialEq)]
pub struct EFSpriteMgr_BC {
    /// +0x4 next group id (starts at 1)
    pub next_group: u32,
    /// +0x8 group list, head first
    pub groups: Vec<EFSpriteGroup_SP>,
    /// EFSpriteGroup_BC::muiNextSpriteID (global, never reset)
    pub next_sprite: u32,
}

impl Default for EFSpriteMgr_BC {
    fn default() -> Self {
        Self::new()
    }
}

impl EFSpriteMgr_BC {
    /// EFSpriteMgr_BC::EFSpriteMgr_BC (0x002356d0)
    pub fn new() -> Self {
        EFSpriteMgr_BC { next_group: 1, groups: Vec::new(), next_sprite: SPRITE_ID_BASE }
    }
    /// EFSpriteMgr_BC::Terminate (0x00235750)
    pub fn Terminate(&mut self) {
        self.RemoveAllSpriteGroups();
    }
    /// EFSpriteMgr_BC::GetSprite (0x00235770) (EFWorld::GetObjectPosData for 0x4c uids)
    pub fn GetSprite(&mut self, id: u32) -> Option<&mut EFSprite> {
        self.groups.iter_mut().find_map(|g| g.GetSprite(id))
    }
    /// EFSpriteMgr_BC::AddSpriteGroup (0x002357d0): returns the new group's id
    pub fn AddSpriteGroup(&mut self) -> u32 {
        let id = self.next_group;
        self.next_group = self.next_group.wrapping_add(1);
        self.groups.insert(0, EFSpriteGroup_SP::new(id));
        id
    }
    /// EFSpriteMgr_BC::GetSpriteGroup (0x00235850)
    pub fn GetSpriteGroup(&mut self, id: u32) -> Option<&mut EFSpriteGroup_SP> {
        self.groups.iter_mut().find(|g| g.id == id)
    }
    /// EFSpriteMgr_BC::RemoveSpriteGroup (0x00235890)
    pub fn RemoveSpriteGroup(&mut self, id: u32) {
        if let Some(i) = self.groups.iter().position(|g| g.id == id) {
            self.groups.remove(i);
        }
    }
    /// EFSpriteMgr_BC::RemoveAllSpriteGroups (0x00235980)
    pub fn RemoveAllSpriteGroups(&mut self) {
        self.groups.clear();
    }
    /// EFSpriteMgr_BC::RenderNormal (0x002737f0): groups with (flags & 0x22) == 0. Fog is off
    /// around all three passes (RwRenderStateSet(0xe, 0) / restore).
    pub fn RenderNormal(&self, w: &dyn GfxWorld, cam: &SpriteCamera) -> Vec<SpriteDraw> {
        let mut out = Vec::new();
        for g in self.groups.iter().filter(|g| g.flags & 0x22 == 0) {
            g.Render(w, cam, &mut out);
        }
        out
    }
    /// EFSpriteMgr_BC::RenderTransparent (0x00273870): flags & 2 and not & 0x20
    pub fn RenderTransparent(&self, w: &dyn GfxWorld, cam: &SpriteCamera) -> Vec<SpriteDraw> {
        let mut out = Vec::new();
        for g in self.groups.iter().filter(|g| g.flags & 2 != 0 && g.flags & 0x20 == 0) {
            g.Render(w, cam, &mut out);
        }
        out
    }
    /// EFSpriteMgr_BC::RenderOccluded (0x00273900): flags & 0x20
    pub fn RenderOccluded(&self, w: &dyn GfxWorld, cam: &SpriteCamera) -> Vec<SpriteDraw> {
        let mut out = Vec::new();
        for g in self.groups.iter().filter(|g| g.flags & 0x20 != 0) {
            g.Render(w, cam, &mut out);
        }
        out
    }
    /// APIgameobj::AddChild(obj, joint, sprite uid, ..) support (RwFrameAddChild of the sprite's
    /// own frame onto the object frame; local matrix kept). Returns false if no such sprite.
    pub fn AttachSprite(&mut self, id: u32, obj: u32, bone: u32) -> bool {
        match self.GetSprite(id) {
            Some(s) => {
                let local = match s.frame {
                    SpriteFrame::Own { local, .. } => local,
                    _ => IDENTITY,
                };
                s.frame = SpriteFrame::Own { local, parent: Some((obj, bone)) };
                true
            }
            None => false,
        }
    }
}

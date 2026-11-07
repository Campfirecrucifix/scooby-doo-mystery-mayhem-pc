//! WP-I: EFCamera / EFCamera_SP / EFCamera_BC, EFFrustum / EFPlane, the camera part of
//! EFGraphicsMgr (CreateCamera / GetCamera / DestroyCamera), EFCameraAnimNode and the APIcamera_*
//! host functions (EXE.1 Classes/EF/EFCamera*.cpp, EFFrustum.cpp, EFPlane.cpp, EFApi_Camera.cpp).
//!
//! Replaced (not 1:1): the RenderWare `RwCamera` is a plain [`RwCamera`] struct holding exactly the
//! fields EFCamera reads/writes (+0x14 projection, +0x60/+0x64 rasters, +0x68 view window,
//! +0x80/+0x84/+0x88 near/far/fog planes). Rasters are reduced to their sub-raster rectangle; the
//! actual render target / clear / Begin/End are done by the Bevy renderer from this data.
//! The camera frame (RwFrame shared with the game object) is [`EFCamera::ltm`], which the
//! coordinator syncs from the owning object's matrix (`frame_owner`) before using the frustum.
#![allow(non_snake_case)]

use crate::engine::anim::res::DTAnimHdr;
use crate::engine::{Mem, Ptr};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

/// WP-G1 (not in the original): Hor+ widening of the frustum x extent (f32 bits, default 1.0 =
/// the 1:1 PS2 frustum). The renderer sets it when the output is wider than the view window so
/// objects at the screen sides are not culled.
pub static FRUSTUM_HSCALE: AtomicU32 = AtomicU32::new(0x3f80_0000);
fn hscale() -> f32 {
    f32::from_bits(FRUSTUM_HSCALE.load(Ordering::Relaxed))
}
/// Sets [`FRUSTUM_HSCALE`] (clamped to >= 1).
pub fn set_frustum_hscale(s: f32) {
    FRUSTUM_HSCALE.store(s.max(1.0).to_bits(), Ordering::Relaxed);
}

/// DTMatrix / RwMatrix (right +0x0, up +0x10, at +0x20, pos +0x30; the flags word is dropped).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DTMatrix {
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub at: [f32; 3],
    pub pos: [f32; 3],
}

impl Default for DTMatrix {
    fn default() -> Self {
        DTMatrix { right: [1.0, 0.0, 0.0], up: [0.0, 1.0, 0.0], at: [0.0, 0.0, 1.0], pos: [0.0; 3] }
    }
}

impl DTMatrix {
    /// Reads an RwMatrix laid out in Mem (0x40 bytes).
    pub fn read(mem: &Mem, p: Ptr) -> Self {
        DTMatrix { right: mem.rv3(p), up: mem.rv3(p + 0x10), at: mem.rv3(p + 0x20), pos: mem.rv3(p + 0x30) }
    }
    /// Writes as an RwMatrix (flags word left untouched).
    pub fn write(&self, mem: &mut Mem, p: Ptr) {
        mem.wv3(p, self.right);
        mem.wv3(p + 0x10, self.up);
        mem.wv3(p + 0x20, self.at);
        mem.wv3(p + 0x30, self.pos);
    }
    /// DTMatrix::SetToRotation (0x0010bae0): rotation part from euler angles (x, y, z), pos untouched.
    pub fn SetToRotation(&mut self, e: [f32; 3]) {
        let (s1, c1) = (e[0].sin(), e[0].cos());
        let (s2, c2) = (e[1].sin(), e[1].cos());
        let (s3, c3) = (e[2].sin(), e[2].cos());
        self.right = [c2 * c3, c2 * s3, -s2];
        self.up = [s1 * s2 * c3 - c1 * s3, c1 * c3 + s1 * s2 * s3, s1 * c2];
        self.at = [s1 * s3 + c1 * s2 * c3, c1 * s2 * s3 - s1 * c3, c1 * c2];
    }
    /// RwV3dTransformVectors with one vector (no translation).
    pub fn transform_vector(&self, v: [f32; 3]) -> [f32; 3] {
        let m = self;
        [
            v[0] * m.right[0] + v[1] * m.up[0] + v[2] * m.at[0],
            v[0] * m.right[1] + v[1] * m.up[1] + v[2] * m.at[1],
            v[0] * m.right[2] + v[1] * m.up[2] + v[2] * m.at[2],
        ]
    }
    /// View matrix for the renderer (inverse of an orthonormal LTM): rows = right/up/at.
    /// Not in EXE.1 (RwCameraBeginUpdate does it internally); provided for the Bevy side.
    pub fn view_matrix(&self) -> [[f32; 4]; 4] {
        let d = |a: [f32; 3]| -(a[0] * self.pos[0] + a[1] * self.pos[1] + a[2] * self.pos[2]);
        [
            [self.right[0], self.right[1], self.right[2], d(self.right)],
            [self.up[0], self.up[1], self.up[2], d(self.up)],
            [self.at[0], self.at[1], self.at[2], d(self.at)],
            [0.0, 0.0, 0.0, 1.0],
        ]
    }
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[2] * b[2] + a[0] * b[0] + a[1] * b[1]
}

/// EFPlane (0x10): normal + distance; a point p is on the positive side when dot(n,p) - d > 0.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EFPlane {
    pub n: [f32; 3],
    /// +0xc
    pub d: f32,
}

impl EFPlane {
    pub fn dist(&self, p: [f32; 3]) -> f32 {
        dot(p, self.n) - self.d
    }
    /// EFPlane::Transform (0x00267dc0)
    pub fn Transform(&self, m: &DTMatrix) -> EFPlane {
        let n = m.transform_vector(self.n);
        EFPlane { n, d: m.pos[2] * n[2] + m.pos[0] * n[0] + m.pos[1] * n[1] + self.d }
    }
    /// EFPlane::IntersectRay (0x00267c70): 0 hit (out written), 1 parallel and on plane, 2 parallel off plane.
    pub fn IntersectRay(&self, origin: [f32; 3], dir: [f32; 3], out: &mut [f32; 3]) -> u32 {
        let nd = dot(self.n, dir);
        // quirk: "parallel" is an exponent test, |n.dir| < 2^-7
        if (nd.to_bits() & 0x7f80_0000) < 0x3c00_0000 {
            if 0x3bff_ffff < ((dot(self.n, origin) - self.d).to_bits() & 0x7f80_0000) { 2 } else { 1 }
        } else {
            let t = (self.d - dot(self.n, origin)) / nd;
            *out = [dir[0] * t + origin[0], dir[1] * t + origin[1], dir[2] * t + origin[2]];
            0
        }
    }
}

/// EFFrustum (0x80 bytes): planes near, far, left(+0x20), right(+0x30), top(+0x40), bottom(+0x50);
/// +0x78 parallel projection flag.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EFFrustum {
    pub planes: [EFPlane; 6],
    /// +0x78
    pub parallel: bool,
}

/// RwV3dNormalize: normalises the plane normal only (d is left as is, like the original).
fn normalize(v: &mut [f32; 3]) {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l > 0.0 {
        let r = 1.0 / l;
        v.iter_mut().for_each(|x| *x *= r);
    }
}

impl EFFrustum {
    /// EFFrustum::SetFromCameraParams (0x00267e40). vw = view window, near/far = clip planes.
    pub fn SetFromCameraParams(vwx: f32, vwy: f32, near: f32, far: f32, parallel: bool) -> EFFrustum {
        let p = |x: f32, y: f32, z: f32, d: f32| EFPlane { n: [x, y, z], d };
        let mut f = EFFrustum { parallel, ..Default::default() };
        f.planes[0] = p(0.0, 0.0, -1.0, -near);
        f.planes[1] = p(0.0, 0.0, 1.0, far);
        if !parallel {
            f.planes[2] = p(1.0, 0.0, -vwx, 0.0);
            f.planes[3] = p(-1.0, 0.0, -vwx, 0.0);
            f.planes[4] = p(0.0, 1.0, -vwy, 0.0);
            f.planes[5] = p(0.0, -1.0, -vwy, 0.0);
        } else {
            f.planes[2] = p(1.0, 0.0, 0.0, vwx);
            f.planes[3] = p(-1.0, 0.0, 0.0, vwx);
            f.planes[4] = p(0.0, 1.0, 0.0, vwy);
            f.planes[5] = p(0.0, -1.0, 0.0, vwy);
        }
        f.Normalize();
        f
    }
    /// EFFrustum::Normalize (0x00267fd0)
    pub fn Normalize(&mut self) {
        for p in &mut self.planes {
            normalize(&mut p.n);
        }
    }
    /// EFFrustum::TransformPlanes (0x00267f60). quirk: the +0x78 flag of `out` is not written.
    pub fn TransformPlanes(&self, m: &DTMatrix, out: &mut EFFrustum) {
        for i in 0..6 {
            out.planes[i] = self.planes[i].Transform(m);
        }
    }
    /// EFFrustum::ProjectOntoFrustum (0x00268030): clamps `p` onto the frustum; returns the region code
    /// (0 outside far, 1 outside near (returned as is), 2/3 right/left, 4/5 top/bottom, 6-9 corners, 10 inside).
    pub fn ProjectOntoFrustum(&self, m: &DTMatrix, p: [f32; 3], out: &mut [f32; 3]) -> u32 {
        let pl = &self.planes;
        let neg = |v: [f32; 3]| [-v[0], -v[1], -v[2]];
        if pl[0].dist(p) > 0.0 {
            *out = p;
            return 1;
        }
        if pl[1].dist(p) > 0.0 {
            *out = p;
            return 0;
        }
        let mut tmp = [0.0f32; 3];
        for (side, dir, code) in [(2usize, neg(m.right), 3u32), (3, m.right, 2)] {
            if pl[side].dist(p) > 0.0 {
                if pl[4].dist(p) > 0.0 {
                    pl[side].IntersectRay(p, dir, &mut tmp);
                    pl[4].IntersectRay(tmp, neg(m.up), out);
                    return if side == 2 { 6 } else { 7 };
                }
                if pl[5].dist(p) > 0.0 {
                    pl[side].IntersectRay(p, dir, &mut tmp);
                    pl[5].IntersectRay(tmp, m.up, out);
                    return if side == 2 { 8 } else { 9 };
                }
                pl[side].IntersectRay(p, dir, out);
                return code;
            }
        }
        if pl[4].dist(p) > 0.0 {
            pl[4].IntersectRay(p, neg(m.up), out);
            return 4;
        }
        if pl[5].dist(p) > 0.0 {
            pl[5].IntersectRay(p, m.up, out);
            return 5;
        }
        *out = p;
        10
    }
}

/// Sub-raster rectangle (x, y, w, h) standing in for an RwRaster.
pub type RasterRect = [i32; 4];

/// Replacement for RenderWare's RwCamera (only what EFCamera observes). Defaults = RwCameraCreate.
#[derive(Debug, Clone, PartialEq)]
pub struct RwCamera {
    /// +0x14: 1 perspective, 2 parallel
    pub projection: u32,
    /// +0x60 frame buffer raster, +0x64 z raster
    pub frame_raster: Option<RasterRect>,
    pub z_raster: Option<RasterRect>,
    /// +0x68 / +0x6c
    pub view_window: [f32; 2],
    /// +0x80 / +0x84 / +0x88
    pub near: f32,
    pub far: f32,
    pub fog_plane: f32,
    /// RwCameraGetWorld() != 0
    pub in_world: bool,
}

impl Default for RwCamera {
    fn default() -> Self {
        RwCamera {
            projection: 1,
            frame_raster: None,
            z_raster: None,
            view_window: [1.0, 1.0],
            near: 0.05,
            far: 10.0,
            fog_plane: 5.0,
            in_world: false,
        }
    }
}

/// C float -> unsigned int conversion as compiled (values >= 2^31 are rebased).
fn f2u(f: f32) -> u32 {
    if f < 2.147_483_6e9 { f as i32 as u32 } else { (f - 2.147_483_6e9) as i32 as u32 }
}

/// EFCamera (0x250 bytes, incl. EFCamera_SP / EFCamera_BC).
#[derive(Debug, Clone, PartialEq)]
pub struct EFCamera {
    /// +0x8 uid (slot | 0x10000000)
    pub uid: u32,
    /// +0xc bit0 = active (APIcamera Activate)
    pub flags: u32,
    /// +0x14 frame: the LTM of the frame, and the game object that owns it (synced by the coordinator)
    pub ltm: DTMatrix,
    pub frame_owner: u32,
    /// +0x2c packed fog colour (APIcamera SetFogColor)
    pub fog_color: u32,
    /// +0x30
    pub rw: RwCamera,
    /// +0x34 / +0x38 explicit rasters (EFCamera_SP::SetRasters)
    pub raster: Option<RasterRect>,
    pub zraster: Option<RasterRect>,
    /// +0x3c clear colour RGBA8
    pub clear_color: [u8; 4],
    /// +0x40 clear mode (RwCameraClear mode, |4 on Clear)
    pub clear_mode: u32,
    /// +0x44 / +0x48 guard-band scale
    pub guard_scale: [f32; 2],
    /// +0x4c camera-space frustum, +0xcc world frustum
    pub frustum: EFFrustum,
    pub world_frustum: EFFrustum,
    /// +0x148 / +0x1c4 guard band frusta
    pub guard_frustum: EFFrustum,
    pub world_guard_frustum: EFFrustum,
    /// +0xc8 (200) frustum dirty
    pub frustum_dirty: u32,
    /// +0x240
    pub render_flag: u32,
    /// +0x244
    pub fog_enabled: u32,
    /// +0x248
    pub fog_type: u32,
    /// +0x24c
    pub fog_density: f32,
}

impl EFCamera {
    /// EFCamera::EFCamera(DTFrame*) (0x0026b4e0). `screen` = EFGraphicsMgr +0x38/+0x3c (None = no gfx mgr).
    pub fn new(frame_owner: u32, ltm: DTMatrix, screen: Option<(u32, u32)>) -> Self {
        let mut c = EFCamera {
            uid: 0,
            flags: 0,
            ltm,
            frame_owner,
            fog_color: 0,
            rw: RwCamera::default(),
            raster: None,
            zraster: None,
            clear_color: [0; 4],
            clear_mode: 3,
            guard_scale: [1.0, 1.0],
            frustum: Default::default(),
            world_frustum: Default::default(),
            guard_frustum: Default::default(),
            world_guard_frustum: Default::default(),
            frustum_dirty: 0,
            render_flag: 0xffff_ffef,
            fog_enabled: 0,
            fog_type: 1,
            fog_density: 0.5,
        };
        c.SetDisplayArea([0.0, 0.0, 1.0, 1.0], screen.unwrap_or((0, 0)));
        let (x, y) = match screen {
            None => (0.75, 1.0),
            Some((w, h)) => (0.5, (h as f32 * 0.5) / w as f32),
        };
        c.SetViewWindow(x, y);
        c.flags |= 1;
        c.frustum_dirty = 1;
        c
    }
    /// EFCamera::UpdateGuardBand (0x0026b890)
    pub fn UpdateGuardBand(&mut self) {
        let [x, y] = self.GetViewWindow();
        self.guard_frustum = EFFrustum::SetFromCameraParams(
            x * self.guard_scale[0] * hscale(),
            y * self.guard_scale[1],
            self.rw.near,
            self.rw.far,
            self.GetCameraProjection(),
        );
        let g = self.guard_frustum;
        g.TransformPlanes(&self.ltm, &mut self.world_guard_frustum);
    }
    /// The frustum refresh both ProjectOntoFrustum and IsSphereInFrustrum (API) do first.
    pub fn refresh_frustum(&mut self) {
        if self.frustum_dirty != 0 {
            let [x, y] = self.GetViewWindow();
            self.frustum = EFFrustum::SetFromCameraParams(x * hscale(), y, self.rw.near, self.rw.far, self.GetCameraProjection());
            self.frustum_dirty = 0;
        }
        let f = self.frustum;
        f.TransformPlanes(&self.ltm, &mut self.world_frustum);
        self.UpdateGuardBand();
    }
    /// EFCamera::SetClearColor (0x0026d100)
    pub fn SetClearColor(&mut self, c: [f32; 4]) {
        for i in 0..4 {
            self.clear_color[i] = f2u(c[i] * 255.0) as u8;
        }
    }
    /// EFCamera::SetFOV (0x0026d350): x = tan(fov/2), y = x / (vw.x / vw.y)
    pub fn SetFOV(&mut self, fov: f32) {
        let [a, b] = self.rw.view_window;
        let x = (fov / 2.0).tan();
        self.rw.view_window = [x, x / (a / b)];
        self.frustum_dirty = 1;
    }
    /// EFCamera::SetViewWindow (0x0026d3d0)
    pub fn SetViewWindow(&mut self, x: f32, y: f32) {
        self.rw.view_window = [x, y];
        self.frustum_dirty = 1;
    }
    /// EFCamera::GetViewWindow (0x0026d410)
    pub fn GetViewWindow(&self) -> [f32; 2] {
        self.rw.view_window
    }
    /// EFCamera::SetDisplayArea (0x0026d430): rect = normalised {x, y, w, h}; screen = gfx mgr size.
    pub fn SetDisplayArea(&mut self, r: [f32; 4], screen: (u32, u32)) {
        if self.raster.is_none() && self.zraster.is_none() {
            let (w, h) = (screen.0 as i32 as f32, screen.1 as i32 as f32);
            let fx = r[0] * w;
            let fy = r[1] * h;
            let x = fx as i32;
            let y = fy as i32;
            let rw = (((r[2] * w + fx) - x as f32) + 0.5) as i32;
            let rh = (((r[3] * h + fy) - y as f32) + 0.5) as i32;
            // RwRasterCreate(0,0,0,camera|zbuffer) + RwRasterSubRaster(parent = gfx mgr rasters)
            self.rw.frame_raster = Some([x, y, rw, rh]);
            self.rw.z_raster = Some([x, y, rw, rh]);
        }
    }
    /// EFCamera::GetDisplayArea (0x0026d590): {0, 0, raster w, raster h}
    pub fn GetDisplayArea(&self) -> [f32; 4] {
        let r = self.rw.frame_raster.unwrap_or([0; 4]);
        [0.0, 0.0, r[2] as f32, r[3] as f32]
    }
    /// EFCamera::SetCameraProjection (0x0026d5d0): 1 = parallel, else perspective
    pub fn SetCameraProjection(&mut self, p: u8) {
        self.rw.projection = if p == 1 { 2 } else { 1 };
        self.frustum_dirty = 1;
    }
    /// EFCamera::GetCameraProjection (0x0026d620): true = parallel
    pub fn GetCameraProjection(&self) -> bool {
        self.rw.projection == 2
    }
    /// EFCamera_SP::SetRasters (0x0026d640)
    pub fn SetRasters(&mut self, r: Option<RasterRect>, z: Option<RasterRect>) {
        self.raster = r;
        self.zraster = z;
        self.rw.frame_raster = r;
        self.rw.z_raster = z;
    }
    /// RwCameraSetNearClipPlane / SetFarClipPlane
    pub fn set_near(&mut self, v: f32) {
        self.rw.near = v;
    }
    pub fn set_far(&mut self, v: f32) {
        self.rw.far = v;
    }
}

/// EFCameraAnimNode (class 0x53, "EF_CAMERA_ANIM_NODE", 0x48 bytes, DTOperator type 7): drives
/// a camera from a DTAnimHdr. Created by EFGameObject::CreateInstances (0x001ff350) for the
/// DT_CAMERA_ANIM_DESCR descriptors, connected to a camera by APIanim::Connect /
/// EFCutScenePlayer::Play (vtable +0x28), updated by the EFAnimationMgr tree.
/// The EFAnimNode state/time fields use the same bit meanings as anim::nodes::state.
#[derive(Debug, Clone)]
pub struct EFCameraAnimNode {
    /// +0x30 -> res (+0x10 DTAnimHdr)
    pub anim: Arc<DTAnimHdr>,
    /// +0x24 exclusivity key (EFAnimNode::Initialize: hdr byte +0xd)
    pub excl: u32,
    /// +0x2c creation params +4 (EFGameObject::ResolveRefPUID); only FindExclNode reads it
    pub obj: u32,
    /// +0x34
    pub state: u32,
    /// +0x38
    pub time: f32,
    /// +0x3c duration (Initialize: hdr +8)
    pub duration: f32,
    /// +0x40
    pub speed: f32,
    /// +0x20 user value passed to Notify
    pub user: u32,
    /// +0x44 connected camera uid (Connect accepts only uid>>24 == 0x10)
    pub camera: Option<u32>,
    /// EFAnimNode::Notify(code, user) calls (code 0 = Stop, 1 = end), drained by the coordinator
    pub notifies: Vec<(u32, u32)>,
    /// what the last Update wrote to the camera (applied by the coordinator, which owns the
    /// cameras and the camera RwFrame shared with the owning game object)
    pub out: Option<CameraAnimOut>,
}

/// One EFCameraAnimNode::Update write: SetFOV calls (channel order), then the camera RwFrame
/// modelling matrix (SetToRotation if any rotation channel, translation always) + RwFrameUpdateObjects.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CameraAnimOut {
    pub camera: u32,
    pub fovs: Vec<f32>,
    pub rot: Option<[f32; 3]>,
    pub pos: [f32; 3],
}

impl CameraAnimOut {
    /// Applies the write to camera `self.camera`; `frame` = the camera's RwFrame (the owning game
    /// object's frame), else the camera LTM itself.
    pub fn Apply(&self, cams: &mut EFGraphicsMgrCameras, frame: Option<&mut DTMatrix>) {
        let Some(c) = cams.GetCamera(self.camera) else { return };
        for &f in &self.fovs {
            c.SetFOV(f);
        }
        let mut m = frame.as_deref().copied().unwrap_or(c.ltm);
        if let Some(e) = self.rot {
            m.SetToRotation(e);
        }
        m.pos = self.pos;
        c.ltm = m;
        if let Some(f) = frame {
            *f = m;
        }
    }
}

impl EFCameraAnimNode {
    /// ctor (0x002369d0) + EFBaseIDescriptor::CreateNode (res) + Initialize (0x00236a70)
    pub fn new(anim: Arc<DTAnimHdr>, user: u32) -> Self {
        let duration = anim.duration;
        let excl = anim.unkd[0] as u32;
        EFCameraAnimNode { anim, excl, obj: 0, state: 0x800_0000, time: 0.0, duration, speed: 1.0, user, camera: None, notifies: vec![], out: None }
    }
    pub fn GetClassID(&self) -> u32 {
        0x53
    }
    /// 0x00237090
    pub fn Connect(&mut self, uid: u32) {
        if uid >> 24 == 0x10 {
            self.camera = Some(uid);
        }
    }
    /// EFAnimNode::Play: just assigns the state (float ignored)
    pub fn Play(&mut self, flags: u32, _f: f32) {
        self.state = flags;
    }
    /// EFCameraAnimNode::Stop (0x00237060): LeaveParent (done by the mgr) + EFAnimNode::Stop
    pub fn Stop(&mut self) {
        if self.state != 0x800_0000 {
            self.state = 0x800_0000;
            self.time = 0.0;
            self.notifies.push((0, self.user));
        }
    }
    /// EFCameraAnimNode::Update (0x00236ae0)
    pub fn Update(&mut self, dt: f32) -> u32 {
        let Some(uid) = self.camera else { return 0 };
        if self.state & 0x800_0000 == 0 && self.duration > 0.0 {
            self.time += dt * self.speed;
            self.state &= 0xbfff_ffff;
            if self.duration <= self.time {
                if self.state & 2 != 0 {
                    while self.duration < self.time {
                        self.time -= self.duration;
                        self.state |= 0x4000_0000;
                    }
                } else if self.state & 1 != 0 {
                    self.time = self.duration;
                    self.state |= 0x200_0000;
                }
            } else if self.time <= 0.0 {
                if self.state & 2 != 0 {
                    while self.time < 0.0 {
                        self.time += self.duration;
                        self.state |= 0x4000_0000;
                    }
                } else if self.state & 1 != 0 {
                    self.time = 0.0;
                    self.state |= 0x200_0000;
                }
            }
            if self.state & 0x4000_0000 != 0 {
                self.state |= 0x200_0000;
            }
        }
        let mut rot = false;
        let mut out = CameraAnimOut { camera: uid, ..Default::default() };
        let mut e = [0.0f32; 3];
        for set in &self.anim.sets {
            for ch in &set.channels {
                let v = ch.curve.Evaluate(self.time);
                match ch.id & 0xff_ffff {
                    // quirk: SetFOV is applied immediately, in channel order
                    0x14 => out.fovs.push((((12.44712 / v) as f64).atan() * 2.0) as f32),
                    8 => {
                        rot = true;
                        e[2] = v;
                    }
                    7 => {
                        rot = true;
                        e[1] = v + std::f32::consts::PI;
                    }
                    6 => {
                        rot = true;
                        e[0] = -v;
                    }
                    5 => out.pos[2] = v,
                    4 => out.pos[1] = v,
                    3 => out.pos[0] = v,
                    _ => {}
                }
            }
        }
        if rot {
            out.rot = Some(e);
        }
        self.out = Some(out);
        if self.state & 1 != 0
            && ((self.speed < 0.0 && self.time == 0.0) || (0.0 < self.speed && self.time == self.duration))
        {
            self.state |= 0x2000_0000;
        }
        if self.state & 0x200_0000 != 0 {
            self.notifies.push((1, self.user));
            self.state &= 0xfdff_ffff;
        }
        0
    }
}

/// Camera slots of EFGraphicsMgr (+0x4: 2 x EFCamera*, +0x6c count, +0x38/+0x3c screen size).
/// Named separately so that WP-C's EFGraphicsMgr can embed it.
#[derive(Debug, Clone, Default)]
pub struct EFGraphicsMgrCameras {
    pub cameras: [Option<EFCamera>; 2],
    /// +0x6c
    pub count: i32,
    /// +0x38 / +0x3c (None = no graphics manager)
    pub screen: Option<(u32, u32)>,
}

impl EFGraphicsMgrCameras {
    pub fn new(screen_w: u32, screen_h: u32) -> Self {
        EFGraphicsMgrCameras { screen: Some((screen_w, screen_h)), ..Default::default() }
    }
    /// EFGraphicsMgr::CreateCamera (0x0026b130): first free of 2 slots; uid = slot | 0x10000000.
    /// EFWorld::AddCameraInstance = RpWorldAddCamera -> rw.in_world.
    pub fn CreateCamera(&mut self, frame_owner: u32, ltm: DTMatrix) -> u32 {
        let Some(slot) = self.cameras.iter().position(|c| c.is_none()) else { return 0xffff_ffff };
        let mut c = EFCamera::new(frame_owner, ltm, self.screen);
        c.rw.in_world = true;
        let uid = (slot as u32 & 0xff_ffff) | 0x1000_0000;
        c.uid = uid;
        self.cameras[slot] = Some(c);
        self.count += 1;
        uid
    }
    /// EFGraphicsMgr::GetCamera (0x0026b200): slot = uid & 0xfff (quirk: no bound check in the
    /// original; out-of-range slots read the next fields -> here None).
    pub fn GetCamera(&mut self, uid: u32) -> Option<&mut EFCamera> {
        if uid == 0xffff_ffff {
            return None;
        }
        self.cameras.get_mut((uid & 0xfff) as usize)?.as_mut()
    }
    /// EFGraphicsMgr::DestroyCamera (0x0026b230)
    pub fn DestroyCamera(&mut self, uid: u32) {
        if uid == 0xffff_ffff {
            return;
        }
        if let Some(s) = self.cameras.get_mut((uid & 0xfff) as usize) {
            if s.take().is_some() {
                self.count -= 1;
            }
        }
    }

    // ---- APIcamera (EFApi_Camera.cpp). Signatures follow api_gen.rs; CreateCamera additionally needs
    // the object's frame (EFWorld::GetGameObject is the coordinator's), passed as `obj`.

    /// CreateCamera (0x001eb3f0). `obj` = Some(object LTM) if EFWorld::GetGameObject(a0) found it.
    /// The coordinator must also do GetClippingParams/SetClippingParams(…,{0,0},1) on the object.
    pub fn APIcamera_CreateCamera(&mut self, _mem: &mut Mem, a0: u32, obj: Option<DTMatrix>) -> u32 {
        match obj {
            Some(m) => self.CreateCamera(a0, m),
            None => 0xffff_ffff,
        }
    }
    pub fn APIcamera_DestroyCamera(&mut self, _mem: &mut Mem, a0: u32) -> u32 {
        self.DestroyCamera(a0);
        0
    }
    pub fn APIcamera_Activate(&mut self, _mem: &mut Mem, a0: u32, a1: u32) -> u32 {
        if let Some(c) = self.GetCamera(a0) {
            if a1 == 0 { c.flags &= !1 } else { c.flags |= 1 }
        }
        0
    }
    /// SetPriority: empty in EXE.1
    pub fn APIcamera_SetPriority(&mut self, _mem: &mut Mem, _a0: u32, _a1: f32) -> u32 {
        0
    }
    /// GetPriority: always -1.0f
    pub fn APIcamera_GetPriority(&mut self, _mem: &mut Mem, _a0: u32) -> u32 {
        0xbf80_0000
    }
    pub fn APIcamera_SetFarClipPlane(&mut self, _mem: &mut Mem, a0: u32, a1: f32) -> u32 {
        if let Some(c) = self.GetCamera(a0) {
            c.set_far(a1);
            c.frustum_dirty = 1;
        }
        0
    }
    pub fn APIcamera_SetNearClipPlane(&mut self, _mem: &mut Mem, a0: u32, a1: f32) -> u32 {
        if let Some(c) = self.GetCamera(a0) {
            c.set_near(if a1 < 0.1 { 0.1 } else { a1 });
            c.frustum_dirty = 1;
        }
        0
    }
    pub fn APIcamera_SetClearColor(&mut self, mem: &mut Mem, a0: u32, a1: Ptr) -> u32 {
        if let Some(c) = self.GetCamera(a0) {
            let col = [mem.rf(a1), mem.rf(a1 + 4), mem.rf(a1 + 8), mem.rf(a1 + 0xc)];
            c.SetClearColor(col);
        }
        0
    }
    pub fn APIcamera_SetClearMode(&mut self, _mem: &mut Mem, a0: u32, a1: u32) -> u32 {
        if let Some(c) = self.GetCamera(a0) {
            c.clear_mode = a1;
        }
        0
    }
    pub fn APIcamera_SetProjection(&mut self, _mem: &mut Mem, a0: u32, a1: u32) -> u32 {
        if let Some(c) = self.GetCamera(a0) {
            c.SetCameraProjection(a1 as u8);
        }
        0
    }
    pub fn APIcamera_GetViewWindow(&mut self, mem: &mut Mem, a0: u32, a1: Ptr, a2: Ptr) -> u32 {
        if let Some(c) = self.GetCamera(a0) {
            let [x, y] = c.GetViewWindow();
            mem.wf(a1, x);
            mem.wf(a2, y);
        }
        0
    }
    pub fn APIcamera_SetViewWindow(&mut self, _mem: &mut Mem, a0: u32, a1: f32, a2: f32) -> u32 {
        if let Some(c) = self.GetCamera(a0) {
            c.SetViewWindow(a1, a2);
        }
        0
    }
    pub fn APIcamera_SetDisplayArea(&mut self, mem: &mut Mem, a0: u32, a1: Ptr) -> u32 {
        let screen = self.screen.unwrap_or((0, 0));
        if let Some(c) = self.GetCamera(a0) {
            let r = [mem.rf(a1), mem.rf(a1 + 4), mem.rf(a1 + 8), mem.rf(a1 + 0xc)];
            c.SetDisplayArea(r, screen);
        }
        0
    }
    pub fn APIcamera_GetDisplayArea(&mut self, mem: &mut Mem, a0: u32, a1: Ptr) -> u32 {
        if let Some(c) = self.GetCamera(a0) {
            let r = c.GetDisplayArea();
            // write order of the original: +0, +4, +0xc, +8
            mem.wf(a1, r[0]);
            mem.wf(a1 + 4, r[1]);
            mem.wf(a1 + 0xc, r[3]);
            mem.wf(a1 + 8, r[2]);
        }
        0
    }
    pub fn APIcamera_SetRenderFlag(&mut self, _mem: &mut Mem, a0: u32, a1: u32) -> u32 {
        if let Some(c) = self.GetCamera(a0) {
            c.render_flag = a1;
        }
        0
    }
    pub fn APIcamera_SetFOV(&mut self, _mem: &mut Mem, a0: u32, a1: f32) -> u32 {
        if let Some(c) = self.GetCamera(a0) {
            c.SetFOV(a1);
        }
        0
    }
    /// TakeSnapShot: GS frame-buffer dump to a file (debug tool). Unported: logged no-op.
    pub fn APIcamera_TakeSnapShot(&mut self, mem: &mut Mem, a0: u32, a1: Ptr) -> u32 {
        if self.GetCamera(a0).is_some() {
            log::info!("APIcamera::TakeSnapShot({}) ignored", mem.cstr_at(a1));
        }
        0
    }
    /// ProjectOntoFrustum (a1 = in point, a2 = out point). quirk: no NULL check on the camera in the
    /// original (crash); here returns 0.
    pub fn APIcamera_ProjectOntoFrustum(&mut self, mem: &mut Mem, a0: u32, a1: Ptr, a2: Ptr) -> u8 {
        let Some(c) = self.GetCamera(a0) else { return 0 };
        c.refresh_frustum();
        let mut out = [0.0; 3];
        let r = c.world_frustum.ProjectOntoFrustum(&c.ltm, mem.rv3(a1), &mut out);
        mem.wv3(a2, out);
        r as u8
    }
    /// IsSphereInFrustrum: 0 outside, 1 fully inside, 2 intersecting.
    pub fn APIcamera_IsSphereInFrustrum(&mut self, mem: &mut Mem, a0: u32, a1: Ptr, a2: f32) -> u32 {
        let Some(c) = self.GetCamera(a0) else { return 0 };
        c.refresh_frustum();
        c.IsSphereInFrustrum(mem.rv3(a1), a2)
    }
    pub fn APIcamera_EnableFog(&mut self, _mem: &mut Mem, a0: u32, a1: i32) -> u32 {
        if let Some(c) = self.GetCamera(a0) {
            c.fog_enabled = (a1 != 0) as u32;
        }
        0
    }
    pub fn APIcamera_SetFogType(&mut self, _mem: &mut Mem, a0: u32, a1: u32) -> u32 {
        if let Some(c) = self.GetCamera(a0) {
            c.fog_type = a1;
        }
        0
    }
    pub fn APIcamera_GetFogType(&mut self, _mem: &mut Mem, a0: u32) -> u32 {
        self.GetCamera(a0).map_or(0, |c| c.fog_type)
    }
    /// SetFogColor. quirk: the CColor floats are truncated to int WITHOUT scaling by 255 (so 0..1
    /// colours give 0/1); packed as A<<24 | R<<16 | G<<8 | B (alpha not masked).
    pub fn APIcamera_SetFogColor(&mut self, mem: &mut Mem, a0: u32, a1: Ptr) -> u32 {
        let col = [mem.rf(a1), mem.rf(a1 + 4), mem.rf(a1 + 8), mem.rf(a1 + 0xc)];
        if let Some(c) = self.GetCamera(a0) {
            let [r, g, b, a] = col.map(|v| if 2.147_483_6e9 <= v { v - 2.147_483_6e9 } else { v } as i32 as u32);
            c.fog_color = (b & 0xff) | (g & 0xff) << 8 | a << 24 | (r & 0xff) << 16;
        }
        0
    }
    pub fn APIcamera_SetFogDensity(&mut self, _mem: &mut Mem, a0: u32, a1: f32) -> u32 {
        if let Some(c) = self.GetCamera(a0) {
            c.fog_density = a1;
        }
        0
    }
    /// returns the float bits
    pub fn APIcamera_GetFogDensity(&mut self, _mem: &mut Mem, a0: u32) -> u32 {
        self.GetCamera(a0).map_or(0, |c| c.fog_density.to_bits())
    }
    pub fn APIcamera_SetFogDistance(&mut self, _mem: &mut Mem, a0: u32, a1: f32) -> u32 {
        if let Some(c) = self.GetCamera(a0) {
            c.rw.fog_plane = a1;
        }
        0
    }
    /// returns the float bits
    pub fn APIcamera_GetFogDistance(&mut self, _mem: &mut Mem, a0: u32) -> u32 {
        self.GetCamera(a0).map_or(0, |c| c.rw.fog_plane.to_bits())
    }
}

impl EFCamera {
    /// The sphere loop of APIcamera IsSphereInFrustrum (0x001eb..): world frustum planes.
    pub fn IsSphereInFrustrum(&self, p: [f32; 3], r: f32) -> u32 {
        let mut cross = false;
        for pl in &self.world_frustum.planes {
            let d = pl.dist(p);
            let k = if d <= r { if -r <= d { 2 } else { 1 } } else { 0 };
            if k == 0 {
                return 0;
            }
            if k == 2 {
                cross = true;
            }
        }
        if cross { 2 } else { 1 }
    }
}

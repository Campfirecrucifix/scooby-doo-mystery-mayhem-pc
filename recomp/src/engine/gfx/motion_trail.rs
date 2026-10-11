//! EFMotionTrailMgr_BC / EFMotionTrailMgr_SP (Classes/EF/EFMotionTrailMgr*.cpp), the global
//! `goMotionTrailMgr`. Logic 1:1, including the DTMap<uint,_sSystem*> iteration order
//! (37 buckets, key % 37, head-inserted chains) which decides update/render order.
//!
//! Replacements: RwFrame* cache of a control point (+0x8) -> the resolved (object, joint) key,
//! the LTM is queried from [`GfxWorld::GetJointLTM`] every update (RwFrameGetLTM);
//! RwIm3DTransform/RenderIndexedPrimitive(TRISTRIP) -> [`MTStrip`] draw data.
#![allow(non_snake_case, non_camel_case_types)]

use super::api::GfxWorld;

/// DTMap bucket count passed by the constructor (0x25)
pub const MT_BUCKETS: usize = 0x25;

/// _sControlPoint (0x28 bytes)
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct _sControlPoint {
    /// +0x0 life span (seconds)
    pub lifespan: f32,
    /// +0x4 game object uid (EFGameObject +8, so SELF is resolved at SetJoint time)
    pub obj: u32,
    /// +0x8 cached frame (None = NULL): replaced by the resolved (obj, joint index/id) key
    pub frame: Option<(u32, u32)>,
    /// +0xc joint (index, or HAnim node id when the high half is set)
    pub joint: u32,
    /// +0x10 fixed position (system flag 0x1000)
    pub position: [f32; 3],
    /// +0x1c rgb
    pub rgb: [f32; 3],
}

/// _sVertex (0x10 bytes): position + age
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct _sVertex {
    pub pos: [f32; 3],
    /// +0xc
    pub age: f32,
}

/// _sSystem (0x28 bytes)
#[derive(Debug, Clone, PartialEq)]
pub struct _sSystem {
    /// +0x0 flags: 1 tracking, 2 drawing, 0x1000 use fixed positions (CreateSystem param 3)
    pub flags: u32,
    /// +0x4 id
    pub id: u32,
    /// +0x8 sample period (init 1e8 = 0x4cbebc20)
    pub frequency: f32,
    /// +0xc timer
    pub timer: f32,
    /// +0x10 control points / +0x14 array
    pub cps: Vec<_sControlPoint>,
    /// +0x18 samples (rows of the ring buffer)
    pub samples: u32,
    /// +0x1c vertices [samples][control points]
    pub verts: Vec<_sVertex>,
    /// +0x20 head row, +0x24 tail row
    pub head: u32,
    pub tail: u32,
}

/// One RwIm3DVertex (0x24 bytes; only pos and colour are written)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MTVertex {
    pub pos: [f32; 3],
    pub rgba: [u8; 4],
}

/// One RwIm3DRenderIndexedPrimitive(rwPRIMTYPETRISTRIP) call. Render states around all strips:
/// VERTEXALPHA on (0xc=1), SRCBLEND ONE (10=2), DESTBLEND ONE (0xb=2) = additive, z-write off,
/// cull none (0x14=1); the texture state is not touched (whatever was bound stays).
#[derive(Debug, Clone, PartialEq)]
pub struct MTStrip {
    pub system: u32,
    pub verts: Vec<MTVertex>,
    pub indices: Vec<u16>,
}

/// EFMotionTrailMgr_BC + EFMotionTrailMgr_SP
#[derive(Debug, Clone, PartialEq)]
pub struct EFMotionTrailMgr_BC {
    /// +0x4 next id (starts at 0, never reset)
    pub next_id: u32,
    /// +0x8 DTMap: +0xc buckets, +0x10 bucket count, +0x14 count; chains head first
    pub buckets: Vec<Vec<_sSystem>>,
    pub count: u32,
}

impl Default for EFMotionTrailMgr_BC {
    fn default() -> Self {
        Self::new()
    }
}

/// KeyToHashIndex(uint, uint) (0x0010bd40): signed modulo
fn hash(key: u32) -> usize {
    ((key as i32) % (MT_BUCKETS as i32)).rem_euclid(MT_BUCKETS as i32) as usize
}

impl EFMotionTrailMgr_BC {
    /// EFMotionTrailMgr_BC::EFMotionTrailMgr_BC (0x002608c0)
    pub fn new() -> Self {
        EFMotionTrailMgr_BC { next_id: 0, buckets: vec![Vec::new(); MT_BUCKETS], count: 0 }
    }
    /// EFMotionTrailMgr_BC::Initialize (0x00260970)
    pub fn Initialize(&mut self) -> u32 {
        0
    }
    /// EFMotionTrailMgr_BC::Terminate (0x00260980): deletes every system (id counter kept)
    pub fn Terminate(&mut self) -> u32 {
        for b in &mut self.buckets {
            b.clear();
        }
        self.count = 0;
        0
    }
    /// DTMap::Find
    pub fn Get(&mut self, id: u32) -> Option<&mut _sSystem> {
        self.buckets[hash(id)].iter_mut().find(|s| s.id == id)
    }
    /// systems in DTMap iteration order (bucket ascending, chain head first)
    pub fn Systems(&self) -> impl Iterator<Item = &_sSystem> {
        self.buckets.iter().flatten()
    }
    /// EFMotionTrailMgr_BC::CreateSystem (0x00260c70)
    pub fn CreateSystem(&mut self, n_cp: u32, samples: u32, flags: u32) -> u32 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        let s = _sSystem {
            flags,
            id,
            frequency: f32::from_bits(0x4cbe_bc20),
            timer: 0.0,
            cps: vec![_sControlPoint::default(); n_cp as usize],
            samples,
            verts: vec![_sVertex::default(); (n_cp * samples) as usize],
            head: 0,
            tail: 0,
        };
        let b = &mut self.buckets[hash(id)];
        if let Some(e) = b.iter_mut().find(|e| e.id == id) {
            // DTMap::Insert on an existing key replaces the value (the old system leaks)
            *e = s;
        } else {
            b.insert(0, s);
            self.count += 1;
        }
        id
    }
    /// EFMotionTrailMgr_BC::DestroySystem (0x00260eb0). quirk: an unknown id deletes NULL+0x1c
    /// in the C++ (crash); no-op here.
    pub fn DestroySystem(&mut self, id: u32) {
        let b = &mut self.buckets[hash(id)];
        if let Some(i) = b.iter().position(|e| e.id == id) {
            b.remove(i);
            self.count -= 1;
        }
    }
    // quirk (all setters): an unknown id writes through an uninitialised register in the C++;
    // no-op here.
    /// EFMotionTrailMgr_BC::SetFrequency (0x00261000)
    pub fn SetFrequency(&mut self, id: u32, f: f32) {
        if let Some(s) = self.Get(id) {
            s.frequency = f;
        }
    }
    /// EFMotionTrailMgr_BC::SetLifeSpan (0x00261090)
    pub fn SetLifeSpan(&mut self, id: u32, cp: u32, t: f32) {
        if let Some(c) = self.Get(id).and_then(|s| s.cps.get_mut(cp as usize)) {
            c.lifespan = t;
        }
    }
    /// EFMotionTrailMgr_BC::SetJoint (0x00261140): stores EFWorld::GetGameObject(obj)+8 and the
    /// joint; quirk: the cached frame (+8) is NOT reset. A missing object crashes the C++ (NULL+8);
    /// 0 is stored here.
    pub fn SetJoint(&mut self, id: u32, cp: u32, obj: u32, joint: u32, w: &dyn GfxWorld) {
        let uid = w.ResolveGameObject(obj).unwrap_or(0);
        if let Some(c) = self.Get(id).and_then(|s| s.cps.get_mut(cp as usize)) {
            c.obj = uid;
            c.joint = joint;
        }
    }
    /// EFMotionTrailMgr_BC::SetPosition (0x00261220)
    pub fn SetPosition(&mut self, id: u32, cp: u32, p: [f32; 3]) {
        if let Some(c) = self.Get(id).and_then(|s| s.cps.get_mut(cp as usize)) {
            c.position = p;
        }
    }
    /// EFMotionTrailMgr_BC::SetRGB (0x002612f0)
    pub fn SetRGB(&mut self, id: u32, cp: u32, r: f32, g: f32, b: f32) {
        if let Some(c) = self.Get(id).and_then(|s| s.cps.get_mut(cp as usize)) {
            c.rgb = [r, g, b];
        }
    }
    /// EFMotionTrailMgr_BC::Start (0x002613d0): flags |= 3, timer = head = tail = 0
    pub fn Start(&mut self, id: u32) {
        if let Some(s) = self.Get(id) {
            s.flags |= 3;
            s.timer = 0.0;
            s.head = 0;
            s.tail = 0;
        }
    }
    /// EFMotionTrailMgr_BC::StopTracking (0x00261460)
    pub fn StopTracking(&mut self, id: u32) {
        if let Some(s) = self.Get(id) {
            s.flags &= !1;
        }
    }
    /// EFMotionTrailMgr_BC::StopDraw (0x002614f0)
    pub fn StopDraw(&mut self, id: u32) {
        if let Some(s) = self.Get(id) {
            s.flags &= !3;
        }
    }
    /// EFMotionTrailMgr_BC::Update(float) (0x00260b60), called by EFEngineController::MainLoop
    /// after EFWorld::Update / EFPaletteMgr_SP::Update, before EFGraphicsMgr::Render.
    pub fn Update(&mut self, dt: f32, w: &dyn GfxWorld) -> u32 {
        for b in &mut self.buckets {
            for s in b.iter_mut() {
                Self::UpdateSystem(s, dt, w);
            }
        }
        0
    }
    /// EFMotionTrailMgr_BC::Update(_sSystem*, float) (0x00261580)
    pub fn UpdateSystem(s: &mut _sSystem, dt: f32, w: &dyn GfxWorld) -> u32 {
        let n = s.cps.len();
        let ns = s.samples.max(1);
        if s.flags & 1 != 0 {
            // quirk: dt is not added on the frame that emits a new row
            if s.frequency <= s.timer {
                s.timer -= s.frequency;
                s.head = (s.head + 1) % ns;
                if s.tail == s.head {
                    s.tail = (s.tail + 1) % ns;
                }
            } else {
                s.timer += dt;
            }
            let row = s.head as usize * n;
            for i in 0..n {
                let cp = s.cps[i];
                let v = &mut s.verts[row + i];
                if s.flags & 0x1000 == 0 {
                    let mut frame = cp.frame;
                    if frame.is_none() {
                        if w.ResolveGameObject(cp.obj).is_none() {
                            s.flags &= !3;
                            return 0xffff_ffff;
                        }
                        // Get3dObject(0); no hierarchy (+0x2c & 4 == 0) or unknown node -> NULL
                        if w.GetJointLTM(cp.obj, cp.joint).is_some() {
                            frame = Some((cp.obj, cp.joint));
                        }
                        s.cps[i].frame = frame;
                    }
                    // quirk: RwFrameGetLTM(NULL) crashes the C++; the position is left as is
                    if let Some(m) = frame.and_then(|(o, j)| w.GetJointLTM(o, j)) {
                        v.pos = m.pos;
                    }
                } else {
                    v.pos = cp.position;
                }
                v.age = 0.0;
            }
        }
        let mut r = s.tail;
        while r != s.head {
            for i in 0..n {
                let v = &mut s.verts[r as usize * n + i];
                v.age += dt;
                if s.cps[i].lifespan < v.age {
                    v.age = s.cps[i].lifespan;
                }
            }
            r = (r + 1) % ns;
        }
        0
    }

    /// EFMotionTrailMgr_SP::Render (0x00261a50): one triangle strip per drawing system, rows
    /// tail..=head, vertex colour = rgb * (1 - age/lifespan) * 255, alpha 0xff; drawn only
    /// when at least 2 rows of vertices exist.
    pub fn Render(&self) -> Vec<MTStrip> {
        let mut out = Vec::new();
        for s in self.Systems() {
            if s.flags & 2 == 0 {
                continue;
            }
            let n = s.cps.len();
            let ns = s.samples.max(1);
            let mut verts = Vec::new();
            let mut idx: Vec<u16> = Vec::new();
            let mut k: u16 = 0;
            let mut r = s.tail;
            loop {
                idx.push(k);
                for i in 0..n {
                    let v = s.verts[r as usize * n + i];
                    let c = s.cps[i];
                    let f = 1.0 - v.age / c.lifespan;
                    let b = |x: f32| {
                        let y = f * x * 255.0;
                        if y < 2147483648.0 { (y as i32) as u8 } else { ((y - 2147483648.0) as i32) as u8 }
                    };
                    verts.push(MTVertex { pos: v.pos, rgba: [b(c.rgb[0]), b(c.rgb[1]), b(c.rgb[2]), 0xff] });
                    idx.push(k);
                    if r != s.head {
                        idx.push(k.wrapping_add(n as u16));
                    }
                    k = k.wrapping_add(1);
                }
                let last = *idx.last().unwrap();
                idx.push(last);
                r = (r + 1) % ns;
                if r == (s.head + 1) % ns {
                    break;
                }
            }
            if 2 * n <= verts.len() {
                out.push(MTStrip { system: s.id, verts, indices: idx });
            }
        }
        out
    }
}

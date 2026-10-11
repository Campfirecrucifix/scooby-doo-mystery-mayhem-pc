//! PLEmitter_BC / PLEmitter (0x124 bytes): Classes/PL/PLEmitter_BC.cpp, PLEmitter.cpp.

#![allow(non_snake_case)]

use super::*;
use crate::engine::camera::EFGraphicsMgrCameras;
use std::collections::VecDeque;

/// PLEmitter_BC::IF_EMITTER_PLANE_1 / _PLANE_2 / box / IF_EMITTER_CLIPPED bits of `iflags` (+0xe8).
pub const IF_EMITTER_PLANE_1: u32 = 1;
pub const IF_EMITTER_PLANE_2: u32 = 2;
pub const IF_EMITTER_BOX: u32 = 0x10;
pub const IF_EMITTER_CLIPPED: u32 = 0x20;

/// PLMgr_BC particle pool (+0x14 array of 600, +0x38 free list).
#[derive(Debug, Clone)]
pub struct PLParticlePool {
    pub parts: Vec<PLParticle>,
    /// PLLinkList<PLParticle> free list, front = head
    pub free: VecDeque<usize>,
    pub rng: CRand,
}

impl PLParticlePool {
    pub const NB: usize = 600;
    pub fn new() -> Self {
        // PLLinkList<PLParticle>::ResetFreeList(array, 600, 0x2c): array order
        PLParticlePool { parts: vec![PLParticle::default(); Self::NB], free: (0..Self::NB).collect(), rng: CRand::default() }
    }
}
impl Default for PLParticlePool {
    fn default() -> Self {
        Self::new()
    }
}

/// PLEmitter (PLEmitter_BC : EFPosData).
#[derive(Debug, Clone, PartialEq)]
pub struct PLEmitter {
    /// +0x8 id (0x2e000000 | n loaded, 0x2e800001.. created)
    pub id: u32,
    /// +0x10 owning EFGameObject (EmitterCreate: the 0x2a000000 manager object)
    pub owner: Option<u32>,
    /// +0x14 frame (None = no frame: LaunchParticle does nothing)
    pub frame: Option<PLFrame>,
    /// +0x2c flags: 2 = particles in emitter space, 0x10 = stopped/frozen, 0x20 = freeze when the
    /// world is paused, 0x40 = don't render when clipped, 0x80 = don't update when clipped,
    /// 0x100 = release particles when clipped. Default 0xf0.
    pub flags: u32,
    /// +0x34 char[0x20] name
    pub name: String,
    /// +0xb4 uid of the game object the emitter is attached to (0 = none)
    pub object_uid: u32,
    /// +0xb8 message handler (notification receiver), -1 = none
    pub msg_handler: u32,
    /// +0xbc PLLinkList<PLParticle>: pool indices, index 0 = head
    pub particles: Vec<usize>,
    /// +0xc8 started
    pub started: bool,
    /// +0xc9 continuous
    pub continuous: bool,
    /// +0xcc launch time accumulator
    pub acc: f32,
    /// +0xd0 box min, +0xdc box size
    pub box_min: V3,
    pub box_size: V3,
    /// +0xe8 internal flags (IF_EMITTER_*)
    pub iflags: u32,
    /// +0xec / +0xfc collision planes (normal, d)
    pub planes: [[f32; 4]; 2],
    /// +0x10c / +0x110 bounce factors
    pub bounce: [f32; 2],
    /// +0x114 bounding-sphere radius for clipping
    pub bsphere_radius: f32,
    /// +0x120 particles sent last frame (PLEmitter::RenderBatch counter)
    pub nb_rendered: u32,
}

impl Default for PLEmitter {
    fn default() -> Self {
        Self::PLEmitter()
    }
}

impl PLEmitter {
    pub const SIZE: usize = 0x124;
    /// PLEmitter_BC::PLEmitter_BC (0x0020cb90) + PLEmitter::PLEmitter (0x00238e80; the EFRefBuffer
    /// DMA buffer +0x118 is dropped)
    pub fn PLEmitter() -> Self {
        PLEmitter {
            id: 0,
            owner: None,
            frame: None,
            flags: 0xf0,
            name: String::new(),
            object_uid: 0,
            msg_handler: 0xffff_ffff,
            particles: Vec::new(),
            started: false,
            continuous: false,
            acc: 0.0,
            box_min: [0.0; 3],
            box_size: [0.0; 3],
            iflags: 0,
            planes: [[0.0; 4]; 2],
            bounce: [1.0, 1.0],
            bsphere_radius: 0.1,
            nb_rendered: 0,
        }
    }
    fn ltm(&self) -> DTMatrix {
        self.frame.map(|f| f.ltm).unwrap_or_default()
    }

    /// PLEmitter_BC::Start (0x0020cd70)
    pub fn Start(&mut self, continuous: u32) {
        self.started = true;
        self.flags &= !0x10;
        self.continuous = continuous as u8 != 0;
    }
    /// PLEmitter_BC::Reset (0x0020cda0)
    pub fn Reset(&mut self, pool: &mut PLParticlePool) {
        self.ReleaseParticlesFromList(pool);
        self.started = false;
        self.flags |= 0x10;
    }
    /// PLEmitter_BC::ReleaseParticlesFromList (0x0020cde0). quirk: the particle matrices are
    /// not freed here (RwMatrix leak in the original); the slot's matrix is simply overwritten later.
    pub fn ReleaseParticlesFromList(&mut self, pool: &mut PLParticlePool) {
        for p in self.particles.drain(..) {
            pool.free.push_front(p);
        }
    }

    /// PLEmitter_BC::LaunchParticleImmediately (0x0020ce50); returns false when the pool is empty.
    pub fn LaunchParticleImmediately(&mut self, ty: &PLTypeData, pool: &mut PLParticlePool) -> bool {
        let half = (ty.angle as i32) >> 1;
        let Some(pi) = pool.free.pop_front() else { return false };
        self.particles.insert(0, pi);
        let ltm = self.ltm();
        let mut off = [0.0f32; 3];
        if self.iflags & IF_EMITTER_BOX != 0 {
            for i in 0..3 {
                let r = pool.rng.rand();
                off[i] = self.box_min[i] + self.box_size[i] * ((((r & 0xfff) as i32) * 1000) >> 12) as f32 * 0.001;
            }
            off = mat_vec(&ltm, off);
        }
        let p = &mut pool.parts[pi];
        if ty.flags & 1 == 0 {
            p.matrix = None;
            p.pos = if self.flags & 2 == 0 { [ltm.pos[0] + off[0], ltm.pos[1] + off[1], ltm.pos[2] + off[2]] } else { off };
        } else {
            let mut m = if self.flags & 2 == 0 { ltm } else { DTMatrix::default() };
            for i in 0..3 {
                m.pos[i] += off[i];
            }
            p.matrix = Some(m);
            p.pos = m.pos;
        }
        let a = ty.angle as i32;
        let r = pool.rng.rand();
        let a1 = ((((r & 0xfff) as i32).wrapping_mul(a) >> 12) - half) as f32 * 0.0174;
        let (c1, s1) = (a1.cos(), a1.sin());
        let r = pool.rng.rand();
        let a2 = ((((r & 0xfff) as i32).wrapping_mul(a) >> 12) - half) as f32 * 0.0174;
        let v = [c1 * -a2.sin() * ty.speed, c1 * a2.cos() * ty.speed, s1 * ty.speed];
        let p = &mut pool.parts[pi];
        p.vel = mat_vec(&ltm, v);
        p.age = 0.0;
        p.size = ty.scale.v0;
        true
    }

    /// PLEmitter_BC::LaunchParticle (0x0020d330): returns the launch count.
    pub fn LaunchParticle(&mut self, ty: &PLTypeData, pool: &mut PLParticlePool, dt: f32, paused: bool) -> u16 {
        if self.flags & 0x20 != 0 {
            if paused {
                self.flags |= 0x10;
                return 0;
            }
            self.flags &= !0x10;
        }
        if !self.started || self.frame.is_none() {
            return 0;
        }
        if self.continuous {
            let acc = self.acc + dt;
            self.acc = acc;
            // quirk: the very first accumulation (acc == dt) always launches
            if acc != dt {
                if acc < ty.rate {
                    return 0;
                }
                self.acc = acc - ty.rate;
            }
        }
        for _ in 0..ty.count {
            self.LaunchParticleImmediately(ty, pool);
        }
        if !self.continuous {
            self.started = false;
        }
        ty.count
    }

    /// PLEmitter_BC::GetNbActivePart (0x0020d460)
    pub fn GetNbActivePart(&self) -> i32 {
        self.particles.len() as i32
    }

    /// PLEmitter_BC::UpdatePhysic (0x0020d4c0)
    pub fn UpdatePhysic(&mut self, ty: &PLTypeData, pool: &mut PLParticlePool, dt: f32) -> i32 {
        if self.flags & 0x10 != 0 {
            return self.GetNbActivePart();
        }
        let planes_on = self.iflags & 3;
        let mut n = 0;
        let mut i = 0;
        while i < self.particles.len() {
            let pi = self.particles[i];
            let p = &mut pool.parts[pi];
            p.age += dt;
            if ty.lifespan < p.age {
                self.particles.remove(i);
                p.matrix = None;
                pool.free.push_front(pi);
                continue;
            }
            if ty.flags & 1 != 0 {
                if let Some(m) = p.matrix {
                    p.pos = m.pos;
                }
            }
            for k in 0..3 {
                p.vel[k] += dt * ty.gravity[k];
            }
            let fr = dt * ty.friction;
            let d = [p.vel[0] * fr, p.vel[1] * fr, p.vel[2] * fr];
            for k in 0..3 {
                p.vel[k] -= d[k];
            }
            if planes_on != 0 {
                let side = |pl: &[f32; 4]| p.pos[2] * pl[2] + p.pos[0] * pl[0] + p.pos[1] * pl[1] - pl[3] <= 0.0;
                let mut hit = 0;
                if self.iflags & IF_EMITTER_PLANE_1 != 0 && side(&self.planes[0]) {
                    hit = 1;
                }
                if self.iflags & IF_EMITTER_PLANE_2 != 0 && side(&self.planes[1]) {
                    hit |= 2;
                }
                if hit == 3 {
                    let f = (self.bounce[0] + self.bounce[1]) * -0.5;
                    p.vel.iter_mut().for_each(|v| *v *= f);
                } else if hit != 0 {
                    let k = hit - 1;
                    let pl = self.planes[k];
                    let f = (p.vel[2] * pl[2] + p.vel[0] * pl[0] + p.vel[1] * pl[1]) * 2.0;
                    if f < 0.0 {
                        for j in 0..3 {
                            p.vel[j] -= pl[j] * f;
                        }
                        let b = self.bounce[k];
                        p.vel.iter_mut().for_each(|v| *v *= b);
                    }
                }
            }
            for k in 0..3 {
                p.pos[k] += p.vel[k] * dt;
            }
            if ty.flags & 1 != 0 {
                if let Some(m) = p.matrix.as_mut() {
                    m.pos = p.pos;
                }
            }
            n += 1;
            i += 1;
        }
        n
    }

    /// PLEmitter_BC::GetBBox (0x0020da10): mask 1/2 min/max x, 4/8 y, 0x10/0x20 z; 4 = no particle.
    pub fn GetBBox(&self, pool: &PLParticlePool, min: &mut V3, max: &mut V3, mask: u32) -> u32 {
        if self.particles.is_empty() {
            return 4;
        }
        let mut lo = [1e6f32; 3];
        let mut hi = [-1e6f32; 3];
        for &pi in &self.particles {
            let p = pool.parts[pi].pos;
            for k in 0..3 {
                if mask & (1 << (2 * k)) != 0 && p[k] < lo[k] {
                    lo[k] = p[k];
                }
                if mask & (2 << (2 * k)) != 0 && hi[k] < p[k] {
                    hi[k] = p[k];
                }
            }
        }
        for k in 0..3 {
            if mask & (1 << (2 * k)) != 0 {
                min[k] = lo[k];
            }
            if mask & (2 << (2 * k)) != 0 {
                max[k] = hi[k];
            }
        }
        0
    }
    /// PLEmitter_BC::SetBox (0x0020dc00)
    pub fn SetBox(&mut self, min: V3, max: V3) {
        self.box_min = min;
        self.box_size = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
        self.iflags |= IF_EMITTER_BOX;
    }
    /// PLEmitter_BC::SetCollisionPlane (0x0020dca0)
    pub fn SetCollisionPlane(&mut self, normal: V3, point: V3, bounce: f32, mask: u32) {
        for k in 0..2 {
            if mask & (1 << k) != 0 {
                let mut n = normal;
                let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                if l > 0.0 {
                    n.iter_mut().for_each(|x| *x /= l);
                }
                self.planes[k] = [n[0], n[1], n[2], n[2] * point[2] + n[0] * point[0] + n[1] * point[1]];
                self.bounce[k] = bounce;
                self.iflags |= 1 << k;
            }
        }
    }

    /// PLEmitter_BC::UpdateClippingState (0x0020ddf0): bounding sphere vs the world frustum of
    /// every active camera; notifies the message handler on visibility changes.
    pub fn UpdateClippingState(&mut self, pool: &mut PLParticlePool, cams: &mut EFGraphicsMgrCameras, notes: &mut Vec<PLNotification>) {
        if self.flags & 0xc0 == 0 {
            self.iflags &= !IF_EMITTER_CLIPPED;
            return;
        }
        let c = self.ltm().pos;
        let r = self.bsphere_radius;
        let count = cams.count.max(0) as usize;
        if count == 0 {
            return;
        }
        let mut visible = false;
        for slot in 0..count.min(cams.cameras.len()) {
            if visible {
                break;
            }
            // replacement: the original has no NULL check on GetCamera(slot)
            let Some(cam) = cams.cameras[slot].as_mut() else { continue };
            if cam.flags & 1 == 0 {
                continue;
            }
            cam.refresh_frustum();
            let mut res = 1;
            let mut inter = false;
            for pl in cam.world_frustum.planes.iter() {
                let d = pl.dist(c);
                if d > r {
                    res = 0;
                    break;
                }
                if -r <= d {
                    inter = true;
                }
            }
            if res != 0 && inter {
                res = 2;
            }
            if res != 0 {
                visible = true;
            }
        }
        if visible && self.iflags & IF_EMITTER_CLIPPED != 0 {
            self.iflags &= !IF_EMITTER_CLIPPED;
            if self.msg_handler != 0xffff_ffff {
                notes.push(PLNotification { msg: 0x1a, handler: self.msg_handler, sender: self.id, clipped: 0, emitter: self.id });
            }
        } else if !visible && self.iflags & IF_EMITTER_CLIPPED == 0 {
            self.iflags |= IF_EMITTER_CLIPPED;
            if self.msg_handler != 0xffff_ffff {
                notes.push(PLNotification { msg: 0x1a, handler: self.msg_handler, sender: self.id, clipped: 1, emitter: self.id });
            }
            if self.flags & 0x100 != 0 {
                self.ReleaseParticlesFromList(pool);
            }
        }
    }

    /// PLEmitter_BC::Initialize (0x0020e150) from a 0x98-byte PARTEMI record: +0 name[0x20],
    /// +0x20 type name[0x20], +0x40 started, +0x44 continuous, +0x48 has box, +0x4c box min,
    /// +0x58 box max, +0x64 has plane 1, +0x68 plane 1 {n, d}, +0x78 bounce 1, +0x7c has plane 2,
    /// +0x80 plane 2, +0x90 bounce 2, +0x94 flags. Returns the index of the type in `types`.
    pub fn Initialize(&mut self, rec: &[u8], size: u32, object_uid: Option<u32>, types: &[PLType]) -> Result<usize, i32> {
        if size != 0x98 || rec.len() < 0x98 {
            return Err(-1);
        }
        let u = |o: usize| u32::from_le_bytes(rec[o..o + 4].try_into().unwrap());
        let f = |o: usize| f32::from_bits(u(o));
        let s = |o: usize| {
            let b = &rec[o..o + 0x20];
            let n = b.iter().position(|&c| c == 0).unwrap_or(0x20);
            String::from_utf8_lossy(&b[..n]).into_owned()
        };
        self.name = s(0);
        self.object_uid = object_uid.unwrap_or(0);
        let tn = s(0x20);
        let Some(ti) = types.iter().position(|t| strncmp_eq(&t.d.name, &tn, 0x1f)) else { return Err(-1) };
        self.flags = u(0x94);
        if u(0x48) != 0 {
            self.SetBox([f(0x4c), f(0x50), f(0x54)], [f(0x58), f(0x5c), f(0x60)]);
        }
        if u(0x64) != 0 {
            self.planes[0] = [f(0x68), f(0x6c), f(0x70), f(0x74)];
            self.bounce[0] = f(0x78);
            self.iflags |= 1;
        }
        if u(0x7c) != 0 {
            self.planes[1] = [f(0x80), f(0x84), f(0x88), f(0x8c)];
            self.bounce[1] = f(0x90);
            self.iflags |= 2;
        }
        self.continuous = u(0x44) != 0;
        if u(0x40) == 0 {
            self.started = false;
            self.flags |= 0x10;
        } else {
            self.started = true;
            self.flags &= !0x10;
        }
        Ok(ti)
    }

    /// PLEmitter::Render (0x00238f70) -> Render2dParticles / Render3dParticles, emitting a batch.
    /// None when the type has no texture (the original counts an assert and returns) or no particle.
    pub fn Render(&mut self, ty: &PLTypeData, pool: &PLParticlePool) -> Option<PLDrawBatch> {
        let tex = ty.texture.clone()?;
        if self.particles.is_empty() {
            return None;
        }
        let prims = if ty.flags & 1 == 0 { PLPrims::Sprites(self.Render2dParticles(ty, pool)) } else { PLPrims::Quads(self.Render3dParticles(ty, pool)) };
        let blend = if ty.flags & 0x80 != 0 {
            0xa1
        } else if ty.flags & 0x200 != 0 {
            0x44
        } else {
            0x29
        };
        Some(PLDrawBatch { emitter: self.id, texture: tex, blend, stencil_read: ty.flags & 0x100 != 0, prims })
    }

    /// PLEmitter::Render2dParticles (0x002399e0) + RenderBatch (0x00239650): the 50-sprite batching
    /// is a DMA detail and is not reproduced (one list per emitter).
    pub fn Render2dParticles(&mut self, ty: &PLTypeData, pool: &PLParticlePool) -> Vec<PLSprite> {
        self.nb_rendered = 0;
        let e = self.ltm().pos;
        let mut out = Vec::with_capacity(self.particles.len());
        for &pi in &self.particles {
            let p = &pool.parts[pi];
            let pos = if self.flags & 2 == 0 { p.pos } else { [p.pos[0] + e[0], p.pos[1] + e[1], p.pos[2] + e[2]] };
            let a = p.age;
            let rgba = [f2b(ty.color[0].Get(a) * 120.0), f2b(ty.color[1].Get(a) * 120.0), f2b(ty.color[2].Get(a) * 120.0), f2b(ty.color[3].Get(a) * 120.0)];
            let rot = f2b(ty.rotation.Get(a) * 128.0) & 0x7f;
            out.push(PLSprite { pos, rgba, rot_index: rot, size: ty.scale.Get(a) * 0.75 });
        }
        self.nb_rendered = out.len() as u32;
        out
    }

    /// PLEmitter::Render3dParticles (0x00239040): one diamond quad per particle with a matrix:
    /// verts pos+right*s, pos+up*s, pos-right*s, pos-up*s (s = scale/2), uv (0,1),(0,0),(1,0),(1,1);
    /// emitter-space particles (flag 2) are transformed by the emitter LTM.
    pub fn Render3dParticles(&self, ty: &PLTypeData, pool: &PLParticlePool) -> Vec<PLQuad> {
        let ltm = self.ltm();
        let mut out = Vec::new();
        for &pi in &self.particles {
            let p = &pool.parts[pi];
            let Some(m) = p.matrix else { continue };
            let a = p.age;
            let s = ty.scale.Get(a) * 0.5;
            let r = [m.right[0] * s, m.right[1] * s, m.right[2] * s];
            let u = [m.up[0] * s, m.up[1] * s, m.up[2] * s];
            let o = m.pos;
            let mut verts = [
                [r[0] + o[0], r[1] + o[1], r[2] + o[2]],
                [u[0] + o[0], u[1] + o[1], u[2] + o[2]],
                [o[0] - r[0], o[1] - r[1], o[2] - r[2]],
                [o[0] - u[0], o[1] - u[1], o[2] - u[2]],
            ];
            if self.flags & 2 != 0 {
                verts.iter_mut().for_each(|v| *v = mat_point(&ltm, *v));
            }
            let rgba = [f2b(ty.color[0].Get(a) * 120.0), f2b(ty.color[1].Get(a) * 120.0), f2b(ty.color[2].Get(a) * 120.0), 0x78];
            out.push(PLQuad { verts, uvs: [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]], rgba });
        }
        out
    }
}

/// strncmp(a, b, n) == 0 on C strings.
pub(crate) fn strncmp_eq(a: &str, b: &str, n: usize) -> bool {
    let a = &a.as_bytes()[..a.len().min(n)];
    let b = &b.as_bytes()[..b.len().min(n)];
    a == b
}

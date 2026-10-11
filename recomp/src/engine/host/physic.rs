//! WP-S2: APIphysic host bridge (EFApi_Physic.cpp) for the functions the runtime still stubbed.
//! The manager logic is the 1:1 port in crate::engine::physic::api; this file only moves the
//! guest-memory arguments/results exactly as the C++ wrappers do. `uid` is already SELF-resolved.
use super::dtmath::quartic_solve;
use crate::engine::physic::api as papi;
use crate::engine::physic::mgr::EFPhysicMgr;
use crate::engine::physic::module::PhysicWorld;
use crate::engine::runtime::j2::J2;
use crate::engine::runtime::SharedParticles;
use crate::engine::{Mem, Ptr};

fn wv3(m: &mut Mem, a: Ptr, v: [f32; 3]) {
    for (i, x) in v.iter().enumerate() {
        m.w32(a + 4 * i as u32, x.to_bits());
    }
}

/// GetFloorAttribs(uid, EnvPrimAttribs*) (0x001f6b..): no object/physic -> 0, `out` untouched;
/// else the 0x20-byte floor material record (zeroed without a floor) and
/// `state&1 && !(state&4)`.
pub fn GetFloorAttribs(p: &SharedParticles, m: &mut Mem, uid: u32, out: Ptr) -> u32 {
    let Some((r, a)) = J2(p).physic(|pm, _| papi::GetFloorAttribs(pm, uid)) else { return 0 };
    for (i, w) in a.iter().enumerate() {
        m.w32(out + 4 * i as u32, *w);
    }
    r
}
/// SetSpherePrimaryVolume(uid, r, x, y, z) (0x001f5a20), void
pub fn SetSpherePrimaryVolume(p: &SharedParticles, uid: u32, r: f32, x: f32, y: f32, z: f32) -> u32 {
    J2(p).physic(|pm, _| papi::SetSpherePrimaryVolume(pm, uid, r, x, y, z));
    0
}
/// JumpOnCoor(uid, JumpOnCoorParam*) (0x001f7940), void; the param block is 10 guest words.
pub fn JumpOnCoor(p: &SharedParticles, m: &mut Mem, uid: u32, prm: Ptr) -> u32 {
    let w: Vec<u32> = (0..10).map(|i| m.r32(prm + 4 * i)).collect();
    J2(p).physic(|pm, pw| jump_on_coor(pm, uid, &w, pw));
    0
}

/// 1:1 of the C++ body incl. flags 4/8 (DTQuartic time solves) that physic::api::JumpOnCoor
/// leaves out. +0 target, +0xc target velocity, +0x18 max speed, +0x1c flags, +0x20 apex height,
/// +0x24 DM handle whose +0x30 vector is the gravity (DMGetParams words 0..3; invalid handle:
/// the C++ reads low memory, gravity 0 here).
fn jump_on_coor(m: &mut EFPhysicMgr, uid: u32, prm: &[u32], w: &dyn PhysicWorld) {
    let f = |i: usize| f32::from_bits(prm[i]);
    let id = m.PhysicOf(uid);
    let Some(pos) = m.Physic(id).map(|p| p.frame.pos) else { return };
    let g = papi::DMGetParams(m, prm[9], w).filter(|v| v.len() >= 3).map_or([0.0; 3], |v| [0, 1, 2].map(|i| f32::from_bits(v[i])));
    let (gx, gy, gz) = (g[0], g[1], g[2]);
    let (dx, dy, dz) = (f(0) - pos[0], f(1) - pos[1], f(2) - pos[2]);
    let (vx, vy, vz) = (f(3), f(4), f(5));
    let flags = prm[7];
    let mut t0 = 0.01f32;
    if flags & 2 != 0 {
        let l = -(gx * gx + gy * gy + gz * gz).sqrt();
        let k = 1.0 / l;
        let (nx, ny, nz) = (gx * k, gy * k, gz * k);
        let h = dz * nz + dx * nx + dy * ny;
        let vt = vz * nz + vx * nx + vy * ny;
        let v0 = (l * -2.0 * f(8)).sqrt();
        let hh = h + f(8);
        t0 = v0;
        if 0.0 < hh {
            t0 = vt + (l * -2.0 * hh).sqrt();
            if t0 <= v0 {
                t0 = v0;
            }
        }
        t0 -= vt;
        t0 = (t0 + (t0 * t0 + h * 2.0 * l).sqrt()) / -l;
    }
    let mut t = t0;
    if flags & 4 != 0 {
        // smallest root >= t0 (10000 without one)
        let q = [(gz * gz + gx * gx + gy * gy) * 0.5, -(vz * gz + vx * gx + vy * gy), 0.0, (dz * vz + dx * vx + dy * vy) * -2.0, (dz * dz + dx * dx + dy * dy) * -2.0];
        t = 10000.0;
        for r in quartic_solve(q) {
            if t0 <= r && r < t {
                t = r;
            }
        }
    }
    if flags & 8 != 0 {
        // smallest positive root (overrides flag 4)
        let mx = f(6);
        let q = [
            (gz * gz + gx * gx + gy * gy) * 0.25,
            -(vz * gz + vx * gx + vy * gy),
            ((vz * vz + vx * vx + vy * vy) - (dz * gz + dx * gx + dy * gy)) - mx * mx,
            (dz * vz + dx * vx + dy * vy) * 2.0,
            dz * dz + dx * dx + dy * dy,
        ];
        t = 10000.0;
        for r in quartic_solve(q) {
            if 0.0 < r && r < t {
                t = r;
            }
        }
    }
    let ht = t * 0.5;
    let it = 1.0 / t;
    let mut v = [(dx * it + vx) - gx * ht, (dy * it + vy) - gy * ht, (dz * it + vz) - gz * ht];
    if flags & 1 != 0 {
        let mx = f(6);
        let l2 = v[2] * v[2] + v[0] * v[0] + v[1] * v[1];
        if mx * mx < l2 {
            let k = mx / l2.sqrt();
            v = [v[0] * k, v[1] * k, v[2] * k];
        }
    }
    if uid != 0 {
        if let Some(p) = m.PhysicMut(id) {
            p.velocity = v;
            p.falling_timer = p.falling_delay;
        }
    }
}
/// PrimSetIntAttribBits / PrimResetIntAttribBits / PrimSetIntAttribValue / PrimUnlockIntAttrib (void)
pub fn PrimIntAttrib(p: &SharedParticles, op: u32, uid: u32, tag: u32, i: u32, v: u32) -> u32 {
    J2(p).physic(|pm, _| match op {
        0 => papi::PrimSetIntAttribBits(pm, uid, tag, i, v),
        1 => papi::PrimResetIntAttribBits(pm, uid, tag, i, v),
        2 => papi::PrimSetIntAttribValue(pm, uid, tag, i, v),
        _ => papi::PrimUnlockIntAttrib(pm, uid, tag, i, v),
    });
    0
}
/// PrimGetIntAttrib(uid, tag, index) (0x001f7060)
pub fn PrimGetIntAttrib(p: &SharedParticles, uid: u32, tag: u32, i: u32) -> u32 {
    J2(p).physic(|pm, _| papi::PrimGetIntAttrib(pm, uid, tag, i))
}
/// PrimGetOffsetFromJoint(uid, tag, DTVector3d*) (0x001f7410) -> 0 / 0xffffffff (out untouched)
pub fn PrimGetOffsetFromJoint(p: &SharedParticles, m: &mut Mem, uid: u32, tag: u32, out: Ptr) -> u32 {
    match J2(p).physic(|pm, _| papi::PrimGetOffsetFromJoint(pm, uid, tag)) {
        Some(v) => {
            wv3(m, out, v);
            0
        }
        None => 0xffff_ffff,
    }
}
/// PrimSetOffsetFromJoint(uid, tag, const DTVector3d*) (0x001f74f0) -> 0 / 0xffffffff
pub fn PrimSetOffsetFromJoint(p: &SharedParticles, m: &mut Mem, uid: u32, tag: u32, v: Ptr) -> u32 {
    let o = m.rv3(v);
    J2(p).physic(|pm, _| papi::PrimSetOffsetFromJoint(pm, uid, tag, o))
}

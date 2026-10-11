//! WP-S3: APIextra host side (EFApi_Extra.cpp 0x001ec880..0x001ed27f) for the functions the
//! runtime still stubbed. Waypoints read the `HostGame::waypoints` copy of DTWpGrpMgr 0x3441c0.
#![allow(non_snake_case)]

use super::game::HostGame;
use crate::engine::{Mem, Ptr};

/// WaypointGetPos 0x001ecbe0 (grp uid, index, DTVector3d *out): copies POINTS[index] when the
/// group exists (no index check, like the C++: out of range reads 0 here); returns `out`.
pub fn WaypointGetPos(g: &HostGame, m: &mut Mem, grp: u32, i: u32, out: Ptr) -> Ptr {
    if let Some(w) = g.waypoints.GetWaypointGrp(grp) {
        let o = i as usize * 3;
        for k in 0..3 {
            m.w32(out + 4 * k as u32, w.points.get(o + k).copied().unwrap_or(0.0).to_bits());
        }
    }
    out
}
/// WaypointSetPos 0x001ecc60: POINTS[index] = *in when the group exists; returns `in`.
/// (quirk kept: the C++ writes past the end for a bad index; ignored here)
pub fn WaypointSetPos(g: &mut HostGame, m: &Mem, grp: u32, i: u32, v: Ptr) -> Ptr {
    if let Some(w) = g.waypoints.groups.get_mut(&grp) {
        let o = i as usize * 3;
        for k in 0..3 {
            if let Some(p) = w.points.get_mut(o + k) {
                *p = m.rf(v + 4 * k as u32);
            }
        }
    }
    v
}
/// WaypointGetNb 0x001ecce0: NB_WAYPOINT (+0x14), 0 without the group
pub fn WaypointGetNb(g: &HostGame, grp: u32) -> u32 {
    g.waypoints.GetWaypointGrp(grp).map_or(0, |w| w.nb_waypoint)
}
/// DemoLoopIsPlaying 0x001ece30: EFDemoLoop state (+0x70 byte) in 1..=4. The port has no demo
/// loop recorder/player (EFDemoLoop is unported): state 0 = idle -> false.
pub fn DemoLoopIsPlaying() -> u8 {
    0
}

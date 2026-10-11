//! FloorDetect (Classes/Other/FloorDetect.cpp): the 0x24-byte floor probe record EFDMClimb keeps
//! five of (EFDMClimb+0xc0, stride 0x24) and fills via EFCollisionStage::FindWeightedAvgNormal.
//! Plus `FindFloor`, the downward ray against the level EFCollisionMap used for the hero-spawn
//! test (same RayIntersect call the DM modules make: nearest hit, t initialised to 1).
#![allow(non_snake_case)]

use super::collision_map::*;

pub type V3 = [f32; 3];

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FloorDetect {
    /// +0x00 lateral offset along the frame's at axis (EFDMClimb: LTM.up*h + LTM.at*offset + pos)
    pub offset: f32,
    /// +0x04 probe position
    pub pos: V3,
    /// +0x10 found (FindWeightedAvgNormal result)
    pub found: u32,
    /// +0x14 averaged floor normal
    pub normal: V3,
    /// +0x20 weight
    pub weight: f32,
}

impl FloorDetect {
    /// FloorDetect(): zeroes +4..+0xc and +0x14..+0x1c (offset/found/weight left as constructed).
    pub fn new() -> Self {
        Self::default()
    }
}

/// Casts a segment from `from` down by `depth` against the map (mask words: all bits).
/// Returns (hit position, normal, primitive) of the nearest hit.
pub fn FindFloor(map: &mut EFCollisionMap, from: V3, depth: f32, stamp: i32) -> Option<(V3, V3, usize)> {
    let rin = RayIntersectIn { origin: from, dir: [0.0, -depth, 0.0], mask1: !0, mask2: !0 };
    let mut out = EFRayIntersectOut::default();
    let mut t = 1.0f32;
    if map.RayIntersect(&rin, &mut out, &mut t, stamp) != 2 {
        return None;
    }
    let p = [from[0], from[1] - depth * t, from[2]];
    Some((p, out.hit.normal, out.hit.prim?))
}

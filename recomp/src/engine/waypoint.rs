//! DTWpGrpMgr (Classes/DT/DTWpGrpMgr.cpp, global at 0x3441c0) + DTWaypointGrp runtime helpers
//! (Classes/DT/DTWaypointGrp.cpp). The DTWaypointGrp descriptor itself (class 0x30, members UID,
//! PUID, REFPUID, NB_WAYPOINT, POINTS) is parsed by dt::descriptors.

#![allow(non_snake_case)]

use crate::engine::dt::descriptors::DTWaypointGrp;
use crate::engine::pack::EFRessourcesMgr;
use std::collections::BTreeMap;

/// DTWaypointGrp::Initialize (0x00100fa0): NB_WAYPOINT = n, POINTS resized to 3n.
pub fn DTWaypointGrp_Initialize(g: &mut DTWaypointGrp, n: u32) {
    g.nb_waypoint = n;
    g.points.resize(n as usize * 3, 0.0);
}

/// DTWaypointGrp::Copy (0x00101230): count + the 3n floats (UID/PUID not copied).
pub fn DTWaypointGrp_Copy(dst: &mut DTWaypointGrp, src: &DTWaypointGrp) {
    DTWaypointGrp_Initialize(dst, src.nb_waypoint);
    for i in 0..(src.nb_waypoint as usize * 3) {
        dst.points[i] = src.points.get(i).copied().unwrap_or(0.0);
    }
}

/// Waypoint i of a group (POINTS xyz triples).
pub fn DTWaypointGrp_GetPoint(g: &DTWaypointGrp, i: u32) -> Option<[f32; 3]> {
    let o = i as usize * 3;
    g.points.get(o..o + 3).map(|p| [p[0], p[1], p[2]])
}

/// DTWpGrpMgr: DTMap<uint, DTWaypointGrp*> (37 buckets in the C++; a map here).
#[derive(Debug, Clone, Default)]
pub struct DTWpGrpMgr {
    pub groups: BTreeMap<u32, DTWaypointGrp>,
}

impl DTWpGrpMgr {
    /// DTWpGrpMgr::AddWaypointGrp (0x00101500): DTMap::SetAt(uid, grp) — a later group with
    /// the same UID replaces the earlier one (quirk: the replaced group leaks in the C++).
    pub fn AddWaypointGrp(&mut self, g: DTWaypointGrp) {
        self.groups.insert(g.uid, g);
    }
    /// DTWpGrpMgr::GetWaypointGrp (0x00101630). quirk: the C++ dereferences the Find result
    /// without a NULL check (crash on unknown uid); None here.
    pub fn GetWaypointGrp(&self, uid: u32) -> Option<&DTWaypointGrp> {
        self.groups.get(&uid)
    }
    /// DTWpGrpMgr::RemoveAllWaypointGrp (0x00101660)
    pub fn RemoveAllWaypointGrp(&mut self) {
        self.groups.clear();
    }
    /// Loading-stream hook: DTWaypointGrp::LoadFromStream (0x00101190) registers every loaded
    /// group with the manager; the pack reader keeps the parsed 0x30 objects, this replays the
    /// registrations in pack order. Returns the number of groups added.
    pub fn LoadFromLevel(&mut self, res: &mut EFRessourcesMgr) -> u32 {
        let mut n = 0;
        for o in res.objects.iter_mut() {
            let Some(d) = o.as_dt() else { continue };
            if let Some(g) = d.as_any().downcast_ref::<DTWaypointGrp>() {
                self.AddWaypointGrp(g.clone());
                n += 1;
            }
        }
        n
    }
}

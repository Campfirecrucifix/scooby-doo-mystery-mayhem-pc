//! WP-E5: collision prim tests (sphere / swept / bbox / polygon), EFCollisionSystem, real EP1L01 map.
#![allow(non_snake_case)]
mod sphere { include!("e5/e5_sphere.rs"); }
mod swept { include!("e5/e5_swept.rs"); }
mod bbox { include!("e5/e5_bbox.rs"); }
mod system { include!("e5/e5_system.rs"); }

mod real_map {
    use scooby_recomp::engine::iso::IsoFs;
    use scooby_recomp::engine::pack::*;
    use scooby_recomp::engine::physic::{collision_map::*, floor::*, model::*, objcoll_check::*, objcoll_prim::*};
    use crate::common::ISO;

    /// Hero-sized sphere dropped onto the real EP1L01 floor: the env prims gathered by
    /// GetPrimitivesInSphere stop the downward motion at the floor (t < 1, normal ~ +Y).
    #[test]
    fn sphere_lands_on_ep1l01_floor() {
        let Ok(fs) = IsoFs::open(ISO) else { eprintln!("ISO missing, skipped"); return };
        let mut m = EFRessourcesMgr::default();
        m.level_file_name = Some("EP1L01.P2R".into());
        assert_eq!(m.LoadLevel(&fs, "", "EP1/L01", "EN", &mut RawInstanciator, |_, _| {}).unwrap(), 0);
        let mut map = m.objects.iter().filter_map(|o| o.as_raw()).find(|r| r.class_id == 0x24)
            .map(|r| EFCollisionMap::LoadFromRaw(&r.data).unwrap().0).expect("collision map");
        let mut found = None;
        for (k, g) in m.game_objects.iter().enumerate() {
            let p = g.glo.pos;
            if p == [0.0; 3] { continue; }
            if let Some((hp, n, _)) = FindFloor(&mut map, [p[0], p[1] + 1.0, p[2]], 20.0, 1 + k as i32) {
                if n[1] > 0.9 { found = Some(hp); break; }
            }
        }
        let floor = found.expect("floor");
        let r = 0.5f32;
        let c = [floor[0], floor[1] + r + 0.5, floor[2]];
        let mut cm = EFCollisionModel::new(1);
        let mut l = IDENTITY;
        l[3] = [c[0], c[1], c[2], 1.0];
        let s = cm.AddPrimitive(0, 1, PRIM_SPHERE, l).unwrap();
        s.base.radius = r;
        s.PrecalcPrimData();
        let s = s.clone();
        let mut d = EFGetPrimitivesInSphereData { center: c, radius: 2.0, mask1: !0, mask2: !0, out: vec![], capacity: 256 };
        map.GetPrimitivesInSphere(&mut d, 0x7000);
        assert!(!d.out.is_empty());
        let i = EFCheckCollisionIn { tmax: 1.0, motion: [0.0, -1.0, 0.0] };
        let mut best: Option<EFCheckCollisionOut> = None;
        for &p in &d.out {
            let e = EnvPrim::from_map(&map, p);
            let mut o = EFCheckCollisionOut::default();
            if s.CheckEnvCollision(&i, &mut o, &e) != 0 && best.map_or(true, |b| o.t < b.t) { best = Some(o); }
        }
        let b = best.expect("sphere hits the floor");
        eprintln!("hit t {} n {:?} depth {}", b.t, b.normal, b.depth);
        assert!(b.t > 0.3 && b.t < 0.7, "t {}", b.t);
        assert!(b.normal[1] > 0.9);
    }
}

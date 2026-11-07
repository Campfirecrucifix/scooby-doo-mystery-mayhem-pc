//! WP-E3: collision system tests (ISO tests skip when the ISO is absent).
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::logic_link::LogicCodeEntry;
use scooby_recomp::engine::pack::*;
use scooby_recomp::engine::physic::{collision_map::*, floor::*, model::*, objcoll_prim::*, prim_anim::*};

use crate::common::ISO;

#[test]
fn sphere_sphere_and_vector() {
    let i = EFCheckSphereSphereCollisionIn { center: [0.0, 5.0, 0.0], motion: [0.0, -10.0, 0.0], radius: 1.0 };
    let mut o = EFCheckSphereSphereCollisionOut::default();
    assert_eq!(EFObjCollisionPrim::CheckSphereSphereCollision(&i, &mut o), 1);
    assert!((o.t - 0.4).abs() < 1e-6 && (o.normal[1] - 1.0).abs() < 1e-6);
    let i = EFCheckSphereVectorCollisionIn { vec: [10.0, 0.0, 0.0], center: [5.0, 3.0, 0.0], motion: [0.0, -4.0, 0.0], radius: 1.0 };
    let mut o = EFCheckSphereVectorCollisionOut::default();
    assert_eq!(EFObjCollisionPrim::CheckSphereVectorCollision(&i, &mut o), 0x101);
    assert!((o.t - 0.5).abs() < 1e-6);
    let i = EFVectorVectorNearestPointIn { a: [1.0, 0.0, 0.0], b: [0.0, 0.0, 1.0], off: [0.5, 2.0, -0.5], maxdist: 10.0 };
    let mut o = EFVectorVectorNearestPointOut::default();
    let r = EFObjCollisionPrim::GetVectorVectorNearestPoint(&i, &mut o);
    assert_eq!(r, 0x2100 | 0x400);
    assert!((o.dist - 2.0).abs() < 1e-6);
}

#[test]
fn model_tags_and_translate() {
    let mut m = EFCollisionModel::new(3);
    m.AddPrimitive(1, 0x55, 1, IDENTITY).unwrap();
    m.AddPrimitive(2, 0x66, 0, IDENTITY).unwrap();
    assert_eq!(m.PrimTagToPrimIndex(0x66), 2);
    assert_eq!(m.PrimTagToPrimIndex(7), 0xffff_ffff);
    m.PrecalcData(&IDENTITY);
    assert!(m.Translate([1.0, 0.0, 0.0]));
    assert!(!m.Translate([0.0, 0.0, 0.0]));
    assert_eq!(m.prims[2].as_ref().unwrap().base.center, [1.0, 0.0, 0.0]);
    let p = m.prims[1].as_ref().unwrap();
    assert!((p.base.radius - 0.1).abs() < 1e-6);
}

#[test]
fn prim_anim_roundtrip() {
    let mut d = vec![0u8; 4];
    d.extend(2i16.to_le_bytes());
    d.extend(1.5f32.to_le_bytes());
    d.extend(7u16.to_le_bytes());
    d.extend(4u16.to_le_bytes());
    for v in [1.0f32, 2.0, 3.0] { d.extend(v.to_le_bytes()); }
    d.extend(0f32.to_le_bytes());
    d.extend(0u16.to_le_bytes());
    d.extend(9u16.to_le_bytes());
    let (a, n) = EFCollisionPrimAnim::LoadFromRaw(&d).unwrap();
    assert_eq!(n, d.len());
    assert_eq!(a.cmds[0].payload.as_ref().unwrap(), &vec![2f32.to_bits(), 1f32.to_bits(), 3f32.to_bits()]);
    assert!(a.cmds[1].payload.is_none());
}

#[test]
fn real_prim_anims_parse() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let mut n = 0;
    for p in fs.names().filter(|n| n.to_uppercase().ends_with(".P2R")).map(|s| s.to_string()).collect::<Vec<_>>() {
        let b = fs.read(&p).unwrap();
        let u = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        let cnt = u(4) as usize;
        let mut off = 8 + 16 * cnt;
        for i in 0..cnt {
            let (sz, cls) = (u(8 + 16 * i + 4) as usize, u(8 + 16 * i + 12));
            if cls == 0x26 {
                let (_, used) = EFCollisionPrimAnim::LoadFromRaw(&b[off..off + sz]).unwrap_or_else(|e| panic!("{p}: {e}"));
                assert_eq!(used, sz, "{p}");
                n += 1;
            }
            off += sz;
        }
    }
    eprintln!("{n} prim anims");
}

#[test]
fn hero_spawn_floor() {
    let Ok(fs) = IsoFs::open(ISO) else { eprintln!("ISO missing, skipped"); return };
    let mut m = EFRessourcesMgr::default();
    m.level_file_name = Some("EP1L01.P2R".into());
    assert_eq!(m.LoadLevel(&fs, "", "EP1/L01", "EN", &mut RawInstanciator, |_, _| {}).unwrap(), 0);
    let mut map = None;
    let mut hero_codes = Vec::new();
    for o in &m.objects {
        let Some(r) = o.as_raw() else { continue };
        if r.class_id == 0x24 {
            map = Some(EFCollisionMap::LoadFromRaw(&r.data).unwrap().0);
        } else if let Some(e) = LogicCodeEntry::parse(r.class_id, &r.data) {
            if e.base_name().starts_with("herospot") || e.base_name().starts_with("chkpnt") {
                hero_codes.push(e.uid);
            }
        }
    }
    let mut map = map.expect("collision map");
    let heroes: Vec<_> = m.game_objects.iter().filter(|g| hero_codes.contains(&g.glo.glo_file_id) && g.glo.pos != [0.0; 3] /* herospot manager sits at the origin */).collect();
    assert!(!heroes.is_empty(), "no hero descriptor (codes {hero_codes:x?})");
    for (k, h) in heroes.iter().enumerate() {
        let pos = h.glo.pos;
        if pos[1].abs() < 1e-6 && pos[0] == 0.0 { continue; }
        let hit = FindFloor(&mut map, [pos[0], pos[1] + 1.0, pos[2]], 20.0, 1000 + k as i32);
        eprintln!("hero {:x} at {pos:?} -> {hit:?}", h.uid);
        let (p, n, _) = hit.expect("hero spawn has no floor below");
        assert!(n[1] > 0.5 && (pos[1] - p[1]).abs() < 3.0);
    }
}

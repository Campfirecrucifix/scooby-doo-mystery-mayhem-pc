include!("edge_common.rs");
use scooby_recomp::engine::physic::collision_map::EFCollisionMap;
use scooby_recomp::engine::physic::dm::edge_detect::*;

fn module(p: &mut EFPhysic, flags: u32) -> EFDMEdgeDetect {
    let mut m = EFDMEdgeDetect::new();
    with(p, 0, &NoWorld, |c| m.SetParams(c, Some(&[0x55, 7, fw(0.5), flags])));
    m
}

#[test]
fn edge_detect_env_edges() {
    let mut p = owner([0.5, 0.3, -0.1], [0.0; 3], 1.0);
    let mut m = module(&mut p, 4);
    assert_eq!((m.radius, m.prim, m.user, m.target), (0.5, 0, 0x55, 5));
    let mut out = [0u32; 4];
    with(&mut p, 0, &NoWorld, |c| m.GetParams(c, &mut out));
    assert_eq!(out, [0x55, 7, fw(0.5), 4]);
    let env = [ledge()];
    with(&mut p, 1, &NoWorld, |c| m.EdgeDetect(c, &env, &[]));
    let n = m.last.take().expect("notify");
    assert_eq!((n.target, n.tag, n.count), (5, 7, 1));
    assert!(close(n.edges[0].v0, [0.0; 3]) && close(n.edges[0].v1, [1.0, 0.0, 0.0]) && close(n.edges[0].normal, [0.0, 1.0, 0.0]));
    // unchanged edges: no notify
    with(&mut p, 2, &NoWorld, |c| m.EdgeDetect(c, &env, &[]));
    assert!(m.last.is_none());
    // flag 1: radius from the prim (1.0) -> two more edges reach (far edge z=1 is 1.14 away)
    let mut m = module(&mut p, 5);
    assert_eq!(m.radius, 1.0);
    with(&mut p, 1, &NoWorld, |c| m.EdgeDetect(c, &env, &[]));
    assert_eq!(m.count, 3);
    // masks: material without matching bits is ignored
    let mut e2 = ledge();
    e2.mat[1] = 2;
    let mut m = module(&mut p, 4);
    with(&mut p, 1, &NoWorld, |c| m.EdgeDetect(c, &[e2], &[]));
    assert!(m.last.is_none() && m.count == 0);
}

#[test]
fn edge_detect_object_polygon() {
    let mut p = owner([0.5, 0.3, -0.1], [0.0; 3], 1.0);
    let mut m = module(&mut p, 8);
    // another physic with a polygon prim (flag 0x200), translated by +10 x
    let mut other = EFCollisionModel::new(1);
    let mut mtx = IDENTITY;
    mtx[3] = [10.0, 0.0, 0.0, 1.0];
    let mut inv = IDENTITY;
    inv[3] = [-10.0, 0.0, 0.0, 1.0];
    let pr = other.AddPrimitive(0, 1, PRIM_POLYGON, IDENTITY).unwrap();
    pr.base.set_word(0x10, 0x200);
    pr.ext = PrimExt::Polygon { m: mtx, inv, prims: vec![ledge()] };
    // owner at x=10.5 -> local x=0.5
    let mut p = owner([10.5, 0.3, -0.1], [0.0; 3], 1.0);
    with(&mut p, 0, &NoWorld, |c| m.SetParams(c, Some(&[0, 7, fw(0.5), 8])));
    with(&mut p, 1, &NoWorld, |c| m.EdgeDetect(c, &[], &[&other]));
    let n = m.last.take().expect("notify");
    assert_eq!(n.count, 1);
    assert!(close(n.edges[0].v0, [10.0, 0.0, 0.0]) && close(n.edges[0].v1, [11.0, 0.0, 0.0]));
}

/// Real EP1L01 collision map: an owner placed at a flagged edge's midpoint finds that edge.
#[test]
fn edge_detect_real_map() {
    use scooby_recomp::engine::iso::IsoFs;
    use scooby_recomp::engine::pack::*;
    let Ok(fs) = IsoFs::open("../Scooby Doo Mystery Mayhem.ISO") else { eprintln!("ISO missing, skipped"); return };
    let mut rm = EFRessourcesMgr::default();
    rm.level_file_name = Some("EP1L01.P2R".into());
    assert_eq!(rm.LoadLevel(&fs, "", "EP1/L01", "EN", &mut RawInstanciator, |_, _| {}).unwrap(), 0);
    let map = rm.objects.iter().filter_map(|o| o.as_raw()).find(|r| r.class_id == 0x24)
        .map(|r| EFCollisionMap::LoadFromRaw(&r.data).unwrap().0).expect("collision map");
    let e = map.Primitives().into_iter().map(|q| EnvPrim::from_map(&map, q))
        .find(|e| e.w4 & 1 != 0 && e.nverts >= 3 && e.nverts < 0x8000).expect("flagged prim");
    let mid = scale(add(e.verts[0], e.verts[1]), 0.5);
    let mut p = owner(mid, [0.0; 3], 0.2);
    let mut m = module(&mut p, 4);
    let mut env = e.clone();
    env.mat[1] = 1;
    with(&mut p, 1, &NoWorld, |c| m.EdgeDetect(c, &[env], &[]));
    let n = m.last.take().expect("notify");
    assert!(n.count >= 1);
    assert!(close(n.edges[0].v0, e.verts[0]) && close(n.edges[0].v1, e.verts[1]));
}

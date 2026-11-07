//! WP-E: physics + collision tests (real data skipped when the ISO is absent).
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::physic::collision_map::*;

const ISO: &str = "../Scooby Doo Mystery Mayhem.ISO";

fn u32le(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

/// FAT entries (uid, size, offset-from-data-base, class) + data base of a P2R pack.
fn fat(b: &[u8]) -> (Vec<[u32; 4]>, usize) {
    let n = u32le(b, 4) as usize;
    let v = (0..n).map(|i| [0, 1, 2, 3].map(|k| u32le(b, 8 + 16 * i + 4 * k))).collect();
    (v, 8 + 16 * n)
}

fn collision_entry(b: &[u8]) -> Option<&[u8]> {
    let (f, base) = fat(b);
    // entries are sequential: cumulative offsets
    let mut off = base;
    for e in &f {
        let s = e[1] as usize;
        if e[3] == 0x24 {
            return Some(&b[off..off + s]);
        }
        off += s;
    }
    None
}

#[test]
fn ep1l01_collision_map() {
    let Ok(fs) = IsoFs::open(ISO) else { eprintln!("ISO missing, skipped"); return };
    let b = fs.read("LEVEL/EP1/L01/GEN/EP1L01.P2R").unwrap();
    let e = collision_entry(&b).expect("no 0x24 entry");
    let (mut m, used) = EFCollisionMap::LoadFromRaw(e).unwrap();
    assert_eq!(used, e.len(), "must consume exactly the FAT size");
    let prims = m.Primitives();
    assert!(prims.len() > 100);
    eprintln!("prims {} root {:?}", prims.len(), &m.w[m.root..m.root + 7]);
    // a ray straight down through the middle of a primitive hits that floor
    let mut tested = 0;
    let mut stamp = 1;
    for &p in prims.iter() {
        let n = m.PrimNormal(p);
        if n[1] < 0.9 {
            continue;
        }
        let nv = m.PrimCount(p);
        // 0.77*centroid + 0.23*v0: off the octree cell boundaries, which often coincide with
        // polygon centroids (a ray exactly on a cell face is missed by the original's walk too)
        let v0 = m.PrimVertex(p, 0);
        let mut c = [v0[0] * 0.23, v0[1] * 0.23, v0[2] * 0.23];
        for i in 0..nv {
            let v = m.PrimVertex(p, i);
            for k in 0..3 {
                c[k] += v[k] / nv as f32 * 0.77;
            }
        }
        let rin = RayIntersectIn { origin: [c[0], c[1] + 1.0, c[2]], dir: [0.0, -2.0, 0.0], mask1: !0, mask2: !0 };
        let mut out = EFRayIntersectOut::default();
        let mut t = 2.0;
        stamp += 1;
        let r = m.RayIntersect(&rin, &mut out, &mut t, stamp);
        assert_eq!(r, 2, "prim {p} at {c:?}");
        let hy = rin.origin[1] + rin.dir[1] * t;
        assert!(hy >= c[1] - 0.01, "hit below the floor {hy} < {}", c[1]);
        tested += 1;
        if tested > 200 {
            break;
        }
    }
    assert!(tested > 10);
}

#[test]
fn all_levels_collision_maps_parse() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let packs: Vec<String> = fs.names().filter(|n| n.to_uppercase().ends_with(".P2R")).map(|s| s.to_string()).collect();
    let mut maps = 0;
    for p in &packs {
        let b = fs.read(p).unwrap();
        if let Some(e) = collision_entry(&b) {
            let (m, used) = EFCollisionMap::LoadFromRaw(e).unwrap_or_else(|er| panic!("{p}: {er}"));
            assert_eq!(used, e.len(), "{p}");
            assert!(!m.Primitives().is_empty());
            maps += 1;
        }
    }
    eprintln!("{maps} maps in {} packs", packs.len());
    assert!(maps > 20);
}


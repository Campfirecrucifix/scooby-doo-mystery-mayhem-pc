//! P25 R75 scratch: world sector vertices (pos, prelit RGBA, uv) in a box, to find the EP1/L01 light cone.
//! cargo run --example p25_r75_wall -- EP1/L01 x0,y0,z0,x1,y1,z1
use scooby_recomp::engine::{iso::IsoFs, gfx::EFStatic3dObj, runtime::EngineRuntime};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let lvl = a[1].clone();
    if a[2] == "res" { return res(&lvl, u32::from_str_radix(&a[3], 16).unwrap()); }
    let b: Vec<f32> = a[2].split(',').map(|s| s.parse().unwrap()).collect();
    let fs = IsoFs::open("../Scooby Doo Mystery Mayhem.ISO").expect("iso");
    let pack = EngineRuntime::ReadLevel(&fs, &lvl, &format!("{}.P2R", lvl.replace('/', ""))).unwrap();
    for o in &pack.objects {
        let Some(r) = o.as_raw() else { continue };
        if r.class_id != 0x18 { continue; }
        let mut s = EFStatic3dObj::default();
        s.LoadPropertiesBinary(&r.data).unwrap();
        let w = s.world.as_ref().unwrap();
        for (si, sec) in w.sectors.iter().enumerate() {
            for sm in &sec.meshes {
                let p = &sm.mesh.positions;
                let m = &w.materials[sm.material as usize];
                for (ti, t) in sm.indices.chunks(3).enumerate() {
                    if !t.iter().all(|&i| { let q = p[i as usize]; (0..3).all(|k| q[k] >= b[k] && q[k] <= b[k + 3]) }) { continue; }
                    let v: Vec<String> = t.iter().map(|&i| { let i = i as usize; let q = p[i];
                        format!("({:.2},{:.2},{:.2}) c{:?} uv{:?}", q[0], q[1], q[2], sm.mesh.colors.get(i), sm.mesh.uvs.get(i)) }).collect();
                    println!("sec {si} mat {} {:?} tri {ti}: {}", sm.material, m.texture.as_ref().map(|t| &t.name), v.join(" "));
                }
            }
        }
    }
}

/// `-- LVL res UIDHEX`: prelit/material colours and geometry flags of one EF3dObj resource.
fn res(lvl: &str, uid: u32) {
    use scooby_recomp::engine::gfx::EF3dObjRes;
    let fs = IsoFs::open("../Scooby Doo Mystery Mayhem.ISO").expect("iso");
    let pack = EngineRuntime::ReadLevel(&fs, lvl, &format!("{}.P2R", lvl.replace('/', ""))).unwrap();
    for o in &pack.objects {
        let Some(r) = o.as_raw() else { continue };
        if r.class_id != 0x5b { continue; }
        let mut x = EF3dObjRes::default();
        if x.LoadPropertiesBinary(&r.data).is_err() || x.res.uid != uid { continue; }
        let Some(c) = &x.clump else { continue };
        for (gi, g) in c.geometries.iter().enumerate() {
            let mut h = std::collections::BTreeMap::new();
            for p in &g.prelit { *h.entry(*p).or_insert(0) += 1; }
            println!("geom {gi} flags {:#x} prelit n {} distinct {}: {:?}", g.flags, g.prelit.len(), h.len(), h.iter().rev().take(6).collect::<Vec<_>>());
            for m in &g.materials { println!("  mat {:?} col {:?} ext {:?}", m.texture.as_ref().map(|t| &t.name), m.color, m.ext); }
        }
    }
}

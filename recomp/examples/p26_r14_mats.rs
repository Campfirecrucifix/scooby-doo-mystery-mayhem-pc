//! P26 R14 scratch: world sector meshes in a box: material texture, colour, prelit alpha range.
use scooby_recomp::engine::{iso::IsoFs, gfx::EFStatic3dObj, gfx::EF3dObjRes, runtime::EngineRuntime};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let lvl = a[1].clone();
    let b: Vec<f32> = a[2].split(',').map(|s| s.parse().unwrap()).collect();
    let fs = IsoFs::open("../Scooby Doo Mystery Mayhem.ISO").expect("iso");
    let pack = EngineRuntime::ReadLevel(&fs, &lvl, &format!("{}.P2R", lvl.replace('/', ""))).unwrap();
    for g in &pack.game_objects {
        let r = g.glo.object_descriptors.objects.iter().find_map(|d| d.as_any().downcast_ref::<scooby_recomp::engine::dt::descriptors::EFIDescriptor<scooby_recomp::engine::dt::descriptors::DT3dObjectDescriptor>>().map(|x| x.desc.puid as u32));
        if let Some(r) = r { println!("glo3d {:#x} res {:#x} pos {:?} latent {}", g.uid, r, g.glo.pos, g.IsLatent()); }
    }
    for o in &pack.objects {
        let Some(r) = o.as_raw() else { continue };
        if r.class_id == 0x18 {
            let mut s = EFStatic3dObj::default();
            s.LoadPropertiesBinary(&r.data).unwrap();
            let w = s.world.as_ref().unwrap();
            let mut used = vec![0usize; w.materials.len()];
            for sec in &w.sectors { for sm in &sec.meshes { if !sm.indices.is_empty() { used[sm.material as usize] += 1; } else { println!("EMPTY mesh mat {} npos {}", sm.material, sm.mesh.positions.len()); } } }
            for (i, m) in w.materials.iter().enumerate() { println!("wmat {i} used {} tex {:?} col {:?} mask {:?}", used[i], m.texture.as_ref().map(|t| &t.name), m.color, m.texture.as_ref().map(|t| &t.mask)); }
            println!("sectors {} tris {} verts {}", w.sectors.len(), w.num_triangles, w.num_vertices);
            println!("sum sector tris {}", w.sectors.iter().map(|s| s.num_triangles).sum::<u32>());
            println!("mesh tris {}", w.sectors.iter().flat_map(|s| s.meshes.iter()).map(|m| m.indices.len()/3).sum::<usize>());
            for (si, sec) in w.sectors.iter().enumerate() {
                for sm in &sec.meshes {
                    let p = &sm.mesh.positions;
                    let inb: Vec<usize> = (0..p.len()).filter(|&i| p[i][0] >= b[0] && p[i][0] <= b[3] && p[i][1] >= b[1] && p[i][1] <= b[4] && p[i][2] >= b[2] && p[i][2] <= b[5]).collect();
                    if inb.is_empty() { continue; }
                    let m = &w.materials[sm.material as usize];
                    let al: Vec<u8> = inb.iter().filter_map(|&i| sm.mesh.colors.get(i).map(|c| c[3])).collect();
                    let (mn, mx) = (al.iter().min().copied().unwrap_or(0), al.iter().max().copied().unwrap_or(0));
                    let mut lo = [1e9f32; 3]; let mut hi = [-1e9f32; 3];
                    for &i in &inb { for k in 0..3 { lo[k] = lo[k].min(p[i][k]); hi[k] = hi[k].max(p[i][k]); } }
                    println!("sec {si} mat {} tex {:?} col {:?} flags {:#x} n {} alpha {mn}..{mx} box {lo:?}-{hi:?} ext {:?}", sm.material, m.texture.as_ref().map(|t| &t.name), m.color, m.flags, inb.len(), m.ext);
                }
            }
        }
        if r.class_id == 0x5b && a.len() > 3 {
            let mut x = EF3dObjRes::default();
            if x.LoadPropertiesBinary(&r.data).is_ok() {
                if let Some(c) = &x.clump {
                    let t: Vec<String> = c.geometries.iter().flat_map(|g| g.materials.iter().map(|m| format!("{:?}/{:?}", m.texture.as_ref().map(|t| t.name.clone()), m.color))).collect();
                    for (ai, at) in c.atomics.iter().enumerate() { let gm = &c.geometries[at.geometry as usize]; println!("  res {:#x} atomic {ai} flags {} ap {:?} mats {:?} prelit {:?}", x.res.uid, at.flags, at.ap_flags, gm.materials.iter().map(|m| (m.texture.as_ref().map(|t| t.name.clone()), m.color)).collect::<Vec<_>>(), gm.prelit.iter().map(|c| c[3]).min()); }
                    for g in &c.geometries { if let Some(mt)=g.morph_targets.first() { let mut lo=[1e9f32;3]; let mut hi=[-1e9f32;3]; for v in &mt.verts { for k in 0..3 { lo[k]=lo[k].min(v[k]); hi[k]=hi[k].max(v[k]); } } println!("res {:#x} geom flags {:#x} nv {} box {lo:?}-{hi:?} prelit_a {:?}", x.res.uid, g.flags, mt.verts.len(), g.prelit.iter().map(|c| c[3]).collect::<std::collections::BTreeSet<_>>()); } }
                    println!("res {:#x} atomics {} flags {:?} mats {:?}", x.res.uid, c.atomics.len(), c.atomics.iter().map(|a| a.flags).collect::<Vec<_>>(), t);
                }
            }
        }
    }
}

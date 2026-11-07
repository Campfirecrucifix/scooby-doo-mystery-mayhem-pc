//! P25 R75 scratch: level static lights + prelit/material lighting coefficients of one EF3dObjRes.
//! cargo run --example p25_r75_lights -- EP2/L03A 0x7c8
use scooby_recomp::engine::{iso::IsoFs, gfx::EFStatic3dObj, gfx::EF3dObjRes, runtime::EngineRuntime};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let lvl = a[1].clone();
    let want = u32::from_str_radix(a[2].trim_start_matches("0x"), 16).unwrap();
    let fs = IsoFs::open("../Scooby Doo Mystery Mayhem.ISO").expect("iso");
    let pack = EngineRuntime::ReadLevel(&fs, &lvl, &format!("{}.P2R", lvl.replace('/', ""))).unwrap();
    for o in &pack.objects {
        let Some(r) = o.as_raw() else { continue };
        if r.class_id == 0x18 {
            let mut s = EFStatic3dObj::default();
            s.LoadPropertiesBinary(&r.data).unwrap();
            for l in &s.lights { println!("light type {:#x} flags {:#x} col {:?} radius {} at {:?}", l.light.light_type, l.light.flags, l.light.color, l.light.radius, &l.matrix[6..9]); }
        }
        if r.class_id == 0x5b {
            let mut x = EF3dObjRes::default();
            if x.LoadPropertiesBinary(&r.data).is_err() || x.res.uid as u32 & 0xffff != want & 0xffff { continue; }
            let Some(c) = &x.clump else { continue };
            for g in &c.geometries {
                let n = g.prelit.len().max(1) as f32;
                let mut avg = [0f32; 4]; let mut mx = [0u8; 4];
                for p in &g.prelit { for k in 0..4 { avg[k] += p[k] as f32 / n; mx[k] = mx[k].max(p[k]); } }
                println!("geom flags {:#x} prelit n {} avg {avg:?} max {mx:?} mats {:?}", g.flags, g.prelit.len(),
                    g.materials.iter().map(|m| (m.texture.as_ref().map(|t| t.name.clone()), m.color, m.ambient, m.diffuse)).collect::<Vec<_>>());
            }
        }
    }
}

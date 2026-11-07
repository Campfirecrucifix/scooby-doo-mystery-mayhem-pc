//! P26 R17 scratch: world format flags, static lights, sector prelit averages of a level.
//! cargo run --example p26_r17_world -- EP4/L03A
use scooby_recomp::engine::{iso::IsoFs, gfx::EFStatic3dObj, runtime::EngineRuntime};
fn main() {
    let lvl = std::env::args().nth(1).unwrap();
    let fs = IsoFs::open("../Scooby Doo Mystery Mayhem.ISO").expect("iso");
    let pack = EngineRuntime::ReadLevel(&fs, &lvl, &format!("{}.P2R", lvl.replace('/', ""))).unwrap();
    for o in &pack.objects {
        let Some(r) = o.as_raw() else { continue };
        if r.class_id != 0x18 { continue; }
        let mut s = EFStatic3dObj::default();
        if s.LoadPropertiesBinary(&r.data).is_err() { continue; }
        let Some(w) = &s.world else { continue };
        let (mut n, mut avg) = (0f32, [0f32; 3]);
        for sec in &w.sectors { for m in &sec.meshes { for c in &m.mesh.colors { n += 1.0; for k in 0..3 { avg[k] += c[k] as f32; } } } }
        println!("uid {:#x} format {:#x} sectors {} extra {} prelit avg {:?}", s.uid, w.format, w.sectors.len(), s.extra, avg.map(|x| x / n.max(1.0)));
        for l in &s.lights { println!("  light type {:#x} flags {:#x} col {:?} radius {} at {:?}", l.light.light_type, l.light.flags, l.light.color, l.light.radius, &l.matrix[6..9]); }
        for (i, m) in w.materials.iter().enumerate().take(6) { println!("  mat{i} col {:?} amb {} dif {} tex {:?}", m.color, m.ambient, m.diffuse, m.texture.as_ref().map(|t| t.name.clone())); }
    }
}

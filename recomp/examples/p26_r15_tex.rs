//! P26 R15 scratch: dump every texture whose name contains ARG2 from a level's TXDs (side TEXDIC_* + pack 0x22) as PPM.
use scooby_recomp::engine::{iso::IsoFs, gfx::EFTextureDict, rw::image::RwTexDictionary, runtime::EngineRuntime};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let lvl = a[1].clone();
    let fs = IsoFs::open("../Scooby Doo Mystery Mayhem.ISO").expect("iso");
    let pack = EngineRuntime::ReadLevel(&fs, &lvl, &format!("{}.P2R", lvl.replace('/', ""))).unwrap();
    let mut dicts: Vec<(String, RwTexDictionary)> = Vec::new();
    for (p, b) in &pack.side_files { if p.to_uppercase().contains("TEXDIC_") { let mut d = RwTexDictionary::default(); EFTextureDict::LoadPropertiesBinary(b, &mut d); dicts.push((p.clone(), d)); } }
    for o in &pack.objects { if let Some(r) = o.as_raw() { if r.class_id == 0x22 && r.data.len() > 4 { let mut d = RwTexDictionary::default(); EFTextureDict::LoadPropertiesBinary(&r.data[4..], &mut d); dicts.push((format!("pack22 {:#x}", r.uid), d)); } } }
    for (p, d) in &dicts {
        for (i, t) in d.textures.iter().enumerate() {
            if !t.name.to_lowercase().contains(&a[2].to_lowercase()) { continue; }
            let n = (t.width * t.height) as usize;
            let mut s = [0u64; 4];
            for k in 0..n { for c in 0..4 { s[c] += t.rgba[k * 4 + c] as u64; } }
            let amax = (0..n).map(|k| t.rgba[k*4+3]).max().unwrap_or(0); println!("amax {amax} {p} #{i} {} {}x{} fmt {:#x} avg {:?}", t.name, t.width, t.height, t.raster_format, s.map(|x| x / n.max(1) as u64));
            let f = format!("C:/Temp/p26_r14/tex_{}_{}.ppm", t.name, i);
            let mut out = format!("P6 {} {} 255\n", t.width, t.height).into_bytes();
            for k in 0..n { out.extend_from_slice(&t.rgba[k * 4..k * 4 + 3]); }
            std::fs::write(f, out).unwrap();
        }
    }
}

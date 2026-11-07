//! WP-C: RenderWare loaders + EF 3D resources against the real disc image (skipped without ISO).
use scooby_recomp::engine::gfx::*;
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::rw::image::RwTexDictionary;

use crate::common::ISO;

fn entries(pk: &[u8], want: &[u32]) -> Vec<(u32, Vec<u8>)> {
    let u = |o: usize| u32::from_le_bytes(pk[o..o + 4].try_into().unwrap());
    let n = u(4) as usize;
    let base = 8 + 16 * n;
    (0..n)
        .filter_map(|i| {
            let (sz, off, cid) = (u(12 + 16 * i) as usize, u(16 + 16 * i) as usize, u(20 + 16 * i));
            want.contains(&cid).then(|| (cid, pk[base + off..base + off + sz].to_vec()))
        })
        .collect()
}

fn check_level(fs: &IsoFs, dir: &str, pack: &str) {
    if let Ok(t) = fs.read(&format!("LEVEL/{dir}/EN/TEXDIC_0.TXD")) {
        let mut d = RwTexDictionary::default();
        assert_eq!(EFTextureDict::LoadPropertiesBinary(&t, &mut d), 0, "{dir} txd");
        assert!(!d.textures.is_empty());
        for tx in &d.textures {
            assert!(tx.width > 0 && tx.width <= 1024 && tx.height > 0 && tx.height <= 1024);
            assert_eq!(tx.rgba.len(), (tx.width * tx.height * 4) as usize);
        }
    }
    let pk = fs.read(&format!("LEVEL/{dir}/GEN/{pack}")).unwrap();
    for (cid, b) in entries(&pk, &[0x5b, 0x18]) {
        if cid == 0x5b {
            let mut r = EF3dObjRes::default();
            assert_eq!(r.LoadPropertiesBinary(&b).unwrap(), b.len(), "{dir} 0x5b uid {:x}", r.res.uid);
            let c = r.GetClump().unwrap();
            for g in &c.geometries {
                let nv = g.num_vertices as u32;
                for m in g.submeshes() {
                    assert!(m.indices.iter().all(|&i| i < nv));
                }
            }
        } else {
            let mut s = EFStatic3dObj::default();
            assert_eq!(s.LoadPropertiesBinary(&b).unwrap(), b.len(), "{dir} 0x18");
            let w = s.world.as_ref().unwrap();
            assert!(!w.sectors.is_empty());
            let mut verts = 0;
            for sec in &w.sectors {
                for m in &sec.meshes {
                    let n = m.mesh.positions.len() as u32;
                    verts += n;
                    assert!(m.indices.iter().all(|&i| i < n));
                    assert_eq!(m.mesh.uvs.len(), m.mesh.positions.len());
                }
            }
            assert!(verts > 0);
        }
    }
}

#[test]
fn ep1l01_assets() {
    let Ok(fs) = IsoFs::open(ISO) else { eprintln!("ISO missing, skipped"); return };
    check_level(&fs, "EP1/L01", "EP1L01.P2R");
}

#[test]
fn all_levels_assets() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let packs: Vec<String> = fs
        .names()
        .filter(|n| n.to_uppercase().starts_with("LEVEL/EP") && n.to_uppercase().ends_with(".P2R"))
        .map(|s| s.to_string())
        .collect();
    assert!(packs.len() > 10, "{}", packs.len());
    eprintln!("{} packs", packs.len());
    for p in packs {
        let parts: Vec<&str> = p.split('/').collect();
        let dir = format!("{}/{}", parts[1], parts[2]);
        check_level(&fs, &dir, parts[parts.len() - 1]);
    }
}

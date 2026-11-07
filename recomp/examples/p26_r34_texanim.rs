//! P26 R34 scratch: texture-anim descriptors (0x54) per level: game object, texture, REFPUID, frames.
//! cargo run --example p26_r34_texanim -- EP1/L01 EP2/L01A ...
use scooby_recomp::engine::{iso::IsoFs, runtime::EngineRuntime, dt::descriptors::{EFIDescriptor, DTTextureAnimationDescriptor}};
fn main() {
    let fs = IsoFs::open("../Scooby Doo Mystery Mayhem.ISO").expect("iso");
    for lvl in std::env::args().skip(1) {
        let Ok(mut pack) = EngineRuntime::ReadLevel(&fs, &lvl, &format!("{}.P2R", lvl.replace('/', ""))) else { println!("{lvl}: read fail"); continue };
        let mut frames = std::collections::HashMap::new();
        for o in pack.objects.iter_mut() {
            if let Some(d) = o.as_dt() {
                if let Some(r) = d.as_any().downcast_ref::<scooby_recomp::engine::anim::res::EFTextureAnimRes>() {
                    frames.insert(r.hdr.uid, (r.frames.len(), r.frames.last().map_or(0.0, |f| f.1), r.frames.first().map(|f| f.0.clone())));
                }
            }
        }
        for go in &pack.game_objects {
            for (i, c) in go.glo.object_descriptors.objects.iter().enumerate() {
                if std::env::var("ALLD").is_ok_and(|v| u32::from_str_radix(&v, 16).ok() == Some(go.uid)) {
                    if let Some(d) = scooby_recomp::engine::dt::descriptors::as_idesc(c.as_ref()) { println!("  idx {i} cls {:#x} puid {:#x}", d.GetInstanceCLSID(), d.DescPUID()); } else { println!("  idx {i} ?"); }
                }
                if let Some(d) = c.as_any().downcast_ref::<EFIDescriptor<DTTextureAnimationDescriptor>>() {
                    println!("{lvl} pos {:?} go {:#x} glo {:#x} idx {i} tex {} ref {:#x} res {:#x} {:?}", go.glo.pos, go.uid, go.glo.glo_file_id, d.desc.texture_file_name, d.desc.refpuid, d.desc.puid, frames.get(&(d.desc.puid as u32)));
                }
            }
        }
    }
}

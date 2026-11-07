//! P26 R16 scratch: class ids of a level pack + static light count per class 0x18 object.
//! cargo run --example p26_r16_cls -- EP1/L04
use scooby_recomp::engine::{iso::IsoFs, gfx::EFStatic3dObj, runtime::EngineRuntime};
fn main() {
    let lvl = std::env::args().nth(1).unwrap();
    let fs = IsoFs::open("../Scooby Doo Mystery Mayhem.ISO").expect("iso");
    let pack = EngineRuntime::ReadLevel(&fs, &lvl, &format!("{}.P2R", lvl.replace('/', ""))).unwrap();
    let mut cnt = std::collections::BTreeMap::new();
    for o in &pack.objects {
        let Some(r) = o.as_raw() else { continue };
        *cnt.entry(r.class_id).or_insert(0) += 1;
        if r.class_id == 0x18 {
            let mut s = EFStatic3dObj::default();
            let ok = s.LoadPropertiesBinary(&r.data).is_ok();
            println!("static ok {ok} lights {} world {}", s.lights.len(), s.world.is_some());
        }
    }
    println!("{cnt:x?}");
}

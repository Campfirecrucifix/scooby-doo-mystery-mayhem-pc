use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::runtime::*;
use crate::common::ISO;

#[test]
#[ignore]
fn p20_glo_dump() {
    let fs = IsoFs::open(ISO).unwrap();
    let lv = std::env::var("P20_LV").unwrap_or("EP2/L02A".into());
    let res = EngineRuntime::ReadLevel(&fs, &lv, &format!("{}.P2R", lv.replace('/', ""))).unwrap();
    for go in &res.game_objects {
        let d = &go.glo;
        let lo = u32::from_str_radix(&std::env::var("P20_LO").unwrap_or("b0".into()), 16).unwrap();
        let hi = u32::from_str_radix(&std::env::var("P20_HI").unwrap_or("c8".into()), 16).unwrap();
        if (go.uid & 0xffff) < lo || (go.uid & 0xffff) > hi { continue; }
        let ds: Vec<u32> = d.object_descriptors.objects.iter().map(|o| o.GetClassID()).collect();
        println!("{:08x} file {:08x} pos {:?} at {:?} flags {:x} descs {:x?}", go.uid, d.glo_file_id, d.pos, d.look_at, d.object_flags, ds);
    }
}

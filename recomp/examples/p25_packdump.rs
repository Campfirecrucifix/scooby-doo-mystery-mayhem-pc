//! P25 R32: dump a level pack (classes, uids, GLOs with logic codes, side files).
//! cargo run --example p25_packdump -- INTRO INTRO.P2R
use scooby_recomp::engine::{iso::IsoFs, pack::*, dt::types::class_name, Engine, logic_link::*};
struct NullHost;
impl scooby_recomp::engine::Host for NullHost {}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (dir, file) = (a.get(1).cloned().unwrap_or("INTRO".into()), a.get(2).cloned().unwrap_or("INTRO.P2R".into()));
    let fs = IsoFs::open("../Scooby Doo Mystery Mayhem.ISO").expect("iso");
    let mut m = EFRessourcesMgr::default();
    m.level_file_name = Some(file);
    let err = m.LoadLevel(&fs, "", &dir, "EN", &mut RawInstanciator, |_, _| {}).unwrap();
    println!("LoadLevel {dir} -> {err}; {} objects", m.objects.len());
    for (i, o) in m.objects.iter().enumerate() {
        let c = o.GetClassID();
        let uid = o.as_raw().map(|r| format!("{:#x} raw {}B", r.uid, r.data.len())).unwrap_or_default();
        println!("obj {i:3} class {c:#x} {} {uid}", class_name(c).unwrap_or("?"));
    }
    for f in &m.side_files {
        println!("side {} {}B", f.0, f.1.len());
    }
    for g in &m.game_objects {
        println!("glo uid {:#x} code {:#x} pos {:?} flags {:#x} shadows {:#x} desc {:?}", g.uid, g.glo.glo_file_id, g.glo.pos, g.glo.object_flags, g.glo.shadows_flags, g.glo.object_descriptors);
    }
    let mut e = Engine::new(Box::new(NullHost));
    let (mut map, mut rep) = (LogicCodeMap::default(), LevelLogicReport::default());
    let objs = LinkLevelLogic(&mut e, &m, &mut map, &mut rep);
    for h in objs {
        if let Some(o) = e.rt.objects[h].as_ref() {
            let name = o.inst.code.map(|k| e.rt.codes[k].def.name.to_string()).unwrap_or_default();
            println!("logic {:#x} {name}", o.id);
        }
    }
}

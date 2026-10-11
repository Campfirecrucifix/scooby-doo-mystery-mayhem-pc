//! WP-A: ISO + P2R pack loading against the real disc image (skipped when the ISO is absent).
use scooby_recomp::engine::{iso::IsoFs, pack::*};
struct NullHost;
impl scooby_recomp::engine::Host for NullHost {}
/// NullHost plus one identity matrix for every Get*Mat: modules (safepuzzle, zdaphne) dereference the
/// matrix the engine returns for an existing object; NULL there is a test artifact, not engine behaviour.
#[derive(Default)]
struct MatHost(Option<u32>);
impl MatHost {
    fn mat(&mut self, mem: &mut scooby_recomp::engine::Mem) -> u32 {
        *self.0.get_or_insert_with(|| {
            let m = mem.malloc(0x40);
            mem.memset(m, 0, 0x40);
            for o in [0x00, 0x14, 0x28] {
                mem.wf(m + o, 1.0);
            }
            mem.w32(m + 0xc, 0x20003);
            m
        })
    }
}
impl scooby_recomp::engine::Host for MatHost {
    fn APIgameobj_GetLMat(&mut self, mem: &mut scooby_recomp::engine::Mem, _: u32) -> i32 { self.mat(mem) as i32 }
    fn APIgameobj_GetWMat(&mut self, mem: &mut scooby_recomp::engine::Mem, _: u32) -> u32 { self.mat(mem) }
    fn APIgameobj_GetBoneLMat(&mut self, mem: &mut scooby_recomp::engine::Mem, _: u32, _: u32) -> i32 { self.mat(mem) as i32 }
    fn APIgameobj_GetBoneWMat(&mut self, mem: &mut scooby_recomp::engine::Mem, _: u32, _: u32) -> u32 { self.mat(mem) }
}

const ISO: &str = "../Scooby Doo Mystery Mayhem.ISO";

#[test]
fn load_ep1l01_pack() {
    let Ok(fs) = IsoFs::open(ISO) else { eprintln!("ISO missing, skipped"); return };
    assert_eq!(fs.size("LEVEL/EP1/L01/GEN/EP1L01.P2R"), 13502603);
    let mut m = EFRessourcesMgr::default();
    m.level_file_name = Some("EP1L01.P2R".into());
    let mut last = 0;
    let err = m.LoadLevel(&fs, "", "EP1/L01", "EN", &mut RawInstanciator, |i, _| last = i).unwrap();
    assert_eq!(err, 0);
    assert_eq!(last, 471);
    // 471 entries, the trailing 0x1f container dropped, EF_AUDIOENV_DESC deleted (IsLoader)
    assert_eq!(m.objects.len(), 469);
    assert!(m.side_files.iter().any(|f| f.0.ends_with("TEXT.TX")));
    assert!(m.side_files.iter().any(|f| f.0.ends_with("TEXDIC_0.TXD")));
}

#[test]
fn fat_header_loader() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let b = fs.read("level/loader/gen/loader.p2r").unwrap();
    let mut s = scooby_recomp::engine::dt::stream::DTBinaryPersistStream::new(b);
    let mut fat = DTStreamFAT::default();
    assert_eq!(fat.LoadPropertiesBinary(&mut s), 0);
    assert_eq!((fat.version, fat.count), (0x1dbb4, 12));
}

#[test]
fn load_ep1l01_glo_container() {
    use scooby_recomp::engine::dt::{descriptors::*, property::DTObject, stream::*};
    use scooby_recomp::engine::level::EFGameObject;
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let pk = fs.read("LEVEL/EP1/L01/GEN/EP1L01.P2R").unwrap();
    let mut s = DTBinaryPersistStream::new(pk);
    let mut fat = DTStreamFAT::default();
    assert_eq!(fat.LoadPropertiesBinary(&mut s), 0);
    let last = *fat.records.last().unwrap();
    assert_eq!(last.class_id, 0x1f);
    for r in &fat.records[..fat.records.len() - 1] {
        s.ReadRaw(r.size as usize);
    }
    let start = s.Tell();
    let mut c = DTDynamicPropertyContainer::default();
    assert_eq!(c.Load(&mut s), 0);
    assert_eq!(s.Tell() - start, last.size as usize, "container must consume exactly its FAT size");
    let glos: Vec<&EFGameObject> = c.objects.iter().filter_map(|o| o.as_any().downcast_ref::<EFGameObject>()).collect();
    assert!(!glos.is_empty());
    eprintln!("{} objects, {} GLOs; first: {:x?}", c.objects.len(), glos.len(), glos[0].glo.object_descriptors);
}

#[test]
fn load_ep1l01_level_objects() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let mut m = EFRessourcesMgr::default();
    m.level_file_name = Some("EP1L01.P2R".into());
    let err = m.LoadLevel(&fs, "", "EP1/L01", "EN", &mut RawInstanciator, |_, _| {}).unwrap();
    assert_eq!(err, 0);
    assert_eq!(m.game_objects.len(), 166);
    assert_eq!(m.waypoint_groups.len(), 9);
    assert!(m.waypoint_groups.iter().all(|w| w.points.len() == 3 * w.nb_waypoint as usize));
}

#[test]
fn link_ep1l01_logic() {
    use scooby_recomp::engine::{Engine, logic_link::*};
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let mut m = EFRessourcesMgr::default();
    m.level_file_name = Some("EP1L01.P2R".into());
    assert_eq!(m.LoadLevel(&fs, "", "EP1/L01", "EN", &mut RawInstanciator, |_, _| {}).unwrap(), 0);
    let mut e = Engine::new(Box::new(NullHost));
    let (mut map, mut rep) = (LogicCodeMap::default(), LevelLogicReport::default());
    let objs = LinkLevelLogic(&mut e, &m, &mut map, &mut rep);
    eprintln!("codes {}: {:?}", rep.codes.len(), rep.codes.iter().map(|c| format!("{}@{}", c.name, c.sha8)).collect::<Vec<_>>());
    eprintln!("unported: {:?}", rep.unported.iter().map(|c| c.name.clone()).collect::<Vec<_>>());
    eprintln!("objects {} latents {} with logic {}", rep.objects, rep.latents, objs.iter().filter(|&&h| e.rt.objects[h].as_ref().unwrap().inst.code.is_some()).count());
    assert!(rep.codes.iter().any(|c| c.sha8 == "802a33e5"), "tomedoom EP1L01 build must be found");
    assert_eq!(rep.objects + rep.latents, m.game_objects.len());
}

/// Every level pack on the disc: loads, links its logic, and runs one logic frame of the linked
/// objects without panicking (MatHost).
#[test]
fn link_all_levels_logic() {
    use scooby_recomp::engine::{Engine, logic_link::*};
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let mut packs: Vec<String> = fs.names().filter(|n| n.starts_with("LEVEL/") && n.ends_with(".P2R")).map(String::from).collect();
    packs.sort();
    let mut ported = std::collections::BTreeSet::new();
    let mut failures = Vec::new();
    for p in &packs {
        let parts: Vec<&str> = p.split('/').collect(); // LEVEL/<ep>/<lv>/GEN/<file>  or LEVEL/<x>/GEN/<file>
        let dir = parts[1..parts.len() - 2].join("/");
        let mut m = EFRessourcesMgr::default();
        m.level_file_name = Some(parts[parts.len() - 1].to_string());
        let err = m.LoadLevel(&fs, "", &dir, "EN", &mut RawInstanciator, |_, _| {}).unwrap();
        assert_eq!(err, 0, "{p}");
        let mut e = Engine::new(Box::new(MatHost::default()));
        let (mut map, mut rep) = (LogicCodeMap::default(), LevelLogicReport::default());
        let objs = LinkLevelLogic(&mut e, &m, &mut map, &mut rep);
        let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            for &h in &objs {
                if e.rt.objects[h].is_some() {
                    e.run_object(h, 1.0 / 60.0);
                }
            }
        }));
        if let Err(x) = run {
            let msg = x.downcast_ref::<String>().cloned().or_else(|| x.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
            failures.push(format!("{p}: {msg}"));
        }
        for c in &rep.codes {
            if !rep.unported.iter().any(|u| u.uid == c.uid) {
                ported.insert(c.base_name());
            }
        }
        eprintln!("{p}: {} codes ({} unported), {} objects, {} latents", rep.codes.len(), rep.unported.len(), rep.objects, rep.latents);
    }
    eprintln!("ported modules seen: {ported:?}");
    assert!(failures.is_empty(), "first logic frame panicked:
{}", failures.join("
"));
}

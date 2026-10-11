use scooby_recomp::engine::audio::{bank::SoundBank, container::*, mgr::EFNewAudioMgr, paths::*, sbh::*, tables::*};
use scooby_recomp::engine::iso::IsoFs;
const ISO: &str = "../Scooby Doo Mystery Mayhem.ISO";

#[test]
fn randomized_values() {
    let mut r = Rand::default();
    assert_eq!(SOUND_ComputeRandomizedValue("0.7", &mut r), 0.7);
    assert_eq!(SOUND_ComputeRandomizedValue("-1", &mut r), -1.0);
    assert_eq!(SOUND_ComputeRandomizedValue("DEF", &mut r), 0.0);
    for _ in 0..50 {
        let v = SOUND_ComputeRandomizedValue("R/0.8/1.2", &mut r);
        assert!((0.8..=1.2).contains(&v), "{v}");
    }
    assert_eq!(SOUND_GetFinalPitchOfSample(22050, 12.0), 44100);
}

#[test]
fn container_types() {
    let mut r = Rand::default();
    let mut d = EFSoundContainerData { kind: CT_SEQUENCE, children: vec![10, 11, 12], ..Default::default() };
    let mut s = ContainerState::default();
    let seq: Vec<u32> = (0..4).map(|_| select(&mut d, &mut s, &mut r, None)[0]).collect();
    assert_eq!(seq, [10, 11, 12, 10]);
    d.kind = CT_RANDOM;
    let mut last = 0;
    for _ in 0..20 { let c = select(&mut d, &mut s, &mut r, None)[0]; assert_ne!(c, last); last = c; }
    d.kind = CT_ALL;
    assert_eq!(select(&mut d, &mut s, &mut r, None).len(), 3);
    d.kind = CT_SHUFFLE;
    select(&mut d, &mut s, &mut r, None);
    let mut c = d.children.clone(); c.sort();
    assert_eq!(c, [10, 11, 12]);
}

#[test]
fn paths() {
    assert_eq!(bank_paths(0, "EP1/L01", "EN", "EP1/", "GLALL.SBH")[1], "GLOBAL/EP1/EN/AUDIO/GLALL.SBH");
    assert_eq!(bank_paths(1, "EP1/L01", "EN", "EP1/", "X.SBH")[1], "LEVEL/EP1/L01/GEN/AUDIO/X.SBH");
}

#[test]
fn iso_sbh_banks() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let g = SoundBank::load("GLALL", &fs.read("GLOBAL/EP1/EN/AUDIO/GLALL.SBB").unwrap(), &fs.read("GLOBAL/EP1/EN/AUDIO/GLALL.SBH").unwrap()).unwrap();
    assert_eq!(g.sbh.samples.len(), 0x1a40 / 0x8c);
    assert_eq!(g.sbh.playbacks.len(), 0x39b0 / 0xd0);
    assert_eq!(g.sbh.samples[0].name, "FXSnackBoxPicked.VAG");
    assert_eq!(g.sbh.samples[0].sampling_rate, 22050);
    assert_eq!(g.sbh.playbacks[0].volume, "0.7");
    assert!(g.pcm.iter().all(|p| p.as_ref().map_or(false, |p| !p.samples.is_empty())));
    let dir = "LEVEL/EP1/L01/EN/AUDIO/";
    let st = SoundBank::load("STE1L1", &[], &fs.read(&format!("{dir}STE1L1.SBH")).unwrap()).unwrap();
    assert!(st.sbh.is_stream());
    let p = stream_path(&st.sbh.samples[0].name);
    assert!(fs.read(&p).is_ok(), "{p}");
    // full play path through level tables
    let mut m = EFNewAudioMgr::new();
    let lo = SoundBank::load("LOE1L1", &fs.read(&format!("{dir}LOE1L1.SBB")).unwrap(), &fs.read(&format!("{dir}LOE1L1.SBH")).unwrap()).unwrap();
    let mut c = EFTSCTable::EFTSCTable(1);
    c.Initialize("LOE1L1", &fs.read(&format!("{dir}LOE1L1.TSC")).unwrap()).unwrap();
    let mut e = EFSETTable::EFSETTable(1);
    e.Initialize("LOE1L1", &fs.read(&format!("{dir}LOE1L1.TSE")).unwrap()).unwrap();
    m.add_bank(1, c, e);
    m.add_sound_bank(1, lo);
    let mut ok = 0;
    for i in 0..m.tse[1].data.len() as u32 {
        if m.Play(0x4000_0000 | i, 1) != 0 && m.voices.iter().any(|v| v.sample_rate > 0) { ok += 1; }
        m.voices.clear();
    }
    assert!(ok > 0);
}

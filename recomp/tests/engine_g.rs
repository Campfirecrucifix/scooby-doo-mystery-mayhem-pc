use scooby_recomp::engine::audio::{bank::SoundBank, mgr::EFNewAudioMgr, tables::*, vag::decode_adpcm};
use scooby_recomp::engine::iso::IsoFs;
const ISO: &str = "../Scooby Doo Mystery Mayhem.ISO";

#[test]
fn adpcm_end_flag() {
    let mut d = vec![0u8; 48];
    d[17] = 1; // end in block 2
    let p = decode_adpcm(&d, true);
    assert_eq!(p.samples.len(), 56);
}

#[test]
fn ep1l01_banks_and_tables() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let dir = "LEVEL/EP1/L01/EN/AUDIO/";
    for n in ["LOE1L1", "STE1L1"] {
        let sbb = fs.read(&format!("{dir}{n}.SBB")).unwrap_or_default();
        let sbh = fs.read(&format!("{dir}{n}.SBH")).unwrap();
        let b = SoundBank::load(n, &sbb, &sbh).unwrap();
        let total = b.pcm.iter().flatten().count() + b.sbh.samples.len();
        assert!(total > 0, "{n}");
        let mut c = EFTSCTable::EFTSCTable(1);
        c.Initialize(n, &fs.read(&format!("{dir}{n}.TSC")).unwrap()).unwrap();
        let mut e = EFSETTable::EFSETTable(1);
        e.Initialize(n, &fs.read(&format!("{dir}{n}.TSE")).unwrap()).unwrap();
        assert!(!c.data.is_empty() && !e.data.is_empty());
    }
}

#[test]
fn play_stop() {
    let mut e = EFSETTable::EFSETTable(1);
    let mut d = EFSoundEventData::default();
    d.set_word(0x24, 1); d.set_word(0x2c, 1);
    d.playbacks = vec![0x4000_0005];
    e.data.push(d);
    let mut m = EFNewAudioMgr::new();
    m.add_bank(1, EFTSCTable::EFTSCTable(1), e);
    assert_ne!(m.Play(0x4000_0000, 7), 0);
    assert!(m.IsPlaying(0x4000_0000, 7));
    m.StopSound(0x4000_0000, 7);
    assert!(!m.IsPlaying(0x4000_0000, 7));
}

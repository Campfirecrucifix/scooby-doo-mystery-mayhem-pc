use scooby_recomp::engine::audio::{handlers::*, mgr::*};
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::pack::EFRessourcesMgr;

use crate::common::ISO;

fn load_ep1l01(fs: &IsoFs) -> (EFRessourcesMgr, EFNewAudioMgr, usize) {
    let mut res = EFRessourcesMgr::default();
    res.level_file_name = Some("EP1L01.P2R".into());
    let mut inst = AudioInstanciator::default();
    assert_eq!(res.LoadLevel(fs, "", "EP1/L01", "EN", &mut inst, |_, _| {}).unwrap(), 0);
    let mut m = EFNewAudioMgr::new();
    let envs = inst.env.lock().unwrap().clone();
    assert_eq!(envs.len(), 1, "one EF_AUDIOENV_DESC");
    envs[0].Apply(&mut m);
    eprintln!("env {:?} banks {:?}", envs[0].strings, m.bank_names);
    let n = m.LoadSoundBanks("EP1/L01", "EN", &mut |p| fs.read(p).ok());
    (res, m, n)
}

#[test]
fn ep1l01_banks() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (_r, m, n) = load_ep1l01(&fs);
    eprintln!("{n} banks; tse {:?}", m.tse.iter().map(|t| (t.name.clone(), t.data.len())).collect::<Vec<_>>());
}

#[test]
fn ep1l01_descriptors_and_logic_events() {
    use scooby_recomp::engine::audio::stream::load_stream;
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (res, mut m, n) = load_ep1l01(&fs);
    assert!(n >= 2, "banks loaded: {n}");
    // every sound descriptor of every GLO is registered on its owner
    let (mut want, mut got) = (0, 0);
    for go in &res.game_objects {
        let regs = CreateSoundInstances(&mut m, go);
        got += regs.len();
        want += go.glo.object_descriptors.objects.iter().filter(|d| {
            let a = d.as_any();
            a.is::<scooby_recomp::engine::dt::descriptors::EFIDescriptor<scooby_recomp::engine::dt::descriptors::DTSoundDescriptor>>()
                || a.is::<scooby_recomp::engine::dt::descriptors::EFIDescriptor<scooby_recomp::engine::dt::descriptors::DTSoundContainerDescriptor>>()
        }).count();
        for (_, id) in regs { assert!(m.objects[&go.uid].refs.iter().any(|r| r.id == id)); }
    }
    eprintln!("{got}/{want} sound descriptors");
    assert!(want > 0 && got == want);
    // every event id literal passed to APInewSound::Play* by src/modules (grep)
    for id in [0x4000_0030u32, 0x4000_0045, 0x8000_0003, 0x8000_0006, 0x8000_0007, 0x8000_0008, 0x8000_0009, 0xa000_0002] {
        m.voices.clear();
        // bank = id>>30 (EFAudioEnvDescriptor 0x001f9db0 -> SendSpecificData(s1,s3,s2): bank0 GLALL,
        // bank1 LOE1L1, bank2 STE1L1). Ids past the level table do not exist in this level.
        let (b, i) = ((id >> 30) as usize, (id & 0xfff_ffff) as usize);
        if id & 0x2000_0000 == 0 && i >= m.tse[b].data.len() { eprintln!("{id:#x} not in {} ({} entries)", m.tse[b].name, m.tse[b].data.len()); continue; }
        assert_ne!(m.Play(id, 1), 0, "{id:#x}");
        let v = m.voices.iter().find(|v| v.sample_rate > 0).unwrap_or_else(|| panic!("{id:#x} no sample"));
        let ok = if v.stream.is_some() {
            load_stream(v, &mut |p| fs.read(p).ok()).map_or(false, |(_, p)| !p.samples.is_empty())
        } else {
            m.banks[v.bank as usize].as_ref().unwrap().pcm[v.sample as usize].as_ref().map_or(false, |p| !p.samples.is_empty())
        };
        assert!(ok, "{id:#x} empty pcm");
        m.StopSound(id, 1);
    }
}

use scooby_recomp::engine::audio::{adsr::*, bank::SoundBank, handlers::*, mgr::*, sbh::*, sound3d::*, stream::load_stream, tables::*, wav::pcm_to_wav};
use scooby_recomp::engine::dt::property::DTObject;
use scooby_recomp::engine::dt::stream::DTMemoryPersistStream;
use scooby_recomp::engine::iso::IsoFs;
use std::sync::{Arc, Mutex};

fn iso() -> Option<IsoFs> {
    ["../Scooby Doo Mystery Mayhem.ISO", "C:/Users/Mami/Documents/Decomp/Scooby Doo MM Decomp/Scooby Doo Mystery Mayhem.ISO"]
        .iter().find_map(|p| IsoFs::open(p).ok())
}

#[test]
fn newlib_rand_shared() {
    // newlib: next = next * 0x5851f42d4c957f2d + 1; return (next >> 32) & 0x7fffffff; seed 1
    let mut r = Rand::default();
    let s = 1u64.wrapping_mul(0x5851f42d4c957f2d).wrapping_add(1);
    assert_eq!(r.rand(), ((s >> 32) & 0x7fff_ffff) as i32);
    let shared: SharedRand = Arc::new(Mutex::new(Rand::default()));
    let a = EFNewAudioMgr::with_rng(shared.clone());
    a.rng.lock().unwrap().rand();
    assert_eq!(shared.lock().unwrap().0, s); // same global state
}

#[test]
fn adsr_registers() {
    let t = &G_ATTACK_RATE;
    assert_eq!(SOUND_LogarithmicSearch(t, 100, 0, 100, t[0].1), t[0].0);
    assert_eq!(SOUND_LogarithmicSearch(t, 100, 0, 100, -5.0), t[0].0);
    assert_eq!(SOUND_LogarithmicSearch(t, 100, 0, 100, 1e12), t[99].0);
    assert_eq!(SOUND_LogarithmicSearch(t, 100, 0, 100, (t[40].1 + t[41].1) / 2.0), t[41].0); // decomp quirk: upper entry in this branch
    let p = SbhPlayback { attack: "0".into(), decay: "0".into(), sustain_level: "0".into(), sustain_rate: "-1".into(), release: "0".into(), ..Default::default() };
    let a = SOUND_GetFinalADSROfSample(&p, &mut Rand::default());
    assert_eq!(a & 0x8000, 0x8000);
    assert_eq!(a & 0xc020_0000, 0xc020_0000);
    assert_eq!((a >> 22) & 0x7f, G_SUSTAIN_RATE_RATE[108].0 & 0x7f);
    // fastest attack reaches full level, release goes to 0
    let e = envelope(0x0000_000f, 4000, Some(2000)); // fastest attack + release;
    assert!(e[..2000].iter().cloned().fold(0.0, f32::max) > 0.99);
    assert!(e[3999] < 0.01);
    assert_eq!(apply_adsr(&[1000; 10], 48000, 1.0, 0, None).len(), 10);
}

fn ev(mode: u32) -> EFSoundEventData {
    let mut d = EFSoundEventData::default();
    d.set_word(0x08, mode);
    for (o, v) in [(0x10, 1.0f32), (0x14, 1.0), (0x18, 11.0), (0x1c, 0.0), (0x20, 20.0)] { d.set_word(o, v.to_bits()); }
    d.set_word(0x2c, 1);
    d
}

#[test]
fn volume_and_pan() {
    let m = MixParams { global: 1.0, group: 1.0, master: 1.0, mono: false };
    // 2D: snaps first time, centred pan -> half each side
    let mut i = EventInst { base_vol: 0x3fff, pan_sbh: 0.0, ..Default::default() };
    ComputeVolumeAndPan(&mut i, &ev(0), &Geometry::default(), &m);
    assert_eq!((i.left, i.right), (0x3fff as f32 / 2.0, 0x3fff as f32 / 2.0));
    // 3D: halfway between min/max -> 0.5; emitter on +x of a listener looking down +z
    let fr = Frame { at: [0.0, 0.0, 1.0], pos: [0.0; 3] };
    let g = Geometry { listener: Some(fr), camera: Some(fr), emitter: Some([6.0, 0.0, 0.0]) };
    let mut i = EventInst { base_vol: 100, ..Default::default() };
    ComputeVolumeAndPan(&mut i, &ev(2), &g, &m);
    assert!((i.vol_cur - 0.5).abs() < 1e-6);
    assert!((i.pan_cur + 1.0).abs() < 1e-6); // -sin(pi/2)
    assert!(i.left > i.right);
    // beyond cut-off: silent
    let g2 = Geometry { emitter: Some([30.0, 0.0, 0.0]), ..g };
    let mut j = EventInst { base_vol: 100, ..Default::default() };
    ComputeVolumeAndPan(&mut j, &ev(1), &g2, &m);
    assert_eq!(j.left + j.right, 0.0);
    // smoothing: 0.1 per update toward target
    let mut k = i;
    ComputeVolumeAndPan(&mut k, &ev(2), &Geometry { emitter: Some([0.5, 0.0, 0.0]), ..g }, &m);
    assert!((k.vol_cur - 0.6).abs() < 1e-6);
    // mono
    let mut n = EventInst { base_vol: 100, ..Default::default() };
    ComputeVolumeAndPan(&mut n, &ev(2), &g, &MixParams { mono: true, ..m });
    assert_eq!(n.left, n.right);
}

fn synth_mgr() -> EFNewAudioMgr {
    let mut m = EFNewAudioMgr::new();
    let mut e = EFSETTable::EFSETTable(0);
    for _ in 0..4 { let mut d = ev(0); d.playbacks = vec![0]; e.data.push(d); }
    m.add_bank(0, EFTSCTable::EFTSCTable(0), e);
    m
}

#[test]
fn sound_object_lifetimes() {
    let mut m = synth_mgr();
    assert_eq!(m.APInewSound_Play(5, 1, 0), 1); // not registered by a descriptor
    let h = CreateAudioInstance(SOUND_INST).unwrap();
    assert_eq!(h.GetClassID(), 0x17);
    assert_eq!(CreateAudioInstance(SOUND_CONTAINER_INST).unwrap().GetClassName(), "SOUND_CONTAINER_INST");
    h.as_any().downcast_ref::<EFSoundDescriptorHandler>().unwrap().Initialize(&mut m, 5, 1);
    assert_eq!(m.APInewSound_Play(5, 1, 0), 0);
    let hd = m.objects[&5].refs[0].handle;
    assert_ne!(hd, NO_HANDLE);
    assert!(m.APInewSound_IsPlaying(5, 1));
    assert_eq!(m.APInewSound_Play(5, 99, 0), 1);
    // completion releases the handle (not parented)
    m.Update(1.0, &|_| Some(0.5));
    assert!(!m.APInewSound_IsPlaying(5, 1));
    assert_eq!(m.objects[&5].refs[0].handle, NO_HANDLE);
    assert!(!m.handles[hd as usize].used);
    // parented sounds keep their handle
    m.APInewSound_ParentSoundEventToJoint(5, 1, 3);
    m.APInewSound_Play(5, 1, 0);
    m.Update(1.0, &|_| Some(0.5));
    assert_ne!(m.objects[&5].refs[0].handle, NO_HANDLE);
    // object deletion while playing: handle becomes orphan/auto-release, freed on completion
    m.object(6).AddSound(2);
    m.APInewSound_Play(6, 2, 0);
    let h6 = m.objects[&6].refs[0].handle;
    m.DeleteAllSoundReferences(6);
    assert!(m.handles[h6 as usize].auto_release && m.handles[h6 as usize].orphan);
    m.Update(1.0, &|_| Some(0.5));
    assert!(!m.handles[h6 as usize].used);
    // StopSound(all)
    m.object(7).AddSound(3);
    m.APInewSound_Play(7, 3, 0);
    assert_eq!(m.APInewSound_StopSound(7, ALL_SOUNDS), 0);
    assert!(!m.APInewSound_IsPlaying(7, 3));
    // master volume carried into the handle
    assert_eq!(m.APInewSound_SetSoundEventMasterVolume(7, 3, 0.25), 0);
    assert_eq!(m.APInewSound_GetSoundEventMasterVolume(7, 3), 0.25);
    m.APInewSound_SetFXVolume(3.0);
    assert_eq!(m.APInewSound_GetFXVolume(), 1.0);
}

#[test]
fn handle_pool_limit() {
    let mut m = synth_mgr();
    for o in 0..MAX_SOUND_HANDLES as u32 { m.object(o + 100).AddSound(0); assert_eq!(m.APInewSound_Play(o + 100, 0, 0), 0); }
    m.object(1).AddSound(0);
    assert_eq!(m.APInewSound_Play(1, 0, 0), 2);
}

#[test]
fn audio_env_descriptor() {
    let mut b = Vec::new();
    for _ in 0..3 { b.extend_from_slice(&7i32.to_le_bytes()); }
    for s in ["EP1\\", "a", "b", "c"] { b.extend_from_slice(&(s.len() as i32 + 1).to_le_bytes()); b.extend_from_slice(s.as_bytes()); b.push(0); }
    let mut d = EFAudioEnvDescriptor::default();
    assert_eq!(d.LoadPropertiesBinary(&mut DTMemoryPersistStream::new(b)), 0);
    let mut m = EFNewAudioMgr::new();
    d.Apply(&mut m);
    assert_eq!(m.env_name, "EP1\\");
    assert_eq!(m.env_specific, ["a", "c", "b"]);
    let w = pcm_to_wav(&[100, -100], 22050, 1.0, 0.0);
    assert_eq!(&w[..4], b"RIFF");
    assert_eq!(w.len(), 44 + 8);
}

/// Book (Tome of Doom: tomedoom/tome_doom.rs 0x2e,0x14,0xf,0x42,0x41,0x13,0x10), orb collected
/// into the book (wisps/wisp_base.rs: Play(book, 0x17)) resolve to SBH samples on EP1/L01.
#[test]
fn iso_book_capture_orb_events() {
    let Some(fs) = iso() else { return };
    let mut m = EFNewAudioMgr::new();
    let g = "GLOBAL/EP1/EN/AUDIO/";
    let l = "LEVEL/EP1/L01/EN/AUDIO/";
    let rd = |p: String| fs.read(&p).unwrap();
    let mut c = EFTSCTable::EFTSCTable(0); c.Initialize("GLALL", &rd(format!("{g}GLALL.TSC"))).unwrap();
    let mut e = EFSETTable::EFSETTable(0); e.Initialize("GLALL", &rd(format!("{g}GLALL.TSE"))).unwrap();
    m.add_bank(0, c, e);
    m.add_sound_bank(0, SoundBank::load("GLALL", &rd(format!("{g}GLALL.SBB")), &rd(format!("{g}GLALL.SBH"))).unwrap());
    let mut c = EFTSCTable::EFTSCTable(1); c.Initialize("LOE1L1", &rd(format!("{l}LOE1L1.TSC"))).unwrap();
    let mut e = EFSETTable::EFSETTable(1); e.Initialize("LOE1L1", &rd(format!("{l}LOE1L1.TSE"))).unwrap();
    m.add_bank(1, c, e);
    m.add_sound_bank(1, SoundBank::load("LOE1L1", &rd(format!("{l}LOE1L1.SBB")), &rd(format!("{l}LOE1L1.SBH"))).unwrap());
    let mut names = Vec::new();
    for id in [0x2e, 0x14, 0xf, 0x42, 0x41, 0x13, 0x10, 0x17] {
        m.voices.clear();
        assert_ne!(m.Play(id, 1), 0, "{id:#x}");
        let v = m.voices.iter().find(|v| v.sample_rate > 0).unwrap_or_else(|| panic!("{id:#x} no sample"));
        let b = m.banks[v.bank as usize].as_ref().unwrap();
        assert!(b.pcm[v.sample as usize].as_ref().map_or(false, |p| !p.samples.is_empty()));
        names.push(format!("{id:#x}={}", b.sbh.samples[v.sample as usize].name));
        m.StopSound(id, 1);
    }
    println!("{}", names.join(" "));
    // stream bank: decode the first STREAM VAG through the voice path
    let st = SoundBank::load("STE1L1", &[], &rd(format!("{l}STE1L1.SBH"))).unwrap();
    let s0 = &st.sbh.samples[0];
    m.Play(0x2e, 1);
    let mut v = m.voices[0].clone();
    v.stream = Some(scooby_recomp::engine::audio::paths::stream_path(&s0.name));
    v.sample_rate = s0.sampling_rate;
    let (rate, pcm) = load_stream(&v, &mut |p| fs.read(p).ok()).unwrap();
    assert!(rate > 0 && !pcm.samples.is_empty());
}

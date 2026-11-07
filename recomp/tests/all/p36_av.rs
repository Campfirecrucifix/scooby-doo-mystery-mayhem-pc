//! P36 (audio / FMV / HUD polish) checks.
use scooby_recomp::engine::audio::{bank::SoundBank, mgr::EFNewAudioMgr, tables::*};
use scooby_recomp::engine::iso::IsoFs;
use crate::common::ISO;

/// Every key-on gets its own `Voice::serial` (the host's identity of an SPU voice): two instances
/// of a multi-instance event on one handle, and a Stop + Play of the same handle, must not share
/// an identity (the host keyed voices by (handle, bank, sample): the 2nd instance was never heard
/// and a restart continued the old clip).
#[test]
fn p36_voice_serial_per_key_on() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let dir = "LEVEL/EP1/L01/EN/AUDIO/";
    let mut m = EFNewAudioMgr::new();
    let lo = SoundBank::load("LOE1L1", &fs.read(&format!("{dir}LOE1L1.SBB")).unwrap(), &fs.read(&format!("{dir}LOE1L1.SBH")).unwrap()).unwrap();
    let mut c = EFTSCTable::EFTSCTable(1);
    c.Initialize("LOE1L1", &fs.read(&format!("{dir}LOE1L1.TSC")).unwrap()).unwrap();
    let mut e = EFSETTable::EFSETTable(1);
    e.Initialize("LOE1L1", &fs.read(&format!("{dir}LOE1L1.TSE")).unwrap()).unwrap();
    m.add_bank(1, c, e);
    m.add_sound_bank(1, lo);
    let (mut multi, mut restart) = (0, 0);
    for i in 0..m.tse[1].data.len() as u32 {
        let id = 0x4000_0000 | i;
        let inst = m.tse[1].data[i as usize].word(0x2c);
        let h = m.GetSoundHandle(0x2a00_0001, id);
        m.PlaySoundHandle(h, None);
        let first: Vec<u32> = m.voices.iter().filter(|v| v.handle == h).map(|v| v.serial).collect();
        if first.is_empty() {
            m.ReleaseSoundHandle(h);
            m.voices.clear();
            continue;
        }
        if inst >= 2 {
            m.PlaySoundHandle(h, None);
            let all: Vec<u32> = m.voices.iter().filter(|v| v.handle == h).map(|v| v.serial).collect();
            let mut d = all.clone();
            d.sort();
            d.dedup();
            assert_eq!(d.len(), all.len(), "event {id:#x}: instances share a serial {all:?}");
            multi += 1;
        }
        m.StopSoundHandle(h);
        m.PlaySoundHandle(h, None);
        assert!(m.voices.iter().filter(|v| v.handle == h).all(|v| !first.contains(&v.serial)), "event {id:#x}: restart reused a serial");
        restart += 1;
        m.StopSoundHandle(h);
        m.ReleaseSoundHandle(h);
        m.voices.clear();
    }
    eprintln!("p36 serial: {restart} events restarted, {multi} multi-instance");
    assert!(restart > 0 && multi > 0, "{restart} {multi}");
}

/// The menu packs (INTRO, LOADER) are flat level dirs: the sound bank dir derived from the pack
/// name must find the level + stream banks ReadLevel pre-read (before, INTRO -> "" and LOADER ->
/// "LOA/DER": title music 0x80000000 and every menu SFX failed with "no handle").
#[test]
fn p36_menu_pack_bank_dir() {
    use scooby_recomp::engine::runtime::{level_bank_dir, EngineRuntime};
    assert_eq!(level_bank_dir("EP1L03A.P2R"), "EP1/L03A");
    assert_eq!(level_bank_dir("INTRO.P2R"), "INTRO");
    assert_eq!(level_bank_dir("LOADER.P2R"), "LOADER");
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let m = EngineRuntime::ReadLevel(&fs, "INTRO", "INTRO.P2R").unwrap();
    let pre = |f: &str| m.side_files.iter().any(|(p, _)| p == &format!("AUDIO:LEVEL/{}/{f}", level_bank_dir("INTRO.P2R")));
    assert!(pre("EN/AUDIO/LOLOAD.SBH") || pre("GEN/AUDIO/LOLOAD.SBH"), "INTRO level bank not found under its bank dir");
    assert!(pre("EN/AUDIO/STLOAD.SBH") || pre("GEN/AUDIO/STLOAD.SBH"), "INTRO stream bank not found under its bank dir");
    // the title / menu music streams named by that bank are on the disc
    let get = |f: &str| m.side_files.iter().find(|(p, _)| p.starts_with("AUDIO:LEVEL/INTRO/") && p.ends_with(f)).map(|(_, b)| b.clone()).unwrap();
    let sb = SoundBank::load("STLOAD", &[], &get("STLOAD.SBH")).unwrap();
    let names: Vec<String> = sb.sbh.samples.iter().map(|s| scooby_recomp::engine::audio::paths::stream_path(&s.name)).collect();
    eprintln!("p36 STLOAD streams: {names:?}");
    for n in &names { assert!(fs.read(n).is_ok(), "stream {n} missing"); }
}

/// R2: the offline capture mix (GAME_AUDIO_WAV): resampling to 48 kHz, gain, loop, pause, bus stats
/// and the WAV header; a looped 1 kHz bank tone at 24 kHz/pitch 1 must come out at the right level.
#[test]
fn p36_capture_mix() {
    use scooby_recomp::engine::audio::capture::{Bus, Capture, RATE};
    use scooby_recomp::engine::audio::spu::{SpuData, SpuVoice};
    let tone: Vec<i16> = (0..24).map(|i| ((i as f32 / 24.0 * std::f32::consts::TAU).sin() * 16384.0) as i16).collect();
    let mut c = Capture::default();
    for side in 0..2u8 {
        let v = SpuVoice::new(SpuData::Pcm(tone.clone().into(), Some(0)), 24000, 1.0, 0, true, 0.0);
        c.spawn(7, side, v, 24000, 1.0, Bus::Music);
    }
    c.set(7, 1.0, 0.5, 1.0, false);
    c.advance(1.0);
    assert_eq!(c.out.len(), 2 * RATE as usize);
    let rms = |ch: usize| (c.out.iter().skip(ch).step_by(2).skip(1000).map(|&s| (s as f64 / 32768.0).powi(2)).sum::<f64>() / (RATE as f64 - 1000.0)).sqrt();
    let (l, r) = (rms(0), rms(1));
    eprintln!("p36 capture rms L {l:.3} R {r:.3}");
    assert!((l - 0.354).abs() < 0.03 && (r - 0.177).abs() < 0.02, "{l} {r}");
    // zero crossings: 1 kHz -> ~2000 per second
    let zc = c.out.iter().step_by(2).collect::<Vec<_>>().windows(2).filter(|w| (*w[0] < 0) != (*w[1] < 0)).count();
    assert!((1900..2100).contains(&zc), "{zc}");
    c.set(7, 1.0, 1.0, 1.0, true);
    c.advance(0.5);
    assert!(c.out[2 * RATE as usize..].iter().all(|&s| s == 0), "paused voice still audible");
    c.retain(&|_| false);
    c.advance(0.5);
    assert_eq!(c.clipped, 0);
    assert!(c.stats[&Bus::Music].rms_db() < -6.0 && c.stats[&Bus::Sfx].rms_db() <= -120.0);
    let w = c.wav_bytes();
    assert_eq!(&w[0..4], b"RIFF");
    assert_eq!(w.len(), 44 + c.out.len() * 2);
    assert!(c.report().contains("bus Music"));
}

/// R2: the original Options sliders scale the right buses: miscHUDFUNC_SetMusicVolume = groups 0xa8,
/// SetFXVolume = 0x17 (+ fx volume), movies = 0x300 / video volume. Every EP1/L01 stream event that
/// plays a music stream (MU*.INT) must be in a music group, no level-bank SFX event may be.
#[test]
fn p36_option_slider_groups() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let dir = "LEVEL/EP1/L01/EN/AUDIO/";
    let tse = |n: &str, b: u32| {
        let mut e = EFSETTable::EFSETTable(b);
        e.Initialize(n, &fs.read(&format!("{dir}{n}.TSE")).unwrap()).unwrap();
        e
    };
    let lvl = tse("LOE1L1", 1);
    let lv: Vec<u32> = lvl.data.iter().map(|d| d.word(0x28)).collect();
    eprintln!("p36 level-bank groups: {:?}", { let mut v = lv.clone(); v.sort(); v.dedup(); v });
    assert!(lv.iter().all(|g| g & 0xa8 == 0), "level SFX in a music group");
    assert!(lv.iter().filter(|g| *g & 0x17 != 0).count() * 2 > lv.len(), "most level SFX should follow the FX slider");
    let names: Vec<String> = fs.names().filter(|n| n.starts_with(dir) && n.ends_with(".TSE")).map(|n| n[dir.len()..n.len() - 4].to_string()).collect();
    eprintln!("p36 L01 tables: {names:?}");
    // the stream table (bank 2): 0x80000000 = level music MUEP1L1.INT (GAME_SNDLOG run), 0x80000009 = cut-scene
    // stream CS1_1_1.INT, 0x8000002c = hero VO SCO501EN.VAG
    let st = names.iter().find(|n| n.starts_with("ST")).expect("stream table");
    let s = tse(st, 2);
    let g = |i: usize| s.data[i].word(0x28);
    eprintln!("p36 stream groups: music {:#x} cut-scene {:#x} vo {:#x}", g(0), g(9), g(0x2c));
    assert!(g(0) & 0xa8 != 0 && g(0) & 0x17 == 0, "level music not on the music slider: {:#x}", g(0));
}

/// R4 (SFX coverage helper): bank sample names of each episode's first level (LO*.SBH) so the
/// GAME_SNDLOG key-on `name` column can be matched to monsters. `cargo test ... p36_dump_level_sfx -- --ignored --nocapture`
#[test]
#[ignore]
fn p36_dump_level_sfx() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    for (d, b) in [("EP1/L01", "LOE1L1"), ("EP2/L01A", "LOE2L1A"), ("EP3/L01", "LOE3L1"), ("EP4/L01", "LOE4L1"), ("EP6/L01", "LOE6L1"), ("EP1/L03A", "LOE1L3A"), ("EP3/L03", "LOE3L3")] {
        let p = ["EN", "GEN"].iter().map(|l| format!("LEVEL/{d}/{l}/AUDIO/{b}.SBH")).find(|p| fs.read(p).is_ok());
        let Some(p) = p else { eprintln!("{d}: no {b}.SBH"); continue };
        let sb = SoundBank::load(b, &[], &fs.read(&p).unwrap()).unwrap();
        let n: Vec<String> = sb.sbh.samples.iter().enumerate().map(|(i, s)| format!("{i}:{}", s.name)).collect();
        eprintln!("{d} {b}: {}", n.join(" "));
    }
}

/// R10 (USER_CHECKS E5 / D): PC volume round trip through real temp settings files plus the bus each
/// slider drives. The launcher saves music, the running game changes sfx while the launcher (again)
/// writes voice: the game's per-key merge keeps all three, and a restarted launcher/game reads them.
/// Then every L01 voice class lands on the right PC slider (game.rs pc_cat -> device::pc_category):
/// level music MUEP1L1.INT (group 0x8) -> Music, hero VO SCO501EN.VAG -> Voice / movies, level-bank SFX
/// -> Sound effects. Finding: the intro cut-scene stream CS1_1_1.INT is group 0x4 on the disc (original
/// FX slider group 0x17, NOT the Movies group 0x300 R5 assumed) and interleaved, so it lands on PC Music
/// (BUGS row P36 R10; classification left as is pending the driver).
#[test]
fn p36_r10_volume_round_trip_and_buses() {
    use scooby_recomp::engine::audio::device::pc_category;
    use scooby_recomp::engine::settings::Settings;
    let d = std::env::temp_dir().join(format!("p36_r10_vol_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let p = d.join("settings.toml");
    // 1. launcher: music 0.3
    let (mut l, _) = Settings::load_or_create(&p, None);
    l.audio.music = 0.3;
    l.save(&p).unwrap();
    // 2. game starts (base = file), player sets Sound effects 0.6 in PC SETTINGS
    let (base, _) = Settings::load_or_create(&p, None);
    assert_eq!(base.audio.music, 0.3);
    let mut game = base.clone();
    game.audio.sfx = 0.6;
    // 3. launcher meanwhile writes voice 0.4
    let (mut l2, _) = Settings::load_or_create(&p, None);
    l2.audio.voice = 0.4;
    l2.save(&p).unwrap();
    // 4. game saves with the per-key merge (game_settings::save)
    let disk = Settings::parse(&std::fs::read_to_string(&p).unwrap()).unwrap();
    let m = game.merged_keys(&base, &disk);
    m.save(&p).unwrap();
    // 5. restart: launcher and game read all three; master untouched
    let (r, warn) = Settings::load_or_create(&p, None);
    assert!(warn.is_none(), "{warn:?}");
    assert_eq!((r.audio.master, r.audio.music, r.audio.sfx, r.audio.voice, r.audio.mute), (1.0, 0.3, 0.6, 0.4, false));
    assert_eq!(r.audio.gain(r.audio.sfx), 0.6);
    let mut muted = r.clone();
    muted.audio.mute = true;
    assert_eq!(muted.audio.gain(muted.audio.music), 0.0);
    let _ = std::fs::remove_dir_all(&d);

    // bus per voice class on the real EP1/L01 tables / files
    assert_eq!(pc_category(Some(0x17), false, false), 1);
    assert_eq!(pc_category(Some(0x300), true, true), 2);
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let dir = "LEVEL/EP1/L01/EN/AUDIO/";
    let tse = |n: &str, b: u32| {
        let mut e = EFSETTable::EFSETTable(b);
        e.Initialize(n, &fs.read(&format!("{dir}{n}.TSE")).unwrap()).unwrap();
        e
    };
    let lvl = tse("LOE1L1", 1);
    assert!(lvl.data.iter().all(|e| pc_category(Some(e.word(0x28)), false, false) == 1), "a level-bank SFX off the SFX slider");
    let mut sts: Vec<String> = fs.names().filter(|n| n.starts_with(dir) && n.ends_with(".TSE")).map(|n| n[dir.len()..n.len() - 4].to_string()).filter(|n| n.starts_with("ST")).collect();
    sts.sort();
    assert!(!sts.is_empty(), "no ST*.TSE in {dir}");
    let s = tse(&sts[0], 2);
    // R11: several dirs carry a file of the same name (STREAM/ and the STREA_ shadow copy); IsoFs::names()
    // is HashMap-ordered, so take the sorted first copy. Every failure is printed before the assert: the
    // `all` binary can run with a silenced panic hook (logic_l19) and the main gate lost the message.
    let file = |name: &str| -> Result<bool, String> {
        let mut c: Vec<&str> = fs.names().filter(|n| n.to_ascii_uppercase().ends_with(&format!("/{name}"))).collect();
        c.sort();
        let n = c.first().ok_or(format!("{name} not on the disc"))?;
        let f = fs.read(n).map_err(|e| format!("{n}: {e:?}"))?;
        Ok(scooby_recomp::engine::audio::spu::is_interleaved(&f))
    };
    let mut errs = Vec::new();
    for (i, name, want) in [(0usize, "MUEP1L1.INT", 0usize), (9, "CS1_1_1.INT", 0), (0x2c, "SCO501EN.VAG", 2)] {
        let Some(ev) = s.data.get(i) else { errs.push(format!("{}: no event {i:#x} ({} events)", sts[0], s.data.len())); continue };
        let g = ev.word(0x28);
        match file(name) {
            Err(e) => errs.push(e),
            Ok(il) => {
                let cat = pc_category(Some(g), true, il);
                eprintln!("p36 r10 {name} ({}): group {g:#x} interleaved {il} -> PC category {cat}", ev.name);
                if cat != want { errs.push(format!("{name}: PC category {cat}, want {want}")); }
            }
        }
    }
    for e in &errs { eprintln!("p36 r10 FAIL {e}"); }
    assert!(errs.is_empty(), "{errs:?}");
}

/// R10 probe: which events sit in the original Options "movies" group (0x300), and the group of every
/// CS* (in-engine cut-scene) stream event, over all TSE tables on the disc.
/// `cargo test ... p36_r10_group_probe -- --ignored --nocapture`
#[test]
#[ignore]
fn p36_r10_group_probe() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let names: Vec<String> = fs.names().filter(|n| n.to_ascii_uppercase().ends_with(".TSE")).map(|s| s.to_string()).collect();
    let mut cs = std::collections::BTreeMap::<u32, u32>::new();
    for n in &names {
        let mut e = EFSETTable::EFSETTable(2);
        let base = n.rsplit('/').next().unwrap();
        if e.Initialize(&base[..base.len() - 4], &fs.read(n).unwrap()).is_err() { continue; }
        for d in &e.data {
            let g = d.word(0x28);
            if g & 0x300 != 0 { eprintln!("p36 r10 movies-group {n}: {} {g:#x}", d.name); }
            if d.name.to_ascii_uppercase().starts_with("CS") { *cs.entry(g).or_default() += 1; }
        }
    }
    eprintln!("p36 r10 CS* event groups (group -> count): {cs:x?} over {} tables", names.len());
}

/// R15: the GAME_LOADLEVEL debug hook fires once: the frame counter stays at the switch frame
/// while the level loads, and the unlatched hook re-queued the request every tick (14x in R14).
#[test]
fn p36_r15_loadlevel_hook_latched() {
    use scooby_recomp::engine::host::game::LevelRequest;
    let mut fired = false;
    assert_eq!(LevelRequest::forced("EP1L02:1500", 1499, &mut fired), None);
    let r = LevelRequest::forced("EP1L02:1500", 1500, &mut fired).unwrap();
    assert_eq!((r.level.as_str(), r.pack.as_str()), ("EP1L02", "EP1L02.P2R"));
    for _ in 0..14 { assert_eq!(LevelRequest::forced("EP1L02:1500", 1500, &mut fired), None); }
    assert_eq!(LevelRequest::forced("bad", 1500, &mut false), None);
}

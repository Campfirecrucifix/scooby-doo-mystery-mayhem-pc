//! P34: settings.toml load / save / migration (engine::settings).
use scooby_recomp::engine::input::host::Controls;
use scooby_recomp::engine::settings::{Settings, WindowMode};

/// P34 R12: a disc image on a missing drive / an unreachable share fails fast, never blocks > the limit
#[test]
fn p34_settings_disc_probe_is_bounded() {
    use scooby_recomp::engine::settings::validate_iso_within;
    let limit = std::time::Duration::from_secs(2);
    for p in [r"Q:\nope\scooby.iso", r"\\10.255.255.1\x\x.iso"] {
        let t = std::time::Instant::now();
        let r = validate_iso_within(std::path::Path::new(p), limit);
        assert!(r.is_err(), "{p}: {r:?}");
        assert!(t.elapsed() < limit + std::time::Duration::from_millis(500), "{p}: {:?}", t.elapsed());
    }
    assert_eq!(Settings::parse("[launcher]\npos = \"12, -7\"\n").unwrap().launcher.pos, Some((12, -7)));
    assert!(Settings::parse("[launcher]\npos = \"12\"\n").is_err());
}

fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("p34_settings_{}_{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn p34_settings_roundtrip() {
    let mut s = Settings::default();
    assert_eq!(Settings::parse(&s.to_toml()).unwrap(), s);
    s.graphics.window_mode = WindowMode::Fullscreen;
    s.graphics.resolution = Some((1920, 1080));
    s.graphics.vsync = false;
    s.graphics.fps_cap = 144;
    s.graphics.msaa = 1;
    s.audio.master = 0.5;
    s.audio.voice = 0.25;
    s.audio.mute = true;
    s.remaster.lighting = true;
    s.launcher.skip = true;
    s.launcher.pos = Some((-1500, 40));
    s.controls.invert_y = true;
    s.controls.deadzone = 0.3;
    s.controls.keyboard[0].1 = vec!["KeyX".into()];
    s.global_ray_tracing = Some(true);
    let t = s.to_toml();
    assert_eq!(Settings::parse(&t).unwrap(), s, "{t}");
}

#[test]
fn p34_settings_parse_partial_and_errors() {
    let s = Settings::parse("deadzone = 0.2\n[audio]\nmusic = 0.4 # c\n[graphics]\nwindow_mode = \"borderless\"\nresolution = \"1600x900\"\n[keyboard]\ncross = [\"KeyQ\"]\n").unwrap();
    assert_eq!(s.controls.deadzone, 0.2);
    assert_eq!(s.audio.music, 0.4);
    assert_eq!(s.audio.master, 1.0);
    assert_eq!(s.graphics.window_mode, WindowMode::Borderless);
    assert_eq!(s.graphics.resolution, Some((1600, 900)));
    assert_eq!(s.controls.keyboard.iter().find(|k| k.0 == "cross").unwrap().1, vec!["KeyQ".to_string()]);
    assert!(Settings::parse("[audio]\nmusic = loud\n").unwrap_err().contains("line 2"));
    assert!(Settings::parse("[graphics]\nmsaa = 3\n").is_err());
    // controls errors keep their line number and name the file
    let e = Settings::parse("[audio]\nmute = true\n[keyboard]\ncross = 5\n").unwrap_err();
    assert!(e.contains("settings.toml line 4"), "{e}");
    // P22 rtgi::save_setting appends its key after the last section
    let s = Settings::parse("[launcher]\nskip = true\nglobal_ray_tracing = true\n").unwrap();
    assert_eq!((s.launcher.skip, s.global_ray_tracing), (true, Some(true)));
}

#[test]
fn p34_settings_old_controls_file_is_a_settings_file() {
    let c = Controls { deadzone: 0.4, mouse_camera: true, ..Controls::default() };
    let s = Settings::parse(&c.to_toml()).unwrap();
    assert_eq!(s.controls, c);
    assert_eq!(s.graphics, Default::default());
}

#[test]
fn p34_settings_migration_and_save() {
    let d = tmp("mig");
    let (sp, cp) = (d.join("settings.toml"), d.join("controls.toml"));
    let mut c = Controls { trigger_threshold: 0.6, ..Controls::default() };
    c.gamepad[0].1 = vec!["North".into()];
    std::fs::write(&cp, c.to_toml()).unwrap();
    let (s, e) = Settings::load_or_create(&sp, Some(&cp));
    assert!(e.is_none(), "{e:?}");
    assert_eq!(s.controls, c);
    assert!(sp.exists(), "settings.toml written");
    assert!(cp.exists(), "controls.toml left alone");
    // second load reads settings.toml (controls.toml changes no longer matter)
    std::fs::write(&cp, Controls::default().to_toml()).unwrap();
    let mut s2 = Settings::load_or_create(&sp, Some(&cp)).0;
    assert_eq!(s2.controls, c);
    s2.audio.sfx = 0.75;
    s2.save(&sp).unwrap();
    assert_eq!(Settings::load_or_create(&sp, None).0.audio.sfx, 0.75);
    // broken file: defaults + message, the old file kept as settings.toml.bad, defaults written
    std::fs::write(&sp, "[graphics]\nvsync = maybe\n").unwrap();
    let (s3, e) = Settings::load_or_create(&sp, None);
    assert_eq!(s3, Settings::default());
    let e = e.unwrap();
    assert!(e.contains("line 2") && e.contains("settings.toml.bad"), "{e}");
    assert!(std::fs::read_to_string(d.join("settings.toml.bad")).unwrap().contains("maybe"));
    assert_eq!(Settings::load_or_create(&sp, None), (Settings::default(), None));
    // R3: binary garbage (not UTF-8) the same, never a crash
    std::fs::write(&sp, [0xffu8, 0xfe, 0, 7, 0x80, b'[', 0xc3]).unwrap();
    let (s5, e) = Settings::load_or_create(&sp, None);
    assert!(s5 == Settings::default() && e.is_some());
    assert_eq!(std::fs::read(d.join("settings.toml.bad")).unwrap()[0], 0xff);
    assert_eq!(Settings::load_or_create(&sp, None).1, None);
    // no files at all: defaults written
    let d2 = tmp("new");
    let (s4, e) = Settings::load_or_create(&d2.join("settings.toml"), Some(&d2.join("controls.toml")));
    assert!(e.is_none() && s4 == Settings::default());
    let _ = std::fs::remove_dir_all(&d);
    let _ = std::fs::remove_dir_all(&d2);
}

/// R2: a settings.toml written by a newer / older build (unknown keys, sections, actions; missing
/// keys) still loads: unknown entries are ignored, missing ones keep the defaults
#[test]
fn p34_settings_other_build_file() {
    let t = "future_top = 3\ndeadzone = 0.3\n[graphics]\nrender_scale_x = 0.5\nvsync = false\n[hdr]\npeak = 1000\n[keyboard]\njump = [\"Space\"]\ncross = [\"KeyJ\"]\n[audio]\nmaster = 0.5\n";
    let s = Settings::parse(t).unwrap();
    assert_eq!((s.controls.deadzone, s.graphics.vsync, s.audio.master), (0.3, false, 0.5));
    assert_eq!(s.controls.keyboard.iter().find(|k| k.0 == "cross").unwrap().1, vec!["KeyJ".to_string()]);
    assert_eq!(s.graphics.fps_cap, Settings::default().graphics.fps_cap);
    // a default file and an empty file
    assert_eq!(Settings::parse(&Settings::default().to_toml()).unwrap(), Settings::default());
    assert_eq!(Settings::parse("").unwrap(), Settings::default());
}

/// R2: remaster env: lighting flag always explicit (the remaster build turns lighting on with
/// GAME_TEXPACK unless GAME_REMASTER_LIGHT=0), pack + light_levels.tsv next to the exe
#[test]
fn p34_settings_remaster_env() {
    use scooby_recomp::engine::settings::{Remaster, remaster_env};
    let d = tmp("remaster_env");
    std::fs::create_dir_all(d.join("texpack")).unwrap();
    std::fs::create_dir_all(d.join("remaster")).unwrap();
    std::fs::write(d.join("remaster").join("light_levels.tsv"), "x\n").unwrap();
    let get = |r: &Remaster| remaster_env(r, &d).into_iter().map(|(k, v)| (k.to_string(), v)).collect::<std::collections::HashMap<_, _>>();
    let e = get(&Remaster { lighting: false, texture_pack: true });
    assert_eq!(e["GAME_REMASTER_LIGHT"], "0");
    assert!(e["GAME_TEXPACK"].ends_with("texpack"));
    if std::env::var_os("GAME_RL_LEVELS").is_none() {
        assert!(e["GAME_RL_LEVELS"].ends_with("light_levels.tsv"));
    }
    let e = get(&Remaster { lighting: true, texture_pack: false });
    assert_eq!((e["GAME_REMASTER_LIGHT"].as_str(), e["GAME_TEXPACK"].as_str()), ("1", ""));
}

/// R3: what Play starts (launcher --print-env prints the same table): game.exe unless the remaster
/// build is installed and an option is on; lighting flag always explicit; pack off = var removed
#[test]
fn p34_settings_launch_plan() {
    use scooby_recomp::engine::settings::{Remaster, launch_plan};
    let d = tmp("launch_plan");
    std::fs::create_dir_all(d.join("texpack")).unwrap();
    let with = |lighting: bool, texture_pack: bool| Settings { remaster: Remaster { lighting, texture_pack }, ..Settings::default() };
    assert_eq!(launch_plan(&with(true, true), &d), (d.join("game.exe"), vec![]), "no remaster build: plain game");
    std::fs::write(d.join("game_remaster.exe"), "").unwrap();
    let plan = |l, p| {
        let (exe, env) = launch_plan(&with(l, p), &d);
        let env: Vec<_> = env.into_iter().filter(|e| e.0 != "GAME_RL_LEVELS").map(|(k, v)| (k, v.map(|v| if v.ends_with("texpack") { "<pack>".to_string() } else { v }))).collect();
        (exe.file_name().unwrap().to_string_lossy().into_owned(), env)
    };
    let s = |v: &str| Some(v.to_string());
    assert_eq!(plan(false, false), ("game.exe".into(), vec![]));
    assert_eq!(plan(true, false), ("game_remaster.exe".into(), vec![("GAME_REMASTER_LIGHT", s("1")), ("GAME_TEXPACK", None)]));
    assert_eq!(plan(false, true), ("game_remaster.exe".into(), vec![("GAME_REMASTER_LIGHT", s("0")), ("GAME_TEXPACK", s("<pack>"))]));
    assert_eq!(plan(true, true), ("game_remaster.exe".into(), vec![("GAME_REMASTER_LIGHT", s("1")), ("GAME_TEXPACK", s("<pack>"))]));
    let _ = std::fs::remove_dir_all(&d);
}

/// R4: exclusive fullscreen mode selection (the old Fullscreen(Current, Current) panicked
/// "Unable to get monitor." in bevy_winit): exact size at the best refresh, else the closest size,
/// None when the monitor reports no modes (-> borderless)
#[test]
fn p34_settings_pick_video_mode() {
    use scooby_recomp::engine::settings::pick_video_mode;
    let modes = [(1920, 1080, 60000), (1920, 1080, 144000), (1280, 720, 60000), (2560, 1440, 60000)];
    assert_eq!(pick_video_mode(&modes, Some((1920, 1080)), (2560, 1440)), Some(1));
    assert_eq!(pick_video_mode(&modes, None, (2560, 1440)), Some(3));
    assert_eq!(pick_video_mode(&modes, Some((1366, 768)), (2560, 1440)), Some(2));
    assert_eq!(pick_video_mode(&[], Some((1920, 1080)), (1920, 1080)), None);
    // default window mode = borderless
    assert_eq!(Settings::default().graphics.window_mode, WindowMode::Borderless);
}

/// R4: [game] iso round trip + disc validation (the real ISO when present, a wrong file otherwise)
#[test]
fn p34_settings_iso_validation() {
    use scooby_recomp::engine::settings::validate_iso;
    let mut s = Settings::default();
    s.game.iso = Some(std::path::PathBuf::from(r"D:\Games\Scooby Doo Mystery Mayhem.ISO"));
    assert_eq!(Settings::parse(&s.to_toml()).unwrap(), s);
    assert_eq!(Settings::parse("[game]\niso = \"\"\n").unwrap().game.iso, None);
    let d = tmp("iso");
    let junk = d.join("junk.iso");
    std::fs::write(&junk, vec![0u8; 40000]).unwrap();
    assert!(validate_iso(&junk).unwrap_err().contains("no ISO-9660"));
    assert!(validate_iso(&d.join("missing.iso")).unwrap_err().contains("does not exist"));
    if std::path::Path::new(crate::common::ISO).is_file() {
        let ok = validate_iso(std::path::Path::new(crate::common::ISO)).unwrap();
        assert!(ok.contains("PBPX-95506") && !ok.starts_with('!'), "{ok}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// R6: a tiny ISO-9660 image: SYSTEM.CNF booting `elf` + one file in each of `dirs` (paths like
/// "LEVEL/LOADER"), as 2048-byte sectors or raw 2352-byte (MODE2 form 1: data at +24) sectors.
fn tiny_iso(elf: &str, dirs: &[&str], raw: bool) -> Vec<u8> {
    fn rec(name: &[u8], lba: u32, size: u32, dir: bool) -> Vec<u8> {
        let len = (33 + name.len() + 1) & !1;
        let mut r = vec![0u8; len];
        r[0] = len as u8;
        r[2..6].copy_from_slice(&lba.to_le_bytes());
        r[10..14].copy_from_slice(&size.to_le_bytes());
        r[25] = if dir { 2 } else { 0 };
        r[32] = name.len() as u8;
        r[33..33 + name.len()].copy_from_slice(name);
        r
    }
    // sectors: 16 PVD, 17 root, 18 SYSTEM.CNF, then per dir: dir sector + a file sector
    let mut secs: Vec<Vec<u8>> = vec![vec![0u8; 2048]; 19];
    let cnf = format!("BOOT2 = cdrom0:\\{elf};1\r\nVER = 1.00\r\nVMODE = NTSC\r\n");
    secs[18][..cnf.len()].copy_from_slice(cnf.as_bytes());
    let mut root = [rec(&[0], 17, 2048, true), rec(&[1], 17, 2048, true), rec(b"SYSTEM.CNF;1", 18, cnf.len() as u32, false)].concat();
    for d in dirs {
        // flat: each top dir holds the nested path as one dir (enough for prefix checks)
        let (top, sub) = d.split_once('/').unwrap_or((d, ""));
        let dl = secs.len() as u32;
        secs.push(vec![0u8; 2048]);
        secs.push(b"x".repeat(2048));
        let mut body = [rec(&[0], dl, 2048, true), rec(&[1], 17, 2048, true)].concat();
        if sub.is_empty() {
            body.extend(rec(b"F.BIN;1", dl + 1, 1, false));
        } else {
            let sl = secs.len() as u32;
            secs.push(vec![0u8; 2048]);
            let sb = [rec(&[0], sl, 2048, true), rec(&[1], dl, 2048, true), rec(b"F.BIN;1", dl + 1, 1, false)].concat();
            secs[sl as usize][..sb.len()].copy_from_slice(&sb);
            body.extend(rec(sub.as_bytes(), sl, 2048, true));
        }
        secs[dl as usize][..body.len()].copy_from_slice(&body);
        root.extend(rec(top.as_bytes(), dl, 2048, true));
    }
    secs[17][..root.len()].copy_from_slice(&root);
    secs[16][0] = 1;
    secs[16][1..6].copy_from_slice(b"CD001");
    let r = rec(&[0], 17, 2048, true);
    secs[16][156..156 + 34].copy_from_slice(&r[..34]);
    if !raw {
        return secs.concat();
    }
    secs.iter().flat_map(|s| [vec![0u8; 24], s.clone(), vec![0u8; 280]].concat()).collect()
}

/// R6: disc regions + image formats + wrong files, each with a clear launcher message
#[test]
fn p34_r6_disc_formats() {
    use scooby_recomp::engine::settings::validate_iso;
    let d = tmp("r6disc");
    let all = ["LEVEL/LOADER", "GLOBAL", "STREAM", "MOVIES"];
    let put = |name: &str, b: &[u8]| {
        let p = d.join(name);
        std::fs::write(&p, b).unwrap();
        p
    };
    // the tested disc, as .iso and as a raw .bin + .cue
    let ok = validate_iso(&put("a.iso", &tiny_iso("PBPX_955.06", &all, false))).unwrap();
    assert!(ok.contains("PBPX-95506") && !ok.starts_with('!'), "{ok}");
    put("b.bin", &tiny_iso("PBPX_955.06", &all, true));
    assert!(validate_iso(&d.join("b.bin")).unwrap().contains("PBPX-95506"));
    let cue = put("b.cue", b"FILE \"b.bin\" BINARY\r\n  TRACK 01 MODE2/2352\r\n    INDEX 01 00:00:00\r\n");
    assert!(validate_iso(&cue).unwrap().contains("PBPX-95506"));
    let fs = scooby_recomp::engine::iso::IsoFs::open(&cue).unwrap();
    assert_eq!(fs.read("GLOBAL/F.BIN").unwrap(), b"x");
    assert!(validate_iso(&put("c.cue", b"FILE \"nothere.bin\" BINARY\r\n")).unwrap_err().contains("does not exist"));
    // other releases: refused with "isn't supported yet" (P39 R3), playable with an "untested" warning only
    // with SCOOBY_UNTESTED_DISC=1; other games / damaged images: errors
    use scooby_recomp::engine::settings::{check_disc_supported, validate_iso_with};
    let us_iso = put("us.iso", &tiny_iso("SLUS_207.01", &all, false));
    let e = validate_iso(&us_iso).unwrap_err();
    assert!(e.contains("This disc version (NTSC-U disc SLUS-20701) isn't supported yet") && e.contains("supported: SCEE disc PBPX-95506"), "{e}");
    assert_eq!(check_disc_supported(&us_iso).unwrap_err(), e);
    let us = validate_iso_with(&us_iso, true).unwrap();
    assert!(us.starts_with('!') && us.contains("SLUS-20701") && us.contains("untested"), "{us}");
    let pal = validate_iso_with(&put("pal.img", &tiny_iso("SLES_518.83", &all, false)), true).unwrap();
    assert!(pal.starts_with('!') && pal.contains("SLES-51883"), "{pal}");
    assert!(validate_iso(&d.join("pal.img")).unwrap_err().contains("(PAL disc SLES-51883) isn't supported yet"));
    assert_eq!(check_disc_supported(&d.join("a.iso")), Ok(()));
    let e = validate_iso(&put("other.iso", &tiny_iso("SLUS_200.62", &all, false))).unwrap_err();
    assert!(e.contains("not Scooby-Doo") && e.contains("SLUS_200.62") && e.contains("PBPX-95506"), "{e}");
    let e = validate_iso(&put("dmg.iso", &tiny_iso("PBPX_955.06", &all[..3], false))).unwrap_err();
    assert!(e.contains("MOVIES/ is missing"), "{e}");
    // archives / compressed formats / tiny files
    let mut zip = b"PK\x03\x04".to_vec();
    zip.resize(5000, 0);
    assert!(validate_iso(&put("z.zip", &zip)).unwrap_err().contains("extract the .iso"));
    let mut sz = b"7z\xbc\xaf\x27\x1c".to_vec();
    sz.resize(5000, 0);
    assert!(validate_iso(&put("z.iso", &sz)).unwrap_err().contains(".7z archive"));
    let mut chd = b"MComprHD".to_vec();
    chd.resize(5000, 0);
    assert!(validate_iso(&put("z.chd", &chd)).unwrap_err().contains("convert it to a plain .iso"));
    assert!(validate_iso(&put("t.iso", b"abc")).unwrap_err().contains("too small"));
    // read-only file: still fine (the game only reads the image)
    let ro = put("ro.iso", &tiny_iso("PBPX_955.06", &all, false));
    let mut perm = std::fs::metadata(&ro).unwrap().permissions();
    perm.set_readonly(true);
    std::fs::set_permissions(&ro, perm.clone()).unwrap();
    assert!(validate_iso(&ro).is_ok());
    perm.set_readonly(false);
    std::fs::set_permissions(&ro, perm).unwrap();
    let _ = std::fs::remove_dir_all(&d);
}

/// R7: fresh profile defaults (skip launcher OFF), the remembered tab round-trips, and the
/// launcher <-> in-game consistency: a launcher that was open while the game's Options menu
/// changed the file keeps those changes on Save (three-way merge per section), its own edits win
/// in the sections it touched.
#[test]
fn p34_r7_fresh_defaults_and_merge() {
    let d = tmp("r7");
    let p = d.join("settings.toml");
    let (fresh, err) = Settings::load_or_create(&p, None);
    assert!(err.is_none(), "{err:?}");
    assert!(!fresh.launcher.skip, "Skip launcher must default to off");
    assert_eq!(fresh.launcher.tab, 0);
    let mut t = fresh.clone();
    t.launcher.tab = 2;
    assert_eq!(Settings::parse(&t.to_toml()).unwrap().launcher.tab, 2);
    assert_eq!(Settings::parse("[launcher]\ntab = 9\n").unwrap().launcher.tab, 3);

    // launcher opens (base), the game changes audio + controls in the file meanwhile
    let base = Settings::parse(&std::fs::read_to_string(&p).unwrap()).unwrap();
    let mut game = base.clone();
    game.audio.music = 0.25;
    game.controls.invert_y = !base.controls.invert_y;
    game.save(&p).unwrap();
    // the launcher only changed graphics, then Save
    let mut mine = base.clone();
    mine.graphics.vsync = !base.graphics.vsync;
    let disk = Settings::parse(&std::fs::read_to_string(&p).unwrap()).unwrap();
    let out = mine.merged(&base, &disk);
    assert_eq!(out.graphics.vsync, mine.graphics.vsync, "own edit kept");
    assert_eq!(out.audio.music, 0.25, "in-game audio change not overwritten");
    assert_eq!(out.controls.invert_y, game.controls.invert_y, "in-game controls change not overwritten");
    // same section edited on both sides: the saver (last writer) wins for that section
    let mut mine2 = base.clone();
    mine2.audio.sfx = 0.5;
    let out2 = mine2.merged(&base, &disk);
    assert_eq!((out2.audio.sfx, out2.audio.music), (0.5, base.audio.music));
    // nothing changed anywhere: identity
    assert_eq!(base.merged(&base, &base), base);
}

/// R7: crash detection in the launcher = this run's own crash records only
#[test]
fn p34_r7_crash_records() {
    use scooby_recomp::engine::settings::crash_records;
    let tail = "=== UNCLEAN EXIT pid 41: the run ended without a clean exit\n  version x\n\
=== SAVES MIGRATED pid 42: a -> b\n\
=== PANIC pid 42 thread Some(\"main\"): index out of bounds\n  level EP2/L03\n\
=== PANIC pid 420 thread None: other process\n  x\n\
=== CONSOLE EXIT pid 42: closed\n  x\n";
    let r = crash_records(tail, 42);
    assert_eq!(r.len(), 1, "{r:?}");
    assert!(r[0].starts_with("PANIC pid 42 ") && r[0].ends_with("\nlevel EP2/L03"), "{}", r[0]);
    assert!(crash_records("=== SAVES MIGRATED pid 42: a -> b\n", 42).is_empty());
    assert_eq!(crash_records(tail, 41).len(), 1);
    let ex = crash_records("=== EXCEPTION pid 7 code 0xc0000005 at 0x1 = x access addr 0x0 thread None\n  ctx\n", 7);
    assert_eq!(ex.len(), 1);
}

/// P34 R8: `[game] pause_on_focus_loss` (user decision Nov 6): default OFF, round-trips, old
/// files without the key load as OFF, and the launcher's disc change does not revert an in-game
/// toggle of the flag (per-key merge in [game]).
#[test]
fn p34_r8_pause_on_focus_loss() {
    let d = tmp("r8");
    let p = d.join("settings.toml");
    let (fresh, err) = Settings::load_or_create(&p, None);
    assert!(err.is_none(), "{err:?}");
    assert!(!fresh.game.pause_on_focus_loss, "must default to off");
    let text = std::fs::read_to_string(&p).unwrap();
    assert!(text.contains("pause_on_focus_loss = false"), "{text}");
    assert!(!Settings::parse("[game]\niso = \"x.iso\"\n").unwrap().game.pause_on_focus_loss);
    let mut on = fresh.clone();
    on.game.pause_on_focus_loss = true;
    assert!(Settings::parse(&on.to_toml()).unwrap().game.pause_on_focus_loss);
    assert!(Settings::parse("[game]\npause_on_focus_loss = maybe\n").is_err() || !Settings::parse("[game]\npause_on_focus_loss = maybe\n").unwrap().game.pause_on_focus_loss);
    // launcher open (base); the game turns the flag on; the launcher picks another disc + Save
    let base = fresh.clone();
    on.save(&p).unwrap();
    let mut mine = base.clone();
    mine.game.iso = Some("D:/Scooby.iso".into());
    let disk = Settings::parse(&std::fs::read_to_string(&p).unwrap()).unwrap();
    let out = mine.merged(&base, &disk);
    assert!(out.game.pause_on_focus_loss, "in-game toggle kept");
    assert_eq!(out.game.iso, mine.game.iso, "own disc kept");
}

/// P34 R9: the in-game save merges per KEY: a launcher change to another key of the same section
/// (and keys the game never touched) survive; pc_key_prompts round-trips (default off).
#[test]
fn p34_r9_merge_per_key_and_key_prompts() {
    let base = Settings::default();
    assert!(!base.game.pc_key_prompts && !base.game.pause_on_focus_loss);
    // game changed master volume + pc_key_prompts
    let mut game = base.clone();
    game.audio.master = 0.5;
    game.game.pc_key_prompts = true;
    // launcher meanwhile changed music (same section) + pause flag (same [game] section) + rtgi
    let mut disk = base.clone();
    disk.audio.music = 0.3;
    disk.game.pause_on_focus_loss = true;
    disk.global_ray_tracing = Some(true);
    disk.controls.deadzone = 0.4;
    let m = game.merged_keys(&base, &disk);
    assert_eq!(m.audio.master, 0.5);
    assert_eq!(m.audio.music, 0.3);
    assert!(m.game.pc_key_prompts && m.game.pause_on_focus_loss);
    assert_eq!(m.global_ray_tracing, Some(true));
    assert_eq!(m.controls.deadzone, 0.4);
    // own change of the same key wins
    let mut g2 = base.clone();
    g2.audio.music = 0.9;
    assert_eq!(g2.merged_keys(&base, &disk).audio.music, 0.9);
    // round trip
    assert_eq!(Settings::parse(&m.to_toml()).unwrap(), m);
}

#[test]
fn p34_r9_bug_report_url_from_readme() {
    let t = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../tools/dist/README.txt")).unwrap();
    assert!(t.contains("Project page (GitHub):"));
    let f = |s: &str| {
        let u = s.lines().find_map(|l| l.trim().strip_prefix("Project page (GitHub):"))?.split_whitespace().next()?;
        u.starts_with("http").then(|| format!("{}/issues", u.trim_end_matches('/')))
    };
    assert_eq!(f(&t.replace("{{GITHUB_URL}}", "https://github.com/a/b/")).as_deref(), Some("https://github.com/a/b/issues"));
    assert_eq!(f(&t), None);
    // P40 R11: the README line holds the URL alone (a rewrap once joined the next sentence onto it)
    let line = t.lines().find(|l| l.starts_with("Project page (GitHub):")).unwrap();
    assert_eq!(line.trim_end(), "Project page (GitHub): {{GITHUB_URL}}");
    assert_eq!(f("Project page (GitHub): https://x/y Bug reports").as_deref(), Some("https://x/y/issues"));
}

/// P34 R15: a 7-row PC Settings tab (CONTROLS) ends on the 6-row tab's last spot, clear of the panel's
/// bottom-left flower (menu audit 640x480 / 1280x720 / 2560x1080: PC KEY PROMPTS was under it)
#[test]
fn p34_pc_rows_fit_panel() {
    use scooby_recomp::modules::panel::pc_row_step;
    assert_eq!(pc_row_step(6), 1.0);
    assert_eq!(3.0 * pc_row_step(6), 4.0 * pc_row_step(7));
}

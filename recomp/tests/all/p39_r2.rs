//! P39 R2 (robustness): (1) a damaged memory card file must give the game's own "corrupted" path (load error,
//! no level started), never a host panic; (2) a load-only sweep of every level pack on the disc + 60 frames, and
//! the demux / audio decode of every movie, with each panic collected (not just the first).
use crate::common::{ISO, load_carried};
use crate::engine_p17::{CARD, run};
use scooby_recomp::engine::iso::IsoFs;
use std::panic::{AssertUnwindSafe, catch_unwind};

fn panic_text(p: Box<dyn std::any::Any + Send>) -> String {
    p.downcast_ref::<String>().cloned().or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default()
}

/// one card: `BASLUS-20701/` with the disc's icons + `save` as the save file
fn make_card(fs: &IsoFs, dir: &std::path::Path, save: &[u8]) {
    let _ = std::fs::remove_dir_all(dir);
    let d = dir.join("BASLUS-20701");
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("BASLUS-20701"), save).unwrap();
    std::fs::write(d.join("icon.sys"), fs.read("ICON.SYS").unwrap()).unwrap();
    std::fs::write(d.join("static.ico"), fs.read("STATIC.ICO").unwrap()).unwrap();
}

/// P39 R5: the R2 card cases that R4's slot-list test leaves out (a 4-byte CRC-only file, an all-zero file,
/// a 4 MB oversized file), now through the R4 slot-list macro (R2's `menuload` macro never reached "Load").
/// crc only -> the damaged-data dialog; zeros (CRC 0 over zero bytes: valid for the original too) and huge
/// (the first 14988 bytes are a real save) may load; every case: no panic, the card file untouched.
#[test]
#[ignore]
fn p39_r2_corrupt_card_load() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let good = std::fs::read(format!("{}/tests/all/data/p39_card_save.bin", env!("CARGO_MANIFEST_DIR"))).unwrap();
    let cases: Vec<(&str, Vec<u8>, Expect)> = vec![
        ("crc_only", good[..4].to_vec(), Expect::Damaged),
        ("zeros", vec![0; good.len()], Expect::Either),
        ("huge", [good.clone(), vec![0xcd; 4 << 20]].concat(), Expect::Either),
    ];
    slot_list_cases(&fs, "scooby_p39r5_card", cases);
}

#[test]
#[ignore]
fn p39_r2_load_all_levels() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let _card = CARD.lock().unwrap_or_else(|e| e.into_inner());
    let card = std::env::temp_dir().join(format!("scooby_p39_sweep_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&card);
    // SAFETY: tests touching GAME_SAVEDIR hold CARD; only memory-card init reads it
    unsafe { std::env::set_var("GAME_SAVEDIR", &card) };
    let mut levels: Vec<String> = fs
        .names()
        .filter_map(|n| n.strip_prefix("LEVEL/")?.split_once("/GEN/").filter(|(_, f)| f.ends_with(".P2R")).map(|(d, _)| d.to_string()))
        .collect();
    levels.sort();
    levels.dedup();
    if let Ok(only) = std::env::var("P39_LEVELS") {
        levels.retain(|l| only.split(',').any(|o| l.starts_with(o)));
    }
    let frames = std::env::var("P39_FRAMES").unwrap_or_else(|_| "60".into());
    let mut bad = vec![];
    for lvl in &levels {
        let t = std::time::Instant::now();
        let r = catch_unwind(AssertUnwindSafe(|| {
            let (mut e, mut rt) = load_carried(&fs, lvl, &[])?;
            Ok::<_, String>(run(&mut e, &mut rt, &format!("f {frames}")))
        }));
        let s = match r {
            Err(p) => format!("PANIC {}", panic_text(p)),
            Ok(Err(e)) => format!("LOAD ERROR {e}"),
            Ok(Ok(r)) => format!("ok ({r:?})"),
        };
        println!("P39LOAD {lvl}: {s} in {:.1} s", t.elapsed().as_secs_f32());
        if !s.starts_with("ok") {
            bad.push(format!("{lvl}: {s}"));
        }
    }
    // movies: demux + audio decode of every PSS
    let movies: Vec<String> = fs.names().filter(|n| n.ends_with(".PSS")).map(str::to_string).collect();
    for m in &movies {
        let r = catch_unwind(|| {
            let d = fs.read(m).map_err(|e| format!("{e:?}"))?;
            let p = scooby_recomp::engine::video::demux_pss(&d);
            let fps = scooby_recomp::engine::video::video_fps(&p.video_es);
            let a = scooby_recomp::engine::video::decode_audio(&p, 0).map_or(0, |a| a.1.len());
            Ok::<_, String>(format!("{} KB video, {fps} fps, {a} samples", p.video_es.len() / 1024))
        });
        let s = match r {
            Err(p) => format!("PANIC {}", panic_text(p)),
            Ok(Err(e)) => format!("READ ERROR {e}"),
            Ok(Ok(s)) => s,
        };
        println!("P39PSS {m}: {s}");
        if s.contains("PANIC") || s.contains("ERROR") {
            bad.push(format!("{m}: {s}"));
        }
    }
    println!("P39 sweep: {} levels, {} movies, {} problems", levels.len(), movies.len(), bad.len());
    let _ = std::fs::remove_dir_all(&card);
    assert!(bad.is_empty(), "P39 load sweep: {bad:#?}");
}

/// P39 R4: the HUD texts currently set on text windows (decoded: the game's text bytes are ASCII - 0x32)
fn hud_texts(rt: &scooby_recomp::engine::runtime::EngineRuntime) -> Vec<(u32, String)> {
    let mut p = rt.particles.lock().unwrap_or_else(|e| e.into_inner());
    let ids: Vec<u32> = p.hud.slots.iter_mut().filter_map(|(_, w)| w.as_mut()?.text().map(|t| t.text_id)).collect();
    let dec = |id: u32| -> String {
        let b = p.hud.text.as_ref().and_then(|t| t.GetText(id)).unwrap_or(&[]);
        b.iter().take(120).filter(|c| (0x2f..=0x48).contains(*c)).map(|&c| (c + 0x32) as char).collect::<String>()
    };
    ids.into_iter().map(|id| (id, dec(id))).collect()
}

/// P39 R4 the title Load slot list on a damaged card: INTRO -> Start -> "load a game" -> X -> (checking memory
/// card) -> slot list -> down to slot 2 (this card's save, "gold mountain hotel main street") -> X -> "reload?"
/// -> down (yes) -> X. good: the load starts (a level request); damaged: the original's damaged-data flow
/// (no load, no panic, the card file untouched). `P39_MACRO` overrides the input steps (tuning).
/// R4 run: good -> "reload your last save?" -> yes -> "loading data" -> level request; xor64 / truncated / garbage
/// -> text 0xb5 (damaged save data) + Retry / Cancel; empty (0 bytes) loaded as an all-zero save until the
/// sceMcRead short-read fix (card_read).
#[test]
#[ignore]
fn p39_r4_card_slot_list() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let good = std::fs::read(format!("{}/tests/all/data/p39_card_save.bin", env!("CARGO_MANIFEST_DIR"))).unwrap();
    let mut xor = good.clone();
    for b in &mut xor[200..264] {
        *b ^= 0x5a;
    }
    let cases: Vec<(&str, Vec<u8>, Expect)> = vec![
        ("good", good.clone(), Expect::Load),
        ("xor64", xor, Expect::Damaged),
        ("truncated", good[..good.len() / 3].to_vec(), Expect::Damaged),
        ("empty", vec![], Expect::Damaged),
        ("garbage", (0..good.len() as u32).map(|i| (i.wrapping_mul(2654435761) >> 11) as u8).collect(), Expect::Damaged),
    ];
    slot_list_cases(&fs, "scooby_p39r4_card", cases);
}

#[derive(PartialEq, Debug)]
enum Expect {
    Load,
    Damaged,
    Either,
}

/// each card through the title Load slot-list macro: the expected outcome, no panic, the card file untouched
fn slot_list_cases(fs: &IsoFs, tag: &str, cases: Vec<(&str, Vec<u8>, Expect)>) {
    let _card = CARD.lock().unwrap_or_else(|e| e.into_inner());
    let steps = std::env::var("P39_MACRO").unwrap_or_else(|_| {
        "f 100; p 8 3; f 100; s 80 ff 10; f 30; p 4000 3; f 300; s 80 ff 10; f 30; p 4000 3; f 300; s 80 ff 10; f 30; p 4000 3; f 900".into()
    });
    let tmp = std::env::temp_dir().join(format!("{tag}_{}", std::process::id()));
    let mut bad = vec![];
    for (what, bytes, want) in cases {
        if std::env::var("P39_CARD").is_ok_and(|o| o != what) {
            continue;
        }
        let card = tmp.join(what);
        make_card(fs, &card, &bytes);
        // SAFETY: tests touching GAME_SAVEDIR hold CARD; only memory-card init reads it
        unsafe { std::env::set_var("GAME_SAVEDIR", &card) };
        let r = catch_unwind(AssertUnwindSafe(|| {
            let (mut e, mut rt) = load_carried(fs, "INTRO", &[]).unwrap();
            let r = run(&mut e, &mut rt, &steps);
            (r, hud_texts(&rt))
        }));
        match r {
            Err(p) => bad.push(format!("{what}: PANIC {}", panic_text(p))),
            Ok((r, texts)) => {
                let msg: Vec<String> = texts.iter().filter(|t| t.1.len() > 12).map(|t| format!("{:#x} {}", t.0, t.1)).collect();
                println!("P39R4CARD {what}: request {r:?}\n  texts: {}", msg.join("\n         "));
                // text 0xb5 "the save game data for Scooby-Doo Mystery Mayhem on the memory card ..." + Retry / Cancel
                let damaged = texts.iter().any(|t| t.0 == 0xb5);
                if want == Expect::Load && r.is_none() {
                    bad.push(format!("{what}: the slot-list macro did not start the load"));
                }
                if want == Expect::Damaged && (r.is_some() || !damaged) {
                    bad.push(format!("{what}: expected the damaged-data dialog, got request {r:?} dialog {damaged}"));
                }
                println!("P39R4CARD {what}: damaged-data text on screen: {damaged} (expected {want:?})");
            }
        }
        assert_eq!(std::fs::read(card.join("BASLUS-20701/BASLUS-20701")).unwrap(), bytes, "{what}: card file changed");
    }
    let _ = std::fs::remove_dir_all(&tmp);
    assert!(bad.is_empty(), "P39 card slot list ({tag}): {bad:?}");
}

//! P35 R4 (full-game QA): EP3/L03 cart track-change trigger under GAME_LOGICCLIP vs default (diagnostic).
use crate::common::{ISO, leader, load_carried, pos};
use crate::engine_p17::{routes_ep3, run};
use scooby_recomp::engine::iso::IsoFs;

/// Replays the default-mode cart inputs (EP3_L03_REAL) and, from P35_FROM (default 9700) to P35_TO (10100) step
/// frames, prints every change of the hs_changeTrack objects (state, logic flags, clip word / logic-clip / physic
/// and anim clip-off) and of the cart spline (mcart local +0x100), to compare GAME_LOGICCLIP=1 with the default.
#[test]
#[ignore]
fn p35_r4_cart_changetrack_clip() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = load_carried(&fs, "EP3/L03", &[]).unwrap();
    let from: u32 = std::env::var("P35_FROM").ok().and_then(|v| v.parse().ok()).unwrap_or(9700);
    let to: u32 = std::env::var("P35_TO").ok().and_then(|v| v.parse().ok()).unwrap_or(10100);
    let data = e.rt.codes.iter().find(|c| &*c.def.name == "mcart").expect("mcart loaded").data;
    let ct: Vec<u32> = e.rt.objects.iter().flatten().filter(|o| o.inst.code.is_some_and(|c| e.rt.codes[c].def.name.contains("changeTrack"))).map(|o| o.id).collect();
    println!("P35R4 clip={} changeTrack objects {ct:x?}", rt.logic_clip.is_some());
    let mut last: Vec<String> = vec![String::new(); ct.len() + 1];
    let mut f = 0u32;
    for st in routes_ep3::EP3_L03_REAL.split("; ") {
        let r = run(&mut e, &mut rt, st);
        f += st.split_whitespace().last().and_then(|x| x.parse::<u32>().ok()).unwrap_or(0);
        if let Some(r) = r { println!("P35R4 f{f} LEVEL {r:?}"); return; }
        if f < from { continue; }
        if f > to { break; }
        let l = leader(&e);
        let cl = e.mem.r32(data + 0x38);
        let s = format!("spline {:#x} acc {:#x}/{:#x}", e.mem.r32(cl + 0x100), e.mem.r32(cl + 0x90), e.mem.r32(cl + 0x94));
        if s != last[0] { println!("P35R4 f{f} [{st}] cart {s} at {:?}", pos(&mut e, &rt, l)); last[0] = s; }
        for (i, &u) in ct.iter().enumerate() {
            let lf = e.rt.objects.iter().flatten().find(|o| o.id == u).map_or(0, |o| o.logic_flags);
            let cw = { let g = rt.particles.lock().unwrap(); (g.s1.clip_cur.get(&u).copied(), rt.logic_clip.as_ref().map(|l| l.contains(&u)), g.sub.physic.clip_off.contains(&u), g.sub.anim_clip_off.contains(&u)) };
            let s = format!("state {} lflags {lf:#x} clip {cw:x?}", scooby_recomp::engine::core::apigameobj::GetCurrentStateID(&mut e, u));
            if s != last[i + 1] { println!("P35R4 f{f} ct {u:#x} {s} at {:?}", pos(&mut e, &rt, u)); last[i + 1] = s; }
        }
    }
}

/// P35 R4: EP1 end save. The real EP1 end menus (LOADER -> STATUS save -> SPLASH -> LOADER) from the EP1/L04 end carry
/// on a fresh card must hand over to EP2/L01A, write the save and unlock episode 2 (GB2+0x28 bit 0x20).
/// P35_CARD_OUT=<dir> keeps a copy of the card (for a game.exe Load Game -> EP2 start check).
/// P35 R6: P35_END_CARRY=<LOADER carry of another episode end> + P35_END_WANT=<first level, e.g. EP4/L01> run the same
/// menus for that episode end (the EP2 unlock-bit check is then skipped).
/// P35 R8: P35_END_WANT=INTRO for the EP6 end (LOADER -> STATUS save -> CREDITS played to the end -> INTRO).
#[test]
#[ignore]
fn p35_r4_ep1_end_save() {
    use crate::common::load_dump;
    use crate::engine_p17::CARD;
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let _card = CARD.lock().unwrap_or_else(|e| e.into_inner());
    let card = std::env::temp_dir().join(format!("scooby_p35_ep1end_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&card);
    // SAFETY: tests touching GAME_SAVEDIR hold CARD; only memory-card init reads it
    unsafe { std::env::set_var("GAME_SAVEDIR", &card) };
    let end = std::env::var("P35_END_CARRY").ok();
    let want = std::env::var("P35_END_WANT").unwrap_or_else(|_| "EP2/L01A".into());
    let mut bufs = load_dump(&end.clone().unwrap_or_else(|| format!("{}/tests/all/data/ep1_chain_EP1L04_LOADER.bin", env!("CARGO_MANIFEST_DIR"))));
    let mut lvl = String::from("LOADER");
    for _ in 0..8 {
        if lvl.to_ascii_uppercase().starts_with("EP") || lvl.eq_ignore_ascii_case("INTRO") { break }
        // P35 R8 (EP6 end): CREDITS run untouched to their end (C Panel.c scroll timer 192 s -> LoadLevel("INTRO"))
        let steps = if lvl.eq_ignore_ascii_case("CREDITS") { "f 20000".to_string() } else { "f 282; s 80 ff 6; f 6; p 4000 6; ".repeat(30) };
        let (mut e, mut rt) = load_carried(&fs, &lvl, &bufs).unwrap();
        let (l, f) = run(&mut e, &mut rt, &steps).unwrap_or_else(|| panic!("{lvl}: no level request"));
        if lvl.eq_ignore_ascii_case("CREDITS") { assert!((11400..11700).contains(&f), "credits cut short: f{f} (192 s = 11520)") }
        println!("P35EP1END {lvl} -> {l} at f{f}");
        bufs = crate::common::carry(&mut e);
        lvl = l;
    }
    let saved = card.join("BASLUS-20701/BASLUS-20701").is_file();
    let ul = bufs.iter().find(|b| b.0 == 2).map_or(0, |b| u32::from_le_bytes(b.1[0x28..0x2c].try_into().unwrap()));
    println!("P35EP1END -> {lvl}, saved {saved}, GB2+0x28 {ul:#x}");
    if let Ok(d) = std::env::var("P35_CARD_OUT") {
        fn cp(a: &std::path::Path, b: &std::path::Path) {
            let _ = std::fs::create_dir_all(b);
            for f in std::fs::read_dir(a).into_iter().flatten().flatten() {
                if f.path().is_dir() { cp(&f.path(), &b.join(f.file_name())) } else { let _ = std::fs::copy(f.path(), b.join(f.file_name())); }
            }
        }
        cp(&card, std::path::Path::new(&d));
    }
    let _ = std::fs::remove_dir_all(&card);
    assert!(lvl.eq_ignore_ascii_case(&want), "episode end menus -> {lvl} (want {want})");
    assert!(saved, "EP1 end: no save on the card");
    assert!(end.is_some() || ul & 0x20 != 0, "EP1 end: episode 2 not unlocked (GB2+0x28 {ul:#x})");
}

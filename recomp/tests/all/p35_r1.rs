//! P35 R1 (full-game QA): checkpoint save -> pause Quit -> title Load -> the loaded level, for the EP1 and EP3
//! levels p22_save_load_roundtrip does not cover. Checks what the player sees after Continue: the level loads,
//! does not leave at once, the hero stands at the checkpoint he saved at (not at the level start), and the saved
//! game words GB1/GB2 come back unchanged.
use crate::common::{ISO, carry, leader, load_carried, load_dump, pos, uid};
use crate::engine_p17::{CARD, run};
use scooby_recomp::engine::iso::IsoFs;

fn dist(a: [f32; 3], b: [f32; 3]) -> f32 { ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt() }

#[test]
#[ignore]
fn p35_r1_checkpoint_save_load_ep1_ep3() {
    // (level, entry carry under tests/all/data); EP3/L02: the warp skips the walk into camera area 1 whose first
    // visit plays a cut-scene and sets GB2+0x1c bit 4 (camera__46dad128 camera.c:93-99, msg 0x52); a save made
    // without it replays that cut on load (hero moved to ~270,277.7 for the walk-in), as the C would, so mark it
    // seen as the real walk does
    const CASES: &[(&str, &str)] = &[
        ("EP1/L02", "p32_r79_EP1L02"),
        ("EP1/L03A", "p32_r79_EP1L03A"),
        ("EP1/L03B", "p32_r79_EP1L03B"),
        ("EP1/L04", "ep1_chain_EP1L04"),
        // EP3/L02: entry carry of a P17_EP=3 p17_chains run (P17_SAVE, P35 R1); EP3/L03 (cart) and L05 (boss) have
        // no checkpoint object
        ("EP3/L02", "p35_r1_EP3L02"),
        ("EP3/L04", "ep3_l04_realcarry"),
    ];
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let _card = CARD.lock().unwrap_or_else(|e| e.into_inner());
    let only = std::env::var("P35_SAVELOAD").ok();
    let tmp = std::env::temp_dir().join(format!("scooby_p35_sl_{}", std::process::id()));
    let mut bad = vec![];
    for &(lvl, entry) in CASES {
        if only.as_deref().is_some_and(|o| o != lvl) { continue }
        let card = tmp.join(lvl.replace('/', "_"));
        let _ = std::fs::remove_dir_all(&card);
        // SAFETY: tests touching GAME_SAVEDIR hold CARD; only memory-card init reads it
        unsafe { std::env::set_var("GAME_SAVEDIR", &card) };
        let bufs = load_dump(&format!("{}/tests/all/data/{entry}.bin", env!("CARGO_MANIFEST_DIR")));
        let (mut e, mut rt) = load_carried(&fs, lvl, &bufs).unwrap();
        if let Some(r) = run(&mut e, &mut rt, "f 200") { bad.push(format!("{lvl}: left at once {r:?}")); continue }
        let has_cp = e.rt.objects.iter().flatten().any(|o| o.inst.code.is_some_and(|k| e.rt.codes[k].def.name.to_lowercase().starts_with("chkpnt")));
        if !has_cp { println!("P35SL {lvl}: no checkpoint object"); continue }
        let cp = uid(&e, "@chkpnt");
        let cpat = pos(&mut e, &rt, cp);
        println!("P35SL {lvl}: checkpoint {cp:#x} at {cpat:?}");
        let a = card.join("a.bin");
        let pre = if lvl == "EP3/L02" { "gor 2 1c 4; " } else { "" };
        let r = run(&mut e, &mut rt, &format!("{pre}warp {cp:x}; f 60; save; gbdump {}", a.display()));
        let l = leader(&e);
        let at = pos(&mut e, &rt, l).unwrap_or_default();
        let r2 = r.or_else(|| run(&mut e, &mut rt, "f 470; pausequit"));
        let file = card.join("BASLUS-20701/BASLUS-20701");
        println!("P35SL {lvl}: checkpoint {cp:#x}, saved at {at:?}, save+quit -> {r2:?}, card file {}", file.is_file());
        if !matches!(&r2, Some((l, _)) if l == "INTRO") || !file.is_file() { bad.push(format!("{lvl}: save/quit {r2:?} file {}", file.is_file())); continue }
        let (mut e, mut rt) = load_carried(&fs, "INTRO", &[]).unwrap();
        let r = run(&mut e, &mut rt, "menuload");
        println!("P35SL {lvl}: menuload -> {r:?}");
        if !matches!(&r, Some((l, _)) if l.eq_ignore_ascii_case(lvl)) { bad.push(format!("{lvl}: load -> {r:?}")); continue }
        let c = carry(&mut e);
        let (mut e, mut rt) = load_carried(&fs, lvl, &c).unwrap();
        let b = card.join("b.bin");
        if let Some(r) = run(&mut e, &mut rt, &format!("f 260; gbdump {}", b.display())) { bad.push(format!("{lvl}: loaded level left at once {r:?}")); continue }
        let l = leader(&e);
        let now = pos(&mut e, &rt, l).unwrap_or_default();
        let d = dist(at, now);
        println!("P35SL {lvl}: after load hero {l:#x} at {now:?} ({d:.2} m from the save spot)");
        if d > 4.0 { bad.push(format!("{lvl}: hero {d:.1} m from the checkpoint after load ({now:?} vs {at:?})")) }
        let (ga, gb) = (load_dump(&a.to_string_lossy()), load_dump(&b.to_string_lossy()));
        for id in [1u32, 2] {
            let (Some(x), Some(y)) = (ga.iter().find(|g| g.0 == id), gb.iter().find(|g| g.0 == id)) else { bad.push(format!("{lvl}: GB{id} missing")); continue };
            // GB2+0 bit 0x20000 = save prompt on screen (UI state, as p22_save_load_roundtrip; after the load the
            // hero stands on the checkpoint, so its prompt is up again); GB1+0xc = the radar manager's per-scan
            // "no enemy near" flag (rdrmngr.rs: 1, then 0 on the first enemy found), live state, not saved progress
            let (mut x, mut y) = (x.clone(), y.clone());
            if id == 2 { x.1[2] &= !2; y.1[2] &= !2 }
            if id == 1 { x.1[0xc..0x10].fill(0); y.1[0xc..0x10].fill(0) }
            let d: Vec<String> = (0..x.1.len().min(y.1.len()) / 4).filter(|&i| x.1[4 * i..4 * i + 4] != y.1[4 * i..4 * i + 4])
                .map(|i| format!("+{:#x} {:08x}->{:08x}", 4 * i, u32::from_le_bytes(x.1[4 * i..4 * i + 4].try_into().unwrap()), u32::from_le_bytes(y.1[4 * i..4 * i + 4].try_into().unwrap()))).collect();
            println!("P35SL {lvl}: GB{id} {} words differ {}", d.len(), d.join(" "));
            if !d.is_empty() { bad.push(format!("{lvl}: GB{id} {}", d.join(" "))) }
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
    assert!(bad.is_empty(), "P35 checkpoint save/load: {bad:?}");
}

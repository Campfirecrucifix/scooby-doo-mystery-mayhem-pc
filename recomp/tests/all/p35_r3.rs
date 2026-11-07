//! P35 R3 (full-game QA): EP3/L03 mine-cart ride diagnostics (GAME_LOGICCLIP from the environment).
use crate::common::{ISO, leader, load_carried, pos};
use crate::engine_p17::{routes_ep3, run};
use scooby_recomp::engine::iso::IsoFs;

#[test]
#[ignore]
fn p35_r3_ep3_l03_cart_trace() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = load_carried(&fs, "EP3/L03", &[]).unwrap();
    let mut f = 0u32;
    for st in routes_ep3::EP3_L03_REAL.split("; ") {
        let r = run(&mut e, &mut rt, st);
        f += st.split_whitespace().last().and_then(|x| x.parse::<u32>().ok()).unwrap_or(0);
        let l = leader(&e);
        eprintln!("STEP f~{f} {st} pos {:?}", pos(&mut e, &rt, l));
        if let Some(r) = r { eprintln!("LEVEL {r:?}"); return; }
    }
    panic!("no level switch");
}

/// P35 R3: checkpoint save on a full memory card (EP1/L02): the save must fail without writing the save file and
/// without leaving the player stuck in the dialog (pause -> Quit still reaches the title afterwards).
#[test]
#[ignore]
fn p35_r3_full_card_save() {
    use crate::common::{load_dump, uid};
    use crate::engine_p17::CARD;
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let _card = CARD.lock().unwrap_or_else(|e| e.into_inner());
    let card = std::env::temp_dir().join(format!("scooby_p35_full_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&card);
    // 8000 KB card: another game's 7990 KB save leaves less than the save needs
    std::fs::create_dir_all(card.join("BASLUS-99999")).unwrap();
    std::fs::write(card.join("BASLUS-99999/fill.bin"), vec![0u8; 7990 * 1024]).unwrap();
    // SAFETY: tests touching GAME_SAVEDIR hold CARD; only memory-card init reads it
    unsafe { std::env::set_var("GAME_SAVEDIR", &card) };
    let bufs = load_dump(&format!("{}/tests/all/data/p32_r79_EP1L02.bin", env!("CARGO_MANIFEST_DIR")));
    let (mut e, mut rt) = load_carried(&fs, "EP1/L02", &bufs).unwrap();
    assert!(run(&mut e, &mut rt, "f 200").is_none());
    let cp = uid(&e, "@chkpnt");
    let r = run(&mut e, &mut rt, &format!("warp {cp:x}; f 60; save; card; f 470; pausequit"));
    let file = card.join("BASLUS-20701/BASLUS-20701");
    println!("P35FULL save+quit -> {r:?}, save file {}", file.is_file());
    let _ = std::fs::remove_dir_all(&card);
    assert!(!file.is_file(), "save written to a full card");
    assert!(matches!(&r, Some((l, _)) if l == "INTRO"), "stuck after the full-card save: {r:?}");
}

/// P35 R3: EP3/L03 cart ride by a closed-loop driver (no fixed timeline): each frame it reads the cart
/// (mcart local data: +0x90 accident on, +0x94 accident kind, +0x100 spline) and its position. On an accident it
/// marks the spot a lead of L frames before it on the current ride, and on the retry (the C restarts the ride at
/// the last checkpoint, mineCartFUNC_RestartRide) it acts there: jump (X), duck (R1/0x2000) or lean left/right,
/// cycling actions and leads while the same spot keeps failing. P35_CART_FRAMES caps the run (default 60000).
#[test]
#[ignore]
fn p35_r3_ep3_l03_cart_reactive() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = load_carried(&fs, "EP3/L03", &[]).unwrap();
    let cap: usize = std::env::var("P35_CART_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(60000);
    let data = e.rt.codes.iter().find(|c| &*c.def.name == "mcart").expect("mcart loaded").data;
    let st = |e: &scooby_recomp::engine::Engine| { let l = e.mem.r32(data + 0x38); (e.mem.r32(l + 0x90), e.mem.r32(l + 0x94), e.mem.r32(l + 0x100)) };
    const ACTS: [&str; 4] = ["p 4000 4", "p 2000 31", "s 20 80 40", "s e0 80 40"];
    const LEADS: [usize; 7] = [3, 6, 10, 16, 25, 45, 90];
    // try k: action k / LEADS.len(), lead k % LEADS.len() (all leads of a jump first)
    // P35 R5: a 0x1000 accident (off the end of a spline: the cart missed a hs_changeTrack switch, which needs a
    // lean held when the 0x17f arrives, ~25 m before the drop) tries the leans with the long leads first
    let act = |k: usize, kind: u32| if kind == 0x1000 { ["s 20 80 40", "s e0 80 40", "p 4000 4", "p 2000 31"][(k / LEADS.len()) % ACTS.len()] } else { ACTS[(k / LEADS.len()) % ACTS.len()] };
    let lead_of = |k: usize, kind: u32| if kind == 0x1000 { [90, 60, 45, 120, 30, 150, 20][k % LEADS.len()] } else { LEADS[k % LEADS.len()] };
    // hazard: (death spot, trigger spot, try index, armed this ride)
    let mut hz: Vec<([f32; 3], [f32; 3], usize, bool, u32)> = vec![];
    let mut traj: Vec<[f32; 3]> = vec![];
    let (mut f, mut prev_acc, mut deaths) = (0usize, 0u32, 0usize);
    let d2 = |a: [f32; 3], b: [f32; 3]| (a[0] - b[0]).hypot(a[2] - b[2]) + (a[1] - b[1]).abs();
    let r = run(&mut e, &mut rt, "f 60");
    assert!(r.is_none());
    while f < cap {
        let l = leader(&e);
        let p = pos(&mut e, &rt, l).unwrap_or_default();
        let (acc, kind, spl) = st(&e);
        let mut step = "f 1".to_string();
        // 0x2000 (and 0x1000 in the air) = the C's automatic jump, 0x10 = a rock hit (courage): not a crash
        if acc != 0 && prev_acc == 0 && kind != 0x2000 && kind != 0x10 {
            deaths += 1;
            match hz.iter_mut().find(|h| d2(h.0, p) < if kind == 0x1000 { 12.0 } else { 4.0 }) {
                Some(h) => h.2 += 1,
                None => hz.push((p, p, 0, false, kind)),
            }
            let h = hz.iter_mut().find(|h| d2(h.0, p) < if kind == 0x1000 { 12.0 } else { 4.0 }).unwrap();
            let lead = lead_of(h.2, h.4);
            h.1 = traj[traj.len().saturating_sub(lead)];
            eprintln!("CART f{f} accident {kind:#x} spline {spl:#x} at {p:?}: try {} = {} lead {lead}", h.2, act(h.2, h.4));
            hz.iter_mut().for_each(|h| h.3 = false);
            traj.clear();
        } else if acc == 0 {
            if let Some(h) = hz.iter_mut().find(|h| !h.3 && d2(h.1, p) < 1.5) {
                h.3 = true;
                step = act(h.2, h.4).to_string();
            }
        }
        prev_acc = acc;
        traj.push(p);
        let n: usize = step.split_whitespace().last().unwrap().parse().unwrap();
        f += n;
        if let Some(r) = run(&mut e, &mut rt, &step) {
            eprintln!("CART f{f} level switch {r:?} after {deaths} accidents, {} hazards", hz.len());
            assert!(matches!(&r, (lv, _) if lv.eq_ignore_ascii_case("EP3/L04")), "{r:?}");
            return;
        }
    }
    panic!("cart ride not finished in {cap} frames ({deaths} accidents, hazards {:?})", hz.iter().map(|h| (h.0, h.2)).collect::<Vec<_>>());
}

//! P32 R61: EP6 end -> ending / credits (the p21 chain stops at the EP6 end: "its end menus (credits) are not chained").
//! EP6/L05 fire ghost boss (fireghostboss) state machine vs C FireGhostBoss.c: see the test.
//! C (OVERALL_STATUS STPanel.c LoadNextLevel): GB "current episode" == 5 -> LoadLevel("CREDITS"), < 5 -> SPLASH.
use crate::common::{ISO, carry, load_carried, load_dump};
use crate::engine_p17::run;
use crate::p32_r59::routes_ep6;
use scooby_recomp::engine::iso::IsoFs;

/// EP6/L05B (final boss) -> LOADER -> STATUS (real end menus, as p21_real_carry_episodes) -> CREDITS -> runs on.
#[test]
fn p32_r61_ep6_end_to_credits() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let _card = crate::engine_p17::CARD.lock().unwrap_or_else(|e| e.into_inner());
    let card = std::env::temp_dir().join(format!("scooby_p32r61_card_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&card);
    // SAFETY: set at test start; only memory-card init reads it
    unsafe { std::env::set_var("GAME_SAVEDIR", &card) };
    let mut bufs = load_dump(&format!("{}/tests/all/data/p32_r60_EP6L05B.bin", env!("CARGO_MANIFEST_DIR")));
    let r = routes_ep6::EP6_ROUTE.iter().find(|r| r.0 == "EP6/L05B").unwrap().1;
    let (mut e, mut rt) = load_carried(&fs, "EP6/L05B", &bufs).unwrap();
    let (mut last, f) = run(&mut e, &mut rt, &crate::engine_p17::route_steps("EP6/L05B", r)).expect("L05B: no level request");
    println!("R61 EP6/L05B -> {last} at f{f}");
    bufs = carry(&mut e);
    let mut seq = vec![last.clone()];
    for _ in 0..8 {
        if last.eq_ignore_ascii_case("CREDITS") { break }
        let (mut e, mut rt) = load_carried(&fs, &last, &bufs).unwrap();
        let (l, f) = run(&mut e, &mut rt, &"f 282; s 80 ff 6; f 6; p 4000 6; ".repeat(30)).unwrap_or_else(|| panic!("{last}: no level request ({seq:?})"));
        println!("R61   {last} -> {l} at f{f}");
        bufs = carry(&mut e);
        last = l;
        seq.push(last.clone());
    }
    assert_eq!(seq, ["LOADER", "STATUS", "CREDITS"], "EP6 end menus");
    let (mut e, mut rt) = load_carried(&fs, "CREDITS", &bufs).unwrap();
    let r = run(&mut e, &mut rt, "f 20000");
    println!("R61   CREDITS -> {r:?}");
    // C Credits Panel.c: scroll timer (frequency 192) -> PanelCB_TIMER_SCROLL -> LoadLevel("INTRO") after 192 s
    let (l, f) = r.expect("CREDITS: no level request");
    assert_eq!(l, "INTRO");
    assert!((11400..11700).contains(&f), "credits length f{f} (192 s = 11520)");
    let _ = std::fs::remove_dir_all(&card);
}

type Eng = scooby_recomp::engine::Engine;
/// EP6/L05 fireghostboss: sub-state fn L+0, four-fireball flag L+0x60, water source L+0x64, last anim L+0x94
pub(crate) fn fgb(e: &Eng) -> Vec<u32> {
    let Some(l) = e.rt.objects.iter().flatten().find(|o| o.inst.code.is_some_and(|k| &*e.rt.codes[k].def.name == "fireghostboss")).map(|o| o.inst.local) else { return vec![] };
    vec![e.mem.r32(l), e.mem.r32(l + 0x60), e.mem.r32(l + 0x64), e.mem.r32(l + 0x94)]
}

#[test]
fn p32_r61_ep6_l05_fireghostboss_states() {
    let r = routes_ep6::EP6_ROUTE.iter().find(|r| r.0 == "EP6/L05").unwrap().1;
    let car = format!("{}/tests/all/data/p32_r61_EP6L05.bin", env!("CARGO_MANIFEST_DIR"));
    let (res, log) = crate::p32_r59::trace(None, "EP6/L05", &car, &crate::engine_p17::route_steps("EP6/L05", r), fgb);
    if log.is_empty() { return }
    assert_eq!(res.map(|r| r.0), Some("EP6/L05B".to_string()), "L05 end");
    // C FireGhostBoss.c: 0xd8 (water from brazier param_2) -> BeingAttack_In (L+0x60 = 1: four fireballs next attack), anim 5
    // end -> SearchingHero_In -> ... -> Attack_In (four fireballs) -> CB_ANIM key 3 releases them at the brazier's targets
    // and clears L+0x60; anim 3 end -> anim 1 / 6 -> AttackPreparation_In
    let (mut hits, mut cur, mut hit_f, mut four) = (vec![], 0u32, 0u32, false);
    for (f, s) in &log {
        let (st, fl, src, an) = (s[0], s[1], s[2], s[3]);
        if st == 0x11fa0 && cur != 0x11fa0 { hits.push(src); hit_f = *f }
        if st == 0x11e18 && cur == 0x120b0 { assert_eq!(an, 5, "f{f}: BeingAttack left before anim 5 ended"); assert_eq!(fl, 1, "f{f}"); four = true }
        if four && fl == 0 { assert!(f - hit_f < 200, "f{f}: four fireballs never released"); four = false }
        if st == 0x11a98 { assert!(matches!(an, 1 | 5 | 6), "f{f}: AttackPreparation after anim {an}") }
        cur = st;
    }
    assert_eq!(hits, [0x2a0000ce, 0x2a0000cf, 0x2a0000d0, 0x2a0000d1], "water hits by brazier");
}

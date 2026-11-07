//! P32 R60: boss fights vs C, part 2.
//! - EP4/L04 zombie boss hook 2 captured before the scared anim ends: TomeDoomFUNC_DrainMonsterLife (C = port) drains
//!   2.5 per matching button press (L+0x1e0 & 0x10 set by every CIRCLE/TRIANGLE/CROSS), so the drain speed is the press
//!   rate; the route bot mashes every 2nd frame. Not a port bug.
//! - EP3/L05 standalone catch (R59): the R59 test ran the raw EP3_ROUTE steps, the chain runs the ANIMSND variant
//!   (route_steps); deterministic across repeated runs / an L04 pre-run / a fresh card. Not leaked process state.
//! - EP6/L05B ghost boss 0x2a000012 (ghostbosslevel5 on Ghost_base/Monster_Base): see the test.
use crate::p32_r59::{routes_ep6, trace};

type Eng = scooby_recomp::engine::Engine;
/// EP6/L05B boss 0x2a000012 (ghostbosslevel5): boss state id, book state, book life L+0x70 (5-step), hook L+0x60, power (10-step)
pub(crate) fn gb5(e: &Eng) -> Vec<u32> {
    let Some(gs) = e.rt.objects.iter().flatten().find(|o| o.id == 0x2a00_0012).map(|o| o.inst.state) else { return vec![] };
    let bk = e.rt.objects.iter().flatten().find(|o| o.inst.code.is_some_and(|k| &*e.rt.codes[k].def.name == "tomedoom")).map(|o| (o.inst.state, o.inst.local));
    let Some((bs, l)) = bk else { return vec![] };
    let gb1 = e.rt.game_buffers.get(&1).map_or(0, |b| b.0);
    let hook = e.mem.r32(l + 0x60);
    let life = if hook == 0x2a00_0012 { (e.mem.rf(l + 0x70).max(0.0) / 5.0).floor() as u32 } else { 0 };
    vec![gs, bs, life, hook, (e.mem.rf(gb1 + 4) / 10.0).floor() as u32]
}

/// EP6/L05B ghost boss (C Monster_Base MessageFunction 0xd6: 0x65 hook sends life L+0x5c with 0x66 to the book, 0x100
/// stores the book's remaining life back in L+0x5c; Ghost_Base_Func_Initialisation (0xc9 hero caught / 0xc1) resets it to
/// max L+0xb0 = 200 and state 0x11): every new hook starts at the life the last hook ended with, or at 200 after a reset;
/// life never grows during a hook; the last hook drains it to 0 -> captured -> LOADER.
#[test]
fn p32_r60_ep6_l05b_ghostboss_life() {
    let r = routes_ep6::EP6_ROUTE.iter().find(|r| r.0 == "EP6/L05B").unwrap().1;
    let car = format!("{}/tests/all/data/p32_r60_EP6L05B.bin", env!("CARGO_MANIFEST_DIR"));
    let (res, log) = trace(None, "EP6/L05B", &car, &crate::engine_p17::route_steps("EP6/L05B", r), gb5);
    if log.is_empty() { return }
    assert_eq!(res.map(|r| r.0), Some("LOADER".to_string()), "boss captured");
    let (mut carry, mut cur, mut resets, mut hooks) = (40u32, 0u32, 0, 0);
    for (f, s) in &log {
        let (gs, life) = (s[0], s[2]);
        if gs == 0x11 && cur == 0 { carry = 40; resets += 1 }
        if life != 0 && cur == 0 {
            assert_eq!(life, carry, "f{f}: new hook life {} want {}", life * 5, carry * 5);
            hooks += 1;
        }
        if life != 0 && cur != 0 { assert!(life <= cur, "f{f}: life grew during a hook") }
        if life == 0 && cur != 0 { carry = cur }
        cur = life;
    }
    assert!(hooks >= 3 && resets >= 1, "hooks {hooks} resets {resets}");
}

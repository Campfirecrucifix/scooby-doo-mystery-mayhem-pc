//! P32 R59: boss fights vs C (state traces through common::FRAME_HOOK, one line per change).
use crate::common::{FRAME_HOOK, ISO, carry, load_carried, load_dump};
use crate::engine_p17::run;
use scooby_recomp::engine::iso::IsoFs;
#[path = "../../src/bin/routes_ep3.rs"]
#[allow(dead_code)]
pub(crate) mod routes_ep3;
#[path = "../../src/bin/routes_ep6.rs"]
#[allow(dead_code)]
pub(crate) mod routes_ep6;
#[path = "../../src/bin/routes_ep4.rs"]
#[allow(dead_code)]
mod routes_ep4;

type Eng = scooby_recomp::engine::Engine;
fn local_of(e: &Eng, id: u32) -> Option<u32> { e.rt.objects.iter().flatten().find(|o| o.id == id).map(|o| o.inst.local) }
fn find_mod(e: &Eng, name: &str) -> Vec<u32> { e.rt.objects.iter().flatten().filter(|o| o.inst.code.is_some_and(|k| &*e.rt.codes[k].def.name == name)).map(|o| o.id).collect() }
fn gb(e: &Eng, b: u32, o: u32) -> u32 { let p = e.rt.game_buffers.get(&b).unwrap().0; e.mem.r32(p + o) }

/// run `route` with a per-frame tracer `sample` (only printed / collected when the sample changes)
/// `pre` = (level, steps) played first from `carry` (its end carry is the boss level's entry carry, as the chain)
#[allow(clippy::type_complexity)]
pub(crate) fn trace(pre: Option<(&str, &str)>, lv: &str, carry: &str, route: &str, sample: fn(&Eng) -> Vec<u32>) -> (Option<(String, u32)>, Vec<(u32, Vec<u32>)>) {
    let Ok(fs) = IsoFs::open(ISO) else { return (None, vec![]) };
    let mut bufs = if carry.is_empty() { vec![] } else if carry.contains(':') { load_dump(carry) } else { load_dump(&format!("{}/tests/all/data/{carry}.bin", env!("CARGO_MANIFEST_DIR"))) };
    if let Some((pl, ps)) = pre {
        let (mut e, mut rt) = load_carried(&fs, pl, &bufs).unwrap();
        let r = run(&mut e, &mut rt, ps);
        assert_eq!(r.as_ref().map(|r| r.0.as_str()), Some(lv), "pre level {pl}");
        println!("R59 pre {pl} -> {r:?}");
        bufs = crate::common::carry(&mut e);
    }
    let (mut e, mut rt) = load_carried(&fs, lv, &bufs).unwrap();
    let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::<(u32, Vec<u32>)>::new()));
    let (l2, mut f) = (log.clone(), 0u32);
    FRAME_HOOK.with(|h| *h.borrow_mut() = Some(Box::new(move |e, _| {
        f += 1;
        let s = sample(e);
        if s.is_empty() { return }
        let mut l = l2.borrow_mut();
        if l.last().is_none_or(|(_, p)| *p != s) { l.push((f, s)) }
    })));
    let r = run(&mut e, &mut rt, route);
    FRAME_HOOK.with(|h| *h.borrow_mut() = None);
    let v = log.borrow().clone();
    (r, v)
}

pub(crate) fn dd(e: &Eng) -> Vec<u32> {
    let Some(lo) = local_of(e, 0x2a00009d) else { return vec![] };
    // life (L+0x5c, float bits), max (L+0xb0), stage bits L+0x184, small-devil kill bits GB2+4 & 0x3f000, external state word
    vec![e.mem.r32(lo + 0x5c), e.mem.r32(lo + 0xb0), e.mem.r32(lo + 0x184), gb(e, 2, 4) & 0x3f000]
}
pub(crate) fn zb(e: &Eng) -> Vec<u32> {
    let Some(&id) = find_mod(e, "zombieboss").first() else { return vec![] };
    let lo = local_of(e, id).unwrap();
    // state fn L+0x178, phase L+0x198, pull L+0x1fc, hooked L+0x1f0, loading L+0x1f4
    vec![id, e.mem.r32(lo + 0x178), e.mem.r32(lo + 0x198), e.mem.r32(lo + 0x1fc), e.mem.r32(lo + 0x1f0), e.mem.r32(lo + 0x1f4)]
}

/// EP3/L05 dust devil boss 0x2a00009d (dustdevillevelfiveboss DustDevilCB_NOTIFY_TIMER_LOCAL): the stage bits L+0x184
/// go 0 -> 1 -> 3 -> 7 -> f, each stage only after its pair of small devils is captured (GB2+4 0x3000 / 0xc000 /
/// 0x30000 on top of the earlier ones), life (L+0x5c = max L+0xb0 = 200) untouched until the last stage, then drained
/// by the book. Carry = the chain's EP3/L05 entry (P17_SAVE).
#[test]
fn p32_r59_ep3_l05_dustdevil_boss_stages() {
    let r = routes_ep3::EP3_ROUTE.iter().find(|r| r.0 == "EP3/L05").unwrap();
    let car = format!("{}/tests/all/data/p32_r59_EP3L05.bin", env!("CARGO_MANIFEST_DIR"));
    // P32 R60: the chain's ANIMSND route variant (route_steps), as p17_chains: with the raw EP3_ROUTE steps the hero was
    // caught at f10745 (0xc9 restart) standalone - a test route mismatch, not leaked process state
    let (res, log) = trace(None, "EP3/L05", &car, &crate::engine_p17::route_steps("EP3/L05", r.1), dd);
    if log.is_empty() { return }
    assert_eq!(res.map(|r| r.0), Some("LOADER".to_string()), "boss captured");
    let mut stage = 0u32;
    let mut drained = false;
    for (f, s) in &log {
        let (life, max, st, kills) = (f32::from_bits(s[0]), f32::from_bits(s[1]), s[2], s[3]);
        assert_eq!(max, 200.0);
        if st == 0 && kills == 0 && life == max { assert!(*f <= 1, "f{f}: hero caught (0xc9 restart)"); stage = 0; continue }
        if st != stage {
            assert_eq!(st, stage * 2 + 1, "f{f}: stage skipped/doubled {stage:#x} -> {st:#x}");
            let need = match st { 1 => 0, 3 => 0x3000, 7 => 0xf000, _ => 0x3f000 };
            assert_eq!(kills & need, need, "f{f}: stage {st:#x} without its captures {kills:#x}");
            stage = st;
        }
        if life < max { assert_eq!(st, 0xf, "f{f}: life drained before the last stage"); drained = true }
    }
    assert!(drained, "boss reached and drained");
}

/// EP4/L04 zombie boss (ZombieBoss.c): hooked -> SCARED_SET (L+0x1f0 = 1); in SCARED_FOLLOW the 0x100 condition asks the
/// book for the pull (0x125 -> L+0x1fc) and phase L+0x198 0 -> 1 only once pull <= 200 (1 -> 2 at <= 100), through
/// SCARED_FOLLOW_LOOP 0x20000 -> DEFENSIVE_ATTACK_SET (link broken, wisps); never more than one phase per hook; captured
/// (0xd6 0x68 -> L+0x1f4) -> LOADER. Carry = the chain's EP4/L04 entry.
#[test]
fn p32_r59_ep4_l04_zombieboss_phases() {
    use scooby_recomp::modules::zombieboss::zombieboss::addr;
    let r = routes_ep4::EP4_ROUTE.iter().find(|r| r.0 == "EP4/L04").unwrap();
    let car = format!("{}/tests/all/data/p32_r59_EP4L04.bin", env!("CARGO_MANIFEST_DIR"));
    let (res, log) = trace(None, "EP4/L04", &car, &crate::engine_p17::route_steps("EP4/L04", r.1), zb);
    if log.is_empty() { return }
    assert_eq!(res.map(|r| r.0), Some("LOADER".to_string()));
    let mut prev = &log[0].1;
    let mut hooks = 0;
    for (f, s) in &log[1..] {
        let (st, phase, pull) = (s[1], s[2], f32::from_bits(s[3]));
        // (count hooks by L+0x1f0 0 -> 1: SCARED_SET can run and leave within one frame)
        if s[4] == 1 && prev[4] == 0 { hooks += 1 }
        if phase != prev[2] {
            assert_eq!(phase, prev[2] + 1, "f{f}: phase skipped");
            assert_eq!(prev[1], addr::ZombieBossST_SCARED_FOLLOW_LOOP, "f{f}: phase change outside SCARED_FOLLOW_LOOP");
            assert_eq!(st, addr::ZombieBossST_DEFENSIVE_ATTACK_SET, "f{f}");
            assert!(pull <= if phase == 1 { 200.0 } else { 100.0 }, "f{f}: phase {phase} at pull {pull}");
        }
        prev = s;
    }
    let last = &log.last().unwrap().1;
    assert!(hooks >= 2 && last[2] >= 1, "hooks {hooks} phase {}", last[2]);
    assert_eq!(last[5], 1, "captured (L+0x1f4 loading)");
}

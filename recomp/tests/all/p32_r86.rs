//! P32 R86: probe for the GAME_LOGICCLIP soak stall in EP4/L01 (p17_chains route waits on the patrolling ghosts
//! 0x17f/0x17d/0x185). Ignored; run with GAME_LOGICCLIP=1 (and =0 to compare).
use crate::common::{leader, load_carried, load_dump, pos, ISO};
use crate::engine_p17::run;
use scooby_recomp::engine::iso::IsoFs;

#[test]
#[ignore]
fn p32_r86_probe_ep4_l01_patrol() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let route = crate::engine_p17::routes_ep4::EP4_ROUTE.iter().find(|r| r.0 == "EP4/L01").unwrap().1;
    // (R88: cut before the R87 `ifnlc waitx ..` step, not inside it)
    let cut = route.find("ifnlc waitx 2a00017f").or_else(|| route.find("waitx 2a00017f")).unwrap();
    let bufs = load_dump(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/all/data/ep3_end_EP4L01.bin"));
    let (mut e, mut rt) = load_carried(&fs, "EP4/L01", &bufs).unwrap();
    assert!(run(&mut e, &mut rt, &route[..cut]).is_none());
    let us: Vec<u32> = std::env::var("P86_UIDS").unwrap_or("2a00017f,2a00017d,2a000185".into()).split(',').map(|h| u32::from_str_radix(h, 16).unwrap()).collect();
    for i in 0..60 {
        let l = leader(&e);
        let h = pos(&mut e, &rt, l);
        let mut s = format!("P86 t{} hero {h:.1?}", i * 20);
        for &u in &us {
            let p = pos(&mut e, &rt, u);
            let st = scooby_recomp::engine::core::apigameobj::GetCurrentStateID(&mut e, u);
            let w = rt.particles.lock().unwrap().s1.clip_cur.get(&u).copied();
            s += &format!(" | {u:#x} {p:.1?} st {st} clip {w:x?}");
        }
        println!("{s}");
        assert!(run(&mut e, &mut rt, "f 20").is_none());
    }
}

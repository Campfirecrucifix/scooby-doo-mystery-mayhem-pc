//! P32 R53: FMV triggers. Episode-end / intro movies reach EFVideoPlayer through APIgame::LoadLevel
//! (stleveltransitionparams flags & 2, +4 = movie path). c2rs builds read the path from the module's own data
//! image; the hand-ported sites carry it as a literal, which must equal the string in the module binary
//! (extracted/op2/modules/*.o): dustdevillevelfiveboss EP03A, zombieboss EP04D, shaggy 26992d64 EP02G (hero_level),
//! hs_door_l01 b930f068 EP01C.
use scooby_recomp::engine::Mem;
use scooby_recomp::engine::host::game::HostGame;

#[test]
fn p32_r53_loadlevel_movie_request() {
    for (lvl, mv) in [("LOADER", r"\MOVIES\EP03\EP03A"), ("LOADER", r"\MOVIES\EP04\EP04D"), (r"EP2\L03A", r"\MOVIES\EP02\EP02G"), (r"EP1\L02", r"\MOVIES\EP01\EP01C")] {
        let mut m = Mem::new();
        let mut g = HostGame::default();
        let (l, p, s) = (m.cstr(lvl), m.cstr(lvl), m.cstr(mv));
        let par = m.malloc(0x7dc);
        m.w32(par, 2);
        m.w32(par + 4, s);
        g.LoadLevel(&m, l, p, par);
        let r = g.take_level_request().expect("request");
        assert_eq!(r.level, lvl);
        // APIgame::LoadLevel strips the one leading '\'; EN audio track 0
        assert_eq!(r.video, Some((mv[1..].to_string(), 0)), "{lvl}");
    }
    // the hand-ported literals (src/modules) == the module binaries' strings
    let src = |f: &str| std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/modules/").to_string() + f).unwrap();
    for (f, mv) in [("dustdevil/level.rs", r"EP03\\EP03A"), ("zombieboss/zombieboss.rs", r"EP04\\EP04D"), ("hero/hero_level.rs", r"EP02\\EP02G"), ("hs_door_l01.rs", r"EP01\\EP01C")] {
        assert!(src(f).contains(&format!(r"\\MOVIES\\{mv}")), "{f} {mv}");
    }
}

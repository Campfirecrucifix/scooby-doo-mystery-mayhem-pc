//! P32 R79: the three origin objects of the R78 sweep are placed at the origin by the level
//! data itself (EF_GLO POS), so the port's origin frames match C. EP4L03B hs_rideobst
//! 0x2a00018e (its 107 siblings are on the track; this one stays invisible + asleep until msg
//! 0xdc), EP6L04B shaggy/scooby_security_guard 0x2a000136/7 (inactive disguise heroes).
use crate::common::ISO;
use scooby_recomp::engine::{iso::IsoFs, pack::*};

#[test]
fn p32_r79_origin_objects_are_level_data() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    for (f, d, ids) in [("EP4L03B.P2R", "EP4/L03B", &[0x2a00018eu32][..]), ("EP6L04B.P2R", "EP6/L04B", &[0x2a000136, 0x2a000137][..])] {
        let mut m = EFRessourcesMgr::default();
        m.level_file_name = Some(f.into());
        assert_eq!(m.LoadLevel(&fs, "", d, "EN", &mut RawInstanciator, |_, _| {}).unwrap(), 0);
        for &id in ids {
            let go = m.game_objects.iter().find(|g| g.uid == id).unwrap_or_else(|| panic!("{f} {id:#x}"));
            assert_eq!(go.glo.pos, [0.0; 3], "{f} {id:#x}");
        }
    }
}

/// P32 R79 progress: EP1 per-level segments of ep1_chain (that chain needs ~25 min in one process, over the
/// 10-min run rule). Each runs one level with `--route ep1` from the real chained carry (GAME_CARRY_DUMP of a
/// R79 chain run, tests/all/data/p32_r79_EP1Lxx.bin) and must request the next story level. Measured (R79):
/// L01 -> L02 ~2.5 min, L02 -> L03A, L03A -> L03B f~14.5k (~4.5 min), L03B -> L04 f~10k. L04 -> LOADER =
/// p20_ep1_l04_segment. Ignored (minutes each, game binary).
fn ep1_segment(level: &str, carry: Option<&str>, want: &str) {
    use std::io::BufRead;
    if !crate::common::iso_or_skip("p32_r79_ep1_segment") {
        return;
    }
    let card = std::env::temp_dir().join(format!("scooby_r79_card_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&card);
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_game"));
    cmd.env("GAME_SAVEDIR", &card).args([level, "--headless", "--autoplay", "--route", "ep1", "--frames", "30000", "--iso", ISO]);
    if let Some(c) = carry {
        cmd.env("GAME_CARRY", format!("{}/tests/all/data/{c}", env!("CARGO_MANIFEST_DIR")));
    }
    let mut child = cmd.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::piped()).spawn().expect("run game");
    let mut got = None;
    for l in std::io::BufReader::new(child.stderr.take().unwrap()).lines().map_while(Result::ok) {
        assert!(!l.contains("panicked"), "{level}: {l}");
        if let Some(d) = l.strip_prefix("game: level switch -> ") {
            got = Some(d.split(' ').next().unwrap().to_string());
            break;
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    assert_eq!(got.as_deref(), Some(want), "{level}: no / wrong level switch");
}

#[test]
#[ignore]
fn p32_r79_ep1_l01_to_l02() {
    ep1_segment("EP1/L01", None, "EP1/L02");
}

#[test]
#[ignore]
fn p32_r79_ep1_l02_to_l03a() {
    ep1_segment("EP1/L02", Some("p32_r79_EP1L02.bin"), "EP1/L03A");
}

#[test]
#[ignore]
fn p32_r79_ep1_l03a_to_l03b() {
    ep1_segment("EP1/L03A", Some("p32_r79_EP1L03A.bin"), "EP1/L03B");
}

#[test]
#[ignore]
fn p32_r79_ep1_l03b_to_l04() {
    ep1_segment("EP1/L03B", Some("p32_r79_EP1L03B.bin"), "EP1/L04");
}

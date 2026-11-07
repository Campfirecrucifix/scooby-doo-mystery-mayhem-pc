//! P32 R52: save -> quit -> title Load round trip with non-default options (volumes) and an extra unlock bit.
//! p22_save_load_roundtrip compares GB1/GB2 with the values a route reaches; here the options buffer (GB3: +0 music,
//! +4 movies, +8 FX) and GB2+0x20 (gallery unlock bits) are set to distinctive values before the checkpoint save, and
//! after the title Load the restored buffers AND the live FX volume (SavePanelFUNC_SetSavedConfig, R48) must match.
use crate::common::{ISO, carry, load_carried, load_dump};
use crate::engine_p17::{CARD, run};
use scooby_recomp::engine::APInewSound;
use scooby_recomp::engine::iso::IsoFs;

#[test]
fn p32_r52_save_load_options_unlocks() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let _card = CARD.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = std::env::temp_dir().join(format!("scooby_p32_r52_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    // SAFETY: tests touching GAME_SAVEDIR hold CARD; only memory-card init reads it
    unsafe { std::env::set_var("GAME_SAVEDIR", &tmp) };
    let (a, b) = (tmp.join("a.bin"), tmp.join("b.bin"));
    let (mut e, mut rt) = load_carried(&fs, "EP1/L01", &[]).unwrap();
    assert!(run(&mut e, &mut rt, "f 200; p 4000 3; f 200; warp 2a00009b; f 200").is_none());
    let (g2, g3) = (e.rt.game_buffers[&2].0, e.rt.game_buffers[&3].0);
    let (music, movies, fx) = (0.25f32, 0.5f32, 0.75f32);
    e.mem.w32(g3, music.to_bits());
    e.mem.w32(g3 + 4, movies.to_bits());
    e.mem.w32(g3 + 8, fx.to_bits());
    let unl = e.mem.r32(g2 + 0x20) | 0x00f0_0000;
    e.mem.w32(g2 + 0x20, unl);
    let r = run(&mut e, &mut rt, &format!("f 60; save; gbdump {}; f 470; pausequit", a.display()));
    assert!(matches!(&r, Some((l, _)) if l == "INTRO"), "save/quit -> {r:?}");
    let (mut e, mut rt) = load_carried(&fs, "INTRO", &[]).unwrap();
    let r = run(&mut e, &mut rt, "menuload");
    assert!(matches!(&r, Some((l, _)) if l.eq_ignore_ascii_case("EP1/L01")), "load -> {r:?}");
    let live_fx = f32::from_bits(APInewSound::GetFXVolume(&mut e));
    let c = carry(&mut e);
    let (mut e, mut rt) = load_carried(&fs, "EP1/L01", &c).unwrap();
    assert!(run(&mut e, &mut rt, &format!("f 30; gbdump {}", b.display())).is_none());
    let (ga, gb) = (load_dump(&a.to_string_lossy()), load_dump(&b.to_string_lossy()));
    let gbuf = |g: &Vec<(u32, Vec<u8>)>, id: u32| g.iter().find(|x| x.0 == id).unwrap_or_else(|| panic!("GB{id}")).1.clone();
    let w = |v: &[u8], o: usize| u32::from_le_bytes(v[o..o + 4].try_into().unwrap());
    let (o3, n3) = (gbuf(&ga, 3), gbuf(&gb, 3));
    println!("P32R52 GB3 saved {:x?} loaded {:x?} live fx {live_fx}", &o3[..12], &n3[..12]);
    assert_eq!((w(&n3, 0), w(&n3, 4), w(&n3, 8)), (music.to_bits(), movies.to_bits(), fx.to_bits()), "volumes restored");
    assert_eq!(live_fx, fx, "SetSavedConfig applied the saved FX volume");
    assert_eq!(w(&gbuf(&gb, 2), 0x20) & 0x00f0_0000, 0x00f0_0000, "unlock bits restored");
    for id in [1u32, 2] {
        let (x, y) = (gbuf(&ga, id), gbuf(&gb, id));
        let d: Vec<usize> = (0..x.len().min(y.len()) / 4).filter(|&i| (w(&x, 4 * i) ^ w(&y, 4 * i)) & if id == 2 && i == 0 { !0x20000 } else { !0 } != 0).collect();
        assert!(d.is_empty(), "GB{id} words differ at {d:x?}");
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

//! P32 R49: same Ghidra-dropped float load as R48, in the TITLE panel (build 81d6342a): panel_baseFUNC_SetVolumes
//! (0x33f48) and panel_baseFUNC_OptionsMenuStickLeftRight (title Options > Sound bars) passed the options-buffer
//! POINTER (GBP[2]) as the music/FX/movie volume. Disasm: SetVolumes 0x33f70/0x33f94/0x33fb8 `lwc1 $f12, 0/8/4($a0)`,
//! StickLeftRight 0x31390/0x31444/0x314fc `lwc1 $f12, 0/8/4($a0)` with a0 = GBP[2].
use std::path::Path;

#[test]
fn p32_r49_title_panel_volume_setters_read_option_floats() {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/modules/panel/xb/b_81d6342a.rs");
    let src = std::fs::read_to_string(&p).unwrap();
    let mut n = 0;
    for l in src.lines().filter(|l| l.contains("LIBMiscHUD::miscHUDFUNC_Set") && l.contains("Volume(e, a1)")) {
        let off = if l.contains("SetMusicVolume") { "0u32" } else if l.contains("SetFXVolume") { "8u32" } else { "4u32" };
        let want = format!("e.r32((e.r32((e.r32(g(e, 0x23e60))).wrapping_add(8u32))).wrapping_add({off}))");
        assert!(l.contains(&want), "volume setter must read the float at options+{off}: {l}");
        n += 1;
    }
    assert_eq!(n, 6, "expected 3 SetVolumes + 3 StickLeftRight setter calls");
}

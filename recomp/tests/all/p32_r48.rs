//! P32 R48: SavePanelFUNC_SetSavedConfig (after a memory-card load) passed the GB3 options-buffer POINTER as all
//! three volumes (Ghidra lost the `lwc1 $f12` float loads). Disasm 0x1aac8..0x1ab2c: a0 = GBP[2] (options buffer),
//! music = lwc1 0($a0), FX = lwc1 8($a0), movies = lwc1 4($a0) (same layout as panel_baseFUNC_SetVolumes).
use std::path::Path;

#[test]
fn p32_r48_savepanel_set_saved_config_reads_option_floats() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/modules/savepanel");
    let mut n = 0;
    for ent in std::fs::read_dir(&dir).unwrap() {
        let p = ent.unwrap().path();
        let src = std::fs::read_to_string(&p).unwrap();
        let Some(i) = src.find("pub fn SavePanelFUNC_SetSavedConfig(") else { continue };
        let body = &src[i..i + src[i..].find("\n}\n").unwrap()];
        n += 1;
        for (setter, read) in [
            ("SetMusicVolume", "e.r32(opt)"),
            ("SetFXVolume", "e.r32(opt.wrapping_add(8))"),
            ("SetMoviesVolume", "e.r32(opt.wrapping_add(4))"),
        ] {
            let line = body.lines().find(|l| l.contains(setter)).unwrap_or_else(|| panic!("{}: no {setter}", p.display()));
            assert!(line.contains(read), "{}: {setter} must read {read}: {line}", p.display());
        }
    }
    assert_eq!(n, 40, "expected 40 savepanel builds");
}

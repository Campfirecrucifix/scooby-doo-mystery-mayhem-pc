//! WP-F: HUD / 2D.
use scooby_recomp::engine::hud::*;
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::pack::{EFRessourcesMgr, RawInstanciator};

use crate::common::ISO;

fn level_hud() -> Option<EFHudMgr> {
    let fs = IsoFs::open(ISO).ok()?;
    let mut rm = EFRessourcesMgr::default();
    rm.LoadLevel(&fs, "", "EP1/L01", "EN", &mut RawInstanciator, |_, _| {}).map_err(|e| eprintln!("{e}")).ok()?;
    let mut m = EFHudMgr::new();
    m.Init();
    m.LoadSideFiles(&rm.side_files).unwrap();
    Some(m)
}

#[test]
fn ep1l01_side_files() {
    let Some(mut m) = level_hud() else { eprintln!("ISO not found, skipped"); return };
    let f0 = m.fonts[0].as_ref().expect("font 0");
    assert_eq!(f0.nb_chars, 0x4a);
    assert_eq!(f0.chars.len(), 0x4a);
    assert_eq!(f0.text_index.len(), 0x56);
    assert_eq!(f0.textures, vec!["reserved_fonts0_0".to_string()]);
    assert!((f0.space - 0.0125).abs() < 1e-6);
    assert_eq!(m.fonts[1].as_ref().unwrap().textures[0], "reserved_fonts1_0");
    // char 1 of font 0: u=0 v=0.796875
    assert_eq!(f0.chars[1].v, 0.796875);
    let g = m.global_palette.as_ref().unwrap();
    assert_eq!(g.entries.len(), 12);
    assert_eq!(g.entries[0].colors[0], [0xff; 4]);
    assert_eq!(m.palettes[0].as_ref().unwrap().entries.len(), 12);
    let ls = m.loading_screen.as_ref().unwrap();
    assert_eq!(ls.background, "EP1LO1_LS_B");
    assert_eq!(ls.progress_bar, "Progress_Bar_B");
    let t = m.text.as_ref().unwrap();
    assert_eq!(t.langs.len(), 1);
    assert_eq!(t.langs[0].0.len(), 0x164);
    // text 0: [0 ""] [4 colour arg] [0 5 glyphs] [1]
    let t0 = t.GetText(0).unwrap();
    assert_eq!(&t0[8..12], &4u32.to_le_bytes());
    assert_eq!(u16::from_le_bytes([t0[0x10], t0[0x11]]), 0x26);

    // lay out text 0 in a text window
    let w = m.APIhud_WindowAdd(1, 0xffff_ffff, 0, 5);
    assert_ne!(w, 0);
    m.APIhud_WindowSetPos(w, 0.1, 0.1);
    m.APIhud_WindowSetDims(w, 0.5, 0.2);
    m.APIhud_TextWindowSetText(w, 0, &[0x8000_0000]);
    m.Update(1.0 / 30.0, 640, 448, &NoMem);
    let glyphs: Vec<_> = m.draw.iter().filter(|c| matches!(c.tex, HudTex::Font { .. })).collect();
    assert_eq!(glyphs.len(), 5);
    // left aligned, starting at the window x (0.1 * 640), increasing x
    assert!((glyphs[0].pos[0][0] - 64.0).abs() < 0.01);
    for p in glyphs.windows(2) {
        assert!(p[1].pos[0][0] > p[0].pos[0][0]);
    }
    assert_eq!(glyphs[0].colors[0], [0xff; 4]);

    // a number argument through a string
    let w2 = m.APIhud_WindowAdd(1, 0xffff_ffff, 0, 5);
    m.APIhud_TextWindowSetStringToDisplay(w2, "SCORE 120");
    m.Update(0.0, 640, 448, &NoMem);
    let n = m.draw.iter().filter(|c| matches!(c.tex, HudTex::Font { .. })).count();
    assert_eq!(n, 5 + 8);
}

#[test]
fn window_tree_and_handles() {
    let mut m = EFHudMgr::new();
    m.Init();
    let root = m.root.unwrap();
    assert_eq!(root, 1); // first allocation is slot last+1 = 1
    let a = m.APIhud_WindowAdd(0, 0xffff_ffff, 0, 1);
    let b = m.APIhud_WindowAdd(2, 0xffff_ffff, 0, 5);
    let c = m.APIhud_WindowAdd(0, a, 0, 0);
    assert_eq!(a, 2 << 16 | 1);
    assert_eq!(b, 3 << 16 | 1);
    // children sorted by priority: b (5) before a (1)
    assert_eq!(m.APIhud_WindowGetFirstChild(0xffff_ffff), b);
    assert_eq!(m.APIhud_WindowGetNextSibling(b), a);
    assert_eq!(m.APIhud_WindowGetNextSibling(a), a); // quirk: no sibling -> argument
    assert_eq!(m.APIhud_WindowGetParent(c), a);
    m.APIhud_WindowSetFlags(a, 0x800 | 3);
    assert_eq!(m.APIhud_WindowGetFlags(a), 0x803);
    m.APIhud_WindowResetFlags(a, 0x800);
    assert_eq!(m.APIhud_WindowGetFlags(a), 3);
    assert_eq!(m.APIhud_WindowSetID(c, 77), 1);
    assert_eq!(m.APIhud_WindowFind(77), c);
    m.APIhud_WindowSetPos(a, 0.25, 0.5);
    m.APIhud_WindowSetDims(a, 0.5, 0.5);
    assert_eq!(m.APIhud_WindowGetPos(a), Some((0.25, 0.5)));
    m.APIhud_WindowSetPos(c, 0.5, 0.5);
    m.APIhud_WindowSetDims(c, 1.0, 1.0);
    m.APIhud_WindowSetFlags(c, 3);
    m.Update(0.0, 640, 480, &NoMem);
    let wa = m.Window(a).unwrap();
    assert_eq!(wa.rect, [160.0, 240.0, 320.0, 240.0]);
    let wc = m.Window(c).unwrap();
    assert_eq!(wc.rect, [320.0, 360.0, 320.0, 240.0]);
    assert_eq!(wc.clip, [320.0, 360.0, 160.0, 120.0]); // clipped to the parent
    // deleting a deletes c; stale handles fail
    m.APIhud_WindowDelete(a);
    assert!(m.Window(a).is_none() && m.Window(c).is_none());
    assert_eq!(m.count, 2);
    let d = m.APIhud_WindowAdd(0, 0xffff_ffff, 0, 0);
    assert_eq!(d >> 16, 5); // allocation continues after the last allocated slot (4)
    // timer: hide on expiry
    m.APIhud_WindowSetFlags(b, 0x200);
    m.APIhud_WindowSetTimer(b, 0.1);
    m.Update(0.2, 640, 480, &NoMem);
    assert_eq!(m.APIhud_WindowGetFlags(b) & 0x8000_0100, 0x100);
}

#[test]
fn palette_ops() {
    let mut m = EFHudMgr::new();
    let mut p = EFHudPalette { id: 0, entries: vec![] };
    let mut g = vec![0u8; 48];
    g[16..32].fill(200);
    p.Init(3, &g);
    m.AddPalette(p);
    m.APIhud_ColorBlend(0, 2, 0, 1, 0.5, 0xff);
    assert_eq!(m.Color(0, 2).unwrap().colors[3], [100; 4]);
    m.APIhud_ColorCopy(0, 0, 1, 0x10 | 1);
    assert_eq!(m.Color(0, 0).unwrap().colors[0], [200, 0, 0, 0]);
    m.APIhud_ColorFadeStart(0, 2, 0, 1, 1.0, 0xff);
    assert_eq!(m.APIhud_ColorIsFading(0, 2), 1);
    m.Update(1.0, 640, 480, &NoMem);
    assert_eq!(m.APIhud_ColorIsFading(0, 2), 0);
    assert_eq!(m.Color(0, 2).unwrap().colors[1], [200; 4]);
}

#[test]
fn string_commands() {
    let ascii: Vec<u16> = (0..128).collect();
    let c = api::ConvertStringToTextCommands(b"A B", &ascii);
    assert_eq!(c, [0, 0, 0, 0, 65, 0, 0xfe, 0xff, 66, 0, 0, 0, 1, 0, 0, 0]);
}

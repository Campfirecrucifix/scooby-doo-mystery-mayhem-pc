//! `panel`: the in-game HUD (Tome of Doom meter, courage meter, slide-ins) and the pause / options /
//! episode-files menus. Port of `Game Logic Modules/Globals/Panel/panel__<sha8>/` (37 builds, listed in
//! tools/rustport/panel_family.txt; reference build panel__014a381f).
//!
//! Unlike the other families, the globals area of the image differs in size and layout between builds
//! (per-level window tables), so every build loads its OWN image ([`image::image`], relocated by
//! `panel_build_init`) and globals are reached through the per-build symbol table [`image::syms`]
//! (`G(e, S_x)`), never through the reference build's addresses. Window-table addresses passed to
//! `panel_baseFUNC_CreateHudWins` and window handles (`0xf000xxx`) are per-build literals (`k()`).
//!
//! PORT STATUS: the in-game HUD path (meters, faders) is ported first (book priority); unported functions
//! `todo!()` with their name. See PORT_NOTES.md Step 5m.
#![allow(non_snake_case, non_upper_case_globals, unused_variables, unused_mut, unused_assignments)]
#![allow(clippy::all)]

pub mod builds;
pub mod image;
pub mod keys;
pub mod local;
pub mod skeleton;

mod panel;
mod panel_base;
mod panel_base_options;
mod panel_frontier;

pub use image::*;
pub use panel::*;
pub use panel_base::*;
pub use panel_base_options::*;

use crate::engine::{Engine, Mem, Ptr};
use crate::logic::ModuleDef;

/// Largest globals area of the family (+ slack); every build's image fits.
const DATA_SIZE: u32 = 0xb200;

fn panel_build_init(m: &mut Mem, data: Ptr, build: usize) {
    let (sha8, img, rel) = image::image(build);
    assert_eq!(sha8, builds::BUILDS[build].sha8, "panel image/build order mismatch");
    assert!(img.len() as u32 <= DATA_SIZE);
    // the generic loader wrote the reference build's INIT words first: clear them
    for i in 0..DATA_SIZE / 4 {
        m.w32(data + i * 4, 0);
    }
    for (i, c) in img.chunks(4).enumerate() {
        let mut w = [0u8; 4];
        w[..c.len()].copy_from_slice(c);
        m.w32(data + i as u32 * 4, u32::from_le_bytes(w));
    }
    // R_MIPS_32 (REL): the addend is the word in place
    for r in rel.chunks(8) {
        let off = u32::from_le_bytes(r[0..4].try_into().unwrap());
        let kind = u32::from_le_bytes(r[4..8].try_into().unwrap());
        let add = m.r32(data + off);
        let v = if kind == 0 { data + add } else { 0x10000 + add };
        m.w32(data + off, v);
    }
}

pub static MODULE: ModuleDef = ModuleDef {
    name: "panel",
    data_size: 0x3a860, // max(DATA_SIZE, generated build 81d6342a)
    ctor: ctor_dispatch,
    dtor: dtor_dispatch,
    register: None,
    init: &[],
    build_init: Some(build_init_dispatch),
};

mod xb;

/// number of hand-ported builds (indices 0..N_OLD = [`builds::BUILDS`]); generated builds (`xb`, c2rs) follow
pub(crate) const N_OLD: usize = builds::BUILDS.len();

fn ctor_dispatch(e: &mut Engine, api: Ptr, n: i32) -> crate::logic::LResult<()> {
    if e.rt.cur_build() < N_OLD { panelConstructor(e, api, n) } else { xb::ctor(e, api, n) }
}

fn dtor_dispatch(e: &mut Engine, n: i32) -> crate::logic::LResult<()> {
    if e.rt.cur_build() < N_OLD { panelDestructor(e, n) } else { xb::dtor(e, n) }
}

fn build_init_dispatch(m: &mut Mem, data: Ptr, b: usize) {
    if b < N_OLD {
        panel_build_init(m, data, b);
    } else {
        xb::build_init(m, data, b - N_OLD);
    }
}

/// Index of a build in [`builds::BUILDS`] by the first 8 hex digits of its SHA-1.
pub fn build_index(sha8: &str) -> Option<usize> {
    builds::BUILDS.iter().position(|b| b.sha8 == sha8).or_else(|| xb::build_index(sha8).map(|i| i + N_OLD))
}

/// Address of a module global of the current build (`S_*` index into [`image::SYM_NAMES`]).
#[inline]
pub(crate) fn G(e: &Engine, s: usize) -> Ptr {
    let a = image::syms(e.rt.cur_build()).1[s];
    assert!(a != 0, "panel global {} absent from this build", image::SYM_NAMES[s]);
    e.rt.cur_data() + (a - 0x10000)
}
/// Address of module-relative image address `a` (window tables etc.).
#[inline]
pub(crate) fn img(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// `pLocalData_BasePanel`
#[inline]
pub(crate) fn LB(e: &Engine) -> Ptr {
    e.r32(G(e, S_pLocalData_BasePanel))
}
/// Port-side test hook (P32 R85): the panel's current menu row (`pLocalData_BasePanel + 0xa8`, the row
/// MenuRefresh puts the selection flower on); the address is recorded by panel_baseST_OPTIONS (0 = not yet).
pub fn menu_row(e: &Engine) -> Option<u32> {
    let a = MENU_ROW_AT.load(std::sync::atomic::Ordering::Relaxed);
    (a != 0).then(|| e.r32(a))
}
pub(crate) static MENU_ROW_AT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
/// `pLocalData_Panel`
#[inline]
pub(crate) fn LP(e: &Engine) -> Ptr {
    e.r32(G(e, S_pLocalData_Panel))
}
/// Offset `off` of the reference build's `pLocalData_BasePanel` layout in the current build: 6 builds
/// (local size 0x4d0) insert a word at 0xfc, shifting every field from 0xfc up by 4 (tools/rustport/pshift.py).
#[inline]
pub(crate) fn lo(e: &Engine, off: u32) -> u32 {
    if off >= 0xfc && k(e, builds::panel_baseConstructor_0_1) == 0x4d0 {
        off + 4
    } else {
        off
    }
}
/// `pLocalData_BasePanel + lo(off)`
#[inline]
pub(crate) fn lb(e: &Engine, off: u32) -> Ptr {
    LB(e) + lo(e, off)
}
/// `pGlobalData_BasePanel[i]` (game buffer pointers: 0 game, 1 hero/meters, 2 options, 3 flags, 4 ToD, 5)
#[inline]
pub(crate) fn GBP(e: &Engine, i: u32) -> Ptr {
    let gd = e.r32(G(e, S_pGlobalData_BasePanel));
    e.r32(gd + i * 4)
}
/// Address of a string literal of the current build's image (the original passes and compares these
/// pointers, so the identity must be the image's, not an interned copy). Panics if absent.
pub(crate) fn lit(e: &Engine, s: &str) -> Ptr {
    let img_b = image::image(e.rt.cur_build()).1;
    let pat = s.as_bytes();
    let mut i = 0;
    while let Some(p) = img_b[i..].windows(pat.len() + 1).position(|w| &w[..pat.len()] == pat && w[pat.len()] == 0) {
        let at = i + p;
        if at == 0 || img_b[at - 1] == 0 {
            return e.rt.cur_data() + at as u32;
        }
        i = at + 1;
    }
    panic!("panel: string literal {s:?} not in the image of this build");
}
/// Per-build literal (see builds.rs).
#[inline]
pub(crate) fn k(e: &Engine, i: usize) -> u32 {
    builds::BUILDS[e.rt.cur_build()].k[i]
}
/// Per-build code shape group of a function (see builds.rs).
#[inline]
pub(crate) fn g(e: &Engine, i: usize) -> u8 {
    builds::BUILDS[e.rt.cur_build()].g[i]
}
#[cold]
pub(crate) fn unported_shape(e: &Engine, f: &str, gi: usize) -> ! {
    let b = &builds::BUILDS[e.rt.cur_build()];
    panic!("{f}: code shape group {} of build {} is not ported yet", b.g[gi], b.sha8)
}
pub(crate) fn call_code_extra(e: &mut Engine, f: u32, a0: Ptr) -> crate::logic::LResult<()> {
    panic!("panel: call through code pointer {f:#x}: not a function of this module")
}
/// The 6 builds whose pLocalData_BasePanel layout is shifted by 4 from +0xfc (see `lo`).
#[inline]
pub(crate) fn shifted(e: &Engine) -> bool {
    k(e, builds::panel_baseConstructor_0_1) == 0x4d0
}

#[cfg(test)]
mod lit_tests {
    /// Every string the shared panel code looks up with lit() exists in every build (lit panics otherwise).
    /// The level-meter strings ("dustdevil", "gasoline_can", "gold_nuggets") are only used by, and only
    /// present in, their own level builds.
    #[test]
    fn lit_strings_present() {
        // WP-T1: the panel images come from the disc (cwd = recomp/ under cargo test)
        // P40 R6: found from any worktree (SCOOBY_ISO / GAME_ISO / worktree root / main checkout)
        let Some(iso) = crate::engine::gamedata::find_test_iso() else {
            eprintln!("SKIP lit_strings_present: ISO not found (set SCOOBY_ISO)");
            return;
        };
        crate::engine::gamedata::set_iso_path(&iso);
        // P40 R7 (P32 R93 race): a P39 test may have opened a temp image first (ISO_PATH keeps the first)
        crate::engine::gamedata::register_iso_elfs(&iso);
        let lits: &[&str] = &["BACK FROM FADE\n","Controller Used in Base Panel: %d\n","INTRO","NO INPUT \n\n ","NOCONTROLLER WINDOW AVAILABLE!!!\n\n","TOD_Normal_Meter","drama_masks","invalid _iTypeTraitement\n",];
        let mut miss = Vec::new();
        for (b, (sha, img)) in (0..super::builds::BUILDS.len()).map(|b| super::image::image(b)).map(|x| (x.0, x.1)).enumerate() {
            for s in lits {
                let mut pat = s.as_bytes().to_vec();
                pat.push(0);
                if !img.windows(pat.len()).any(|w| w == &pat[..]) {
                    miss.push(format!("{b} {sha}: {s:?}"));
                }
            }
        }
        eprintln!("missing:\n{}", miss.join("\n"));
    }
}

/// P33 R10: every shape group of every disc build reaches a ported arm of the panel_base functions
/// that panic with `unported_shape` (BossMeter group 3 = `<absent>`, never dispatched).
#[cfg(test)]
#[test]
fn p33_r10_panel_shape_groups_covered() {
    use builds::*;
    let lim = [
        (G_panel_baseFUNC_Init, 1),
        (G_panel_baseST_IN_GAME, 2),
        (G_panel_baseCB_PAUSE_MENU_OPEN, 1),
        (G_panel_baseCB_MENU_OUT_LF, 1),
        (G_panel_baseCB_INGAME_LTRIGGER1, 2),
        (G_panel_baseCB_TRIANGLE, 1),
        (G_panel_baseFUNC_MessageHandler, 5),
        (G_panel_baseFUNC_PauseOff, 2),
        (G_panel_baseFUNC_RerunMenuConfirm, 1),
        (G_panel_baseFUNC_YesNoConfirm, 1),
        (G_panel_baseFUNC_ShowInGame, 1),
        (G_panel_baseFUNC_MoveMetersOut, 1),
        (G_panel_baseFUNC_EpFilesClueTexFade, 1),
        (G_panel_baseFUNC_EpFilesMonstersUpdate, 3),
        (G_panel_baseFUNC_InOut_ActNotification, 1),
        (G_panel_baseFUNC_MessageHandlerBossMeter, 3),
    ];
    for b in BUILDS.iter() {
        for &(gi, max) in &lim {
            assert!(b.g[gi] <= max, "build {} group index {gi}: group {} not ported", b.sha8, b.g[gi]);
        }
    }
}

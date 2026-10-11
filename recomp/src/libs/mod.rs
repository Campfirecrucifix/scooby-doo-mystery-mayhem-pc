//! Custom libraries: library modules (EF_LIBRARY_FILE: animmngr.op2, miscfunc.op2, ...) whose
//! `<name>Register` registers a function table through APIRegLib ("AnimMngr", "MiscFunc", ...).
//! Module code calls them through the generated `LIB*` traits (`LIBAnimMngr::PlayAnim(e, ..)`),
//! which dispatch straight to the functions here.
//!
//! Non-obvious, deliberate: a library function is a plain call, not a logic frame. It runs in the
//! CALLER's logic context (so SELF means the calling object) but reads and writes the LIBRARY module's
//! globals. Those live in the library code's data image ([`lib_data`]), and its `pLocalData*` slot is
//! filled when the library's own object is constructed, as on the PS2.
#![allow(non_snake_case)]
use crate::engine::{Engine, Ptr};
use crate::logic::{CodeId, ModuleDef};

pub mod anim_mngr;
pub mod api_c_functions;
pub mod api_list;
pub mod api_loadsave_debug;
pub mod api_memcard;
pub mod api_path_finding;
pub mod api_sector_clipping;
pub mod api_text;
pub mod debug;
pub mod misc_func;
pub mod misc_hud;
pub mod snd_mngr;

/// Code of the newest loaded copy of library module `def`. Its table is the one GetApiCustom finds.
pub fn lib_code(e: &Engine, def: &'static ModuleDef) -> CodeId {
    e.rt.codes.iter().rposition(|c| std::ptr::eq(c.def, def)).unwrap_or_else(|| panic!("library module {} called but not loaded", def.name))
}

/// Build index the newest loaded copy of library module `def` was loaded with. Library functions run in
/// the CALLER's logic context, so per-build constants of a library must come from here, not from
/// `LogicRuntime::cur_build` (that is the caller's build).
pub fn lib_build(e: &Engine, def: &'static ModuleDef) -> usize {
    e.rt.codes[lib_code(e, def)].build
}

/// Address of a library module global (module-relative address `addr`, 0x10000-based).
pub fn lib_data(e: &Engine, def: &'static ModuleDef, addr: u32) -> Ptr {
    e.rt.codes[lib_code(e, def)].data + (addr - 0x10000)
}

/// Writes the API table pointers the library's Register stores into its globals.
pub fn set_global(e: &mut Engine, def: &'static ModuleDef, addr: u32, v: u32) {
    let a = lib_data(e, def, addr);
    e.w32(a, v);
}

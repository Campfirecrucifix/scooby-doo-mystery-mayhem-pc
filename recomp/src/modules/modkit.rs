//! Small shared helpers for one-off logic module ports (WP-L11): module-global slots and API table setup.
#![allow(non_snake_case)]
use crate::engine::*;

/// address of module global `a` (module-relative 0x1xxxx) in the current module image
#[inline]
pub fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
/// write a module global word
pub fn gset(e: &mut Engine, a: u32, v: u32) {
    let p = G(e, a);
    e.w32(p, v);
}
/// `slot = GetApiCustom(name)`
pub fn custom(e: &mut Engine, slot: u32, name: &str) {
    let n = e.cstr(name);
    let v = APIcons::GetApiCustom(e, n);
    gset(e, slot, v);
}
/// register external functions 0.. / at explicit indices
pub fn ext(e: &mut Engine, fns: &[(i32, crate::logic::ExtFn)]) {
    for &(i, f) in fns {
        APIcons::SetExternalFunction(e, i, f);
    }
}
/// word at (*global ptr slot) + o
pub fn pw(e: &Engine, slot: u32, o: u32) -> Ptr {
    e.r32(G(e, slot)) + o
}

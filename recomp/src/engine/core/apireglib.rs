//! APIRegLib (EXE.1 `Engine API (module interface)/EFApi_RegLib.cpp`) and the bank behind it,
//! EFCustomLibBank (`Classes/EF/EFCustomLibBank.cpp`, the instance at 0x1f720f0).
#![allow(non_snake_case)]
use crate::engine::{Engine, Ptr};
use crate::logic::LibEntry;

/// `strncmp(node->name, name, 0x20) == 0`
fn name_eq(node: &[u8; 0x20], name: &[u8]) -> bool {
    for i in 0..0x20 {
        let a = node[i];
        let b = name.get(i).copied().unwrap_or(0);
        if a != b {
            return false;
        }
        if a == 0 {
            return true;
        }
    }
    true
}

fn read_name(e: &Engine, a: Ptr) -> Vec<u8> {
    (0..0x20).map(|i| e.r8(a + i)).take_while(|&c| c != 0).collect()
}

/// Index in the bank (newest first) of the first entry named like the C string at `a`.
pub fn find_lib(e: &Engine, a: Ptr) -> Option<usize> {
    let n = read_name(e, a);
    e.rt.lib_bank.iter().position(|l| name_eq(&l.name, &n))
}

/// slot +0x0 RegisterLib(name, size) @0x002133f0 -> EFCustomLibBank::RegisterLib @0x00213120:
/// MMMalloc(size) for the table, new node first in the list. Registering a name twice leaves both
/// entries; lookups then find the newest one.
pub fn RegisterLib(e: &mut Engine, a0: Ptr, a1: u32) -> u32 {
    let table = e.malloc(a1);
    let mut name = [0u8; 0x20];
    for (i, c) in read_name(e, a0).into_iter().enumerate() {
        name[i] = c;
    }
    e.rt.lib_bank.insert(0, LibEntry { name, table, words: a1 >> 2 });
    table
}

/// slot +0x4 UnRegisterLib(name) @0x00213420 -> EFCustomLibBank::UnRegisterLib @0x002131b0:
/// unlinks the first (newest) entry of that name and frees its table; unknown names are ignored.
pub fn UnRegisterLib(e: &mut Engine, a0: Ptr) -> u32 {
    if let Some(i) = find_lib(e, a0) {
        let l = e.rt.lib_bank.remove(i);
        e.free(l.table);
    }
    0
}

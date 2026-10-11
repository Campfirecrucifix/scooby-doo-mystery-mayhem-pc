//! ApiList custom library (`Game Logic Modules/A2MUtilities/Api_List/apilist/apiList.c`, 13 functions,
//! one build 35915f47). Registers the 9-slot table "ApiList" (0x24 bytes): a pool of 128 lists of u32
//! items. Each list is a 0x400-byte game buffer (id 50000 + list index) holding 256 items; a full
//! list chains into an overflow list (`next`).
//!
//! Module globals: pExtra 0x10000, pGameObj 0x10004, pRegLib 0x10008, pList 0x1000c,
//! listPool[128] 0x10010 (0xc each: +0 buffer, +4 size, +8 next list).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]
use super::{lib_data, set_global};
use crate::engine::core::apicons as cons;
use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "apilist",
    data_size: 0x610,
    ctor: apilistConstructor,
    dtor: apilistDestructor,
    register: Some(apilistRegister),
    init: &[],
    build_init: None,
};

const pExtra: u32 = 0x10000;
const pGameObj: u32 = 0x10004;
const pRegLib: u32 = 0x10008;
const pList: u32 = 0x1000c;
const listPool: u32 = 0x10010;

/// &listPool[i] + o (o: 0 buffer, 4 size, 8 next)
fn pool(e: &Engine, i: i32, o: u32) -> Ptr {
    lib_data(e, &MODULE, listPool) + (i as u32).wrapping_mul(0xc) + o
}
fn rd(e: &Engine, i: i32, o: u32) -> i32 {
    e.ri32(pool(e, i, o))
}
fn wr(e: &mut Engine, i: i32, o: u32, v: i32) {
    let a = pool(e, i, o);
    e.wi32(a, v);
}
fn assert0(e: &mut Engine, c: u32, msg: &str) {
    let s = e.cstr(msg);
    APIextra::Assert(e, c, s);
}

/// 0x00011390
fn IsPosValid(e: &Engine, param_1: i32, param_2: i32) -> bool {
    !(param_2 < 0 || rd(e, param_1, 4) <= param_2)
}

/// 0x00011410 first free pool slot (Assert + -1 when all 128 are used)
pub fn NewList(e: &mut Engine) -> i32 {
    let mut i = 0;
    loop {
        if 0x7f < i {
            assert0(e, 0, "No more free list was available, list not created!");
            return -1;
        }
        if rd(e, i, 0) == 0 {
            break;
        }
        i += 1;
    }
    let b = APIgameobj::GetGameBuffer(e, (i + 50000) as u32, 0x400, 0);
    wr(e, i, 0, b as i32);
    wr(e, i, 4, 0);
    wr(e, i, 8, 0);
    i
}

/// 0x00011550 (also frees the overflow chain)
pub fn DeleteList(e: &mut Engine, a0: i32) {
    if rd(e, a0, 8) != 0 {
        let n = rd(e, a0, 8);
        DeleteList(e, n);
        wr(e, a0, 8, 0);
    }
    if rd(e, a0, 0) == 0 {
        assert0(e, 0, "You are trying to free memory that was not previously allocated!");
    } else {
        APIgameobj::FreeGameBuffer(e, (a0 + 50000) as u32);
        wr(e, a0, 0, 0);
    }
}

/// 0x00011680 returns the old size
pub fn ClearList(e: &mut Engine, a0: i32) -> u32 {
    if rd(e, a0, 8) != 0 {
        let n = rd(e, a0, 8);
        DeleteList(e, n);
        wr(e, a0, 8, 0);
    }
    let r = rd(e, a0, 4);
    wr(e, a0, 4, 0);
    r as u32
}

/// 0x00011770 quirk: the item count of a chained list counts every item (the overflow list holds
/// items 256..), but AddItem only appends locally while size < 256.
pub fn AddItem(e: &mut Engine, a0: i32, a1: u32) {
    if rd(e, a0, 4) < 0x100 {
        let a = rd(e, a0, 0) as u32 + rd(e, a0, 4) as u32 * 4;
        // deviation: a list that was never created (buffer NULL) is written at PS2 address 0+4n (low
        // kernel RAM, harmless there). EP1: ApiSectorClipping clients (ghost_base) register into the
        // pending list 0 before the ApiSectorClipping object's constructor allocates it.
        if rd(e, a0, 0) != 0 {
            e.w32(a, a1);
        }
        let s = rd(e, a0, 4) + 1;
        wr(e, a0, 4, s);
    } else {
        if rd(e, a0, 8) == 0 {
            let n = NewList(e);
            wr(e, a0, 8, n);
        }
        let n = rd(e, a0, 8);
        AddItem(e, n, a1);
    }
}

/// 0x00011900
pub fn RemoveItem(e: &mut Engine, a0: i32, a1: i32) -> u32 {
    let p = GetItemPos(e, a0, a1);
    RemoveItemAt(e, a0, p)
}

/// 0x00011978 swap-remove (the last item replaces the removed one); 1 = invalid position
pub fn RemoveItemAt(e: &mut Engine, a0: i32, a1: i32) -> u32 {
    if !IsPosValid(e, a0, a1) {
        return 1;
    }
    if a1 < 0x100 {
        let buf = rd(e, a0, 0) as u32;
        let last = GetItemAt(e, a0, rd(e, a0, 4) - 1);
        if buf != 0 {
            e.w32(buf + a1 as u32 * 4, last);
        }
    } else {
        let n = rd(e, a0, 8);
        RemoveItemAt(e, n, a1 - 0x100);
    }
    let s = rd(e, a0, 4) - 1;
    wr(e, a0, 4, s);
    if rd(e, a0, 4) < 0x101 && rd(e, a0, 8) != 0 {
        let n = rd(e, a0, 8);
        DeleteList(e, n);
        wr(e, a0, 8, 0);
    }
    0
}

/// 0x00011b98
pub fn GetSize(e: &mut Engine, a0: i32) -> u32 {
    rd(e, a0, 4) as u32
}

/// 0x00011b98 (0 for an invalid position)
pub fn GetItemAt(e: &mut Engine, a0: i32, a1: i32) -> u32 {
    if !IsPosValid(e, a0, a1) {
        0
    } else if a1 < 0x100 {
        // deviation: NULL buffer (see AddItem) reads 0
        if rd(e, a0, 0) == 0 {
            return 0;
        }
        e.r32(rd(e, a0, 0) as u32 + a1 as u32 * 4)
    } else {
        let n = rd(e, a0, 8);
        assert0(e, n as u32, "We have more than LIST_LENGTH items but no nextList to check in!\n");
        GetItemAt(e, n, a1 - 0x100)
    }
}

/// 0x00011ce0 position of item a1, -1 if absent
pub fn GetItemPos(e: &mut Engine, a0: i32, a1: i32) -> i32 {
    let mut i = 0;
    loop {
        if 0xff < i {
            let n = rd(e, a0, 8);
            return GetItemPos(e, n, a1) + 0x100;
        }
        if rd(e, a0, 4) <= i {
            return -1;
        }
        if GetItemAt(e, a0, i) as i32 == a1 {
            return i;
        }
        i += 1;
    }
}

/// 0x00011e50
pub fn apilistRegister(e: &mut Engine, api: Ptr) {
    let v = cons::GetApiExtra(e);
    set_global(e, &MODULE, pExtra, v);
    let v = cons::GetApiGameObj(e);
    set_global(e, &MODULE, pGameObj, v);
    let v = cons::GetApiRegLib(e);
    set_global(e, &MODULE, pRegLib, v);
    let n = e.cstr("ApiList");
    let v = APIRegLib::RegisterLib(e, n, 0x24);
    set_global(e, &MODULE, pList, v);
    // the 9 slot writes: LIBApiList dispatches straight to the functions above
}

/// 0x00011fa8
pub fn apilistConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    Ok(())
}

/// 0x00011fd0 frees every allocated list buffer
pub fn apilistDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    for i in 0..0x80 {
        if rd(e, i, 0) != 0 {
            APIgameobj::FreeGameBuffer(e, (i + 50000) as u32);
            wr(e, i, 0, 0);
        }
    }
    Ok(())
}

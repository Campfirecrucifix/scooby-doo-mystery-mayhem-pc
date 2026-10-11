//! `memorymanager`: memory-card request queue in front of the Api_MemCard library. A client sends a
//! request message (1..0xb), the next update starts the matching card operation (step +8 = 0 -> call ->
//! 1..3), the Api_MemCard callback (memmanagerMemCardCallback) sets flag bit 1 with the result, and the
//! result is sent back to the client as message `result` with payload local +0x3c. Both builds
//! (8d7a41e2, dc145aeb) are the same code. Port of
//! `Game Logic Modules/A2MUtilities/MemoryManager/memorymanager__*/MemoryManager.c`.
//!
//! Local data (0x44): +0 flags (1 = card callback arrived), +4 request mask, +8 step, +0xc client id,
//! +0x10 result (msg id), +0x14 running operation, +0x18 port, +0x1c/+0x20/+0x24 (cleared), +0x28,
//! +0x30, +0x34, +0x38 request params, +0x3c callback payload.
//! The running operation at +0x14 is a code pointer in the C; the port stores the same module address
//! (0x11150 LoadGame, ...) and dispatches on it.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]

use crate::engine::*;
use crate::logic::{LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "memorymanager",
    data_size: 0x10,
    ctor: memorymanagerConstructor,
    dtor: memorymanagerDestructor,
    register: None,
    init: &[],
    build_init: None,
};

pub fn build_index(key: &str) -> Option<usize> {
    ["8d7a41e2", "dc145aeb"].iter().position(|k| *k == key)
}

pub const pMemManagerLocalData: u32 = 0x10000;
pub const pMemCard: u32 = 0x10004;
pub const pGameObj: u32 = 0x10008;
pub const pExtra: u32 = 0x1000c;

const LOADGAME: u32 = 0x11150;
const GETGAMEINFO: u32 = 0x112d8;
const ISCARDPRESENT: u32 = 0x11398;
const SAVEGAMEDATA: u32 = 0x11450;
const SAVEHEADER: u32 = 0x11538;
const CREATESAVEGAMEDATA: u32 = 0x11608;
const FORMATMEMCARD: u32 = 0x116f8;
const DELETESAVEGAMEDATA: u32 = 0x117b0;
const ISMEMAVAILABLE: u32 = 0x11868;
const GETCARDNAME: u32 = 0x11940;
const MANAGEMEMCARD: u32 = 0x11a00;

#[inline]
fn G(e: &Engine, a: u32) -> Ptr {
    e.rt.cur_data() + (a - 0x10000)
}
fn L(e: &Engine) -> Ptr {
    e.r32(G(e, pMemManagerLocalData))
}
fn lr(e: &Engine, o: u32) -> u32 {
    e.r32(L(e) + o)
}
fn lw(e: &mut Engine, o: u32, v: u32) {
    let a = L(e) + o;
    e.w32(a, v);
}

/// 0x00010d38 (external function 0)
pub fn memmanagerUpdate(e: &mut Engine, _: u32) -> LResult<u64> {
    memmanagerPreLogic(e);
    match lr(e, 0x14) {
        0 => {}
        LOADGAME => memmanagerLoadGame(e),
        GETGAMEINFO => memmanagerGetGameInfo(e),
        ISCARDPRESENT => memmanagerIsCardPresent(e),
        SAVEGAMEDATA => memmanagerSaveGameData(e),
        SAVEHEADER => memmanagerSaveHeader(e),
        CREATESAVEGAMEDATA => memmanagerCreateSaveGameData(e),
        FORMATMEMCARD => memmanagerFormatMemCard(e),
        DELETESAVEGAMEDATA => memmanagerDeleteSaveGameData(e),
        ISMEMAVAILABLE => memmanagerIsMemAvailable(e),
        GETCARDNAME => memmanagerGetCardName(e),
        MANAGEMEMCARD => memmanagerManageMemCard(e),
        f => panic!("memorymanager: bad operation {f:#x}"),
    }
    memmanagerPostLogic(e);
    Ok(0)
}

/// 0x00010dc0 lowest request bit wins
fn memmanagerPreLogic(e: &mut Engine) {
    let m = lr(e, 4);
    if m == 0 || lr(e, 0x14) != 0 {
        return;
    }
    const ORDER: [(u32, u32); 11] = [
        (1, CREATESAVEGAMEDATA),
        (2, LOADGAME),
        (4, SAVEGAMEDATA),
        (8, SAVEHEADER),
        (0x10, ISCARDPRESENT),
        (0x20, FORMATMEMCARD),
        (0x40, MANAGEMEMCARD),
        (0x80, DELETESAVEGAMEDATA),
        (0x100, ISMEMAVAILABLE),
        (0x200, GETCARDNAME),
        (0x400, GETGAMEINFO),
    ];
    if let Some(&(_, f)) = ORDER.iter().find(|(b, _)| m & b != 0) {
        lw(e, 0x14, f);
        lw(e, 8, 0);
    }
}

/// 0x00011118
fn memmanagerPostLogic(e: &mut Engine) {
    lw(e, 0, 0);
    lw(e, 4, 0);
}

fn done(e: &Engine) -> bool {
    lr(e, 0) & 1 != 0
}

/// 0x00011150
fn memmanagerLoadGame(e: &mut Engine) {
    if lr(e, 8) == 0 {
        let (p, a) = (lr(e, 0x18) as i32, lr(e, 0x30));
        LIBApi_MemCard::ReadSGH(e, p, a);
        lw(e, 8, 2);
    }
    if lr(e, 8) == 2 && done(e) {
        if lr(e, 0x10) == 0 {
            let f = lr(e, 0) & !1;
            lw(e, 0, f);
            let (p, a, b) = (lr(e, 0x18) as i32, lr(e, 0x28), lr(e, 0x34));
            LIBApi_MemCard::ReadSGS(e, p, a, b);
            lw(e, 8, 3);
        } else {
            memmanagerReportResults(e);
        }
    }
    if lr(e, 8) == 3 && done(e) {
        memmanagerReportResults(e);
    }
}

/// shape shared by the one-call operations: call at step 0 -> step 1, report when the callback came
fn one_step(e: &mut Engine, call: fn(&mut Engine), free: bool) {
    if lr(e, 8) == 0 {
        call(e);
        lw(e, 8, 1);
    }
    if lr(e, 8) == 1 && done(e) {
        if free {
            let v = LIBApi_MemCard::GetMemToFree_Direct(e);
            lw(e, 0x40, v);
        }
        memmanagerReportResults(e);
    }
}

/// 0x000112d8
fn memmanagerGetGameInfo(e: &mut Engine) {
    one_step(e, |e| { let (p, a) = (lr(e, 0x18) as i32, lr(e, 0x30)); LIBApi_MemCard::ReadSGH(e, p, a) }, false);
}
/// 0x00011398
fn memmanagerIsCardPresent(e: &mut Engine) {
    one_step(e, |e| { let p = lr(e, 0x18) as i32; LIBApi_MemCard::MemCardPresent(e, p) }, false);
}
/// 0x00011450
fn memmanagerSaveGameData(e: &mut Engine) {
    one_step(e, |e| {
        let (p, a, b, c, d) = (lr(e, 0x18) as i32, lr(e, 0x30), lr(e, 0x28), lr(e, 0x34), lr(e, 0x38));
        LIBApi_MemCard::WriteSGS(e, p, a, b, c, d)
    }, false);
}
/// 0x00011538
fn memmanagerSaveHeader(e: &mut Engine) {
    one_step(e, |e| { let (p, a, b) = (lr(e, 0x18) as i32, lr(e, 0x30), lr(e, 0x38)); LIBApi_MemCard::WriteSGH(e, p, a, b) }, false);
}
/// 0x00011608 (+0x40 = memory to free)
fn memmanagerCreateSaveGameData(e: &mut Engine) {
    one_step(e, |e| { let (p, a, b) = (lr(e, 0x18) as i32, lr(e, 0x30), lr(e, 0x38)); LIBApi_MemCard::CreateSGData(e, p, a, b) }, true);
}
/// 0x000116f8
fn memmanagerFormatMemCard(e: &mut Engine) {
    one_step(e, |e| { let p = lr(e, 0x18) as i32; LIBApi_MemCard::FormatMemCard(e, p) }, false);
}
/// 0x000117b0
fn memmanagerDeleteSaveGameData(e: &mut Engine) {
    one_step(e, |e| { let p = lr(e, 0x18) as i32; LIBApi_MemCard::DeleteSGData(e, p) }, false);
}
/// 0x00011868 (+0x40 = memory to free)
fn memmanagerIsMemAvailable(e: &mut Engine) {
    one_step(e, |e| { let p = lr(e, 0x18) as i32; LIBApi_MemCard::GetAvailableMem(e, p) }, true);
}
/// 0x00011940
fn memmanagerGetCardName(e: &mut Engine) {
    one_step(e, |e| { let (p, a) = (lr(e, 0x18), lr(e, 0x38)); LIBApi_MemCard::GetMemCardName(e, p, a) }, false);
}
/// 0x00011a00
fn memmanagerManageMemCard(e: &mut Engine) {
    one_step(e, |e| { let p = lr(e, 0x18); LIBApi_MemCard::ManageMemCard(e, p) }, false);
}

/// 0x00011ab8 (empty)
fn memmanagerEnableReset(e: &mut Engine) {}

/// 0x00011ae0
fn memmanagerResetMsgParameters(e: &mut Engine) {
    for (o, v) in [(0x30, 0), (0x34, 0), (0x18, 0xffff_ffff), (0x20, 0), (0x28, 0), (0x1c, 0), (0x24, 0)] {
        lw(e, o, v);
    }
}

/// 0x00011b60 result message (+0x10) to the client (+0xc), payload +0x3c
fn memmanagerReportResults(e: &mut Engine) {
    lw(e, 0x14, 0);
    lw(e, 8, 0);
    memmanagerResetMsgParameters(e);
    memmanagerEnableReset(e);
    let l = L(e);
    let me = APIgameobj::GetID(e, SELF, NULL);
    let (to, msg) = (e.r32(l + 0xc), e.r32(l + 0x10));
    APIgameobj::SendMessageTo(e, to, NULL, msg, me, l + 0x3c);
}

/// 0x00011c40 Api_MemCard completion callback (pfnReturnCallback)
pub fn memmanagerMemCardCallback(e: &mut Engine, param_1: u32, param_2: u32) -> u32 {
    lw(e, 0x10, param_1);
    lw(e, 0x3c, param_2);
    let f = lr(e, 0) | 1;
    lw(e, 0, f);
    0
}

fn par(e: &Engine, p3: Ptr, i: u32) -> u32 {
    e.r32(p3 + i * 4)
}
fn req(e: &mut Engine, bit: u32) {
    let v = lr(e, 4) | bit;
    lw(e, 4, v);
}
/// local[o] = param[i] for each pair
fn copy(e: &mut Engine, p3: Ptr, pairs: &[(u32, u32)]) {
    for &(o, i) in pairs {
        let v = par(e, p3, i);
        lw(e, o, v);
    }
}

/// 0x00011ca8
pub fn memmanagerMessageHandler(e: &mut Engine, m: u32, p2: u32, p3: Ptr) -> LResult<u64> {
    match m {
        0 => {
            lw(e, 0xc, p2);
            let (a, b, c) = (par(e, p3, 1), par(e, p3, 3), par(e, p3, 2));
            LIBApi_MemCard::SetMemCardParams(e, a, b, c);
            let s = par(e, p3, 5);
            LIBApi_MemCard::SetSizeToCheck(e, s);
        }
        1 => {
            req(e, 2);
            copy(e, p3, &[(0x30, 6), (0x34, 7), (0x18, 0), (0x28, 4)]);
        }
        2 => {
            req(e, 0x400);
            copy(e, p3, &[(0x18, 0), (0x30, 6)]);
        }
        3 => {
            req(e, 0x10);
            copy(e, p3, &[(0x18, 0)]);
        }
        4 => {
            req(e, 4);
            copy(e, p3, &[(0x30, 6), (0x34, 7), (0x18, 0), (0x28, 4), (0x38, 8)]);
            memmanagerEnableReset(e);
        }
        5 | 6 => {
            req(e, if m == 5 { 8 } else { 1 });
            copy(e, p3, &[(0x30, 6), (0x18, 0), (0x38, 8)]);
            memmanagerEnableReset(e);
        }
        7 | 8 => {
            req(e, if m == 7 { 0x20 } else { 0x80 });
            copy(e, p3, &[(0x18, 0)]);
            memmanagerEnableReset(e);
        }
        9 => {
            req(e, 0x200);
            copy(e, p3, &[(0x18, 0), (0x38, 8)]);
            let s = e.cstr("\n\n*** MSGMEMMANAGER_GetCardName ***\n\n");
            APIextra::null_printf(e, s, 0);
        }
        10 | 0xb => {
            req(e, if m == 10 { 0x40 } else { 0x100 });
            copy(e, p3, &[(0x18, 0)]);
        }
        0xc => memmanagerEnableReset(e),
        _ => {}
    }
    Ok(0)
}

/// 0x000121c8
pub fn memorymanagerConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        APIcons::SetExternalFunction(e, 0, memmanagerUpdate);
        let s = e.cstr("Api_MemCard");
        let v = APIcons::GetApiCustom(e, s);
        let a = G(e, pMemCard);
        e.w32(a, v);
        for (a, v) in [(pGameObj, APIcons::GetApiGameObj(e)), (pExtra, APIcons::GetApiExtra(e))] {
            let a = G(e, a);
            e.w32(a, v);
        }
    }
    let c = e.alloca(0x10);
    APIgameobj::GetClippingParams(e, SELF, c);
    for (o, v) in [(0, 0), (4, 0xbf80_0000), (0xc, 0), (8, 0)] {
        e.w32(c + o, v);
    }
    APIgameobj::SetClippingParams(e, SELF, c, 0);
    APIcons::SetMessageFunction(e, memmanagerMessageHandler);
    // memmanagerMemCardCallback (0x11c40) as a code-pointer token
    let cb = e.code_ptr(crate::logic::CodeFn::Word2(memmanagerMemCardCallback));
    LIBApi_MemCard::SetMemCardCallback(e, cb as i32);
    let slot = G(e, pMemManagerLocalData);
    APIcons::GetLocalData(e, slot, 0x44);
    APIgameobj::ResetLogicFlag(e, SELF, 2);
    APIcons::SetInitialState(e, 0);
    for (o, v) in [(0, 0), (4, 0), (8, 0), (0x10, 0xffff_ffff), (0xc, 0xffff_ffff)] {
        lw(e, o, v);
    }
    Ok(())
}

/// 0x000123a8
pub fn memorymanagerDestructor(e: &mut Engine, _: i32) -> LResult<()> {
    Ok(())
}

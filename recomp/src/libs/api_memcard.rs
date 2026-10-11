//! Api_MemCard custom library (`Game Logic Modules/A2MUtilities/Api_MemCard/api_memcard__*/Api_MemCard.c`,
//! builds 58d4cc1e / db2868ba, identical code). Registers the 16-slot table "Api_MemCard" (0x40): a
//! save-game layer over APILoadSave (one file `<product code>` holding a header + N slots). Each
//! request sets the operation, then SetWorkingMemoryCard; APILoadSave answers through
//! CompletionCallback (-> the operation's result handler, which issues the next card command) or
//! ErrorCallback; the final outcome goes to the client callback (pfnReturnCallback) via SendError.
//!
//! Port notes: code pointers (pfnResultHandler, pfnReturnCallback, the APILoadSave callbacks) are
//! code-pointer tokens ([`Engine::code_ptr`]). The `pExtra->null_printf((char*)0x10d20)` traces (an
//! empty debug string, output discarded) are dropped. With the port's APILoadSave stub (no card) every
//! request ends in ErrorCallback(0x12) -> SendError(1) (or 0xb), as on a PS2 with no card inserted.
//!
//! Globals: uiLastAction 0x10000, uiLastSubAction 04, pfnReturnCallback 08, pfnResultHandler 0c,
//! uiSizeSaveGameHeader 10, uiSizeSaveGameSlot 14, uiNumSaveGameSlots 18, pSaveGameSlotBuffer 1c,
//! pSaveGameHeaderBuffer 20, uiSaveGameSlotId 24, uiTimeStamp 28, pBuffer 2c, pString 30,
//! uiCurrentOperation 34, uiSizeToCheck 38, uiLastSizeToFree 3c, oLastCommandParams 40 (+44..58 saved
//! args for RedoLastAction), pLoadSave 5c, pRegLib 60, pExtra 64, pGameObj 68.
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]
use super::{lib_data, set_global};
use crate::engine::core::apicons as cons;
use crate::engine::*;
use crate::logic::{CodeFn, LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "api_memcard",
    data_size: 0x80,
    ctor: api_memcardConstructor,
    dtor: api_memcardDestructor,
    register: Some(api_memcardRegister),
    init: &[],
    build_init: None,
};

const uiLastAction: u32 = 0x10000;
const uiLastSubAction: u32 = 0x10004;
const pfnReturnCallback: u32 = 0x10008;
const pfnResultHandler: u32 = 0x1000c;
const uiSizeSaveGameHeader: u32 = 0x10010;
const uiSizeSaveGameSlot: u32 = 0x10014;
const uiNumSaveGameSlots: u32 = 0x10018;
const pSaveGameSlotBuffer: u32 = 0x1001c;
const pSaveGameHeaderBuffer: u32 = 0x10020;
const uiSaveGameSlotId: u32 = 0x10024;
const uiTimeStamp: u32 = 0x10028;
const pBuffer: u32 = 0x1002c;
const pString: u32 = 0x10030;
const uiCurrentOperation: u32 = 0x10034;
const uiSizeToCheck: u32 = 0x10038;
const uiLastSizeToFree: u32 = 0x1003c;
const oLastCommandParams: u32 = 0x10040;
const DAT_44: u32 = 0x10044;
const DAT_48: u32 = 0x10048;
const DAT_4c: u32 = 0x1004c;
const DAT_50: u32 = 0x10050;
const DAT_54: u32 = 0x10054;
const DAT_58: u32 = 0x10058;
const pLoadSave: u32 = 0x1005c;
const pRegLib: u32 = 0x10060;
const pExtra: u32 = 0x10064;
const pGameObj: u32 = 0x10068;

fn g(e: &Engine, a: u32) -> u32 {
    e.r32(lib_data(e, &MODULE, a))
}
fn sg(e: &mut Engine, a: u32, v: u32) {
    set_global(e, &MODULE, a, v);
}
/// whole save file: header + all slots
fn file_size(e: &Engine) -> u32 {
    g(e, uiSizeSaveGameSlot).wrapping_mul(g(e, uiNumSaveGameSlots)).wrapping_add(g(e, uiSizeSaveGameHeader))
}
fn set_handler(e: &mut Engine, f: fn(&mut Engine, u32) -> u32) {
    let t = e.code_ptr(CodeFn::Word(f));
    sg(e, pfnResultHandler, t);
}

/// 0x000110e0
fn memcpy_local(e: &mut Engine, dst: Ptr, src: Ptr, n: u32) {
    for i in 0..n {
        let b = e.r8(src + i);
        e.w8(dst + i, b);
    }
}

/// 0x00011168
pub fn SetMemCardCallback(e: &mut Engine, param_1: i32) {
    if param_1 != 0 {
        sg(e, pfnReturnCallback, param_1 as u32);
    }
}
/// 0x000111a8
pub fn SetSizeToCheck(e: &mut Engine, param_1: u32) {
    sg(e, uiSizeToCheck, param_1);
}
/// 0x000111d8
pub fn FormatMemCard(e: &mut Engine, param_1: i32) {
    set_handler(e, FormatMemCardResultHandler);
    sg(e, uiCurrentOperation, 5);
    sg(e, uiLastAction, 0xf);
    sg(e, oLastCommandParams, param_1 as u32);
    APILoadSave::SetWorkingMemoryCard(e, 0, param_1 - 1);
}
/// 0x00011260
pub fn ManageMemCard(e: &mut Engine, param_1: u32) {
    sg(e, oLastCommandParams, param_1);
}
/// 0x000112c0
pub fn CreateSGData(e: &mut Engine, param_1: i32, param_2: u32, param_3: u32) {
    set_handler(e, CreateSGDataResultHandler);
    sg(e, uiCurrentOperation, 6);
    sg(e, pSaveGameHeaderBuffer, param_2);
    sg(e, pString, param_3);
    sg(e, oLastCommandParams, param_1 as u32);
    sg(e, DAT_44, param_2);
    sg(e, DAT_48, param_3);
    sg(e, uiLastAction, 0xf);
    APILoadSave::SetWorkingMemoryCard(e, 0, param_1 - 1);
}
/// 0x00011398
pub fn DeleteSGData(e: &mut Engine, param_1: i32) {
    set_handler(e, DeleteSGDataResultHandler);
    sg(e, uiCurrentOperation, 7);
    sg(e, oLastCommandParams, param_1 as u32);
    sg(e, uiLastAction, 0xf);
    APILoadSave::SetWorkingMemoryCard(e, 0, param_1 - 1);
}
/// 0x00011438
pub fn ReadSGH(e: &mut Engine, param_1: i32, param_2: u32) {
    set_handler(e, ReadSGHResultHandler);
    sg(e, uiCurrentOperation, 1);
    sg(e, pSaveGameHeaderBuffer, param_2);
    sg(e, oLastCommandParams, param_1 as u32);
    sg(e, DAT_4c, param_2);
    sg(e, uiLastAction, 0xf);
    APILoadSave::SetWorkingMemoryCard(e, 0, param_1 - 1);
}
/// 0x000114f8
pub fn ReadSGS(e: &mut Engine, param_1: i32, param_2: u32, param_3: u32) {
    set_handler(e, ReadSGSResultHandler);
    sg(e, uiCurrentOperation, 2);
    sg(e, pSaveGameSlotBuffer, param_3);
    sg(e, uiSaveGameSlotId, param_2);
    sg(e, oLastCommandParams, param_1 as u32);
    sg(e, DAT_50, param_3);
    sg(e, DAT_54, param_2);
    sg(e, uiLastAction, 0xf);
    APILoadSave::SetWorkingMemoryCard(e, 0, param_1 - 1);
}
/// 0x000115d0
pub fn WriteSGH(e: &mut Engine, param_1: i32, param_2: u32, param_3: u32) {
    set_handler(e, WriteSGHResultHandler);
    sg(e, uiCurrentOperation, 3);
    sg(e, pSaveGameHeaderBuffer, param_2);
    sg(e, pString, param_3);
    sg(e, oLastCommandParams, param_1 as u32);
    sg(e, DAT_48, param_3);
    sg(e, DAT_4c, param_2);
    sg(e, uiLastAction, 0xf);
    APILoadSave::SetWorkingMemoryCard(e, 0, param_1 - 1);
}
/// 0x000116a8
pub fn WriteSGS(e: &mut Engine, param_1: i32, param_2: u32, param_3: u32, param_4: u32, param_5: u32) {
    set_handler(e, WriteSGSResultHandler);
    sg(e, uiCurrentOperation, 4);
    sg(e, pSaveGameSlotBuffer, param_4);
    sg(e, pSaveGameHeaderBuffer, param_2);
    sg(e, uiSaveGameSlotId, param_3);
    sg(e, pString, param_5);
    sg(e, oLastCommandParams, param_1 as u32);
    sg(e, DAT_48, param_5);
    sg(e, DAT_4c, param_2);
    sg(e, DAT_50, param_4);
    sg(e, DAT_54, param_3);
    sg(e, uiLastAction, 0xf);
    APILoadSave::SetWorkingMemoryCard(e, 0, param_1 - 1);
}
/// 0x000117b8
pub fn MemCardPresent(e: &mut Engine, param_1: i32) {
    set_handler(e, MemCardPresentResultHandler);
    sg(e, uiCurrentOperation, 1);
    sg(e, uiLastAction, 0xf);
    sg(e, oLastCommandParams, param_1 as u32);
    APILoadSave::SetWorkingMemoryCard(e, 0, param_1 - 1);
}
/// 0x00011840
pub fn SetMemCardParams(e: &mut Engine, param_1: u32, param_2: u32, param_3: u32) {
    sg(e, uiSizeSaveGameHeader, param_1);
    sg(e, uiSizeSaveGameSlot, param_2);
    sg(e, uiNumSaveGameSlots, param_3);
}
/// 0x00011890
pub fn GetAvailableMem(e: &mut Engine, param_1: i32) {
    set_handler(e, GetAvailableMemResultHandler);
    sg(e, uiCurrentOperation, 8);
    sg(e, oLastCommandParams, param_1 as u32);
    sg(e, uiLastAction, 0xf);
    APILoadSave::SetWorkingMemoryCard(e, 0, param_1 - 1);
}
/// 0x00011930 (no card command: the request never completes, as in the original)
pub fn GetMemCardName(e: &mut Engine, param_1: u32, param_2: u32) {
    set_handler(e, GetMemCardNameResultHandler);
    sg(e, uiCurrentOperation, 1);
    sg(e, pString, param_2);
    sg(e, oLastCommandParams, param_1);
    sg(e, DAT_58, param_2);
}
/// 0x00011a48 (not in the table; kept for completeness)
pub fn RedoLastAction(e: &mut Engine, param_1: u32) {
    let p = g(e, oLastCommandParams) as i32;
    match param_1 {
        1 => {
            let a = g(e, DAT_4c);
            ReadSGH(e, p, a)
        }
        2 => {
            let (a, b) = (g(e, DAT_54), g(e, DAT_50));
            ReadSGS(e, p, a, b)
        }
        3 => {
            let (a, b) = (g(e, DAT_4c), g(e, DAT_48));
            WriteSGH(e, p, a, b)
        }
        4 => {
            let (a, b, c, d) = (g(e, DAT_4c), g(e, DAT_54), g(e, DAT_50), g(e, DAT_48));
            WriteSGS(e, p, a, b, c, d)
        }
        5 => FormatMemCard(e, p),
        6 => {
            let (a, b) = (g(e, DAT_44), g(e, DAT_48));
            CreateSGData(e, p, a, b)
        }
        7 => DeleteSGData(e, p),
        8 => GetAvailableMem(e, p),
        _ => SendError(e, 0xd, 0),
    }
}
/// 0x00011d28
fn ContinueLastAction(e: &mut Engine, param_1: u32) {
    let h: fn(&mut Engine, u32) -> u32 = match param_1 {
        1 => ReadSGHResultHandler,
        2 => ReadSGSResultHandler,
        3 => WriteSGHResultHandler,
        4 => WriteSGSResultHandler,
        5 => FormatMemCardResultHandler,
        6 => CreateSGDataResultHandler,
        7 => DeleteSGDataResultHandler,
        8 => GetAvailableMemResultHandler,
        _ => return SendError(e, 0xd, 0),
    };
    h(e, 0);
}
/// 0x00011e90
pub fn GetMemToFree_Direct(e: &mut Engine) -> u32 {
    g(e, uiLastSizeToFree)
}
/// 0x00011ec8 APILoadSave completion callback
pub fn CompletionCallback(e: &mut Engine, param_1: u32) -> u32 {
    let h = g(e, pfnResultHandler);
    if h != 0 {
        match e.code_fn(h) {
            CodeFn::Word(f) => {
                f(e, param_1);
            }
            _ => panic!("Api_MemCard: result handler is not a word function"),
        }
    }
    0
}
/// 0x00011f18 APILoadSave error callback; param_1 -> {last action, error}
pub fn ErrorCallback(e: &mut Engine, param_1: u32) -> u32 {
    let err = e.r32(param_1 + 4);
    let (op, la) = (g(e, uiCurrentOperation), g(e, uiLastAction));
    if op == 4 && la == 9 && g(e, uiLastSubAction) == 0xb && err == 0x12 {
        SendError(e, 0, 0);
        return 0;
    }
    // (first switch: null_printf traces only)
    if la == 0x1b && err == 0x16 {
        ContinueLastAction(e, op);
    }
    // the second switch re-reads the globals (ContinueLastAction may have changed them)
    let (op, la) = (g(e, uiCurrentOperation), g(e, uiLastAction));
    match err {
        1 | 0x18 | 0x1d | 0x1e => SendError(e, 4, 0),
        0x12 => SendError(e, if la == 0xf { 1 } else { 0xb }, 0),
        0x13 => {
            if op == 5 {
                sg(e, uiLastAction, 0xf);
                FormatMemCardResultHandler(e, 0);
            } else {
                SendError(e, 2, 0)
            }
        }
        0x14 => SendError(e, 5, 0),
        0x15 | 0x1b => SendError(e, 9, 0),
        0x16 => SendError(e, if op == 1 || op == 2 { 3 } else { 9 }, 0),
        0x17 => SendError(e, if op == 1 || op == 2 { 10 } else if op == 3 || op == 4 { 9 } else { 0xd }, 0),
        0x1a => SendError(e, if la == 7 { 9 } else { 10 }, 0),
        0x21 => SendError(e, 8, 0),
        // quirk: 0x19, 0x1c, 0x1f, 0x20 have no case in the second switch: nothing is reported
        0x19 | 0x1c | 0x1f | 0x20 => {}
        _ => SendError(e, 0xd, 0),
    }
    0
}

/// GetFileDescriptor()+0x24: the save file's time stamp
fn file_stamp(e: &mut Engine) -> u32 {
    let d = APILoadSave::GetFileDescriptor(e);
    e.r32(d + 0x24)
}
fn chdir_up(e: &mut Engine) {
    let s = e.cstr("..");
    APILoadSave::ChangeDirectory(e, s);
}
fn chdir_product(e: &mut Engine) {
    let p = APILoadSave::GetProductCode(e);
    APILoadSave::ChangeDirectory(e, p);
}
fn getdir_product(e: &mut Engine) {
    let p = APILoadSave::GetProductCode(e);
    APILoadSave::GetDirectory(e, p);
}
fn read_file(e: &mut Engine) {
    let p = APILoadSave::GetProductCode(e);
    let n = file_size(e);
    APILoadSave::Read(e, p, n as i32, 0);
}
fn set_actions(e: &mut Engine, a: u32, s: u32) {
    sg(e, uiLastAction, a);
    sg(e, uiLastSubAction, s);
}
fn stamp_done(e: &mut Engine) {
    let t = file_stamp(e);
    sg(e, uiTimeStamp, t);
    SendError(e, 0, t);
}

/// 0x000126c8
pub fn CreateSGDataResultHandler(e: &mut Engine, _: u32) -> u32 {
    match g(e, uiLastAction) {
        7 => {
            set_actions(e, 9, 0xb);
            chdir_up(e);
        }
        9 => match g(e, uiLastSubAction) {
            10 => {
                set_actions(e, 0x12, 0);
                APILoadSave::TransferIcons(e);
            }
            0xb => {
                sg(e, uiLastAction, 0x15);
                getdir_product(e);
            }
            _ => {}
        },
        0xf => {
            sg(e, uiLastAction, 0x19);
            let n = g(e, uiSizeToCheck);
            APILoadSave::IsMyFileFit(e, n as i32);
        }
        0x12 => {
            let h = g(e, pSaveGameHeaderBuffer);
            if h == 0 {
                SendError(e, 9, 0);
            } else {
                sg(e, uiLastAction, 7);
                let p = APILoadSave::GetProductCode(e);
                let n = file_size(e);
                APILoadSave::Write(e, p, h, n as i32, 0, 0x203);
            }
        }
        0x13 => {
            set_actions(e, 9, 10);
            chdir_product(e);
        }
        0x15 => stamp_done(e),
        0x19 => {
            sg(e, uiLastAction, 0x1b);
            chdir_up(e);
        }
        0x1b => {
            sg(e, uiLastAction, 0x13);
            let p = APILoadSave::GetProductCode(e);
            APILoadSave::CreateDirectory(e, p);
        }
        _ => {}
    }
    0
}

/// ReadSGHResultHandler (0x00012a00) / ReadSGSResultHandler (0x00012d40): they differ only in the copy
/// of action 0x14 (header vs one slot)
fn read_result(e: &mut Engine, slot: bool) {
    match g(e, uiLastAction) {
        9 => match g(e, uiLastSubAction) {
            0xc => {
                sg(e, uiLastAction, 0x14);
                read_file(e);
            }
            0xb => {
                sg(e, uiLastAction, 0x15);
                getdir_product(e);
            }
            _ => {}
        },
        0xf => {
            set_actions(e, 0x1b, 0xc);
            chdir_up(e);
        }
        0x14 => {
            let b = APILoadSave::GetReadBuffer(e);
            if slot {
                sg(e, pBuffer, b);
                let (dst, id, ss, hs) = (g(e, pSaveGameSlotBuffer), g(e, uiSaveGameSlotId), g(e, uiSizeSaveGameSlot), g(e, uiSizeSaveGameHeader));
                let src = id.wrapping_sub(1).wrapping_mul(ss).wrapping_add(hs).wrapping_add(b);
                memcpy_local(e, dst, src, ss);
            } else {
                let (h, n) = (g(e, pSaveGameHeaderBuffer), g(e, uiSizeSaveGameHeader));
                memcpy_local(e, h, b, n);
            }
            APILoadSave::DestroyReadBuffer(e);
            if slot {
                sg(e, pBuffer, 0);
            }
            set_actions(e, 9, 0xb);
            chdir_up(e);
        }
        0x15 => stamp_done(e),
        0x1b => {
            set_actions(e, 9, 0xc);
            chdir_product(e);
        }
        _ => {}
    }
}
/// 0x00012a00
pub fn ReadSGHResultHandler(e: &mut Engine, _: u32) -> u32 {
    read_result(e, false);
    0
}
/// 0x00012d40
pub fn ReadSGSResultHandler(e: &mut Engine, _: u32) -> u32 {
    read_result(e, true);
    0
}

/// WriteSGHResultHandler (0x00013038) / WriteSGSResultHandler (0x000134c0): they differ only in the
/// slot copy of action 0x14
fn write_result(e: &mut Engine, slot: bool) {
    match g(e, uiLastAction) {
        7 => {
            APILoadSave::DestroyTempBuffer(e);
            sg(e, pBuffer, 0);
            set_actions(e, 9, 0xb);
            chdir_up(e);
        }
        9 => match g(e, uiLastSubAction) {
            0xc => {
                sg(e, uiLastAction, 0x14);
                read_file(e);
            }
            0xb => SendError(e, 0, 0),
            _ => {}
        },
        0xf => {
            sg(e, uiLastAction, 0x1b);
            chdir_up(e);
        }
        0x14 => {
            let n = file_size(e);
            let b = APILoadSave::GetTempBuffer(e, n as i32);
            sg(e, pBuffer, b);
            let r = APILoadSave::GetReadBuffer(e);
            memcpy_local(e, b, r, n);
            APILoadSave::DestroyReadBuffer(e);
            let (h, hs) = (g(e, pSaveGameHeaderBuffer), g(e, uiSizeSaveGameHeader));
            memcpy_local(e, b, h, hs);
            if slot {
                let (id, ss, src) = (g(e, uiSaveGameSlotId), g(e, uiSizeSaveGameSlot), g(e, pSaveGameSlotBuffer));
                let dst = id.wrapping_sub(1).wrapping_mul(ss).wrapping_add(hs).wrapping_add(b);
                memcpy_local(e, dst, src, ss);
            }
            sg(e, uiLastAction, 7);
            let p = APILoadSave::GetProductCode(e);
            APILoadSave::Write(e, p, b, n as i32, 0, 3);
        }
        0x15 => {
            let t = file_stamp(e);
            let ts = g(e, uiTimeStamp);
            if t == ts {
                set_actions(e, 9, 0xc);
                chdir_product(e);
            } else {
                SendError(e, 0xb, ts);
            }
        }
        0x1b => {
            sg(e, uiLastAction, 0x15);
            getdir_product(e);
        }
        _ => {}
    }
}
/// 0x00013038
pub fn WriteSGHResultHandler(e: &mut Engine, _: u32) -> u32 {
    write_result(e, false);
    0
}
/// 0x000134c0
pub fn WriteSGSResultHandler(e: &mut Engine, _: u32) -> u32 {
    write_result(e, true);
    0
}
/// 0x000139a0
pub fn MemCardPresentResultHandler(e: &mut Engine, _: u32) -> u32 {
    if g(e, uiLastAction) == 0xf {
        SendError(e, 0, 0);
    }
    0
}
/// 0x00013a10
pub fn FormatMemCardResultHandler(e: &mut Engine, _: u32) -> u32 {
    match g(e, uiLastAction) {
        1 => SendError(e, 0, 0),
        0xf => {
            sg(e, uiLastAction, 1);
            APILoadSave::FormatCard(e);
        }
        _ => {}
    }
    0
}
/// 0x00013af0
pub fn DeleteSGDataResultHandler(e: &mut Engine, _: u32) -> u32 {
    let la = g(e, uiLastAction);
    match la {
        4 => SendError(e, 0, 0),
        9 => match g(e, uiLastSubAction) {
            0xc => {
                sg(e, uiLastAction, 0x18);
                APILoadSave::DeleteAllFiles(e);
            }
            0xb => {
                sg(e, uiLastAction, 4);
                let p = APILoadSave::GetProductCode(e);
                APILoadSave::DeleteFile(e, p);
            }
            _ => {}
        },
        0xf | 0x1b => {
            if la == 0xf {
                sg(e, uiLastAction, 0x1b);
                chdir_up(e);
                // quirk: case 0xf has no break and falls through into case 0x1b
            }
            set_actions(e, 9, 0xc);
            chdir_product(e);
        }
        0x18 => {
            set_actions(e, 9, 0xb);
            chdir_up(e);
        }
        _ => {}
    }
    0
}
/// 0x00013d50
pub fn GetAvailableMemResultHandler(e: &mut Engine, _: u32) -> u32 {
    match g(e, uiLastAction) {
        0xf => {
            sg(e, uiLastAction, 0x19);
            let n = g(e, uiSizeToCheck);
            APILoadSave::IsMyFileFit(e, n as i32);
        }
        0x19 => SendError(e, 0, 0),
        _ => {}
    }
    0
}
/// 0x00013e00
pub fn GetMemCardNameResultHandler(e: &mut Engine, _: u32) -> u32 {
    0
}
/// 0x00013e28 (only a trace)
pub fn mcEnableReset(e: &mut Engine) {}

/// 0x00013e70 reports to the client and resets the request state
pub fn SendError(e: &mut Engine, param_1: u32, param_2: u32) {
    let cb = g(e, pfnReturnCallback);
    if cb != 0 {
        match e.code_fn(cb) {
            CodeFn::Word2(f) => {
                f(e, param_1, param_2);
            }
            _ => panic!("Api_MemCard: return callback is not a 2-word function"),
        }
    }
    for a in [uiLastAction, uiLastSubAction, pSaveGameSlotBuffer, pSaveGameHeaderBuffer, uiSaveGameSlotId, pfnResultHandler, pString, uiCurrentOperation] {
        sg(e, a, 0);
    }
}

/// 0x00013f08
pub fn api_memcardRegister(e: &mut Engine, api: Ptr) {
    let v = cons::GetApiRegLib(e);
    sg(e, pRegLib, v);
    let v = cons::GetApiExtra(e);
    sg(e, pExtra, v);
    let v = cons::GetApiGameObj(e);
    sg(e, pGameObj, v);
    let v = cons::GetApiLoadSave(e);
    sg(e, pLoadSave, v);
    let n = e.cstr("Api_MemCard");
    APIRegLib::RegisterLib(e, n, 0x40);
    // the 16 slot writes: LIBApi_MemCard dispatches straight to the functions above
}
/// 0x000140c0
pub fn api_memcardConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    let c = e.code_ptr(CodeFn::Word(CompletionCallback));
    APILoadSave::SetCompleteCallBack(e, c);
    let c = e.code_ptr(CodeFn::Word(ErrorCallback));
    APILoadSave::SetErrorCallBack(e, c);
    Ok(())
}
/// 0x00014128
pub fn api_memcardDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    let n = e.cstr("Api_MemCard");
    APIRegLib::UnRegisterLib(e, n);
    Ok(())
}

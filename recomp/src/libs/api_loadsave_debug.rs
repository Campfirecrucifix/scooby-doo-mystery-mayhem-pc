//! Api_LoadSave_Debug custom library (`Game Logic Modules/A2MUtilities/Api_MemCard/api_loadsave_debug__*/
//! Api_LoadSave_Debug.c`, builds 4ae80760 / de1cdde7: identical code and data image). Registers the table
//! "Api_LoadSave_Debug" (0x98): a fake memory card with the APILoadSave interface. Every command answers
//! asynchronously: Intern_ReturnSuccess/Error resets the 0.1 s timer and stores the code; CB_TIMER (external
//! function 1, notified on object 0x2a000000) then calls the complete/error callback with &DAT_101f4.
//!
//! Port notes: the callbacks are code-pointer tokens ([`Engine::code_ptr`], `CodeFn::Word`). Library functions
//! run in the caller's logic context but use this module's globals ([`lib_data`]). Data-relative addresses
//! (oFakeInfo = &oCardInfoPort1_Start, GetFileDescriptor, GetReadBuffer, ...) are relocated through lib_data.
//! No module in the shipped packs fetches this table (GetApiCustom("Api_LoadSave_Debug") never occurs); it
//! only runs its constructor and timer.
//!
//! Globals: oCardInfoPort1_Start 0x10000, oCardInfoPort2_Start 10, oFakeInfo 20 (+24 port2 ptr, +28),
//! file descriptors 2c + slot*0x2c (+50 = 10, +7c = 0), pfnComplete 101e4, pfnError 101e8, port 101ec,
//! slot 101f0, result {success 101f4, error 101f8}, current directory (char[]) 101fc, temp buffer 13645,
//! read buffer 105e5, product code 166a5 ("A2MCODE"), fake file 166b4 (0x388 bytes), written[2] 16a3c,
//! present[2] 16a64, formatted[2] 16a8c, free size[2] 16ab4, uiLoadSaveDebugTimer 16adc,
//! uiSuccessReturnCode 16ae0, uiErrorReturnCode 16ae4, pRegLib 16ae8, pExtra 16aec, pGameObj 16af0,
//! pMisc 16af4, gpoLanguagesNamesAndISOCodes 16af8 (image only, unused).
#![allow(non_snake_case, non_upper_case_globals, unused_variables)]
use super::lib_data;
use crate::engine::core::apicons as cons;
use crate::engine::*;
use crate::logic::{CodeFn, LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "api_loadsave_debug",
    data_size: 0x77d0,
    ctor: api_loadsave_debugConstructor,
    dtor: api_loadsave_debugDestructor,
    register: Some(api_loadsave_debugRegister),
    init: &[],
    build_init: Some(build_init),
};

pub const KEYS: [&str; 2] = ["4ae80760", "de1cdde7"];
pub fn build_index(key: &str) -> Option<usize> {
    KEYS.iter().position(|&k| k == key)
}

const oCardInfoPort1_Start: u32 = 0x10000;
const oCardInfoPort2_Start: u32 = 0x10010;
const oFakeInfo: u32 = 0x10020;
const pfnComplete: u32 = 0x101e4;
const pfnError: u32 = 0x101e8;
const uiPort: u32 = 0x101ec;
const uiSlot: u32 = 0x101f0;
const oResult: u32 = 0x101f4;
const szDirectory: u32 = 0x101fc;
const oTempBuffer: u32 = 0x13645;
const oReadBuffer: u32 = 0x105e5;
const szProductCode: u32 = 0x166a5;
const oFakeFile: u32 = 0x166b4;
const aWritten: u32 = 0x16a3c;
const aPresent: u32 = 0x16a64;
const aFormatted: u32 = 0x16a8c;
const aFreeSize: u32 = 0x16ab4;
const uiLoadSaveDebugTimer: u32 = 0x16adc;
const uiSuccessReturnCode: u32 = 0x16ae0;
const uiErrorReturnCode: u32 = 0x16ae4;
const pRegLib: u32 = 0x16ae8;
const pExtra: u32 = 0x16aec;
const pGameObj: u32 = 0x16af0;
const pMisc: u32 = 0x16af4;

fn a(e: &Engine, addr: u32) -> Ptr {
    lib_data(e, &MODULE, addr)
}
fn g(e: &Engine, addr: u32) -> u32 {
    e.r32(a(e, addr))
}
fn sg(e: &mut Engine, addr: u32, v: u32) {
    let p = a(e, addr);
    e.w32(p, v);
}

/// both builds: same initialised data image (module-relative address, word)
fn build_init(m: &mut Mem, data: Ptr, _b: usize) {
    for (ad, v) in crate::engine::gamedata::c2rs_image(IMAGE_RUNS.0, IMAGE_RUNS.1).words {
        m.w32(data + (ad - 0x10000), v);
    }
}

/// 0x000177d0
pub fn api_loadsave_debugST_BASICLOGIC(e: &mut Engine, _: u32) -> LResult<u64> {
    Ok(0)
}

fn call_cb(e: &mut Engine, cb: u32) {
    let arg = a(e, oResult);
    match e.code_fn(cb) {
        CodeFn::Word(f) => {
            f(e, arg);
        }
        _ => panic!("Api_LoadSave_Debug: callback is not a 1-word function"),
    }
}

/// 0x00017800
pub fn api_loadsave_debugCB_TIMER(e: &mut Engine, _: u32) -> LResult<u64> {
    if g(e, uiSuccessReturnCode) == 0 {
        if g(e, uiErrorReturnCode) != 0 {
            let v = g(e, uiErrorReturnCode);
            sg(e, oResult + 4, v);
            sg(e, uiErrorReturnCode, 0);
            sg(e, oResult, 0);
            let cb = g(e, pfnError);
            call_cb(e, cb);
        }
    } else {
        let v = g(e, uiSuccessReturnCode);
        sg(e, oResult, v);
        sg(e, uiSuccessReturnCode, 0);
        sg(e, oResult + 4, 0);
        let cb = g(e, pfnComplete);
        call_cb(e, cb);
    }
    Ok(0)
}

/// 0x000178d0
fn Intern_ReturnError(e: &mut Engine, param_1: u32) {
    let t = g(e, uiLoadSaveDebugTimer);
    APIextra::TimerReset(e, t);
    sg(e, uiErrorReturnCode, param_1);
}
/// 0x00017928
fn Intern_ReturnSuccess(e: &mut Engine, param_1: u32) {
    let t = g(e, uiLoadSaveDebugTimer);
    APIextra::TimerReset(e, t);
    sg(e, uiSuccessReturnCode, param_1);
}

/// 0x00017980
pub fn CreateDirectory(e: &mut Engine) {
    Intern_ReturnSuccess(e, 2);
}
/// 0x000179c8 appends `param_1` + '\' to the current directory ("..": clears it)
pub fn ChangeDirectory(e: &mut Engine, param_1: Ptr) {
    let dir = a(e, szDirectory);
    if e.r8(param_1) == b'.' && e.r8(param_1 + 1) == b'.' {
        e.w8(dir, 0);
    } else {
        let mut i = 0u32;
        while e.r8(dir + i) != 0 {
            i += 1;
        }
        let mut j = 0u32;
        while e.r8(param_1 + j) != 0 {
            let c = e.r8(param_1 + j);
            e.w8(dir + i, c);
            i += 1;
            j += 1;
        }
        e.w8(dir + i, 0x5c);
        e.w8(dir + i + 1, 0);
    }
    Intern_ReturnSuccess(e, 2);
}
/// 0x00017b50
pub fn DeleteDirectory(e: &mut Engine) {
    Intern_ReturnSuccess(e, 2);
}
/// 0x00017b98
pub fn Write(e: &mut Engine) {
    let s = g(e, uiSlot);
    sg(e, aWritten + s.wrapping_mul(4), 1);
    Intern_ReturnSuccess(e, 2);
}
/// 0x00017c18
pub fn WriteSaveGame(e: &mut Engine) {
    Intern_ReturnSuccess(e, 2);
}
/// 0x00017c68 copies the 0x388-byte fake file into the read buffer, zero-fills up to `param_2`
pub fn Read(e: &mut Engine, param_1: u64, param_2: u32) {
    let s = g(e, uiSlot);
    if s == 0 {
        if g(e, aWritten) == 0 {
            return Intern_ReturnError(e, 0x1e);
        }
    } else if s == 1 && g(e, aWritten + 4) == 0 {
        return Intern_ReturnError(e, 0x1e);
    }
    let (dst, src) = (a(e, oReadBuffer), a(e, oFakeFile));
    let mut i = 0u32;
    while i < 0x388 {
        let b = e.r8(src + i);
        e.w8(dst + i, b);
        i += 1;
    }
    while i < param_2 {
        e.w8(dst + i, 0);
        i += 1;
    }
    Intern_ReturnSuccess(e, 2);
}
/// 0x00017e20
pub fn ReadSaveGame(e: &mut Engine) {
    Intern_ReturnSuccess(e, 2);
}
/// 0x00017e70
pub fn DeleteFile(e: &mut Engine) {
    Intern_ReturnSuccess(e, 2);
}
/// 0x00017eb8
pub fn DeleteAllFiles(e: &mut Engine) {
    Intern_ReturnSuccess(e, 2);
}
/// 0x00017f00
pub fn GetDirectory(e: &mut Engine) {
    Intern_ReturnSuccess(e, 2);
}
/// 0x00017f48
pub fn IsMyFileFit(e: &mut Engine, param_1: i32) {
    let s = g(e, uiSlot);
    if (g(e, aFreeSize + s.wrapping_mul(4)) as i32) < param_1 {
        Intern_ReturnError(e, 0x14);
    } else {
        Intern_ReturnSuccess(e, 2);
    }
}
/// 0x00017fe0
pub fn SetErrorCallBack(e: &mut Engine, param_1: u32) {
    sg(e, pfnError, param_1);
}
/// 0x00018010
pub fn SetCompleteCallBack(e: &mut Engine, param_1: u32) {
    sg(e, pfnComplete, param_1);
}
/// 0x00018040
pub fn GetLastCommand(e: &mut Engine) -> u32 {
    0
}
/// 0x00018070
pub fn GetLastAction(e: &mut Engine) -> u32 {
    0
}
/// 0x000180a0
pub fn GetError(e: &mut Engine) -> u32 {
    0
}
/// 0x000180d0
pub fn GetStatus(e: &mut Engine) -> u32 {
    0
}
/// 0x00018100
pub fn GetReadBuffer(e: &mut Engine) -> Ptr {
    a(e, oReadBuffer)
}
/// 0x00018130
pub fn DestroyReadBuffer(e: &mut Engine) {}
/// 0x00018150
pub fn SetWorkingMemoryCard(e: &mut Engine, param_1: u32, param_2: i32) {
    let dir = a(e, szDirectory);
    e.w8(dir, 0x2e);
    e.w8(dir + 1, 0);
    sg(e, uiPort, param_1);
    sg(e, uiSlot, param_2 as u32);
    let off = (param_2 as u32).wrapping_mul(4);
    if g(e, aPresent + off) == 0 {
        Intern_ReturnError(e, 0x12);
    } else if g(e, aFormatted + off) == 0 {
        Intern_ReturnError(e, 0x13);
    } else {
        Intern_ReturnSuccess(e, 2);
    }
}
/// 0x00018250
pub fn GetPortActuel(e: &mut Engine) -> u32 {
    g(e, uiPort)
}
/// 0x00018288
pub fn GetSlotActuel(e: &mut Engine) -> u32 {
    g(e, uiSlot)
}
/// 0x000182c0
pub fn TransferIcons(e: &mut Engine) {
    Intern_ReturnSuccess(e, 2);
}
/// 0x00018308
pub fn FormatCard(e: &mut Engine) {
    let s = g(e, uiSlot);
    sg(e, aFormatted + s.wrapping_mul(4), 1);
    Intern_ReturnSuccess(e, 2);
}
/// 0x00018370
pub fn GetCardStatus(e: &mut Engine) {}
/// 0x00018390
pub fn GetPort1CardInfoBuffer(e: &mut Engine) -> u32 {
    g(e, oFakeInfo)
}
/// 0x000183c8
pub fn GetPort2CardInfoBuffer(e: &mut Engine) -> u32 {
    g(e, oFakeInfo + 4)
}
/// 0x00018400
pub fn GetPort1ActiveMemoryCard(e: &mut Engine) -> u32 {
    g(e, uiSlot)
}
/// 0x00018438
pub fn GetPort2ActiveMemoryCard(e: &mut Engine) -> u32 {
    g(e, uiSlot)
}
/// 0x00018470
pub fn GetFileDescriptor(e: &mut Engine) -> Ptr {
    a(e, 0x1002c) + g(e, uiSlot).wrapping_mul(0x2c)
}
/// 0x000184b8
pub fn GetProductCode(e: &mut Engine) -> Ptr {
    a(e, szProductCode)
}
/// 0x000184e8
pub fn GetTempBuffer(e: &mut Engine) -> Ptr {
    a(e, oTempBuffer)
}
/// 0x00018520
pub fn DestroyTempBuffer(e: &mut Engine) {}

/// 0x00018540
pub fn api_loadsave_debugRegister(e: &mut Engine, api: Ptr) {
    let v = cons::GetApiRegLib(e);
    sg(e, pRegLib, v);
    let n = e.cstr("Api_LoadSave_Debug");
    APIRegLib::RegisterLib(e, n, 0x98);
    // the 33 slot writes: LIBApi_LoadSave_Debug (api_gen.rs) is meant to dispatch to the functions above
}

/// 0x000187c0
pub fn api_loadsave_debugConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    cons::SetExternalFunction(e, 0, api_loadsave_debugST_BASICLOGIC);
    cons::SetExternalFunction(e, 1, api_loadsave_debugCB_TIMER);
    let v = cons::GetApiGameObj(e);
    sg(e, pGameObj, v);
    let v = cons::GetApiExtra(e);
    sg(e, pExtra, v);
    let n = e.cstr("MiscFunc");
    let v = cons::GetApiCustom(e, n);
    sg(e, pMisc, v);
    sg(e, uiSuccessReturnCode, 0);
    sg(e, uiErrorReturnCode, 0);
    let pc = a(e, szProductCode);
    for (i, &c) in b"A2MCODE\0".iter().enumerate() {
        e.w8(pc + i as u32, c);
    }
    let dir = a(e, szDirectory);
    e.w8(dir, 0);
    e.w8(dir + 1, 0);
    for o in [0, 4, 8, 0xc, 0x10] {
        sg(e, oFakeFile + o, 0);
    }
    let t = APIextra::TimerCreate(e);
    sg(e, uiLoadSaveDebugTimer, t);
    APIextra::TimerReset(e, t);
    APIextra::TimerSetFrequency(e, t, 0.1);
    APIextra::TimerRegisterNotify(e, 0x2a00_0000, t, 1);
    let p1 = a(e, oCardInfoPort1_Start);
    sg(e, oFakeInfo, p1);
    let p2 = a(e, oCardInfoPort2_Start);
    sg(e, oFakeInfo + 4, p2);
    let fi = a(e, oFakeInfo);
    sg(e, oFakeInfo + 8, fi);
    sg(e, uiPort, 0);
    sg(e, uiSlot, 0);
    for (ad, v) in [
        (aPresent, 1),
        (aPresent + 4, 1),
        (aFormatted, 1),
        (aFormatted + 4, 1),
        (aWritten, 1),
        (aWritten + 4, 0),
        (aFreeSize, 8000),
        (aFreeSize + 4, 8000),
        (0x10050, 10),
        (0x1007c, 0),
    ] {
        sg(e, ad, v);
    }
    APIgameobj::ResetLogicFlag(e, 0x2a00_0000, 0xe3a);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, 0x2a00_0000, 2, NULL, 0);
    cons::SetInitialState(e, 0);
    Ok(())
}

/// 0x00018ab0
pub fn api_loadsave_debugDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    let n = e.cstr("Api_LoadSave_Debug");
    APIRegLib::UnRegisterLib(e, n);
    let t = g(e, uiLoadSaveDebugTimer);
    APIextra::TimerDestroy(e, t);
    Ok(())
}

/// initialised data image of both builds (mostly the language name table): module address runs whose
/// non-zero words are read from the module on the disc image (WP-X1).
static IMAGE_RUNS: (&str, &[(u32, u32)]) = ("4ae80760", &[(0x10000, 0x177c8)]);

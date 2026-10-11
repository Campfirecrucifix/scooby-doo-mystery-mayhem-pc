//! APIcons (EXE.1 Engine API (module interface)/EFApi_cons.cpp)
use crate::engine::{Engine, Ptr, NULL};
use crate::logic::{ExtFn, MsgFn};

/// Address the engine passes to module constructors as `APIcons *` (0x1f71630 on the PS2).
pub const API_CONS: Ptr = 0x01f7_1630;
/// Stand-in addresses of the API tables returned by GetApiXxx (modules only store them).
const API_TABLE_BASE: Ptr = 0x01f7_0000;

fn top_kind(e: &Engine) -> u8 {
    let t = e.rt.stack.top;
    if t == 0 { 0 } else { e.rt.stack.kinds[t - 1] }
}

/// slot +0x0: function `idx` of the running module's code (states and callbacks)
pub fn SetExternalFunction(e: &mut Engine, a0: i32, a1: ExtFn) -> u32 {
    let c = e.rt.current_code().expect("SetExternalFunction outside logic");
    let f = &mut e.rt.codes[c].funcs;
    let i = a0 as usize;
    if f.len() <= i {
        f.resize(i + 1, None);
    }
    f[i] = Some(a1);
    0
}
/// slot +0x4: only honoured inside a constructor (kind 4) or kind 0x80
pub fn GetGlobalData(e: &mut Engine, a0: Ptr, a1: i32) -> u32 {
    if top_kind(e) & 0x84 != 0 {
        let c = e.rt.current_code().unwrap();
        e.InitGlobalData(c, a0, a1 as u32);
    }
    0
}
/// slot +0x8: only honoured inside a constructor (kind 4) or kind 0x80
pub fn GetLocalData(e: &mut Engine, a0: Ptr, a1: i32) -> u32 {
    if top_kind(e) & 0x84 != 0 {
        let h = e.rt.current_instance().unwrap();
        e.InitLocalData(h, a0, a1 as u32);
    }
    0
}
/// slot +0xc: only inside a constructor
pub fn SetInitialState(e: &mut Engine, a0: i32) -> u32 {
    if top_kind(e) & 4 != 0 {
        let h = e.rt.current_instance().unwrap();
        e.rt.inst(h).state = a0 as u32;
    }
    0
}
/// slot +0x10
pub fn SetMessageFunction(e: &mut Engine, a0: MsgFn) -> u32 {
    let h = e.rt.current_instance().expect("SetMessageFunction outside logic");
    e.rt.inst(h).msg_fn = Some(a0);
    0
}
pub fn GetApiStdlib(e: &mut Engine) -> Ptr { API_TABLE_BASE + 0x000 }
pub fn GetApiInput(e: &mut Engine) -> Ptr { API_TABLE_BASE + 0x100 }
pub fn GetApiMath(e: &mut Engine) -> Ptr { API_TABLE_BASE + 0x200 }
pub fn GetApiGameObj(e: &mut Engine) -> Ptr { API_TABLE_BASE + 0x300 }
pub fn GetApiExtra(e: &mut Engine) -> Ptr { API_TABLE_BASE + 0x400 }
pub fn GetApiAnim(e: &mut Engine) -> Ptr { API_TABLE_BASE + 0x500 }
pub fn GetApiCamera(e: &mut Engine) -> Ptr { API_TABLE_BASE + 0x600 }
pub fn GetApiLight(e: &mut Engine) -> Ptr { API_TABLE_BASE + 0x700 }
pub fn GetApiPhysic(e: &mut Engine) -> Ptr { API_TABLE_BASE + 0x800 }
pub fn GetApiNewSound(e: &mut Engine) -> Ptr { API_TABLE_BASE + 0x900 }
pub fn GetApiParticle(e: &mut Engine) -> Ptr { API_TABLE_BASE + 0xa00 }
pub fn GetApiProjector(e: &mut Engine) -> Ptr { API_TABLE_BASE + 0xb00 }
pub fn GetApiConsole(e: &mut Engine) -> Ptr { API_TABLE_BASE + 0xc00 }
pub fn GetApiHUD(e: &mut Engine) -> Ptr { API_TABLE_BASE + 0xd00 }
pub fn GetApiGame(e: &mut Engine) -> Ptr { API_TABLE_BASE + 0xe00 }
pub fn GetApiQA(e: &mut Engine) -> Ptr { API_TABLE_BASE + 0xf00 }
pub fn GetApiFBEffects(e: &mut Engine) -> Ptr { API_TABLE_BASE + 0x1000 }
pub fn GetApiGraphics(e: &mut Engine) -> Ptr { API_TABLE_BASE + 0x1100 }
pub fn GetApiKaraoke(e: &mut Engine) -> Ptr { API_TABLE_BASE + 0x1200 }
pub fn GetApiSimpleInput(e: &mut Engine) -> Ptr { API_TABLE_BASE + 0x1300 }
pub fn GetApiLoadSave(e: &mut Engine) -> Ptr { API_TABLE_BASE + 0x1400 }
pub fn GetApiRegLib(e: &mut Engine) -> Ptr { API_TABLE_BASE + 0x1500 }
pub fn GetApiSpline(e: &mut Engine) -> Ptr { API_TABLE_BASE + 0x1600 }
/// slot +0x80: EFCustomLibBank::GetLib(name): the table of the newest library registered under that
/// name (strncmp, 0x20 chars), NULL if none. Calls through a table go to the ported library code
/// (crate::libs) via the generated LIB* traits; the table words themselves are not filled in.
pub fn GetApiCustom(e: &mut Engine, a0: Ptr) -> Ptr {
    super::apireglib::find_lib(e, a0).map_or(NULL, |i| e.rt.lib_bank[i].table)
}
/// slot +0x88
pub fn GetExternalFunction(e: &mut Engine, a0: i32) -> u32 {
    // returns the function pointer in the original; only its presence is observable here
    let c = e.rt.current_code().unwrap();
    e.GetFunction(c, a0 as u32).is_some() as u32
}

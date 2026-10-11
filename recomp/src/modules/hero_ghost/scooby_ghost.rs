//! scooby_ghost.c / shaggy_ghost.c (5 functions each, same code in all 8 builds apart from the
//! character name and Scooby's MSG_FOLLOW_SET debug print). Addresses: scooby_ghost__08a011d0.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// 0x00028208 (32 bytes) `void heroFUNC_CheckActionEventsLocal(void)`: empty in every ghost build
pub fn heroFUNC_CheckActionEventsLocal(e: &mut Engine) -> LResult<()> {
    Ok(())
}

/// 0x00028228 (72 bytes) `void scooby_ghostFUNC_Create_Local(void)` (shaggy: same, Shaggy tables
/// which live at the Scooby slots, see mod.rs)
pub fn scooby_ghostFUNC_Create_Local(e: &mut Engine) -> LResult<()> {
    let l = L(e);
    let (s, g_) = (G(e, goSurfaceSoundScoobyGhost), G(e, goGenericSoundScoobyGhost));
    e.w32(l, s);
    e.w32(l + 4, g_);
    Ok(())
}

/// 0x00028270 (188 bytes) `int scooby_ghostFUNC_MessageHandler(uint param_1,uint param_2,int *param_3)`
pub fn scooby_ghostFUNC_MessageHandler(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    if param_1 == 0x7d && g(e, G_scooby_ghostFUNC_MessageHandler) == 0 {
        debug_info(e, "MSG_FOLLOW_SET");
    }
    Ok(heroFUNC_MessageHandler(e, param_1, param_2, param_3)? as u32 as u64)
}

/// 0x00028260 (112 bytes, 17126826) `int shaggy_ghostFUNC_MessageHandler(...)`
pub fn shaggy_ghostFUNC_MessageHandler(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    Ok(heroFUNC_MessageHandler(e, param_1, param_2, param_3)? as u32 as u64)
}

/// 0x00028330 (168 bytes) `void scooby_ghostConstructor(APIcons *api,int instanceCount)`
pub fn scooby_ghostConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    heroFUNC_Constructor(e, api, instanceCount)?;
    APIcons::SetMessageFunction(e, scooby_ghostFUNC_MessageHandler);
    scooby_ghostFUNC_Create_Local(e)?;
    scooby_ghostFUNC_MessageHandler(e, 0xd2, 0x195, NULL)?;
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x000282d0 (168 bytes, 17126826) `void shaggy_ghostConstructor(APIcons *api,int instanceCount)`
pub fn shaggy_ghostConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    heroFUNC_Constructor(e, api, instanceCount)?;
    APIcons::SetMessageFunction(e, shaggy_ghostFUNC_MessageHandler);
    scooby_ghostFUNC_Create_Local(e)?;
    shaggy_ghostFUNC_MessageHandler(e, 0xd2, 0x195, NULL)?;
    APIcons::SetInitialState(e, 0);
    Ok(())
}

/// 0x000283d8 (72 bytes) `void scooby_ghostDestructor(int instanceCount)` (shaggy identical)
pub fn scooby_ghostDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    heroFUNC_Destructor(e, instanceCount)
}
pub fn shaggy_ghostDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    heroFUNC_Destructor(e, instanceCount)
}

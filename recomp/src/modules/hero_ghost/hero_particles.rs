//! hero_particles.c (3 functions). Addresses are module-relative (.text at 0x10000) in scooby_ghost__08a011d0.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// 0x000221b0 (36 bytes) `void heroFUNC_LinkHandleFootStepFromApi(void)`
pub fn heroFUNC_LinkHandleFootStepFromApi(_e: &mut Engine) -> LResult<()> {
    Ok(())
}

/// 0x000221d8 (100 bytes) `void heroFUNC_LaunchParticles(void)`
pub fn heroFUNC_LaunchParticles(e: &mut Engine) -> LResult<()> {
    heroFUNC_GetSurface(e)?;
    heroDisplayFloorSurface(e)
}

/// 0x00022240 (36 bytes) `void heroDisplayFloorSurface(void)`
pub fn heroDisplayFloorSurface(_e: &mut Engine) -> LResult<()> {
    Ok(())
}

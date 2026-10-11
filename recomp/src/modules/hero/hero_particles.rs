//! hero_particles.c (3 functions). Same code in all builds.
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// 0x000251e8 (36 bytes) `void heroFUNC_LinkHandleFootStepFromApi(void)` -- empty in the original.
pub fn heroFUNC_LinkHandleFootStepFromApi(_e: &mut Engine) -> LResult<()> {
    Ok(())
}

/// 0x00025210 (100 bytes) `void heroFUNC_LaunchParticles(void)` -- the surface result is unused.
pub fn heroFUNC_LaunchParticles(e: &mut Engine) -> LResult<()> {
    heroFUNC_GetSurface(e)?;
    heroDisplayFloorSurface(e)
}

/// 0x00025278 (36 bytes) `void heroDisplayFloorSurface(void)` -- empty in the original.
pub fn heroDisplayFloorSurface(_e: &mut Engine) -> LResult<()> {
    Ok(())
}

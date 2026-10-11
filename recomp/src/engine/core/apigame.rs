//! APIgame slots that belong to the frame loop (EXE.1 EFApi_Game.cpp).
use crate::engine::Engine;

/// EFApi_Game.cpp:178: the filtered frame time of EFEngineController (+0x2b8), see
/// [`crate::logic::frame::FrameClock`]
pub fn GetFrameDeltaTime(e: &mut Engine) -> f32 {
    e.clock.dt
}

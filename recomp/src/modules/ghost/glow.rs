//! Glow.c (6 functions): settings of the ghost glow particle types. Same code in all 31 builds.
//! Addresses are module-relative (.text at 0x10000) in smallghostlevelone__757255de.
//!
//! `param_1` is the APIparticle table, called through raw offsets in the original (+0xc
//! TypeSetInter, +0x10 TypeSetInt, +0x18 TypeSetFloat, +0x1c TypeSetVec); the calls go through the
//! [`APIparticle`] trait. Float arguments are in $f12..$f15 in the order Ghidra shows them (as in
//! tod_beam's ToD_Beam_Set_Particule).
use crate::engine::*;
use crate::logic::LResult;

/// The body every SetParticle* shares; they differ in the xy of interpolators 2 and 4, the w of
/// interpolator 0x10 and float 2.
fn set_particle(e: &mut Engine, param_2: u32, inter2_xy: u32, inter4_xy: u32, inter10_w: u32, float2: u32) {
    let t = param_2 as i32;
    let h = f32::from_bits;
    APIparticle::TypeSetInter(e, t, 1, 0.0, 0.0, 0.0, h(0x3e2e_147b));
    APIparticle::TypeSetInter(e, t, 2, h(inter2_xy), h(inter2_xy), 0.0, h(0x3e2e_147b));
    APIparticle::TypeSetInter(e, t, 4, h(inter4_xy), h(inter4_xy), 0.0, h(0x3e2e_147b));
    APIparticle::TypeSetInter(e, t, 8, 0.0, 0.0, 0.0, 0.0);
    APIparticle::TypeSetInter(e, t, 0x10, 1.5, 0.75, h(0x3dcc_cccd), h(inter10_w));
    APIparticle::TypeSetInter(e, t, 0x20, 0.0, 0.0, 0.0, 0.0);
    APIparticle::TypeSetInt(e, t, 1, 0);
    APIparticle::TypeSetInt(e, t, 4, 0);
    APIparticle::TypeSetInt(e, t, 8, 1);
    APIparticle::TypeSetFloat(e, t, 1, 0.0);
    APIparticle::TypeSetFloat(e, t, 2, h(float2));
    APIparticle::TypeSetFloat(e, t, 4, h(0x3d0f_5c29));
    APIparticle::TypeSetFloat(e, t, 8, 0.0);
    APIparticle::TypeSetVec(e, t, 1, 0.0, 0.0, 0.0);
    APIparticle::TypeSetVec(e, t, 2, 0.0, 0.0, 0.0);
}

/// 0x00013008
pub fn SetParticle5(e: &mut Engine, param_1: Ptr, param_2: u32) -> LResult<()> {
    set_particle(e, param_2, 0, 0x3e99_999a, 0x3e2e_147b, 0x3f28_f5c3);
    Ok(())
}

/// 0x00013288
pub fn SetParticle6(e: &mut Engine, param_1: Ptr, param_2: u32) -> LResult<()> {
    set_particle(e, param_2, 0, 0x3e99_999a, 0x3e4c_cccd, 0x3f80_0000);
    Ok(())
}

/// 0x00013500
pub fn SetParticle5Sml(e: &mut Engine, param_1: Ptr, param_2: u32) -> LResult<()> {
    set_particle(e, param_2, 0x3e19_999a, 0, 0x3e2e_147b, 0x3ea8_f5c3);
    Ok(())
}

/// 0x00013780
pub fn SetParticle5Big(e: &mut Engine, param_1: Ptr, param_2: u32) -> LResult<()> {
    set_particle(e, param_2, 0x3e99_999a, 0, 0x3e2e_147b, 0x3ea8_f5c3);
    Ok(())
}

/// 0x00013a00
pub fn SetParticle6Big(e: &mut Engine, param_1: Ptr, param_2: u32) -> LResult<()> {
    set_particle(e, param_2, 0x3e99_999a, 0, 0x3e4c_cccd, 0x3f00_0000);
    Ok(())
}

/// 0x00013c78 (the same settings as SetParticle6Big)
pub fn SetParticle4Big(e: &mut Engine, param_1: Ptr, param_2: u32) -> LResult<()> {
    set_particle(e, param_2, 0x3e99_999a, 0, 0x3e4c_cccd, 0x3f00_0000);
    Ok(())
}

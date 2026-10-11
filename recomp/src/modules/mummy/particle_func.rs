//! Particle_Func.c (6 functions, identical in every build): the mummy's dust particles.
//! Local data: L+500 "particles not created" flag (1 = none), L+0x200..0x218 the 7 types,
//! L+0x21c..0x278 the 24 emitters, L+0x27c / L+0x288 the two eye-glow offsets.
//! NOTE (PS2 behaviour kept): Destroy_Particles destroys "emitters" L+0x200..L+0x25c (it starts at the
//! type slots, so the types are destroyed twice and the last 8 emitters are never destroyed).
use super::*;
use crate::engine::*;
use crate::logic::LResult;

fn particle_api(e: &Engine) -> u32 {
    e.r32(G(e, pParticle))
}

/// emitter of type L+`ty` into L+`slot`, type flag 0x80, joint 0x6fffffff, child of bone `bone`
/// (flags before joint, as in the original)
fn emitter(e: &mut Engine, iVar3: Ptr, slot: u32, ty: u32, bone: u32) -> u32 {
    let t = e.r32(iVar3 + ty) as i32;
    let uVar2 = APIparticle::EmitterCreate(e, t);
    e.w32(iVar3 + slot, uVar2);
    let p = particle_api(e) as i32;
    let em = e.r32(iVar3 + slot);
    LIBMiscFunc::MiscFunc_EmitterUpdateFlags(e, p, em, 0x80, 1);
    APIparticle::EmitterSetJoint(e, em as i32, SELF, 0x6fff_ffff);
    APIgameobj::AddChild(e, SELF, bone, em, 1)
}

/// eye glow: type L+0x20c, joint, child of `bone`, translation L+`ofs`, flags 0x80 on / 2 off, start
fn eye(e: &mut Engine, iVar3: Ptr, slot: u32, bone: u32, ofs: u32) {
    let t = e.r32(iVar3 + 0x20c) as i32;
    let uVar2 = APIparticle::EmitterCreate(e, t);
    e.w32(iVar3 + slot, uVar2);
    APIparticle::EmitterSetJoint(e, uVar2 as i32, SELF, 0x6fff_ffff);
    APIgameobj::AddChild(e, SELF, bone, uVar2, 1);
    APIgameobj::SetTranslation(e, uVar2, iVar3 + ofs);
    let p = particle_api(e) as i32;
    LIBMiscFunc::MiscFunc_EmitterUpdateFlags(e, p, uVar2, 0x80, 1);
    LIBMiscFunc::MiscFunc_EmitterUpdateFlags(e, p, uVar2, 2, 0);
    APIparticle::EmitterStart(e, uVar2 as i32, 1);
}

/// 0x000145f8 the 7 types (BrasDust, BrasDustBlur, PiedDustBlur, Yeux, PiedDust, ShockWave_Dust,
/// ShockWave_DustBlur), arm dust on bones 0xc..0xf / 0x12..0x15, foot dust on 0x1b / 0x20, the two
/// eye glows (bones 7 / 8, x +-0.04) and the shock-wave dust on 0x1d. Returns L, or the last AddChild
/// result when the particles were created.
pub fn Create_Particles(e: &mut Engine) -> LResult<i32> {
    let iVar3 = L(e);
    if e.r32(L(e) + 500) == 0 {
        return Ok(iVar3 as i32);
    }
    e.w32(iVar3 + 500, 0);
    for (o, v) in [(0x27c, 0), (0x280, 0), (0x284, 0x3d23_d70a), (0x288, 0), (0x28c, 0), (0x290, 0xbd23_d70a)] {
        e.w32(iVar3 + o, v);
    }
    let types: [(u32, &str, &str, fn(&mut Engine, u32, u32) -> LResult<()>); 7] = [
        (0x200, "dust", "BrasDust", SetParticle_01),
        (0x204, "dust_blur", "BrasDustBlur", SetParticle_02),
        (0x208, "dust_blur", "PiedDustBlur", SetParticle_03),
        (0x20c, "glow", "Yeux", SetParticle_04),
        (0x210, "dust", "PiedDust", SetParticle_05),
        (0x214, "dust", "ShockWave_Dust", SetParticle_06),
        (0x218, "dust_blur", "ShockWave_DustBlur", SetParticle_07),
    ];
    for (o, tex, name, set) in types {
        let uVar2 = APIparticle::TypeCreate(e);
        e.w32(iVar3 + o, uVar2);
        let s = e.cstr(tex);
        APIparticle::TypeSetTexture(e, uVar2 as i32, s);
        let s = e.cstr(name);
        APIparticle::TypeSetName(e, uVar2 as i32, s);
        let p = particle_api(e);
        set(e, p, uVar2)?;
    }
    let ems: [(u32, u32, u32); 20] = [
        (0x21c, 0x200, 0xc),
        (0x220, 0x200, 0xd),
        (0x224, 0x200, 0xe),
        (0x228, 0x200, 0xf),
        (0x22c, 0x204, 0xc),
        (0x230, 0x204, 0xd),
        (0x234, 0x204, 0xe),
        (0x238, 0x204, 0xf),
        (0x23c, 0x200, 0x12),
        (0x240, 0x200, 0x13),
        (0x244, 0x200, 0x14),
        (0x248, 0x200, 0x15),
        (0x24c, 0x204, 0x12),
        (0x250, 0x204, 0x13),
        (0x254, 0x204, 0x14),
        (600, 0x204, 0x15),
        (0x25c, 0x208, 0x1b),
        (0x260, 0x210, 0x1b),
        (0x264, 0x208, 0x20),
        (0x268, 0x210, 0x20),
    ];
    for (slot, ty, bone) in ems {
        emitter(e, iVar3, slot, ty, bone);
    }
    eye(e, iVar3, 0x26c, 7, 0x27c);
    eye(e, iVar3, 0x270, 8, 0x288);
    emitter(e, iVar3, 0x274, 0x214, 0x1d);
    let r = emitter(e, iVar3, 0x278, 0x218, 0x1d);
    Ok(r as i32)
}

/// 0x00015888
pub fn Destroy_Particles(e: &mut Engine) -> LResult<()> {
    let iVar2 = L(e);
    if e.r32(L(e) + 500) == 0 {
        e.w32(iVar2 + 500, 1);
        for i in 0..7 {
            let t = e.r32(iVar2 + 0x200 + i * 4) as i32;
            APIparticle::TypeDestroy(e, t);
        }
        for i in 0..0x18 {
            let em = e.r32(iVar2 + 0x200 + i * 4) as i32;
            APIparticle::EmitterDestroy(e, em);
        }
    }
    Ok(())
}

fn start_all(e: &mut Engine, slots: &[u32]) {
    let l = L(e);
    for &o in slots {
        let em = e.r32(l + o) as i32;
        APIparticle::EmitterStart(e, em, 0);
    }
}

/// 0x000159b8 right arm + right foot
pub fn Particles_Drop_RT(e: &mut Engine) -> LResult<()> {
    start_all(e, &[0x21c, 0x220, 0x224, 0x228, 0x22c, 0x230, 0x234, 0x238, 0x25c, 0x260]);
    Ok(())
}

/// 0x00015b10 left arm + left foot
pub fn Particles_Drop_LT(e: &mut Engine) -> LResult<()> {
    start_all(e, &[0x23c, 0x240, 0x244, 0x248, 0x24c, 0x250, 0x254, 600, 0x264, 0x268]);
    Ok(())
}

/// 0x00015c68
pub fn Particles_LeftArm(e: &mut Engine) -> LResult<()> {
    start_all(e, &[0x23c, 0x240, 0x244, 0x248, 0x24c, 0x250, 0x254, 600]);
    Ok(())
}

/// 0x00015d88
pub fn Particles_RightArm(e: &mut Engine) -> LResult<()> {
    start_all(e, &[0x21c, 0x220, 0x224, 0x228, 0x22c, 0x230, 0x234, 0x238]);
    Ok(())
}

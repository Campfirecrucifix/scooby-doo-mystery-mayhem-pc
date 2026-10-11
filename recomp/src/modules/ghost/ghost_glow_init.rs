//! GhostGlowInit.c (8 functions): the glow around a ghost, a group of additive "glow" sprites on
//! its joints plus a few trail particle emitters, per monster type (L+0x44: 1 small, 2 medium, 3 big,
//! 0x13 the Ep6 boss, sprites only). Same code in all 31 builds. Addresses are module-relative
//! (.text at 0x10000) in smallghostlevelone__757255de.
//!
//! Local data: +0x1a8..0x1b8 particle types, +0x1bc sprite group, +0x1d0 + i*4 sprite / emitter of
//! glow i, +0x2dc + i*4 its joint, +0x3e8 + i*0xc its offset, +0x70c + i*4 boss sprite sizes.
use super::emitter_pos::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// 0x00013ef0
pub fn CreateGlow(e: &mut Engine) -> LResult<()> {
    CreateEmitterPos(e)?;
    InitParticules(e)?;
    CreateLight(e)
}

/// 0x00013f50 destroy the glow. NOTE: EmitterDestroy is called on every slot +0x1d0 + i*4 for i
/// below 50 / 36 / 65 / 15, sprite handles included (and, for the small ghost, 2 slots past the 34
/// that InitParticules fills): as in the original; type 0x1a8 is destroyed without ever being created.
pub fn DestroyGlow(e: &mut Engine) -> LResult<()> {
    let uVar1 = e.r32(LM(e) + 0x44);
    let (n, types): (i8, &[u32]) = match uVar1 {
        2 => (0x32, &[0x1a8, 0x1ac, 0x1b0, 0x1b4, 0x1b8]),
        1 => (0x24, &[0x1a8, 0x1ac, 0x1b4]),
        3 => (0x41, &[0x1a8, 0x1ac, 0x1b4, 0x1b8, 0x1b0]),
        0x13 => (0xf, &[]),
        _ => return Ok(()),
    };
    let mut cStack_30: i8 = 0;
    while cStack_30 < n {
        let id = e.r32(L(e) + 0x1d0 + (cStack_30 as i32 * 4) as u32);
        APIparticle::EmitterDestroy(e, id as i32);
        cStack_30 += 1;
    }
    let gr = e.r32(L(e) + 0x1bc);
    APIgraphics::SpriteRemoveGroup(e, gr);
    for &o in types {
        let t = e.r32(L(e) + o);
        APIparticle::TypeDestroy(e, t as i32);
    }
    Ok(())
}

/// 0x000143d8 the joints and offsets of the glow points (constant tables, see emitter_pos.rs)
pub fn CreateEmitterPos(e: &mut Engine) -> LResult<()> {
    let uVar1 = e.r32(LM(e) + 0x44);
    let t: &[(u32, u32)] = match uVar1 {
        2 => EMITTER_POS_2,
        1 => EMITTER_POS_1,
        3 => EMITTER_POS_3,
        0x13 => EMITTER_POS_13,
        _ => return Ok(()),
    };
    let l = L(e);
    for &(o, v) in t {
        e.w32(l + o, v);
    }
    Ok(())
}

/// 0x000178f0 emitter flag 0x100 (clipped with the object)
pub fn SetEmetterClipping(e: &mut Engine, param_1: i32) -> LResult<()> {
    let b = e.alloca(0xc);
    APIparticle::EmitterGetFlags(e, param_1, b);
    let f = e.r32(b) | 0x100;
    APIparticle::EmitterSetFlags(e, param_1, f);
    Ok(())
}

/// the sprite group at L+0x1bc: texture "glow", colour, additive, flags 6
fn glow_group(e: &mut Engine, r: f32, g: f32, b: f32) {
    let uVar1 = APIgraphics::SpriteAddGroup(e);
    let l = L(e);
    e.w32(l + 0x1bc, uVar1);
    let gr = e.r32(l + 0x1bc);
    let s = e.cstr("glow");
    APIgraphics::SpriteSetGroupTexture(e, gr, s);
    APIgraphics::SpriteSetGroupColor(e, gr, r, g, b, 0.0);
    APIgraphics::SpriteSetGroupBlendMode(e, gr, 1);
    APIgraphics::SpriteSetGroupFlags(e, gr, 6);
}

/// particle type at L+`slot`: texture "glow", `name`, settings `set`
fn glow_type(e: &mut Engine, slot: u32, name: &str, set: fn(&mut Engine, Ptr, u32) -> LResult<()>) -> LResult<()> {
    let uVar1 = APIparticle::TypeCreate(e);
    let l = L(e);
    e.w32(l + slot, uVar1);
    let t = e.r32(l + slot) as i32;
    let s = e.cstr("glow");
    APIparticle::TypeSetTexture(e, t, s);
    let n = e.cstr(name);
    APIparticle::TypeSetName(e, t, n);
    let p = e.r32(G(e, pParticle));
    let t = e.r32(L(e) + slot);
    set(e, p, t)
}

/// glow sprite i (size `size`) on joint L+0x2dc + i*4 at offset L+0x3e8 + i*0xc
fn glow_sprite(e: &mut Engine, i: u32, size: f32) {
    let gr = e.r32(L(e) + 0x1bc);
    let uVar1 = APIgraphics::SpriteGroupAddSprite(e, gr, SELF, 0x6fff_ffff, size);
    let l = L(e);
    e.w32(l + 0x1d0 + i * 4, uVar1);
    let (j, sp) = (e.r32(l + 0x2dc + i * 4), e.r32(l + 0x1d0 + i * 4));
    APIgameobj::AddChild(e, SELF, j, sp, 1);
    let sp = e.r32(L(e) + 0x1d0 + i * 4);
    let p = i * 0xc + 1000 + L(e);
    APIgameobj::SetTranslation(e, sp, p);
}

/// glow i as a started emitter of the particle type at L+`ty`; returns EmitterStart's result
fn glow_emitter(e: &mut Engine, i: u32, ty: u32) -> LResult<u32> {
    let t = e.r32(L(e) + ty);
    let uVar1 = APIparticle::EmitterCreate(e, t as i32);
    let l = L(e);
    e.w32(l + 0x1d0 + i * 4, uVar1);
    let em = e.r32(l + 0x1d0 + i * 4);
    SetEmetterClipping(e, em as i32)?;
    let em = e.r32(L(e) + 0x1d0 + i * 4);
    APIparticle::EmitterSetJoint(e, em as i32, SELF, 0x6fff_ffff);
    let l = L(e);
    let (j, em) = (e.r32(l + 0x2dc + i * 4), e.r32(l + 0x1d0 + i * 4));
    APIgameobj::AddChild(e, SELF, j, em, 1);
    let em = e.r32(L(e) + 0x1d0 + i * 4);
    let p = L(e) + 1000 + i * 0xc;
    APIgameobj::SetTranslation(e, em, p);
    let em = e.r32(L(e) + 0x1d0 + i * 4);
    Ok(APIparticle::EmitterStart(e, em as i32, 1))
}

/// 0x00017960 create the glow (the return value is unused by CreateGlow: EmitterStart's result,
/// 15 for the boss, the monster type otherwise)
pub fn InitParticules(e: &mut Engine) -> LResult<u64> {
    APIgameobj::GetID(e, SELF, NULL);
    let mut uVar3 = e.r32(LM(e) + 0x44) as i32 as i64 as u64;
    match uVar3 {
        2 => {
            glow_group(e, 0.0, 0.0, 0.3);
            glow_type(e, 0x1b4, "TRAINEE_MID_MEDGHOST", SetParticle5)?;
            glow_type(e, 0x1b8, "TRAINEE_SIDE_MEDGHOST", SetParticle6)?;
            for i in 0..0x2f {
                glow_sprite(e, i, 0.45);
            }
            glow_emitter(e, 0x2f, 0x1b4)?;
            glow_emitter(e, 0x30, 0x1b8)?;
            uVar3 = glow_emitter(e, 0x31, 0x1b4)? as i32 as i64 as u64;
        }
        1 => {
            glow_group(e, 0.0, 0.15, 0.0);
            glow_type(e, 0x1b4, "TRAINEE_SMLGHOST", SetParticle5Sml)?;
            for i in 0..0x21 {
                glow_sprite(e, i, 0.45);
            }
            uVar3 = glow_emitter(e, 0x21, 0x1b4)? as i32 as i64 as u64;
        }
        3 => {
            glow_group(e, 0.0, 0.3, 0.0);
            glow_type(e, 0x1b4, "TRAINEE_FARSIDE_BIG_GHOST", SetParticle5Big)?;
            glow_type(e, 0x1b8, "TRAINEE_SIDE_BIG_GHOST", SetParticle6Big)?;
            glow_type(e, 0x1b0, "TRAINEE_MID_BIG_GHOST", SetParticle4Big)?;
            for i in 0..0x3c {
                glow_sprite(e, i, 0.45);
            }
            glow_emitter(e, 0x3c, 0x1b4)?;
            glow_emitter(e, 0x3d, 0x1b4)?;
            glow_emitter(e, 0x3e, 0x1b8)?;
            glow_emitter(e, 0x3f, 0x1b8)?;
            uVar3 = glow_emitter(e, 0x40, 0x1b0)? as i32 as i64 as u64;
        }
        0x13 => {
            glow_group(e, 0.35, 0.35, 0.35);
            for i in 0..0xf {
                let size = e.rf(L(e) + 0x70c + i * 4);
                glow_sprite(e, i, size);
            }
            uVar3 = 0xf;
        }
        _ => {}
    }
    Ok(uVar3)
}

/// 0x00018ec8 stop the trail emitters (on message 0x202)
pub fn StopGhostEmetter(e: &mut Engine) -> LResult<()> {
    APIgameobj::GetID(e, SELF, NULL);
    for o in trail_emitters(e) {
        let em = e.r32(L(e) + o);
        APIparticle::EmitterStop(e, em as i32);
    }
    Ok(())
}

/// 0x000190a0 restart the trail emitters that exist (on message 0x201)
pub fn StartGhostEmetter(e: &mut Engine) -> LResult<()> {
    APIgameobj::GetID(e, SELF, NULL);
    for o in trail_emitters(e) {
        let em = e.r32(L(e) + o);
        if em != 0 {
            APIparticle::EmitterStart(e, em as i32, 1);
        }
    }
    Ok(())
}

/// the emitter slots of InitParticules for this monster type
fn trail_emitters(e: &Engine) -> &'static [u32] {
    match e.r32(LM(e) + 0x44) {
        2 => &[0x28c, 0x290, 0x294],
        1 => &[0x254],
        3 => &[0x2c0, 0x2c4, 0x2c8, 0x2cc, 0x2d0],
        _ => &[],
    }
}

/// 0x00019350 (empty)
pub fn CreateLight(e: &mut Engine) -> LResult<()> {
    Ok(())
}

//! ZombieBossParticle_Func.c (10 functions): the boss's particle types (L+0x200..0x210, shared through
//! the first instance's guard L+0x200 == -1) and emitters (L+0x214 gas, L+0x218 lumps ("Grumeaux"),
//! L+0x21c..0x254 15 stomp-gas emitters, L+600 / L+0x25c the two dirt emitters of the stomp).
//! NOTE (PS2 behaviour kept): "Dirt2Stomp" (L+0x20c) is set up with the Dirt1Stomp settings, and the
//! first dirt emitter (L+600) reuses the collision-plane vectors left in the stack by the lumps
//! emitter (y 0.5, distance 0.25); the second one gets no plane.
use super::*;
use crate::engine::*;
use crate::logic::LResult;

fn particle_api(e: &Engine) -> u32 {
    e.r32(G(e, pParticle))
}

/// 0x00014620 Gaz, GazStomp, Grumeaux, Dirt1Stomp, Dirt2Stomp (texture "dust2")
pub fn ZombieBossFUNC_CreateParticlesTypes(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    if e.r32(L(e) + 0x200) as i32 != -1 {
        return Ok(());
    }
    let types: [(u32, &str, fn(&mut Engine, u32, u32) -> LResult<()>); 5] = [
        (0x200, "Gaz", ZombieBossFUNC_SetParticle_Gaz),
        (0x204, "GazStomp", ZombieBossFUNC_SetParticle_GazStomp),
        (0x210, "Grumeaux", ZombieBossFUNC_SetParticle_Grumeaux),
        (0x208, "Dirt1Stomp", ZombieBossFUNC_SetParticle_Dirt1Stomp),
        (0x20c, "Dirt2Stomp", ZombieBossFUNC_SetParticle_Dirt1Stomp),
    ];
    for (o, name, set) in types {
        let uVar2 = APIparticle::TypeCreate(e);
        e.w32(iVar1 + o, uVar2);
        let t = e.r32(iVar1 + o) as i32;
        let s = e.cstr("dust2");
        APIparticle::TypeSetTexture(e, t, s);
        let s = e.cstr(name);
        APIparticle::TypeSetName(e, t, s);
        let p = particle_api(e);
        set(e, p, t as u32)?;
    }
    Ok(())
}

/// 0x00014908
pub fn ZombieBossFUNC_DestroyTypes(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    if e.r32(L(e) + 0x200) as i32 != -1 {
        for o in [0x200, 0x204, 0x208, 0x20c, 0x210] {
            let t = e.r32(iVar1 + o) as i32;
            APIparticle::TypeDestroy(e, t);
        }
        e.w32(iVar1 + 0x200, 0xffff_ffff);
    }
    Ok(())
}

/// create an emitter of type L+`ty` into L+`slot`, update the type flags (0x80), joint 0x6fffffff
fn emitter(e: &mut Engine, iVar1: Ptr, slot: u32, ty: u32) {
    let t = e.r32(iVar1 + ty) as i32;
    let uVar2 = APIparticle::EmitterCreate(e, t);
    e.w32(iVar1 + slot, uVar2);
    let p = particle_api(e) as i32;
    let t = e.r32(iVar1 + ty);
    LIBMiscFunc::MiscFunc_EmitterUpdateFlags(e, p, t, 0x80, 1);
    let em = e.r32(iVar1 + slot) as i32;
    APIparticle::EmitterSetJoint(e, em, SELF, 0x6fff_ffff);
}

/// 0x000149f8 gas (plane y 0.75) and lumps (plane y 0.5) on joint 0x2401000, 15 stomp gas and the
/// two dirt emitters on the root
pub fn ZombieBossFUNC_CreateParticlesEmitters(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    if e.r32(L(e) + 0x214) as i32 != -1 {
        return Ok(());
    }
    let n = e.alloca(0x10);
    let p = e.alloca(0x10);
    let set_vec = |e: &mut Engine, a: Ptr, v: [f32; 3]| {
        for (i, x) in v.into_iter().enumerate() {
            e.wf(a + i as u32 * 4, x);
        }
    };
    for (slot, ty, y) in [(0x214, 0x200, 0.75), (0x218, 0x210, 0.5)] {
        emitter(e, iVar1, slot, ty);
        set_vec(e, n, [0.0, 1.0, 0.0]);
        set_vec(e, p, [0.0, y, 0.0]);
        let em = e.r32(iVar1 + slot) as i32;
        APIparticle::EmitterSetCollisionPlane(e, em, 1, n, p, 0.5);
        let em = e.r32(iVar1 + slot);
        APIgameobj::AddChild(e, SELF, 0x240_1000, em, 1);
    }
    for iStack_4c in 0..0xf {
        let slot = 0x21c + iStack_4c * 4;
        emitter(e, iVar1, slot, 0x204);
        let em = e.r32(iVar1 + slot);
        APIgameobj::AddChild(e, SELF, ALL, em, 1);
    }
    emitter(e, iVar1, 600, 0x208);
    let em = e.r32(iVar1 + 600) as i32;
    APIparticle::EmitterSetCollisionPlane(e, em, 1, n, p, 0.25);
    let em = e.r32(iVar1 + 600);
    APIgameobj::AddChild(e, SELF, ALL, em, 1);
    emitter(e, iVar1, 0x25c, 0x20c);
    let em = e.r32(iVar1 + 0x25c);
    APIgameobj::AddChild(e, SELF, ALL, em, 1);
    Ok(())
}

/// 0x00014ee8 (never called)
pub fn ZombieBossFUNC_DestroyParticlesEmitters(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    if e.r32(L(e) + 0x214) as i32 != -1 {
        for o in [0x214, 0x218, 600, 0x25c] {
            let em = e.r32(iVar1 + o) as i32;
            APIparticle::EmitterDestroy(e, em);
        }
        for i in 0..0xf {
            let em = e.r32(iVar1 + 0x21c + i * 4) as i32;
            APIparticle::EmitterDestroy(e, em);
        }
        e.w32(iVar1 + 0x214, 0xffff_ffff);
    }
    Ok(())
}

/// 0x00015020
pub fn ZombieBossFUNC_Particles_Start_Gaz(e: &mut Engine) -> LResult<()> {
    let em = e.r32(L(e) + 0x214) as i32;
    APIparticle::EmitterSetFlags(e, em, 0x24);
    APIparticle::EmitterStart(e, em, 1);
    Ok(())
}

/// 0x000150a0
pub fn ZombieBossFUNC_Particles_Start_Grumeaux(e: &mut Engine) -> LResult<()> {
    let em = e.r32(L(e) + 0x218) as i32;
    APIparticle::EmitterStart(e, em, 1);
    Ok(())
}

/// 0x00015100
pub fn ZombieBossFUNC_Particles_Stop_Gaz(e: &mut Engine) -> LResult<()> {
    let em = e.r32(L(e) + 0x214) as i32;
    APIparticle::EmitterStop(e, em);
    Ok(())
}

/// 0x00015158
pub fn ZombieBossFUNC_Particles_Stop_Grumeaux(e: &mut Engine) -> LResult<()> {
    let em = e.r32(L(e) + 0x218) as i32;
    APIparticle::EmitterStop(e, em);
    Ok(())
}

/// 0x000151b0 the 15 stomp-gas emitters at random spots within 4 m (y 0.3), one shot each, then both
/// dirt emitters
pub fn ZombieBossFUNC_Particles_Start_GazStomp(e: &mut Engine) -> LResult<()> {
    let iVar2 = L(e);
    let d = e.alloca(0x10);
    e.wf(d + 4, 0.3);
    for i in 0..0xf {
        let em = e.r32(iVar2 + 0x21c + i * 4);
        APIparticle::EmitterSetFlags(e, em as i32, 0x24);
        let x = LIBApiCFunctions::FloatRandom(e, -4.0, 4.0, 0.05);
        e.wf(d, x);
        let z = LIBApiCFunctions::FloatRandom(e, -4.0, 4.0, 0.05);
        e.wf(d + 8, z);
        APIgameobj::SetTranslation(e, em, d);
        APIparticle::EmitterStart(e, em as i32, 0);
    }
    for o in [600, 0x25c] {
        let em = e.r32(iVar2 + o) as i32;
        APIparticle::EmitterSetFlags(e, em, 0x20);
        APIparticle::EmitterStart(e, em, 0);
    }
    Ok(())
}

/// 0x000153b8 (never called)
pub fn ZombieBossFUNC_Particles_End_GazStomp(e: &mut Engine) -> LResult<()> {
    let iVar2 = L(e);
    for i in 0..0xf {
        let em = e.r32(iVar2 + 0x21c + i * 4) as i32;
        APIparticle::EmitterStop(e, em);
    }
    Ok(())
}

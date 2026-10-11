//! DustDevilGlowInit.c: the dust devil's look. A ring of "dust_blur" sprites on each of ten joints
//! (0xe..0x17, 132 sprites, group L+0x1c0), twelve bigger "dust_blur" sprites on joint 0x17 (group
//! L+0x1cc), a red "glow" sprite on joints 0x18 / 0x19 (group L+0x1c8), and the rock debris
//! (L+0xef4, bouncing off the ground plane) and dust (L+0xef8) emitters. Identical in every build.
//! Addresses are module-relative in dustdevilleveltwo.
//!
//! Local data: rings {count, first sprite, radius} at L+0x1d4 + i*0xc, ring joints at L+0x56c + i*4,
//! sprite handles at L+0x24c + j*4, sprite offsets at L+0x594 + j*0xc, the 12 big sprites at L+0x534
//! (offsets at L+0xe4c), the two glow sprites at L+0x564 / L+0x568.
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// `{count, radius bits, joint}` of the ten rings (CreateEmitterPos)
const RINGS: [(u32, u32, u32); 10] = [
    (0x18, 0x4006_6666, 0xe),
    (0x17, 0x4033_3333, 0xf),
    (0x11, 0x4006_6666, 0x10),
    (0xf, 0x3fcc_cccd, 0x11),
    (0xd, 0x3fcc_cccd, 0x12),
    (10, 0x3f80_0000, 0x13),
    (7, 0x3f19_999a, 0x14),
    (8, 0x3ecc_cccd, 0x15),
    (7, 0x3e4c_cccd, 0x16),
    (8, 0x3dcc_cccd, 0x17),
];

/// offsets of the 12 big sprites on joint 0x17 (L+0xe4c..L+0xedb)
const BIG_SPRITES: [u32; 36] = [
    0xbfb1_eb85, 0x3e4c_cccd, 0, 0xbf4a_3d71, 0, 0xbf4f_5c29, 0, 0, 0xbffa_e148, 0x3de9_78d5, 0, 0x3dbe_76c9,
    0, 0x3f1c_28f6, 0xbf9c_28f6, 0x3f83_d70a, 0x3ea3_d70a, 0xbf7a_e148, 0x3fec_cccd, 0, 0x3cf5_c28f, 0x3fb0_a3d7,
    0x3f30_a3d7, 0xbd8f_5c29, 0x3f1e_b852, 0, 0x3f21_47ae, 0, 0, 0x3fd7_0a3d, 0xbea3_d70a, 0, 0x3f73_3333,
    0xbf7d_70a4, 0x3f1e_b852, 0x3f70_a3d7,
];

/// `(first, count)` of ring `i`
fn ring(e: &Engine, iVar1: Ptr, i: u32) -> (u32, u32) {
    (e.r32(iVar1 + 0x1d8 + i * 0xc), e.r32(iVar1 + 0x1d4 + i * 0xc))
}

/// 0x00012f30 build the glow once (sprite size `param_1`)
pub fn CreateGlow(e: &mut Engine, param_1: f32) -> LResult<()> {
    if e.r32(L(e) + 0x18c) == 0 {
        CreateEmitterPos(e)?;
        InitParticules(e, param_1)?;
        e.w32(L(e) + 0x18c, 1);
    }
    Ok(())
}

/// 0x00012fb8 remove every sprite and group, stop and destroy both emitters
pub fn DestroyGlow(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    if e.r32(L(e) + 0x18c) != 0 {
        e.w32(L(e) + 0x18c, 0);
        for i in 0..10 {
            let (first, count) = ring(e, iVar1, i);
            let mut j = first;
            while j < first.wrapping_add(count) {
                let (gr, s) = (e.r32(iVar1 + 0x1c0), e.r32(iVar1 + 0x24c + j * 4));
                APIgraphics::SpriteGroupRemoveSprite(e, gr, s);
                j += 1;
            }
        }
        for i in 0..0xc {
            let (gr, s) = (e.r32(iVar1 + 0x1cc), e.r32(iVar1 + 0x534 + i * 4));
            APIgraphics::SpriteGroupRemoveSprite(e, gr, s);
        }
        for o in [0x564, 0x568] {
            let (gr, s) = (e.r32(iVar1 + 0x1c8), e.r32(iVar1 + o));
            APIgraphics::SpriteGroupRemoveSprite(e, gr, s);
        }
        for o in [0x1c0, 0x1c8, 0x1cc] {
            let gr = e.r32(iVar1 + o);
            APIgraphics::SpriteRemoveGroup(e, gr);
        }
        for o in [0xef4, 0xef8] {
            let em = e.r32(iVar1 + o) as i32;
            APIparticle::EmitterStop(e, em);
            APIparticle::EmitterDestroy(e, em);
        }
    }
    Ok(())
}

/// 0x00013298 lay out the rings (each sprite on a circle of the ring's radius, at a random height up to
/// radius/3) and the big sprites' offsets
pub fn CreateEmitterPos(e: &mut Engine) -> LResult<()> {
    let iVar1 = L(e);
    let mut first = 0u32;
    for (i, &(count, radius, joint)) in RINGS.iter().enumerate() {
        let r = iVar1 + 0x1d4 + i as u32 * 0xc;
        e.w32(r, count);
        e.w32(r + 4, first);
        e.w32(r + 8, radius);
        e.w32(iVar1 + 0x56c + i as u32 * 4, joint);
        first += count;
    }
    for i in 0..10u32 {
        let (first, count) = ring(e, iVar1, i);
        // (float) of an unsigned count
        let fVar5 = count as f32;
        let fVar6 = e.rf(iVar1 + 0x1dc + i * 0xc);
        let mut fStack_3c = 6.283_185_5f32 / fVar5;
        let mut j = first;
        while j < first.wrapping_add(count) {
            let fVar7 = APImath::cosf(e, fStack_3c);
            e.wf(iVar1 + 0x594 + j * 0xc, fVar6 * fVar7);
            let iVar4 = (fVar6 * 10.0) as i32 + 1;
            let uVar3 = APIstdlib::MyRand(e);
            // iVar4 == 0 traps (division by zero) on the PS2; it cannot be 0 for these radii
            let h = ((uVar3 as i32) % iVar4) as u32 as f32 / 30.0;
            e.wf(iVar1 + 0x598 + j * 0xc, h);
            let fVar7 = APImath::sinf(e, fStack_3c);
            e.wf(iVar1 + 0x59c + j * 0xc, fVar6 * fVar7);
            fStack_3c += 6.283_185_5f32 / fVar5;
            j += 1;
        }
    }
    for (i, &w) in BIG_SPRITES.iter().enumerate() {
        e.w32(iVar1 + 0xe4c + i as u32 * 4, w);
    }
    Ok(())
}

/// `SpriteAddGroup; texture; colour; blend mode; flags 6`
fn sprite_group(e: &mut Engine, tex: &str, c: [f32; 4], blend: u32) -> u32 {
    let gr = APIgraphics::SpriteAddGroup(e);
    let t = e.cstr(tex);
    APIgraphics::SpriteSetGroupTexture(e, gr, t);
    APIgraphics::SpriteSetGroupColor(e, gr, c[0], c[1], c[2], c[3]);
    APIgraphics::SpriteSetGroupBlendMode(e, gr, blend);
    APIgraphics::SpriteSetGroupFlags(e, gr, 6);
    gr
}

/// `SpriteGroupAddSprite(group, SELF, 0x6fffffff, size)`, child of `joint`, at `pos`
fn add_sprite(e: &mut Engine, gr: u32, size: f32, joint: u32, pos: Ptr) -> u32 {
    let s = APIgraphics::SpriteGroupAddSprite(e, gr, SELF, 0x6fff_ffff, size);
    APIgameobj::AddChild(e, SELF, joint, s, 1);
    APIgameobj::SetTranslation(e, s, pos);
    s
}

/// an emitter of particle type `ty` on the whole object, 0.2 m up, started, flags & ~0xc0
fn emitter(e: &mut Engine, ty: u32, pos: Ptr) -> u32 {
    let em = APIparticle::EmitterCreate(e, ty as i32);
    APIparticle::EmitterSetJoint(e, em as i32, SELF, 0x6fff_ffff);
    APIgameobj::AddChild(e, SELF, ALL, em, 1);
    APIgameobj::SetTranslation(e, em, pos);
    APIparticle::EmitterStart(e, em as i32, 2);
    let f = e.alloca(4);
    APIparticle::EmitterGetFlags(e, em as i32, f);
    let v = e.r32(f) & 0xffff_ff3f;
    e.w32(f, v);
    APIparticle::EmitterSetFlags(e, em as i32, v);
    em
}

/// 0x00013a88 create the sprite groups, sprites and emitters; the debris bounce off the ground plane
/// through the dust devil (restitution 0.65). Returns EmitterSetCollisionPlane's result.
pub fn InitParticules(e: &mut Engine, param_1: f32) -> LResult<u32> {
    let iStack_44 = L(e);
    let DStack_90 = e.alloca(0xc);
    let g0 = sprite_group(e, "dust_blur", [1.0, 1.0, 0.5, 1.0], 3);
    e.w32(iStack_44 + 0x1c0, g0);
    let g1 = sprite_group(e, "glow", [0.5, 0.0, 0.0, 1.0], 1);
    e.w32(iStack_44 + 0x1c8, g1);
    let g2 = sprite_group(e, "dust_blur", [1.0, 1.0, 0.5, 1.0], 3);
    e.w32(iStack_44 + 0x1cc, g2);
    for i in 0..10u32 {
        let (first, count) = ring(e, iStack_44, i);
        let mut j = first;
        while j < first.wrapping_add(count) {
            let gr = e.r32(iStack_44 + 0x1c0);
            let joint = e.r32(iStack_44 + 0x56c + i * 4);
            let s = APIgraphics::SpriteGroupAddSprite(e, gr, SELF, 0x6fff_ffff, param_1);
            e.w32(iStack_44 + 0x24c + j * 4, s);
            APIgameobj::AddChild(e, SELF, joint, s, 1);
            APIgameobj::SetTranslation(e, s, j * 0xc + 0x594 + iStack_44);
            j += 1;
        }
    }
    for i in 0..0xcu32 {
        let gr = e.r32(iStack_44 + 0x1cc);
        let s = add_sprite(e, gr, param_1 + param_1, 0x17, i * 0xc + 0xe4c + iStack_44);
        e.w32(iStack_44 + 0x534 + i * 4, s);
    }
    for (o, joint) in [(0x564, 0x18), (0x568, 0x19)] {
        let gr = e.r32(iStack_44 + 0x1c8);
        let s = add_sprite(e, gr, param_1, joint, DStack_90);
        e.w32(iStack_44 + o, s);
    }
    let DStack_80 = e.alloca(0xc);
    e.wf(DStack_80 + 4, 0.2);
    let ty = e.r32(GDD(e) + 0xc);
    let em = emitter(e, ty, DStack_80);
    e.w32(iStack_44 + 0xef4, em);
    let ty = e.r32(GDD(e) + 0x10);
    let em = emitter(e, ty, DStack_80);
    e.w32(iStack_44 + 0xef8, em);
    let DStack_70 = e.alloca(0xc);
    e.wf(DStack_70 + 4, 1.0);
    let DStack_60 = e.alloca(0xc);
    APIgameobj::GetPosition(e, SELF, ALL, DStack_60);
    let em = e.r32(iStack_44 + 0xef4) as i32;
    Ok(APIparticle::EmitterSetCollisionPlane(e, em, 1, DStack_70, DStack_60, 0.65))
}

/// 0x00014380 the "rock" debris and "dust_blur" dust particle types (global data +0xc / +0x10)
pub fn CreateType(e: &mut Engine) -> LResult<()> {
    let pp = e.r32(G(e, pParticle));
    for (o, tex, name, set) in [
        (0xc, "rock", "DustDevil_Base_Debris_Particle", DustDevil_baseFUNC_SetParticle_Debris as fn(&mut Engine, u32, u32) -> LResult<()>),
        (0x10, "dust_blur", "DustDevil_Base_Poussiere_Particle", DustDevil_baseFUNC_SetParticle_Poussiere),
    ] {
        let t = APIparticle::TypeCreate(e);
        let gd = GDD(e);
        e.w32(gd + o, t);
        let s = e.cstr(tex);
        APIparticle::TypeSetTexture(e, t as i32, s);
        let s = e.cstr(name);
        APIparticle::TypeSetName(e, t as i32, s);
        set(e, pp, t)?;
    }
    Ok(())
}

/// 0x000144d8
pub fn DestroyType(e: &mut Engine) -> LResult<()> {
    for o in [0xc, 0x10] {
        let t = e.r32(GDD(e) + o) as i32;
        APIparticle::TypeDestroy(e, t);
    }
    Ok(())
}

//! Port of fakezombie_ParticleSettings.c (shared by fakezombie / fakezombieb: identical bodies).
use crate::engine::*;

pub fn SetParticleBigDirt(e: &mut Engine, t: i32) {
    APIparticle::TypeSetInt(e, t, 8, 0x14);
    APIparticle::TypeSetInt(e, t, 1, 0x200);
    APIparticle::TypeSetInt(e, t, 4, 0x168);
    APIparticle::TypeSetInt(e, t, 0x10, 0);
    APIparticle::TypeSetInt(e, t, 0x20, 0);
    APIparticle::TypeSetInt(e, t, 0x40, 0);
    APIparticle::TypeSetInt(e, t, 0x80, 0);
    APIparticle::TypeSetInt(e, t, 0x100, 0);
    APIparticle::TypeSetFloat(e, t, 1, 0.0);
    APIparticle::TypeSetFloat(e, t, 4, 0.8);
    APIparticle::TypeSetFloat(e, t, 2, 2.0);
    APIparticle::TypeSetFloat(e, t, 8, 2.0);
    APIparticle::TypeSetVec(e, t, 1, 0.0, -1.5, 0.0);
    APIparticle::TypeSetVec(e, t, 2, 0.0, 0.0, 0.0);
    APIparticle::TypeSetInter(e, t, 1, 1.0, 1.0, 1.0, 1.0);
    APIparticle::TypeSetInter(e, t, 2, 1.0, 1.0, 1.0, 1.0);
    APIparticle::TypeSetInter(e, t, 4, 1.0, 1.0, 1.0, 1.0);
    APIparticle::TypeSetInter(e, t, 8, 10.0, 10.0, 10.0, 1.0);
    APIparticle::TypeSetInter(e, t, 0x10, 0.125, 0.125, 0.125, 1.0);
    APIparticle::TypeSetInter(e, t, 0x20, 0.65, 0.65, 0.65, 0.5);
}

pub fn SetParticleSmallDirt(e: &mut Engine, t: i32) {
    APIparticle::TypeSetInt(e, t, 8, 1);
    APIparticle::TypeSetInt(e, t, 1, 0x200);
    APIparticle::TypeSetInt(e, t, 4, 0x50);
    APIparticle::TypeSetInt(e, t, 0x10, 0);
    APIparticle::TypeSetInt(e, t, 0x20, 0);
    APIparticle::TypeSetInt(e, t, 0x40, 0);
    APIparticle::TypeSetInt(e, t, 0x80, 0);
    APIparticle::TypeSetInt(e, t, 0x100, 0);
    APIparticle::TypeSetFloat(e, t, 1, 0.0);
    APIparticle::TypeSetFloat(e, t, 4, 0.2);
    APIparticle::TypeSetFloat(e, t, 2, 1.5);
    APIparticle::TypeSetFloat(e, t, 8, 0.0);
    APIparticle::TypeSetVec(e, t, 1, 0.0, -1.0, 0.0);
    APIparticle::TypeSetVec(e, t, 2, 0.0, 0.0, 0.0);
    APIparticle::TypeSetInter(e, t, 1, 1.0, 1.0, 1.0, 0.753);
    APIparticle::TypeSetInter(e, t, 2, 0.8, 0.8, 0.4, 0.75);
    APIparticle::TypeSetInter(e, t, 4, 0.6, 0.6, 0.3, 0.75);
    APIparticle::TypeSetInter(e, t, 8, 0.7, 0.7, 0.01, 0.75);
    APIparticle::TypeSetInter(e, t, 0x10, 0.55, 1.75, 2.0, 0.75);
    APIparticle::TypeSetInter(e, t, 0x20, 0.65, 0.65, 0.65, 0.5);
}

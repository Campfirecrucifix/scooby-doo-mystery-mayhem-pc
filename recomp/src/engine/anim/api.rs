//! APIanim_* engine API (Engine API (module interface)/EFApi_Anim.cpp), 1:1.
//! Each C++ function starts with `EFWorld::GetGameObject(_gpoWorldMgr, obj, NULL)` and works on the
//! game object's EFAnimationMgr (EFGameObject+0x60). Replaced: the lookup is the coordinator's job
//! (Host impl); these functions take `mgr: Option<&mut EFAnimationMgr>` (None = GetGameObject
//! failed) plus the other Host args unchanged. The `pcRam00343d5c` debug-name writes are dropped.
//! quirk kept: the C++ indexes the anim table without a NULL check in GetTime/SetTime/SetSpeed/
//! GetSpeed/GetDuration/GetGroup (crash on an unknown id); here those return 0 / do nothing.
#![allow(non_snake_case)]

use super::mgr::{EFAnimationMgr, EFBlendParams};
use super::nodes::state;
use crate::engine::mem::{Mem, Ptr};

/// EFBlendParams read from module memory (5 words, see [`EFBlendParams`]); NULL -> None.
pub fn ReadBlendParams(mem: &Mem, p: Ptr) -> Option<EFBlendParams> {
    (p != 0).then(|| EFBlendParams { id: mem.r32(p), ramp: mem.rf(p + 4), weight: mem.rf(p + 8), fade_out: mem.rf(p + 0xc), sound_threshold: mem.rf(p + 0x10) })
}

/// Play (0x001eae80)
pub fn APIanim_Play(mgr: Option<&mut EFAnimationMgr>, anim: u32, flags: u32, f: f32) -> u32 {
    if let Some(m) = mgr {
        m.Play(anim, flags, f);
    }
    0
}

/// PlayBlend (0x001eaef0): EFAnimationMgr::PlayBlend(anim, flags, -1, f, params)
pub fn APIanim_PlayBlend(mgr: Option<&mut EFAnimationMgr>, mem: &Mem, anim: u32, flags: u32, f: f32, params: Ptr) -> u32 {
    if let Some(m) = mgr {
        m.PlayBlend(anim, flags, u32::MAX, f, ReadBlendParams(mem, params));
    }
    0
}

/// ChangeBlendParams (0x001eaf80)
pub fn APIanim_ChangeBlendParams(mgr: Option<&mut EFAnimationMgr>, mem: &Mem, params: Ptr) -> u32 {
    if let (Some(m), Some(p)) = (mgr, ReadBlendParams(mem, params)) {
        m.ChangeBlendParam(&p);
    }
    0
}

/// StopAnim (0x001eaa80)
pub fn APIanim_StopAnim(mgr: Option<&mut EFAnimationMgr>, anim: u32) -> u32 {
    if let Some(m) = mgr {
        m.Stop(anim);
    }
    0
}

/// StopAllAnim (0x001eab20)
pub fn APIanim_StopAllAnim(mgr: Option<&mut EFAnimationMgr>) -> u32 {
    if let Some(m) = mgr {
        m.StopAll();
    }
    0
}

/// GetTime (0x001eab70): node+0x38
pub fn APIanim_GetTime(mgr: Option<&EFAnimationMgr>, anim: u32) -> f32 {
    mgr.and_then(|m| m.AnimFields(anim)).map_or(0.0, |f| f.2)
}

/// SetTime (0x001eabe0): node vtable +0x2c SetTime
pub fn APIanim_SetTime(mgr: Option<&mut EFAnimationMgr>, anim: u32, t: f32) -> u32 {
    if let Some(m) = mgr {
        m.SetTime(anim, t);
    }
    0
}

/// GetSpeed (0x001ead00): node+0x40, returned as raw float bits (Host returns u32)
pub fn APIanim_GetSpeed(mgr: Option<&EFAnimationMgr>, anim: u32) -> u32 {
    mgr.and_then(|m| m.AnimFields(anim)).map_or(0, |f| f.4.to_bits())
}

/// SetSpeed (0x001eac70): node+0x34 &= ~0x20000000 (FINISHED); node+0x40 = speed
pub fn APIanim_SetSpeed(mgr: Option<&mut EFAnimationMgr>, anim: u32, s: f32) -> u32 {
    if let Some(m) = mgr {
        m.SetSpeed(anim, s);
    }
    0
}

/// GetDuration (0x001ead70): node+0x3c
pub fn APIanim_GetDuration(mgr: Option<&EFAnimationMgr>, anim: u32) -> f32 {
    mgr.and_then(|m| m.AnimFields(anim)).map_or(0.0, |f| f.3)
}

/// Connect (0x001eade0): if GetObjectPosData(target) != NULL, node vtable +0x28 Connect.
/// EFHAnimNode inherits the empty EFAnimNode::Connect; only texture nodes react, see
/// [`super::texture_node::EFTextureAnimNode::Connect`] which needs WP-C instance data, so the
/// coordinator calls it directly. `target_exists` = GetObjectPosData(target) != NULL.
pub fn APIanim_Connect(mgr: Option<&mut EFAnimationMgr>, anim: u32, target_exists: bool) -> u32 {
    let _ = (mgr, anim, target_exists);
    0
}

/// GetGroup (0x001eafe0): node+0x24 (exclusivity group), 0xffffffff when the object is missing
pub fn APIanim_GetGroup(mgr: Option<&EFAnimationMgr>, anim: u32) -> u32 {
    match mgr {
        None => u32::MAX,
        Some(m) => m.AnimFields(anim).map_or(0, |f| f.0),
    }
}

/// BeginTree (0x001eb050)
pub fn APIanim_BeginTree(mgr: Option<&mut EFAnimationMgr>) -> u32 {
    if let Some(m) = mgr {
        m.BeginTree();
    }
    0
}

/// PlayBlendConnect (0x001eb090): EFAnimationMgr::PlayBlend(anim, flags, blend_to, 0.0, params)
/// (note the arg swap: API order is (obj, anim, blend_to, flags, params)).
pub fn APIanim_PlayBlendConnect(mgr: Option<&mut EFAnimationMgr>, mem: &Mem, anim: u32, blend_to: u32, flags: u32, params: Ptr) -> u32 {
    if let Some(m) = mgr {
        m.PlayBlend(anim, flags, blend_to, 0.0, ReadBlendParams(mem, params));
    }
    0
}

/// EndTree (0x001eb120)
pub fn APIanim_EndTree(mgr: Option<&mut EFAnimationMgr>) -> u32 {
    if let Some(m) = mgr {
        m.EndTree();
    }
    0
}

/// IsAnimPlaying (0x001eb160): node != NULL && !(node+0x34 & 0x8000000)
pub fn APIanim_IsAnimPlaying(mgr: Option<&EFAnimationMgr>, anim: u32) -> u8 {
    mgr.and_then(|m| m.AnimFields(anim)).is_some_and(|f| f.1 & state::STOPPED == 0) as u8
}

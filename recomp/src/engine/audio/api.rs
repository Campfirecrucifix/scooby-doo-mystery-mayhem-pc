//! APInewSound_* bindings (EFApi_NewSound.cpp 0x0023df60..), 1:1 over `EFNewAudioMgr`. The game
//! resolves `obj` with EFWorld::GetGameObject (missing object -> no-op); here an object exists
//! once it has a sound list. Error codes: 0 ok, 1 unknown sound, 2 no handle (HandleSoundError).
#![allow(non_snake_case)]
use super::mgr::{EFNewAudioMgr, ALL_SOUNDS};

fn clamp01(v: f32) -> f32 { let v = if v <= 1.0 { v } else { 1.0 }; if v < 0.0 { 0.0 } else { v } }

impl EFNewAudioMgr {
    pub fn APInewSound_Play(&mut self, obj: u32, id: u32, _p3: u32) -> u32 {
        let r = self.ObjPlaySound(obj, id, None);
        if std::env::var_os("GAME_SNDLOG").is_some() {
            eprintln!("snd: APInewSound::Play obj {obj:#x} id {id:#x} -> {r}");
        }
        r
    }
    pub fn APInewSound_StopSound(&mut self, obj: u32, id: u32) -> u32 {
        if id == ALL_SOUNDS { self.ObjStopAllSound(obj); 0 } else { self.ObjStopSound(obj, id) }
    }
    pub fn APInewSound_SetListener(&mut self, obj: u32) { self.listener = obj; }
    pub fn APInewSound_SetVolumeByGroup(&mut self, group: u32, v: f32) { self.SetVolumeByGroup(group, v); }
    pub fn APInewSound_PauseSoundEvents(&mut self, mask: u32) { self.PauseSoundEventsByGroup(mask, true); }
    pub fn APInewSound_ResumeSoundEvents(&mut self, mask: u32) { self.PauseSoundEventsByGroup(mask, false); }
    pub fn APInewSound_SetGlobalVolume(&mut self, v: f32) { self.global_volume = clamp01(v); }
    pub fn APInewSound_GetGlobalVolume(&self) -> f32 { self.global_volume }
    pub fn APInewSound_SetFXVolume(&mut self, v: f32) { self.fx_volume = clamp01(v); }
    pub fn APInewSound_GetFXVolume(&self) -> f32 { self.fx_volume }
    pub fn APInewSound_SetStreamVolume(&mut self, v: f32) { self.stream_volume = clamp01(v); }
    pub fn APInewSound_GetStreamVolume(&self) -> f32 { self.stream_volume }
    pub fn APInewSound_SetSoundEventMasterVolume(&mut self, obj: u32, id: u32, v: f32) -> u32 { self.ObjSetSoundMasterVolume(obj, id, v) }
    pub fn APInewSound_GetSoundEventMasterVolume(&mut self, obj: u32, id: u32) -> f32 {
        self.object(obj).GetSound(id).map_or(0.0, |r| r.master_volume)
    }
    pub fn APInewSound_IsPlaying(&mut self, obj: u32, id: u32) -> bool { self.ObjIsSoundPlaying(obj, id).1 }
    pub fn APInewSound_SetPitch(&mut self, obj: u32, id: u32, p: f32) -> u32 { self.ObjSetSoundPitch(obj, id, p) }
    pub fn APInewSound_ParentSoundEventToJoint(&mut self, obj: u32, id: u32, joint: u32) -> u32 { self.ObjParentSoundToJoint(obj, id, joint) }
    /// DSP effects are stored only (no reverb emulation).
    pub fn APInewSound_SetDSPEffectForGroup(&mut self, group: u32, fx: u32) { self.dsp_group.insert(group, fx); }
    pub fn APInewSound_SetDSPEffectMasterVolume(&mut self, _fx: u32, _v: f32) {}
    pub fn APInewSound_SetGCDSPEffect(&mut self, _a: u32, _b: u32) {}
    pub fn APInewSound_SetAudioEngineMode(&mut self, m: u32) { self.engine_mode = m; }
    pub fn APInewSound_SetVideoVolume(&mut self, v: f32) { self.video_volume = clamp01(v); }
    pub fn APInewSound_GetVideoVolume(&self) -> f32 { self.video_volume }
}

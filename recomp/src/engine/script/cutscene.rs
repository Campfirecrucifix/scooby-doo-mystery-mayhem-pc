//! WP-K2: EFCutScenePlayer (EXE.1 0x00271cd0..0x002727db), the engine's only cut-scene/script
//! player, plus the APIgame/APIgameobj forwarders (EFApi_Game.cpp PlayCutScene 0x001ed630,
//! PlugNPlayCutScene 0x001ed650, StopCutScene 0x001ed680, SkipCutScene 0x001ed6a0,
//! SetCutSceneVolume 0x001ed7a0; EFApi_GameObj.cpp IsInCutScene 0x001f0360).
//!
//! The player is a singleton DTStreamable at 0x1f83e70 (EF_CUTSCENE, class 0x52) loaded from the
//! level pack. Its data is a table of cut-scenes; each cut-scene is a duration plus a list of
//! (game object uid, animation id) pairs. Play starts every pair's animation on its object
//! (EFAnimationMgr::Play(anim, 1, 0)), notifies all logic objects with notification type 0x16
//! payload {cutscene id, 1}; Update counts time and at the end stops the animations and
//! notifies 0x16 {id, 0}. Everything else a cut-scene does (camera cuts, messages, book
//! handover...) is done by the logic modules reacting to notification 0x16 and to the
//! animations' own events (EFAnimNode callbacks / sounds), i.e. there is no message script.
//!
//! Binary layout (LoadPropertiesBinary 0x00272620 + LoadFile 0x00272690):
//! `u32 ?, u32 ?` (two DTBasePersistStream reads, ignored), then the block that LoadFile copies
//! verbatim into one MMMalloc'd buffer (+0x1c):
//! `u16 +0, u16 +2, u16 size(+4), u16 count(+6), u32 offset[count]` (offsets relative to the
//! buffer, relocated to pointers), and per cut-scene at its offset:
//! `u32 n, f32 duration, {u32 object uid, u32 anim id}[n]`.
#![allow(non_snake_case)]

use crate::engine::error::EngineError;

/// One cut-scene (the block a +0x1c table slot points to).
#[derive(Debug, Clone, PartialEq)]
pub struct CutScene {
    /// +4: duration in seconds (copied to player +0x14 by Play)
    pub duration: f32,
    /// +8: (game object uid, animation id) pairs
    pub actors: Vec<(u32, u32)>,
}

/// The world the player talks to (EFWorld, EFAnimationMgr, EFNotificationMgr, EFNewAudioMgr_BC,
/// _HackTimer, EFEngineController). Replaces the original's globals.
pub trait CutSceneHost {
    /// EFWorld::GetGameObject(uid, NULL) != NULL
    fn GameObjectExists(&mut self, uid: u32) -> bool;
    /// EFWorld::GetObjectPosData(uid) (0 = none)
    fn GetObjectPosData(&mut self, uid: u32) -> u32;
    /// EFAnimationMgr::Play(obj+0x60, anim, flags, time)
    fn AnimPlay(&mut self, obj: u32, anim: u32, flags: u32, time: f32);
    /// EFAnimationMgr::Stop(obj+0x60, anim)
    fn AnimStop(&mut self, obj: u32, anim: u32);
    /// node = obj+0x64[anim - obj+0x60 base]; if node && node type (+0x14 byte) == 7: vfunc +0x28(pos)
    fn AnimNodeSetPosData(&mut self, obj: u32, anim: u32, pos_data: u32);
    /// node vfunc +0x2c(time) (skip to the end); nothing if the node does not exist
    fn AnimNodeSetTime(&mut self, obj: u32, anim: u32, time: f32);
    /// EFNotificationMgr::NotifyAll(_gpoGameLogicMgr+0x14, ty, &{a, b}) (synchronous)
    fn NotifyAll(&mut self, ty: u32, a: u32, b: u32);
    /// *(float*)(_gpoNewAudioMgr + 0x11350)
    fn GetAudioVolume(&mut self) -> f32;
    fn SetAudioVolume(&mut self, v: f32);
    /// EFNewAudioMgr_BC::PlayOrphanSoundEvent
    fn PlayOrphanSoundEvent(&mut self, id: u32) -> u32;
    /// EFNewAudioMgr_BC::StopSound
    fn StopSound(&mut self, h: u32);
    /// (**(_HackTimer + 0xc))() (reset/sample the real-time hack timer); returns the elapsed real
    /// time since Play reset it (fRam01f83ec0 after the call).
    fn HackTimer(&mut self) -> f32;
    /// _gpoEngineController+0x29c &= ~0x20
    fn ClearControllerFlag20(&mut self) {}
}

/// EFCutScenePlayer (DTStreamable, size 0x30).
#[derive(Debug, Clone)]
pub struct EFCutScenePlayer {
    /// +0xc: 1 playing, 2 paused, 4 skipped (one frame), 8 reached the end (one frame grace)
    pub flags: u32,
    /// +0x10: elapsed time
    pub time: f32,
    /// +0x14: duration of the current cut-scene
    pub duration: f32,
    /// +0x18: current cut-scene index (0xffffffff none)
    pub current: u32,
    /// +0x1c buffer header words (u16 +0, +2, size +4)
    pub header: [u16; 3],
    /// +0x1c buffer: the cut-scene table (None = not loaded, NULL)
    pub scenes: Option<Vec<CutScene>>,
    /// +0x20: the cut-scene's orphan sound handle (0xffffffff none)
    pub sound: u32,
    /// +0x24: the volume to restore (swapped with the audio volume by Play/Stop)
    pub volume: f32,
    /// +0x28: sound event id passed to Play
    pub sound_id: u32,
    /// +0x2c: pos-data object passed to PlugNPlay (0xffffffff none)
    pub pos_obj: u32,
    /// _fHackTimer
    pub hack_timer: f32,
}

impl Default for EFCutScenePlayer {
    fn default() -> Self {
        Self::EFCutScenePlayer()
    }
}

fn rd16(d: &[u8], o: usize) -> Result<u16, EngineError> {
    d.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]])).ok_or_else(|| EngineError::Format("EF_CUTSCENE: truncated".into()))
}
fn rd32(d: &[u8], o: usize) -> Result<u32, EngineError> {
    d.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).ok_or_else(|| EngineError::Format("EF_CUTSCENE: truncated".into()))
}

impl EFCutScenePlayer {
    /// 0x00271ce0
    pub fn EFCutScenePlayer() -> Self {
        Self {
            flags: 0,
            time: 0.0,
            duration: 0.0,
            current: 0xffff_ffff,
            header: [0; 3],
            scenes: None,
            sound: 0xffff_ffff,
            volume: 1.0,
            sound_id: 0xffff_ffff,
            pos_obj: 0xffff_ffff,
            hack_timer: 0.0,
        }
    }

    /// 0x002727c0 / 0x002727d0
    pub fn GetClassID(&self) -> u32 {
        0x52
    }
    pub fn GetClassName(&self) -> &'static str {
        "EF_CUTSCENE"
    }

    /// 0x00272620: two u32 reads (ignored), then LoadFile. `data` = the pack object's payload.
    pub fn LoadPropertiesBinary(&mut self, data: &[u8]) -> Result<u32, EngineError> {
        self.LoadFile(data.get(8..).ok_or_else(|| EngineError::Format("EF_CUTSCENE: truncated".into()))?)?;
        Ok(0)
    }

    /// 0x00272690: reads the 8-byte header, MMMalloc(size), reads size-8 more bytes, relocates
    /// the count offsets. Replacement: the buffer is parsed into [`CutScene`]s (offsets are
    /// buffer-relative, the buffer starts at the header).
    pub fn LoadFile(&mut self, d: &[u8]) -> Result<(), EngineError> {
        let h0 = rd16(d, 0)?;
        let h1 = rd16(d, 2)?;
        let size = rd16(d, 4)? as usize;
        let count = rd16(d, 6)? as usize;
        let buf = d.get(..size).ok_or_else(|| EngineError::Format("EF_CUTSCENE: short buffer".into()))?;
        let mut scenes = Vec::with_capacity(count);
        for i in 0..count {
            let o = rd32(buf, 8 + i * 4)? as usize;
            let n = rd32(buf, o)? as usize;
            let duration = f32::from_bits(rd32(buf, o + 4)?);
            let mut actors = Vec::with_capacity(n);
            for k in 0..n {
                actors.push((rd32(buf, o + 8 + k * 8)?, rd32(buf, o + 0xc + k * 8)?));
            }
            scenes.push(CutScene { duration, actors });
        }
        self.header = [h0, h1, size as u16];
        self.scenes = Some(scenes);
        Ok(())
    }

    pub fn scene(&self, i: u32) -> Option<&CutScene> {
        self.scenes.as_ref().and_then(|s| s.get(i as usize))
    }

    // quirk (Play/Stop/Skip/Update): the audio volume and +0x24 are SWAPPED (clamped 0..1 on the
    // way in), so Play installs the cut-scene volume and Stop restores the game's.
    fn swap_volume(&mut self, h: &mut dyn CutSceneHost) {
        let old = h.GetAudioVolume();
        let v = if self.volume <= 1.0 { self.volume } else { 1.0 };
        let v = if v < 0.0 { 0.0 } else { v };
        h.SetAudioVolume(v);
        self.volume = old;
    }

    /// 0x00271da0
    pub fn Play(&mut self, h: &mut dyn CutSceneHost, id: u32, sound_id: u32, pos_obj: u32) {
        h.HackTimer();
        self.hack_timer = 0.0;
        let mut pos = 0;
        if pos_obj != 0x7fff_ffff {
            self.pos_obj = pos_obj;
            pos = h.GetObjectPosData(pos_obj);
        }
        self.time = 0.0;
        self.flags |= 1;
        self.current = id;
        // quirk: no bounds check on id (the original reads past the table)
        let Some(sc) = self.scene(id).cloned() else { return };
        self.duration = sc.duration;
        h.NotifyAll(0x16, self.current, 1);
        self.swap_volume(h);
        self.sound_id = sound_id;
        self.sound = if sound_id == 0xffff_ffff { 0xffff_ffff } else { h.PlayOrphanSoundEvent(sound_id) };
        for &(obj, anim) in &sc.actors {
            if h.GameObjectExists(obj) {
                if pos != 0 {
                    h.AnimNodeSetPosData(obj, anim, pos);
                }
                h.AnimPlay(obj, anim, 1, 0.0);
            }
        }
        self.flags &= !8;
    }

    fn stop_anims(&self, h: &mut dyn CutSceneHost) {
        if let Some(sc) = self.scene(self.current) {
            for &(obj, anim) in &sc.actors {
                if h.GameObjectExists(obj) {
                    h.AnimStop(obj, anim);
                }
            }
        }
    }

    /// 0x00271fd0
    pub fn Stop(&mut self, h: &mut dyn CutSceneHost) {
        self.time = self.duration;
        self.stop_anims(h);
        if self.sound != 0xffff_ffff {
            h.StopSound(self.sound);
        }
        self.swap_volume(h);
        self.flags &= !9;
    }

    /// 0x00272100: jump every animation to the end (node vfunc +0x2c(duration)); Update then ends
    /// the cut-scene on the following frames.
    pub fn Skip(&mut self, h: &mut dyn CutSceneHost) {
        self.time = self.duration;
        self.flags |= 4;
        if let Some(sc) = self.scene(self.current).cloned() {
            for &(obj, anim) in &sc.actors {
                if h.GameObjectExists(obj) {
                    h.AnimNodeSetTime(obj, anim, self.duration);
                }
            }
        }
        if self.sound != 0xffff_ffff {
            h.StopSound(self.sound);
        }
        self.swap_volume(h);
    }

    /// 0x00272250
    pub fn Pause(&mut self, on: u32) {
        if on == 0 {
            self.flags &= !2;
        } else {
            self.flags |= 2;
        }
    }

    /// 0x00272280: is `obj` an actor of cut-scene `id` (id < count)
    pub fn IsInCutScene(&self, obj: u32, id: u32) -> bool {
        self.scene(id).map_or(false, |s| s.actors.iter().any(|a| a.0 == obj))
    }

    /// 0x002722f0 (EFEngineController main loop, every frame)
    pub fn Update(&mut self, h: &mut dyn CutSceneHost, dt: f32) {
        if self.flags & 1 == 0 {
            h.ClearControllerFlag20();
            return;
        }
        self.hack_timer = h.HackTimer();
        let f = self.flags;
        if f & 2 != 0 {
            return;
        }
        if f & 4 != 0 {
            self.flags = f & !4;
            return;
        }
        self.time += dt;
        if self.duration <= self.time {
            // one frame of grace (flag 8) before ending
            if self.flags & 8 == 0 {
                self.flags |= 8;
            } else {
                self.stop_anims(h);
                self.flags &= !9;
                if self.sound != 0xffff_ffff {
                    h.StopSound(self.sound);
                }
                self.swap_volume(h);
                h.NotifyAll(0x16, self.current, 0);
            }
        } else if self.time + 1.5 < self.hack_timer {
            // quirk: real time ran >1.5 s ahead of the cut-scene clock (loading hitch): restart
            self.Stop(h);
            self.Play(h, self.current, self.sound_id, self.pos_obj);
        }
    }

    /// 0x00272530
    pub fn Terminate(&mut self) {
        self.scenes = None;
        self.flags = 0;
    }

    // ---- EFApi_Game / EFApi_GameObj forwarders
    /// PlayCutScene 0x001ed630
    pub fn APIPlayCutScene(&mut self, h: &mut dyn CutSceneHost, id: u32, sound: u32) {
        self.Play(h, id, sound, 0x7fff_ffff);
    }
    /// SetCutSceneVolume 0x001ed7a0 (writes +0x24, clamped)
    pub fn APISetCutSceneVolume(&mut self, v: f32) {
        self.volume = if 1.0 < v { 1.0 } else if v < 0.0 { 0.0 } else { v };
    }
}

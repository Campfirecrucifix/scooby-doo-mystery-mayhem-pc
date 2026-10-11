//! WP-K2: cut-scene/script system (EFCutScenePlayer) and its binding to the runtime.
pub mod cutscene;

use crate::engine::Engine;
use crate::engine::dt::types::EF_CUTSCENE;
use crate::engine::error::EngineError;
use crate::engine::pack::DTStreamable;
use crate::engine::runtime::EngineRuntime;
use crate::engine::runtime::J2;
pub use cutscene::{CutScene, CutSceneHost, EFCutScenePlayer};

impl EFCutScenePlayer {
    /// Level load: EFRessourcesMgr streams the pack's EF_CUTSCENE object into the singleton
    /// (0x1f83e70). Returns a default (unloaded) player when the pack has none.
    pub fn FromLevelObjects(objects: &[Box<dyn DTStreamable>]) -> Result<Self, EngineError> {
        let mut p = Self::EFCutScenePlayer();
        for o in objects {
            if let Some(r) = o.as_raw() {
                if r.class_id == EF_CUTSCENE {
                    p.LoadPropertiesBinary(&r.data)?;
                }
            }
        }
        Ok(p)
    }
}

/// [`CutSceneHost`] over the running engine + runtime managers.
pub struct RuntimeCutSceneHost<'a> {
    pub e: &'a mut Engine,
    pub rt: &'a EngineRuntime,
    /// NotifyAll calls made (ty, a, b) — debug/tests
    pub notified: Vec<(u32, u32, u32)>,
}

impl<'a> RuntimeCutSceneHost<'a> {
    pub fn new(e: &'a mut Engine, rt: &'a EngineRuntime) -> Self {
        Self { e, rt, notified: Vec::new() }
    }
}

impl CutSceneHost for RuntimeCutSceneHost<'_> {
    fn GameObjectExists(&mut self, uid: u32) -> bool {
        self.e.rt.map.get(uid).is_some()
    }
    fn GetObjectPosData(&mut self, _uid: u32) -> u32 {
        // Pos-data relocation (PlugNPlay) only reaches type-7 anim nodes, see AnimNodeSetPosData.
        0
    }
    fn AnimPlay(&mut self, obj: u32, anim: u32, flags: u32, time: f32) {
        J2(&self.rt.particles).anim_play(obj, anim, flags, time);
    }
    fn AnimStop(&mut self, obj: u32, anim: u32) {
        J2(&self.rt.particles).anim_stop(obj, anim);
    }
    fn AnimNodeSetPosData(&mut self, _obj: u32, _anim: u32, _pos: u32) {
        // unported: node type 7 vfunc +0x28 (pos-data anim node); EP1L01 never plugs a pos object
    }
    fn AnimNodeSetTime(&mut self, obj: u32, anim: u32, t: f32) {
        J2(&self.rt.particles).anim_set_time(obj, anim, t);
    }
    fn NotifyAll(&mut self, ty: u32, a: u32, b: u32) {
        // payload lives on the C++ stack: a Mem block for the call's duration
        let p = self.e.mem.malloc(8);
        self.e.mem.w32(p, a);
        self.e.mem.w32(p + 4, b);
        // the host resolves SELF through the logic stack (as EngineRuntime::StepParticles does)
        self.rt.particles.lock().unwrap().logic = Some(&self.e.rt as *const _);
        self.e.NotifyAll(ty, p);
        self.e.mem.free(p);
        self.notified.push((ty, a, b));
    }
    fn GetAudioVolume(&mut self) -> f32 {
        // assumption: _gpoNewAudioMgr+0x11350 = the global volume
        J2(&self.rt.particles).sound(|a| a.global_volume)
    }
    fn SetAudioVolume(&mut self, v: f32) {
        J2(&self.rt.particles).sound(|a| a.global_volume = v);
    }
    fn PlayOrphanSoundEvent(&mut self, id: u32) -> u32 {
        J2(&self.rt.particles).sound(|a| a.PlayOrphanSoundEvent(id))
    }
    fn StopSound(&mut self, h: u32) {
        J2(&self.rt.particles).sound(|a| a.StopSoundHandle(h));
    }
    fn HackTimer(&mut self) -> f32 {
        // replacement: the real-time watchdog is a host clock; the deterministic port has none, so
        // the "real time ran 1.5 s ahead -> restart" branch never fires.
        0.0
    }
}

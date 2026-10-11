//! EFTexFader / EFTexFader_BC / EFTexFader_SP (Classes/EF/EFTexFader*.cpp): cross-fades a material
//! texture of an EF3dObj between two textures (two draws with plugin alphas), 1:1.
//!
//! Replacements: the intrusive entry list (global head uRam00343e10, MMMalloc'd 0x74-byte
//! _TFEntry) is a Vec kept in list order (AddEntry pushes at the head = index 0); EF3dObj*/
//! RpAtomic*/RpMaterial*/RwTexture* pointers become owner uid / atomic index / (res uid,
//! geometry, material) keys / texture names; the 0x69690000 material plugin (flags, alpha) is
//! a side table; the pipeline global uRam003440e4 becomes `DrawCmd::fade_pass`;
//! EFNotificationMgr::Notify is queued in `EFGraphicsMgr::notifications`.
#![allow(non_snake_case, non_camel_case_types)]

use super::mgr::EFGraphicsMgr;
use super::render::{EF3dObj_SP, OwnerInfo};
use crate::engine::rw::clump::RpClump;
use crate::engine::rw::geometry::RpTextureRef;
use crate::engine::rw::image::RwTexDictionary;

/// (resource uid, geometry index, material index): an RpMaterial* of a shared clump.
pub type MaterialKey = (u32, usize, usize);

/// _sTexFaderParams
#[derive(Debug, Clone, Default)]
pub struct _sTexFaderParams {
    /// +0x0 flags: 1 = notify at the end, 2 = keep the destination texture and delete the entry
    pub flags: u32,
    /// +0x4 duration
    pub duration: f32,
    /// +0x8 source texture names per LOD (also the material lookup name)
    pub src: [Option<String>; 3],
    /// +0x14 destination texture names per LOD
    pub dst: [Option<String>; 3],
    /// +0x20/+0x24 alpha A from/to (source texture pass)
    pub a_from: f32,
    pub a_to: f32,
    /// +0x28/+0x2c alpha B from/to (destination texture pass)
    pub b_from: f32,
    pub b_to: f32,
    /// +0x30 notification argument
    pub notify: u32,
}

/// _TFEntry (0x74 bytes)
#[derive(Debug, Clone, Default)]
pub struct _TFEntry {
    /// +0x8 flags (bit 31 = paused)
    pub flags: u32,
    /// +0xc EF3dObj (owner uid)
    pub obj: u32,
    /// +0x10 LOD being drawn
    pub cur_lod: u32,
    /// +0x14 atomics per LOD (LodGetAtomic 0..2)
    pub atomics: [Option<usize>; 3],
    /// +0x20 materials per LOD
    pub materials: [Option<MaterialKey>; 3],
    /// +0x2c source textures, +0x38 destination textures
    pub src_tex: [Option<String>; 3],
    pub dst_tex: [Option<String>; 3],
    /// +0x44 the material's texture saved during FadeAtomic
    pub saved_tex: Option<String>,
    /// +0x50 alpha A, +0x54 alpha B, +0x58/+0x5c per-second deltas, +0x60/+0x64 end values
    pub alpha_a: f32,
    pub alpha_b: f32,
    pub da: f32,
    pub db: f32,
    pub a_end: f32,
    pub b_end: f32,
    /// +0x68 duration, +0x6c elapsed
    pub duration: f32,
    pub time: f32,
    /// +0x70
    pub notify: u32,
}

#[derive(Debug, Clone, Default)]
pub struct EFTexFader_SP {
    /// entry list, head first
    pub entries: Vec<_TFEntry>,
    /// material plugin 0x69690000: (flags, alpha) per material (absent = (0, 0.0))
    pub plugin: Vec<(MaterialKey, u32, f32)>,
}

pub type EFTexFader = EFTexFader_SP;

fn mat_tex<'a>(clump: &'a RpClump, k: MaterialKey) -> Option<&'a str> {
    clump.geometries.get(k.1)?.materials.get(k.2)?.texture.as_ref().map(|t| t.name.as_str())
}

impl EFTexFader_SP {
    /// EFTexFader_BC::AddEntry (zeroed entry at the head); returns its index (0).
    pub fn AddEntry(&mut self) -> usize {
        self.entries.insert(0, _TFEntry::default());
        0
    }
    /// EFTexFader_BC::DelEntry
    pub fn DelEntry(&mut self, i: usize) {
        if i < self.entries.len() {
            self.entries.remove(i);
        }
    }
    /// EFTexFader_SP::KillEntry
    pub fn KillEntry(&mut self, i: usize) {
        self.DelEntry(i);
    }
    /// EFTexFader_BC::Terminate: kill every entry from the head.
    pub fn Terminate(&mut self) {
        while !self.entries.is_empty() {
            self.KillEntry(0);
        }
    }
    /// EFTexFader_BC::FindEntry: entry of `obj` whose LOD-0 material's texture name == name
    /// (DTstricmp).
    pub fn FindEntry(&self, obj: u32, name: &str, clump: &RpClump) -> Option<usize> {
        self.entries.iter().position(|e| {
            e.obj == obj && e.materials[0].and_then(|k| mat_tex(clump, k)).is_some_and(|t| t.eq_ignore_ascii_case(name))
        })
    }
    /// EFTexFader_BC::Pause(obj, on): bit 31 of the entry flags.
    pub fn Pause(&mut self, obj: u32, on: u32) {
        for e in self.entries.iter_mut().filter(|e| e.obj == obj) {
            if on == 0 {
                e.flags &= 0x7fff_ffff;
            } else {
                e.flags |= 0x8000_0000;
            }
        }
    }
    /// EFTexFader_SP::CreateEntry (the FindEntry(obj, "") result is ignored, as in the C++).
    pub fn CreateEntry(&mut self, obj: u32, o: &EF3dObj_SP) -> usize {
        let i = self.AddEntry();
        let e = &mut self.entries[i];
        e.obj = obj;
        e.atomics = [o.LodGetAtomic(0), o.LodGetAtomic(1), o.LodGetAtomic(2)];
        i
    }
    /// EFTexFader_SP::DestroyEntry: delete every entry of obj.
    pub fn DestroyEntry(&mut self, obj: u32) {
        self.entries.retain(|e| e.obj != obj);
    }
    /// EFTexFader_SP::FindMaterial: first atomic (list order) of LOD `lod` with a material whose
    /// texture name matches (strcasecmp, FindTexture callback).
    pub fn FindMaterial(o: &EF3dObj_SP, clump: &RpClump, lod: u32, name: Option<&str>) -> Option<MaterialKey> {
        for a in &o.atomics {
            if a.ApGetAtomicLod() != lod {
                continue;
            }
            let g = clump.geometries.get(a.geometry)?;
            if let Some(mi) = g.materials.iter().position(|m| {
                m.texture.as_ref().is_some_and(|t| name.is_some_and(|n| t.name.eq_ignore_ascii_case(n)))
            }) {
                return Some((o.res_uid, a.geometry, mi));
            }
        }
        None
    }
    /// EFTexFader_SP::FadeTo. Returns 1 (also when a material is missing: quirk, the entry is
    /// left half-initialised and the fading counter is not incremented).
    pub fn FadeTo(&mut self, obj: u32, o: &mut EF3dObj_SP, clump: &RpClump, dict: &RwTexDictionary, p: &_sTexFaderParams) -> u32 {
        let i = match self.FindEntry(obj, p.src[0].as_deref().unwrap_or(""), clump) {
            Some(i) => i,
            None => self.CreateEntry(obj, o),
        };
        for lod in 0..3 {
            if self.entries[i].atomics[lod].is_some() {
                let Some(m) = Self::FindMaterial(o, clump, lod as u32, p.src[lod].as_deref()) else { return 1 };
                let e = &mut self.entries[i];
                e.materials[lod] = Some(m);
                if let Some(n) = &p.src[lod] {
                    e.src_tex[lod] = dict.Get(n).map(|t| t.name.clone());
                }
                if let Some(n) = &p.dst[lod] {
                    e.dst_tex[lod] = dict.Get(n).map(|t| t.name.clone());
                }
            }
        }
        let e = &mut self.entries[i];
        e.flags = p.flags;
        e.alpha_a = p.a_from;
        e.alpha_b = p.b_from;
        e.da = (p.a_to - p.a_from) / p.duration;
        e.db = (p.b_to - p.b_from) / p.duration;
        e.a_end = p.a_to;
        e.b_end = p.b_to;
        e.duration = p.duration;
        e.time = 0.0;
        e.notify = p.notify;
        o.base.fading += 1;
        1
    }
    /// EFTexFader_SP::Update(dt)
    pub fn Update(&mut self, dt: f32) -> u32 {
        for e in self.entries.iter_mut() {
            if e.flags & 0x8000_0000 == 0 && e.time < e.duration {
                e.alpha_a += e.da * dt;
                e.alpha_b += e.db * dt;
                let t = e.time;
                e.time = t + dt;
                if e.duration <= t + dt {
                    e.alpha_a = e.a_end;
                    e.alpha_b = e.b_end;
                    e.time = e.duration;
                }
            }
        }
        1
    }
    /// Plugin data of a material ((0, 0.0) when never set: MaterialPlugInConstructor).
    pub fn MaterialPlugin(&self, res: u32, g: usize, m: usize) -> (u32, f32) {
        self.plugin.iter().find(|p| p.0 == (res, g, m)).map(|p| (p.1, p.2)).unwrap_or((0, 0.0))
    }
    /// EFTexFader_SP::MaterialPlugInUpdate: RpMaterialSetTexture + plugin (flags, alpha); no-op
    /// when the material or the texture is NULL.
    pub fn MaterialPlugInUpdate(&mut self, clump: &mut RpClump, m: Option<MaterialKey>, tex: Option<&str>, alpha: f32, flags: u32) {
        let (Some(k), Some(t)) = (m, tex) else { return };
        if let Some(mat) = clump.geometries.get_mut(k.1).and_then(|g| g.materials.get_mut(k.2)) {
            match &mut mat.texture {
                Some(r) => r.name = t.to_string(),
                None => mat.texture = Some(RpTextureRef { name: t.to_string(), ..Default::default() }),
            }
        }
        match self.plugin.iter_mut().find(|p| p.0 == k) {
            Some(p) => {
                p.1 = flags;
                p.2 = alpha;
            }
            None => self.plugin.push((k, flags, alpha)),
        }
    }

    /// EFTexFader_SP::FadeAtomic (called by EF3dObj::Render when the object is fading):
    /// pass 1 = source textures with alpha A, pass 2 = destination textures with alpha B, then
    /// restore / finish the entries of this object.
    pub fn FadeAtomic(&mut self, o: &mut EF3dObj_SP, ai: usize, lod: u32, clump: &mut RpClump, owner: &OwnerInfo, mgr: &mut EFGraphicsMgr) {
        let obj = owner.uid;
        for i in 0..self.entries.len() {
            if self.entries[i].obj != obj {
                continue;
            }
            let e = &mut self.entries[i];
            e.cur_lod = lod;
            let l = lod as usize;
            if e.atomics.get(l).copied().flatten().is_none() || e.src_tex.get(l).cloned().flatten().is_none() {
                continue;
            }
            e.alpha_a = e.alpha_a.clamp(0.0, 1.0);
            e.alpha_b = e.alpha_b.clamp(0.0, 1.0);
            e.saved_tex = e.materials[l].and_then(|k| mat_tex(clump, k)).map(str::to_string);
            let (m, t, a, f) = (e.materials[l], e.src_tex[l].clone(), e.alpha_a, e.flags | 0xc000_0000);
            self.MaterialPlugInUpdate(clump, m, t.as_deref(), a, f);
        }
        // RpSkyRenderStateGet(2) saved; ApSetAtomicFlags(flags | 2) (quirk: wipes the LOD bits)
        let f = o.atomics[ai].ApGetAtomicFlags();
        o.atomics[ai].ApSetAtomicFlags(f | 2);
        let d = o.ExecutePipeline(clump, ai, owner, mgr, 0xc000_0000, lod);
        mgr.draws.push(d);
        for i in 0..self.entries.len() {
            if self.entries[i].obj == obj {
                let e = &self.entries[i];
                let l = (e.cur_lod as usize).min(2);
                let (m, t, a, f) = (e.materials[l], e.dst_tex[l].clone(), e.alpha_b, e.flags | 0x8000_0000);
                self.MaterialPlugInUpdate(clump, m, t.as_deref(), a, f);
            }
        }
        let d = o.ExecutePipeline(clump, ai, owner, mgr, 0x8000_0000, lod);
        mgr.draws.push(d);
        let f = o.atomics[ai].ApGetAtomicFlags();
        o.atomics[ai].ApSetAtomicFlags(f & 0xffff_fffd);
        let mut i = 0;
        while i < self.entries.len() {
            if self.entries[i].obj != obj {
                i += 1;
                continue;
            }
            let e = self.entries[i].clone();
            let l = (e.cur_lod as usize).min(2);
            if e.duration <= e.time {
                let mut deleted = false;
                if e.flags & 2 == 0 {
                    self.MaterialPlugInUpdate(clump, e.materials[l], e.saved_tex.as_deref(), 0.0, 0);
                } else {
                    o.base.fading = o.base.fading.wrapping_sub(1);
                    self.MaterialPlugInUpdate(clump, e.materials[l], e.dst_tex[l].as_deref(), e.alpha_b, 0);
                    self.DelEntry(i);
                    deleted = true;
                }
                if e.flags & 1 != 0 {
                    // EFNotificationMgr::Notify(0x10, owner uid, owner uid, notify arg, 0)
                    mgr.notifications.push((0x10, obj, obj, e.notify));
                }
                if deleted {
                    continue;
                }
            } else {
                self.MaterialPlugInUpdate(clump, e.materials[l], e.saved_tex.as_deref(), 0.0, 0);
            }
            i += 1;
        }
    }
}

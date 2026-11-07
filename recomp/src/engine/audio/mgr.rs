//! EFNewAudioMgr: sound handles, per-object EFNewSoundMgr lists, EFSoundEvent/EFSoundContainer
//! instance pools, SPU channel allocation, volumes; outputs backend-agnostic `Voice`s.
//! 1:1: handle table (EFNewAudioMgr_BC::GetSoundHandle 0x0023fbd0: 100 handles, round-robin
//! cursor +0xc88; InitializeSound -> FindFreeSoundEvent (300) / FindFreeSoundContainer (100)),
//! EFNewSoundMgr (0x00240ee0..) reference list + SoundIsCompleted release, container selection,
//! SBH playback evaluation, ADSR register (`adsr`), ComputeVolumeAndPan (`sound3d`),
//! SOUND_FindFreeSPUChannelBalanceLoad (48 channels, 24/core, no priority/stealing).
//! REPLACEMENT: SPU keying/timing -> `Update` retires voices by decoded length.
#![allow(non_snake_case)]
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use super::adsr::SOUND_GetFinalADSROfSample;
use super::bank::SoundBank;
use super::container::{self, ContainerState};
use super::events::*;
use super::paths::stream_path;
use super::sbh::*;
use super::sound3d::{self, EventInst, Geometry, MixParams};
use super::tables::{EFSETTable, EFTSCTable};

/// newlib rand() state is one global (_impure_ptr+0xa8) shared by rand() callers and MyRand.
pub type SharedRand = Arc<Mutex<Rand>>;

pub const MAX_SOUND_HANDLES: usize = 100;
pub const MAX_SOUND_EVENTS: usize = 300;
pub const MAX_SOUND_CONTAINERS: usize = 100;
pub const SPU_CHANNELS: usize = 48;
pub const NO_HANDLE: u32 = 0xffff_ffff;
/// APInewSound StopSound id meaning "all sounds of the object".
pub const ALL_SOUNDS: u32 = 0x0fff_ffff;

#[derive(Debug, Clone, PartialEq)]
pub struct Voice {
    /// +0x32 virtual flag: no free SPU channel yet; the voice keeps its state and is restarted
    /// on a channel by Update when one frees up.
    pub virt: bool,
    /// sound handle index (EFNewAudioMgr_BC handle table)
    pub handle: u32,
    pub event_id: u32,
    pub owner: u32,
    /// sound event that produced this voice (its EFSoundEventData drives 3D)
    pub source_event: u32,
    /// SBH bank index (0 global, 1 level, 2 stream)
    pub bank: u32,
    /// bank-local SBH playback index
    pub playback: u32,
    /// bank-local SBH sample index (`SoundBank::pcm[sample]`)
    pub sample: u32,
    /// STREAM\..\*.VAG path (ISO, '/') for stream-bank samples
    pub stream: Option<String>,
    /// SOUND_SAMPLING_RATE of the sample (Hz)
    pub sample_rate: u32,
    /// 0..1 mono volume ((left+right)/2 / 0x3fff * FX volume)
    pub volume: f32,
    /// -1..1 current pan (EFSoundEvent_BC +0x88)
    pub pan: f32,
    /// 0..1 left/right (SetFinalVolume: EFSoundEvent_BC +0x80/+0x84 / 0x3fff * FX or stream volume)
    pub left: f32,
    pub right: f32,
    /// playback-rate multiplier (2^(semitones/12) * SetPitch)
    pub pitch: f32,
    /// start delay in seconds (SBH playback +0x44 randomized, milliseconds / 1000)
    pub delay: f32,
    /// SOUND_GetFinalADSROfSample (ADSR1 low 16, ADSR2 high 16)
    pub adsr: u32,
    /// SPU2 channel (0..47), None for bank-less placeholder voices
    pub channel: Option<u8>,
    pub looped: bool,
    pub paused: bool,
    /// seconds since start (driven by `Update`)
    pub time: f32,
    pub inst: EventInst,
    /// P36: unique per key-on (host identity of the SPU voice): two instances of one event on the
    /// same handle/sample, or a Stop + Play in one frame, are distinct voices (SPU2 key-on restarts
    /// the channel); the host keyed voices by (handle, bank, sample) and merged/continued them.
    pub serial: u32,
}

/// EFNewSoundMgr list node (0x14): +0 next, +4 sound id, +8 handle, +0xc master volume, +0x10 parented.
#[derive(Debug, Clone, PartialEq)]
pub struct EFNewSoundRef { pub id: u32, pub handle: u32, pub master_volume: f32, pub parented: bool }

/// EFNewSoundMgr (EFGameObject +0xc0); list order = head insertion (newest first).
#[derive(Debug, Clone, Default)]
pub struct EFNewSoundMgr { pub refs: Vec<EFNewSoundRef> }
impl EFNewSoundMgr {
    pub fn AddSound(&mut self, id: u32) { self.refs.insert(0, EFNewSoundRef { id, handle: NO_HANDLE, master_volume: 1.0, parented: false }); }
    pub fn GetSound(&mut self, id: u32) -> Option<&mut EFNewSoundRef> { self.refs.iter_mut().find(|r| r.id == id) }
}

/// structSoundHandle (0x14): +0 used, +4 sound object, +8 ref, +0x10 auto-release.
#[derive(Debug, Clone, Default)]
pub struct SoundHandle {
    pub used: bool,
    pub owner: u32,
    pub event_id: u32,
    pub auto_release: bool,
    pub orphan: bool,
    pub master_volume: f32,
    pub pitch: f32,
    pub joint: Option<u32>,
    /// EFSoundEvent +0x6c / EFSoundContainer +0x40 selection state, per (sub)object id, living
    /// as long as the handle's instance.
    pub state: HashMap<u32, ContainerState>,
}

/// Host-side 3D source: returns the listener/camera/emitter (owner object or joint LTM) for a voice.
pub trait GeometryProvider { fn geometry(&self, owner: u32, joint: Option<u32>) -> Option<Geometry>; }

#[derive(Debug, Default)]
pub struct EFNewAudioMgr {
    pub tsc: Vec<EFTSCTable>,
    pub tse: Vec<EFSETTable>,
    pub banks: Vec<Option<SoundBank>>,
    pub voices: Vec<Voice>,
    /// +0x438 group volumes (indexed by lowest set bit of the group mask)
    pub group_volume: [f32; 32],
    /// +0x11348
    pub global_volume: f32,
    /// +0x1134c
    pub fx_volume: f32,
    pub stream_volume: f32,
    /// +0x1132c
    pub video_volume: f32,
    /// +0x11334 listener object (0 = camera)
    pub listener: u32,
    /// +0x430 (1 = mono)
    pub engine_mode: u32,
    /// +0x32c audio environment / episode prefix (EFAudioEnvDescriptor)
    pub env_name: String,
    /// EFNewAudioMgr_SP::SendSpecificData strings (DSP setup; stored, not applied)
    pub env_specific: Vec<String>,
    pub dsp_group: HashMap<u32, u32>,
    /// bank file names: [0] = +300 (global), [1] = +0x22c (level), [2] = +0x2c (stream)
    pub bank_names: [String; 3],
    pub rng: SharedRand,
    pub handles: Vec<SoundHandle>,
    /// +0xc88
    pub handle_cursor: usize,
    /// per game object (uid) sound lists (EFGameObject +0xc0)
    pub objects: HashMap<u32, EFNewSoundMgr>,
    pub spu: Vec<bool>,
    /// 3D inputs for the next ComputeVolumeAndPan (set by the host each frame)
    pub geometry: HashMap<u32, Geometry>,
    pub default_geometry: Geometry,
    /// P26 R10: (owner object id, sound event id) of sounds that ended by themselves this update.
    /// EFSoundEvent_BC::SoundIsCompleted (0x00243260) / EFSoundContainer (EFSoundContainer.cpp:248/273)
    /// raise EFNotificationMgr::Notify(gameLogic, 0xf, owner+8, id, {owner id, id}, 1) when the
    /// event still has its notify flag (+0xa8: set at init, cleared by StopSound/KillSound) and is
    /// not looping (descr +0x30). The runtime drains this into the logic notification queue.
    pub completed: Vec<(u32, u32)>,
    /// P26 R23 debug: UpdateWith calls (GAME_SNDSTAT=N headless dump period)
    pub updates: u32,
    /// P36: last `Voice::serial` handed out
    pub next_serial: u32,
}

impl EFNewAudioMgr {
    pub fn new() -> Self { Self::with_rng(Arc::new(Mutex::new(Rand::default()))) }
    /// The manager draws from the game's single global rand() state.
    pub fn with_rng(rng: SharedRand) -> Self {
        Self {
            group_volume: [1.0; 32], global_volume: 1.0, fx_volume: 1.0, stream_volume: 1.0, video_volume: 1.0, rng,
            handles: vec![SoundHandle::default(); MAX_SOUND_HANDLES], spu: vec![false; SPU_CHANNELS], ..Default::default()
        }
    }
    pub fn add_bank(&mut self, bank: usize, tsc: EFTSCTable, tse: EFSETTable) {
        if self.tsc.len() <= bank { self.tsc.resize_with(bank + 1, Default::default); self.tse.resize_with(bank + 1, Default::default); }
        self.tsc[bank] = tsc;
        self.tse[bank] = tse;
    }
    pub fn add_sound_bank(&mut self, bank: usize, b: SoundBank) {
        if self.banks.len() <= bank { self.banks.resize_with(bank + 1, || None); }
        self.banks[bank] = Some(b);
    }
    /// EFNewAudioMgr_BC::GetGroupVolume: volume of the lowest set bit (mask 0 -> 1.0).
    pub fn GetGroupVolume(&self, mask: u32) -> f32 {
        if mask == 0 { return 1.0; }
        self.group_volume[mask.trailing_zeros() as usize]
    }

    // ---- EFNewAudioMgr_BC handle layer --------------------------------------------------------

    fn events_in_use(&self, containers: bool) -> usize {
        self.handles.iter().filter(|h| h.used && is_container(h.event_id) == containers).count()
    }
    /// GetSoundHandle(obj, id, ref): first free slot from the cursor; InitializeSound fails when
    /// the event/container pool is exhausted or the id has no data -> NO_HANDLE.
    pub fn GetSoundHandle(&mut self, owner: u32, id: u32) -> u32 {
        for _ in 0..MAX_SOUND_HANDLES {
            let c = self.handle_cursor;
            if !self.handles[c].used {
                let full = if is_container(id) { self.events_in_use(true) >= MAX_SOUND_CONTAINERS } else { self.events_in_use(false) >= MAX_SOUND_EVENTS };
                if full || !self.has_data(id) { return NO_HANDLE; }
                self.handles[c] = SoundHandle { used: true, owner, event_id: id, master_volume: 1.0, pitch: 1.0, ..Default::default() };
                return c as u32;
            }
            self.handle_cursor = (c + 1) % MAX_SOUND_HANDLES;
        }
        NO_HANDLE
    }
    fn has_data(&self, id: u32) -> bool {
        let (b, i) = (bank_of(id) as usize, index_of(id) as usize);
        if is_container(id) { self.tsc.get(b).map_or(false, |t| t.data.get(i).is_some()) } else { self.tse.get(b).map_or(false, |t| t.data.get(i).is_some()) }
    }
    fn valid(&self, h: u32) -> bool { self.handles.get(h as usize).map_or(false, |x| x.used) }
    pub fn ReleaseSoundHandle(&mut self, h: u32) {
        if !self.valid(h) { return; }
        self.KillVoices(h);
        self.handles[h as usize] = SoundHandle::default();
    }
    pub fn SetSoundHandleToAutoRelease(&mut self, h: u32) { if self.valid(h) { self.handles[h as usize].auto_release = true; } }
    pub fn SetSoundToOrphanState(&mut self, h: u32) { if self.valid(h) { self.handles[h as usize].orphan = true; } }
    pub fn SetSoundMasterVolume(&mut self, h: u32, v: f32) { if self.valid(h) { self.handles[h as usize].master_volume = v; } }
    pub fn IsSoundPlaying(&self, h: u32) -> bool { self.valid(h) && self.voices.iter().any(|v| v.handle == h) }
    pub fn SoundCanBeKilled(&self, h: u32) -> bool { self.valid(h) }
    pub fn StopSoundHandle(&mut self, h: u32) { if self.valid(h) { self.KillVoices(h); } }
    /// EFNewAudioMgr_BC::SetSoundPitch (0x0023f9d0) -> EFSoundEvent_BC::SetPitch (0x00243b80, only the
    /// playing instance +0x5c) -> EFNewSound_SP::SetPitch (0x00245640): SOUND_GetFinalPitchOfSample(
    /// sample pitch, p) = rate * 2^(p/12), `p` in SEMITONES, replacing the random pitch; not kept for
    /// later plays. P32 R1: was stored as a raw speed ratio on the handle (Tome sucking sound 0x2e
    /// p = 0.1..12 played at 0.1x..12x, negative p -> ~0 speed: the voice never completed).
    pub fn SetSoundPitch(&mut self, h: u32, p: f32) {
        if !self.valid(h) { return; }
        let r = 2f32.powf(p / 12.0);
        self.voices.iter_mut().filter(|v| v.handle == h).for_each(|v| v.pitch = r);
    }
    pub fn ParentSoundToJoint(&mut self, h: u32, joint: u32) { if self.valid(h) { self.handles[h as usize].joint = Some(joint); } }
    fn KillVoices(&mut self, h: u32) {
        let spu = &mut self.spu;
        self.voices.retain(|v| { if v.handle == h { if let Some(c) = v.channel { spu[c as usize] = false; } false } else { true } });
    }
    /// SOUND_FindFreeSPUChannelBalanceLoad (DSP status 0): core with fewer used voices (tie ->
    /// core 1), first free key slot of that core only.
    fn alloc_channel(&mut self) -> Option<u8> {
        let used = |c: usize| self.spu[c * 24..c * 24 + 24].iter().filter(|&&x| x).count();
        let core = if used(0) < used(1) { 0 } else { 1 };
        let ch = (core * 24..core * 24 + 24).find(|&i| !self.spu[i])?;
        self.spu[ch] = true;
        Some(ch as u8)
    }
    /// PlaySound(handle): EFSoundEvent/EFSoundContainer::PlaySound on the handle's instance.
    pub fn PlaySoundHandle(&mut self, h: u32, obj_index: Option<u32>) {
        if !self.valid(h) { return; }
        let (id, owner) = (self.handles[h as usize].event_id, self.handles[h as usize].owner);
        self.play_id(h, id, id, owner, obj_index, 1.0, 0);
    }
    /// PlayOrphanSoundEvent: ownerless handle, auto-released when finished.
    pub fn PlayOrphanSoundEvent(&mut self, id: u32) -> u32 {
        if std::env::var_os("GAME_SNDLOG").is_some() {
            eprintln!("snd: play orphan {id:#x}");
        }
        let h = self.GetSoundHandle(0, id);
        if h == NO_HANDLE { return h; }
        self.PlaySoundHandle(h, None);
        self.SetSoundHandleToAutoRelease(h);
        h
    }

    /// EFSoundEvent_BC::PlaySound: pick one playback (event +0xc type) from the event's bank.
    /// +0x2c == 0 -> not playable.
    fn play_event(&mut self, h: u32, root: u32, event_id: u32, owner: u32, oi: Option<u32>, gvol: f32, depth: u32) {
        if depth > 8 { return; }
        let bank = bank_of(event_id);
        let Some(ev) = self.tse.get_mut(bank as usize).and_then(|t| t.data.get_mut(index_of(event_id) as usize)) else { return };
        let (inst, set) = (ev.word(0x2c), ev.word(0x34));
        if inst == 0 { return; }
        if depth == 0 {
            // A2: EFSoundEvent_BC::Initialize gives a handle +0x2c instances (itself + +0xa0 clones);
            // PlaySound while the instance is busy (+0x5c raw data set until completion) plays on
            // the first idle clone, or does nothing when all are busy (port stacked voices).
            let busy = self.voices.iter().filter(|v| v.handle == h && v.source_event == event_id).count();
            if busy as u32 >= inst { return; }
            // +0x2c == 1 and set mask +0x34: EFNewSoundMgr::StopAllSoundFromSet(owner) stops the
            // owner's playing sounds whose data +0x34 shares a bit (SoundBelongsToSet).
            if inst == 1 && set != 0 && owner != 0 {
                let hs: Vec<u32> = self.objects.get(&owner).map_or(vec![], |o| o.refs.iter().map(|r| r.handle).filter(|&x| x != NO_HANDLE && x != h).collect());
                for x in hs {
                    let id = self.handles[x as usize].event_id;
                    let s2 = if is_container(id) { 0 } else { self.tse.get(bank_of(id) as usize).and_then(|t| t.data.get(index_of(id) as usize)).map_or(0, |d| d.word(0x34)) };
                    if s2 & set != 0 && self.IsSoundPlaying(x) { self.StopSoundHandle(x); }
                }
            }
        }
        let Some(ev) = self.tse.get_mut(bank as usize).and_then(|t| t.data.get_mut(index_of(event_id) as usize)) else { return };
        let kind = ev.word(0x0c);
        let st = self.handles[h as usize].state.entry(event_id).or_default();
        let picked = container::select_list(kind, &mut ev.playbacks, false, st, &mut self.rng.lock().unwrap(), oi);
        for pb in picked {
            self.play_sample(h, root, event_id, (bank << 30) | (pb & 0x0fff_ffff), owner, gvol);
        }
    }
    fn play_id(&mut self, h: u32, root: u32, id: u32, owner: u32, oi: Option<u32>, gvol: f32, depth: u32) {
        if !is_container(id) { return self.play_event(h, root, id, owner, oi, gvol, depth); }
        let bank = bank_of(id);
        let Some(d) = self.tsc.get_mut(bank as usize).and_then(|t| t.data.get_mut(index_of(id) as usize)) else { return };
        let st = self.handles[h as usize].state.entry(id).or_default();
        let kids = container::select(d, st, &mut self.rng.lock().unwrap(), oi);
        // A2: EFSoundContainer::PlaySound applies no volume of its own; each child event's volume
        // comes from its own data group (+0x28) in ComputeVolumeAndPan (was x container group at
        // start only, 1.0 on every later Update).
        for k in kids { self.play_id(h, root, k, owner, oi, gvol, depth + 1) }
    }
    fn play_sample(&mut self, h: u32, root: u32, ev: u32, id: u32, owner: u32, gvol: f32) {
        let bank = bank_of(id);
        let hp = self.handles[h as usize].pitch;
        self.next_serial = self.next_serial.wrapping_add(1);
        let mut v = Voice {
            handle: h, event_id: root, owner, source_event: ev, bank, playback: index_of(id), sample: 0, stream: None, sample_rate: 0,
            volume: gvol, pan: 0.0, left: 0.0, right: 0.0, pitch: hp, delay: 0.0, adsr: 0, channel: None, looped: false, paused: false, virt: false,
            time: 0.0, inst: EventInst::default(), serial: self.next_serial,
        };
        if let Some(b) = self.banks.get(bank as usize).and_then(|b| b.as_ref()) {
            let Some(p) = b.sbh.playbacks.get(index_of(id) as usize).cloned() else {
                if sndlog() { eprintln!("snd: MISSING playback {id:#x} (bank {bank}, {} playbacks) ev {ev:#x} owner {owner:#x}", b.sbh.playbacks.len()); }
                return;
            };
            if sndlog() && !b.sbh.is_stream() && !b.pcm.get(p.sample as usize).is_some_and(|x| x.is_some()) {
                eprintln!("snd: MISSING pcm sample {} of {id:#x} (bank {bank}) ev {ev:#x} owner {owner:#x}", p.sample);
            }
            let s = b.sample(p.sample).cloned().unwrap_or_default();
            v.stream = b.sbh.is_stream().then(|| stream_path(&s.name));
            v.looped = b.pcm.get(p.sample as usize).and_then(|x| x.as_ref()).map_or(false, |x| x.looped);
            // A4: stream samples: sample record +0x84 = driver loop flag (PS2bodyTableRecord+0x84 ->
            // SOUND_PlayInterleavedStream / SOUND_PlayStream; 1 for MU*.INT music, 0 for CS*.INT / VAGs)
            if b.sbh.is_stream() && s.field_84 != 0 { v.looped = true; }
            v.sample = p.sample;
            v.sample_rate = s.sampling_rate;
            let rng_arc = self.rng.clone();
            let rng = &mut *rng_arc.lock().unwrap();
            // RNG draw order (disasm-verified): EFSoundEvent_BC::PlaySound draws volume, pan
            // (0x00242370..), then vtbl 0x18 EFNewSound_SP::ComputeDelay (0x002456c0) the delay;
            // SOUND_PlaySFX (0x002491b0) later draws pitch (jal ComputeRandomizedValue @0x002492a8,
            // playback +0x1c) then ADSR (jal SOUND_GetFinalADSROfSample @0x002492dc).
            v.inst.base_vol = SOUND_GetVolumeOfSample(&p, rng) as i32;
            v.inst.pan_sbh = SOUND_GetPanOfSample(&p, rng);
            // A2: the SBH delay is in milliseconds (EFNewSound_SP::MustSoftDelay 0x002456f0:
            // `elapsed * 1000.0 < delay`); `Voice::delay` is seconds.
            v.delay = SOUND_GetDelayOfSample(&p, rng) / 1000.0;
            v.pitch *= 2f32.powf(SOUND_ComputeRandomizedValue(&p.pitch, rng) / 12.0);
            v.adsr = SOUND_GetFinalADSROfSample(&p, rng);
            if v.stream.is_none() {
                v.channel = self.alloc_channel();
                v.virt = v.channel.is_none(); // +0x32: virtualized until a channel frees up
            }
        } else {
            // bank not loaded: descriptor-less voice so callers can still track the handle
            if sndlog() { eprintln!("snd: MISSING bank {bank} for {id:#x} ev {ev:#x} owner {owner:#x}"); }
            v.inst.base_vol = 0x3fff;
        }
        self.mix(&mut v, gvol, None);
        self.voices.push(v);
    }
    /// ComputeVolumeAndPan + SetFinalVolume (x FX volume, or stream volume for STREAM samples).
    fn mix(&self, v: &mut Voice, gvol: f32, geo: Option<&dyn GeometryProvider>) {
        let Some(d) = self.tse.get(bank_of(v.source_event) as usize).and_then(|t| t.data.get(index_of(v.source_event) as usize)) else {
            v.volume = gvol * self.global_volume * self.fx_volume * v.inst.base_vol as f32 / 16383.0;
            v.left = v.volume; v.right = v.volume;
            return;
        };
        let g = geo.and_then(|p| p.geometry(v.owner, self.handles[v.handle as usize].joint))
            .or_else(|| self.geometry.get(&v.owner).copied());
        // A2: PlaySound runs ComputeVolumeAndPan with the live frames, so a 3D voice starts at its
        // distance volume (+0x44 sentinel -1 -> target). Without geometry at Play time (the host
        // supplies it in Update) keep the sentinels for the first Update instead of smoothing up
        // from 0 (was a 10-frame fade-in of every 3D sound).
        let mode = d.word(0x08);
        if g.is_none() && (1..=3).contains(&mode) && v.inst.vol_cur == -1.0 && self.default_geometry.emitter.is_none() {
            v.left = 0.0; v.right = 0.0; v.volume = 0.0;
            return;
        }
        let g = g.unwrap_or(self.default_geometry);
        let m = MixParams { global: self.global_volume, group: self.GetGroupVolume(d.word(0x28)) * gvol, master: self.handles[v.handle as usize].master_volume, mono: self.engine_mode == 1 };
        sound3d::ComputeVolumeAndPan(&mut v.inst, d, &g, &m);
        let mv = if v.stream.is_some() { self.stream_volume } else { self.fx_volume };
        v.left = v.inst.left * mv / 16383.0;
        v.right = v.inst.right * mv / 16383.0;
        v.pan = v.inst.pan_cur;
        v.volume = (v.left + v.right) * 0.5;
    }

    /// EFNewAudioMgr_Main::Update: per-voice volume/pan update, completion -> SoundIsCompleted.
    /// `len_of(voice)` = playback length in seconds (None = unknown/looping -> never completes).
    pub fn Update(&mut self, dt: f32, len_of: &dyn Fn(&Voice) -> Option<f32>) { self.UpdateWith(dt, len_of, None) }
    /// Update reading object/joint LTMs each frame through `geo` (EFSoundEvent_BC::ComputeVolumeAndPan
    /// reads the owner's frame / joint matrix every update).
    pub fn UpdateWith(&mut self, dt: f32, len_of: &dyn Fn(&Voice) -> Option<f32>, geo: Option<&dyn GeometryProvider>) {
        self.updates += 1;
        self.debug_stat();
        for i in 0..self.voices.len() {
            if self.voices[i].virt && self.voices[i].stream.is_none() {
                if let Some(c) = self.alloc_channel() { self.voices[i].channel = Some(c); self.voices[i].virt = false; }
            }
        }
        let mut vs = std::mem::take(&mut self.voices);
        let mut done = Vec::new();
        vs.retain_mut(|v| {
            if v.paused { return true; }
            v.time += dt;
            self.mix(v, 1.0, geo);
            let end = !v.looped && len_of(v).map_or(false, |l| v.time >= v.delay + l / v.pitch.max(1e-6));
            if end { done.push((v.handle, v.channel)); }
            !end
        });
        self.voices = vs;
        for (_, c) in &done { if let Some(c) = c { self.spu[*c as usize] = false; } }
        let mut hs: Vec<u32> = done.into_iter().map(|(h, _)| h).collect();
        hs.sort_unstable();
        hs.dedup();
        for h in hs {
            if !self.IsSoundPlaying(h) {
                // ended by itself (StopSound/KillSound remove the voices without reaching here,
                // as they clear +0xa8); orphan events have no owner game object (+0x98 = 0)
                if let Some(hd) = self.handles.get(h as usize).filter(|x| x.used && !x.orphan && x.owner != 0) {
                    if std::env::var_os("GAME_SNDLOG").is_some() {
                        eprintln!("snd: completed obj {:#x} id {:#x} -> notify 0xf", hd.owner, hd.event_id);
                    }
                    self.completed.push((hd.owner, hd.event_id));
                }
                self.SoundIsCompleted(h);
            }
        }
    }
    /// P26 R23 debug: GAME_SNDSTAT_MGR=N dumps the manager every N updates (headless silence hunts).
    fn debug_stat(&self) {
        let Some(n) = std::env::var("GAME_SNDSTAT_MGR").ok().and_then(|v| v.parse::<u32>().ok()).filter(|&n| n > 0) else { return };
        let bad = |v: &Voice| !(v.left.is_finite() && v.right.is_finite() && v.pitch.is_finite());
        let nonfinite = self.voices.iter().any(bad);
        if self.updates % n != 0 && !nonfinite { return; }
        let c = |f: &dyn Fn(&Voice) -> bool| self.voices.iter().filter(|v| f(v)).count();
        eprintln!("snd: mstat u{} vol g{} fx{} st{} voices {} paused {} virt {} looped {} nonfinite {} spu {}/48 handles {}",
            self.updates, self.global_volume, self.fx_volume, self.stream_volume, self.voices.len(), c(&|v| v.paused), c(&|v| v.virt), c(&|v| v.looped), c(&bad),
            self.spu.iter().filter(|x| **x).count(), self.handles.iter().filter(|h| h.used).count());
        for v in self.voices.iter().filter(|v| v.paused || v.looped || v.virt || v.stream.is_some() || bad(v)) {
            eprintln!("snd:   h{} obj {:#x} ev {:#x} src {:#x} paused {} virt {} looped {} pitch {} l {:.3} r {:.3} pan {} stream {:?}", v.handle, v.owner, v.event_id, v.source_event, v.paused, v.virt, v.looped, v.pitch, v.left, v.right, v.inst.pan_cur, v.stream);
        }
        let gv: Vec<_> = self.group_volume.iter().enumerate().filter(|(_, x)| **x != 1.0).collect();
        if !gv.is_empty() { eprintln!("snd:   group volumes != 1: {gv:?}"); }
    }
    /// Handle finished: auto-release / orphan handles are released; owned ones go through
    /// EFNewSoundMgr::SoundIsCompleted (released unless parented to a joint).
    pub fn SoundIsCompleted(&mut self, h: u32) {
        if !self.valid(h) { return; }
        let hd = &self.handles[h as usize];
        if hd.auto_release || hd.orphan { return self.ReleaseSoundHandle(h); }
        let owner = hd.owner;
        if let Some(r) = self.objects.get_mut(&owner).and_then(|o| o.refs.iter_mut().find(|r| r.handle == h)) {
            if !r.parented { r.handle = NO_HANDLE; self.ReleaseSoundHandle(h); }
        }
    }

    // ---- EFNewSoundMgr (per object) ----------------------------------------------------------

    pub fn object(&mut self, obj: u32) -> &mut EFNewSoundMgr { self.objects.entry(obj).or_default() }
    /// EFNewSoundMgr::PlaySound: 0 ok, 1 unknown id, 2 no handle.
    pub fn ObjPlaySound(&mut self, obj: u32, id: u32, oi: Option<u32>) -> u32 {
        let Some(r) = self.object(obj).GetSound(id) else { return 1 };
        let (mut h, mv) = (r.handle, r.master_volume);
        if h == NO_HANDLE {
            h = self.GetSoundHandle(obj, id);
            self.object(obj).GetSound(id).unwrap().handle = h;
            if h == NO_HANDLE { return 2; }
            self.SetSoundMasterVolume(h, mv);
        }
        self.PlaySoundHandle(h, oi);
        0
    }
    pub fn ObjStopSound(&mut self, obj: u32, id: u32) -> u32 {
        let Some(r) = self.object(obj).GetSound(id) else { return 1 };
        let h = r.handle;
        if h != NO_HANDLE { self.StopSoundHandle(h); }
        0
    }
    pub fn ObjStopAllSound(&mut self, obj: u32) {
        let hs: Vec<u32> = self.object(obj).refs.iter().map(|r| r.handle).filter(|&h| h != NO_HANDLE).collect();
        for h in hs { if self.SoundCanBeKilled(h) { self.StopSoundHandle(h); } }
    }
    /// EFNewSoundMgr::DeleteAllSoundReferences (object destroyed): idle handles released,
    /// playing ones become orphan + auto-release.
    pub fn DeleteAllSoundReferences(&mut self, obj: u32) {
        let Some(o) = self.objects.remove(&obj) else { return };
        for r in o.refs.iter().filter(|r| r.handle != NO_HANDLE) {
            if self.IsSoundPlaying(r.handle) { self.SetSoundHandleToAutoRelease(r.handle); self.SetSoundToOrphanState(r.handle); } else { self.ReleaseSoundHandle(r.handle); }
        }
    }
    pub fn ObjSetSoundMasterVolume(&mut self, obj: u32, id: u32, v: f32) -> u32 {
        let Some(r) = self.object(obj).GetSound(id) else { return 1 };
        r.master_volume = v;
        let h = r.handle;
        if h != NO_HANDLE { self.SetSoundMasterVolume(h, v); }
        0
    }
    pub fn ObjIsSoundPlaying(&mut self, obj: u32, id: u32) -> (u32, bool) {
        let Some(r) = self.object(obj).GetSound(id) else { return (1, false) };
        let h = r.handle;
        (0, h != NO_HANDLE && self.IsSoundPlaying(h))
    }
    pub fn ObjSetSoundPitch(&mut self, obj: u32, id: u32, p: f32) -> u32 {
        let Some(r) = self.object(obj).GetSound(id) else { return 1 };
        let h = r.handle;
        if h != NO_HANDLE && self.IsSoundPlaying(h) { self.SetSoundPitch(h, p); }
        0
    }
    pub fn ObjParentSoundToJoint(&mut self, obj: u32, id: u32, joint: u32) -> u32 {
        let Some(r) = self.object(obj).GetSound(id) else { return 1 };
        let (mut h, mv) = (r.handle, r.master_volume);
        if h == NO_HANDLE {
            h = self.GetSoundHandle(obj, id);
            self.object(obj).GetSound(id).unwrap().handle = h;
            if h == NO_HANDLE { return 2; }
            self.SetSoundMasterVolume(h, mv);
        }
        self.ParentSoundToJoint(h, joint);
        self.object(obj).GetSound(id).unwrap().parented = true;
        0
    }

    // ---- convenience (pre-G3 API, (event, owner) argument order) ------------------------------

    /// Registers `event_id` on `owner` if no descriptor did (the game registers it through
    /// EFSoundEventDescriptorHandler at level load), plays it; returns handle+1 (0 on failure).
    pub fn Play(&mut self, event_id: u32, owner: u32) -> u32 { self.PlayEx(event_id, owner, None) }
    pub fn PlayEx(&mut self, event_id: u32, owner: u32, obj_index: Option<u32>) -> u32 {
        if std::env::var_os("GAME_SNDLOG").is_some() {
            eprintln!("snd: play {event_id:#x} owner {owner:#x}");
        }
        if self.object(owner).GetSound(event_id).is_none() { self.object(owner).AddSound(event_id); }
        if self.ObjPlaySound(owner, event_id, obj_index) != 0 { return 0; }
        self.object(owner).GetSound(event_id).map_or(0, |r| r.handle + 1)
    }
    pub fn StopSound(&mut self, event_id: u32, owner: u32) { self.ObjStopSound(owner, event_id); }
    pub fn IsPlaying(&mut self, event_id: u32, owner: u32) -> bool { self.ObjIsSoundPlaying(owner, event_id).1 }
    pub fn SetPitch(&mut self, event_id: u32, owner: u32, p: f32) { self.ObjSetSoundPitch(owner, event_id, p); }
    /// All voices (debug/legacy helper; the game API is masked, see `PauseSoundEventsByGroup`).
    pub fn PauseSoundEvents(&mut self, pause: bool) { self.voices.iter_mut().for_each(|v| v.paused = pause); }
    /// EFNewAudioMgr_Main::Pause/ResumeSoundEvents (0x002406a0/0x00240780) -> EFSoundEvent_BC::
    /// PauseSound/ResumeSound: only events whose data group (+0x28) & `mask` != 0 (+0xbc flag), plus
    /// SOUND_Pause/Resume(mask) on their SPU voices. A2: was every voice (camera msg 0x12 pauses
    /// group 0x100 at book open, which froze all EP1L01 audio).
    pub fn PauseSoundEventsByGroup(&mut self, mask: u32, pause: bool) {
        let tse = &self.tse;
        for v in self.voices.iter_mut() {
            let g = tse.get(bank_of(v.source_event) as usize).and_then(|t| t.data.get(index_of(v.source_event) as usize)).map_or(0, |d| d.word(0x28));
            if g & mask != 0 { v.paused = pause; }
        }
    }
    /// EFNewAudioMgr_BC::SetVolumeByGroup (0x0023f530): `group` is a bit mask, every set bit i of the
    /// low 32 sets group volume i (N1: was indexed by the mask value, so the options' music/FX/movie
    /// volumes 0xa8/0x17/0x300 never applied).
    /// P36 R5: data group (+0x28, the GetGroupVolume mask of ComputeVolumeAndPan) of a voice's
    /// source event; the original Options sliders are groups 0xa8 music, 0x17 FX, 0x300 movies.
    pub fn voice_group(&self, v: &Voice) -> Option<u32> {
        self.tse.get(bank_of(v.source_event) as usize).and_then(|t| t.data.get(index_of(v.source_event) as usize)).map(|d| d.word(0x28))
    }
    pub fn SetVolumeByGroup(&mut self, group: u32, v: f32) {
        for i in 0..32 { if group >> i & 1 != 0 { self.group_volume[i] = v; } }
    }
    pub fn SetListener(&mut self, obj: u32) { self.listener = obj; }
}

// ---- bank setup (EFNewAudioMgr_SP) --------------------------------------------------------------
impl EFNewAudioMgr {
    /// EFNewAudioMgr_SP::SendSpecificData(p1, p2, p3): banks cleared, names upper-cased,
    /// p3 -> +300 = bank 0 (global), p2 -> +0x22c = bank 1 (level), p1 -> +0x2c = bank 2 (stream);
    /// +0x32c (episode prefix) upper-cased in place. EFAudioEnvDescriptor calls it with
    /// (s1, s3, s2), so bank 0 = s2, bank 1 = s3, bank 2 = s1.
    pub fn SendSpecificData(&mut self, p1: &str, p2: &str, p3: &str) {
        self.env_specific = vec![p1.to_string(), p2.to_string(), p3.to_string()];
        self.env_name = self.env_name.to_ascii_uppercase();
        self.bank_names = [p3.to_ascii_uppercase(), p2.to_ascii_uppercase(), p1.to_ascii_uppercase()];
    }
    /// EFNewAudioMgr_SP::LoadSoundBanks (0x00246e30): for each non-empty bank name i (0, 1, 2):
    /// SOUND_BankFile(i)+SOUND_BankLoad (SBH/SBB), then BuildSETTable(i) (.TSE), then
    /// BuildTSCTable(i) (.TSC). File lookup = SOUND_BankLoad / EFSETTable::Initialize order
    /// (`paths::bank_paths`). `level` = "EP1/L01", `lang` = "EN". Missing files are skipped
    /// ("File not found" printf in the original). Returns the bank count loaded.
    pub fn LoadSoundBanks(&mut self, level: &str, lang: &str, read: &mut dyn FnMut(&str) -> Option<Vec<u8>>) -> usize {
        let ep = self.env_name.replace(char::from(0x5c), "/");
        let names = self.bank_names.clone();
        let mut find = |b: u32, f: String| super::paths::bank_paths(b, level, lang, &ep, &f).into_iter().find_map(|p| read(&p));
        let mut n = 0;
        for (i, name) in names.iter().enumerate() {
            if name.is_empty() { continue; }
            let b = i as u32;
            let sbh = find(b, format!("{name}.SBH"));
            let sbb = find(b, format!("{name}.SBB")).unwrap_or_default();
            if let Some(h) = sbh { if let Ok(sb) = SoundBank::load(name, &sbb, &h) { self.add_sound_bank(i, sb); n += 1; } }
            let mut e = EFSETTable::EFSETTable(b);
            if let Some(d) = find(b, format!("{name}.TSE")) { let _ = e.Initialize(name, &d); }
            let mut c = EFTSCTable::EFTSCTable(b);
            if let Some(d) = find(b, format!("{name}.TSC")) { let _ = c.Initialize(name, &d); }
            if std::env::var_os("GAME_SNDLOG").is_some() {
                eprintln!("snd: bank {i} {name} sbh {} tse {} tsc {}", self.banks.get(i).is_some_and(|b| b.is_some()), e.data.len(), c.data.len());
            }
            self.add_bank(i, c, e);
        }
        n
    }
}

#[cfg(test)]
mod p36_tests {
    use super::*;

    /// P36 R6: voice-limit overflow as SOUND_FindFreeSPUChannelBalanceLoad: 48 SPU2 channels balanced
    /// over the two cores, no stealing (the 49th voice gets none = virtual), a freed channel is reused.
    #[test]
    fn p36_spu_channel_overflow() {
        let mut m = EFNewAudioMgr::new();
        let chans: Vec<u8> = (0..SPU_CHANNELS).map(|_| m.alloc_channel().expect("48 channels")).collect();
        assert_eq!(chans[..4], [24, 0, 25, 1], "tie -> core 1, then the emptier core");
        assert_eq!(m.alloc_channel(), None, "no stealing: 49th voice stays virtual");
        m.spu[7] = false;
        assert_eq!(m.alloc_channel(), Some(7), "a freed channel goes to the next (virtual) voice");
        assert_eq!(m.alloc_channel(), None);
    }
}

/// P36 R13: GAME_SNDLOG debug log switch (cached)
fn sndlog() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("GAME_SNDLOG").is_some())
}

//! SBH sample-bank header: 1:1 port of the layout read by SOUND_GetSBHInfo (0x0024b340),
//! SOUND_PatchSBH (0x0024ae70), SOUND_Create_Playback_Data (0x0024af40) and SOUND_BankLoad
//! (0x0025c870), plus the SOUND_PLAY_INFO string evaluators (SOUND_ComputeRandomizedValue etc.).
//!
//! Layout (SOUND_SBH_INFO, first 0x28 bytes copied verbatim from the file):
//!   +0x00 'A2M\0'  +0x04 'PS2\0'  +0x08 version  +0x0c (unused here)
//!   +0x10 sample table offset relative to +0x28 (GetSBHInfo)
//!   +0x14 sample table absolute offset (PatchSBH)        +0x18 sample table size (/0x8c)
//!   +0x1c playback table absolute offset                 +0x20 playback table size (/0xd0)
//!   +0x24 bank kind (2 = STREAM bank: sample names are STREAM\.. VAG paths, SBB empty)
//! Sample record (0x8c): char name[0x80]; u32 sbb_offset; u32 field_84; u32 sampling_rate.
//! Playback record (0xd0): u32 field_0; u32 sample index (bank-local); then 10 char[0x14]
//!   strings: Volume, Pitch, Pan, Delay, Attack, Hold, Decay, SustainLevel, SustainRate, Release.
#![allow(non_snake_case)]
use super::tables::rd32;
use crate::engine::error::EngineError;

pub const SBH_SAMPLE_SIZE: usize = 0x8c;
pub const SBH_PLAYBACK_SIZE: usize = 0xd0;
pub const SBH_KIND_STREAM: u32 = 2;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SbhSample {
    pub name: String,
    /// +0x80: byte offset of the ADPCM data inside the SBB (SOUND_PatchSFX(base + this)).
    pub sbb_offset: u32,
    pub field_84: u32,
    /// +0x88: SOUND_SAMPLING_RATE (Hz), returned by SOUND_GetPitchOfSample.
    pub sampling_rate: u32,
}

/// One SOUND_PLAY_INFO entry (strings kept as authored; evaluated per play).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SbhPlayback {
    pub field_0: u32,
    pub sample: u32,
    pub volume: String,
    pub pitch: String,
    pub pan: String,
    pub delay: String,
    pub attack: String,
    pub hold: String,
    pub decay: String,
    pub sustain_level: String,
    pub sustain_rate: String,
    pub release: String,
}

#[derive(Debug, Clone, Default)]
pub struct Sbh {
    pub info: [u32; 10],
    pub samples: Vec<SbhSample>,
    pub playbacks: Vec<SbhPlayback>,
}

fn cstr(b: &[u8]) -> String {
    let n = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..n]).into_owned()
}

impl Sbh {
    pub fn kind(&self) -> u32 { self.info[9] }
    pub fn is_stream(&self) -> bool { self.kind() == SBH_KIND_STREAM }

    pub fn parse(b: &[u8]) -> Result<Self, EngineError> {
        let mut info = [0u32; 10];
        for (i, w) in info.iter_mut().enumerate() {
            *w = rd32(b, i * 4)?;
        }
        let short = || EngineError::Format("SBH: short table".into());
        let mut samples = Vec::new();
        for i in 0..info[6] as usize / SBH_SAMPLE_SIZE {
            // GetSBHInfo: header + 0x28 + ((+0x10) + i*0x8c & ~3)
            let o = 0x28 + ((info[4] as usize + i * SBH_SAMPLE_SIZE) & !3);
            let r = b.get(o..o + SBH_SAMPLE_SIZE).ok_or_else(short)?;
            samples.push(SbhSample {
                name: cstr(&r[..0x80]),
                sbb_offset: rd32(r, 0x80)?,
                field_84: rd32(r, 0x84)?,
                sampling_rate: rd32(r, 0x88)?,
            });
        }
        let mut playbacks = Vec::new();
        for i in 0..info[8] as usize / SBH_PLAYBACK_SIZE {
            let o = (info[7] as usize + i * SBH_PLAYBACK_SIZE) & !3;
            let r = b.get(o..o + SBH_PLAYBACK_SIZE).ok_or_else(short)?;
            // string i lives at word 2 + 5*i (puVar4 + 2, +7, +0xc, ...)
            let s = |k: usize| cstr(&r[8 + k * 0x14..8 + (k + 1) * 0x14]);
            playbacks.push(SbhPlayback {
                field_0: rd32(r, 0)?,
                sample: rd32(r, 4)?,
                volume: s(0),
                pitch: s(1),
                pan: s(2),
                delay: s(3),
                attack: s(4),
                hold: s(5),
                decay: s(6),
                sustain_level: s(7),
                sustain_rate: s(8),
                release: s(9),
            });
        }
        Ok(Sbh { info, samples, playbacks })
    }
}

/// PS2 libc rand() (newlib 64-bit LCG, RAND_MAX 0x7fffffff; the driver divides by 2^31).
#[derive(Debug, Clone)]
pub struct Rand(pub u64);
impl Default for Rand {
    fn default() -> Self { Rand(1) }
}
impl Rand {
    pub fn rand(&mut self) -> i32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((self.0 >> 32) & 0x7fff_ffff) as i32
    }
}

/// SOUND_ComputeRandomizedValue (0x00248010): "R/min/max" -> random in [min,max] at 0.01
/// granularity; a number -> itself; anything else -> 0.
pub fn SOUND_ComputeRandomizedValue(s: &str, rng: &mut Rand) -> f32 {
    let Some(c) = s.chars().next() else { return 0.0 };
    if c.to_ascii_uppercase() == 'R' {
        let mut t = s.split(['/', '\\']).filter(|x| !x.is_empty());
        if t.next().map_or(true, |x| x.len() >= 2) {
            return 0.0;
        }
        let (Some(a), Some(b)) = (t.next(), t.next()) else { return 0.0 };
        if t.next().is_some() {
            return 0.0;
        }
        let (lo, hi) = (atof(a) as f32, atof(b) as f32);
        if hi < lo {
            return 0.0;
        }
        let f8 = (lo * 100.0) as i32 as f32;
        let r = rng.rand() as f32 / 2.147_483_6e9;
        return ((r * ((hi * 100.0) as i32 as f32 - f8)) as i32 as f32 + f8) / 100.0;
    }
    if !(c.is_ascii_digit() || c == '-') {
        return 0.0;
    }
    atof(s) as f32
}

/// C atof: longest numeric prefix.
fn atof(s: &str) -> f64 {
    let s = s.trim_start();
    let mut end = 0;
    for (i, ch) in s.char_indices() {
        if ch.is_ascii_digit() || ch == '.' || ((ch == '-' || ch == '+') && i == 0) {
            end = i + 1;
        } else {
            break;
        }
    }
    s[..end].parse().unwrap_or(0.0)
}

/// SOUND_GetVolumeOfSample (0x0024bc80): value * 16383 (SPU volume units).
pub fn SOUND_GetVolumeOfSample(p: &SbhPlayback, rng: &mut Rand) -> u32 {
    (SOUND_ComputeRandomizedValue(&p.volume, rng) * 16383.0) as u32
}
/// SOUND_GetPanOfSample (0x0024bd10).
pub fn SOUND_GetPanOfSample(p: &SbhPlayback, rng: &mut Rand) -> f32 { SOUND_ComputeRandomizedValue(&p.pan, rng) }
/// SOUND_GetDelayOfSample (0x0024bec0).
pub fn SOUND_GetDelayOfSample(p: &SbhPlayback, rng: &mut Rand) -> f32 { SOUND_ComputeRandomizedValue(&p.delay, rng) }
/// SOUND_GetFinalPitchOfSample (0x0024bd50): rate * 2^(semitones/12).
pub fn SOUND_GetFinalPitchOfSample(rate: u32, semitones: f32) -> i32 {
    (rate as f32 * 2f64.powf((semitones / 12.0) as f64) as f32) as i32
}
/// SOUND_HzToSPU (0x0024ac00).
pub fn SOUND_HzToSPU(hz: u32) -> u32 { (hz as f64 * 4096.0 / 48000.0) as u32 }

//! P36 R2: offline SPU mix for silent verification (`GAME_AUDIO_WAV=path`).
//!
//! The host normally hands every voice to Bevy/rodio (the device). For headless runs this mixes the
//! same voices in software, in lock-step with the game clock (frames x 1/60 s), into a 48 kHz
//! stereo 16-bit WAV, plus per-category level statistics (music / voice / sfx / movie), so title
//! music, loops and the mix can be checked without anything reaching the speakers.

use super::spu::SpuVoice;
use std::collections::HashMap;

pub const RATE: u32 = 48000;

/// The settings.toml / host category a voice is routed to (see game.rs cat_gain).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Bus {
    Music,
    Voice,
    Sfx,
    Movie,
}

pub const BUSES: [Bus; 4] = [Bus::Music, Bus::Voice, Bus::Sfx, Bus::Movie];

struct CapVoice {
    src: SpuVoice,
    /// speed relative to the voice's own rate (rodio_safe_speed)
    rate: f64,
    speed: f64,
    pos: f64,
    cur: f32,
    nxt: f32,
    done: bool,
    bus: Bus,
    gain: f32,
    paused: bool,
}

impl CapVoice {
    fn pull(&mut self) {
        self.cur = self.nxt;
        match self.src.next() {
            Some(s) => self.nxt = s as f32 / 32768.0,
            None => {
                self.nxt = 0.0;
                self.done = true;
            }
        }
    }
    /// one output sample (linear resampling from the voice rate x speed to 48 kHz)
    fn sample(&mut self) -> f32 {
        let s = self.cur + (self.nxt - self.cur) * self.pos as f32;
        self.pos += self.rate * self.speed / RATE as f64;
        while self.pos >= 1.0 {
            self.pos -= 1.0;
            self.pull();
        }
        s
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub struct BusStat {
    pub sum2: f64,
    pub peak: f32,
    pub n: u64,
}

impl BusStat {
    pub fn rms_db(&self) -> f32 {
        if self.n == 0 || self.sum2 <= 0.0 {
            return -120.0;
        }
        (10.0 * (self.sum2 / self.n as f64).log10()) as f32
    }
}

/// Software mixer: voices keyed by (serial, side) like the Bevy sinks.
#[derive(Default)]
pub struct Capture {
    voices: HashMap<(u32, u8), CapVoice>,
    /// interleaved L/R
    pub out: Vec<i16>,
    pub clipped: u64,
    pub stats: HashMap<Bus, BusStat>,
    /// per second of output: (music, voice, sfx, movie) RMS dB, for the log
    pub per_sec: Vec<[f32; 4]>,
    sec_acc: [f64; 4],
    sec_n: u32,
    frac: f64,
}

impl Capture {
    pub fn has(&self, serial: u32) -> bool {
        self.voices.contains_key(&(serial, 0))
    }

    /// Key-on: `side` 0 = left, 1 = right (each side its own decoder, like spawn_spu_voice).
    pub fn spawn(&mut self, serial: u32, side: u8, src: SpuVoice, rate: u32, speed: f32, bus: Bus) {
        let mut v = CapVoice { src, rate: rate.max(1) as f64, speed: speed.max(1e-4) as f64, pos: 0.0, cur: 0.0, nxt: 0.0, done: false, bus, gain: 0.0, paused: false };
        v.pull();
        self.voices.insert((serial, side), v);
    }

    /// Per-frame VOLL/VOLR + speed + pause (update_voice_gains).
    pub fn set(&mut self, serial: u32, l: f32, r: f32, speed: f32, paused: bool) {
        for (side, g) in [(0u8, l), (1, r)] {
            if let Some(v) = self.voices.get_mut(&(serial, side)) {
                v.gain = g.clamp(0.0, 1.0);
                v.speed = speed.max(1e-4) as f64;
                v.paused = paused;
            }
        }
    }

    /// Key-off / despawn of every voice not in `live`.
    pub fn retain(&mut self, live: &dyn Fn(u32) -> bool) {
        self.voices.retain(|(s, _), _| live(*s));
    }

    /// Mix `secs` of output.
    pub fn advance(&mut self, secs: f64) {
        self.frac += secs * RATE as f64;
        let n = self.frac.floor() as usize;
        self.frac -= n as f64;
        for _ in 0..n {
            let mut lr = [0f32; 2];
            let mut bus = [[0f32; 2]; 4];
            for ((_, side), v) in self.voices.iter_mut() {
                if v.paused || (v.done && v.cur == 0.0 && v.nxt == 0.0) {
                    continue;
                }
                let s = v.sample() * v.gain;
                lr[*side as usize] += s;
                bus[BUSES.iter().position(|b| *b == v.bus).unwrap()][*side as usize] += s;
            }
            for (i, b) in BUSES.iter().enumerate() {
                let st = self.stats.entry(*b).or_default();
                // per-channel mean power (L^2 + R^2) / 2, peak of either side
                let p = (bus[i][0] * bus[i][0] + bus[i][1] * bus[i][1]) as f64 / 2.0;
                st.sum2 += p;
                st.peak = st.peak.max(bus[i][0].abs()).max(bus[i][1].abs());
                st.n += 1;
                self.sec_acc[i] += p;
            }
            self.sec_n += 1;
            if self.sec_n == RATE {
                let d = |x: f64| if x <= 0.0 { -120.0 } else { (10.0 * (x / RATE as f64).log10()) as f32 };
                self.per_sec.push([d(self.sec_acc[0]), d(self.sec_acc[1]), d(self.sec_acc[2]), d(self.sec_acc[3])]);
                self.sec_acc = [0.0; 4];
                self.sec_n = 0;
            }
            for s in lr {
                if s.abs() > 1.0 {
                    self.clipped += 1;
                }
                self.out.push((s.clamp(-1.0, 1.0) * 32767.0) as i16);
            }
        }
        self.voices.retain(|_, v| !(v.done && v.cur == 0.0 && v.nxt == 0.0));
    }

    /// Mix an already-decoded stereo 16-bit buffer (FMV audio) starting now.
    pub fn spawn_pcm(&mut self, serial: u32, rate: u32, stereo: Vec<i16>, gain: f32) {
        let l: Vec<i16> = stereo.iter().step_by(2).copied().collect();
        let r: Vec<i16> = stereo.iter().skip(1).step_by(2).copied().collect();
        for (side, d) in [(0u8, l), (1, r)] {
            let src = SpuVoice::new(super::spu::SpuData::Pcm(d.into(), None), rate, 1.0, 0, false, 0.0);
            self.spawn(serial, side, src, rate, 1.0, Bus::Movie);
            self.set(serial, gain, gain, 1.0, false);
        }
    }

    pub fn wav_bytes(&self) -> Vec<u8> {
        let data = (self.out.len() * 2) as u32;
        let mut b = Vec::with_capacity(44 + data as usize);
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36 + data).to_le_bytes());
        b.extend_from_slice(b"WAVEfmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&RATE.to_le_bytes());
        b.extend_from_slice(&(RATE * 4).to_le_bytes());
        b.extend_from_slice(&4u16.to_le_bytes());
        b.extend_from_slice(&16u16.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&data.to_le_bytes());
        for s in &self.out {
            b.extend_from_slice(&s.to_le_bytes());
        }
        b
    }

    pub fn report(&self) -> String {
        let mut s = format!("capture: {:.1}s, clipped samples {}\n", self.out.len() as f64 / 2.0 / RATE as f64, self.clipped);
        for b in BUSES {
            let st = self.stats.get(&b).copied().unwrap_or_default();
            s += &format!("capture: bus {b:?} rms {:.1} dBFS peak {:.3}\n", st.rms_db(), st.peak);
        }
        for (i, x) in self.per_sec.iter().enumerate() {
            s += &format!("capture: t{i:>4}s music {:>6.1} voice {:>6.1} sfx {:>6.1} movie {:>6.1}\n", x[0], x[1], x[2], x[3]);
        }
        s
    }
}

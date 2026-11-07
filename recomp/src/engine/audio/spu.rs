//! A3: SPU2 voice playback as a sample iterator (REPLACEMENT for the SPU2 voice hardware).
//! Key-on after the start delay, plays the sample once from its start; at the ADPCM end block with
//! the repeat flag (bit1) the SPU jumps to the loop start address (LSA = last block with flag
//! bit2) keeping the decoder history, so a looping voice plays its head once then loops
//! [loop_start..end]. The ADSR envelope runs on 48 kHz ticks over the voice's whole life (not per
//! clip repeat). Streams decode their ADPCM incrementally (the IOP streamed STREAM VAGs in chunks).
use std::sync::Arc;
use super::adsr::envelope;
use super::vag::FILTERS;

/// Sample data of a voice.
#[derive(Clone)]
pub enum SpuData {
    /// decoded bank sample + its loop start (DecodedPcm::loop_start)
    Pcm(Arc<[i16]>, Option<usize>),
    /// raw SPU ADPCM blocks (stream VAG body), decoded block by block while playing
    Adpcm(Arc<[u8]>),
    /// A4: one channel (0 = L, 1 = R) of an interleaved stereo music stream (.INT, whole file incl.
    /// the A2M header): see `INT_INTERLEAVE`
    Interleaved(Arc<[u8]>, u8),
}

/// A4: .INT music = 0x30-byte header ('A2M\0' 'PS2\0', +0x10 rate BE = 44100) then SPU ADPCM
/// interleaved L/R in 0x6000-byte chunks (L0 R0 L1 R1 ..; bodies are multiples of 0xC000; no
/// ADPCM flags). 0x6000 = the chunk size EE passes to the IOP (SOUND_PlayInterleavedStream ->
/// SOUND_SetStreamParent_Int(.., 0x6000), cmd 0x2b; IOP buffer 0x18000 = 2 ch x 2 x 0x6000) and
/// the only size giving correlated L/R (EP1L1: corr 0.73 at 0x6000, |corr| < 0.14 at 0x800..0xC000).
pub const INT_HEADER: usize = 0x30;
pub const INT_INTERLEAVE: usize = 0x6000;

/// True for an A2M interleaved stream file.
pub fn is_interleaved(f: &[u8]) -> bool {
    f.len() >= INT_HEADER && &f[0..8] == b"A2M\0PS2\0"
}

/// Incremental SPU ADPCM decoder with SPU2 loop semantics.
#[derive(Clone)]
pub struct AdpcmStream {
    data: Arc<[u8]>,
    pos: usize,
    hist: (i32, i32),
    buf: [i16; 28],
    bi: usize,
    lsa: Option<usize>,
    /// loop at a plain end block too (driver loop flag on data without the repeat bit)
    force_loop: bool,
    done: bool,
    /// A4: interleaved channel (logical pos -> file offset), None = plain ADPCM body
    ch: Option<usize>,
    /// logical data length (bytes of this channel)
    len: usize,
}

impl AdpcmStream {
    pub fn new(data: Arc<[u8]>, force_loop: bool) -> Self {
        let len = data.len();
        AdpcmStream { data, pos: 0, hist: (0, 0), buf: [0; 28], bi: 28, lsa: None, force_loop, done: false, ch: None, len }
    }
    /// A4: channel `ch` of an interleaved .INT file (see INT_INTERLEAVE). The IOP refills both SPU
    /// voices from the file chunk by chunk; each voice decodes its channel continuously (history
    /// kept across chunks). Looping (driver loop flag, SBH sample +0x84) restarts at the body start.
    pub fn interleaved(data: Arc<[u8]>, ch: u8, force_loop: bool) -> Self {
        let len = (data.len().saturating_sub(INT_HEADER) / (2 * INT_INTERLEAVE)) * INT_INTERLEAVE;
        AdpcmStream { ch: Some(ch.min(1) as usize), len, ..Self::new(data, force_loop) }
    }
    fn phys(&self, pos: usize) -> usize {
        match self.ch {
            Some(c) => INT_HEADER + pos / INT_INTERLEAVE * 2 * INT_INTERLEAVE + c * INT_INTERLEAVE + pos % INT_INTERLEAVE,
            None => pos,
        }
    }
    fn block(&mut self) -> bool {
        if self.pos + 16 > self.len {
            // end of data without an end flag: the driver loop restarts the stream
            if !self.force_loop || self.len < 16 {
                return false;
            }
            self.pos = self.lsa.unwrap_or(0);
        }
        let o = self.phys(self.pos);
        let Some(blk) = self.data.get(o..o + 16) else { return false };
        let shift = (blk[0] & 0x0f) as i32;
        let [f0, f1] = FILTERS[((blk[0] >> 4) as usize).min(4)];
        let flags = blk[1];
        if flags & 4 != 0 {
            self.lsa = Some(self.pos);
        }
        let (mut s1, mut s2) = self.hist;
        for i in 0..28 {
            let b = blk[2 + i / 2];
            let nib = if i & 1 == 0 { b & 0x0f } else { b >> 4 } as i32;
            let s = (((nib << 28) >> 28) << 12) >> shift.min(12);
            let v = (s + ((s1 * f0 + s2 * f1 + 32) >> 6)).clamp(-32768, 32767);
            self.buf[i] = v as i16;
            s2 = s1;
            s1 = v;
        }
        self.hist = (s1, s2);
        self.bi = 0;
        self.pos += 16;
        if flags & 1 != 0 {
            if flags & 2 != 0 || self.force_loop {
                self.pos = self.lsa.unwrap_or(0);
            } else {
                self.done = true;
            }
        }
        true
    }
}

impl Iterator for AdpcmStream {
    type Item = i16;
    fn next(&mut self) -> Option<i16> {
        if self.bi == 28 {
            if self.done || !self.block() {
                return None;
            }
        }
        self.bi += 1;
        Some(self.buf[self.bi - 1])
    }
}

/// P26 R23 (audio silence): playback speed handed to the host player (rodio `Speed`) for a voice
/// of `rate` Hz at `pitch`. REPLACEMENT detail, not in the C: rodio 0.22.2's SampleRateConverter
/// computes `from * pos / to` in u32 (from/to = reduced source/device rate ratio); when
/// from * to > 2^32 (e.g. 44100 Hz x 8.1325 = 358643 Hz vs 48 kHz, gcd 1) the product wraps and its
/// `while current_span_pos_in_chunk != req_left_sample` loop runs ~2^32 times inside the cpal
/// callback: the whole output stalls (all audio silent, game keeps running) for minutes. Snapping
/// rate*speed to a multiple of 100 Hz keeps gcd >= 100 with any 44.1/48/96/192 kHz device rate
/// (from * to stays tiny); pitch error <= 50 Hz (~4 cents at 22 kHz). Pitch 1 is passed unchanged.
pub fn rodio_safe_speed(rate: u32, pitch: f32) -> f32 {
    if !(pitch.is_finite() && pitch > 0.0) {
        return 1.0;
    }
    if pitch == 1.0 || rate == 0 {
        return pitch;
    }
    let target = (rate as f64 * pitch as f64 / 100.0).round().max(1.0) * 100.0;
    // +0.5: rodio truncates `rate as f32 * speed` to u32
    ((target + 0.5) / rate as f64) as f32
}

/// Envelope ticks precomputed per voice; past them the last level is held (sustain).
const ENV_TICKS: usize = 48000 * 8;

/// One playing SPU voice: mono i16 output at `rate` (pitch is applied by the player's speed).
#[derive(Clone)]
pub struct SpuVoice {
    src: Src,
    env: Arc<[f32]>,
    step: f64,
    delay: usize,
    n: usize,
}

#[derive(Clone)]
enum Src {
    Pcm { pcm: Arc<[i16]>, loop_start: Option<usize>, i: usize },
    Adpcm(AdpcmStream),
}

impl SpuVoice {
    /// `looped` = Voice::looped, `delay` seconds (EFNewSound_SP::MustSoftDelay; once, before key-on).
    pub fn new(data: SpuData, rate: u32, pitch: f32, adsr: u32, looped: bool, delay: f32) -> Self {
        let (src, len) = match data {
            SpuData::Pcm(pcm, ls) => {
                let n = pcm.len();
                (Src::Pcm { pcm, loop_start: if looped { Some(ls.unwrap_or(0)) } else { None }, i: 0 }, n)
            }
            SpuData::Adpcm(d) => {
                let n = d.len() / 16 * 28;
                (Src::Adpcm(AdpcmStream::new(d, looped)), n)
            }
            SpuData::Interleaved(d, ch) => {
                let a = AdpcmStream::interleaved(d, ch, looped);
                let n = a.len / 16 * 28;
                (Src::Adpcm(a), n)
            }
        };
        let step = rate as f64 * pitch as f64 / 48000.0;
        let ticks = if looped { ENV_TICKS } else { ((len as f64 / step.max(1e-9)).ceil() as usize).clamp(1, ENV_TICKS) };
        let delay = if delay > 0.0 { (delay * rate as f32 * pitch) as usize } else { 0 };
        SpuVoice { src, env: envelope(adsr, ticks, None).into(), step: step.max(1e-9), delay, n: 0 }
    }
}

impl Iterator for SpuVoice {
    type Item = i16;
    fn next(&mut self) -> Option<i16> {
        if self.delay > 0 {
            self.delay -= 1;
            return Some(0);
        }
        let s = match &mut self.src {
            Src::Pcm { pcm, loop_start, i } => {
                if *i >= pcm.len() {
                    match loop_start {
                        Some(l) if *l < pcm.len() => *i = *l,
                        _ => return None,
                    }
                }
                *i += 1;
                pcm[*i - 1]
            }
            Src::Adpcm(a) => a.next()?,
        };
        let t = ((self.n as f64 / self.step) as usize).min(self.env.len() - 1);
        self.n += 1;
        Some((s as f32 * self.env[t]) as i16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// P36 R8: audio callback budget under load. Worst case = all 48 SPU channels busy, each split
    /// into its L/R host source (96 SpuVoice ADPCM decoders, stream-style), for one device buffer
    /// (rodio default ~50 ms -> 2048 frames at 48 kHz = 42.7 ms). The decode share of the callback
    /// must stay well inside the period on this (busy) PC, or the device would underrun.
    #[test]
    fn p36_callback_budget() {
        let mut seed = 0x1234_5678u32;
        let mut rnd = || { seed ^= seed << 13; seed ^= seed >> 17; seed ^= seed << 5; seed };
        let blocks: Vec<u8> = (0..4096).flat_map(|_| {
            let mut b = [0u8; 16];
            b[0] = (rnd() % 5) as u8 * 16 + (rnd() % 12) as u8; // filter 0..4, shift 0..11
            for x in &mut b[2..] { *x = rnd() as u8; }
            b
        }).collect();
        let data: Arc<[u8]> = blocks.into();
        let mut voices: Vec<SpuVoice> = (0..96).map(|k| SpuVoice::new(SpuData::Adpcm(data.clone()), [22050, 32000, 44100, 48000][k % 4], 1.0, 0, true, 0.0)).collect();
        let period = 2048.0 / 48000.0;
        let mut worst = 0f64;
        let mut sum = 0i64;
        for _ in 0..60 {
            let t = std::time::Instant::now();
            for v in voices.iter_mut() {
                for _ in 0..(2048.0 * v.step).ceil() as usize { sum += v.next().unwrap_or(0) as i64; }
            }
            worst = worst.max(t.elapsed().as_secs_f64());
        }
        println!("p36_callback_budget: 96 decoders, worst {:.2} ms per {:.1} ms buffer ({:.0}%) [{sum}]", worst * 1e3, period * 1e3, worst / period * 100.0);
        assert!(worst < period * 0.5, "decode takes {:.1} ms of a {:.1} ms buffer", worst * 1e3, period * 1e3);
    }
}

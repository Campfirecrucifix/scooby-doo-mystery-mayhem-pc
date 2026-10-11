use std::collections::VecDeque;
use std::sync::Arc;

use crate::audio::{Ac3AudioChunk, Ac3AudioDecoder, MpaAudioChunk, MpaAudioDecoder, MpaFrameProbe};
use crate::convert::frame_to_rgba_bt601_limited;
use crate::demux::{Demuxer, Packet, StreamType};
use crate::error::Result;
use crate::video::{Decoder as VideoDecoder, Frame};

#[derive(Debug, Clone, Copy)]
pub struct MpegAudioStreamProbeInfo {
    pub sample_rate: u32,
    pub channels: u16,
    pub first_audio_pts_90k: i64,
    pub first_video_pts_90k: Option<i64>,
}

#[derive(Debug, Clone, Copy)]
pub struct MpegAudioProbeInfo {
    pub sample_rate: u32,
    pub channels: u16,
    pub num_frames: usize,
    pub first_audio_pts_ms: i64,
    pub first_video_pts_ms: Option<i64>,
    pub first_audio_pts_90k: i64,
    pub first_video_pts_90k: Option<i64>,
}

#[derive(Debug, Clone, Copy)]
pub struct MpegAudioTailProbeInfo {
    /// PTS of the first audio PES anchor included in `output_frames`.
    pub anchor_pts_90k: i64,
    pub sample_rate: u32,
    pub channels: u16,
    /// Decoded-output frame count from the anchor through end of input.
    pub output_frames: usize,
}

/// Fast program-stream audio probe for the bounded movie decoder.
///
/// This demuxes the complete container and counts compressed MPEG-audio
/// frames, but deliberately does not synthesize PCM.  Kira requires a finite
/// `num_frames()` value before it starts its streaming scheduler; this probe
/// supplies that value without the old full-movie decode and WAV allocation.
pub struct MpegAudioProbePipeline {
    demux: Demuxer,
    mpa: MpaFrameProbe,
    pkts: Vec<Packet>,
    first_audio_pts_90k: Option<i64>,
    first_video_pts_90k: Option<i64>,
    saw_dvd_private_audio: bool,
}

impl Default for MpegAudioProbePipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl MpegAudioProbePipeline {
    pub fn new() -> Self {
        Self {
            demux: Demuxer::new_auto(),
            mpa: MpaFrameProbe::new(),
            pkts: Vec::new(),
            first_audio_pts_90k: None,
            first_video_pts_90k: None,
            saw_dvd_private_audio: false,
        }
    }

    pub fn push(&mut self, data: &[u8], pts_90k: Option<i64>) {
        self.pkts.clear();
        self.demux.push_into(data, pts_90k, &mut self.pkts);
        for pkt in self.pkts.drain(..) {
            match pkt.stream_type {
                StreamType::MpegAudio => {
                    if self.first_audio_pts_90k.is_none() {
                        self.first_audio_pts_90k = pkt.pts_90k;
                    }
                    self.mpa.push(&pkt.data);
                }
                StreamType::MpegVideo => {
                    if self.first_video_pts_90k.is_none() {
                        self.first_video_pts_90k = pkt.pts_90k;
                    }
                }
                StreamType::DvdLpcmAudio => self.saw_dvd_private_audio = true,
                StreamType::Unknown => {}
            }
        }
    }

    pub fn stream_info(&self) -> Option<MpegAudioStreamProbeInfo> {
        // The existing AC-3/DVD-LPCM path is retained as a static fallback.
        // This metadata path is exact only for MPEG Layer I/II/III.
        if self.mpa.compressed_frames() == 0 || self.saw_dvd_private_audio {
            return None;
        }
        Some(MpegAudioStreamProbeInfo {
            sample_rate: self.mpa.first_sample_rate()?,
            channels: self.mpa.first_channels()?,
            first_audio_pts_90k: self.first_audio_pts_90k?,
            first_video_pts_90k: self.first_video_pts_90k,
        })
    }

    pub fn finish(&self) -> Option<MpegAudioProbeInfo> {
        let stream = self.stream_info()?;
        let first_audio_pts_ms = pts90k_to_ms(stream.first_audio_pts_90k);
        let first_video_pts_ms = stream.first_video_pts_90k.map(pts90k_to_ms);
        let origin_pts = stream
            .first_video_pts_90k
            .map(|video| earlier_pts_90k(video, stream.first_audio_pts_90k))
            .unwrap_or(stream.first_audio_pts_90k);
        let delay_ticks = pts_delta_90k(stream.first_audio_pts_90k, origin_pts).unwrap_or(0);
        let delay_frames = pts90k_ticks_to_frames(delay_ticks, stream.sample_rate);
        Some(MpegAudioProbeInfo {
            sample_rate: stream.sample_rate,
            channels: stream.channels,
            num_frames: delay_frames.saturating_add(self.mpa.output_frames()),
            first_audio_pts_ms,
            first_video_pts_ms,
            first_audio_pts_90k: stream.first_audio_pts_90k,
            first_video_pts_90k: stream.first_video_pts_90k,
        })
    }
}

/// Tail-only MPEG-audio probe used to derive an exact finite Kira length
/// without scanning the complete movie before playback.  Data before the first
/// PTS-bearing MPEG-audio PES in the supplied tail is ignored; output frames
/// are counted from that anchor through EOF.
pub struct MpegAudioTailProbePipeline {
    demux: Demuxer,
    mpa: MpaFrameProbe,
    pkts: Vec<Packet>,
    anchor_pts_90k: Option<i64>,
    saw_dvd_private_audio: bool,
}

impl Default for MpegAudioTailProbePipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl MpegAudioTailProbePipeline {
    pub fn new() -> Self {
        Self {
            demux: Demuxer::new_auto(),
            mpa: MpaFrameProbe::new(),
            pkts: Vec::new(),
            anchor_pts_90k: None,
            saw_dvd_private_audio: false,
        }
    }

    pub fn push(&mut self, data: &[u8]) {
        self.pkts.clear();
        self.demux.push_into(data, None, &mut self.pkts);
        for pkt in self.pkts.drain(..) {
            match pkt.stream_type {
                StreamType::MpegAudio => {
                    if self.anchor_pts_90k.is_none() {
                        let Some(pts) = pkt.pts_90k else {
                            continue;
                        };
                        self.anchor_pts_90k = Some(pts);
                        self.mpa = MpaFrameProbe::new();
                    }
                    self.mpa.push(&pkt.data);
                }
                StreamType::DvdLpcmAudio => self.saw_dvd_private_audio = true,
                StreamType::MpegVideo | StreamType::Unknown => {}
            }
        }
    }

    pub fn finish(&self) -> Option<MpegAudioTailProbeInfo> {
        if self.saw_dvd_private_audio || self.mpa.compressed_frames() == 0 {
            return None;
        }
        Some(MpegAudioTailProbeInfo {
            anchor_pts_90k: self.anchor_pts_90k?,
            sample_rate: self.mpa.first_sample_rate()?,
            channels: self.mpa.first_channels()?,
            output_frames: self.mpa.output_frames(),
        })
    }
}

fn pts_delta_90k(later: i64, earlier: i64) -> Option<i64> {
    const PTS_WRAP: i64 = 1i64 << 33;
    let mut delta = later.saturating_sub(earlier);
    if delta < 0 {
        delta = delta.saturating_add(PTS_WRAP);
    }
    (delta <= PTS_WRAP / 2).then_some(delta)
}

fn earlier_pts_90k(lhs: i64, rhs: i64) -> i64 {
    if pts_delta_90k(lhs, rhs).is_some() {
        rhs
    } else {
        lhs
    }
}

fn pts90k_ticks_to_frames(ticks: i64, sample_rate: u32) -> usize {
    if ticks <= 0 || sample_rate == 0 {
        return 0;
    }
    (((ticks as i128) * (sample_rate as i128) + 45_000) / 90_000).clamp(0, usize::MAX as i128)
        as usize
}

#[derive(Clone)]
pub struct MpegRgbaFrame {
    pub pts_ms: i64,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Clone)]
pub struct MpegAudioF32 {
    pub pts_ms: i64,
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
}

#[derive(Clone)]
pub enum MpegAvEvent {
    Video(MpegRgbaFrame),
    Audio(MpegAudioF32),
}

#[derive(Default)]
pub struct MpegAvPipeline {
    demux: Demuxer,
    vdec: VideoDecoder,
    adec: MpaAudioDecoder,
    ac3dec: Ac3AudioDecoder,

    pkts: Vec<Packet>,
    pub stash: VecDeque<MpegAvEvent>,
}

impl MpegAvPipeline {
    pub fn new() -> Self {
        Self {
            demux: Demuxer::new_auto(),
            vdec: VideoDecoder::new(),
            adec: MpaAudioDecoder::new(),
            ac3dec: Ac3AudioDecoder::new(),
            pkts: Vec::new(),
            stash: VecDeque::new(),
        }
    }

    #[inline]
    pub fn demuxer_mut(&mut self) -> &mut Demuxer {
        &mut self.demux
    }

    #[inline]
    pub fn video_decoder_mut(&mut self) -> &mut VideoDecoder {
        &mut self.vdec
    }

    #[inline]
    pub fn audio_decoder_mut(&mut self) -> &mut MpaAudioDecoder {
        &mut self.adec
    }

    #[inline]
    pub fn ac3_audio_decoder_mut(&mut self) -> &mut Ac3AudioDecoder {
        &mut self.ac3dec
    }

    pub fn push_with<F>(&mut self, data: &[u8], pts_90k: Option<i64>, mut on_event: F) -> Result<()>
    where
        F: FnMut(MpegAvEvent),
    {
        self.pkts.clear();
        self.demux.push_into(data, pts_90k, &mut self.pkts);

        // Move packets out to avoid borrowing self.pkts while calling &mut self handlers.
        let mut local_pkts: Vec<Packet> = Vec::new();
        std::mem::swap(&mut self.pkts, &mut local_pkts);

        for pkt in local_pkts.drain(..) {
            match pkt.stream_type {
                StreamType::MpegVideo => self.handle_video_pkt(&pkt, &mut on_event)?,
                StreamType::MpegAudio => self.handle_audio_pkt(&pkt, &mut on_event)?,
                StreamType::DvdLpcmAudio => {
                    self.handle_dvd_private_audio_pkt(&pkt, &mut on_event)?
                }
                StreamType::Unknown => {}
            }
        }

        std::mem::swap(&mut self.pkts, &mut local_pkts);
        self.pkts.clear();

        Ok(())
    }

    pub fn push(&mut self, data: &[u8], pts_90k: Option<i64>) -> Result<()> {
        let mut tmp: Vec<MpegAvEvent> = Vec::new();
        self.push_with(data, pts_90k, |ev| tmp.push(ev))?;
        for ev in tmp {
            self.stash.push_back(ev);
        }
        Ok(())
    }

    pub fn flush_with<F>(&mut self, mut on_event: F) -> Result<()>
    where
        F: FnMut(MpegAvEvent),
    {
        // Video: flush delayed frames.
        for f in self.vdec.flush_shared()? {
            self.emit_video_frame(f, &mut on_event)?;
        }
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        let mut tmp: Vec<MpegAvEvent> = Vec::new();
        self.flush_with(|ev| tmp.push(ev))?;
        for ev in tmp {
            self.stash.push_back(ev);
        }
        Ok(())
    }

    fn handle_video_pkt<F>(&mut self, pkt: &Packet, on_event: &mut F) -> Result<()>
    where
        F: FnMut(MpegAvEvent),
    {
        let decoded: Vec<Arc<Frame>> = self.vdec.decode_shared(&pkt.data, pkt.pts_90k)?;
        for f in decoded {
            self.emit_video_frame(f, on_event)?;
        }
        Ok(())
    }

    fn emit_video_frame<F>(&mut self, f: Arc<Frame>, on_event: &mut F) -> Result<()>
    where
        F: FnMut(MpegAvEvent),
    {
        let w = f.width as u32;
        let h = f.height as u32;
        let mut rgba = vec![0u8; (w as usize) * (h as usize) * 4];
        frame_to_rgba_bt601_limited(&f, &mut rgba);

        let pts_ms = pts90k_opt_to_ms(f.pts_90k);
        on_event(MpegAvEvent::Video(MpegRgbaFrame {
            pts_ms,
            width: w,
            height: h,
            rgba,
        }));
        Ok(())
    }

    fn handle_audio_pkt<F>(&mut self, pkt: &Packet, on_event: &mut F) -> Result<()>
    where
        F: FnMut(MpegAvEvent),
    {
        let pts_ms_opt = pkt.pts_90k.map(pts90k_to_ms);
        let audio_result = self
            .adec
            .push_with(&pkt.data, pts_ms_opt, |ch: MpaAudioChunk| {
                on_event(MpegAvEvent::Audio(MpegAudioF32 {
                    pts_ms: ch.pts_ms,
                    sample_rate: ch.sample_rate,
                    channels: ch.channels,
                    samples: ch.samples,
                }))
            });
        if let Err(err) = audio_result
            && (std::env::var_os("SG_MOVIE_TRACE").is_some()
                || std::env::var_os("SG_DEBUG").is_some())
        {
            eprintln!("[SG_DEBUG][MOV] mpa.audio_packet.drop: {err}");
        }
        Ok(())
    }

    fn handle_dvd_private_audio_pkt<F>(&mut self, pkt: &Packet, on_event: &mut F) -> Result<()>
    where
        F: FnMut(MpegAvEvent),
    {
        let pts_ms = pts90k_opt_to_ms(pkt.pts_90k);
        let Some((&substream_id, rest)) = pkt.data.split_first() else {
            return Ok(());
        };

        if (0x80..=0x87).contains(&substream_id) {
            let payload = if rest.len() >= 3 { &rest[3..] } else { rest };
            let audio_result = self
                .ac3dec
                .push_with(payload, Some(pts_ms), |ch: Ac3AudioChunk| {
                    on_event(MpegAvEvent::Audio(MpegAudioF32 {
                        pts_ms: ch.pts_ms,
                        sample_rate: ch.sample_rate,
                        channels: ch.channels,
                        samples: ch.samples,
                    }))
                });
            if let Err(err) = audio_result
                && (std::env::var_os("SG_MOVIE_TRACE").is_some()
                    || std::env::var_os("SG_DEBUG").is_some())
            {
                eprintln!("[SG_DEBUG][MOV] ac3.audio_packet.drop: {err}");
            }
            return Ok(());
        }

        if (0xA0..=0xAF).contains(&substream_id) {
            if let Some(audio) = decode_dvd_private_lpcm(&pkt.data, pts_ms) {
                on_event(MpegAvEvent::Audio(audio));
            }
            return Ok(());
        }

        if std::env::var_os("SG_MOVIE_TRACE").is_some() || std::env::var_os("SG_DEBUG").is_some() {
            eprintln!(
                "[SG_DEBUG][MOV] dvd_private_audio.unsupported substream=0x{substream_id:02x} bytes={}",
                pkt.data.len()
            );
        }
        Ok(())
    }
}

/// Audio-only MPEG program/transport stream pipeline.
///
/// Unlike [`MpegAvPipeline`], this pipeline never instantiates the MPEG video
/// decoder and never converts video frames to RGBA. It is intended for movie
/// playback paths that decode video and audio on separate workers.
pub struct MpegAudioPipeline {
    demux: Demuxer,
    adec: MpaAudioDecoder,
    ac3dec: Ac3AudioDecoder,
    pkts: Vec<Packet>,
    first_video_pts_ms: Option<i64>,
}

impl Default for MpegAudioPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl MpegAudioPipeline {
    pub fn new() -> Self {
        Self {
            demux: Demuxer::new_auto(),
            adec: MpaAudioDecoder::new(),
            ac3dec: Ac3AudioDecoder::new(),
            pkts: Vec::new(),
            first_video_pts_ms: None,
        }
    }

    #[inline]
    pub fn demuxer_mut(&mut self) -> &mut Demuxer {
        &mut self.demux
    }

    #[inline]
    pub fn audio_decoder_mut(&mut self) -> &mut MpaAudioDecoder {
        &mut self.adec
    }

    #[inline]
    pub fn ac3_audio_decoder_mut(&mut self) -> &mut Ac3AudioDecoder {
        &mut self.ac3dec
    }

    #[inline]
    pub fn first_video_pts_ms(&self) -> Option<i64> {
        self.first_video_pts_ms
    }

    pub fn push_with<F>(&mut self, data: &[u8], pts_90k: Option<i64>, mut on_audio: F) -> Result<()>
    where
        F: FnMut(MpegAudioF32),
    {
        self.pkts.clear();
        self.demux.push_into(data, pts_90k, &mut self.pkts);

        let mut local_pkts = Vec::new();
        std::mem::swap(&mut self.pkts, &mut local_pkts);
        for pkt in local_pkts.drain(..) {
            match pkt.stream_type {
                StreamType::MpegAudio => {
                    let pts_ms = pkt.pts_90k.map(pts90k_to_ms);
                    let result = self.adec.push_with(&pkt.data, pts_ms, |ch: MpaAudioChunk| {
                        on_audio(MpegAudioF32 {
                            pts_ms: ch.pts_ms,
                            sample_rate: ch.sample_rate,
                            channels: ch.channels,
                            samples: ch.samples,
                        });
                    });
                    if let Err(err) = result
                        && (std::env::var_os("SG_MOVIE_TRACE").is_some()
                            || std::env::var_os("SG_DEBUG").is_some())
                    {
                        eprintln!("[SG_DEBUG][MOV] mpa.audio_packet.drop: {err}");
                    }
                }
                StreamType::DvdLpcmAudio => {
                    let pts_ms = pts90k_opt_to_ms(pkt.pts_90k);
                    let Some((&substream_id, rest)) = pkt.data.split_first() else {
                        continue;
                    };
                    if (0x80..=0x87).contains(&substream_id) {
                        let payload = if rest.len() >= 3 { &rest[3..] } else { rest };
                        let result =
                            self.ac3dec
                                .push_with(payload, Some(pts_ms), |ch: Ac3AudioChunk| {
                                    on_audio(MpegAudioF32 {
                                        pts_ms: ch.pts_ms,
                                        sample_rate: ch.sample_rate,
                                        channels: ch.channels,
                                        samples: ch.samples,
                                    });
                                });
                        if let Err(err) = result
                            && (std::env::var_os("SG_MOVIE_TRACE").is_some()
                                || std::env::var_os("SG_DEBUG").is_some())
                        {
                            eprintln!("[SG_DEBUG][MOV] ac3.audio_packet.drop: {err}");
                        }
                    } else if (0xA0..=0xAF).contains(&substream_id) {
                        if let Some(audio) = decode_dvd_private_lpcm(&pkt.data, pts_ms) {
                            on_audio(audio);
                        }
                    } else if std::env::var_os("SG_MOVIE_TRACE").is_some()
                        || std::env::var_os("SG_DEBUG").is_some()
                    {
                        eprintln!(
                            "[SG_DEBUG][MOV] dvd_private_audio.unsupported substream=0x{substream_id:02x} bytes={}",
                            pkt.data.len()
                        );
                    }
                }
                StreamType::MpegVideo => {
                    if self.first_video_pts_ms.is_none() {
                        self.first_video_pts_ms = pkt.pts_90k.map(pts90k_to_ms);
                    }
                }
                StreamType::Unknown => {}
            }
        }
        std::mem::swap(&mut self.pkts, &mut local_pkts);
        self.pkts.clear();
        Ok(())
    }

    /// MPEG audio, AC-3 and DVD LPCM decoders emit complete frames while data
    /// is pushed. There is no delayed video-style frame queue to flush.
    pub fn flush_with<F>(&mut self, _on_audio: F) -> Result<()>
    where
        F: FnMut(MpegAudioF32),
    {
        Ok(())
    }
}

#[inline]
fn pts90k_to_ms(v: i64) -> i64 {
    (v * 1000) / 90000
}

#[inline]
fn pts90k_opt_to_ms(v: Option<i64>) -> i64 {
    v.map(pts90k_to_ms).unwrap_or(0)
}

fn decode_dvd_private_lpcm(data: &[u8], pts_ms: i64) -> Option<MpegAudioF32> {
    if data.len() < 8 {
        return None;
    }
    let substream_id = data[0];
    if !(0xA0..=0xAF).contains(&substream_id) {
        if (std::env::var_os("SG_MOVIE_TRACE").is_some() || std::env::var_os("SG_DEBUG").is_some())
            && ((0x80..=0x8F).contains(&substream_id) || (0x90..=0x9F).contains(&substream_id))
        {
            eprintln!(
                "[SG_DEBUG][MOV] dvd_private_audio.unsupported substream=0x{substream_id:02x} bytes={}",
                data.len()
            );
        }
        return None;
    }

    // DVD private_stream_1 audio packets carry a substream id followed by a
    // 3-byte private-stream header.  The following LPCM payload begins with
    // the 3-byte DVD LPCM audio header.
    let lpcm = &data[4..];
    if lpcm.len() < 4 {
        return None;
    }

    let format = lpcm[1];
    let bits_code = (format >> 6) & 0x03;
    let rate_code = (format >> 4) & 0x03;
    let channels = ((format & 0x07) + 1) as u16;
    let sample_rate = match rate_code {
        0 => 48_000,
        1 => 96_000,
        2 => 44_100,
        3 => 32_000,
        _ => return None,
    };
    let bits_per_sample = match bits_code {
        0 => 16,
        1 => 20,
        2 => 24,
        _ => return None,
    };

    let pcm = &lpcm[3..];
    let mut samples = Vec::new();
    match bits_per_sample {
        16 => {
            let sample_count = pcm.len() / 2;
            samples.reserve(sample_count);
            for chunk in pcm.as_chunks::<2>().0 {
                let v = i16::from_be_bytes([chunk[0], chunk[1]]) as f32 / 32768.0;
                samples.push(v.clamp(-1.0, 1.0));
            }
        }
        24 => {
            let sample_count = pcm.len() / 3;
            samples.reserve(sample_count);
            for chunk in pcm.as_chunks::<3>().0 {
                let raw = ((chunk[0] as i32) << 24)
                    | ((chunk[1] as i32) << 16)
                    | ((chunk[2] as i32) << 8);
                let v = raw as f32 / 2_147_483_648.0;
                samples.push(v.clamp(-1.0, 1.0));
            }
        }
        20 => {
            if std::env::var_os("SG_MOVIE_TRACE").is_some()
                || std::env::var_os("SG_DEBUG").is_some()
            {
                eprintln!(
                    "[SG_DEBUG][MOV] dvd_lpcm_20bit.unsupported substream=0x{substream_id:02x} rate={} channels={} bytes={}",
                    sample_rate,
                    channels,
                    pcm.len()
                );
            }
            return None;
        }
        _ => return None,
    }

    if samples.is_empty() || channels == 0 {
        return None;
    }

    Some(MpegAudioF32 {
        pts_ms,
        sample_rate,
        channels,
        samples,
    })
}

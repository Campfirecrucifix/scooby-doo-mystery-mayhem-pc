use std::collections::VecDeque;

use crate::error::{AvError, Result};

use symphonia::core::audio::{SampleBuffer, SignalSpec};
use symphonia::core::codecs::{
    CODEC_TYPE_MP1, CODEC_TYPE_MP2, CODEC_TYPE_MP3, CodecParameters, CodecType, Decoder,
    DecoderOptions,
};
use symphonia::core::formats::Packet;

#[derive(Clone)]
pub struct MpaAudioChunk {
    pub pts_ms: i64,
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
}

#[derive(Default)]
pub struct MpaAudioDecoder {
    buf: Vec<u8>,

    dec: Option<Box<dyn Decoder>>,
    dec_codec: Option<CodecType>,
    sample_buf: Option<SampleBuffer<f32>>,
    sample_spec: Option<SignalSpec>,

    // Best-effort PTS tracking. PES boundaries are not guaranteed to align
    // with MPEG audio frame boundaries, so PTS anchors are associated with
    // absolute byte offsets and applied only when a new audio frame begins.
    next_pts_ms: Option<i64>,
    pts_anchors: VecDeque<(u64, i64)>,
    buffer_stream_offset: u64,

    // Symphonia packet track id (arbitrary but must be consistent).
    track_id: u32,
}

impl MpaAudioDecoder {
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            dec: None,
            dec_codec: None,
            sample_buf: None,
            sample_spec: None,
            next_pts_ms: None,
            pts_anchors: VecDeque::new(),
            buffer_stream_offset: 0,
            track_id: 0,
        }
    }

    pub fn push_with<F>(&mut self, data: &[u8], pts_ms: Option<i64>, mut on_chunk: F) -> Result<()>
    where
        F: FnMut(MpaAudioChunk),
    {
        // The PTS carried by a PES packet belongs to the first complete audio
        // access unit that starts in that packet. A previous PES may leave a
        // partial MPEG audio frame in `self.buf`; applying the new PTS before
        // completing that frame creates artificial gaps and repeated samples.
        let new_data_offset = self
            .buffer_stream_offset
            .saturating_add(self.buf.len() as u64);
        if let Some(pts) = pts_ms {
            self.pts_anchors.push_back((new_data_offset, pts));
        }

        self.buf.extend_from_slice(data);

        let mut pos = 0usize;
        while pos + 4 <= self.buf.len() {
            let Some(h) = MpaHeader::parse(&self.buf[pos..]) else {
                pos += 1;
                continue;
            };

            if pos + h.frame_len > self.buf.len() {
                break;
            }

            let frame_stream_offset = self.buffer_stream_offset.saturating_add(pos as u64);
            while let Some(&(anchor_offset, anchor_pts)) = self.pts_anchors.front() {
                if anchor_offset > frame_stream_offset {
                    break;
                }
                self.next_pts_ms = Some(anchor_pts);
                self.pts_anchors.pop_front();
            }

            // Avoid borrowing self.buf while calling into self (decoder state).
            let pkt_owned = self.buf[pos..pos + h.frame_len].to_vec();
            pos += h.frame_len;

            let pts0 = self.next_pts_ms.unwrap_or(0);
            self.decode_one_packet(&pkt_owned, pts0, h, &mut on_chunk)?;
        }

        if pos > 0 {
            self.buf.drain(0..pos);
            self.buffer_stream_offset = self.buffer_stream_offset.saturating_add(pos as u64);
        }

        Ok(())
    }

    fn decode_one_packet<F>(
        &mut self,
        pkt_bytes: &[u8],
        pts_ms: i64,
        header: MpaHeader,
        on_chunk: &mut F,
    ) -> Result<()>
    where
        F: FnMut(MpaAudioChunk),
    {
        let codec_type = header.codec_type;
        if self.dec.is_none() || self.dec_codec != Some(codec_type) {
            let mut cp = CodecParameters::new();
            cp.for_codec(codec_type);

            let dec = match symphonia::default::get_codecs()
                .make(&cp, &DecoderOptions::default())
                .map_err(AvError::from)
            {
                Ok(dec) => dec,
                Err(err) => {
                    if std::env::var_os("SG_MOVIE_TRACE").is_some()
                        || std::env::var_os("SG_DEBUG").is_some()
                    {
                        eprintln!("[SG_DEBUG][MOV] mpa.decoder.open.drop: {err}");
                    }
                    self.dec = None;
                    self.dec_codec = None;
                    self.sample_buf = None;
                    self.sample_spec = None;
                    self.advance_pts_after_drop(pts_ms, header);
                    return Ok(());
                }
            };
            self.dec = Some(dec);
            self.dec_codec = Some(codec_type);
            self.sample_buf = None;
            self.sample_spec = None;
        }

        let pkt = Packet::new_from_boxed_slice(
            self.track_id,
            0,
            0,
            pkt_bytes.to_vec().into_boxed_slice(),
        );

        let dec = self.dec.as_mut().expect("decoder must be initialized");
        match dec.decode(&pkt) {
            Ok(decoded) => {
                let spec = *decoded.spec();
                let duration = decoded.capacity();
                let duration_u64 = duration as u64;

                let need_new_buf = match (self.sample_buf.as_ref(), self.sample_spec.as_ref()) {
                    (Some(sb), Some(cur_spec)) => {
                        sb.capacity() < duration || !same_signal_spec(cur_spec, &spec)
                    }
                    _ => true,
                };
                if need_new_buf {
                    self.sample_buf = Some(SampleBuffer::<f32>::new(duration_u64, spec));
                    self.sample_spec = Some(spec);
                }

                let sb = self.sample_buf.as_mut().expect("sample buffer must exist");
                sb.copy_interleaved_ref(decoded.clone());

                let channels = spec.channels.count() as u16;
                let frames = decoded.frames();
                let valid_sample_count = frames.saturating_mul(channels as usize);
                let all_samples = sb.samples();
                let sample_count = valid_sample_count.min(all_samples.len());
                let samples = all_samples[..sample_count].to_vec();

                let sample_rate = spec.rate;
                if channels != 0 && sample_rate != 0 && !samples.is_empty() {
                    on_chunk(MpaAudioChunk {
                        pts_ms,
                        sample_rate,
                        channels,
                        samples,
                    });
                }

                let frames_i64 = frames as i64;
                if frames_i64 > 0 && sample_rate > 0 {
                    let dur_ms = (frames_i64 * 1000 + sample_rate as i64 / 2) / sample_rate as i64;
                    self.next_pts_ms = Some(pts_ms.saturating_add(dur_ms.max(1)));
                }
            }
            Err(e) => {
                if std::env::var_os("SG_MOVIE_TRACE").is_some()
                    || std::env::var_os("SG_DEBUG").is_some()
                {
                    eprintln!("[SG_DEBUG][MOV] mpa.audio_decode.drop: {e}");
                }
                self.dec = None;
                self.dec_codec = None;
                self.sample_buf = None;
                self.sample_spec = None;
                self.advance_pts_after_drop(pts_ms, header);
            }
        }

        Ok(())
    }

    fn advance_pts_after_drop(&mut self, pts_ms: i64, header: MpaHeader) {
        if header.sample_rate == 0 || header.samples_per_frame == 0 {
            return;
        }
        let duration_ms = ((header.samples_per_frame as i64) * 1000
            + (header.sample_rate as i64 / 2))
            / header.sample_rate as i64;
        self.next_pts_ms = Some(pts_ms.saturating_add(duration_ms.max(1)));
    }
}

fn same_signal_spec(a: &SignalSpec, b: &SignalSpec) -> bool {
    a.rate == b.rate && a.channels == b.channels
}

#[derive(Clone, Copy)]
struct MpaHeader {
    frame_len: usize,
    codec_type: CodecType,
    sample_rate: u32,
    samples_per_frame: u32,
    channels: u16,
}

/// Lightweight MPEG-audio frame counter used by the movie streaming path.
///
/// It follows the same resynchronization and frame-length rules as
/// [`MpaAudioDecoder`] but never invokes the sample decoder.  This allows Kira
/// to receive an exact finite frame count without pre-decoding the whole movie
/// into PCM before playback starts.
#[derive(Debug, Default)]
pub struct MpaFrameProbe {
    buf: Vec<u8>,
    first_sample_rate: Option<u32>,
    first_channels: Option<u16>,
    output_frames: usize,
    compressed_frames: usize,
}

impl MpaFrameProbe {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
        let mut pos = 0usize;
        while pos + 4 <= self.buf.len() {
            let Some(header) = MpaHeader::parse(&self.buf[pos..]) else {
                pos += 1;
                continue;
            };
            if pos + header.frame_len > self.buf.len() {
                break;
            }
            let dst_rate = *self.first_sample_rate.get_or_insert(header.sample_rate);
            self.first_channels.get_or_insert(header.channels);
            let converted_frames = ((header.samples_per_frame as u128) * (dst_rate as u128)
                / (header.sample_rate as u128))
                .max(1) as usize;
            self.output_frames = self.output_frames.saturating_add(converted_frames);
            self.compressed_frames = self.compressed_frames.saturating_add(1);
            pos += header.frame_len;
        }
        if pos > 0 {
            self.buf.drain(0..pos);
        }
    }

    pub fn first_sample_rate(&self) -> Option<u32> {
        self.first_sample_rate
    }

    pub fn first_channels(&self) -> Option<u16> {
        self.first_channels
    }

    pub fn output_frames(&self) -> usize {
        self.output_frames
    }

    pub fn compressed_frames(&self) -> usize {
        self.compressed_frames
    }
}

impl MpaHeader {
    fn parse(buf: &[u8]) -> Option<Self> {
        if buf.len() < 4 {
            return None;
        }
        let b0 = buf[0];
        let b1 = buf[1];
        let b2 = buf[2];

        // Sync.
        if b0 != 0xFF || (b1 & 0xE0) != 0xE0 {
            return None;
        }

        let version_id = (b1 >> 3) & 0x03;
        let layer_id = (b1 >> 1) & 0x03;
        if version_id == 0x01 || layer_id == 0x00 {
            return None;
        }

        let bitrate_idx = (b2 >> 4) & 0x0F;
        let sr_idx = (b2 >> 2) & 0x03;
        if bitrate_idx == 0 || bitrate_idx == 0x0F || sr_idx == 0x03 {
            return None;
        }

        let padding: u32 = ((b2 >> 1) & 0x01) as u32;

        let (sr, is_v1) = match version_id {
            0x03 => (SAMPLE_RATES_V1[sr_idx as usize], true),
            0x02 => (SAMPLE_RATES_V2[sr_idx as usize], false),
            0x00 => (SAMPLE_RATES_V25[sr_idx as usize], false),
            _ => return None,
        };

        let (codec_type, bitrate_kbps, frame_len, samples_per_frame) = match layer_id {
            0x03 => {
                // Layer I
                let br = if is_v1 {
                    BITRATES_V1_L1[bitrate_idx as usize]
                } else {
                    BITRATES_V2_L1[bitrate_idx as usize]
                };
                let fl =
                    (((12u64 * (br as u64) * 1000u64) / (sr as u64)) + (padding as u64)) * 4u64;
                (CODEC_TYPE_MP1, br, fl as usize, 384)
            }
            0x02 => {
                // Layer II
                let br = if is_v1 {
                    BITRATES_V1_L2[bitrate_idx as usize]
                } else {
                    BITRATES_V2_L2L3[bitrate_idx as usize]
                };
                let fl = ((144u64 * (br as u64) * 1000u64) / (sr as u64)) + (padding as u64);
                (CODEC_TYPE_MP2, br, fl as usize, 1152)
            }
            0x01 => {
                // Layer III
                let br = if is_v1 {
                    BITRATES_V1_L3[bitrate_idx as usize]
                } else {
                    BITRATES_V2_L2L3[bitrate_idx as usize]
                };
                let coeff: u64 = if is_v1 { 144 } else { 72 };
                let fl = ((coeff * (br as u64) * 1000u64) / (sr as u64)) + (padding as u64);
                (
                    CODEC_TYPE_MP3,
                    br,
                    fl as usize,
                    if is_v1 { 1152 } else { 576 },
                )
            }
            _ => return None,
        };

        if bitrate_kbps == 0 || frame_len < 4 {
            return None;
        }

        let channel_mode = (buf[3] >> 6) & 0x03;
        let channels = if channel_mode == 0x03 { 1 } else { 2 };

        Some(Self {
            frame_len,
            codec_type,
            sample_rate: sr,
            samples_per_frame,
            channels,
        })
    }
}

const SAMPLE_RATES_V1: [u32; 3] = [44100, 48000, 32000];
const SAMPLE_RATES_V2: [u32; 3] = [22050, 24000, 16000];
const SAMPLE_RATES_V25: [u32; 3] = [11025, 12000, 8000];

const BITRATES_V1_L1: [u32; 16] = [
    0, 32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448, 0,
];
const BITRATES_V1_L2: [u32; 16] = [
    0, 32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384, 0,
];
const BITRATES_V1_L3: [u32; 16] = [
    0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0,
];

const BITRATES_V2_L1: [u32; 16] = [
    0, 32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256, 0,
];
const BITRATES_V2_L2L3: [u32; 16] = [
    0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0,
];

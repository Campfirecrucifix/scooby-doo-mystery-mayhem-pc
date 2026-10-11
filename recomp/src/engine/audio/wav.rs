//! Mono i16 PCM -> stereo 16-bit WAV with per-channel gains (used by the Bevy adapter; testable
//! without Bevy).
pub fn pcm_to_wav(pcm: &[i16], rate: u32, left: f32, right: f32) -> Vec<u8> {
    let n = pcm.len() as u32 * 4;
    let mut w = Vec::with_capacity(44 + n as usize);
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + n).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    for x in [16u32] { w.extend_from_slice(&x.to_le_bytes()); }
    for x in [1u16, 2] { w.extend_from_slice(&x.to_le_bytes()); }
    w.extend_from_slice(&rate.to_le_bytes());
    w.extend_from_slice(&(rate * 4).to_le_bytes());
    for x in [4u16, 16] { w.extend_from_slice(&x.to_le_bytes()); }
    w.extend_from_slice(b"data");
    w.extend_from_slice(&n.to_le_bytes());
    for &s in pcm {
        for g in [left, right] { w.extend_from_slice(&((s as f32 * g.clamp(0.0, 1.0)) as i16).to_le_bytes()); }
    }
    w
}

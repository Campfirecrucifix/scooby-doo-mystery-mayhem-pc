//! STREAM bank samples (SBH kind 2): STREAM\..\*.VAG files read from the disc and decoded with the
//! VAG decoder. REPLACEMENT: the IOP streams them in chunks (SOUND_PlayStream); we decode whole.
use super::mgr::Voice;
use super::vag::{decode_vag, DecodedPcm};

/// Load + decode a voice's stream file. `read(path)` reads an ISO path ('/' separators).
pub fn load_stream(v: &Voice, read: &mut dyn FnMut(&str) -> Option<Vec<u8>>) -> Option<(u32, DecodedPcm)> {
    let path = v.stream.as_ref()?;
    let (h, pcm) = decode_vag(&read(path)?);
    let rate = if v.sample_rate != 0 { v.sample_rate } else { h.map_or(0, |h| h.sample_rate) };
    Some((rate, pcm))
}

/// A3: stream voice source for incremental playback: (rate, raw ADPCM body after the VAG header).
/// The file bytes are read on the loader thread; decoding happens block by block while the voice
/// plays (spu::AdpcmStream), like the IOP's chunked SOUND_PlayStream.
pub fn load_stream_adpcm(v: &Voice, read: &mut dyn FnMut(&str) -> Option<Vec<u8>>) -> Option<(u32, std::sync::Arc<[u8]>)> {
    let f = read(v.stream.as_ref()?)?;
    let (rate, body) = if f.len() >= 0x30 && &f[0..4] == b"VAGp" {
        let be = |o: usize| u32::from_be_bytes(f[o..o + 4].try_into().unwrap());
        (be(0x10), &f[0x30..(0x30 + be(0xc) as usize).min(f.len())])
    } else if super::spu::is_interleaved(&f) {
        // A4: .INT stereo music: whole file kept (spu::SpuData::Interleaved splits L/R), rate BE @0x10
        (u32::from_be_bytes(f[0x10..0x14].try_into().unwrap()), &f[..])
    } else {
        (0, &f[..])
    };
    Some((if v.sample_rate != 0 { v.sample_rate } else { rate }, body.into()))
}

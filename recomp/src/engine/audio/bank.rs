//! SBB sample banks (raw SPU ADPCM) indexed through the SBH (see `sbh`).
//! 1:1: sample i starts at SBH sample[i].sbb_offset (SOUND_PatchSBH -> SOUND_PatchSFX(base + off)).
//! REPLACEMENT: the SPU plays from that address until the ADPCM end flag; we decode the same span
//! (up to the next sample start / SBB end, stopping at the end flag) into PCM.
//! STREAM banks (SBH kind 2) have an empty SBB; their sample names are STREAM\..\*.VAG paths.
use super::sbh::{Sbh, SbhSample};
use super::vag::{decode_adpcm, DecodedPcm};
use crate::engine::error::EngineError;

#[derive(Debug, Clone, Default)]
pub struct SoundBank {
    pub name: String,
    pub sbh: Sbh,
    /// decoded PCM per SBH sample (None for stream samples / out-of-range offsets).
    pub pcm: Vec<Option<DecodedPcm>>,
}

impl SoundBank {
    pub fn load(name: &str, sbb: &[u8], sbh: &[u8]) -> Result<Self, EngineError> {
        let sbh = Sbh::parse(sbh)?;
        let mut starts: Vec<usize> = sbh.samples.iter().map(|s| s.sbb_offset as usize).collect();
        starts.push(sbb.len());
        starts.sort_unstable();
        let pcm = sbh
            .samples
            .iter()
            .map(|s| {
                let o = s.sbb_offset as usize;
                if sbh.is_stream() || o >= sbb.len() {
                    return None;
                }
                let end = starts.iter().copied().find(|&e| e > o).unwrap_or(sbb.len());
                let p = decode_adpcm(&sbb[o..end], true);
                Some(p)
            })
            .collect();
        Ok(SoundBank { name: name.into(), sbh, pcm })
    }
    pub fn sample(&self, i: u32) -> Option<&SbhSample> { self.sbh.samples.get(i as usize) }
}

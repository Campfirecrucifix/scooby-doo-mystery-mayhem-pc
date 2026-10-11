//! PS2 SPU2 ADPCM ("VAG") decoder -> i16 PCM.
//! REPLACEMENT (not 1:1): on the PS2 the SPU2 hardware decodes ADPCM. This is the standard
//! SPU ADPCM algorithm: 16-byte blocks {u8 shift|filter<<4, u8 flags, 14 bytes = 28 nibbles}.
//! Flags: bit0 = loop end (end of sample when bit1 clear), bit1 = loop repeat, bit2 = loop start.

/// SPU ADPCM prediction filters (x/64).
pub const FILTERS: [[i32; 2]; 5] = [[0, 0], [60, 0], [115, -52], [98, -55], [122, -60]];

/// Decoded sample: PCM plus loop information found in the block flags.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DecodedPcm {
    pub samples: Vec<i16>,
    /// Sample index of the loop start (block with flag 4), if the data loops (flag 3 at the end).
    pub loop_start: Option<usize>,
    pub looped: bool,
}

/// Decode raw SPU ADPCM blocks (no VAG header). Stops at the first block with the end flag
/// (bit0 set) — like the SPU, which stops (or jumps to the loop start) after that block.
/// `stop_at_end=false` decodes every block (used for headerless streams).
pub fn decode_adpcm(data: &[u8], stop_at_end: bool) -> DecodedPcm {
    let mut out = DecodedPcm { samples: Vec::with_capacity(data.len() / 16 * 28), ..Default::default() };
    let (mut s1, mut s2) = (0i32, 0i32);
    for blk in data.chunks_exact(16) {
        let shift = (blk[0] & 0x0f) as i32;
        let filter = ((blk[0] >> 4) as usize).min(4);
        let flags = blk[1];
        if flags & 4 != 0 && out.loop_start.is_none() {
            out.loop_start = Some(out.samples.len());
        }
        let [f0, f1] = FILTERS[filter];
        for i in 0..28 {
            let byte = blk[2 + i / 2];
            let nib = if i & 1 == 0 { byte & 0x0f } else { byte >> 4 } as i32;
            let mut s = ((nib << 28) >> 28) << 12; // sign-extend 4 bits, scale to 16 bits
            s >>= shift.min(12);
            let v = s + ((s1 * f0 + s2 * f1 + 32) >> 6);
            let v = v.clamp(-32768, 32767);
            out.samples.push(v as i16);
            s2 = s1;
            s1 = v;
        }
        if stop_at_end && flags & 1 != 0 {
            out.looped = flags & 2 != 0;
            break;
        }
    }
    if !out.looped {
        out.loop_start = None;
    }
    out
}

/// Parsed "VAGp" file header (big-endian fields, 0x30 bytes).
#[derive(Debug, Clone, PartialEq)]
pub struct VagHeader {
    pub version: u32,
    pub data_size: u32,
    pub sample_rate: u32,
    pub name: String,
}

/// Parse a VAG file: returns header (if present) and the decoded PCM. Files without the
/// "VAGp" magic are decoded as raw ADPCM.
pub fn decode_vag(file: &[u8]) -> (Option<VagHeader>, DecodedPcm) {
    if file.len() >= 0x30 && &file[0..4] == b"VAGp" {
        let be = |o: usize| u32::from_be_bytes(file[o..o + 4].try_into().unwrap());
        let name_end = file[0x20..0x30].iter().position(|&c| c == 0).unwrap_or(16);
        let h = VagHeader {
            version: be(4),
            data_size: be(0xc),
            sample_rate: be(0x10),
            name: String::from_utf8_lossy(&file[0x20..0x20 + name_end]).into_owned(),
        };
        let end = (0x30 + h.data_size as usize).min(file.len());
        (Some(h), decode_adpcm(&file[0x30..end], true))
    } else {
        (None, decode_adpcm(file, true))
    }
}

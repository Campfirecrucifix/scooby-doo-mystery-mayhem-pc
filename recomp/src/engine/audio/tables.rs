//! EFTSCTable (.TSC sound-container table) and EFSETTable (.TSE sound-event table): 1:1 port of
//! EFTSCTable::Initialize / EFSETTable::Initialize (Classes/EF/EFTSCTable.cpp, EFSETTable.cpp).
//! File = A2M header {u32 'A2M\0', u32 platform 'PS2\0', u32 version (major<<16|minor), u32 count}
//! then `count` records. Path resolution (LEVEL/<lv>/<lang>/AUDIO, GLOBAL/<ep><lang>/AUDIO, GEN
//! fallbacks) is done by the caller (`audio::paths`); these take the file bytes.
#![allow(non_snake_case)]

use crate::engine::error::EngineError;

pub const A2M_MAGIC: u32 = 0x004d_3241; // "A2M\0"
pub const A2M_PLATFORM_PS2: u32 = 0x0032_5350; // "PS2\0" (Ghidra shows it as a string address)
pub const A2M_MAJOR: u32 = 0x15;
pub const A2M_MINOR: u32 = 0x30c;

pub(crate) fn rd32(b: &[u8], o: usize) -> Result<u32, EngineError> {
    b.get(o..o + 4)
        .map(|s| u32::from_le_bytes(s.try_into().unwrap()))
        .ok_or_else(|| EngineError::Format(format!("short read at {o:#x}")))
}

/// Validate the 16-byte A2M header; returns the record count. Mirrors the checks/messages.
pub fn check_a2m(b: &[u8], what: &str) -> Result<u32, EngineError> {
    if rd32(b, 0)? != A2M_MAGIC {
        return Err(EngineError::Format(format!("{what} file is not a valid A2M file!!!!")));
    }
    if rd32(b, 4)? != A2M_PLATFORM_PS2 {
        return Err(EngineError::Format(format!("{what} file is not a file compiled for this platform!!!!")));
    }
    let v = rd32(b, 8)?;
    if v >> 16 != A2M_MAJOR {
        return Err(EngineError::Format(format!(
            "{what} file was compiled with an incompatible version of the Tool (with v1.{})!!!!",
            v >> 16
        )));
    }
    if v & 0xffff != A2M_MINOR {
        // quirk: only a warning (all shipped files are v1.21.761 = 0x2f9, not 0x30c).
        log::debug!("WARNING!!! {what} file was compiled with another version of the Tool (with v1.{}.{})!!!!", v >> 16, v & 0xffff);
    }
    rd32(b, 12)
}

fn read_name(b: &[u8], o: &mut usize) -> Result<String, EngineError> {
    let n = rd32(b, *o)? as usize;
    *o += 4;
    let s = b.get(*o..*o + n).ok_or_else(|| EngineError::Format("short name".into()))?;
    *o += n;
    Ok(String::from_utf8_lossy(s).into_owned())
}

fn read_u32s(b: &[u8], o: &mut usize, n: usize) -> Result<Vec<u32>, EngineError> {
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        v.push(rd32(b, *o)?);
        *o += 4;
    }
    Ok(v)
}

/// EFSoundContainerData (0x20 bytes in memory; 0x18 bytes on disk + name + child list).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EFSoundContainerData {
    /// +0x00 sound id of this container (bank<<30 | 0x20000000 | index)
    pub id: u32,
    /// +0x04 group mask (EFNewAudioMgr_BC group volumes, bit n -> group n)
    pub group: u32,
    /// +0x08 container type (see EFSoundContainer::PlaySound)
    pub kind: u32,
    /// +0x0c number of children (u32 ids, 0xffffffff = empty slot)
    pub count: u32,
    /// +0x10
    pub field_10: u32,
    /// +0x14
    pub field_14: u32,
    /// +0x18 char* name
    pub name: String,
    /// +0x1c u32* children
    pub children: Vec<u32>,
}

/// EFSoundEventData (0x44 bytes in memory; 0x3c on disk + name + playback list).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EFSoundEventData {
    /// raw 15 words at +0x00..+0x3c; named accessors below.
    pub w: [u32; 15],
    /// +0x3c char* name
    pub name: String,
    /// +0x40 u32* playback ids (count at +0x24)
    pub playbacks: Vec<u32>,
}

impl EFSoundEventData {
    pub fn word(&self, off: usize) -> u32 { self.w[off / 4] }
    pub fn fword(&self, off: usize) -> f32 { f32::from_bits(self.w[off / 4]) }
    pub fn set_word(&mut self, off: usize, v: u32) { self.w[off / 4] = v; }
    /// +0x08 3D mode (0 2D, 1 3D no pan, 2 3D panned, 3 3D inverted) (ComputeVolumeAndPan)
    pub fn mode(&self) -> u32 { self.word(0x08) }
    /// +0x10/+0x14 min distance / volume there; +0x18/+0x1c max distance / volume there
    pub fn min_dist(&self) -> f32 { self.fword(0x10) }
    pub fn vol_at_min(&self) -> f32 { self.fword(0x14) }
    pub fn max_dist(&self) -> f32 { self.fword(0x18) }
    pub fn vol_at_max(&self) -> f32 { self.fword(0x1c) }
    /// +0x20 cut-off distance (beyond: silent)
    pub fn cutoff_dist(&self) -> f32 { self.fword(0x20) }
    /// +0x28 group mask (GetGroupVolume)
    pub fn group(&self) -> u32 { self.word(0x28) }
    /// +0x2c playable flag (PlayOrphanSoundEvent forces 1)
    pub fn playable(&self) -> bool { self.word(0x2c) != 0 }
    /// +0x32 u8: virtualize (release voice) when beyond the cut-off
    pub fn virtualize(&self) -> bool { self.word(0x30) >> 16 & 0xff != 0 }
}

/// EFTSCTable (0x110 bytes): +4 data array, +8 count, +0xc name[0x100], +0x10c bIsGlobal (param==0).
#[derive(Debug, Clone, Default)]
pub struct EFTSCTable {
    /// this+0x4 / this+0x8
    pub data: Vec<EFSoundContainerData>,
    /// this+0xc
    pub name: String,
    /// this+0x10c: true for bank 0 (global)
    pub is_global: bool,
}

impl EFTSCTable {
    pub fn EFTSCTable(bank: u32) -> Self { Self { is_global: bank == 0, ..Default::default() } }

    /// EFTSCTable::Initialize body after the file was opened (file bytes in `b`).
    pub fn Initialize(&mut self, name: &str, b: &[u8]) -> Result<(), EngineError> {
        let count = check_a2m(b, name)?;
        self.name = name.to_string();
        let mut o = 16;
        self.data.clear();
        for _ in 0..count {
            let h = read_u32s(b, &mut o, 6)?;
            let mut d = EFSoundContainerData { id: h[0], group: h[1], kind: h[2], count: h[3], field_10: h[4], field_14: h[5], ..Default::default() };
            d.name = read_name(b, &mut o)?;
            if d.count != 0 {
                d.children = read_u32s(b, &mut o, d.count as usize)?;
            }
            self.data.push(d);
        }
        Ok(())
    }

    /// GetSoundContainerData(index): NULL + warning when uninitialised / out of range.
    pub fn GetSoundContainerData(&self, idx: u32) -> Option<&EFSoundContainerData> {
        let r = self.data.get(idx as usize);
        if r.is_none() {
            log::warn!("WARNING!!!! EFTSCTable::GetSoundContainerData!! TSCTable not initialised!! A bank was not loaded correctly!");
        }
        r
    }
}

/// EFSETTable (0x110 bytes), same layout as EFTSCTable with 0x44-byte records.
#[derive(Debug, Clone, Default)]
pub struct EFSETTable {
    pub data: Vec<EFSoundEventData>,
    pub name: String,
    pub is_global: bool,
}

impl EFSETTable {
    pub fn EFSETTable(bank: u32) -> Self { Self { is_global: bank == 0, ..Default::default() } }

    pub fn Initialize(&mut self, name: &str, b: &[u8]) -> Result<(), EngineError> {
        let count = check_a2m(b, name)?;
        self.name = name.to_string();
        let mut o = 16;
        self.data.clear();
        for _ in 0..count {
            let h = read_u32s(b, &mut o, 15)?;
            let mut d = EFSoundEventData::default();
            d.w.copy_from_slice(&h);
            d.name = read_name(b, &mut o)?;
            let n = d.word(0x24);
            if n != 0 {
                d.playbacks = read_u32s(b, &mut o, n as usize)?;
            }
            self.data.push(d);
        }
        Ok(())
    }

    pub fn GetSoundEventData(&self, idx: u32) -> Option<&EFSoundEventData> {
        let r = self.data.get(idx as usize);
        if r.is_none() {
            log::warn!("WARNING!!!! EFSETTable::GetSoundEventData!! SETTable not initialised!! A bank was not loaded correctly!");
        }
        r
    }
}

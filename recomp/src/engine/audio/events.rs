//! Sound-event ids: bank<<30 | type bits | index (EFTSCTable/EFSETTable ids).
pub fn bank_of(id: u32) -> u32 { id >> 30 }
pub fn index_of(id: u32) -> u32 { id & 0x0fff_ffff }
pub fn is_container(id: u32) -> bool { id & 0x2000_0000 != 0 }

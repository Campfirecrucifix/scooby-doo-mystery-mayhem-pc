//! EFCollisionPrimAnim (class 0x26, "EF_COLLISION_PRIM_ANIM"; Classes/EF/EFCollisionPrimAnim.cpp):
//! a list of timed commands animating collision primitives (EFCPAnimCmdPair, 0xc bytes:
//! {f32 time, u16 prim tag, u16 command, void* payload}).
#![allow(non_snake_case)]

use crate::engine::error::EngineError;

/// EFCPAnimCmdPair. `payload` = the MMMalloc'd command block as u32 words in STRUCT order
/// (the stream order differs per command, see LoadPropertiesBinary).
#[derive(Debug, Clone, PartialEq)]
pub struct EFCPAnimCmdPair {
    /// +0
    pub time: f32,
    /// +4
    pub tag: u16,
    /// +6
    pub cmd: u16,
    /// +8 (None = NULL)
    pub payload: Option<Vec<u32>>,
}

#[derive(Debug, Clone, Default)]
pub struct EFCollisionPrimAnim {
    pub uid: u32,
    /// this+0xc command count (short)
    pub count: i16,
    /// this+0x10 DTVector<EFCPAnimCmdPair>
    pub cmds: Vec<EFCPAnimCmdPair>,
    /// this+0x24
    pub f24: u32,
}

/// AddCommand payload size per command (0 = no payload).
pub fn CommandSize(cmd: u16) -> usize {
    match cmd {
        0 | 1 => 8,
        2 => 0x24,
        3 | 5 | 7 => 0x1c,
        4 | 6 => 0xc,
        8 => 0x10,
        _ => 0,
    }
}

/// Stream order of the payload words (struct word index per stream read) for LoadPropertiesBinary.
fn read_order(cmd: u16) -> &'static [usize] {
    match cmd {
        0 | 1 => &[0, 1],
        2 => &[1, 2, 3, 4, 0, 5, 6, 7, 8],
        3 | 5 | 7 => &[1, 2, 3, 0, 4, 5, 6],
        4 | 6 => &[1, 0, 2],
        8 => &[0, 1, 2, 3],
        _ => &[],
    }
}

impl EFCollisionPrimAnim {
    pub fn GetClassID(&self) -> u32 {
        0x26
    }
    pub fn GetClassName(&self) -> &'static str {
        "EF_COLLISION_PRIM_ANIM"
    }

    /// Pack entry: u32 UID (DTStreamable prefix) then LoadPropertiesBinary. Returns bytes consumed.
    pub fn LoadFromRaw(d: &[u8]) -> Result<(Self, usize), EngineError> {
        let mut a = EFCollisionPrimAnim::default();
        let uid = d.get(0..4).ok_or_else(|| EngineError::Format("prim anim uid".into()))?;
        a.uid = u32::from_le_bytes(uid.try_into().unwrap());
        let n = a.LoadPropertiesBinary(&d[4..])?;
        Ok((a, 4 + n))
    }

    /// LoadPropertiesBinary: ClearCommands; short count; per command {f32 time, u16 tag, u16 cmd,
    /// payload}. Error -5 (short stream) maps to Err. Returns bytes consumed.
    pub fn LoadPropertiesBinary(&mut self, d: &[u8]) -> Result<usize, EngineError> {
        self.ClearCommands();
        let mut p = 0usize;
        let mut rd = |n: usize| -> Result<&[u8], EngineError> {
            let s = d.get(p..p + n).ok_or_else(|| EngineError::Format("prim anim truncated".into()))?;
            p += n;
            Ok(s)
        };
        // quirk: a failing count read returns 0 (not the error) in the C++
        self.count = i16::from_le_bytes(rd(2)?.try_into().unwrap());
        for _ in 0..self.count.max(0) {
            let time = f32::from_le_bytes(rd(4)?.try_into().unwrap());
            let tag = u16::from_le_bytes(rd(2)?.try_into().unwrap());
            let cmd = u16::from_le_bytes(rd(2)?.try_into().unwrap());
            let payload = if CommandSize(cmd) == 0 {
                None // cmd 9 and unknown: NULL payload
            } else {
                let mut w = vec![0u32; CommandSize(cmd) / 4];
                for &i in read_order(cmd) {
                    w[i] = u32::from_le_bytes(rd(4)?.try_into().unwrap());
                }
                Some(w)
            };
            self.cmds.push(EFCPAnimCmdPair { time, tag, cmd, payload });
        }
        Ok(p)
    }

    /// AddCommand(time, tag, cmd, data, vector): payload copied (CommandSize bytes).
    pub fn AddCommand(time: f32, tag: u16, cmd: u16, data: &[u32], v: &mut Vec<EFCPAnimCmdPair>) {
        let n = CommandSize(cmd) / 4;
        let payload = if n != 0 { Some(data[..n].to_vec()) } else { None };
        v.push(EFCPAnimCmdPair { time, tag, cmd, payload });
    }
    /// AddAnimCommand: AddCommand into this+0x10, count++.
    pub fn AddAnimCommand(&mut self, time: f32, tag: u16, cmd: u16, data: &[u32]) {
        Self::AddCommand(time, tag, cmd, data, &mut self.cmds);
        self.count += 1;
    }
    /// AddInitialAnimCommand: empty in the original.
    pub fn AddInitialAnimCommand(&mut self, _tag: u16, _cmd: u16, _data: &[u32]) {}
    /// ClearCommands: free payloads, empty the vector, count = 0.
    pub fn ClearCommands(&mut self) {
        self.cmds.clear();
        self.count = 0;
    }
    /// Update / SetNextKeyframe / GetInitialCommand: empty stubs returning 0 in the original.
    pub fn Update(&mut self, _dt: f32) -> u32 {
        0
    }
    /// GetCommand(uint, float*) (0x001cc330): 8-byte stub returning NULL.
    pub fn GetCommand(&self, _i: usize, _t: &mut f32) -> Option<&EFCPAnimCmdPair> {
        None
    }
}

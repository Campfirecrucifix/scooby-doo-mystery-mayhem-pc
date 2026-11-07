//! Animation resources: DTAnimHdr / DTChannelSet / DTChannel / DTFCurve / DTFloatComp (the
//! engine's own curve animation format, NOT RenderWare RtAnim), EFAnimRes (0x49), EFHAnimRes
//! (0x4f), EFCollisionPrimAnim + EFCPAnimCmdPair (stream part needed by EFHAnimRes),
//! EFTextureAnimRes (0x2c), EFMorphAnimRes (0x13).
//!
//! Memory model: the C++ keeps the anim blob as one MMMalloc'd block and patches the channel
//! data offsets into pointers (DTAnimHdr::ResolvePointers). The logic modules never see the blob,
//! so it is parsed here into native structs; offsets are kept in doc comments.
#![allow(non_snake_case)]

use crate::engine::dt::property::{DTObject, DTProperty};
use crate::engine::dt::stream::DTBasePersistStream;

fn rd_u32(b: &[u8], o: usize) -> Option<u32> {
    b.get(o..o + 4).map(|x| u32::from_le_bytes(x.try_into().unwrap()))
}
fn rd_f32(b: &[u8], o: usize) -> Option<f32> {
    rd_u32(b, o).map(f32::from_bits)
}
fn rd_u16(b: &[u8], o: usize) -> Option<u16> {
    b.get(o..o + 2).map(|x| u16::from_le_bytes(x.try_into().unwrap()))
}

/// DTFloatComp::Decompress1075 (0x00228d30): two 16-bit "1.5.10" floats packed in a u32
/// (sign bit 15, 5-bit exponent rebased by +107, 10-bit mantissa); low half -> a, high half -> b.
pub fn Decompress1075(v: u32) -> (f32, f32) {
    let mut e1 = (v & 0x7c00) << 13;
    if v & 0x7c00 != 0 {
        e1 += 0x3580_0000;
    }
    let a = ((v & 0x8000) << 16) | e1 | ((v & 0x3ff) << 13);
    let mut e2 = (v & 0x7c00_0000) >> 3;
    if e2 != 0 {
        e2 += 0x3580_0000;
    }
    let b = (v & 0x8000_0000) | e2 | ((v & 0x03ff_0000) >> 3);
    (f32::from_bits(a), f32::from_bits(b))
}

/// DTFCurve segments (this+0xc), by curve type (this+9).
#[derive(Debug, Clone, PartialEq)]
pub enum DTFCurveData {
    /// type 1: constant value at +0xc
    Const(f32),
    /// type 2 (and any other non-1/non-4 type, see UpdateHAnim): 0x14-byte segments
    /// {f32 endTime, a, b, c, d}, value = d + t*(c + t*(b + t*a)), t in [0,1] within the segment.
    Cubic(Vec<[f32; 5]>),
    /// type 4: 0xc-byte segments {f32 endTime, u32 packed(a,b), u32 packed(c,d)} (Decompress1075).
    Comp(Vec<(f32, u32, u32)>),
}

/// DTFCurve (variable size, inside the anim blob).
#[derive(Debug, Clone, PartialEq)]
pub struct DTFCurve {
    /// this+0x0
    pub start_time: f32,
    /// this+0x4 GetEndTime
    pub end_time: f32,
    /// this+0x8: high nibble = pre-extrapolation, low nibble = post (0 clamp, 5 mirror, else cycle)
    pub extrap: u8,
    /// this+0x9 GetCurveType
    pub curve_type: u8,
    /// this+0xa GetNbSegment
    pub nb_segment: u16,
    pub data: DTFCurveData,
}

impl DTFCurve {
    pub fn parse(b: &[u8], o: usize) -> Option<DTFCurve> {
        let start_time = rd_f32(b, o)?;
        let end_time = rd_f32(b, o + 4)?;
        let extrap = *b.get(o + 8)?;
        let curve_type = *b.get(o + 9)?;
        let nb_segment = rd_u16(b, o + 10)?;
        let n = nb_segment as usize;
        let data = match curve_type {
            1 => DTFCurveData::Const(rd_f32(b, o + 0xc)?),
            4 => {
                let mut v = Vec::with_capacity(n);
                for i in 0..n {
                    let p = o + 0xc + i * 0xc;
                    v.push((rd_f32(b, p)?, rd_u32(b, p + 4)?, rd_u32(b, p + 8)?));
                }
                DTFCurveData::Comp(v)
            }
            _ => {
                let mut v = Vec::with_capacity(n);
                for i in 0..n {
                    let p = o + 0xc + i * 0x14;
                    v.push([rd_f32(b, p)?, rd_f32(b, p + 4)?, rd_f32(b, p + 8)?, rd_f32(b, p + 12)?, rd_f32(b, p + 16)?]);
                }
                DTFCurveData::Cubic(v)
            }
        };
        Some(DTFCurve { start_time, end_time, extrap, curve_type, nb_segment, data })
    }

    /// DTFCurve::FindSegmentTime (0x00228e30): maps t into [start,end] with the extrapolation
    /// modes. quirk: the C++ short-circuit chain returns start/end (clamp) only when the matching
    /// nibble is 0; a non-zero nibble cycles (5 = ping-pong on odd periods).
    pub fn FindSegmentTime(&self, t: f32) -> f32 {
        let start = self.start_time;
        let end = self.end_time;
        let pre = (self.extrap >> 4) as u32;
        let post = (self.extrap & 0xf) as u32;
        if !(t < start || end < t) {
            return t;
        }
        if !(pre != 0 || start <= t) {
            return start;
        }
        if !(post != 0 || t <= end) {
            return end;
        }
        let f1 = t - start;
        let len = end - start;
        let mut u = (f1 / len) as i32 as u32;
        let mut f4 = f1 - (u as i32) as f32 * len;
        if f1.to_bits() & 0x8000_0000 == 0 {
            u = u.wrapping_add(1);
        } else {
            f4 += len;
        }
        if u & 1 == 0 && ((t < start && pre == 5) || (end < t && post == 5)) {
            f4 = len - f4;
        }
        start + f4
    }

    /// Finds the segment containing `t` (first with endTime >= t) and converts t to [0,1].
    /// FindSegmentAndConvertTime / FindSegmentCompAndConvertTime. Returns the segment index.
    /// Deviation: the C++ walks past the last segment when t > last endTime (never happens after
    /// FindSegmentTime unless NaN); here the walk stops at the last segment.
    fn find_seg(&self, ends: impl Iterator<Item = f32>, t: &mut f32) -> usize {
        let ends: Vec<f32> = ends.collect();
        let mut i = 0;
        while i + 1 < ends.len() && ends[i] < *t {
            i += 1;
        }
        let prev = if i == 0 { self.start_time } else { ends[i - 1] };
        *t = (*t - prev) / (ends[i] - prev);
        i
    }

    /// Evaluates the curve at absolute time `time` exactly like EFHAnimNode::UpdateHAnim.
    pub fn Evaluate(&self, time: f32) -> f32 {
        match &self.data {
            DTFCurveData::Const(v) => *v,
            DTFCurveData::Comp(s) => {
                if s.is_empty() {
                    return 0.0;
                }
                let mut t = self.FindSegmentTime(time);
                let i = self.find_seg(s.iter().map(|x| x.0), &mut t);
                let (f8, fc) = Decompress1075(s[i].1);
                let (f10, f14) = Decompress1075(s[i].2);
                f14 + t * (f10 + t * (fc + f8 * t))
            }
            DTFCurveData::Cubic(s) => {
                if s.is_empty() {
                    return 0.0;
                }
                let mut t = self.FindSegmentTime(time);
                let i = self.find_seg(s.iter().map(|x| x[0]), &mut t);
                let g = &s[i];
                g[4] + t * (g[3] + t * (g[2] + g[1] * t))
            }
        }
    }
}

/// DTChannel {u32 id, DTChannelData data}: id low 24 bits = target (0..8 DTTransfo float index:
/// 0-2 scale, 3-5 euler rotation, 6-8 translation; 9 = visibility), data = blob offset of the
/// DTFCurve (pointer after ResolvePointers).
#[derive(Debug, Clone, PartialEq)]
pub struct DTChannel {
    pub id: u32,
    pub offset: u32,
    pub curve: DTFCurve,
}

/// DTChannelSet {u32 bone (index, or RpHAnim ID when &0xffff0000), u16 nbChannels (+4),
/// u16 flags (+6, bit0 = scale compensate), DTChannel[nb] (+8)}. Size 8 + 8*nb.
#[derive(Debug, Clone, PartialEq)]
pub struct DTChannelSet {
    pub bone: u32,
    pub flags: u16,
    pub channels: Vec<DTChannel>,
}

/// DTAnimHdr (the blob EFAnimRes::LoadAnim reads).
#[derive(Debug, Clone, PartialEq)]
pub struct DTAnimHdr {
    /// +0x0 total blob size (includes this u32)
    pub size: u32,
    /// +0x4 (unknown; 0x1000547f/0x10005493 on real data)
    pub unk4: u32,
    /// +0x8 duration (EFHAnimNode::Initialize copies it to node+0x3c)
    pub duration: f32,
    /// +0xc u8 nb channel sets (bones)
    pub nb_sets: u8,
    /// +0xd..+0xe
    pub unkd: [u8; 2],
    /// +0xf: 1 -> bone 0 gets DTTransfo state bit 8 (UpdateHAnim)
    pub flag_f: u8,
    pub sets: Vec<DTChannelSet>,
    /// the raw blob (for consumers that need the original bytes)
    pub raw: Vec<u8>,
}

impl DTAnimHdr {
    /// Parse + DTAnimHdr::ResolvePointers (0x00110460): walk channel sets via GetNextChannelSet,
    /// resolve every channel's curve offset (relative to the blob start).
    pub fn ResolvePointers(raw: Vec<u8>) -> Option<DTAnimHdr> {
        let b = &raw;
        let nb_sets = *b.get(0xc)?;
        let mut sets = Vec::with_capacity(nb_sets as usize);
        let mut p = 0x10;
        for _ in 0..nb_sets {
            let bone = rd_u32(b, p)?;
            let n = rd_u16(b, p + 4)? as usize;
            let flags = rd_u16(b, p + 6)?;
            let mut channels = Vec::with_capacity(n);
            for c in 0..n {
                let q = p + 8 + c * 8;
                let id = rd_u32(b, q)?;
                let offset = rd_u32(b, q + 4)?;
                channels.push(DTChannel { id, offset, curve: DTFCurve::parse(b, offset as usize)? });
            }
            sets.push(DTChannelSet { bone, flags, channels });
            // quirk: GetNextChannelSet = this + (n-1)*8 + 0x10 (n == 0 would step back 0 bytes... +8)
            p += ((n as isize - 1) * 8 + 0x10) as usize;
        }
        Some(DTAnimHdr {
            size: rd_u32(b, 0)?,
            unk4: rd_u32(b, 4)?,
            duration: rd_f32(b, 8)?,
            nb_sets,
            unkd: [b[0xd], b[0xe]],
            flag_f: b[0xf],
            sets,
            raw,
        })
    }
}

/// DTTransfo (0x48 bytes). Only the euler part is written by animation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DTTransfo {
    /// +0x00 scale, +0x0c translation, +0x18 euler rotation (radians) (UpdateHAnim writes f[ch]).
    /// Layout proven by EFAnimBlendNode::Update: EulerToQuatf(+0x24, +0x18), +0xc transformed as a point.
    pub f: [f32; 9],
    /// +0x24 DTQuaternion
    pub quat: [f32; 4],
    /// +0x34 DTVector3d
    pub v34: [f32; 3],
    /// +0x40 state bits: 1 = euler valid, 2 cleared on write, 4 scale compensate, 8 root
    pub state: u32,
    /// +0x44 bone/channel-set id (-1 by default)
    pub id: i32,
}

impl Default for DTTransfo {
    /// DTTransfo::DTTransfo (scale (1,1,1), rest 0, id -1)
    fn default() -> Self {
        DTTransfo { f: [1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], quat: [0.0; 4], v34: [0.0; 3], state: 0, id: -1 }
    }
}

impl DTTransfo {
    /// DTTransfo::ResetEuler (single element)
    pub fn ResetEuler(&mut self) {
        self.f = [1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        self.state = 1;
    }
    /// DTQuaternion::EulerToQuatf(euler @+0x18) -> (x,y,z,w) (formula verified 1:1 against
    /// DTQuaternion.cpp; = R = Rz*Ry*Rx). Returns `quat` when state bit 1 (euler valid) is clear.
    pub fn rotation_quat(&self) -> [f32; 4] {
        if self.state & 1 == 0 && self.quat != [0.0; 4] {
            return self.quat;
        }
        Self::EulerToQuatf([self.f[6], self.f[7], self.f[8]])
    }
    /// DTQuaternion::EulerToQuatf
    pub fn EulerToQuatf(e: [f32; 3]) -> [f32; 4] {
        let (sx, cx) = (e[0] * 0.5).sin_cos();
        let (sy, cy) = (e[1] * 0.5).sin_cos();
        let (sz, cz) = (e[2] * 0.5).sin_cos();
        [
            sx * cy * cz - cx * sy * sz,
            cx * sy * cz + sx * cy * sz,
            cx * cy * sz - sx * sy * cz,
            cx * cy * cz + sx * sy * sz,
        ]
    }
    pub fn translation(&self) -> [f32; 3] {
        [self.f[3], self.f[4], self.f[5]]
    }
}

/// EFRessource common part: +0x8 uid, the second u32 is the resource "type/owner" passed to
/// EFRessourceFactory::RegisterRessource(uid, arg, this).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EFRessourceHdr {
    /// this+0x8
    pub uid: u32,
    /// RegisterRessource 2nd arg
    pub reg_arg: u32,
}

fn load_res_hdr(s: &mut dyn DTBasePersistStream, h: &mut EFRessourceHdr) -> i32 {
    let r = s.LoadData_uint(&mut h.uid);
    if r != 0 {
        return r;
    }
    s.LoadData_uint(&mut h.reg_arg)
}

/// EFAnimRes (class 0x49, size 0x14).
#[derive(Debug, Clone, Default)]
pub struct EFAnimRes {
    pub hdr: EFRessourceHdr,
    /// this+0x10 the DTAnimHdr blob
    pub anim: Option<DTAnimHdr>,
}

impl EFAnimRes {
    pub const CLASS_ID: u32 = 0x49;
    /// EFAnimRes::LoadAnim (0x0022d3d0): i32 size, then size-4 bytes, ResolvePointers.
    pub fn LoadAnim(&mut self, s: &mut dyn DTBasePersistStream) -> i32 {
        let mut b = [0u8; 4];
        let r = s.LoadBinaryData(&mut b);
        if r != 0 {
            return r;
        }
        let size = i32::from_le_bytes(b).max(4) as usize;
        let mut raw = vec![0u8; size];
        raw[..4].copy_from_slice(&b);
        let r = s.LoadBinaryData(&mut raw[4..]);
        if r != 0 {
            return r;
        }
        self.anim = DTAnimHdr::ResolvePointers(raw);
        0
    }
    pub fn duration(&self) -> f32 {
        self.anim.as_ref().map_or(0.0, |a| a.duration)
    }
}

impl DTObject for EFAnimRes {
    fn GetClassID(&self) -> u32 {
        Self::CLASS_ID
    }
    fn GetClassName(&self) -> &'static str {
        "EF_ANIM_RES"
    }
    fn Members(&mut self) -> Vec<DTProperty<'_>> {
        Vec::new()
    }
    /// LoadPropertiesBinary (0x0022d350). RegisterRessource is done by the owner of the object
    /// (EFRessourceFactory, coordinator) using `hdr`.
    fn LoadPropertiesBinary(&mut self, s: &mut dyn DTBasePersistStream) -> i32 {
        let r = load_res_hdr(s, &mut self.hdr);
        if r != 0 {
            return r;
        }
        self.LoadAnim(s)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// EFCPAnimCmdPair (0xc): {f32 time, u16 a, u16 cmd type, void* data}.
#[derive(Debug, Clone, PartialEq)]
pub struct EFCPAnimCmdPair {
    pub time: f32,
    pub a: u16,
    pub cmd: u16,
    /// command payload as floats/ints in original slot order (slot 0 = data[0])
    pub data: Option<Vec<u32>>,
}

/// EFCollisionPrimAnim (0x28): collision primitive keyframe commands of an EFHAnimRes.
/// The stream part is here because EFHAnimRes::LoadCollPrim needs it; the collision runtime
/// using it belongs to WP-E.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EFCollisionPrimAnim {
    /// this+0xc
    pub nb_cmds: i16,
    /// this+0x10 DTVector<EFCPAnimCmdPair>
    pub cmds: Vec<EFCPAnimCmdPair>,
}

impl EFCollisionPrimAnim {
    /// LoadPropertiesBinary (0x001cb1a0). Payload slot read orders kept from the C++
    /// (e.g. type 2 reads slots 1,2,3,4,0,5,6,7,8).
    pub fn LoadPropertiesBinary(&mut self, s: &mut dyn DTBasePersistStream) -> i32 {
        self.cmds.clear();
        if s.LoadData_short(&mut self.nb_cmds) != 0 {
            return 0;
        }
        for _ in 0..self.nb_cmds.max(0) {
            let mut time = 0f32;
            let (mut a, mut cmd) = (0u16, 0u16);
            let r = s.LoadData_float(&mut time);
            if r != 0 {
                return r;
            }
            let r = s.LoadData_ushort(&mut a);
            if r != 0 {
                return r;
            }
            let r = s.LoadData_ushort(&mut cmd);
            if r != 0 {
                return r;
            }
            // (slots, read order, int?) per command type
            let (n, order, ints): (usize, &[usize], usize) = match cmd {
                0 => (2, &[0, 1], 2),
                1 => (2, &[0, 1], 1),
                2 => (9, &[1, 2, 3, 4, 0, 5, 6, 7, 8], 0),
                3 | 5 | 7 => (7, &[1, 2, 3, 0, 4, 5, 6], 0),
                4 | 6 => (3, &[1, 0, 2], 0),
                8 => (4, &[0, 1, 2, 3], 0),
                _ => (0, &[], 0),
            };
            let data = if n == 0 {
                None
            } else {
                let mut d = vec![0u32; n];
                for (k, &slot) in order.iter().enumerate() {
                    let r = if k < ints {
                        let mut v = 0i32;
                        let r = s.LoadData_int(&mut v);
                        d[slot] = v as u32;
                        r
                    } else {
                        let mut v = 0f32;
                        let r = s.LoadData_float(&mut v);
                        d[slot] = v.to_bits();
                        r
                    };
                    if r != 0 {
                        return r;
                    }
                }
                Some(d)
            };
            self.cmds.push(EFCPAnimCmdPair { time, a, cmd, data });
        }
        0
    }
}

/// EFHAnimRes (class 0x4f, size 0x28): EFAnimRes + collision prim anim + anim callbacks.
#[derive(Debug, Clone, Default)]
pub struct EFHAnimRes {
    pub base: EFAnimRes,
    /// this+0x14
    pub coll_prim: Option<EFCollisionPrimAnim>,
    /// this+0x18 (unknown header word of the callback block)
    pub cb_hdr0: u32,
    /// this+0x1c callback count
    pub cb_count: u32,
    /// this+0x20 callback element size
    pub cb_elem_size: u32,
    /// this+0x24 callback records (count * elem_size bytes). Read by EFHAnimNode as
    /// {f32 time, u32 sound event id, u32 ...} (see nodes.rs).
    pub callbacks: Vec<u8>,
}

impl EFHAnimRes {
    pub const CLASS_ID: u32 = 0x4f;
    /// LoadCollPrim (0x00236870): new EFCollisionPrimAnim, LoadFromStream (binary -> Props).
    pub fn LoadCollPrim(&mut self, s: &mut dyn DTBasePersistStream) -> i32 {
        let mut c = EFCollisionPrimAnim::default();
        let r = c.LoadPropertiesBinary(s);
        self.coll_prim = Some(c);
        r
    }
    /// LoadCallback (0x002368e0). quirk: allocates count*0xc but reads count*elem_size bytes.
    pub fn LoadCallback(&mut self, s: &mut dyn DTBasePersistStream) -> i32 {
        let mut b = [0u8; 12];
        let r = s.LoadBinaryData(&mut b);
        self.cb_hdr0 = u32::from_le_bytes(b[0..4].try_into().unwrap());
        self.cb_count = u32::from_le_bytes(b[4..8].try_into().unwrap());
        self.cb_elem_size = u32::from_le_bytes(b[8..12].try_into().unwrap());
        if r != 0 {
            return r;
        }
        if self.cb_count != 0 {
            self.callbacks = vec![0u8; (self.cb_count as usize).saturating_mul(self.cb_elem_size as usize)];
            return s.LoadBinaryData(&mut self.callbacks);
        }
        0
    }
    /// callback record i as u32 words (elem_size/4 words)
    pub fn callback_words(&self, i: usize) -> Vec<u32> {
        let es = self.cb_elem_size as usize;
        self.callbacks
            .get(i * es..(i + 1) * es)
            .map(|c| c.chunks_exact(4).map(|w| u32::from_le_bytes(w.try_into().unwrap())).collect())
            .unwrap_or_default()
    }
    pub fn duration(&self) -> f32 {
        self.base.duration()
    }
}

impl DTObject for EFHAnimRes {
    fn GetClassID(&self) -> u32 {
        Self::CLASS_ID
    }
    fn GetClassName(&self) -> &'static str {
        "EF_HANIM_RES"
    }
    fn Members(&mut self) -> Vec<DTProperty<'_>> {
        Vec::new()
    }
    /// LoadPropertiesBinary (0x002367d0): uid, arg, LoadAnim, LoadCollPrim, LoadCallback.
    /// quirk: the C++ ignores every return code here.
    fn LoadPropertiesBinary(&mut self, s: &mut dyn DTBasePersistStream) -> i32 {
        load_res_hdr(s, &mut self.base.hdr);
        self.base.LoadAnim(s);
        self.LoadCollPrim(s);
        self.LoadCallback(s);
        0
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// EFTextureAnimRes (class 0x2c, size 0x18): list of {RwTexture*, f32 time}.
/// Replaced: RwTexDictionaryFindNamedTexture -> the texture NAME is kept; resolution against the
/// current TXD (and the refcount++ at texture+0x54) is done by the renderer (WP-C).
#[derive(Debug, Clone, Default)]
pub struct EFTextureAnimRes {
    pub hdr: EFRessourceHdr,
    /// this+0x10 count, this+0x14 array of 8-byte {texture (+0), f32 time (+4)}
    pub frames: Vec<(String, f32)>,
}

impl EFTextureAnimRes {
    pub const CLASS_ID: u32 = 0x2c;
    /// LoadTextAnim (0x0023abc0). quirk: on stream type 1 an extra char is read before the name.
    pub fn LoadTextAnim(&mut self, s: &mut dyn DTBasePersistStream) -> i32 {
        let mut n = 0u32;
        s.LoadData_uint(&mut n);
        self.frames.clear();
        for _ in 0..n {
            let mut t = 0f32;
            s.LoadData_float(&mut t);
            if s.GetStreamType() == 1 {
                let mut c = 0i8;
                s.LoadData_char(&mut c);
            }
            let mut name = Vec::new();
            s.LoadTextData(&mut name, 0x800);
            let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
            self.frames.push((String::from_utf8_lossy(&name[..end]).into_owned(), t));
        }
        0
    }
}

impl DTObject for EFTextureAnimRes {
    fn GetClassID(&self) -> u32 {
        Self::CLASS_ID
    }
    fn GetClassName(&self) -> &'static str {
        "EI_TEXTURE_ANIM_RES"
    }
    fn Members(&mut self) -> Vec<DTProperty<'_>> {
        Vec::new()
    }
    fn LoadPropertiesBinary(&mut self, s: &mut dyn DTBasePersistStream) -> i32 {
        load_res_hdr(s, &mut self.hdr);
        self.LoadTextAnim(s)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// EFMorphAnimRes (class 0x13, size 0x14). The ApMorphAnimationStreamRead (RenderWare
/// plugin) chunk {u32 type, u32 size, u32 version, data} is kept raw and decoded by anim/morph.rs.
/// Not present in EP1L01.
#[derive(Debug, Clone, Default)]
pub struct EFMorphAnimRes {
    pub hdr: EFRessourceHdr,
    /// this+0x10 ApMorphAnimationEx* (raw RW chunk here)
    pub rw_chunk: Vec<u8>,
}

impl EFMorphAnimRes {
    pub const CLASS_ID: u32 = 0x13;
}

impl DTObject for EFMorphAnimRes {
    fn GetClassID(&self) -> u32 {
        Self::CLASS_ID
    }
    fn GetClassName(&self) -> &'static str {
        "MORPH_ANIM_INST"
    }
    fn Members(&mut self) -> Vec<DTProperty<'_>> {
        Vec::new()
    }
    fn LoadPropertiesBinary(&mut self, s: &mut dyn DTBasePersistStream) -> i32 {
        load_res_hdr(s, &mut self.hdr);
        let mut h = [0u8; 12];
        let r = s.LoadBinaryData(&mut h);
        if r != 0 {
            return r;
        }
        let size = u32::from_le_bytes(h[4..8].try_into().unwrap()) as usize;
        self.rw_chunk = h.to_vec();
        self.rw_chunk.resize(12 + size, 0);
        s.LoadBinaryData(&mut self.rw_chunk[12..])
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

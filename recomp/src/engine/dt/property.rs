//! DTVariant / DTProperty / DTStreamable member protocol (Classes/DT/DTVariant.cpp,
//! DTProperty.cpp, DTStreamable.cpp). Facts in ENGINE_PORT.md "DTVariant/DTProperty binary wire
//! protocol" and "DTStreamable".
//!
//! The C++ registers (name, DTVariant{pointer, type code}) pairs in the constructor and walks
//! them in registration order. Here a class returns the same list on demand from
//! [`DTObject::Members`] as borrowed [`DTVariant`]s (same order, same names, same type codes),
//! and the generic DTStreamable functions below walk it exactly like the C++.

#![allow(non_snake_case)]

use super::stream::*;

/// Borrowed DTVariant. The doc comment of each case gives the C++ type code (DTVariant+8 & 0x7f).
pub enum DTVariant<'a> {
    /// 0x42 float*
    Float(&'a mut f32),
    /// 0x45 int*
    Int(&'a mut i32),
    /// 0x48 uint*
    Uint(&'a mut u32),
    /// 0x49 DTString*
    String(&'a mut String),
    /// 0x4a DTStreamable* (nested object, loaded with its own Load, vtbl 0x14)
    Streamable(&'a mut dyn DTObject),
    /// 0x4c DTDummyVector3
    Vec3(&'a mut [f32; 3]),
    /// 0x4d DTDummyVector4
    Vec4(&'a mut [f32; 4]),
    /// 0x64 DTVector<float>
    VecFloat(&'a mut Vec<f32>),
    /// 0x65 DTVector<int>
    VecInt(&'a mut Vec<i32>),
    /// 0x67 DTVector<DTString>
    VecString(&'a mut Vec<String>),
    /// 0x50 data bloc in place (void*, size): the buffer is resized to the size read
    DataBloc(&'a mut Vec<u8>),
}

impl DTVariant<'_> {
    pub fn TypeCode(&self) -> u8 {
        match self {
            DTVariant::Float(_) => 0x42,
            DTVariant::Int(_) => 0x45,
            DTVariant::Uint(_) => 0x48,
            DTVariant::String(_) => 0x49,
            DTVariant::Streamable(_) => 0x4a,
            DTVariant::Vec3(_) => 0x4c,
            DTVariant::Vec4(_) => 0x4d,
            DTVariant::VecFloat(_) => 0x64,
            DTVariant::VecInt(_) => 0x65,
            DTVariant::VecString(_) => 0x67,
            DTVariant::DataBloc(_) => 0x50,
        }
    }
}

/// A registered member: DTProperty(name, variant).
pub struct DTProperty<'a> {
    pub name: &'static str,
    pub var: DTVariant<'a>,
}

pub fn prop<'a>(name: &'static str, var: DTVariant<'a>) -> DTProperty<'a> {
    DTProperty { name, var }
}

fn is_text(s: &dyn DTBasePersistStream) -> bool {
    s.GetStreamType() & STREAM_FLAG_TEXT != 0
}

/// DTProperty::LoadString: LoadTextData(buf, 0x800). Text streams: the string is between quote
/// characters, else -4.
fn LoadString(s: &mut dyn DTBasePersistStream, out: &mut String) -> i32 {
    let mut buf = Vec::new();
    let r = s.LoadTextData(&mut buf, 0x800);
    if r != 0 {
        return r;
    }
    if is_text(s) {
        let t = String::from_utf8_lossy(&buf).trim().to_string();
        if t.len() < 2 || !t.starts_with('"') || !t.ends_with('"') {
            return ERR_QUOTE;
        }
        *out = t[1..t.len() - 1].to_string();
    } else {
        // binary: no NUL guarantee; the C string ends at the first NUL if any
        let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        *out = String::from_utf8_lossy(&buf[..end]).into_owned();
    }
    0
}

/// DTProperty::LoadVector: n x LoadData(float) (text: SkipWhiteSpace around is in the text stream's
/// own LoadData_float).
fn LoadVector(s: &mut dyn DTBasePersistStream, v: &mut [f32]) -> i32 {
    for x in v.iter_mut() {
        let r = s.LoadData_float(x);
        if r != 0 {
            return r;
        }
    }
    0
}

/// DTProperty::LoadDataBloc: LoadData(int size); text streams read one extra char then decode
/// letter pairs (byte = c2 + (c1-'a')*8 - 'a', one extra pair every 0xfa bytes); binary streams
/// LoadBinaryData(buf, size).
fn LoadDataBloc(s: &mut dyn DTBasePersistStream, buf: &mut Vec<u8>) -> i32 {
    let mut size = 0i32;
    let r = s.LoadData_int(&mut size);
    if r != 0 {
        return r;
    }
    if is_text(s) {
        let mut c = 0i8;
        let r = s.LoadData_char(&mut c);
        if r != 0 {
            return r;
        }
    }
    // quirk: the stored size is (size << 8) >> 8 in the property word, i.e. 24 bits
    let n = ((size as u32) & 0x00ff_ffff) as usize;
    buf.clear();
    buf.resize(n, 0);
    if s.GetStreamType() & STREAM_FLAG_BINARY_BLOCKS == 0 {
        let mut err = 0;
        for i in 0..size.max(0) as usize {
            let (mut c1, mut c2) = (0i8, 0i8);
            err |= s.LoadData_char(&mut c1) | s.LoadData_char(&mut c2);
            if i < n {
                buf[i] = (c2 as u8).wrapping_add((c1 as u8).wrapping_sub(b'a').wrapping_mul(8)).wrapping_sub(b'a');
            }
            if i > 0 && i % 0xfa == 0 {
                let (mut a, mut b) = (0i8, 0i8);
                err |= s.LoadData_char(&mut a) | s.LoadData_char(&mut b);
            }
        }
        err
    } else {
        s.LoadBinaryData(buf)
    }
}

/// DTProperty::LoadArray: LoadData(int count) then the elements.
fn LoadArray<T: Default + Clone>(
    s: &mut dyn DTBasePersistStream,
    v: &mut Vec<T>,
    mut el: impl FnMut(&mut dyn DTBasePersistStream, &mut T) -> i32,
) -> i32 {
    let mut n = 0i32;
    let r = s.LoadData_int(&mut n);
    if r != 0 {
        return r;
    }
    v.clear();
    v.resize(n.max(0) as usize, T::default());
    for x in v.iter_mut() {
        let r = el(s, x);
        if r != 0 {
            return r;
        }
    }
    0
}

impl DTProperty<'_> {
    /// DTProperty::LoadFromStream (0x00107fa0)
    pub fn LoadFromStream(&mut self, s: &mut dyn DTBasePersistStream) -> i32 {
        match &mut self.var {
            DTVariant::Float(v) => s.LoadData_float(v),
            DTVariant::Int(v) => s.LoadData_int(v),
            DTVariant::Uint(v) => s.LoadData_uint(v),
            DTVariant::String(v) => LoadString(s, v),
            DTVariant::Streamable(o) => o.Load(s),
            DTVariant::Vec3(v) => LoadVector(s, &mut v[..]),
            DTVariant::Vec4(v) => LoadVector(s, &mut v[..]),
            DTVariant::VecFloat(v) => LoadArray(s, v, |s, x| s.LoadData_float(x)),
            DTVariant::VecInt(v) => LoadArray(s, v, |s, x| s.LoadData_int(x)),
            DTVariant::VecString(v) => LoadArray(s, v, |s, x| {
                let mut b = Vec::new();
                let r = s.LoadTextData(&mut b, 0x800);
                *x = String::from_utf8_lossy(&b).into_owned();
                r
            }),
            DTVariant::DataBloc(b) => LoadDataBloc(s, b),
        }
    }
}

/// DTStreamable vtable (see ENGINE_PORT.md). Implementors give their registered members in
/// registration order (base class members first, "UID" first of all) and override the
/// virtuals they override in the C++.
pub trait DTObject: Send + Sync + std::any::Any {
    /// vtbl 0x08
    fn GetClassID(&self) -> u32;
    /// vtbl 0x0c
    fn GetClassName(&self) -> &'static str;
    /// the registered member list (DTStreamable this+0x4)
    fn Members(&mut self) -> Vec<DTProperty<'_>>;

    /// vtbl 0x14 Load: PrePersistStream, LoadFromStream unless -7, PostPersistStream.
    fn Load(&mut self, s: &mut dyn DTBasePersistStream) -> i32 {
        let r = self.PrePersistStream(s, true);
        let mut ret = 0;
        if r == 0 {
            ret = self.LoadFromStream(s);
        } else if r != ERR_SKIP {
            return r;
        }
        self.PostPersistStream(s, true);
        ret
    }
    /// vtbl 0x1c
    fn PrePersistStream(&mut self, _s: &mut dyn DTBasePersistStream, _load: bool) -> i32 {
        0
    }
    /// vtbl 0x20: if loading and no member of type 0x4f, OnLoadFinish. (No class of this engine
    /// registers a 0x4f member, so it always calls OnLoadFinish.)
    fn PostPersistStream(&mut self, _s: &mut dyn DTBasePersistStream, load: bool) -> i32 {
        if load {
            self.OnLoadFinish();
        }
        0
    }
    /// vtbl 0x24
    fn LoadFromStream(&mut self, s: &mut dyn DTBasePersistStream) -> i32 {
        DTStreamable_LoadFromStream(self, s)
    }
    /// vtbl 0x2c
    fn OnLoadFinish(&mut self) -> i32 {
        0
    }
    /// vtbl 0x30
    fn IsLoader(&self) -> bool {
        false
    }
    /// vtbl 0x3c
    fn LoadPropertiesText(&mut self, tok: &mut Vec<u8>, s: &mut dyn DTBasePersistStream) -> i32 {
        DTStreamable_LoadPropertiesText(self, tok, s)
    }
    /// vtbl 0x40
    fn LoadPropertiesBinary(&mut self, s: &mut dyn DTBasePersistStream) -> i32 {
        DTStreamable_LoadPropertiesBinary(self, s)
    }
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// DTStreamable::LoadFromStream: binary -> LoadPropertiesBinary; text -> read tokens until
/// "BEGIN", LoadPropertiesText, then one byte.
pub fn DTStreamable_LoadFromStream<T: DTObject + ?Sized>(o: &mut T, s: &mut dyn DTBasePersistStream) -> i32 {
    if !is_text(s) {
        return o.LoadPropertiesBinary(s);
    }
    let mut tok = Vec::new();
    loop {
        let r = s.LoadString(&mut tok, 0x800);
        if r != 0 {
            return r;
        }
        if tok == b"BEGIN" {
            break;
        }
    }
    let r = o.LoadPropertiesText(&mut tok, s);
    let mut b = [0u8; 1];
    s.LoadBinaryData(&mut b);
    r
}

/// DTStreamable::LoadPropertiesBinary: every member in registration order.
pub fn DTStreamable_LoadPropertiesBinary<T: DTObject + ?Sized>(o: &mut T, s: &mut dyn DTBasePersistStream) -> i32 {
    for mut p in o.Members() {
        let r = p.LoadFromStream(s);
        if r != 0 {
            return r;
        }
    }
    0
}

/// DTStreamable::LoadPropertiesText: token loop until "END"; FindMember(tok)->LoadFromStream.
/// quirk: unknown tokens are ignored (FindMember returns NULL and the C++ skips them).
pub fn DTStreamable_LoadPropertiesText<T: DTObject + ?Sized>(o: &mut T, tok: &mut Vec<u8>, s: &mut dyn DTBasePersistStream) -> i32 {
    loop {
        let r = s.LoadString(tok, 0x800);
        if r != 0 {
            return r;
        }
        if tok == b"END" {
            return 0;
        }
        let name = String::from_utf8_lossy(tok).into_owned();
        let mut members = o.Members();
        if let Some(p) = members.iter_mut().find(|p| p.name == name) {
            let r = p.LoadFromStream(s);
            if r != 0 {
                return r;
            }
        }
    }
}

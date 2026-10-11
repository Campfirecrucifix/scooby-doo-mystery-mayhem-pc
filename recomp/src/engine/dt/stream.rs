//! DT persist streams: DTBasePersistStream, DTBinaryPersistStream, DTMemoryPersistStream,
//! DTTextPersistStream (Classes/DT/DT*PersistStream.cpp).
//!
//! The vtable protocol is kept as a trait whose method names mirror the C++ overloads
//! (`LoadData(uint&)` -> `LoadData_uint`). Error codes are the original ones (i32):
//! 0 ok, -1 write failure, -2 ferror, -3 open failure / bad name, -4 bad string quoting,
//! -5 EOF / short read, -7 PrePersist "skip".
//!
//! Replacement (rule 2): the original file streams wrap a PS2 file handle (DTFopen/DTFread).
//! Here every stream wraps a byte buffer that was read in full at level-load time, so no stream
//! method ever blocks (Bevy rule 3). Observable behaviour is unchanged: reads are sequential,
//! a short read copies what is left and returns -5 (fread + feof), and save paths are the
//! original stubs (DTBasePersistStream::SaveBinaryData returns 0 without writing: binary saving
//! is not implemented in this engine build at all).

#![allow(non_snake_case)]

pub const ERR_OK: i32 = 0;
pub const ERR_WRITE: i32 = -1;
pub const ERR_READ: i32 = -2;
pub const ERR_OPEN: i32 = -3;
pub const ERR_QUOTE: i32 = -4;
pub const ERR_EOF: i32 = -5;
pub const ERR_SKIP: i32 = -7;

/// Stream type word (DTBasePersistStream this+0xc).
pub const STREAM_TEXT_FILE: u32 = 5;
pub const STREAM_BINARY_FILE: u32 = 6;
pub const STREAM_MEMORY: u32 = 10;
/// bit 0 = text, bit 1 = raw-binary data blocks, bit 0x10 = byteswap on save.
pub const STREAM_FLAG_TEXT: u32 = 1;
pub const STREAM_FLAG_BINARY_BLOCKS: u32 = 2;
pub const STREAM_FLAG_BYTESWAP: u32 = 0x10;

/// DTBasePersistStream vtable. Default LoadData_* are the binary implementations
/// (`LoadData(T&) = LoadBinaryData(&v, sizeof T)`, little endian like the PS2).
pub trait DTBasePersistStream: Send {
    /// GetStreamType (this+0xc)
    fn GetStreamType(&self) -> u32;
    /// vtbl 0x60
    fn LoadBinaryData(&mut self, buf: &mut [u8]) -> i32;
    /// vtbl 0x58. Base: returns 0, writes nothing (original stub).
    fn SaveBinaryData(&mut self, _buf: &[u8]) -> i32 {
        0
    }
    /// vtbl 0x5c. Binary: i32 length then min(len,max) bytes. `out` receives the bytes (no NUL
    /// added: the original gives no NUL guarantee either).
    /// quirk: when len > max the remaining len-max bytes are NOT consumed (stream desyncs).
    fn LoadTextData(&mut self, out: &mut Vec<u8>, max: usize) -> i32 {
        let mut len = 0i32;
        let r = self.LoadData_int(&mut len);
        if r != 0 {
            return r;
        }
        let n = (len.max(0) as usize).min(max);
        out.clear();
        out.resize(n, 0);
        self.LoadBinaryData(out)
    }
    /// vtbl 0x54. Binary streams: returns 0 (unimplemented in the original).
    fn SaveTextData(&mut self, _s: &[u8]) -> i32 {
        0
    }
    /// vtbl 0x68 (text streams only; the binary vtable slot is never called on binary streams).
    fn LoadString(&mut self, _out: &mut Vec<u8>, _max: usize) -> i32 {
        ERR_EOF
    }
    /// vtbl 0x6c (text streams only). Returns the first non-white byte read (consumed) in `c`.
    fn SkipWhiteSpace(&mut self, _c: &mut u8) -> i32 {
        0
    }
    /// vtbl 0x64 (text streams only)
    fn SaveTextIndentation(&mut self) -> i32 {
        0
    }

    fn LoadData_char(&mut self, v: &mut i8) -> i32 {
        let mut b = [0u8; 1];
        let r = self.LoadBinaryData(&mut b);
        *v = b[0] as i8;
        r
    }
    fn LoadData_schar(&mut self, v: &mut i8) -> i32 {
        self.LoadData_char(v)
    }
    fn LoadData_uchar(&mut self, v: &mut u8) -> i32 {
        let mut b = [0u8; 1];
        let r = self.LoadBinaryData(&mut b);
        *v = b[0];
        r
    }
    fn LoadData_short(&mut self, v: &mut i16) -> i32 {
        let mut b = [0u8; 2];
        let r = self.LoadBinaryData(&mut b);
        *v = i16::from_le_bytes(b);
        r
    }
    fn LoadData_ushort(&mut self, v: &mut u16) -> i32 {
        let mut b = [0u8; 2];
        let r = self.LoadBinaryData(&mut b);
        *v = u16::from_le_bytes(b);
        r
    }
    fn LoadData_int(&mut self, v: &mut i32) -> i32 {
        let mut b = [0u8; 4];
        let r = self.LoadBinaryData(&mut b);
        *v = i32::from_le_bytes(b);
        r
    }
    fn LoadData_uint(&mut self, v: &mut u32) -> i32 {
        let mut b = [0u8; 4];
        let r = self.LoadBinaryData(&mut b);
        *v = u32::from_le_bytes(b);
        r
    }
    /// `long` is 8 bytes on the PS2 EE (vtbl 0x1c).
    fn LoadData_long(&mut self, v: &mut i64) -> i32 {
        let mut b = [0u8; 8];
        let r = self.LoadBinaryData(&mut b);
        *v = i64::from_le_bytes(b);
        r
    }
    fn LoadData_float(&mut self, v: &mut f32) -> i32 {
        let mut b = [0u8; 4];
        let r = self.LoadBinaryData(&mut b);
        *v = f32::from_le_bytes(b);
        r
    }

    // SaveData: swap 2/4-byte values when STREAM_FLAG_BYTESWAP is set, then SaveBinaryData.
    fn SaveData_char(&mut self, v: i8) -> i32 {
        self.SaveBinaryData(&[v as u8])
    }
    fn SaveData_uchar(&mut self, v: u8) -> i32 {
        self.SaveBinaryData(&[v])
    }
    fn SaveData_short(&mut self, v: i16) -> i32 {
        let v = if self.GetStreamType() & STREAM_FLAG_BYTESWAP != 0 { v.swap_bytes() } else { v };
        self.SaveBinaryData(&v.to_le_bytes())
    }
    fn SaveData_ushort(&mut self, v: u16) -> i32 {
        let v = if self.GetStreamType() & STREAM_FLAG_BYTESWAP != 0 { v.swap_bytes() } else { v };
        self.SaveBinaryData(&v.to_le_bytes())
    }
    fn SaveData_int(&mut self, v: i32) -> i32 {
        let v = if self.GetStreamType() & STREAM_FLAG_BYTESWAP != 0 { v.swap_bytes() } else { v };
        self.SaveBinaryData(&v.to_le_bytes())
    }
    fn SaveData_uint(&mut self, v: u32) -> i32 {
        let v = if self.GetStreamType() & STREAM_FLAG_BYTESWAP != 0 { v.swap_bytes() } else { v };
        self.SaveBinaryData(&v.to_le_bytes())
    }
    fn SaveData_long(&mut self, v: i64) -> i32 {
        self.SaveBinaryData(&v.to_le_bytes())
    }
    fn SaveData_float(&mut self, v: f32) -> i32 {
        let b = if self.GetStreamType() & STREAM_FLAG_BYTESWAP != 0 { v.to_bits().swap_bytes() } else { v.to_bits() };
        self.SaveBinaryData(&b.to_le_bytes())
    }
}

/// Shared read cursor over a whole-file buffer.
#[derive(Debug, Clone, Default)]
struct Cursor {
    buf: Vec<u8>,
    pos: usize,
}

impl Cursor {
    /// fread semantics: copy what is available, -5 when short.
    fn read(&mut self, out: &mut [u8]) -> i32 {
        if out.is_empty() {
            return 0;
        }
        let avail = self.buf.len().saturating_sub(self.pos);
        let n = out.len().min(avail);
        out[..n].copy_from_slice(&self.buf[self.pos..self.pos + n]);
        self.pos += n;
        if n < out.len() { ERR_EOF } else { 0 }
    }
}

/// DTBinaryPersistStream: binary stream over a file (type 6). `Initialise(fd)` becomes `new(bytes)`.
#[derive(Debug, Clone)]
pub struct DTBinaryPersistStream {
    /// this+0xc
    pub stream_type: u32,
    cur: Cursor,
}

impl DTBinaryPersistStream {
    /// DTBinaryPersistStream(int fd) + Initialise: type=6, loader=0, DTSetmode(fd, 0x8000) (dropped:
    /// binary mode is the only mode of a byte buffer).
    pub fn new(file: Vec<u8>) -> Self {
        Self { stream_type: STREAM_BINARY_FILE, cur: Cursor { buf: file, pos: 0 } }
    }
    /// DTFtell equivalent (used by the pack walker to check entry boundaries).
    pub fn Tell(&self) -> usize {
        self.cur.pos
    }
    pub fn Len(&self) -> usize {
        self.cur.buf.len()
    }
    /// DTFread on the underlying fd, bypassing the stream (LoadLevel does this for classes
    /// 0x27/0x45/0x49/0x22). Returns the bytes read (short at EOF).
    pub fn ReadRaw(&mut self, n: usize) -> Vec<u8> {
        let mut v = vec![0u8; n.min(self.cur.buf.len().saturating_sub(self.cur.pos))];
        self.cur.read(&mut v);
        v
    }
}

impl DTBasePersistStream for DTBinaryPersistStream {
    fn GetStreamType(&self) -> u32 {
        self.stream_type
    }
    fn LoadBinaryData(&mut self, buf: &mut [u8]) -> i32 {
        self.cur.read(buf)
    }
}

/// DTMemoryPersistStream(buf, size): type 10. Fields: +0x4 buf, +0x10 size, +0x14 pos, +0x18 owns.
#[derive(Debug, Clone)]
pub struct DTMemoryPersistStream {
    pub stream_type: u32,
    cur: Cursor,
}

impl DTMemoryPersistStream {
    pub fn new(buf: Vec<u8>) -> Self {
        Self { stream_type: STREAM_MEMORY, cur: Cursor { buf, pos: 0 } }
    }
    /// Forward(n): pos+n > size -> pos = size, -5; else pos += n, 0.
    pub fn Forward(&mut self, n: usize) -> i32 {
        if self.cur.pos + n > self.cur.buf.len() {
            self.cur.pos = self.cur.buf.len();
            ERR_EOF
        } else {
            self.cur.pos += n;
            0
        }
    }
    /// GetCurrAddr (buf + pos): the remaining bytes.
    pub fn GetCurrAddr(&self) -> &[u8] {
        &self.cur.buf[self.cur.pos..]
    }
    pub fn GetBufferSize(&self) -> usize {
        self.cur.buf.len()
    }
    pub fn GetFilePointer(&self) -> usize {
        self.cur.pos
    }
}

impl DTBasePersistStream for DTMemoryPersistStream {
    fn GetStreamType(&self) -> u32 {
        self.stream_type
    }
    /// n==0 -> 0; short -> copy the rest, -5.
    fn LoadBinaryData(&mut self, buf: &mut [u8]) -> i32 {
        self.cur.read(buf)
    }
    /// DTMemoryPersistStream::SaveBinaryData: returns 0 (stub, like the base).
    fn SaveBinaryData(&mut self, _buf: &[u8]) -> i32 {
        0
    }
}

/// DTTextPersistStream(fd): type 5 (text). scanf/printf based. Loads parse whitespace-separated
/// tokens like fscanf; saves append to `out` (fprintf replacement).
#[derive(Debug, Clone)]
pub struct DTTextPersistStream {
    pub stream_type: u32,
    cur: Cursor,
    pub out: Vec<u8>,
}

impl DTTextPersistStream {
    pub fn new(file: Vec<u8>) -> Self {
        Self { stream_type: STREAM_TEXT_FILE, cur: Cursor { buf: file, pos: 0 }, out: Vec::new() }
    }
    fn peek(&self) -> Option<u8> {
        self.cur.buf.get(self.cur.pos).copied()
    }
    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_ascii_whitespace() {
                self.cur.pos += 1;
            } else {
                break;
            }
        }
    }
    /// fscanf("%s"): next whitespace-delimited token, None at EOF.
    fn token(&mut self) -> Option<String> {
        self.skip_ws();
        let start = self.cur.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_whitespace() {
                break;
            }
            self.cur.pos += 1;
        }
        if self.cur.pos == start { None } else { Some(String::from_utf8_lossy(&self.cur.buf[start..self.cur.pos]).into_owned()) }
    }
    /// fscanf of one numeric conversion: -1 (EOF) -> -5.
    fn scan<T: std::str::FromStr>(&mut self, v: &mut T) -> i32 {
        match self.token().and_then(|t| t.parse().ok()) {
            Some(x) => {
                *v = x;
                0
            }
            None => ERR_EOF,
        }
    }
    fn print(&mut self, s: String) -> i32 {
        self.out.extend_from_slice(s.as_bytes());
        0
    }
}

impl DTBasePersistStream for DTTextPersistStream {
    fn GetStreamType(&self) -> u32 {
        self.stream_type
    }
    fn LoadBinaryData(&mut self, buf: &mut [u8]) -> i32 {
        self.cur.read(buf)
    }
    fn SaveBinaryData(&mut self, buf: &[u8]) -> i32 {
        self.out.extend_from_slice(buf);
        0
    }
    fn LoadData_float(&mut self, v: &mut f32) -> i32 {
        self.scan(v)
    }
    fn LoadData_int(&mut self, v: &mut i32) -> i32 {
        self.scan(v)
    }
    fn LoadData_uint(&mut self, v: &mut u32) -> i32 {
        // "%d" into an unsigned: parse as i32 and reinterpret, like scanf does.
        let mut i = 0i32;
        let r = self.scan(&mut i);
        *v = i as u32;
        r
    }
    fn LoadData_short(&mut self, v: &mut i16) -> i32 {
        self.scan(v)
    }
    fn LoadData_ushort(&mut self, v: &mut u16) -> i32 {
        let mut i = 0i16;
        let r = self.scan(&mut i);
        *v = i as u16;
        r
    }
    fn LoadData_long(&mut self, v: &mut i64) -> i32 {
        self.scan(v)
    }
    /// "%c": next byte after skipping whitespace (scanf "%c" does NOT skip; the original format is
    /// " %c" only in SkipWhiteSpace callers). Kept as the raw next byte.
    fn LoadData_char(&mut self, v: &mut i8) -> i32 {
        let mut b = [0u8; 1];
        let r = self.cur.read(&mut b);
        *v = b[0] as i8;
        r
    }
    fn LoadData_uchar(&mut self, v: &mut u8) -> i32 {
        let mut b = [0u8; 1];
        let r = self.cur.read(&mut b);
        *v = b[0];
        r
    }
    fn SaveData_float(&mut self, v: f32) -> i32 {
        self.print(format!("{v:.8}"))
    }
    fn SaveData_int(&mut self, v: i32) -> i32 {
        self.print(format!("{v}"))
    }
    fn SaveData_uint(&mut self, v: u32) -> i32 {
        self.print(format!("{}", v as i32))
    }
    fn SaveData_short(&mut self, v: i16) -> i32 {
        self.print(format!("{v}"))
    }
    fn SaveData_ushort(&mut self, v: u16) -> i32 {
        self.print(format!("{}", v as i16))
    }
    fn SaveData_long(&mut self, v: i64) -> i32 {
        self.print(format!("{v}"))
    }
    fn SaveTextData(&mut self, s: &[u8]) -> i32 {
        self.out.extend_from_slice(s);
        0
    }
    /// LoadString: %s tokens, skipping `//` comments to end of line, until non-empty; EOF -> -5.
    fn LoadString(&mut self, out: &mut Vec<u8>, max: usize) -> i32 {
        loop {
            let Some(t) = self.token() else { return ERR_EOF };
            if t.starts_with("//") {
                while let Some(c) = self.peek() {
                    self.cur.pos += 1;
                    if c == b'\n' {
                        break;
                    }
                }
                continue;
            }
            out.clear();
            out.extend_from_slice(&t.as_bytes()[..t.len().min(max)]);
            return 0;
        }
    }
    /// LoadTextData: fgets lines, skip blank ones, strip "\n"/"\r" and leading spaces.
    fn LoadTextData(&mut self, out: &mut Vec<u8>, max: usize) -> i32 {
        loop {
            if self.cur.pos >= self.cur.buf.len() {
                return ERR_EOF;
            }
            let start = self.cur.pos;
            let mut end = start;
            while end < self.cur.buf.len() && end - start < max.saturating_sub(1) && self.cur.buf[end] != b'\n' {
                end += 1;
            }
            self.cur.pos = if end < self.cur.buf.len() && self.cur.buf[end] == b'\n' { end + 1 } else { end };
            let mut line = &self.cur.buf[start..end];
            while let [rest @ .., b'\r'] = line {
                line = rest;
            }
            while let [b' ', rest @ ..] = line {
                line = rest;
            }
            if line.is_empty() {
                continue;
            }
            out.clear();
            out.extend_from_slice(line);
            return 0;
        }
    }
    /// Reads bytes while ' ', '\t', '\n', '\r'; `c` = the first other byte (consumed).
    fn SkipWhiteSpace(&mut self, c: &mut u8) -> i32 {
        loop {
            let mut b = [0u8; 1];
            let r = self.cur.read(&mut b);
            if r != 0 {
                return r;
            }
            if !matches!(b[0], b' ' | b'\t' | b'\n' | b'\r') {
                *c = b[0];
                return 0;
            }
        }
    }
    fn SaveTextIndentation(&mut self) -> i32 {
        0
    }
}

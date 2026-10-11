//! Emulated 32-bit little-endian memory for module data (see engine/mod.rs for why).
use std::collections::HashMap;

/// A PS2 address inside [`Mem`]. 0 is NULL.
pub type Ptr = u32;
pub const NULL: Ptr = 0;

const BASE: u32 = 0x0010_0000;
/// Scratch stack for module locals whose address is passed around (C stack frames).
const STACK: u32 = 0x0004_0000;

pub struct Mem {
    data: Vec<u8>,
    /// next free byte of the heap (heap starts after the stack)
    brk: u32,
    /// stack pointer (grows up from BASE; frames are released by the logic runtime)
    sp: u32,
    /// free lists per rounded size (MMMalloc/MMFree)
    free: HashMap<u32, Vec<Ptr>>,
    sizes: HashMap<Ptr, u32>,
    /// interned C string literals (module .rodata)
    strs: HashMap<String, Ptr>,
}

impl Default for Mem {
    fn default() -> Self {
        Self::new()
    }
}

impl Mem {
    pub fn new() -> Self {
        Mem {
            data: vec![0; STACK as usize],
            brk: BASE + STACK,
            sp: BASE,
            free: HashMap::new(),
            sizes: HashMap::new(),
            strs: HashMap::new(),
        }
    }

    /// Byte offset of `a`; panics on NULL / wild pointers (the PS2 would crash or corrupt memory).
    #[inline]
    #[track_caller]
    fn at(&self, a: Ptr, n: u32) -> usize {
        let i = a.wrapping_sub(BASE);
        if a < BASE || (i as usize) + n as usize > self.data.len() {
            panic!("bad module memory access {a:#010x} ({n} bytes)");
        }
        i as usize
    }

    #[track_caller]
    pub fn r8(&self, a: Ptr) -> u8 {
        self.data[self.at(a, 1)]
    }
    #[track_caller]
    pub fn r16(&self, a: Ptr) -> u16 {
        let i = self.at(a, 2);
        u16::from_le_bytes([self.data[i], self.data[i + 1]])
    }
    #[track_caller]
    pub fn r32(&self, a: Ptr) -> u32 {
        let i = self.at(a, 4);
        u32::from_le_bytes(self.data[i..i + 4].try_into().unwrap())
    }
    #[track_caller]
    pub fn ri32(&self, a: Ptr) -> i32 {
        self.r32(a) as i32
    }
    #[track_caller]
    pub fn rf(&self, a: Ptr) -> f32 {
        f32::from_bits(self.r32(a))
    }
    #[track_caller]
    pub fn r64(&self, a: Ptr) -> u64 {
        self.r32(a) as u64 | (self.r32(a + 4) as u64) << 32
    }
    #[track_caller]
    pub fn w8(&mut self, a: Ptr, v: u8) {
        let i = self.at(a, 1);
        self.data[i] = v;
    }
    #[track_caller]
    pub fn w16(&mut self, a: Ptr, v: u16) {
        let i = self.at(a, 2);
        self.data[i..i + 2].copy_from_slice(&v.to_le_bytes());
    }
    #[track_caller]
    pub fn w32(&mut self, a: Ptr, v: u32) {
        let i = self.at(a, 4);
        self.data[i..i + 4].copy_from_slice(&v.to_le_bytes());
    }
    #[track_caller]
    pub fn wi32(&mut self, a: Ptr, v: i32) {
        self.w32(a, v as u32)
    }
    #[track_caller]
    pub fn wf(&mut self, a: Ptr, v: f32) {
        self.w32(a, v.to_bits())
    }
    #[track_caller]
    pub fn w64(&mut self, a: Ptr, v: u64) {
        self.w32(a, v as u32);
        self.w32(a + 4, (v >> 32) as u32);
    }
    /// DTVector3d at `a`
    #[track_caller]
    pub fn rv3(&self, a: Ptr) -> [f32; 3] {
        [self.rf(a), self.rf(a + 4), self.rf(a + 8)]
    }
    #[track_caller]
    pub fn wv3(&mut self, a: Ptr, v: [f32; 3]) {
        self.wf(a, v[0]);
        self.wf(a + 4, v[1]);
        self.wf(a + 8, v[2]);
    }
    #[track_caller]
    pub fn memset(&mut self, a: Ptr, v: u8, n: u32) {
        if n == 0 {
            return;
        }
        let i = self.at(a, n);
        self.data[i..i + n as usize].fill(v);
    }
    #[track_caller]
    pub fn memcpy(&mut self, dst: Ptr, src: Ptr, n: u32) {
        if n == 0 {
            return;
        }
        let s = self.at(src, n);
        let d = self.at(dst, n);
        self.data.copy_within(s..s + n as usize, d);
    }
    /// NUL-terminated string at `a`
    #[track_caller]
    pub fn cstr_at(&self, a: Ptr) -> String {
        let mut s = Vec::new();
        let mut p = a;
        loop {
            let c = self.r8(p);
            if c == 0 {
                break;
            }
            s.push(c);
            p += 1;
        }
        String::from_utf8_lossy(&s).into_owned()
    }

    /// A C string literal of the module (.rodata); the same text always yields the same address.
    pub fn cstr(&mut self, s: &str) -> Ptr {
        if let Some(&p) = self.strs.get(s) {
            return p;
        }
        let p = self.malloc(s.len() as u32 + 1);
        for (i, b) in s.bytes().enumerate() {
            self.w8(p + i as u32, b);
        }
        self.strs.insert(s.to_owned(), p);
        p
    }

    /// MMMalloc: 16-byte aligned, contents zeroed (the PS2 heap is not; nothing relies on garbage).
    pub fn malloc(&mut self, n: u32) -> Ptr {
        let size = n.max(1).next_multiple_of(16);
        let p = match self.free.get_mut(&size).and_then(|v| v.pop()) {
            Some(p) => {
                self.memset(p, 0, size);
                p
            }
            None => {
                let p = self.brk;
                self.brk += size;
                self.data.resize((self.brk - BASE) as usize, 0);
                p
            }
        };
        self.sizes.insert(p, size);
        p
    }
    pub fn free(&mut self, p: Ptr) {
        if p == NULL {
            return;
        }
        let size = self.sizes.remove(&p).unwrap_or_else(|| panic!("free of unallocated {p:#x}"));
        self.free.entry(size).or_default().push(p);
    }
    /// MMRealloc: keeps the old contents (bytes past the old size are zero).
    pub fn realloc(&mut self, p: Ptr, n: u32) -> Ptr {
        if p == NULL {
            return self.malloc(n);
        }
        let old = self.sizes[&p];
        if n <= old {
            return p;
        }
        let q = self.malloc(n);
        self.memcpy(q, p, old);
        self.free(p);
        q
    }

    /// Stack space for a C local whose address is taken (`auStack_90`, `&DStack_60`).
    /// Released when the enclosing logic call returns (the runtime restores the stack pointer).
    pub fn alloca(&mut self, n: u32) -> Ptr {
        let p = self.sp;
        let size = n.max(1).next_multiple_of(16);
        assert!(self.sp + size <= BASE + STACK, "module stack overflow");
        self.sp += size;
        self.memset(p, 0, size);
        p
    }
    pub fn sp(&self) -> u32 {
        self.sp
    }
    pub fn set_sp(&mut self, sp: u32) {
        self.sp = sp;
    }
}

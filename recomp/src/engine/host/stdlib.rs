//! WP-S3: APIstdlib host side (EFApi_stdlib.cpp: the table entries are MSL libc functions).
//! Replaced (rule 2, libc): Rust implementations with the C-library contract the modules
//! observe. `long` is 32-bit on the EE ABI the game uses: strtol/strtoul/atol saturate at the
//! 32-bit limits and the i64 results are sign-/zero-extended.
#![allow(non_snake_case)]

use crate::engine::{Mem, Ptr};

fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// strtoul/strtol core: (negative, magnitude saturated to u64::MAX, end pointer, any digits)
fn parse_int(m: &Mem, s: Ptr, base: i32) -> (bool, u64, Ptr, bool) {
    let mut p = s;
    while is_space(m.r8(p)) {
        p += 1;
    }
    let mut neg = false;
    if matches!(m.r8(p), b'+' | b'-') {
        neg = m.r8(p) == b'-';
        p += 1;
    }
    let mut base = base as u32;
    let hex_prefix = m.r8(p) == b'0' && matches!(m.r8(p + 1), b'x' | b'X') && (m.r8(p + 2) as char).is_ascii_hexdigit();
    if (base == 0 || base == 16) && hex_prefix {
        p += 2;
        base = 16;
    } else if base == 0 {
        base = if m.r8(p) == b'0' { 8 } else { 10 };
    }
    if !(2..=36).contains(&base) {
        return (false, 0, s, false);
    }
    let mut v: u64 = 0;
    let mut any = false;
    while let Some(d) = (m.r8(p) as char).to_digit(36).filter(|d| *d < base) {
        v = v.saturating_mul(base as u64).saturating_add(d as u64);
        any = true;
        p += 1;
    }
    (neg, v, if any { p } else { s }, any)
}

pub fn abs(a: i32) -> i32 {
    a.wrapping_abs()
}
pub fn labs(a: i64) -> i64 {
    a.wrapping_abs()
}
/// div: div_t {quot, rem} returned in one 64-bit register (quot low word)
pub fn div(a: i32, b: i32) -> u64 {
    if b == 0 {
        return 0;
    }
    (a.wrapping_div(b) as u32 as u64) | ((a.wrapping_rem(b) as u32 as u64) << 32)
}
/// ldiv(ldiv_t *ret, num, den): {quot, rem} written to the struct-return pointer
pub fn ldiv(m: &mut Mem, ret: Ptr, a: i64, b: i64) -> Ptr {
    if b != 0 && ret != 0 {
        m.w32(ret, a.wrapping_div(b) as u32);
        m.w32(ret + 4, a.wrapping_rem(b) as u32);
    }
    ret
}
pub fn strtol(m: &mut Mem, s: Ptr, end: Ptr, base: i32) -> i64 {
    let (neg, v, p, _) = parse_int(m, s, base);
    if end != 0 {
        m.w32(end, p);
    }
    if neg {
        if v > 0x8000_0000 { i32::MIN as i64 } else { -(v as i64) }
    } else if v > i32::MAX as u64 {
        i32::MAX as i64
    } else {
        v as i64
    }
}
pub fn strtoul(m: &mut Mem, s: Ptr, end: Ptr, base: i32) -> u64 {
    let (neg, v, p, _) = parse_int(m, s, base);
    if end != 0 {
        m.w32(end, p);
    }
    if v > u32::MAX as u64 {
        return u32::MAX as u64;
    }
    (if neg { (v as u32).wrapping_neg() } else { v as u32 }) as u64
}
pub fn atoi(m: &mut Mem, s: Ptr) -> i32 {
    strtol(m, s, 0, 10) as i32
}
pub fn atol(m: &mut Mem, s: Ptr) -> i64 {
    strtol(m, s, 0, 10)
}
/// strtod: [ws][sign](digits[.digits]|.digits)[(e|E)[sign]digits]; inf/nan forms not needed by
/// the game data (MSL accepts them; treated as no conversion here)
pub fn strtod(m: &mut Mem, s: Ptr, end: Ptr) -> f64 {
    let mut p = s;
    while is_space(m.r8(p)) {
        p += 1;
    }
    let start = p;
    if matches!(m.r8(p), b'+' | b'-') {
        p += 1;
    }
    let mut digits = 0;
    while m.r8(p).is_ascii_digit() {
        p += 1;
        digits += 1;
    }
    if m.r8(p) == b'.' {
        p += 1;
        while m.r8(p).is_ascii_digit() {
            p += 1;
            digits += 1;
        }
    }
    let (v, endp) = if digits == 0 {
        (0.0, s)
    } else {
        if matches!(m.r8(p), b'e' | b'E') {
            let mut q = p + 1;
            if matches!(m.r8(q), b'+' | b'-') {
                q += 1;
            }
            if m.r8(q).is_ascii_digit() {
                while m.r8(q).is_ascii_digit() {
                    q += 1;
                }
                p = q;
            }
        }
        let txt: String = (start..p).map(|a| m.r8(a) as char).collect();
        let txt = if txt.ends_with('.') { format!("{txt}0") } else { txt };
        (txt.parse::<f64>().unwrap_or(0.0), p)
    };
    if end != 0 {
        m.w32(end, endp);
    }
    v
}
pub fn atof(m: &mut Mem, s: Ptr) -> f64 {
    strtod(m, s, 0)
}
/// mblen in the "C" locale: NULL or "" -> 0, else 1 (when n >= 1), -1 for n == 0
pub fn mblen(m: &Mem, s: Ptr, n: u32) -> i32 {
    if s == 0 || m.r8(s) == 0 {
        0
    } else if n == 0 {
        -1
    } else {
        1
    }
}

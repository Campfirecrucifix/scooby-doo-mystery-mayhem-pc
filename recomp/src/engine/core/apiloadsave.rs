//! WP-M1: APILoadSave = EFLoadSaveMgr (EFLoadSaveMgr_BC 0x00275270.. + EFLoadSaveMgr_SP 0x002185c0..)
//! with the memory card emulated as host files.
//!
//! 1:1 part: the manager object `goLoadSaveMgr` (0xaa8 bytes) lives in `Mem` with the original field
//! offsets, because the modules read it through pointers (GetFileDescriptor = this+0x30,
//! GetPort{1,2}CardInfoBuffer = *(this+0x678/0x67c), GetReadBuffer = *(this+0x1c)). The action queue
//! (RegisterAction / GetAction / UnRegisterAction / IsAction ring at this+0x3a4, indices +0x5a4/+0x5a8/
//! +0x5ac), every LSM_* state machine, the SpecificUpdate dispatch (two switch blocks, with the
//! original's missing cases: WriteSaveGame 0x1a and DeleteDirectory 7 are queued but never handled, so
//! they complete without doing anything), the status/error codes (+0x28/+0x2c) and the callback
//! parameter `{GetLastCommand(), error}` are transliterated.
//! Vtable (EFLoadSaveMgr::vtable 0x336030, read from the ELF): +8 SpecificUpdate, +0xc
//! SpecificInitialize, +0x14 CreateDirectory, +0x18 ChangeDirectory, +0x1c DeleteDirectory, +0x20 Write,
//! +0x24 Read, +0x28 ReadSaveGame, +0x2c NULL, +0x30 DeleteFile, +0x34 DeleteAllFiles, +0x38
//! GetDirectory, +0x3c IsMyFileFit, +0x40 GetFreeMem, +0x44 VerifyCheckSum, +0x48 GetLastCommand.
//!
//! Replaced (rule 2): Sony libmc (sceMcInit/GetInfo/Mkdir/Chdir/GetDir/Open/Close/Read/Write/Seek/
//! Delete/Format/GetEntSpace/Sync) by [`MemCard`]: an 8 MB PS2 card in memory card slot 1 (port 0,
//! slot 0) whose file system is the host directory `GAME_SAVEDIR` (default `saves/memcard1`, relative
//! to the working directory). Every sce call is asynchronous like the real one: the result (and the
//! data the IOP would DMA into EE memory) is delivered by sceMcSync after a latency in manager
//! updates (see `latency`). `GAME_NOCARD=1` = empty slot (GetInfo -10, type 0 -> error 0x12).
//! Card model: 8000 KB of 1 KB clusters; a file uses ceil(size/1KB) clusters, a directory
//! ceil(entries/2) (512-byte entries incl. "." and ".."; the root is not counted), free = 8000 - used.
//! Time stamps are the PS2 RTC's JST (UTC+9), as written by a real console.
#![allow(non_snake_case)]
use crate::engine::*;
use crate::logic::CodeFn;
use std::path::{Path, PathBuf};

/// card error for "no card in the port" (EFLoadSaveMgr_SP::SetWorkingMemoryCard, card type 0)
pub const ERR_NO_CARD: u32 = 0x12;

// ---------------------------------------------------------------- goLoadSaveMgr field offsets
const F_DIRNAME: u32 = 0xc; // current dir name ptr
const F_FILENAME: u32 = 0x14;
const F_WRITEBUF: u32 = 0x18;
const F_READBUF: u32 = 0x1c;
const F_SIZE: u32 = 0x24;
const F_STATUS: u32 = 0x28;
const F_ERROR: u32 = 0x2c;
const F_DESC: u32 = 0x30; // 20 * 0x2c file descriptors (0x370)
const F_CRC: u32 = 0x3a0;
const F_QUEUE: u32 = 0x3a4;
const F_QEND: u32 = 0x5a4;
const F_QHEAD: u32 = 0x5a8;
const F_QCUR: u32 = 0x5ac;
const F_LASTACTION: u32 = 0x5b0;
const F_TEMPBUF: u32 = 0x5b4;
const F_ERRCB: u32 = 0x5b8;
const F_DONECB: u32 = 0x5bc;
const F_PORT: u32 = 0x5c0;
const F_SLOT: u32 = 0x5c4;
const F_MCCMD: u32 = 0x5c8;
const F_MCRESULT: u32 = 0x5cc;
const F_SLOTMAX1: u32 = 0x5d0;
const F_SLOTMAX2: u32 = 0x5d4;
const F_ACTIVE1: u32 = 0x5d8;
const F_ACTIVE2: u32 = 0x5dc;
const F_SLOTIT1: u32 = 0x5e4;
const F_SLOTIT2: u32 = 0x5e8;
const F_LASTCMD: u32 = 0x5ec;
const F_PENDING: u32 = 0x5f0;
const F_INFO1: u32 = 0x5f4; // port 1: 4 slots * {type, free, format, result}
const F_INFO2: u32 = 0x634;
const F_INFOCUR: u32 = 0x674;
const F_INFOP1: u32 = 0x678;
const F_INFOP2: u32 = 0x67c;
const F_ENTSPACE: u32 = 0x680;
const F_FD: u32 = 0x684;
const F_MODE: u32 = 0x688; // open mode, then seek offset +0x68c / whence +0x690
const F_SEEKOFS: u32 = 0x68c;
const F_SEEKMODE: u32 = 0x690;
const F_GETDIRNAME: u32 = 0x694;
const F_ICONBUF: u32 = 0x69c;
const F_CWD: u32 = 0x6a0;
const F_FLAGS: u32 = 0xaa0;
const F_DIRCOUNT: u32 = 0xaa4;
const MGR_SIZE: u32 = 0xaa8;
/// 0x01f72680: the sceMcGetDir table (0x80 entries of sceMcTblGetDir, 0x40 bytes)
const DIRTBL_SIZE: u32 = 0x80 * 0x40;

/// Per-engine state of the APILoadSave port (stored in `LogicRuntime::loadsave`).
#[derive(Default)]
pub struct McState {
    /// &goLoadSaveMgr in Mem (0 until first use)
    pub mgr: Ptr,
    /// the 0x01f72680 GetDir table in Mem
    pub dirtbl: Ptr,
    /// "" string for ReadSaveGame / "*" (cAllFiles)
    strings: Ptr,
    /// port: the logic instance that installed the callbacks. The module callbacks are plain C
    /// functions in the original; here module code needs its instance on the logic stack, so the
    /// callback runs as that instance (EFLogicInstance::PushLogic / PopLogic, kind 2 like CallCB).
    owner: Option<crate::logic::ObjH>,
    pub card: MemCard,
}

fn call(e: &mut Engine, token: u32, arg: Ptr) {
    if token == 0 {
        return;
    }
    match e.code_fn(token) {
        CodeFn::Word(f) => {
            let h = e.rt.loadsave.owner.filter(|&h| e.rt.objects.get(h).is_some_and(|o| o.is_some()));
            if let Some(h) = h {
                e.PushLogic(h, 2);
            }
            f(e, arg);
            if h.is_some() {
                e.PopLogic();
            }
        }
        _ => panic!("APILoadSave callback is not a word function"),
    }
}

/// goLoadSaveMgr; constructed + EFLoadSaveMgr_BC::Initialize'd on first use (the original does both
/// in EFEngineController's start-up, before any module runs).
fn mgr(e: &mut Engine) -> Ptr {
    if e.rt.loadsave.mgr == 0 {
        let m = e.mem.malloc(MGR_SIZE);
        e.mem.memset(m, 0, MGR_SIZE);
        let t = e.mem.malloc(DIRTBL_SIZE);
        e.mem.memset(t, 0, DIRTBL_SIZE);
        let s = e.mem.malloc(4);
        e.mem.w32(s, 0); // s = "", s+2 = "*"
        e.mem.w8(s + 2, b'*');
        e.rt.loadsave.mgr = m;
        e.rt.loadsave.dirtbl = t;
        e.rt.loadsave.strings = s;
        e.rt.loadsave.card.init();
        // EFLoadSaveMgr_BC ctor
        for o in [F_QEND, F_QHEAD, F_QCUR] {
            e.w32(m + o, 0x40);
        }
        // EFLoadSaveMgr_SP ctor
        e.w32(m + F_INFOCUR, m + F_INFO1);
        e.w32(m + F_INFOP1, m + F_INFO1);
        e.w32(m + F_INFOP2, m + F_INFO2);
        EFLoadSaveMgr_BC::Initialize(e, m);
    }
    e.rt.loadsave.mgr
}

macro_rules! status {
    ($e:ident, $m:ident, $bit:expr) => {{
        let v = $e.r32($m + F_STATUS) | $bit;
        $e.w32($m + F_STATUS, v);
    }};
}
macro_rules! error {
    ($e:ident, $m:ident, $err:expr) => {{
        status!($e, $m, 1);
        $e.w32($m + F_ERROR, $err);
    }};
}

fn busy(e: &Engine, m: Ptr) -> bool {
    e.r32(m + F_FLAGS) & 8 != 0
}
fn set_busy(e: &mut Engine, m: Ptr, cmd: u32) {
    let v = e.r32(m + F_FLAGS) | 8;
    e.w32(m + F_FLAGS, v);
    e.w32(m + F_PENDING, cmd);
}

pub struct EFLoadSaveMgr_BC;
impl EFLoadSaveMgr_BC {
    /// 0x00275340
    pub fn Initialize(e: &mut Engine, m: Ptr) {
        for o in [F_WRITEBUF, F_READBUF, 0x20, F_STATUS, F_ERROR, F_QEND, F_QHEAD, F_QCUR, F_LASTACTION, F_ERRCB, F_DONECB] {
            // quirk: the queue indices are reset to 0 here, not to 0x40 like ResetAction
            e.w32(m + o, 0);
        }
        EFLoadSaveMgr_SP::SpecificInitialize(e, m);
    }

    /// 0x00275390: mode 1 = append at +0x5a4, else push in front of +0x5a8 (reverse order)
    pub fn RegisterAction(e: &mut Engine, m: Ptr, a: &[u32], mode: i32) -> u32 {
        let n = ((a[0] & 0xf00_0000) >> 24) as i32;
        if mode == 1 {
            if 0x80 < e.ri32(m + F_QEND) + n + 1 {
                return 0;
            }
            for i in 0..=n as usize {
                let end = e.r32(m + F_QEND);
                e.w32(m + end * 4 + F_QUEUE, a[i]);
                e.w32(m + F_QEND, end + 1);
            }
        } else {
            if e.ri32(m + F_QHEAD) - n - 1 < 0 {
                return 0;
            }
            for i in (0..=n as usize).rev() {
                let h = e.r32(m + F_QHEAD) - 1;
                e.w32(m + F_QHEAD, h);
                e.w32(m + h * 4 + F_QUEUE, a[i]);
            }
        }
        1
    }

    /// 0x002755a0
    pub fn GetAction(e: &mut Engine, m: Ptr) -> u32 {
        loop {
            let h = e.r32(m + F_QHEAD);
            let w = e.r32(m + h * 4 + F_QUEUE);
            e.w32(m + F_QCUR, h);
            if w & 0x8000_0000 == 0 {
                if w & 0x4000_0000 != 0 {
                    e.w32(m + F_LASTACTION, w & 0xff_ffff);
                }
                return w & 0xff_ffff;
            }
            if Self::UnRegisterAction(e, m) == 0 {
                return 0;
            }
        }
    }

    /// 0x00275650
    pub fn GetArgPtr(e: &Engine, m: Ptr) -> Ptr {
        m + e.r32(m + F_QHEAD) * 4 + F_QUEUE
    }

    /// 0x00275670
    pub fn IsAction(e: &mut Engine, m: Ptr) -> bool {
        let h = e.r32(m + F_QHEAD);
        let end = e.r32(m + F_QEND);
        if h == end {
            Self::ResetAction(e, m);
        }
        h != end
    }

    /// 0x002756a0
    pub fn UnRegisterAction(e: &mut Engine, m: Ptr) -> u32 {
        let h = e.r32(m + F_QHEAD);
        if h == e.r32(m + F_QEND) {
            return 0;
        }
        let cur = e.r32(m + F_QCUR);
        if cur == h {
            let w = e.r32(m + h * 4 + F_QUEUE);
            let next = (h as i32 + 1 + ((w & 0xf00_0000) >> 24) as i32) & 0x7f;
            e.w32(m + F_QHEAD, next as u32);
        } else {
            let w = e.r32(m + cur * 4 + F_QUEUE) | 0x8000_0000;
            e.w32(m + cur * 4 + F_QUEUE, w);
        }
        1
    }

    /// 0x00275720
    pub fn ResetAction(e: &mut Engine, m: Ptr) {
        for o in [F_QEND, F_QHEAD, F_QCUR] {
            e.w32(m + o, 0x40);
        }
    }

    fn report(e: &mut Engine, m: Ptr, cb_field: u32) {
        let cb = e.r32(m + cb_field);
        if std::env::var_os("GAME_MCLOG").is_some() {
            eprintln!("mc: {} cmd {:#x} err {:#x}", if cb_field == F_ERRCB { "error" } else { "complete" }, e.r32(m + F_LASTCMD), e.r32(m + F_ERROR));
        }
        if cb != 0 {
            let p = e.alloca(8);
            let last = EFLoadSaveMgr_SP::GetLastCommand(e, m);
            e.w32(p, last);
            let err = e.r32(m + F_ERROR);
            e.w32(p + 4, err);
            call(e, cb, p);
        }
        Self::Clean(e, m);
    }

    /// 0x00275740
    pub fn ProcessError(e: &mut Engine, m: Ptr) {
        Self::ResetAction(e, m);
        Self::report(e, m, F_ERRCB);
    }

    /// 0x00275900
    pub fn CommandsCompleted(e: &mut Engine, m: Ptr) {
        Self::report(e, m, F_DONECB);
    }

    /// 0x00275ac0
    pub fn Clean(e: &mut Engine, m: Ptr) {
        let p = e.r32(m + 0x20);
        e.mem.free(p);
        e.w32(m + 0x20, 0);
        let p = e.r32(m + F_READBUF);
        e.mem.free(p);
        e.w32(m + F_READBUF, 0);
    }

    /// 0x00275b00
    pub fn ComputeCheckSum(e: &mut Engine, m: Ptr) -> u32 {
        let (p, n) = (e.r32(m + F_WRITEBUF), e.r32(m + F_SIZE));
        let c = EFLoadSaveBuff::ComputeCrc(e, p, n);
        e.w32(m + F_CRC, c);
        0
    }

    /// 0x00275b40
    pub fn VerifyCheckSum(e: &mut Engine, m: Ptr) -> u32 {
        let (p, n) = (e.r32(m + F_READBUF), e.r32(m + F_SIZE));
        if e.r32(m + F_CRC) != EFLoadSaveBuff::ComputeCrc(e, p, n) {
            error!(e, m, 1);
        }
        0
    }

    /// 0x00275b90 (Ghidra lost the MMMalloc size: the API's argument)
    pub fn GetUserTempBuffer(e: &mut Engine, m: Ptr, size: u32) -> Ptr {
        if e.r32(m + F_TEMPBUF) == 0 {
            let p = e.mem.malloc(size.max(1));
            e.w32(m + F_TEMPBUF, p);
        }
        e.r32(m + F_TEMPBUF)
    }

    /// 0x00275bd0
    pub fn DestroyUserTempBuffer(e: &mut Engine, m: Ptr) {
        let p = e.r32(m + F_TEMPBUF);
        if p != 0 {
            e.mem.free(p);
            e.w32(m + F_TEMPBUF, 0);
        }
    }

    /// 0x00275c60
    pub fn DestroyReadBuffer(e: &mut Engine, m: Ptr) {
        let p = e.r32(m + F_READBUF);
        e.mem.free(p);
        e.w32(m + F_READBUF, 0);
    }
}

pub struct EFLoadSaveBuff;
impl EFLoadSaveBuff {
    /// EFLoadSaveBuff::ComputeCrc 0x002162c0: byte sum (see EFLoadSaveBuff.cpp)
    pub fn ComputeCrc(e: &Engine, p: Ptr, n: u32) -> u32 {
        let mut c: u32 = 0;
        for i in 0..n {
            c = c.wrapping_add(e.r8(p + i) as u32);
        }
        c
    }
}

pub struct EFLoadSaveMgr_SP;
impl EFLoadSaveMgr_SP {
    /// 0x00218690 (sceMtapInit / sceMtapPortOpen(2,3) / sceMcInit always succeed)
    pub fn SpecificInitialize(e: &mut Engine, m: Ptr) -> u32 {
        for o in [F_PORT, F_SLOT, F_SLOTMAX1, F_SLOTMAX2, F_PENDING, F_SEEKMODE, F_LASTCMD] {
            e.w32(m + o, 0);
        }
        e.w32(m + F_INFOCUR, m + F_INFO1);
        e.w32(m + F_INFOP1, m + F_INFO1);
        e.w32(m + F_INFOP2, m + F_INFO2);
        e.mem.memset(m + F_INFO1, 0, 0x80);
        e.w32(m + F_FLAGS, 1);
        1
    }

    fn info(e: &Engine, m: Ptr) -> Ptr {
        e.r32(m + F_INFOCUR) + e.r32(m + F_SLOT) * 0x10
    }

    /// 0x00218800
    pub fn SetWorkingMemoryCard(e: &mut Engine, m: Ptr, a: Ptr) -> u32 {
        let st = e.ri32(a + 0xc);
        if st == 2 {
            e.w32(m + F_STATUS, 0);
            let pi = Self::info(e, m);
            match e.ri32(pi) {
                0 => error!(e, m, 0x12),
                1 | 3 => error!(e, m, 0x21),
                2 => status!(e, m, 0x20),
                _ => match e.ri32(pi + 8) {
                    0 => error!(e, m, 0x13),
                    1 => status!(e, m, 4),
                    _ => {}
                },
            }
            let r = e.r32(m + F_MCRESULT);
            e.w32(pi + 0xc, r);
            e.w32(a + 0xc, 0);
            return 0;
        }
        if st != 1 {
            if st != 0 {
                e.w32(a + 0xc, 0);
                return 0;
            }
            let (port, slot) = (e.r32(a + 8), e.r32(a + 4));
            e.w32(m + F_PORT, port);
            e.w32(m + F_SLOT, slot);
            e.w32(m + F_INFOCUR, m + port * 0x40 + F_INFO1);
            e.w32(a + 0xc, 1);
        }
        if !busy(e, m) {
            set_busy(e, m, 0xd);
            let pi = Self::info(e, m);
            let (port, slot) = (e.ri32(m + F_PORT), e.ri32(m + F_SLOT));
            sceMcGetInfo(e, port, slot, pi, pi + 4, pi + 8);
            e.w32(a + 0xc, 2);
        }
        1
    }

    /// 0x002189c0
    pub fn GetCardStatus(e: &mut Engine, m: Ptr, a: Ptr) -> u32 {
        if e.r16(a + 4) != 2 {
            if e.r16(a + 4) != 0 {
                e.w32(a + 4, 0);
                return 0;
            }
            // sceMcGetSlotMax(0/1): no multitap -> 1 slot per port
            e.w32(m + F_SLOTMAX1, 1);
            e.w32(m + F_SLOTMAX2, 1);
            for o in [F_ACTIVE1, F_ACTIVE2, F_SLOTIT1, F_SLOTIT2] {
                e.w32(m + o, 0);
            }
            e.w32(a + 4, 2);
        }
        let res = e.ri32(m + F_MCRESULT);
        let st = e.r32(a + 4);
        if st & 0x1000_0000 != 0 && -10 < res {
            let it = e.r32(m + F_SLOTIT1);
            if e.ri32(m + it * 0x10 + 0x5e4) == 2 {
                let v = e.r32(m + F_ACTIVE1) | 1 << (it.wrapping_sub(1) & 0x1f);
                e.w32(m + F_ACTIVE1, v);
            }
        }
        if st & 0x2000_0000 != 0 && -10 < res {
            let it = e.r32(m + F_SLOTIT2);
            if e.ri32(m + it * 0x10 + 0x624) == 2 {
                let v = e.r32(m + F_ACTIVE2) | 1 << (it.wrapping_sub(1) & 0x1f);
                e.w32(m + F_ACTIVE2, v);
            }
        }
        e.w32(a + 4, st & 0xcfff_ffff);
        if e.ri32(m + F_SLOTIT1) < e.ri32(m + F_SLOTMAX1) {
            if !busy(e, m) {
                set_busy(e, m, 0xc);
                let i = e.r32(m + F_SLOTIT1);
                let pi = m + i * 0x10 + F_INFO1;
                sceMcGetInfo(e, 0, i as i32, pi, pi + 4, pi + 8);
                let v = e.r32(a + 4) | 0x1000_0000;
                e.w32(a + 4, v);
                e.w32(m + F_SLOTIT1, i + 1);
            }
            return 1;
        }
        if e.ri32(m + F_SLOTMAX2) <= e.ri32(m + F_SLOTIT2) {
            e.w32(a + 4, 0);
            return 0;
        }
        if !busy(e, m) {
            set_busy(e, m, 0xc);
            let i = e.r32(m + F_SLOTIT2);
            let pi = m + i * 0x10 + F_INFO2;
            sceMcGetInfo(e, 1, i as i32, pi, pi + 4, pi + 8);
            let v = e.r32(a + 4) | 0x2000_0000;
            e.w32(a + 4, v);
            e.w32(m + F_SLOTIT2, i + 1);
        }
        1
    }

    /// Common shape of the single-sce-call actions: state at `a+so`; state 1 = map the sceMcSync
    /// result, state 0 = issue (unless a call is in flight). `map(result)` returns the error code
    /// or None for success (status |= 2); `ok` runs on success.
    fn simple(
        e: &mut Engine,
        m: Ptr,
        a: Ptr,
        so: u32,
        map: &[(i32, u32)],
        neg_only: bool,
        ok: impl FnOnce(&mut Engine, i32),
        issue: impl FnOnce(&mut Engine) -> u32,
    ) -> u32 {
        match e.ri32(a + so) {
            1 => {
                let r = e.ri32(m + F_MCRESULT);
                if (neg_only && r >= 0) || (!neg_only && r == 0) {
                    ok(e, r);
                    status!(e, m, 2);
                } else if let Some(&(_, c)) = map.iter().find(|(v, _)| *v == r) {
                    error!(e, m, c);
                } else if r < -9 {
                    error!(e, m, 0x12);
                }
                e.w32(a + so, 0);
                0
            }
            0 => {
                if busy(e, m) {
                    return 1;
                }
                let cmd = issue(e);
                set_busy(e, m, cmd);
                e.w32(a + so, 1);
                1
            }
            _ => {
                e.w32(a + so, 0);
                0
            }
        }
    }

    /// 0x00218ce0
    pub fn FormatCard(e: &mut Engine, m: Ptr, a: Ptr) -> u32 {
        match e.ri32(a + 4) {
            1 => {
                if e.ri32(m + F_MCRESULT) == 0 {
                    status!(e, m, 2);
                } else {
                    error!(e, m, 0x12);
                }
                e.w32(a + 4, 0);
                0
            }
            0 => {
                if busy(e, m) {
                    return 1;
                }
                set_busy(e, m, 0xe);
                let (p, s) = (e.ri32(m + F_PORT), e.ri32(m + F_SLOT));
                sceMcFormat(e, p, s);
                e.w32(a + 4, 1);
                1
            }
            _ => {
                e.w32(a + 4, 0);
                0
            }
        }
    }

    /// 0x00218da0
    pub fn CreateDirectory(e: &mut Engine, m: Ptr, a: Ptr) -> u32 {
        if e.ri32(a + 8) == 0 {
            let d = e.r32(a + 4);
            e.w32(m + F_DIRNAME, d);
        }
        Self::simple(e, m, a, 8, &[(-2, 0x13), (-3, 0x14), (-4, 0x15)], false, |_, _| {}, |e| {
            let (p, s, d) = (e.ri32(m + F_PORT), e.ri32(m + F_SLOT), e.r32(m + F_DIRNAME));
            sceMcMkdir(e, p, s, d);
            1
        })
    }

    /// 0x00218ee0
    pub fn ChangeDirectory(e: &mut Engine, m: Ptr, a: Ptr) -> u32 {
        if e.ri32(a + 8) == 0 {
            let d = e.r32(a + 4);
            e.w32(m + F_DIRNAME, d);
            let s = e.cstr_at(d);
            write_cstr(e, m + F_CWD, &s);
        }
        Self::simple(e, m, a, 8, &[(-2, 0x13), (-4, 0x16)], false, |_, _| {}, |e| {
            let (p, s, d) = (e.ri32(m + F_PORT), e.ri32(m + F_SLOT), e.r32(m + F_DIRNAME));
            sceMcChdir(e, p, s, d);
            2
        })
    }

    /// 0x00219010
    pub fn GetDirectoryEntrySpace(e: &mut Engine, m: Ptr, a: Ptr) -> u32 {
        if e.ri32(a + 8) == 0 {
            let d = e.r32(a + 4);
            e.w32(m + F_DIRNAME, d);
        }
        Self::simple(e, m, a, 8, &[(-2, 0x13), (-5, 0x17)], true, |e, r| e.wi32(m + F_ENTSPACE, r), |e| {
            let (p, s, d) = (e.ri32(m + F_PORT), e.ri32(m + F_SLOT), e.r32(m + F_DIRNAME));
            sceMcGetEntSpace(e, p, s, d);
            0xf
        })
    }

    /// 0x00219130
    pub fn OpenFile(e: &mut Engine, m: Ptr, a: Ptr) -> u32 {
        let map = [(-2, 0x13), (-3, 0x14), (-4, 0x18), (-5, 0x17), (-7, 0x19)];
        Self::simple(e, m, a, 4, &map, true, |e, r| e.wi32(m + F_FD, r), |e| {
            let (p, s, n, md) = (e.ri32(m + F_PORT), e.ri32(m + F_SLOT), e.r32(m + F_FILENAME), e.ri32(m + F_MODE));
            sceMcOpen(e, p, s, n, md);
            0x10
        })
    }

    /// 0x002192b0
    pub fn CloseFile(e: &mut Engine, m: Ptr, a: Ptr) -> u32 {
        Self::simple(e, m, a, 4, &[(-2, 0x13), (-4, 0x1a)], true, |_, _| {}, |e| {
            let fd = e.ri32(m + F_FD);
            sceMcClose(e, fd);
            0x11
        })
    }

    /// 0x002193c0
    pub fn WriteFile(e: &mut Engine, m: Ptr, a: Ptr) -> u32 {
        if e.ri32(a + 0xc) == 1 {
            let r = e.ri32(m + F_MCRESULT);
            if r >= 0 && r != e.ri32(a + 8) {
                // quirk: a short write re-issues the whole write
                e.w32(a + 0xc, 0);
                return 1;
            }
        }
        let map = [(-2, 0x13), (-3, 0x14), (-4, 0x1a), (-5, 0x17), (-8, 0x1b)];
        Self::simple(e, m, a, 0xc, &map, true, |_, _| {}, |e| {
            let (fd, b, n) = (e.ri32(m + F_FD), e.r32(a + 4), e.ri32(a + 8));
            sceMcWrite(e, fd, b, n);
            // sceMcFlush right behind it: rejected by libmc while the write is in flight (no effect)
            0x12
        })
    }

    /// 0x00219560
    pub fn ReadFile(e: &mut Engine, m: Ptr, a: Ptr) -> u32 {
        let map = [(-2, 0x13), (-3, 0x1d), (-4, 0x1a), (-5, 0x17)];
        Self::simple(e, m, a, 0xc, &map, true, |_, _| {}, |e| {
            let (fd, b, n) = (e.ri32(m + F_FD), e.r32(a + 4), e.ri32(a + 8));
            sceMcRead(e, fd, b, n);
            0x13
        })
    }

    /// 0x002196c0
    pub fn Seek(e: &mut Engine, m: Ptr, a: Ptr) -> u32 {
        Self::simple(e, m, a, 4, &[(-2, 0x13), (-4, 0x1a)], true, |e, r| e.wi32(m + F_SEEKOFS, r), |e| {
            let (fd, o, w) = (e.ri32(m + F_FD), e.ri32(m + F_SEEKOFS), e.ri32(m + F_SEEKMODE));
            sceMcSeek(e, fd, o, w);
            0x15
        })
    }

    /// 0x002197e0
    pub fn Delete(e: &mut Engine, m: Ptr, a: Ptr) -> u32 {
        let map = [(-2, 0x13), (-4, 0x1e), (-5, 0x1f), (-6, 0x20)];
        Self::simple(e, m, a, 4, &map, true, |_, _| {}, |e| {
            let (p, s, n) = (e.ri32(m + F_PORT), e.ri32(m + F_SLOT), e.r32(m + F_FILENAME));
            sceMcDelete(e, p, s, n);
            0x14
        })
    }

    /// 0x00219970
    pub fn GetDirectory(e: &mut Engine, m: Ptr, a: Ptr) -> u32 {
        let t = e.rt.loadsave.dirtbl;
        match e.ri32(a + 8) {
            1 => {
                let r = e.ri32(m + F_MCRESULT);
                if r < 0 {
                    match r {
                        -2 => error!(e, m, 0x13),
                        -4 => error!(e, m, 0x1e),
                        -5 => error!(e, m, 0x17),
                        r if r < -9 => error!(e, m, 0x12),
                        _ => {}
                    }
                } else {
                    e.wi32(m + F_DIRCOUNT, r);
                    status!(e, m, 2);
                    e.mem.memset(m + F_DESC, 0, 0x370);
                    // quirk: no bound against the 20 descriptors (0x370 / 0x2c)
                    for i in (0..r as u32).rev() {
                        let s = t + i * 0x40;
                        let d = m + F_DESC + i * 0x2c;
                        let size = e.r32(s + 0x10).wrapping_sub(4);
                        e.w32(s + 0x10, size);
                        let name = e.cstr_at(s + 0x20);
                        write_cstr(e, d, &name);
                        e.w32(d + 0x20, size);
                        for (k, so) in [(0x24u32, 0u32), (0x28, 8)] {
                            let b = |o: u32| e.r8(s + so + o) as u32;
                            let mut v = (b(1) + b(2) * 0x3c + b(3) * 0xe10) / 0xf;
                            v |= b(4) << 0xd;
                            v |= b(5) << 0x12;
                            v |= (e.r16(s + so + 6) as u32).wrapping_sub(0x76c).wrapping_mul(0x40_0000);
                            e.w32(d + k, v);
                        }
                    }
                }
                e.w32(a + 8, 0);
                0
            }
            0 => {
                if busy(e, m) {
                    return 1;
                }
                let n = e.r32(a + 4);
                e.w32(m + F_GETDIRNAME, n);
                set_busy(e, m, 3);
                let (p, s) = (e.ri32(m + F_PORT), e.ri32(m + F_SLOT));
                sceMcGetDir(e, p, s, n, 0, 0x80, t);
                e.w32(a + 8, 1);
                e.w32(m + F_DIRCOUNT, 0);
                1
            }
            _ => {
                e.w32(a + 8, 0);
                0
            }
        }
    }

    /// 0x00219ca0 LSM_WRITE {+4 name, +8 data, +0xc size, +0x10 seek ofs, +0x14 mode, +0x18 no-crc,
    /// +0x1c state}
    pub fn Write(e: &mut Engine, m: Ptr, a: Ptr) -> u32 {
        match e.ri32(a + 0x1c) {
            2 => {
                let p = e.r32(m + F_READBUF);
                e.mem.free(p);
                e.w32(m + F_READBUF, 0);
                e.w32(a + 0x1c, 0);
                0
            }
            1 => {
                // read the file back (into +0x1c)
                e.w32(m + F_MODE, 1);
                let p = e.r32(m + F_READBUF);
                e.mem.free(p);
                let n = e.r32(m + F_SIZE);
                let b = e.mem.malloc(n.max(1));
                e.w32(m + F_READBUF, b);
                EFLoadSaveMgr_BC::RegisterAction(e, m, &[0x1000005, 0], 2);
                EFLoadSaveMgr_BC::RegisterAction(e, m, &[0x1000011, 0], 2);
                EFLoadSaveMgr_BC::RegisterAction(e, m, &[0x3000013, b, n, 0], 2);
                EFLoadSaveMgr_BC::RegisterAction(e, m, &[0x1000010, 0], 2);
                // quirk: GetArgPtr moved with the pushes; the state is written through the old `a`
                e.w32(a + 0x1c, 2);
                1
            }
            0 => {
                let md = e.ri32(a + 0x14);
                e.wi32(m + F_MODE, if md == 0 { 0x203 } else { md });
                let o = e.r32(a + 0x10);
                e.w32(m + F_SEEKOFS, o);
                e.w32(m + F_SEEKMODE, 0);
                let old = e.r32(m + F_WRITEBUF);
                e.mem.free(old);
                let n = e.r32(a + 0xc);
                let b = e.mem.malloc(n.max(1));
                e.w32(m + F_WRITEBUF, b);
                let src = e.r32(a + 8);
                e.mem.memcpy(b, src, n);
                e.w32(m + F_SIZE, n);
                let name = e.r32(a + 4);
                e.w32(m + F_FILENAME, name);
                EFLoadSaveMgr_BC::ComputeCheckSum(e, m);
                EFLoadSaveMgr_BC::RegisterAction(e, m, &[0x1000011, 0], 2);
                if e.ri32(a + 0x18) == 0 {
                    EFLoadSaveMgr_BC::RegisterAction(e, m, &[0x3000012, m + F_CRC, 4, 0], 2);
                }
                EFLoadSaveMgr_BC::RegisterAction(e, m, &[0x3000012, b, n, 0], 2);
                EFLoadSaveMgr_BC::RegisterAction(e, m, &[0x1000010, 0], 2);
                e.w32(a + 0x1c, 2);
                1
            }
            _ => {
                e.w32(a + 0x1c, 0);
                0
            }
        }
    }

    /// 0x00219ee0 LSM_READ {+4 name, +8 size (0 = GetDir size), +0xc ?, +0x10 state}
    pub fn Read(e: &mut Engine, m: Ptr, a: Ptr) -> u32 {
        e.w32(m + F_MODE, 1);
        let name = e.r32(a + 4);
        e.w32(m + F_FILENAME, name);
        let t = e.rt.loadsave.dirtbl;
        match e.ri32(a + 0x10) {
            2 => {
                e.w32(a + 0x10, 0);
                0
            }
            1 => {
                let n = if e.ri32(a + 8) == 0 { e.r32(t + 0x10) } else { e.r32(a + 8) };
                e.w32(m + F_SIZE, n);
                let p = e.r32(m + F_READBUF);
                e.mem.free(p);
                let b = e.mem.malloc(n.max(1));
                e.w32(m + F_READBUF, b);
                EFLoadSaveMgr_BC::RegisterAction(e, m, &[0x1000005, 0], 2);
                EFLoadSaveMgr_BC::RegisterAction(e, m, &[0x1000011, 0], 2);
                EFLoadSaveMgr_BC::RegisterAction(e, m, &[0x3000013, m + F_CRC, 4, 0], 2);
                EFLoadSaveMgr_BC::RegisterAction(e, m, &[0x3000013, b, n, 0], 2);
                EFLoadSaveMgr_BC::RegisterAction(e, m, &[0x1000010, 0], 2);
                e.w32(a + 0x10, 2);
                1
            }
            0 => {
                EFLoadSaveMgr_BC::RegisterAction(e, m, &[0x2000003, name, 0], 2);
                e.w32(a + 0x10, 1);
                1
            }
            _ => 0,
        }
    }

    /// 0x0021a080. State 2 (header + data CRC check, EFEngineController+0x2cc = buffer, flags |= 8 ->
    /// EFEngineController::LoadLevel + EFLoadSaveBuff::UpdateEngineData) is not ported: no module
    /// calls ReadSaveGame (Api_MemCard uses Read); it reports error 0x1d (bad data) instead.
    pub fn ReadSaveGame(e: &mut Engine, m: Ptr, a: Ptr) -> u32 {
        e.w32(m + F_MODE, 1);
        let name = e.r32(a + 4);
        e.w32(m + F_FILENAME, name);
        let t = e.rt.loadsave.dirtbl;
        match e.ri32(a + 0x10) {
            2 => {
                log::warn!("APILoadSave::ReadSaveGame: EFLoadSaveBuff engine-data restore is not ported");
                error!(e, m, 0x1d);
                let p = e.r32(m + F_READBUF);
                e.mem.free(p);
                e.w32(m + F_READBUF, 0);
                0
            }
            1 => {
                let p = e.r32(m + F_READBUF);
                e.mem.free(p);
                let n = e.r32(t + 0x10);
                e.w32(m + F_SIZE, n);
                let b = e.mem.malloc(n.max(1));
                e.w32(m + F_READBUF, b);
                EFLoadSaveMgr_BC::RegisterAction(e, m, &[0x1000011, 0], 2);
                EFLoadSaveMgr_BC::RegisterAction(e, m, &[0x3000013, b, n, 0], 2);
                EFLoadSaveMgr_BC::RegisterAction(e, m, &[0x1000010, 0], 2);
                e.w32(a + 0x10, 2);
                1
            }
            0 => {
                EFLoadSaveMgr_BC::RegisterAction(e, m, &[0x2000003, name, 0], 2);
                e.w32(a + 0x10, 1);
                1
            }
            _ => {
                e.w32(a + 0x10, 0);
                0
            }
        }
    }

    /// 0x0021a250: ICON.SYS -> "icon.sys", then STATIC.ICO -> "static.ico" (Write 0x203, no CRC)
    pub fn TransferIcons(e: &mut Engine, m: Ptr, a: Ptr) -> u32 {
        let st = e.ri32(a + 4);
        let (src, dst) = match st {
            4 => {
                let p = e.r32(m + F_ICONBUF);
                e.mem.free(p);
                e.w32(m + F_ICONBUF, 0);
                e.w32(a + 4, 0);
                return 0;
            }
            1 => ("STATIC.ICO", "static.ico"),
            0 => ("ICON.SYS", "icon.sys"),
            _ => {
                e.w32(a + 4, 0);
                return 0;
            }
        };
        if st == 1 {
            let p = e.r32(m + F_ICONBUF);
            e.mem.free(p);
        }
        let Some(bytes) = e.rt.loadsave.card.disc_file(src) else {
            error!(e, m, 0x18); // DTFopen < 0
            return 0;
        };
        let b = e.mem.malloc(bytes.len().max(1) as u32);
        for (i, v) in bytes.iter().enumerate() {
            e.w8(b + i as u32, *v);
        }
        e.w32(m + F_ICONBUF, b);
        let name = e.cstr(dst);
        let n = bytes.len() as u32;
        EFLoadSaveMgr_BC::RegisterAction(e, m, &[0x7000009, name, b, n, 0, 0x203, 0x1b, 0], 2);
        e.w32(a + 4, if st == 0 { 1 } else { 4 });
        1
    }

    /// 0x0021a460
    pub fn DeleteFile(e: &mut Engine, m: Ptr, a: Ptr) -> u32 {
        match e.ri32(a + 8) {
            1 => {
                e.w32(a + 8, 0);
                0
            }
            0 => {
                let n = e.r32(a + 4);
                e.w32(m + F_FILENAME, n);
                EFLoadSaveMgr_BC::RegisterAction(e, m, &[0x1000014, 0], 2);
                e.w32(a + 8, 1);
                1
            }
            _ => 0,
        }
    }

    /// 0x0021a4e0: GetDir("*"), then delete entries count-1 .. 2 (skips "." and "..")
    pub fn DeleteAllFiles(e: &mut Engine, m: Ptr, a: Ptr) -> u32 {
        match e.ri32(a + 4) {
            1 => {
                let n = e.ri32(m + F_DIRCOUNT);
                if 2 < n {
                    let t = e.rt.loadsave.dirtbl;
                    e.w32(m + F_FILENAME, t + (n as u32 - 1) * 0x40 + 0x20);
                    EFLoadSaveMgr_BC::RegisterAction(e, m, &[0x1000014, 0], 2);
                    e.wi32(m + F_DIRCOUNT, n - 1);
                    return 1;
                }
                e.w32(a + 4, 0);
                0
            }
            0 => {
                let all = e.rt.loadsave.strings + 2;
                EFLoadSaveMgr_BC::RegisterAction(e, m, &[0x2000003, all, 0], 2);
                e.w32(a + 4, 1);
                1
            }
            _ => {
                e.w32(a + 4, 0);
                0
            }
        }
    }

    /// 0x0021a5c0 LSM_IS_MY_FILE_FIT {+4 size, +8 state}
    pub fn IsMyFileFit(e: &mut Engine, m: Ptr, a: Ptr) -> u32 {
        let st = e.ri32(a + 8);
        if st == 3 {
            let s = e.ri32(a + 4);
            let mut need = (if s + 0x3ff < 0 { s + 0x7fe } else { s + 0x3ff }) >> 10;
            // quirk (as in the original): one more cluster when the directory still has entry space
            if e.ri32(m + F_ENTSPACE) != 0 {
                need += 1;
            }
            let pi = Self::info(e, m);
            if e.ri32(pi + 4) < need {
                error!(e, m, 0x14);
            } else {
                status!(e, m, 2);
            }
            e.w32(a + 8, 0);
            return 0;
        }
        if st != 2 {
            if st != 1 {
                if st == 0 {
                    let (p, s) = (e.r32(m + F_PORT), e.r32(m + F_SLOT));
                    EFLoadSaveMgr_BC::RegisterAction(e, m, &[0x300000d, s, p, 0], 2);
                    e.w32(a + 8, 1);
                    return 1;
                }
                e.w32(a + 8, 0);
                return 0;
            }
            let pi = Self::info(e, m);
            match e.ri32(pi) {
                0 => {
                    error!(e, m, 0x12);
                    return 0;
                }
                1 => status!(e, m, 0x10),
                2 => status!(e, m, 0x20),
                3 => status!(e, m, 0x40),
                _ => {}
            }
            if e.ri32(pi) != 2 {
                error!(e, m, 0x21);
                return 0;
            }
            if e.ri32(pi + 8) == 0 {
                error!(e, m, 0x13);
                return 0;
            }
            e.w32(a + 8, 2);
        }
        EFLoadSaveMgr_BC::RegisterAction(e, m, &[0x200000f, 0, 0], 2);
        e.w32(a + 8, 3);
        1
    }

    /// 0x0021b070
    pub fn GetLastCommand(e: &Engine, m: Ptr) -> u32 {
        e.r32(m + F_LASTCMD) & 0xff_ffff
    }

    /// action dispatch of SpecificUpdate; `first` = the post-sceMcSync block (no 4/5/6/0xb cases)
    fn dispatch(e: &mut Engine, m: Ptr, act: u32, first: bool) -> u32 {
        let a = EFLoadSaveMgr_BC::GetArgPtr(e, m);
        match act {
            0xc => Self::GetCardStatus(e, m, a),
            0xd => Self::SetWorkingMemoryCard(e, m, a),
            0xe => Self::FormatCard(e, m, a),
            1 => Self::CreateDirectory(e, m, a),
            2 => Self::ChangeDirectory(e, m, a),
            0xf => Self::GetDirectoryEntrySpace(e, m, a),
            0x10 => Self::OpenFile(e, m, a),
            0x11 => Self::CloseFile(e, m, a),
            0x12 => Self::WriteFile(e, m, a),
            0x13 => Self::ReadFile(e, m, a),
            0x15 => Self::Seek(e, m, a),
            9 => Self::Write(e, m, a),
            10 => Self::Read(e, m, a),
            0x14 => Self::Delete(e, m, a),
            3 => Self::GetDirectory(e, m, a),
            8 => Self::DeleteAllFiles(e, m, a),
            0x19 => Self::ReadSaveGame(e, m, a),
            0x1b => Self::TransferIcons(e, m, a),
            4 if !first => EFLoadSaveMgr_BC::ComputeCheckSum(e, m),
            5 if !first => EFLoadSaveMgr_BC::VerifyCheckSum(e, m),
            6 if !first => Self::DeleteFile(e, m, a),
            0xb if !first => Self::IsMyFileFit(e, m, a),
            // quirk: 7 (DeleteDirectory: vtable +0x1c is never called) and 0x1a (WriteSaveGame:
            // vtable +0x2c is NULL) have no case: the action is dropped as if it succeeded
            _ => 0,
        }
    }

    /// 0x0021a7d0
    pub fn SpecificUpdate(e: &mut Engine, m: Ptr) {
        if e.r32(m + F_FLAGS) & 1 == 0 || !EFLoadSaveMgr_BC::IsAction(e, m) {
            return;
        }
        if sceMcSync(e, 1, m + F_MCCMD, m + F_MCRESULT) == 0 {
            return;
        }
        let cmd = e.r32(m + F_PENDING);
        e.w32(m + F_PENDING, 0);
        if cmd != 0 {
            let f = e.r32(m + F_FLAGS) & !8;
            e.w32(m + F_FLAGS, f);
            e.w32(m + F_LASTCMD, cmd);
            e.w32(m + F_STATUS, 0);
            e.w32(m + F_ERROR, 0);
            if Self::dispatch(e, m, cmd, true) != 0 {
                return;
            }
            if e.r32(m + F_STATUS) & 1 != 0 {
                EFLoadSaveMgr_BC::ProcessError(e, m);
                return;
            }
            EFLoadSaveMgr_BC::UnRegisterAction(e, m);
        }
        let mut more = EFLoadSaveMgr_BC::IsAction(e, m);
        loop {
            if !more {
                if !EFLoadSaveMgr_BC::IsAction(e, m) {
                    EFLoadSaveMgr_BC::CommandsCompleted(e, m);
                }
                return;
            }
            e.w32(m + F_STATUS, 0);
            e.w32(m + F_ERROR, 0);
            let act = EFLoadSaveMgr_BC::GetAction(e, m);
            if Self::dispatch(e, m, act, false) != 0 {
                return;
            }
            if e.r32(m + F_STATUS) & 1 != 0 {
                EFLoadSaveMgr_BC::ProcessError(e, m);
                return;
            }
            EFLoadSaveMgr_BC::UnRegisterAction(e, m);
            more = EFLoadSaveMgr_BC::IsAction(e, m);
        }
    }
}

fn write_cstr(e: &mut Engine, p: Ptr, s: &str) {
    for (i, b) in s.bytes().enumerate() {
        e.w8(p + i as u32, b);
    }
    e.w8(p + s.len() as u32, 0);
}

// ================================================================ libmc replacement
const CARD_KB: i64 = 8000;
const MAX_FDS: usize = 3;

struct Fd {
    path: PathBuf,
    pos: u64,
    mode: i32,
}

#[derive(Default)]
pub struct MemCard {
    root: PathBuf,
    nocard: bool,
    /// card-side current directory ("/" = root), as set by sceMcChdir
    cwd: Vec<String>,
    fds: Vec<Option<Fd>>,
    /// in-flight call: (updates left, cmd, result, EE-memory writes done at completion)
    pending: Option<(u32, i32, i32, Vec<(Ptr, Vec<u8>)>)>,
    inited: bool,
}

impl MemCard {
    fn init(&mut self) {
        if self.inited {
            return;
        }
        self.inited = true;
        self.root = PathBuf::from(std::env::var("GAME_SAVEDIR").unwrap_or_else(|_| "saves/memcard1".into()));
        self.nocard = std::env::var("GAME_NOCARD").map(|v| v == "1").unwrap_or(false);
        self.fds = (0..MAX_FDS).map(|_| None).collect();
        if !self.nocard && !self.root.exists() {
            // a new card from the shop: formatted, empty
            let _ = std::fs::create_dir_all(&self.root);
        }
        log::info!("memory card 1: {} ({})", self.root.display(), if self.nocard { "GAME_NOCARD: empty slot" } else { "PS2 8MB" });
    }
    /// card root (for tests)
    pub fn root(&self) -> &Path {
        &self.root
    }
    fn present(&self, port: i32, slot: i32) -> bool {
        !self.nocard && port == 0 && slot == 0
    }
    fn formatted(&self) -> bool {
        self.root.is_dir()
    }
    fn resolve(&self, p: &str) -> Vec<String> {
        let mut v = if p.starts_with('/') { Vec::new() } else { self.cwd.clone() };
        for c in p.split('/') {
            match c {
                "" | "." => {}
                ".." => {
                    v.pop();
                }
                c => v.push(c.to_string()),
            }
        }
        v
    }
    fn host(&self, v: &[String]) -> PathBuf {
        let mut p = self.root.clone();
        for c in v {
            p.push(c);
        }
        p
    }
    fn used_kb(&self) -> i64 {
        fn walk(p: &Path, root: bool) -> i64 {
            let Ok(rd) = std::fs::read_dir(p) else { return 0 };
            let mut n = 2i64;
            let mut kb = 0;
            for d in rd.flatten() {
                n += 1;
                let Ok(md) = d.metadata() else { continue };
                kb += if md.is_dir() { walk(&d.path(), false) } else { (md.len() as i64 + 1023) / 1024 };
            }
            kb + if root { 0 } else { (n + 1) / 2 }
        }
        walk(&self.root, true)
    }
    fn free_kb(&self) -> i64 {
        (CARD_KB - self.used_kb()).max(0)
    }
    /// DTFopen + DTFread of a disc file (TransferIcons). The ISO is GAME_ISO or the default places.
    fn disc_file(&self, name: &str) -> Option<Vec<u8>> {
        let mut c: Vec<String> = std::env::var("GAME_ISO").into_iter().collect();
        c.extend(["../Scooby Doo Mystery Mayhem.ISO", "Scooby Doo Mystery Mayhem.ISO", "../../Scooby Doo Mystery Mayhem.ISO"].map(String::from));
        c.iter().find_map(|p| crate::engine::iso::IsoFs::open(p).ok()?.read(name).ok())
    }
}

/// libmc latency in manager updates (one per engine frame). Approximation of a real card: a
/// control call takes about a frame; data moves at ~120 KB/s writing, ~480 KB/s reading at 30 fps;
/// a format takes ~3 s.
fn latency(cmd: i32, bytes: usize) -> u32 {
    match cmd {
        6 => 1 + (bytes / 4096) as u32,
        5 => 1 + (bytes / 16384) as u32,
        0x10 => 90,
        0xb => 2, // mkdir
        _ => 1,
    }
}

fn issue(e: &mut Engine, cmd: i32, result: i32, writes: Vec<(Ptr, Vec<u8>)>, bytes: usize) {
    let c = &mut e.rt.loadsave.card;
    if c.pending.is_some() {
        log::warn!("sceMc call {cmd:#x} while busy: rejected");
        return;
    }
    if std::env::var_os("GAME_MCLOG").is_some() {
        eprintln!("mc: sce cmd {cmd:#x} -> {result}");
    }
    c.pending = Some((latency(cmd, bytes), cmd, result, writes));
}

/// sceMcSync(mode 1 = no wait): -1 nothing pending, 0 still running, 1 done (cmd/result stored)
pub fn sceMcSync(e: &mut Engine, _mode: i32, cmd: Ptr, result: Ptr) -> i32 {
    let c = &mut e.rt.loadsave.card;
    match &mut c.pending {
        None => -1,
        Some((n, ..)) if *n > 1 => {
            *n -= 1;
            0
        }
        Some(_) => {
            let (_, k, r, w) = c.pending.take().unwrap();
            for (p, b) in w {
                for (i, v) in b.iter().enumerate() {
                    e.w8(p + i as u32, *v);
                }
            }
            e.wi32(cmd, k);
            e.wi32(result, r);
            1
        }
    }
}

fn le(v: i32) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

pub fn sceMcGetInfo(e: &mut Engine, port: i32, slot: i32, ty: Ptr, free: Ptr, fmt: Ptr) {
    let c = &e.rt.loadsave.card;
    let (r, w) = if !c.present(port, slot) {
        (-10, vec![(ty, le(0)), (free, le(0)), (fmt, le(0))])
    } else if !c.formatted() {
        // quirk of the model: the card is reported "same card" (0) every time; a real first
        // query after insertion returns -1 (new card)
        (-2, vec![(ty, le(2)), (free, le(0)), (fmt, le(0))])
    } else {
        (0, vec![(ty, le(2)), (free, le(c.free_kb() as i32)), (fmt, le(1))])
    };
    issue(e, 1, r, w, 0);
}

fn card_err(c: &MemCard, port: i32, slot: i32) -> Option<i32> {
    if !c.present(port, slot) {
        Some(-10)
    } else if !c.formatted() {
        Some(-2)
    } else {
        None
    }
}

pub fn sceMcFormat(e: &mut Engine, port: i32, slot: i32) {
    let c = &e.rt.loadsave.card;
    let r = if !c.present(port, slot) {
        -10
    } else {
        let _ = std::fs::remove_dir_all(&c.root);
        if std::fs::create_dir_all(&c.root).is_ok() { 0 } else { -1 }
    };
    e.rt.loadsave.card.cwd.clear();
    issue(e, 0x10, r, vec![], 0);
}

pub fn sceMcMkdir(e: &mut Engine, port: i32, slot: i32, name: Ptr) {
    let s = e.cstr_at(name);
    let c = &e.rt.loadsave.card;
    let r = card_err(c, port, slot).unwrap_or_else(|| {
        let v = c.resolve(&s);
        let p = c.host(&v);
        if v.is_empty() || p.exists() || !p.parent().is_some_and(|q| q.is_dir()) {
            -4
        } else if c.free_kb() < 1 {
            -3
        } else if std::fs::create_dir(&p).is_ok() {
            0
        } else {
            -4
        }
    });
    issue(e, 0xb, r, vec![], 0);
}

pub fn sceMcChdir(e: &mut Engine, port: i32, slot: i32, name: Ptr) {
    let s = e.cstr_at(name);
    let c = &mut e.rt.loadsave.card;
    let r = card_err(c, port, slot).unwrap_or_else(|| {
        let v = c.resolve(&s);
        if c.host(&v).is_dir() {
            c.cwd = v;
            0
        } else {
            -4
        }
    });
    issue(e, 0xc, r, vec![], 0);
}

pub fn sceMcGetEntSpace(e: &mut Engine, port: i32, slot: i32, name: Ptr) {
    let s = if name == 0 { String::new() } else { e.cstr_at(name) };
    let c = &e.rt.loadsave.card;
    let r = card_err(c, port, slot).unwrap_or_else(|| match std::fs::read_dir(c.host(&c.resolve(&s))) {
        // free 512-byte entries in the directory's last 1 KB cluster ("." and ".." included)
        Ok(rd) => ((rd.count() + 2) % 2) as i32,
        Err(_) => -4,
    });
    issue(e, 0x12, r, vec![], 0);
}

/// JST (UTC+9) sceMcStDateTime {resv, sec, min, hour, day, month, u16 year}
fn stamp(t: std::io::Result<std::time::SystemTime>) -> [u8; 8] {
    let s = t.ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0) + 9 * 3600;
    let (days, sec) = (s.div_euclid(86400), s.rem_euclid(86400));
    // civil_from_days (H. Hinnant)
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if mo <= 2 { 1 } else { 0 };
    let yb = (y as u16).to_le_bytes();
    [0, (sec % 60) as u8, (sec / 60 % 60) as u8, (sec / 3600) as u8, d as u8, mo as u8, yb[0], yb[1]]
}

fn glob(pat: &str, s: &str) -> bool {
    let (p, s) = (pat.as_bytes(), s.as_bytes());
    fn m(p: &[u8], s: &[u8]) -> bool {
        match p.first() {
            None => s.is_empty(),
            Some(b'*') => m(&p[1..], s) || (!s.is_empty() && m(p, &s[1..])),
            Some(b'?') => !s.is_empty() && m(&p[1..], &s[1..]),
            Some(c) => s.first() == Some(c) && m(&p[1..], &s[1..]),
        }
    }
    m(p, s)
}

/// sceMcGetDir(name, mode 0, maxent, table): entries of the directory part of `name` whose name
/// matches the last component; "." and ".." come first for a wildcard listing.
pub fn sceMcGetDir(e: &mut Engine, port: i32, slot: i32, name: Ptr, _mode: i32, maxent: i32, table: Ptr) {
    let s = e.cstr_at(name);
    let c = &e.rt.loadsave.card;
    let mut w = Vec::new();
    let r = card_err(c, port, slot).unwrap_or_else(|| {
        let mut v = c.resolve(&s);
        let pat = if s.ends_with('/') || v.is_empty() { "*".to_string() } else { v.pop().unwrap() };
        let dir = c.host(&v);
        let Ok(rd) = std::fs::read_dir(&dir) else { return -4 };
        let mut ents: Vec<(String, std::fs::Metadata)> = Vec::new();
        if pat.contains(['*', '?']) {
            if let Ok(md) = std::fs::metadata(&dir) {
                ents.push((".".into(), md.clone()));
                ents.push(("..".into(), md));
            }
        }
        let mut list: Vec<_> = rd.flatten().filter_map(|d| Some((d.file_name().into_string().ok()?, d.metadata().ok()?))).collect();
        list.sort_by(|a, b| a.0.cmp(&b.0));
        ents.extend(list.into_iter().filter(|(n, _)| glob(&pat, n)));
        ents.truncate(maxent.max(0) as usize);
        for (i, (n, md)) in ents.iter().enumerate() {
            let mut b = vec![0u8; 0x40];
            b[0..8].copy_from_slice(&stamp(md.created().or_else(|_| md.modified())));
            b[8..16].copy_from_slice(&stamp(md.modified()));
            let size = if md.is_dir() { std::fs::read_dir(dir.join(n)).map(|r| r.count() as u32 + 2).unwrap_or(2) } else { md.len() as u32 };
            b[0x10..0x14].copy_from_slice(&size.to_le_bytes());
            let attr: u16 = if md.is_dir() { 0x8427 } else { 0x8497 };
            b[0x14..0x16].copy_from_slice(&attr.to_le_bytes());
            let nb = n.as_bytes();
            let k = nb.len().min(31);
            b[0x20..0x20 + k].copy_from_slice(&nb[..k]);
            w.push((table + i as u32 * 0x40, b));
        }
        ents.len() as i32
    });
    issue(e, 0xd, r, w, 0);
}

/// sceMcOpen: mode 1 RDONLY, 2 WRONLY, 3 RDWR, 0x200 CREAT (no truncation, as libmc)
pub fn sceMcOpen(e: &mut Engine, port: i32, slot: i32, name: Ptr, mode: i32) {
    let s = e.cstr_at(name);
    let c = &mut e.rt.loadsave.card;
    let r = card_err(c, port, slot).unwrap_or_else(|| {
        let p = c.host(&c.resolve(&s));
        if p.is_dir() {
            return -5;
        }
        if !p.exists() {
            if mode & 0x200 == 0 || !p.parent().is_some_and(|q| q.is_dir()) {
                return -4;
            }
            if c.free_kb() < 1 {
                return -3;
            }
            if std::fs::write(&p, []).is_err() {
                return -4;
            }
        }
        match c.fds.iter().position(|f| f.is_none()) {
            Some(i) => {
                c.fds[i] = Some(Fd { path: p, pos: 0, mode });
                i as i32
            }
            None => -7,
        }
    });
    issue(e, 2, r, vec![], 0);
}

fn fd_mut(e: &mut Engine, fd: i32) -> Option<&mut Fd> {
    e.rt.loadsave.card.fds.get_mut(usize::try_from(fd).ok()?)?.as_mut()
}

pub fn sceMcClose(e: &mut Engine, fd: i32) {
    let r = match usize::try_from(fd).ok().and_then(|i| e.rt.loadsave.card.fds.get_mut(i)) {
        Some(f @ Some(_)) => {
            *f = None;
            0
        }
        _ => -4,
    };
    issue(e, 3, r, vec![], 0);
}

pub fn sceMcSeek(e: &mut Engine, fd: i32, ofs: i32, whence: i32) {
    let r = match fd_mut(e, fd) {
        None => -4,
        Some(f) => {
            let len = std::fs::metadata(&f.path).map(|m| m.len() as i64).unwrap_or(0);
            let base = match whence {
                1 => f.pos as i64,
                2 => len,
                _ => 0,
            };
            f.pos = (base + ofs as i64).max(0) as u64;
            f.pos as i32
        }
    };
    issue(e, 4, r, vec![], 0);
}

pub fn sceMcRead(e: &mut Engine, fd: i32, buf: Ptr, n: i32) {
    use std::io::{Read, Seek, SeekFrom};
    let mut w = Vec::new();
    let r = match fd_mut(e, fd) {
        None => -4,
        Some(f) if f.mode & 1 == 0 => -5,
        Some(f) => {
            let mut b = Vec::new();
            let ok = std::fs::File::open(&f.path).and_then(|mut h| {
                h.seek(SeekFrom::Start(f.pos))?;
                h.take(n.max(0) as u64).read_to_end(&mut b)
            });
            match ok {
                Ok(k) => {
                    f.pos += k as u64;
                    w.push((buf, b));
                    k as i32
                }
                Err(_) => -4,
            }
        }
    };
    issue(e, 5, r, w, n.max(0) as usize);
}

pub fn sceMcWrite(e: &mut Engine, fd: i32, buf: Ptr, n: i32) {
    use std::io::{Seek, SeekFrom, Write};
    let n = n.max(0) as u32;
    let data: Vec<u8> = (0..n).map(|i| e.r8(buf + i)).collect();
    let free = e.rt.loadsave.card.free_kb();
    let r = match fd_mut(e, fd) {
        None => -4,
        Some(f) if f.mode & 2 == 0 => -5,
        Some(f) => {
            let len = std::fs::metadata(&f.path).map(|m| m.len()).unwrap_or(0);
            let end = (f.pos + n as u64).max(len);
            let grow = ((end + 1023) / 1024) as i64 - ((len + 1023) / 1024) as i64;
            if grow > free {
                -3
            } else {
                let ok = std::fs::OpenOptions::new().write(true).open(&f.path).and_then(|mut h| {
                    h.seek(SeekFrom::Start(f.pos))?;
                    h.write_all(&data)
                });
                match ok {
                    Ok(()) => {
                        f.pos += n as u64;
                        n as i32
                    }
                    Err(_) => -4,
                }
            }
        }
    };
    issue(e, 6, r, vec![], n as usize);
}

pub fn sceMcDelete(e: &mut Engine, port: i32, slot: i32, name: Ptr) {
    let s = e.cstr_at(name);
    let c = &e.rt.loadsave.card;
    let r = card_err(c, port, slot).unwrap_or_else(|| {
        let p = c.host(&c.resolve(&s));
        if p.is_dir() {
            if std::fs::remove_dir(&p).is_ok() { 0 } else { -5 } // not empty
        } else if std::fs::remove_file(&p).is_ok() {
            0
        } else {
            -4
        }
    });
    issue(e, 0xf, r, vec![], 0);
}

// ================================================================ EFApi_LoadSave.cpp (0x0021b090..)
fn reg(e: &mut Engine, a: &[u32]) -> u32 {
    let m = mgr(e);
    // port: the callbacks run as the instance that queued the request (its module's globals)
    if let Some(h) = e.rt.current_instance() {
        e.rt.loadsave.owner = Some(h);
    }
    EFLoadSaveMgr_BC::RegisterAction(e, m, a, 1);
    0
}
pub fn CreateDirectory(e: &mut Engine, d: Ptr) -> u32 {
    reg(e, &[0x2000001, d, 0])
}
pub fn ChangeDirectory(e: &mut Engine, d: Ptr) -> u32 {
    reg(e, &[0x2000002, d, 0])
}
pub fn DeleteDirectory(e: &mut Engine, d: Ptr) -> u32 {
    reg(e, &[0x2000007, d, 0])
}
pub fn Write(e: &mut Engine, name: Ptr, data: Ptr, size: i32, ofs: i32, mode: i32) -> u32 {
    reg(e, &[0x7000009, name, data, size as u32, ofs as u32, mode as u32, 0, 0])
}
pub fn WriteSaveGame(e: &mut Engine, a: Ptr, b: Ptr, c: Ptr) -> u32 {
    reg(e, &[0x400001a, a, b, c, 0])
}
pub fn Read(e: &mut Engine, name: Ptr, size: i32, x: i32) -> u32 {
    reg(e, &[0x400000a, name, size as u32, x as u32, 0])
}
pub fn ReadSaveGame(e: &mut Engine, a: Ptr, b: Ptr) -> u32 {
    let empty = {
        mgr(e);
        e.rt.loadsave.strings
    };
    reg(e, &[0x4000019, a, empty, b, 0])
}
pub fn DeleteFile(e: &mut Engine, n: Ptr) -> u32 {
    reg(e, &[0x2000006, n, 0])
}
pub fn DeleteAllFiles(e: &mut Engine) -> u32 {
    reg(e, &[0x1000008, 0])
}
pub fn GetDirectory(e: &mut Engine, n: Ptr) -> u32 {
    reg(e, &[0x2000003, n, 0])
}
pub fn IsMyFileFit(e: &mut Engine, n: i32) -> u32 {
    reg(e, &[0x200000b, n as u32, 0])
}
pub fn FormatCard(e: &mut Engine) -> u32 {
    reg(e, &[0x100000e, 0])
}
pub fn SetWorkingMemoryCard(e: &mut Engine, port: i32, slot: i32) -> u32 {
    reg(e, &[0x300000d, port as u32, slot as u32, 0])
}
pub fn GetCardStatus(e: &mut Engine) -> u32 {
    reg(e, &[0x100000c, 0])
}
pub fn TransferIcons(e: &mut Engine) -> u32 {
    reg(e, &[0x100001b, 0])
}
fn field(e: &mut Engine, o: u32) -> u32 {
    let m = mgr(e);
    e.r32(m + o)
}
pub fn GetFileDescriptor(e: &mut Engine) -> Ptr {
    mgr(e) + F_DESC
}
pub fn GetLastCommand(e: &mut Engine) -> u32 {
    field(e, F_LASTCMD) & 0xff_ffff
}
pub fn GetLastAction(e: &mut Engine) -> u32 {
    field(e, F_LASTACTION) & 0xff_ffff
}
pub fn GetError(e: &mut Engine) -> u32 {
    field(e, F_ERROR)
}
pub fn GetStatus(e: &mut Engine) -> u32 {
    field(e, F_STATUS)
}
pub fn GetReadBuffer(e: &mut Engine) -> u32 {
    field(e, F_READBUF)
}
pub fn DestroyReadBuffer(e: &mut Engine) -> u32 {
    let m = mgr(e);
    EFLoadSaveMgr_BC::DestroyReadBuffer(e, m);
    0
}
pub fn GetPort1CardInfoBuffer(e: &mut Engine) -> u32 {
    field(e, F_INFOP1)
}
pub fn GetPort2CardInfoBuffer(e: &mut Engine) -> u32 {
    field(e, F_INFOP2)
}
pub fn GetPort1ActiveMemoryCard(e: &mut Engine) -> u32 {
    field(e, F_ACTIVE1)
}
pub fn GetPort2ActiveMemoryCard(e: &mut Engine) -> u32 {
    field(e, F_ACTIVE2)
}
pub fn GetPortActuel(e: &mut Engine) -> u32 {
    field(e, F_PORT)
}
pub fn GetSlotActuel(e: &mut Engine) -> u32 {
    field(e, F_SLOT)
}
pub fn GetTempBuffer(e: &mut Engine, n: i32) -> u32 {
    let m = mgr(e);
    EFLoadSaveMgr_BC::GetUserTempBuffer(e, m, n as u32)
}
pub fn DestroyTempBuffer(e: &mut Engine) -> u32 {
    let m = mgr(e);
    EFLoadSaveMgr_BC::DestroyUserTempBuffer(e, m);
    0
}
pub fn SetErrorCallBack(e: &mut Engine, f: Ptr) -> u32 {
    let m = mgr(e);
    e.w32(m + F_ERRCB, f);
    e.rt.loadsave.owner = e.rt.current_instance();
    0
}
pub fn SetCompleteCallBack(e: &mut Engine, f: Ptr) -> u32 {
    let m = mgr(e);
    e.w32(m + F_DONECB, f);
    e.rt.loadsave.owner = e.rt.current_instance();
    0
}
/// ProductCode from ENGINE.INI (szProductCode_1146)
pub fn GetProductCode(e: &mut Engine) -> Ptr {
    e.cstr("BASLUS-20701")
}
/// GetFreeMem: 0 on PS2
pub fn Null(_e: &mut Engine) -> u32 {
    0
}
/// EFLoadSaveMgr_BC::Update 0x00275c10 (EFEngineController's frame, before the logic):
/// vtable +8 = EFLoadSaveMgr_SP::SpecificUpdate
pub fn Update(e: &mut Engine) {
    if e.rt.loadsave.mgr != 0 {
        let m = e.rt.loadsave.mgr;
        EFLoadSaveMgr_SP::SpecificUpdate(e, m);
    }
}

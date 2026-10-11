//! camera__e50fa8e8: ManageEvent* (FloorEvent/BaseCam*.c) and camera.c
#![allow(non_snake_case, unused_variables, unused_imports, dead_code)]
use super::*;
use crate::engine::*;
use crate::logic::{ExtFn, LResult};

/// ManageEvent003 (0x00017c68)
pub fn ManageEvent003(e: &mut Engine, param_1: i32) -> LResult<()> {
    match param_1 {
        0 => {
        }
        1 => {
            { let a = sh(e, 0x10414); e.w32(a, 0); }
            { let a = sh(e, 0x10408); e.w32(a, 0); }
            { let a = sh(e, 0x1040c); e.w32(a, 0x2a000018); }
            { let a = sh(e, 0x10418); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x1041c); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x10420); e.w32(a, 0x41000000); }
            { let t = sh(e, 0x10408); Camera_BaseFUNC_SetCameraAttract(e, t)?; }
        }
        2 => {
        }
        3 => {
            ReleaseAttractor(e);
        }
        4 => {
        }
        _ => {}
    }
    Ok(())
}

/// ManageEvent004 (0x00017d70)
pub fn ManageEvent004(e: &mut Engine, param_1: i32) -> LResult<()> {
    match param_1 {
        0 => {
        }
        1 => {
            { let a = sh(e, 0x10434); e.w32(a, 0); }
            { let a = sh(e, 0x10428); e.w32(a, 0); }
            { let a = sh(e, 0x1042c); e.w32(a, 0x2a000019); }
            { let a = sh(e, 0x10438); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x1043c); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x10440); e.w32(a, 0x41000000); }
            { let t = sh(e, 0x10428); Camera_BaseFUNC_SetCameraAttract(e, t)?; }
        }
        2 => {
        }
        3 => {
            ReleaseAttractor(e);
        }
        4 => {
        }
        _ => {}
    }
    Ok(())
}

/// ManageEventDefault (0x00017e78)
pub fn ManageEventDefault(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10448); e.w32(a, 2); }
        { let a = sh(e, 0x1046c); e.w32(a, lead(e)); }
        { let a = sh(e, 0x10484); e.w32(a, 0); }
        { let a = sh(e, 0x10488); e.w32(a, 0); }
        { let a = sh(e, 0x1048c); e.w32(a, 0xc0000000); }
        { let t = sh(e, 0x10448); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

pub const ui32flagCrawl: u32 = 0x101f0;
fn crawl(e: &Engine) -> u32 {
    e.r32(sh(e, ui32flagCrawl))
}
fn set_crawl(e: &mut Engine, v: u32) {
    let a = sh(e, ui32flagCrawl);
    e.w32(a, v);
}
fn attr_crawl(e: &mut Engine, fr: Ptr, m0: u32, pos: u32) {
    e.w32(fr, m0);
    let p = sh(e, pos);
    e.w32(fr + 0xc, p);
    e.w32(fr + 4, 0);
    e.w32(fr + 0x10, 0x4000_0000);
    e.w32(fr + 0x14, 0x4000_0000);
    e.w32(fr + 0x18, 0x4100_0000);
}
/// 68501e9f/e50fa8e8 ManageEvent001 (0x000176d0): crawl-space attractor 1
pub fn me_crawl1(e: &mut Engine, param_1: i32, kill: u32) -> LResult<()> {
    let fr = e.alloca(0x20);
    let gb0 = GB0(e);
    match param_1 {
        0 => {
            set_crawl(e, 0x20);
            APIgameobj::KillObject(e, kill);
        }
        2 => {
            if e.r32(gb0 + 0x34) & 1 == 0 {
                let c = crawl(e);
                let g14 = e.r32(gb0 + 0x14) & 0x4000 != 0;
                if c & 0x10 == 0 && (g14 || c & 0x21 != 0) {
                    let mut u = c & 0xffff_fff7;
                    if c & 1 != 0 {
                        u = c & 0xffff_ffd7;
                    }
                    set_crawl(e, u | 0x14);
                    attr_crawl(e, fr, if u & 1 != 0 { 0x4d } else { 4 }, 0x101f8);
                    Camera_BaseFUNC_SetCameraAttract(e, fr)?;
                } else if c & 0x10 == 0 || g14 || c & 0x21 != 0 {
                    if c & 0x10 != 0 && IsAttractorReleased(e) {
                        { let t = crawl(e); set_crawl(e, t & 0xffff_ffee); }
                    }
                } else {
                    set_crawl(e, c & 0xffff_ffef);
                    ReleaseAttractor(e);
                }
            }
        }
        3 => {
            if e.r32(gb0 + 0x34) & 1 == 0 {
                { let t = crawl(e); set_crawl(e, t & 0xffff_ffee); }
                ReleaseAttractor(e);
            }
        }
        _ => {}
    }
    Ok(())
}
/// 68501e9f/e50fa8e8 ManageEvent002 (0x000179d8): crawl-space attractor 2
pub fn me_crawl2(e: &mut Engine, param_1: i32, kill: u32) -> LResult<()> {
    let fr = e.alloca(0x20);
    let gb0 = GB0(e);
    match param_1 {
        0 => { APIgameobj::KillObject(e, kill); }
        2 => {
            let c = crawl(e);
            let g14 = e.r32(gb0 + 0x14) & 0x4000 != 0;
            if c & 8 == 0 && (g14 || c & 4 != 0) {
                attr_crawl(e, fr, if c & 4 != 0 { 0x4d } else { 4 }, 0x10208);
                set_crawl(e, c & 0xffff_ffef | 9);
                Camera_BaseFUNC_SetCameraAttract(e, fr)?;
            } else if c & 8 == 0 || g14 || c & 4 != 0 {
                if c & 8 != 0 && IsAttractorReleased(e) {
                    { let t = crawl(e); set_crawl(e, t & 0xffff_fff3); }
                }
            } else {
                set_crawl(e, c & 0xffff_fff7);
                ReleaseAttractor(e);
            }
        }
        3 => {
            { let t = crawl(e); set_crawl(e, t & 0xffff_fff3); }
            ReleaseAttractor(e);
        }
        _ => {}
    }
    Ok(())
}

/// ManageEvent001 (0x000176d0)
pub fn ManageEvent001(e: &mut Engine, param_1: i32) -> LResult<()> {
    me_crawl1(e, param_1, 0x2a000016)
}
/// ManageEvent002 (0x000179d8)
pub fn ManageEvent002(e: &mut Engine, param_1: i32) -> LResult<()> {
    me_crawl2(e, param_1, 0x2a000017)
}

/// Events[] slot dispatch (0 = ManageEventDefault)
pub fn ME(e: &mut Engine, n: u8, s: i32) -> LResult<()> {
    match n {
        0 => ManageEventDefault(e, s),
        1 => ManageEvent001(e, s),
        2 => ManageEvent002(e, s),
        3 => ManageEvent003(e, s),
        4 => ManageEvent004(e, s),
        _ => Ok(()),
    }
}

/// CameraFUNC_MessageHandler -> EpN_CameraFUNC_MessageHandler
pub fn msg(e: &mut Engine, p1: u32, p2: u32, p3: Ptr) -> LResult<u64> {
    std_msg(e, p1, p2, p3)
}
/// cameraConstructor
pub fn ctor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    std_ctor(e, api, n, 0x17c, false)
}
/// CameraST_FIRSTMOVE (0x0001ef78)
pub fn first(e: &mut Engine) -> LResult<u64> {
    EpN_CameraST_FIRSTMOVE(e)?;
    let g = GCi(e, 3) + 0xc;
    or32(e, g, 0x10_0000);
    send(e, 0x2a00_00e3, 0xde, 0, NULL);
    send(e, 0x2a00_00e9, 0xdf, 0, NULL);
    Ok(0)
}

//! camera__46dad128: ManageEvent* (FloorEvent/BaseCam*.c) and camera.c
#![allow(non_snake_case, unused_variables, unused_imports, dead_code)]
use super::*;
use crate::engine::*;
use crate::logic::{ExtFn, LResult};

/// ManageEventDefault (0x000182b8)
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

/// ManageEvent001 (0x000183a0)
pub fn ManageEvent001(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10498); e.w32(a, 4); }
        { let a = sh(e, 0x104b4); e.w32(a, 0x2a000037); }
        { let a = sh(e, 0x104d4); e.w32(a, 0); }
        { let a = sh(e, 0x104d8); e.w32(a, 0); }
        { let a = sh(e, 0x104dc); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x10498); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent002 (0x00018480)
pub fn ManageEvent002(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x104e8); e.w32(a, 4); }
        { let a = sh(e, 0x10504); e.w32(a, 0x2a000035); }
        { let a = sh(e, 0x10524); e.w32(a, 0); }
        { let a = sh(e, 0x10528); e.w32(a, 0); }
        { let a = sh(e, 0x1052c); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x104e8); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent003 (0x00018560)
pub fn ManageEvent003(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10538); e.w32(a, 4); }
        { let a = sh(e, 0x10554); e.w32(a, 0x2a000036); }
        { let a = sh(e, 0x10574); e.w32(a, 0); }
        { let a = sh(e, 0x10578); e.w32(a, 0); }
        { let a = sh(e, 0x1057c); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x10538); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent004 (0x00018640)
pub fn ManageEvent004(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10588); e.w32(a, 4); }
        { let a = sh(e, 0x105a4); e.w32(a, 0x2a00002d); }
        { let a = sh(e, 0x105c4); e.w32(a, 0); }
        { let a = sh(e, 0x105c8); e.w32(a, 0); }
        { let a = sh(e, 0x105cc); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x10588); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent005 (0x00018720)
pub fn ManageEvent005(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x105d8); e.w32(a, 4); }
        { let a = sh(e, 0x105f4); e.w32(a, 0x2a000030); }
        { let a = sh(e, 0x10614); e.w32(a, 0); }
        { let a = sh(e, 0x10618); e.w32(a, 0); }
        { let a = sh(e, 0x1061c); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x105d8); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent006 (0x00018800)
pub fn ManageEvent006(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10628); e.w32(a, 4); }
        { let a = sh(e, 0x10644); e.w32(a, 0x2a00002f); }
        { let a = sh(e, 0x10664); e.w32(a, 0); }
        { let a = sh(e, 0x10668); e.w32(a, 0); }
        { let a = sh(e, 0x1066c); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x10628); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent007 (0x000188e0)
pub fn ManageEvent007(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10678); e.w32(a, 4); }
        { let a = sh(e, 0x10694); e.w32(a, 0x2a00002e); }
        { let a = sh(e, 0x106b4); e.w32(a, 0); }
        { let a = sh(e, 0x106b8); e.w32(a, 0); }
        { let a = sh(e, 0x106bc); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x10678); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent008 (0x000189c0)
pub fn ManageEvent008(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x106c8); e.w32(a, 4); }
        { let a = sh(e, 0x106e4); e.w32(a, 0x2a000034); }
        { let a = sh(e, 0x10704); e.w32(a, 0); }
        { let a = sh(e, 0x10708); e.w32(a, 0); }
        { let a = sh(e, 0x1070c); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x106c8); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent009 (0x00018aa0)
pub fn ManageEvent009(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10718); e.w32(a, 4); }
        { let a = sh(e, 0x10734); e.w32(a, 0x2a00003a); }
        { let a = sh(e, 0x10754); e.w32(a, 0); }
        { let a = sh(e, 0x10758); e.w32(a, 0); }
        { let a = sh(e, 0x1075c); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x10718); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent010 (0x00018b80)
pub fn ManageEvent010(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10768); e.w32(a, 4); }
        { let a = sh(e, 0x10784); e.w32(a, 0x2a00003b); }
        { let a = sh(e, 0x107a4); e.w32(a, 0); }
        { let a = sh(e, 0x107a8); e.w32(a, 0); }
        { let a = sh(e, 0x107ac); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x10768); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent011 (0x00018c60)
pub fn ManageEvent011(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x107b8); e.w32(a, 4); }
        { let a = sh(e, 0x107d4); e.w32(a, 0x2a000039); }
        { let a = sh(e, 0x107f4); e.w32(a, 0); }
        { let a = sh(e, 0x107f8); e.w32(a, 0); }
        { let a = sh(e, 0x107fc); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x107b8); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent012 (0x00018d40)
pub fn ManageEvent012(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10808); e.w32(a, 4); }
        { let a = sh(e, 0x10824); e.w32(a, 0x2a000038); }
        { let a = sh(e, 0x10844); e.w32(a, 0); }
        { let a = sh(e, 0x10848); e.w32(a, 0); }
        { let a = sh(e, 0x1084c); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x10808); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent020 (0x00018e20)
pub fn ManageEvent020(e: &mut Engine, param_1: i32) -> LResult<()> {
    match param_1 {
        0 => {
        }
        1 => {
            if IsAttractorReleased(e) && !IsAttractorSetted(e) {
                { let a = sh(e, 0x10858); e.w32(a, 0); }
                { let a = sh(e, 0x1085c); e.w32(a, 0x2a000029); }
                { let a = sh(e, 0x10868); e.w32(a, 0x3f000000); }
                { let a = sh(e, 0x1086c); e.w32(a, 0x3f800000); }
                { let a = sh(e, 0x10870); e.w32(a, 0x40a00000); }
                { let t = sh(e, 0x10858); Camera_BaseFUNC_SetCameraAttract(e, t)?; }
            }
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

/// ManageEvent021 (0x00018f50)
pub fn ManageEvent021(e: &mut Engine, param_1: i32) -> LResult<()> {
    let fr = e.alloca(0x20);
    match param_1 {
        0 => {
        }
        1 => {
            { let a = fr + 0xc; e.w32(a, sh(e, 0x101f0)); }
            { let a = fr + 0; e.w32(a, 0); }
            { let a = fr + 4; e.w32(a, 0); }
            { let a = fr + 0x10; e.w32(a, 0x3fc00000); }
            { let a = fr + 0x14; e.w32(a, 0x3f800000); }
            { let a = fr + 0x18; e.w32(a, 0x42c80000); }
            Camera_BaseFUNC_SetCameraAttract(e, fr)?;
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

/// ManageEvent022 (0x00019040)
pub fn ManageEvent022(e: &mut Engine, param_1: i32) -> LResult<()> {
    let fr = e.alloca(0x20);
    match param_1 {
        0 => {
        }
        1 => {
            { let a = fr + 0xc; e.w32(a, sh(e, 0x10200)); }
            { let a = fr + 0; e.w32(a, 0); }
            { let a = fr + 4; e.w32(a, 0); }
            { let a = fr + 0x10; e.w32(a, 0x3fc00000); }
            { let a = fr + 0x14; e.w32(a, 0x3f800000); }
            { let a = fr + 0x18; e.w32(a, 0x42c80000); }
            Camera_BaseFUNC_SetCameraAttract(e, fr)?;
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

/// Events[] slot dispatch (0 = ManageEventDefault)
pub fn ME(e: &mut Engine, n: u8, s: i32) -> LResult<()> {
    match n {
        0 => ManageEventDefault(e, s),
        1 => ManageEvent001(e, s),
        2 => ManageEvent002(e, s),
        3 => ManageEvent003(e, s),
        4 => ManageEvent004(e, s),
        5 => ManageEvent005(e, s),
        6 => ManageEvent006(e, s),
        7 => ManageEvent007(e, s),
        8 => ManageEvent008(e, s),
        9 => ManageEvent009(e, s),
        10 => ManageEvent010(e, s),
        11 => ManageEvent011(e, s),
        12 => ManageEvent012(e, s),
        20 => ManageEvent020(e, s),
        21 => ManageEvent021(e, s),
        22 => ManageEvent022(e, s),
        _ => Ok(()),
    }
}

/// CameraFUNC_SetFog / CameraFUNC_SetFogColor (0x00020d60 / 0x00020d80): empty
fn CameraFUNC_SetFog(e: &mut Engine) {}
fn CameraFUNC_SetFogColor(e: &mut Engine) {}
/// CameraST_FIRSTMOVE (0x00020c90)
pub fn first(e: &mut Engine) -> LResult<u64> {
    let v = (e.r32(GBi(e, 3) + 0x1c) & 1 != 0) as u32;
    let l = LC(e);
    e.w32(l + 0x18c, v);
    EpN_CameraST_FIRSTMOVE(e)?;
    CameraFUNC_SetFog(e);
    Ok(0)
}
/// `SendMessageTo(0x2a000026, 0x1de, p, &[w])`
fn msg1de(e: &mut Engine, p: u32, w: u32) {
    let s = e.alloca(0x10);
    e.w32(s, w);
    send(e, 0x2a00_0026, 0x1de, p, s);
}
/// CameraFUNC_MessageHandler (0x00020da8)
pub fn msg(e: &mut Engine, p1: u32, p2: u32, p3: Ptr) -> LResult<u64> {
    if p1 == 0x98 {
        if room(e) == 1 {
            APInewSound::Play(e, SELF_CAM, 0x8000_0004, 0);
        } else {
            APInewSound::StopSound(e, SELF_CAM, 0x8000_0004);
        }
        if matches!(room(e), 4 | 3 | 2) {
            send(e, 0x2a00_0118, 0x1e5, 0, NULL);
            send(e, 0x2a00_0119, 0x1e5, 0, NULL);
        }
    } else if p1 > 0x98 {
        let g = GBi(e, 3) + 0x1c;
        if p1 == 0xc9 {
            if e.r32(LC(e) + 0x18c) == 0 {
                APIgameobj::SetGroupID(e, SELF_CAM, 0);
                and32(e, g, 0xffff_fffe);
            }
        } else if p1 == 0xd4 && e.r32(g) & 1 != 0 {
            let l = LC(e);
            e.w32(l + 0x18c, 1);
        }
        return std_msg(e, p1, p2, p3);
    } else if p1 == 0x52 {
        let g = GBi(e, 3) + 0x1c;
        let r = e.r32(GB0(e) + 0x24);
        if r == 1 && e.r32(g) & 4 == 0 && p2 == 0 {
            or32(e, g, 4);
            msg1de(e, 4, 0x8000_0007);
            APInewSound::StopSound(e, 0x2a00_0044, 0x8000_0000);
        } else if r == 3 && e.r32(g) & 1 == 0 && p2 == 1 {
            APIgameobj::SetGroupID(e, SELF_CAM, 6);
            or32(e, g, 1);
            msg1de(e, 2, 0x8000_000a);
        } else if r == 4 && e.r32(g) & 2 == 0 && p2 == 1 {
            APIgameobj::SetGroupID(e, SELF_CAM, 6);
            send(e, 0x2a10_0006, 0x2a, 7, NULL);
            or32(e, g, 2);
            msg1de(e, 3, 0x8000_0009);
        }
        if e.r32(GB0(e) + 0x24) != 1 || p2 == 0 {
            CameraFUNC_SetFogColor(e);
        }
        return std_msg(e, p1, p2, p3);
    } else if p1 != 0x54 {
        return std_msg(e, p1, p2, p3);
    }
    // 0x54 (and 0x98 falling through): camera 1000 = look at 0x2a00003c
    if p2 == 1000 {
        cam_user_stack(e, &[(0, 4), (0x1c, 0x2a00_003c), (0x3c, 0), (0x40, 0), (0x44, 0x6020_4202)])?;
        return Ok(0);
    }
    std_msg(e, p1, p2, p3)
}
/// cameraConstructor (0x00021370)
pub fn ctor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    std_ctor(e, api, n, 400, true)
}

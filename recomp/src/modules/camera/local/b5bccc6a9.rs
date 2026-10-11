//! camera__5bccc6a9: ManageEvent* (FloorEvent/BaseCam*.c) and camera.c
#![allow(non_snake_case, unused_variables, unused_imports, dead_code)]
use super::*;
use crate::engine::*;
use crate::logic::{ExtFn, LResult};

/// ManageEventDefault (0x00017c80)
pub fn ManageEventDefault(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10440); e.w32(a, 2); }
        { let a = sh(e, 0x10464); e.w32(a, lead(e)); }
        { let a = sh(e, 0x1047c); e.w32(a, 0); }
        { let a = sh(e, 0x10480); e.w32(a, 0); }
        { let a = sh(e, 0x10484); e.w32(a, 0xc0000000); }
        { let t = sh(e, 0x10440); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent001 (0x00017d68)
pub fn ManageEvent001(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10490); e.w32(a, 4); }
        { let a = sh(e, 0x104ac); e.w32(a, 0x2a00001a); }
        { let a = sh(e, 0x104cc); e.w32(a, 0); }
        { let a = sh(e, 0x104d0); e.w32(a, 0); }
        { let a = sh(e, 0x104d4); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x10490); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent002 (0x00017e48)
pub fn ManageEvent002(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x104e0); e.w32(a, 4); }
        { let a = sh(e, 0x104fc); e.w32(a, 0x2a000018); }
        { let a = sh(e, 0x1051c); e.w32(a, 0); }
        { let a = sh(e, 0x10520); e.w32(a, 0); }
        { let a = sh(e, 0x10524); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x104e0); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent003 (0x00017f28)
pub fn ManageEvent003(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10530); e.w32(a, 4); }
        { let a = sh(e, 0x1054c); e.w32(a, 0x2a000019); }
        { let a = sh(e, 0x1056c); e.w32(a, 0); }
        { let a = sh(e, 0x10570); e.w32(a, 0); }
        { let a = sh(e, 0x10574); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x10530); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent004 (0x00018008)
pub fn ManageEvent004(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10580); e.w32(a, 4); }
        { let a = sh(e, 0x1059c); e.w32(a, 0x2a000010); }
        { let a = sh(e, 0x105bc); e.w32(a, 0); }
        { let a = sh(e, 0x105c0); e.w32(a, 0); }
        { let a = sh(e, 0x105c4); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x10580); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent005 (0x000180e8)
pub fn ManageEvent005(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x105d0); e.w32(a, 4); }
        { let a = sh(e, 0x105ec); e.w32(a, 0x2a000012); }
        { let a = sh(e, 0x1060c); e.w32(a, 0); }
        { let a = sh(e, 0x10610); e.w32(a, 0); }
        { let a = sh(e, 0x10614); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x105d0); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent006 (0x000181c8)
pub fn ManageEvent006(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10620); e.w32(a, 4); }
        { let a = sh(e, 0x1063c); e.w32(a, 0x2a000011); }
        { let a = sh(e, 0x1065c); e.w32(a, 0); }
        { let a = sh(e, 0x10660); e.w32(a, 0); }
        { let a = sh(e, 0x10664); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x10620); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent007 (0x000182a8)
pub fn ManageEvent007(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10670); e.w32(a, 4); }
        { let a = sh(e, 0x1068c); e.w32(a, 0x2a00000f); }
        { let a = sh(e, 0x106ac); e.w32(a, 0); }
        { let a = sh(e, 0x106b0); e.w32(a, 0); }
        { let a = sh(e, 0x106b4); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x10670); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent008 (0x00018388)
pub fn ManageEvent008(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x106c0); e.w32(a, 4); }
        { let a = sh(e, 0x106dc); e.w32(a, 0x2a00000e); }
        { let a = sh(e, 0x106fc); e.w32(a, 0); }
        { let a = sh(e, 0x10700); e.w32(a, 0); }
        { let a = sh(e, 0x10704); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x106c0); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent009 (0x00018468)
pub fn ManageEvent009(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10710); e.w32(a, 4); }
        { let a = sh(e, 0x1072c); e.w32(a, 0x2a000015); }
        { let a = sh(e, 0x1074c); e.w32(a, 0); }
        { let a = sh(e, 0x10750); e.w32(a, 0); }
        { let a = sh(e, 0x10754); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x10710); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent010 (0x00018548)
pub fn ManageEvent010(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10760); e.w32(a, 4); }
        { let a = sh(e, 0x1077c); e.w32(a, 0x2a000016); }
        { let a = sh(e, 0x1079c); e.w32(a, 0); }
        { let a = sh(e, 0x107a0); e.w32(a, 0); }
        { let a = sh(e, 0x107a4); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x10760); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent011 (0x00018628)
pub fn ManageEvent011(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x107b0); e.w32(a, 4); }
        { let a = sh(e, 0x107cc); e.w32(a, 0x2a000014); }
        { let a = sh(e, 0x107ec); e.w32(a, 0); }
        { let a = sh(e, 0x107f0); e.w32(a, 0); }
        { let a = sh(e, 0x107f4); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x107b0); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent012 (0x00018708)
pub fn ManageEvent012(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10800); e.w32(a, 4); }
        { let a = sh(e, 0x1081c); e.w32(a, 0x2a000013); }
        { let a = sh(e, 0x1083c); e.w32(a, 0); }
        { let a = sh(e, 0x10840); e.w32(a, 0); }
        { let a = sh(e, 0x10844); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x10800); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent020 (0x000187e8)
pub fn ManageEvent020(e: &mut Engine, param_1: i32) -> LResult<()> {
    match param_1 {
        0 => {
        }
        1 => {
            if IsAttractorReleased(e) && !IsAttractorSetted(e) {
                { let a = sh(e, 0x10850); e.w32(a, 0); }
                { let a = sh(e, 0x10854); e.w32(a, 0x2a000009); }
                { let a = sh(e, 0x10860); e.w32(a, 0x3f000000); }
                { let a = sh(e, 0x10864); e.w32(a, 0x3f800000); }
                { let a = sh(e, 0x10868); e.w32(a, 0x40a00000); }
                { let t = sh(e, 0x10850); Camera_BaseFUNC_SetCameraAttract(e, t)?; }
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

/// ManageEvent021 (0x00018918)
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

/// ManageEvent022 (0x00018a08)
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

/// ManageEvent013 (0x00018af8)
pub fn ManageEvent013(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10870); e.w32(a, 4); }
        { let a = sh(e, 0x1088c); e.w32(a, 0x2a000020); }
        { let a = sh(e, 0x108ac); e.w32(a, 0); }
        { let a = sh(e, 0x108b0); e.w32(a, 0); }
        { let a = sh(e, 0x108b4); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x10870); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent014 (0x00018bd8)
pub fn ManageEvent014(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x108c0); e.w32(a, 4); }
        { let a = sh(e, 0x108dc); e.w32(a, 0x2a000021); }
        { let a = sh(e, 0x108fc); e.w32(a, 0); }
        { let a = sh(e, 0x10900); e.w32(a, 0); }
        { let a = sh(e, 0x10904); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x108c0); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent015 (0x00018cb8)
pub fn ManageEvent015(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10910); e.w32(a, 4); }
        { let a = sh(e, 0x1092c); e.w32(a, 0x2a000022); }
        { let a = sh(e, 0x1094c); e.w32(a, 0); }
        { let a = sh(e, 0x10950); e.w32(a, 0); }
        { let a = sh(e, 0x10954); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x10910); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent017 (0x00018d98)
pub fn ManageEvent017(e: &mut Engine, param_1: i32) -> LResult<()> {
    match param_1 {
        0 => {
        }
        1 => {
            { let a = sh(e, 0x10960); e.w32(a, 0); }
            { let a = sh(e, 0x10964); e.w32(a, 0x2a00001f); }
            { let a = sh(e, 0x10970); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x10974); e.w32(a, 0x3f800000); }
            { let a = sh(e, 0x10978); e.w32(a, 0x40a00000); }
            { let t = sh(e, 0x10960); Camera_BaseFUNC_SetCameraAttract(e, t)?; }
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

/// ManageEvent018 (0x00018e98)
pub fn ManageEvent018(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x10980); e.w32(a, 4); }
        { let a = sh(e, 0x1099c); e.w32(a, 0x2a00001d); }
        { let a = sh(e, 0x109bc); e.w32(a, 0); }
        { let a = sh(e, 0x109c0); e.w32(a, 0); }
        { let a = sh(e, 0x109c4); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x10980); Camera_BaseFUNC_SetCameraUser(e, t)?; }
    }
    Ok(())
}

/// ManageEvent019 (0x00018f78)
pub fn ManageEvent019(e: &mut Engine, param_1: i32) -> LResult<()> {
    if param_1 == 1 {
        { let a = sh(e, 0x109d0); e.w32(a, 4); }
        { let a = sh(e, 0x109ec); e.w32(a, 0x2a00000d); }
        { let a = sh(e, 0x10a0c); e.w32(a, 0); }
        { let a = sh(e, 0x10a10); e.w32(a, 0); }
        { let a = sh(e, 0x10a14); e.w32(a, 0xe0008202); }
        { let t = sh(e, 0x109d0); Camera_BaseFUNC_SetCameraUser(e, t)?; }
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
        13 => ManageEvent013(e, s),
        14 => ManageEvent014(e, s),
        15 => ManageEvent015(e, s),
        17 => ManageEvent017(e, s),
        18 => ManageEvent018(e, s),
        19 => ManageEvent019(e, s),
        20 => ManageEvent020(e, s),
        21 => ManageEvent021(e, s),
        22 => ManageEvent022(e, s),
        _ => Ok(()),
    }
}

/// CameraFUNC_SetFog / CameraFUNC_SetFogColor (0x00020290 / 0x000202b0): empty
fn CameraFUNC_SetFog(e: &mut Engine) {}
fn CameraFUNC_SetFogColor(e: &mut Engine) {}
/// CameraST_FIRSTMOVE (0x00020200)
pub fn first(e: &mut Engine) -> LResult<u64> {
    EpN_CameraST_FIRSTMOVE(e)?;
    CameraFUNC_SetFog(e);
    Ok(0)
}
/// CameraFUNC_MessageHandler (0x000202d8)
pub fn msg(e: &mut Engine, p1: u32, p2: u32, p3: Ptr) -> LResult<u64> {
    if p1 == 0x54 {
        if p2 == 1000 {
            cam_user_stack(e, &[(0, 4), (0x1c, 0x2a00_0017), (0x3c, 0), (0x40, 0), (0x44, 0x6020_4202)])?;
            return Ok(0);
        }
    } else if p1 == 0x52 {
        if e.r32(GB0(e) + 0x24) != 1 || p2 == 0 {
            CameraFUNC_SetFogColor(e);
        }
    } else if p1 == 0x98 {
        match room(e) {
            1 => {
                APInewSound::Play(e, 0x2a00_002a, 0x8000_0000, 0);
                APInewSound::Play(e, SELF_CAM, 0x8000_0002, 0);
            }
            3 => {
                APInewSound::StopSound(e, 0x2a00_002a, 0x8000_0000);
                APInewSound::StopSound(e, SELF_CAM, 0x8000_0002);
            }
            _ => {
                APInewSound::StopSound(e, SELF_CAM, 0x8000_0002);
            }
        }
    }
    std_msg(e, p1, p2, p3)
}
/// cameraConstructor (0x00020540)
pub fn ctor(e: &mut Engine, api: Ptr, n: i32) -> LResult<()> {
    std_ctor(e, api, n, 0x18c, true)
}

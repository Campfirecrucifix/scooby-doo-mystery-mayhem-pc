//! camera_base.c Camera_BaseFUNC_MessageHandler 0x0001dec0 (all ids) + Destructor (WP-L1).
#![allow(non_snake_case, unused_variables)]
use super::*;

/// goHeroesData row i: {id, leader, follower, team, alt id}
fn hero_row(e: &Engine, i: u32) -> Ptr {
    G(e, goHeroesData) + i * 0x14
}

/// copies hero row `ent` into GB0 (+8 leader, +0xc follower, +0x10 team, +4 id, +0 shown id) and sets GB6
fn apply_row(e: &mut Engine, ent: Ptr, id: u32) {
    let gdb = GDB(e);
    let gb0 = e.r32(gdb);
    let gb6 = e.r32(gdb + 0x14);
    e.w32(gb6, 1);
    let (a, b, c, alt) = (e.r32(ent + 4), e.r32(ent + 8), e.r32(ent + 0xc), e.r32(ent + 0x10));
    e.w32(gb0 + 8, a);
    e.w32(gb0 + 0xc, b);
    e.w32(gb0 + 0x10, c);
    e.w32(gb0 + 4, id);
    e.w32(gb0, if alt == 0 { id } else { alt });
}

/// 0x0001dec0
pub fn Camera_BaseFUNC_MessageHandler(e: &mut Engine, param_1: u32, param_2: u32, param_3: Ptr) -> LResult<u64> {
    let piVar2 = GDB(e);
    let l = LB(e);
    let gb0 = e.r32(piVar2);
    match param_1 {
        0x8f => {
            APIphysic::SetFlags(e, SELF_CAM, 1);
        }
        0x54 => match param_2 {
            1 => Camera_BaseFUNC_SetCameraDeath(e)?,
            3 => {
                let (a, b) = (e.r32(param_3 + 0x34), e.r32(param_3 + 0x38));
                e.w32(l + 0x88, a);
                e.w32(l + 0x8c, b);
                Camera_BaseFUNC_SetCameraBattle(e, param_3)?;
                e.w32(l + 0xf4, 2);
            }
            4 => Camera_BaseFUNC_SetCameraAttract(e, param_3)?,
            _ => Camera_BaseFUNC_SetCameraUser(e, param_3)?,
        },
        0x5a => {
            let (d0, d8) = (e.r32(l + 0xd0), e.r32(l + 0xd8));
            APIphysic::DMDoAction(e, d0, 1, NULL);
            APIphysic::DMDoAction(e, d8, 1, NULL);
        }
        0x5b => {
            // clear colour (CColor, 4 words) -> +0xb8
            for k in 0..4 {
                let v = e.r32(param_3 + k * 4);
                e.w32(l + 0xb8 + k * 4, v);
            }
        }
        0x8e => {
            APIphysic::ResetFlags(e, SELF_CAM, 1);
        }
        0x55 => Camera_BaseFUNC_ResetMode(e, param_2 as i32)?,
        0x57 => {
            if param_2 != 0 {
                let c = e.r32(l + 200);
                APIanim::Connect(e, SELF_CAM, param_2, c);
                APIanim::Play(e, SELF_CAM, param_2, 1, 0.0);
                APIgameobj::RegisterNotify(e, SELF_CAM, param_2, 1, 0xe);
            }
            APIgameobj::SendMessageTo(e, 0x2a10_0000, NULL, 0x1e, 0, NULL);
            e.w32(l + 8, 0);
            e.w32(l + 0x44, 1);
            e.w32(l + 0x4c, 0x6000_0200);
            Camera_BaseFUNC_SetCameraUser(e, l + 8)?;
            let c = APIgameobj::GetContextFlag(e, SELF_CAM);
            e.w32(l + 0x108, c as u32);
            APIgameobj::SetContextFlag(e, SELF_CAM, 0x10);
        }
        0x1b => {
            // mirror the CameraFlex orbit angles (fStack_104 / fStack_100)
            let p = e.alloca(0x80);
            let d0 = e.r32(l + 0xd0);
            APIphysic::DMGetParams(e, d0, p);
            for o in [0x5c, 0x60] {
                let v = -e.rf(p + o);
                e.wf(p + o, v);
            }
            APIphysic::DMSetParams(e, d0, p);
        }
        0x4f => {
            if e.r32(l + 0x15c) == 1 {
                e.w32(l + 0x15c, 0);
                APIphysic::ResetFlags(e, SELF_CAM, 1);
                Camera_BaseFUNC_SetCutSceneMode(e, 8)?;
            }
        }
        0x47 => {
            let mut i = 0u32;
            while i <= 0x11 {
                let ent = hero_row(e, i);
                if e.r32(ent) == param_2 {
                    apply_row(e, ent, param_2);
                    break;
                }
                i += 1;
            }
            // LAB_0001e418
            let gb6 = e.r32(piVar2 + 0x14);
            if e.r32(gb6) != 1 {
                e.w32(gb0 + 4, 1);
                e.w32(gb0, 1);
                e.w32(gb0 + 8, 0x2a00_00f0);
                e.w32(gb0 + 0xc, 0x2a00_00ef);
                e.w32(gb0 + 0x10, 2);
            }
            let f = e.r32(gb0 + 0xc);
            APIgameobj::SendMessageTo(e, f, NULL, 0x7d, 0, NULL);
            let ld = e.r32(gb0 + 8);
            APIgameobj::SendMessageTo(e, ld, NULL, 0x7c, 0, NULL);
            e.w32(l + 0xf0, 1);
            if e.r32(l + 0x110) == 2 {
                return Ok(0);
            }
            let p = e.alloca(0x80); // &uStack_d0
            let d0 = e.r32(l + 0xd0);
            APIphysic::DMGetParams(e, d0, p);
            let ld = e.r32(e.r32(GDB(e)) + 8);
            e.w32(p, ld);
            APIphysic::DMSetParams(e, d0, p);
        }
        0x12 | 0xb4 => {
            // LAB_0001e18c: pause
            APInewSound::PauseSoundEvents(e, 0x100);
            APIinput::InputConfigActivate(e, 4, 0);
            let s = e.r32(gb0 + 0x3c);
            if 9 < s && s < 0xd {
                let w = e.r32(l + 0x138);
                APIhud::WindowSetFlags(e, w, 0x100);
            }
        }
        0x13 | 0xb5 => {
            // LAB_0001e220: resume
            APInewSound::ResumeSoundEvents(e, 0x100);
            APIinput::InputConfigActivate(e, 4, 1);
        }
        0xc6 => {}
        0xc1 => FloorEventForceState(e, 5)?,
        0x97 => {
            e.w32(l + 0xe8, 5);
            if param_3 != NULL && e.r32(param_3) != 0 && e.r32(gb0) != 0 && e.r32(gb0) != e.r32(gb0 + 4) {
                // swap the team to the hero id in GB0+0
                let (u50, u4c, i48) = (e.r32(gb0 + 8), e.r32(gb0 + 0xc), e.r32(gb0 + 0x10));
                let (mut u44, mut u40, mut i3c) = (u50, u4c, i48);
                for h in [u50, u4c] {
                    APIgameobj::SendMessageTo(e, h, NULL, 0x166, 0, NULL);
                    APIgameobj::SendMessageTo(e, h, NULL, 0xd0, 0, NULL);
                }
                APIinput::InputConfigActivate(e, i48 as i32, 0);
                for i in 0..0x12 {
                    let ent = hero_row(e, i);
                    let id = e.r32(ent);
                    if id == e.r32(gb0) {
                        apply_row(e, ent, id);
                        u44 = e.r32(ent + 4);
                        u40 = e.r32(ent + 8);
                        i3c = e.r32(ent + 0xc);
                        break;
                    }
                }
                for h in [u44, u40] {
                    APIgameobj::SendMessageTo(e, h, NULL, 0x165, 0, NULL);
                    APIgameobj::SendMessageTo(e, h, NULL, 0xd1, 0, NULL);
                }
                APIinput::InputConfigActivate(e, i3c as i32, 1);
                let r = e.alloca(8);
                e.w32(r, 1);
                APIgameobj::SendMessageTo(e, 0xff00_0001, r, 0xcd, u44, NULL);
            }
            if e.r32(l + 0x164) != 1 {
                e.w32(l + 0x164, 1);
                Camera_BaseFUNC_FadeInOut(e, 0.051, 0);
            } else {
                Camera_BaseFUNC_FadeInOut(e, 1.0, 0);
            }
        }
        0x151 | 0x152 => {
            // EnvColl (DM 0x22) flag 0x2000000 off (0x151) / on (0x152), mirrored in oRayIntersectIn+0x20
            let p = e.alloca(0x80);
            let d = e.r32(l + 0x16c);
            APIphysic::DMGetParams(e, d, p);
            let ri = G(e, oRayIntersectIn) + 0x20;
            if param_1 == 0x151 {
                let v = e.r32(p + 0x1c) & 0xfdff_ffff;
                e.w32(p + 0x1c, v);
                APIphysic::DMSetParams(e, d, p);
                let v = e.r32(ri) & 0xfdff_ffff;
                e.w32(ri, v);
                APIphysic::PrimResetIntAttribBits(e, SELF_CAM, 0, 5, 0x200_0000);
            } else {
                let v = e.r32(p + 0x1c) | 0x200_0000;
                e.w32(p + 0x1c, v);
                APIphysic::DMSetParams(e, d, p);
                let v = e.r32(ri) | 0x200_0000;
                e.w32(ri, v);
                APIphysic::PrimSetIntAttribBits(e, SELF_CAM, 0, 5, 0x200_0000);
            }
        }
        0x1de => {
            let a = e.r32(param_3);
            let c = e.r32(l + 200);
            APIgame::PlugNPlayCutScene(e, param_2, a, c);
            if LIBApiText::IsTextDisplayed(e) != 0 {
                LIBApiText::KillText(e);
            }
        }
        0x1df => {
            let (a, b) = (e.r32(param_3), e.r32(param_3 + 4));
            let c = e.r32(l + 200);
            APIgame::PlugNPlayCutScene(e, a, b, c);
            if e.r32(param_3 + 8) & 1 != 0 {
                e.w32(l + 0x15c, 1);
            }
        }
        0xc9 => {
            let c = e.alloca(0x10);
            let cam = e.r32(l + 200);
            APIcamera::SetClearColor(e, cam, c);
            APIcamera::EnableFog(e, cam, 0);
        }
        0x98 | 0xcd => {
            e.w32(l + 0x144, 0xffff_fffe);
            let a = G(e, i32ActualFloorEvent);
            e.w32(a, 0xffff_ffff);
            e.w32(l + 0xf0, 0);
        }
        _ => {}
    }
    Ok(0)
}

/// 0x0001f840 (l03_cameraDestructor -> Ep1_CameraFUNC_Destructor -> here; FloorEventDestructor
/// has no caller in l03).
pub fn Camera_BaseFUNC_Destructor(e: &mut Engine, instanceCount: i32) {
    let l = LB(e);
    let c = e.r32(l + 200);
    APIcamera::DestroyCamera(e, c);
    for o in [0x114, 0x124, 0x118, 0x11c, 0x120, 300] {
        let t = e.r32(l + o);
        APIextra::TimerDestroy(e, t);
    }
    if instanceCount == 0 {
        for b in [0, 1, 3, 2, 4, 6] {
            APIgameobj::FreeGameBuffer(e, b);
        }
    }
}

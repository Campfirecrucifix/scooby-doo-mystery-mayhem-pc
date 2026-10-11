//! hero_constructor.c (8 functions). Addresses are module-relative (.text at 0x10000) in scooby_ghost__08a011d0.
use super::builds::*;
use super::*;
use crate::engine::*;
use crate::logic::LResult;

/// `DMAdd(self, kind)` -> local+`slot`, then DMGetParams / the stores / DMSetParams / DMSetContextMask.
fn dm(e: &mut Engine, param_1: Ptr, kind: u32, slot: u32, words: &[(u32, u32)], mask: u32) {
    let h = APIphysic::DMAdd(e, SELF, kind);
    e.w32(param_1 + slot, h);
    let p = e.alloca(0x40);
    APIphysic::DMGetParams(e, e.r32(param_1 + slot), p);
    for &(o, v) in words {
        e.w32(p + o, v);
    }
    APIphysic::DMSetParams(e, e.r32(param_1 + slot), p);
    APIphysic::DMSetContextMask(e, e.r32(param_1 + slot), mask);
}

/// 0x000163c8 (236 bytes) `void heroFUNC_Create_Hero(int param_1)`
/// Group 0 = Scooby ("SCOOBYDOO_COSTUME"), group 1 = Shaggy ("SHAGGY_COSTUME").
pub fn heroFUNC_Create_Hero(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    debug_info(e, "heroFUNC_Create_Hero");
    e.w32(param_1 + 0x178, 2);
    let id = APIgameobj::GetID(e, SELF, 0);
    e.w32(param_1 + 0x17c, id);
    e.w32(param_1 + 0x180, k(e, ID_2a000143));
    e.w32(param_1 + 0x184, 0x250_0000);
    e.w32(param_1 + 0x188, k(e, heroFUNC_Create_Hero_0_1));
    e.w32(param_1 + 0x18c, k(e, heroFUNC_Create_Hero_0_2));
    let name = match g(e, G_heroFUNC_Create_Hero) {
        0 => "SCOOBYDOO_COSTUME",
        _ => "SHAGGY_COSTUME",
    };
    let n = e.cstr(name);
    APIgameobj::DisplayInfo(e, k(e, heroFUNC_Create_Hero_0_3), n, SELF, 0x80f);
    Ok(())
}

/// 0x000164b8 (1432 bytes) `void heroFUNC_Create_Input(int param_1)`
/// One input config per character (local+0x188); the pad is game buffer 0 +0x4c.
pub fn heroFUNC_Create_Input(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let c = e.r32(param_1 + 0x188) as i32;
    let pad = e.r32(GB0(e) + 0x4c) as i32;
    debug_info(e, "FUNC_Create_Input");
    APIinput::InputConfigAdd(e, c);
    APIinput::InputActionAdd(e, c, 6, 0);
    for d in [
        [3, pad, 0, 0, 0, 0x8000],
        [4, pad, 0, 1, 0, 0x8001],
        [0xb, pad, 1, 0, -100, 0xe],
        [0xc, pad, 1, 0, 100, 0xf],
        [0xd, pad, 1, 1, -100, 0xd],
        [0xe, pad, 1, 1, 100, 0xc],
        [0xb, 1, 2, 0, -100, 0xcb],
        [0xc, 1, 2, 0, 100, 0xcd],
        [0xe, 1, 2, 1, 100, 200],
        [0xd, 1, 2, 1, -100, 0xd0],
    ] {
        APIinput::InputDeviceAdd(e, c, 6, d[0], d[1], d[2], d[3], d[4], d[5]);
    }
    APIinput::InputContextAdd(e, SELF, 0x11, c, 6, 2, 0x20000);
    APIinput::InputContextSetState(e, c, 6, 2, 2);
    APIinput::InputContextAdd(e, SELF, 0x10, c, 6, 1, 0x10000);
    APIinput::InputContextSetState(e, c, 6, 1, 1);
    APIinput::InputActionAdd(e, c, 4, 0);
    APIinput::InputDeviceAdd(e, c, 4, 0x20, pad, 0, 2, 0, 5);
    APIinput::InputDeviceAdd(e, c, 4, 0x21, 1, 1, 2, 0, 0x1d);
    APIinput::InputContextAdd(e, SELF, 0x13, c, 4, 0x12, 8);
    APIinput::InputContextAdd(e, SELF, 0x13, c, 4, 0x13, 0x20);
    APIinput::InputContextSetState(e, c, 4, 0x12, 0x80);
    APIinput::InputContextSetState(e, c, 4, 0x13, 0x80);
    APIinput::InputActionAdd(e, c, 0x23, 0);
    APIinput::InputDeviceAdd(e, c, 0x23, 0x26, pad, 0, 2, 0, 0);
    APIinput::InputDeviceAdd(e, c, 0x23, 0x27, 1, 1, 2, 0, 0x39);
    APIinput::InputContextAdd(e, SELF, 0x18, c, 0x23, 0x12, 8);
    APIinput::InputContextSetState(e, c, 0x23, 0x12, 0);
    APIinput::InputConfigActivate(e, c, 1);
    APIinput::InputConfigSetState(e, c, 0);
    Ok(())
}

/// 0x00016a50 (816 bytes) `void heroFUNC_Create_Physic(int param_1)`
pub fn heroFUNC_Create_Physic(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    debug_info(e, "FUNC_Create_Physic");
    APIphysic::CreatePhysic(e, SELF, 0);
    APIphysic::SetFlags(e, SELF, 0xe);
    APIphysic::SetPrimaryPrim(e, SELF, e.r32(param_1 + 0x18c));
    APIphysic::SetWeight(e, SELF, 5.0);
    dm(e, param_1, 8, 0x1b0, &[(0, 0x4248_0000), (4, 0x3f49_0fdb)], 0x10);
    dm(e, param_1, 7, 0x1b4, &[(0, 0x4080_0000), (4, 5)], 0x20);
    dm(e, param_1, 0, 0x1b8, &[(0, 0), (4, 0xc248_0000), (8, 0)], 0x40);
    APIphysic::SetFallingSpeed(e, SELF, -1.5);
    APIphysic::SetFallingDelay(e, SELF, 0.2);
    heroFUNC_CreateMovementsPhysic(e, param_1)?;
    heroFUNC_CreateFollowPhysic(e, param_1)?;
    heroFUNC_CreateHittedPhysic(e, param_1)?;
    Ok(())
}

/// 0x00016d80 (880 bytes) `undefined4 heroFUNC_Create_Game(int param_1)`
/// Returns the last DMSetStateChangeNotifyBits result.
pub fn heroFUNC_Create_Game(e: &mut Engine, param_1: Ptr) -> LResult<u32> {
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0xb, 6);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x16, 0xb);
    APIgameobj::SetGroupID(e, SELF, 7);
    APIgameobj::SetGroupBitMask(e, SELF, 0x1001);
    let sp = e.alloca(0x40);
    APIgameobj::GetShadowParams(e, SELF, sp);
    for (o, v) in [(0, 2), (4, 0), (0x14, 0x80), (0x1c, 0x3e19_999a), (0x20, 0x40f0_0000), (0x24, 0x80), (0x28, 0x3f00_0000)] {
        e.w32(sp + o, v);
    }
    APIgameobj::SetShadowParams(e, SELF, sp);
    APIgameobj::SetLogicFlag(e, SELF, 0x800);
    for o in [500, 0x1f8, 0x1fc, 0x208, 0x200, 0x204, 0x20c, 0x210] {
        let t = APIextra::TimerCreate(e);
        e.w32(param_1 + o, t);
    }
    let b = GB0(e);
    e.w32(b + 0x14, 0);
    e.w32(param_1 + 0x21c, 0xffff_ffff);
    let seed = APIstdlib::MyRand(e);
    APIstdlib::srand(e, seed);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 3, 3);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 8, 4);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 5, 7);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 6, 8);
    APIphysic::DMSetStateChangeNotifyBits(e, e.r32(param_1 + 0x1dc), 2);
    Ok(APIphysic::DMSetStateChangeNotifyBits(e, e.r32(param_1 + 0x1dc), 1))
}

/// 0x000170f0 (476 bytes) `void heroFUNC_Init_Speed(int param_1)`
/// TmpSpeedMap: 8 records {hero id, run speed, walk speed}.
pub fn heroFUNC_Init_Speed(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    let map = G(e, TmpSpeedMap);
    let mut i = 0u32;
    loop {
        if 7 < i {
            debug_info(e, "Initialisation FAILED hero ID unrecognize");
            return Ok(());
        }
        let id = APIgameobj::GetID(e, SELF, 0);
        if id == e.r32(map + i * 0xc) {
            break;
        }
        i += 1;
    }
    let (run, walk) = (e.r32(map + i * 0xc + 4), e.r32(map + i * 0xc + 8));
    e.w32(param_1 + 0x70, run);
    e.w32(param_1 + 0x74, walk);
    debug_info(e, "Initialisation des vitesses de deplacement");
    debug_info_value(e, "fRunSpeed", param_1 + 0x70, 2);
    debug_info_value(e, "fWalkSpeed", param_1 + 0x74, 2);
    Ok(())
}

/// 0x000172d0 (420 bytes) `void heroFUNC_Init_Idle(int param_1)`
/// local+600 = 0; local+0xa4.. = 7 idle records {anim, 0.25, 5, kind}.
pub fn heroFUNC_Init_Idle(e: &mut Engine, param_1: Ptr) -> LResult<()> {
    e.w32(param_1 + 600, 0);
    let recs: [(u32, u32); 7] = [
        (0xfffff831, 0),
        (0xfffff832, 0),
        (0xfffff833, 1),
        (0xfffff834, 2),
        (0xfffff835, 1),
        (0xfffff841, 0),
        (0xfffff836, 0),
    ];
    for (i, (anim, kind)) in recs.into_iter().enumerate() {
        let r = param_1 + 0xa4 + i as u32 * 0x10;
        e.w32(r, anim);
        e.w32(r + 4, 0x3e80_0000);
        e.w32(r + 8, 5);
        e.w32(r + 0xc, kind);
    }
    Ok(())
}

/// 0x00017478 (2072 bytes) `void heroFUNC_Constructor(APIcons *api,int param_2)`
/// First instance: externals, API tables, global data (6 game-buffer pointers: buffers 0 (0x8f8),
/// 1 (0x68), 2 (0x158), 3 (0x40), 4 (8), 6 (0x14, not cleared)). Every instance: local data 0x388.
pub fn heroFUNC_Constructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    if instanceCount == 1 {
        let fns: [(i32, crate::logic::ExtFn); 20] = [
            (0, heroST_FIRSTMOVE),
            (1, heroST_ENTRY_POINT),
            (3, heroCB_SCARED),
            (4, heroCB_HITTED_PARTICLES),
            (5, heroCB_LASTFRAME),
            (6, heroCB_ANIM),
            (7, heroCB_STATECHANGE),
            (8, heroCB_DMSTATECHANGE),
            (9, heroCB_EDGEDETECT),
            (10, heroCB_TIMER),
            (0xb, heroCB_CUTSCENE),
            (0xe, heroCB_PRELOGIC),
            (0xf, heroCB_POSTLOGIC),
            (0x10, heroCB_INPUT_Idle),
            (0x11, heroCB_INPUT_Stick),
            (0x1c, heroCB_INPUT_Switch_Leader),
            (0x13, heroCB_INPUT_Sneak),
            (0x14, heroCB_INPUT_Tome),
            (0x15, heroCB_INPUT_DetectEdge),
            (0x18, heroCB_INPUT_PowerUp_Action),
        ];
        for (i, f) in fns {
            APIcons::SetExternalFunction(e, i, f);
        }
        let tables: [(u32, fn(&mut Engine) -> Ptr); 12] = [
            (pAnim, |e| APIcons::GetApiAnim(e)),
            (pExtra, |e| APIcons::GetApiExtra(e)),
            (pGame, |e| APIcons::GetApiGame(e)),
            (pGameObj, |e| APIcons::GetApiGameObj(e)),
            (pGraphics, |e| APIcons::GetApiGraphics(e)),
            (pHud, |e| APIcons::GetApiHUD(e)),
            (pInput, |e| APIcons::GetApiInput(e)),
            (pMath, |e| APIcons::GetApiMath(e)),
            (pParticle, |e| APIcons::GetApiParticle(e)),
            (pPhysic, |e| APIcons::GetApiPhysic(e)),
            (pSound, |e| APIcons::GetApiNewSound(e)),
            (pStdlib, |e| APIcons::GetApiStdlib(e)),
        ];
        for (slot, get) in tables {
            let v = get(e);
            let a = G(e, slot);
            e.w32(a, v);
        }
        for (slot, name) in [
            (pAnimMngr, "AnimMngr"),
            (pDebug, "debug"),
            (pFunctions, "ApiCFunctions"),
            (pMiscFunc, "MiscFunc"),
            (pSndMngr, "SndMngr"),
            (pText, "ApiText"),
        ] {
            let n = e.cstr(name);
            let v = APIcons::GetApiCustom(e, n);
            let a = G(e, slot);
            e.w32(a, v);
        }
        let slot = G(e, pGlobalDataHero);
        APIcons::GetGlobalData(e, slot, 0x18);
        for (i, (buf, size, clear)) in [(0, 0x8f8, 1), (1, 0x68, 1), (2, 0x158, 1), (3, 0x40, 1), (4, 8, 1), (6, 0x14, 0)]
            .into_iter()
            .enumerate()
        {
            let v = APIgameobj::GetGameBuffer(e, buf, size, clear);
            let gd = GD(e);
            e.w32(gd + i as u32 * 4, v);
        }
    }
    let slot = G(e, pLocalDataHero);
    APIcons::GetLocalData(e, slot, 0x388);
    debug_info(e, "***************************************");
    e.w32(L(e) + 0x260, 0);
    let v = GB0(e) + 0x88;
    debug_info_value(e, "LOAD AT THE START OF THE LEVEL", v, 1);
    debug_info(e, "***************************************");
    heroFUNC_Init_Speed(e, L(e))?;
    heroFUNC_Init_Idle(e, L(e))?;
    heroFUNC_Create_Hero(e, L(e))?;
    heroFUNC_Create_Input(e, L(e))?;
    heroFUNC_Create_Physic(e, L(e))?;
    heroFUNC_Create_Game(e, L(e))?;
    heroFUNC_LinkHandleFootStepFromApi(e)?;
    let pos = e.alloca(0x10);
    APIgameobj::GetPosition(e, SELF, ALL, pos);
    debug_info_value(e, "init", pos, 4);
    LIBMiscFunc::MiscFunc_ChangeClipping(e, SELF, 2, 0, 0);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x14, 0xe);
    APIgameobj::RegisterNotify(e, SELF, 0xffff_ffff, 0x15, 0xf);
    // C: heroFUNC_StartAnim(0 /* time 0.0 */, 2, 2, 0)
    let a = heroFUNC_StartAnim(e, 0.0, 2, 2, 0)?;
    e.w32(L(e) + 0x218, a as i32 as u32);
    Ok(())
}

/// 0x00017d18 (336 bytes) `void heroFUNC_Destructor(int param_1)`
pub fn heroFUNC_Destructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    for o in [500, 0x1f8, 0x1fc, 0x200, 0x204, 0x208, 0x20c, 0x210] {
        let t = e.r32(L(e) + o);
        APIextra::TimerDestroy(e, t);
    }
    if instanceCount == 0 {
        APIgameobj::FreeGameBuffer(e, 0);
    }
    Ok(())
}

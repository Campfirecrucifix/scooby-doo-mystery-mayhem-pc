//! ApiSectorClipping custom library (`Game Logic Modules/Globals/Lib/Api_SectorClipping/apisectorclipping/
//! ApiSectorClipping.c`, 25 functions, one build 7198aca5). Registers the 7-slot table "ApiSectorClipping"
//! (0x1c bytes). Objects ("clients") register a 0xc-byte record; the library shows/hides them (blend alpha
//! fade + in/out messages) when the current sector changes, from a per-sector visibility map.
//!
//! Client record: +0 object id, +4 f32 alpha, +8 u16 sector, +0xa u16 flags (2 = find the sector by a
//! floor ray, 4 = no fade (messages only), 8 = fading in, 0x20 = visible). The Register parameter is typed
//! `APIcons*` in the decompile (SetExternalFunction = +0, GetGlobalData = +4, GetLocalData = +8).
//! Sector map: `count` pointers to u16 lists of the sectors visible from that sector, ending with 0xffff
//! (sorted ascending by SortSectorArray at Initialize).
//!
//! ABI note: ApiImpl_Initialize's first argument is the fade time in $f12 (all sectormanager builds pass
//! 1.0); Ghidra shifted the integer parameters by one. Real order: (fade f32, map, count, current sector,
//! extractor, msg in, msg out, flag). Flag 1 = sector clipping disabled: every entry point is a no-op
//! (every EP1 sectormanager build passes 1).
//!
//! Module globals: pCons 0x10000, pExtra 4, pGame 8, pGameObj c, pGraphics 10, pPhysic 14, pRegLib 18,
//! pStdlib 1c, pApiList 20, pApiSectClip 24, ui32Flag 28, ApiData (fade) 30, DAT_34 msg in, 38 msg out,
//! 3c count, 40 map, 44 extractor (code token), 48 current sector, 4c pending list, 50 client-list buffer,
//! 54 fade list, 58 initialized.
#![allow(non_snake_case, non_upper_case_globals, unused_variables, clippy::too_many_arguments)]
use super::{api_list, lib_data, set_global};
use crate::engine::core::apicons as cons;
use crate::engine::*;
use crate::logic::{CodeFn, LResult, ModuleDef};

pub static MODULE: ModuleDef = ModuleDef {
    name: "apisectorclipping",
    data_size: 0x60,
    ctor: apisectorclippingConstructor,
    dtor: apisectorclippingDestructor,
    register: Some(apisectorclippingRegister),
    init: &[],
    build_init: None,
};

const pCons: u32 = 0x10000;
const pExtra: u32 = 0x10004;
const pGame: u32 = 0x10008;
const pGameObj: u32 = 0x1000c;
const pGraphics: u32 = 0x10010;
const pPhysic: u32 = 0x10014;
const pRegLib: u32 = 0x10018;
const pStdlib: u32 = 0x1001c;
const pApiList: u32 = 0x10020;
const pApiSectClip: u32 = 0x10024;
const ui32Flag: u32 = 0x10028;
const ApiData: u32 = 0x10030;
const MsgIn: u32 = 0x10034;
const MsgOut: u32 = 0x10038;
const Count: u32 = 0x1003c;
const Map: u32 = 0x10040;
const Extractor: u32 = 0x10044;
const Current: u32 = 0x10048;
const Pending: u32 = 0x1004c;
const Clients: u32 = 0x10050;
const FadeList: u32 = 0x10054;
const Initialized: u32 = 0x10058;

fn g(e: &Engine, a: u32) -> u32 {
    e.r32(lib_data(e, &MODULE, a))
}
fn sg(e: &mut Engine, a: u32, v: u32) {
    set_global(e, &MODULE, a, v)
}
fn assert(e: &mut Engine, c: u32, msg: &str) {
    let s = e.cstr(msg);
    APIextra::Assert(e, c, s);
}
fn flags(e: &Engine, c: Ptr) -> u16 {
    e.r16(c + 10)
}
fn set_flags(e: &mut Engine, c: Ptr, v: u16) {
    e.w16(c + 10, v)
}
/// client list of sector `s`
fn sector_list(e: &Engine, s: u32) -> i32 {
    e.ri32(g(e, Clients) + s * 4)
}
/// `if (pApiList == 0) pApiList = GetApiCustom("ApiList")`
fn ensure_api_list(e: &mut Engine) {
    if g(e, pApiList) == 0 {
        let n = e.cstr("ApiList");
        let v = cons::GetApiCustom(e, n);
        sg(e, pApiList, v);
    }
}
fn extract(e: &mut Engine, attrib: u32) -> u16 {
    let t = g(e, Extractor);
    match e.code_fn(t) {
        CodeFn::Word(f) => f(e, attrib) as u16,
        _ => panic!("ApiSectorClipping: sector extractor is not a word function"),
    }
}
/// RayIntersectIn from the object's position + 1 m, direction (0,-11,0), masks +0x20/+0x24;
/// returns (CheckRayIntersection result, RayIntersectOut+0x24 floor attribute)
fn floor_ray(e: &mut Engine, obj: u32, m20: u32, m24: u32) -> (u32, u32) {
    let ray = e.alloca(0x40);
    let out = e.alloca(0x60);
    APIgameobj::GetPosition(e, obj, 0x7fff_ffff, ray);
    let y = e.rf(ray + 4) + 1.0;
    e.wf(ray + 4, y);
    e.wf(ray + 0x10, -11.0);
    e.w32(ray + 0x20, m20);
    e.w32(ray + 0x24, m24);
    let r = APIphysic::CheckRayIntersection(e, ray, out);
    (r, e.r32(out + 0x24))
}

/// 0x00010ec8
pub fn apisectorclippingRegister(e: &mut Engine, api: Ptr) {
    sg(e, pCons, api);
    for (a, v) in [
        (pExtra, cons::GetApiExtra(e)),
        (pGame, cons::GetApiGame(e)),
        (pGameObj, cons::GetApiGameObj(e)),
        (pGraphics, cons::GetApiGraphics(e)),
        (pPhysic, cons::GetApiPhysic(e)),
        (pRegLib, cons::GetApiRegLib(e)),
        (pStdlib, cons::GetApiStdlib(e)),
    ] {
        sg(e, a, v);
    }
    let n = e.cstr("ApiList");
    let v = cons::GetApiCustom(e, n);
    sg(e, pApiList, v);
    let n = e.cstr("ApiSectorClipping");
    let v = APIRegLib::RegisterLib(e, n, 0x1c);
    sg(e, pApiSectClip, v);
    // the 7 slot writes: LIBApiSectorClipping dispatches straight to the ApiImpl_* functions here
    sg(e, Map, 0);
    sg(e, Count, 0);
    let t = e.code_ptr(CodeFn::Word(DefaultSectorExtractor));
    sg(e, Extractor, t);
    sg(e, MsgOut, 0);
    sg(e, MsgIn, 0);
    sg(e, Current, 0);
    sg(e, Clients, 0);
    sg(e, Initialized, 0);
}

/// 0x000110d0 two lists (pending registrations, fade list); own object: clipping distance -1, flags 0x6bf
pub fn apisectorclippingConstructor(e: &mut Engine, api: Ptr, instanceCount: i32) -> LResult<()> {
    ensure_api_list(e);
    let l = api_list::NewList(e);
    sg(e, Pending, l as u32);
    let l = api_list::NewList(e);
    sg(e, FadeList, l as u32);
    let p = e.alloca(0x40);
    APIgameobj::GetClippingParams(e, 0x2a00_0000, p);
    e.w32(p + 8, 0x6bf);
    e.w32(p + 0xc, 0x6bf);
    e.w32(p + 4, 0xbf80_0000);
    APIgameobj::SetClippingParams(e, 0x2a00_0000, p, 0);
    Ok(())
}

/// 0x000111d8
pub fn apisectorclippingDestructor(e: &mut Engine, instanceCount: i32) -> LResult<()> {
    APIgameobj::FreeGameBuffer(e, 40000);
    let n = e.cstr("ApiSectorClipping");
    APIRegLib::UnRegisterLib(e, n);
    Ok(())
}

/// 0x00011238
pub fn ApiImpl_Initialize(e: &mut Engine, fade: f32, map: Ptr, count: u32, cur: u16, extractor: u32, msg_in: u32, msg_out: u32, flag: u32) {
    sg(e, ui32Flag, flag);
    if flag == 1 {
        let l = g(e, Pending) as i32;
        api_list::DeleteList(e, l);
    } else {
        let c = (g(e, Initialized) == 0) as u32;
        assert(e, c, "ApiSectorClipping::Initialize - Already initialized");
        assert(e, ((cur as u32) < count) as u32, "ApiSectorClipping::Initialize - Invalid sector ID");
        ensure_api_list(e);
        if extractor != 0 {
            sg(e, Extractor, extractor);
        }
        let a = lib_data(e, &MODULE, ApiData);
        e.wf(a, fade);
        sg(e, MsgIn, msg_in);
        sg(e, MsgOut, msg_out);
        sg(e, Count, count);
        sg(e, Map, map);
        sg(e, Current, cur as u32);
        for i in 0..count {
            let s = e.r32(map + i * 4);
            SortSectorArray(e, s);
        }
        let b = APIgameobj::GetGameBuffer(e, 40000, count << 2, 0);
        sg(e, Clients, b);
        assert(e, b, "ApiSectorClipping::Initialize - Couldn't allocate client lists");
        for i in 0..count {
            let l = api_list::NewList(e);
            e.wi32(b + i * 4, l);
        }
        sg(e, Initialized, 1);
        let mut i = 0;
        while i < api_list::GetSize(e, g(e, Pending) as i32) {
            let c = GetListItemAt(e, g(e, Pending) as i32, i as i32);
            ApiImpl_Register(e, c);
            i += 1;
        }
        let l = g(e, Pending) as i32;
        api_list::DeleteList(e, l);
    }
    sg(e, Pending, 0);
}

/// 0x000115a0
pub fn ApiImpl_Register(e: &mut Engine, c: Ptr) {
    if g(e, ui32Flag) == 1 {
        return;
    }
    ensure_api_list(e);
    if e.r32(c + 8) & 0x20000 != 0 {
        let (r, attrib) = floor_ray(e, e.r32(c), 0, 0xfc00);
        let s = if r == 2 { extract(e, attrib) } else { 0 };
        e.w16(c + 8, s);
    }
    if g(e, Initialized) == 0 {
        api_list::AddItem(e, g(e, Pending) as i32, c);
        return;
    }
    let vis = IsSectorVisible(e, e.r16(c + 8) as i16);
    let l = sector_list(e, e.r16(c + 8) as u32);
    api_list::AddItem(e, l, c);
    if vis == 0 {
        NotifyClientOut(e, c);
    } else {
        NotifyClientIn(e, c);
    }
    if e.r32(c + 8) & 0x40000 == 0 {
        if api_list::GetItemPos(e, g(e, FadeList) as i32, c as i32) == -1 {
            e.w32(c + 4, if vis == 0 { 0 } else { 0x3f80_0000 });
        } else if vis == 0 {
            let f = flags(e, c) & 0xfff7;
            set_flags(e, c, f);
        } else {
            let f = flags(e, c) | 8;
            set_flags(e, c, f);
        }
    }
}

/// 0x00011880
pub fn ApiImpl_Unregister(e: &mut Engine, c: Ptr) {
    if g(e, ui32Flag) == 1 {
        return;
    }
    if g(e, Initialized) == 0 {
        api_list::RemoveItem(e, g(e, Pending) as i32, c as i32);
        return;
    }
    let l = sector_list(e, e.r16(c + 8) as u32);
    api_list::RemoveItem(e, l, c as i32);
    api_list::RemoveItem(e, g(e, FadeList) as i32, c as i32);
    if APIgameobj::GetID(e, e.r32(c), NULL) != 0x7fff_fffe {
        if IsSectorVisible(e, e.r16(c + 8) as i16) == 0 {
            NotifyClientIn(e, c);
        }
        if flags(e, c) & 4 == 0 {
            APIgraphics::SetGameObjBlendColor(e, e.r32(c), 0, 1.0, 1.0, 1.0, 1.0);
        }
    }
}

/// 0x00011a28
pub fn ApiImpl_ChangeSector(e: &mut Engine, c: Ptr, s: u16) {
    if g(e, ui32Flag) == 1 {
        return;
    }
    ensure_api_list(e);
    if g(e, Initialized) == 0 {
        api_list::RemoveItem(e, g(e, Pending) as i32, c as i32);
        api_list::AddItem(e, g(e, Pending) as i32, c);
        return;
    }
    let l = sector_list(e, e.r16(c + 8) as u32);
    api_list::RemoveItem(e, l, c as i32);
    api_list::RemoveItem(e, g(e, FadeList) as i32, c as i32);
    e.w16(c + 8, s);
    let l = sector_list(e, s as u32);
    api_list::AddItem(e, l, c);
    if IsSectorVisible(e, s as i16) == 0 {
        NotifyClientOut(e, c);
    }
}

/// 0x00011be0 sector change -> fade lists; then steps every fading client by dt / fade time
pub fn ApiImpl_Update(e: &mut Engine, s: u16) {
    if g(e, ui32Flag) == 1 {
        return;
    }
    let i = g(e, Initialized);
    assert(e, i, "ApiSectorClipping::Update - Initialize was not called");
    let c = ((s as u32) < g(e, Count)) as u32;
    assert(e, c, "ApiSectorClipping::Update - Invalid sector ID");
    if g(e, Current) != s as u32 {
        ResolveSectorVisibility(e, s, SectorListInFadeCallback, SectorListOutFadeCallback);
    }
    let mut i = 0;
    while i < api_list::GetSize(e, g(e, FadeList) as i32) as i32 {
        let mut advance = true;
        let c = GetListItemAt(e, g(e, FadeList) as i32, i);
        let step = APIgame::GetFrameDeltaTime(e) / e.rf(lib_data(e, &MODULE, ApiData));
        let a = ((flags(e, c) >> 2 ^ 1) & 1) as u32;
        assert(e, a, "ApiSectorClipping::Update - Object should not be in fade list");
        if flags(e, c) & 8 == 0 {
            let v = e.rf(c + 4) - step;
            e.wf(c + 4, v);
            if v < 0.0 {
                e.w32(c + 4, 0);
                NotifyClientOut(e, c);
                api_list::RemoveItemAt(e, g(e, FadeList) as i32, i);
                advance = false;
            }
        } else {
            let v = e.rf(c + 4) + step;
            e.wf(c + 4, v);
            if 1.0 < v {
                e.w32(c + 4, 0x3f80_0000);
                api_list::RemoveItemAt(e, g(e, FadeList) as i32, i);
                advance = false;
            }
        }
        let alpha = e.rf(c + 4);
        APIgraphics::SetGameObjBlendColor(e, e.r32(c), 0, 1.0, 1.0, 1.0, alpha);
        if advance {
            i += 1;
        }
    }
}

/// 0x00011ee0 ends every fade at once, then switches to sector `s` without fading
pub fn ApiImpl_Reset(e: &mut Engine, s: u16) {
    if g(e, ui32Flag) == 1 {
        return;
    }
    let mut i = 0;
    while i < api_list::GetSize(e, g(e, FadeList) as i32) as i32 {
        let c = GetListItemAt(e, g(e, FadeList) as i32, i);
        assert(e, 1, "ApiSectorClipping::Reset - Object should not be in fade list");
        if flags(e, c) & 8 == 0 {
            e.w32(c + 4, 0);
            APIgraphics::SetGameObjBlendColor(e, e.r32(c), 0, 1.0, 1.0, 1.0, 0.0);
            NotifyClientOut(e, c);
        } else {
            e.w32(c + 4, 0x3f80_0000);
            APIgraphics::SetGameObjBlendColor(e, e.r32(c), 0, 1.0, 1.0, 1.0, 1.0);
        }
        i += 1;
    }
    api_list::ClearList(e, g(e, FadeList) as i32);
    if s as u32 != g(e, Current) {
        ResolveSectorVisibility(e, s, SectorListInNoFadeCallback, SectorListOutNoFadeCallback);
    }
}

/// 0x000120f8 floor-ray sector of a client with flag 2 (0 otherwise)
pub fn ApiImpl_GetCurrentSector(e: &mut Engine, c: Ptr) -> u16 {
    if g(e, ui32Flag) == 1 || flags(e, c) & 2 == 0 {
        return 0;
    }
    let (_, attrib) = floor_ray(e, e.r32(c), 8, 0);
    extract(e, attrib)
}

/// 0x00012220
fn DefaultSectorExtractor(e: &mut Engine, a: u32) -> u32 {
    a & 0xffff
}

/// 0x00012258
fn GetListItemAt(e: &mut Engine, l: i32, i: i32) -> Ptr {
    api_list::GetItemAt(e, l, i)
}

/// 0x000122b8
fn NotifyClientIn(e: &mut Engine, c: Ptr) -> u32 {
    let f = flags(e, c) | 0x20;
    set_flags(e, c, f);
    let m = g(e, MsgIn);
    APIgameobj::SendMessageTo(e, e.r32(c), NULL, m, 0, c)
}

/// 0x00012338
fn NotifyClientOut(e: &mut Engine, c: Ptr) -> u32 {
    let f = flags(e, c) & 0xffdf;
    set_flags(e, c, f);
    let m = g(e, MsgOut);
    APIgameobj::SendMessageTo(e, e.r32(c), NULL, m, 0, c)
}

/// 0x000123b8
fn NotifyClientListIn(e: &mut Engine, l: i32) {
    let mut i = 0;
    while i < api_list::GetSize(e, l) as i32 {
        let c = GetListItemAt(e, l, i);
        NotifyClientIn(e, c);
        i += 1;
    }
}

/// 0x00012468
fn NotifyClientListOut(e: &mut Engine, l: i32) {
    let mut i = 0;
    while i < api_list::GetSize(e, l) as i32 {
        let c = GetListItemAt(e, l, i);
        NotifyClientOut(e, c);
        i += 1;
    }
}

/// 0x00012518 quirk: a client is only queued when its alpha is exactly 0 or 1 (not already fading)
fn AddClientListToFadeList(e: &mut Engine, l: i32, fade_in: i32) {
    let mut i = 0;
    while i < api_list::GetSize(e, l) as i32 {
        let c = GetListItemAt(e, l, i);
        if flags(e, c) & 4 == 0 {
            let f = if fade_in == 0 { flags(e, c) & 0xfff7 } else { flags(e, c) | 8 };
            set_flags(e, c, f);
            let a = e.rf(c + 4);
            if a == 0.0 || a == 1.0 {
                api_list::AddItem(e, g(e, FadeList) as i32, c);
            }
        } else if fade_in == 0 {
            NotifyClientOut(e, c);
        }
        i += 1;
    }
}

/// 0x00012690
fn SetClientListBlendColor(e: &mut Engine, alpha: f32, l: i32) {
    let mut i = 0;
    while i < api_list::GetSize(e, l) as i32 {
        let c = GetListItemAt(e, l, i);
        e.wf(c + 4, alpha);
        if flags(e, c) & 4 == 0 {
            APIgraphics::SetGameObjBlendColor(e, e.r32(c), 0, 1.0, 1.0, 1.0, alpha);
        }
        i += 1;
    }
}

/// 0x00012790 exchange sort of a 0xffff-terminated u16 list (restarts at index 1 after each swap)
fn SortSectorArray(e: &mut Engine, p: Ptr) {
    let mut i = 0u32;
    while e.r16(p + i * 2 + 2) != 0xffff {
        let (a, b) = (e.r16(p + i * 2), e.r16(p + i * 2 + 2));
        if b < a {
            e.w16(p + i * 2 + 2, a);
            e.w16(p + i * 2, b);
            i = 0;
        }
        i += 1;
    }
}

/// 0x000128b8
fn SectorListInFadeCallback(e: &mut Engine, l: i32) {
    AddClientListToFadeList(e, l, 1);
    NotifyClientListIn(e, l);
}
/// 0x00012918
fn SectorListInNoFadeCallback(e: &mut Engine, l: i32) {
    SetClientListBlendColor(e, 1.0, l);
    NotifyClientListIn(e, l);
}
/// 0x00012980
fn SectorListOutFadeCallback(e: &mut Engine, l: i32) {
    AddClientListToFadeList(e, l, 0);
}
/// 0x000129d0
fn SectorListOutNoFadeCallback(e: &mut Engine, l: i32) {
    SetClientListBlendColor(e, 0.0, l);
    NotifyClientListOut(e, l);
}

/// 0x00012a30 merge walk of the old and new (sorted) visible lists: sectors only in the new one get
/// `cb_in`, only in the old one `cb_out`
fn ResolveSectorVisibility(e: &mut Engine, s: u16, cb_in: fn(&mut Engine, i32), cb_out: fn(&mut Engine, i32)) {
    let map = g(e, Map);
    let mut old = e.r32(map + g(e, Current) * 4);
    let mut new = e.r32(map + s as u32 * 4);
    loop {
        let o = e.r16(old);
        if o == 0xffff {
            while e.r16(new) != 0xffff {
                let l = sector_list(e, e.r16(new) as u32);
                cb_in(e, l);
                new += 2;
            }
            sg(e, Current, s as u32);
            return;
        }
        let n = e.r16(new);
        if n == 0xffff {
            break;
        }
        if o == n {
            old += 2;
            new += 2;
        } else if o < n {
            let l = sector_list(e, o as u32);
            cb_out(e, l);
            old += 2;
        } else {
            let l = sector_list(e, n as u32);
            cb_in(e, l);
            new += 2;
        }
    }
    while e.r16(old) != 0xffff {
        let l = sector_list(e, e.r16(old) as u32);
        cb_out(e, l);
        old += 2;
    }
    sg(e, Current, s as u32);
}

/// 0x00012ca8
fn IsSectorVisible(e: &mut Engine, s: i16) -> u32 {
    let p = e.r32(g(e, Map) + g(e, Current) * 4);
    let mut i = 0;
    loop {
        if e.r16(p + i * 2) as i16 == -1 {
            return 0;
        }
        let v = e.r16(p + i * 2) as i16;
        i += 1;
        if v == s {
            return 1;
        }
    }
}

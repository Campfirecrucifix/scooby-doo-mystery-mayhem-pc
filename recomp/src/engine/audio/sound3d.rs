//! EFSoundEvent_BC volume/pan, 1:1: ComputeVolumeAndPan (0x00243620) and AdjustSoundToDistance
//! (0x00242af0). Event data (EFSoundEventData, this+0x90): +0x08 mode (0 = 2D, 1 = 3D no pan,
//! 2 = 3D panned, 3 = 3D panned + inverted pan/volume), +0x10 min distance, +0x14 volume at min,
//! +0x18 max distance, +0x1c volume at max, +0x20 cut-off distance, +0x28 group mask, +0x32 u8
//! "virtualize when out of range". Instance fields keep their EFSoundEvent_BC offsets.
//! Positions are supplied by the caller (RwFrameGetLTM of listener/camera/emitter).
#![allow(non_snake_case)]
use super::tables::EFSoundEventData;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventInst {
    /// +0x40 distance volume factor
    pub dist_vol: f32,
    /// +0x44 current (smoothed) volume, -1 = not yet set
    pub vol_cur: f32,
    /// +0x48 target volume
    pub vol_target: f32,
    /// +0x70 SBH volume (int, 0..0x3fff)
    pub base_vol: i32,
    /// +0x74 SBH pan (-1..1)
    pub pan_sbh: f32,
    /// +0x80 / +0x84 final left / right volume
    pub left: f32,
    pub right: f32,
    /// +0x88 current pan, -2 = not yet set
    pub pan_cur: f32,
    /// +0x8c target pan
    pub pan_target: f32,
    /// +0xb0 virtual (out of range, voice released)
    pub virt: bool,
}
impl Default for EventInst {
    fn default() -> Self {
        EventInst { dist_vol: 0.0, vol_cur: -1.0, vol_target: 0.0, base_vol: 0, pan_sbh: 0.0, left: 0.0, right: 0.0, pan_cur: -2.0, pan_target: 0.0, virt: false }
    }
}

/// RwMatrix parts used: `at` (+0x20) and `pos` (+0x30).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Frame { pub at: [f32; 3], pub pos: [f32; 3] }

/// Listener = SetListener object (gpoNewAudioMgr+0x11334) else camera 0; pan reference = camera 0.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Geometry { pub listener: Option<Frame>, pub camera: Option<Frame>, pub emitter: Option<[f32; 3]> }

/// Inputs from the manager: gpoNewAudioMgr+0x11348 global volume, GetGroupVolume(+0x28),
/// vtbl 0xd0 GetSEMasterVolume, +0x430 audio engine mode (1 = mono).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MixParams { pub global: f32, pub group: f32, pub master: f32, pub mono: bool }

fn approach(cur: &mut f32, target: f32, sentinel: f32, le: bool) {
    if *cur == sentinel { *cur = target; return; }
    let d = target - *cur;
    // pan uses `0.1 <= diff`; the 2D volume path uses `0.1 < diff` (decomp)
    let big = |x: f32| if le { 0.1 <= x } else { 0.1 < x };
    if *cur < target { *cur = if big(d) { *cur + 0.1 } else { target } }
    else if target < *cur { *cur = if big(-d) { *cur - 0.1 } else { target } }
    else { *cur = target }
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }

pub fn AdjustSoundToDistance(i: &mut EventInst, d: &EFSoundEventData, g: &Geometry) {
    let (Some(l), Some(em)) = (g.listener.or(g.camera), g.emitter) else { i.dist_vol = 0.0; return };
    let v = sub(l.pos, em);
    let dist = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    let mode = d.word(0x08);
    if d.fword(0x20) < dist {
        i.dist_vol = 0.0;
        if d.w[0x30 / 4] >> 16 & 0xff != 0 { i.virt = true; }
        return;
    }
    i.virt = false;
    if mode == 2 || mode == 3 {
        let c = g.camera.unwrap_or(l);
        let r = sub(em, c.pos);
        // A2: the facing is camera 0's LTM `at` (0x00242af0 iVar3), not the listener's
        let mut a = r[0].atan2(r[2]) - c.at[0].atan2(c.at[2]);
        if 3.1415927 < a { a -= 6.2831855 } else if a < -3.1415927 { a += 6.2831855 }
        i.pan_target = -(a as f64).sin() as f32;
        if mode == 3 { i.pan_target *= -1.0; }
    } else {
        i.pan_target = -1.0;
    }
    let (min, vmin, max, vmax) = (d.fword(0x10), d.fword(0x14), d.fword(0x18), d.fword(0x1c));
    i.dist_vol = if dist < min { vmin } else if max < dist { vmax } else {
        let t = ((dist - min) / (max - min)).min(1.0);
        vmin - t * (vmin - vmax).max(0.0)
    };
    if mode == 3 { i.dist_vol = 1.0 - i.dist_vol; }
}

pub fn ComputeVolumeAndPan(i: &mut EventInst, d: &EFSoundEventData, g: &Geometry, m: &MixParams) {
    let mode = d.word(0x08);
    let clamp = |x: f32| if x <= 1.0 { if x < 0.0 { 0.0 } else { x } } else { 1.0 };
    if mode == 0 {
        i.virt = false;
        i.pan_target = i.pan_sbh;
        approach(&mut i.pan_cur, i.pan_target, -2.0, true);
        i.vol_target = m.global * m.group;
        approach(&mut i.vol_cur, i.vol_target, -1.0, false);
        let f = clamp(i.vol_cur * m.master);
        i.left = i.base_vol as f32 * f;
        i.right = i.base_vol as f32 * f;
    } else if mode <= 3 {
        i.dist_vol = 1.0;
        AdjustSoundToDistance(i, d, g);
        i.vol_target = i.dist_vol * m.global * m.group;
        approach(&mut i.vol_cur, i.vol_target, -1.0, true);
        let f = clamp(i.vol_cur * m.master);
        i.dist_vol = i.base_vol as f32 * f;
        i.left = i.dist_vol;
        i.right = i.dist_vol;
        if mode == 1 { i.pan_target = i.pan_sbh; }
        approach(&mut i.pan_cur, i.pan_target, -2.0, true);
    } else {
        i.left = 0.0; i.right = 0.0; i.vol_cur = 0.0;
    }
    if m.mono { i.pan_cur = 0.0; }
    i.left = -((i.pan_cur - 1.0) * i.left) / 2.0;
    i.right = ((i.pan_cur + 1.0) * i.right) / 2.0;
}

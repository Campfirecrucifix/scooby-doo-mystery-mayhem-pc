//! P34: PC settings in the game: settings.toml applied at start-up and live (window mode, vsync,
//! FPS cap, MSAA, volumes, controls) + the "PC SETTINGS" page of the pause Options menu.
//! The page is host-side (the PS2 panel has no such page): SELECT on the Game Options page opens it,
//! it is drawn in the panel's own font / colours over the panel background (the original rows are
//! hidden while it is open), navigated like the panel (up/down, left/right changes, L1/R1 = tab,
//! TRIANGLE / START = back, saved on close) and plays the panel's own menu sounds.
use bevy::prelude::*;
use scooby_recomp::engine::hud::{EFFont, HudDrawCmd, HudTex};
use scooby_recomp::engine::input::{PadState, pad};
use scooby_recomp::engine::settings::{self, FPS_CAPS, MSAA, RESOLUTIONS, Settings, WindowMode};
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
use std::sync::Mutex;

/// settings + file (None path = headless: defaults, never written)
static CUR: Mutex<Option<(Settings, Option<std::path::PathBuf>)>> = Mutex::new(None);
/// live category gains (f32 bits): music, sfx, voice
static GAINS: [AtomicU32; 3] = [AtomicU32::new(0x3f80_0000), AtomicU32::new(0x3f80_0000), AtomicU32::new(0x3f80_0000)];
/// bumped on every change (systems re-apply)
static GEN: AtomicU32 = AtomicU32::new(1);

pub enum Cat {
    Music,
    Sfx,
    Voice,
}

pub fn gain(c: Cat) -> f32 {
    f32::from_bits(GAINS[c as usize].load(Relaxed))
}

fn set_gains(s: &Settings) {
    let a = &s.audio;
    for (i, v) in [a.music, a.sfx, a.voice].into_iter().enumerate() {
        GAINS[i].store(a.gain(v).to_bits(), Relaxed);
    }
}

/// main(): load (or create / migrate) settings.toml; headless runs use the defaults (P41 R8 test hook:
/// unless GAME_SETTINGS_IO=1 with an explicit SCOOBY_SETTINGS scratch file: rebinding round trips).
pub fn init(headless: bool) -> Settings {
    let io = std::env::var_os("GAME_SETTINGS_IO").is_some() && std::env::var_os("SCOOBY_SETTINGS").is_some_and(|p| !p.is_empty());
    let s = if headless && !io {
        *CUR.lock().unwrap() = Some((Settings::default(), None));
        Settings::default()
    } else {
        let (s, path, err) = settings::load();
        if let Some(e) = err {
            eprintln!("settings: {e} (using defaults)");
        }
        *CUR.lock().unwrap() = Some((s.clone(), Some(path)));
        *BASE.lock().unwrap() = Some(s.clone());
        s
    };
    set_gains(&s);
    s
}

pub fn get() -> Settings {
    CUR.lock().unwrap().as_ref().map(|c| c.0.clone()).unwrap_or_default()
}

fn update(f: impl FnOnce(&mut Settings)) {
    if let Some(c) = CUR.lock().unwrap().as_mut() {
        f(&mut c.0);
        set_gains(&c.0);
    }
    GEN.fetch_add(1, Relaxed);
}

/// P34 R9: the settings as last read from / written to the file (3-way merge base for `save`)
static BASE: Mutex<Option<Settings>> = Mutex::new(None);

fn save() {
    let mut g = CUR.lock().unwrap();
    let Some((s, Some(p))) = g.as_mut() else { return };
    let mut base = BASE.lock().unwrap();
    // the launcher (or P22 rtgi::save_setting: global_ray_tracing) may have written the file
    // meanwhile: keep their keys, only the keys changed in-game win (per key, P34 R9)
    if let Some(disk) = std::fs::read_to_string(&*p).ok().and_then(|t| Settings::parse(&t).ok()) {
        let mut b = base.clone().unwrap_or_else(|| s.clone());
        if base.is_none() {
            b.global_ray_tracing = disk.global_ray_tracing;
        }
        *s = s.merged_keys(&b, &disk);
    }
    if let Err(e) = s.save(p) {
        eprintln!("settings: {e}");
        // P39 R9: a full disk / read-only settings folder: the options panel says so (draw), the old file stays
        scooby_recomp::crashlog::note("SETTINGS NOT SAVED", &e);
        SAVE_FAILED.store(true, Relaxed);
    } else {
        SAVE_FAILED.store(false, Relaxed);
    }
    *base = Some(s.clone());
}

/// P39 R9: the last in-game settings save failed (shown under the PC SETTINGS entry row)
static SAVE_FAILED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Window mode for the settings value. P34 R4: never `MonitorSelection::Current` (no current monitor
/// while the window is created: bevy_winit panicked "Unable to get monitor."); exclusive fullscreen
/// starts as borderless on the primary monitor and `fit_fullscreen` switches it to a real video mode.
pub fn window_mode(m: WindowMode) -> bevy::window::WindowMode {
    use bevy::window::MonitorSelection;
    match m {
        WindowMode::Windowed => bevy::window::WindowMode::Windowed,
        WindowMode::Borderless | WindowMode::Fullscreen => bevy::window::WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
    }
}

/// P34 R4: exclusive fullscreen = the primary monitor's mode closest to the resolution setting
/// (desktop size when "default"), once per settings change; no monitor / no modes = stay borderless.
fn fit_fullscreen(mut done: Local<u32>, mut windows: Query<&mut Window>, monitors: Query<(&bevy::window::Monitor, Has<bevy::window::PrimaryMonitor>)>) {
    use bevy::window::{MonitorSelection, VideoModeSelection};
    let g = GEN.load(Relaxed);
    let s = get();
    if *done == g || s.graphics.window_mode != WindowMode::Fullscreen || monitors.is_empty() {
        return;
    }
    *done = g;
    let Some((m, _)) = monitors.iter().max_by_key(|(_, p)| *p) else { return };
    let modes: Vec<(u32, u32, u32)> = m.video_modes.iter().map(|v| (v.physical_size.x, v.physical_size.y, v.refresh_rate_millihertz)).collect();
    let Some(i) = scooby_recomp::engine::settings::pick_video_mode(&modes, s.graphics.resolution, (m.physical_width, m.physical_height)) else {
        eprintln!("settings: no fullscreen video modes on {:?}, staying borderless", m.name);
        return;
    };
    for mut w in &mut windows {
        if w.mode != bevy::window::WindowMode::Windowed {
            w.mode = bevy::window::WindowMode::Fullscreen(MonitorSelection::Primary, VideoModeSelection::Specific(m.video_modes[i]));
        }
    }
}

pub fn present_mode(vsync: bool) -> bevy::window::PresentMode {
    if vsync { bevy::window::PresentMode::AutoVsync } else { bevy::window::PresentMode::AutoNoVsync }
}

pub struct PcSettingsPlugin;

impl Plugin for PcSettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (apply_live, apply_msaa, fit_fullscreen.after(apply_live), window_icon)).add_systems(Last, fps_cap);
    }
}

/// window mode / size / vsync + controls follow the settings (only when they changed; F11 stays a
/// free toggle until the next change)
fn apply_live(mut seen: Local<u32>, mut windows: Query<&mut Window>, ctl: Option<ResMut<crate::game_input::ControlsRes>>) {
    let g = GEN.load(Relaxed);
    if *seen == g {
        return;
    }
    let first = *seen == 0;
    *seen = g;
    let s = get();
    if let Some(mut c) = ctl {
        c.0 = s.controls.clone();
    }
    if first {
        return; // start-up values are set in main() (CLI flags win)
    }
    for mut w in &mut windows {
        let m = window_mode(s.graphics.window_mode);
        if w.mode != m && !(s.graphics.window_mode == WindowMode::Fullscreen && matches!(w.mode, bevy::window::WindowMode::Fullscreen(..))) {
            w.mode = m;
        }
        let p = present_mode(s.graphics.vsync);
        if w.present_mode != p {
            w.present_mode = p;
        }
        if let (Some((x, y)), WindowMode::Windowed) = (s.graphics.resolution, s.graphics.window_mode) {
            if (w.resolution.width() as u32, w.resolution.height() as u32) != (x, y) {
                w.resolution.set(x as f32, y as f32);
            }
        }
    }
}

/// P34 R5: the window / taskbar icon = the exe's own icon resource (res/app.rc id 1; winit registers
/// its window class without one, so the taskbar showed the generic icon). Posted (not sent): the
/// window belongs to the main thread, systems may run elsewhere.
fn window_icon(mut done: Local<u32>, windows: Query<(), With<bevy::window::PrimaryWindow>>) {
    if *done > 600 || windows.is_empty() {
        return;
    }
    *done += 1;
    #[cfg(windows)]
    unsafe {
        type Hwnd = *mut core::ffi::c_void;
        unsafe extern "system" {
            fn GetModuleHandleW(n: *const u16) -> Hwnd;
            fn LoadImageW(inst: Hwnd, name: *const u16, kind: u32, cx: i32, cy: i32, flags: u32) -> Hwnd;
            fn GetSystemMetrics(i: i32) -> i32;
            fn EnumWindows(f: unsafe extern "system" fn(Hwnd, isize) -> i32, lp: isize) -> i32;
            fn GetWindowThreadProcessId(h: Hwnd, pid: *mut u32) -> u32;
            fn IsWindowVisible(h: Hwnd) -> i32;
            fn PostMessageW(h: Hwnd, msg: u32, wp: usize, lp: isize) -> i32;
        }
        unsafe extern "system" fn each(h: Hwnd, found: isize) -> i32 {
            let mut pid = 0;
            unsafe { GetWindowThreadProcessId(h, &mut pid) };
            if pid == std::process::id() && unsafe { IsWindowVisible(h) } != 0 {
                unsafe { *(found as *mut Hwnd) = h };
                return 0;
            }
            1
        }
        let mut hwnd: Hwnd = std::ptr::null_mut();
        EnumWindows(each, &mut hwnd as *mut Hwnd as isize);
        if hwnd.is_null() {
            return;
        }
        let inst = GetModuleHandleW(std::ptr::null());
        // IMAGE_ICON, LR_SHARED; SM_CXICON 11 / SM_CXSMICON 49; WM_SETICON ICON_BIG 1 / ICON_SMALL 0
        for (kind, m) in [(1usize, 11), (0, 49)] {
            let sz = GetSystemMetrics(m);
            let ic = LoadImageW(inst, 1 as *const u16, 1, sz, sz, 0x8000);
            if !ic.is_null() {
                PostMessageW(hwnd, 0x80, kind, ic as isize);
            }
        }
        *done = u32::MAX;
    }
}

/// MSAA of the game cameras (the port's default Sample4 ones; explicit Off passes stay off)
fn apply_msaa(mut q: Query<&mut Msaa, Added<Camera>>) {
    let want = match get().graphics.msaa {
        1 => Msaa::Off,
        2 => Msaa::Sample2,
        8 => Msaa::Sample8,
        _ => return,
    };
    for mut m in &mut q {
        if *m == Msaa::Sample4 {
            *m = want;
        }
    }
}

fn fps_cap(mut last: Local<Option<std::time::Instant>>) {
    let cap = get().graphics.fps_cap;
    if cap > 0 {
        if let Some(t) = *last {
            let frame = std::time::Duration::from_secs_f64(1.0 / cap as f64);
            if let Some(rest) = frame.checked_sub(t.elapsed()) {
                std::thread::sleep(rest);
            }
        }
    }
    *last = Some(std::time::Instant::now());
}

// ---------------------------------------------------------------------------------------------
// in-game page

pub static PAGE: Mutex<Page> = Mutex::new(Page { open: false, tab: 0, sel: 0, prev: 0, prev_stick: (0, 0), hold: false, dirty: false });

pub struct Page {
    pub open: bool,
    tab: usize,
    sel: usize,
    prev: u16,
    prev_stick: (i8, i8),
    /// swallow the pad until every button is released (the close press must not reach the panel)
    hold: bool,
    dirty: bool,
}

/// W fallback: second V drawn this fraction of a V width to the right
const VV_STEP: f32 = 0.55;
const DIGIT_K: f32 = 1.5;
static MISSING: Mutex<std::collections::BTreeSet<char>> = Mutex::new(std::collections::BTreeSet::new());

const TABS: [&str; 5] = ["GRAPHICS", "AUDIO", "CONTROLS", "BUTTONS", "REMASTER"];
const BUTTONS_TAB: usize = 3;
/// BUTTONS tab: the action being rebound (index into the [keyboard] actions)
static ACTION: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
/// press-to-bind in progress: 0 no, 1 next key / mouse button, 2 next gamepad button
static CAPTURE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
/// P41 R8: a press-to-bind just ended: the buttons still held then are not fresh presses (the key just
/// bound to cross must not re-open the capture on its next step)
static CAPTURE_END: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn capturing() -> u8 {
    CAPTURE.load(Relaxed)
}

/// end of a press-to-bind (game_input): `name` (None = cancelled) becomes the action's first binding
pub fn capture_done(name: Option<String>) {
    let mode = CAPTURE.swap(0, Relaxed);
    CAPTURE_END.store(true, Relaxed);
    let Some(n) = name else { return };
    update(|s| {
        let Some(a) = s.controls.keyboard.get(ACTION.load(Relaxed)).map(|a| a.0.clone()) else { return };
        let list = if mode == 2 { &mut s.controls.gamepad } else { &mut s.controls.keyboard };
        scooby_recomp::engine::input::host::rebind(list, &a, &n);
    });
    save();
}

/// a binding / action name in the panel font: no lowercase, "KeyA" -> "A", "Mouse.Left" -> "MOUSE LEFT"
fn bind_label(n: &str) -> String {
    let n = n.strip_prefix("Key").filter(|r| r.len() == 1).or_else(|| n.strip_prefix("Digit")).unwrap_or(n);
    n.replace(['_', '.'], " ").to_uppercase()
}

fn pct(v: f32) -> String {
    format!("{}", (v * 100.0).round() as i32)
}
fn onoff(b: bool) -> String {
    if b { "ON".into() } else { "OFF".into() }
}
fn step<T: PartialEq + Copy>(list: &[T], cur: T, d: i32) -> T {
    let i = list.iter().position(|&x| x == cur).unwrap_or(0) as i32;
    list[(i + d).clamp(0, list.len() as i32 - 1) as usize]
}
fn stepf(v: f32, d: i32, inc: f32, lo: f32, hi: f32) -> f32 {
    (((v + d as f32 * inc) / inc).round() * inc).clamp(lo, hi)
}

/// tabs shown: the shipped (original-game) build hides REMASTER unless a remaster build is installed
fn ntabs() -> usize {
    if remaster_installed() { TABS.len() } else { TABS.len() - 1 }
}

/// remaster build next to the running exe (main never contains the remaster code)
fn remaster_installed() -> bool {
    std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())).is_some_and(|d| settings::remaster_exe(&d).is_some())
}

/// rows of a tab: (label, value, restart note). `d` != 0 changes the row `sel`.
fn rows(s: &mut Settings, tab: usize, sel: usize, d: i32) -> Vec<(String, String, bool)> {
    let mut out = Vec::new();
    let acts: Vec<String> = s.controls.keyboard.iter().map(|a| a.0.clone()).collect();
    let mut row = |i: usize, label: &str, f: &mut dyn FnMut(&mut Settings, i32) -> String, restart: bool| {
        let v = f(s, if i == sel { d } else { 0 });
        out.push((label.to_string(), v, restart));
    };
    match tab {
        0 => {
            row(0, "WINDOW MODE", &mut |s, d| {
                s.graphics.window_mode = step(&WindowMode::ALL, s.graphics.window_mode, d);
                s.graphics.window_mode.name().to_uppercase()
            }, false);
            row(1, "RESOLUTION", &mut |s, d| {
                let cur = s.graphics.resolution.unwrap_or((1280, 720));
                if d != 0 {
                    s.graphics.resolution = Some(step(&RESOLUTIONS, cur, d));
                }
                let (w, h) = s.graphics.resolution.unwrap_or((1280, 720));
                format!("{w}X{h}")
            }, false);
            row(2, "VSYNC", &mut |s, d| {
                if d != 0 { s.graphics.vsync = !s.graphics.vsync; }
                onoff(s.graphics.vsync)
            }, false);
            row(3, "FPS CAP", &mut |s, d| {
                s.graphics.fps_cap = step(&FPS_CAPS, s.graphics.fps_cap, d);
                if s.graphics.fps_cap == 0 { "OFF".into() } else { s.graphics.fps_cap.to_string() }
            }, false);
            row(4, "ANTI-ALIASING", &mut |s, d| {
                s.graphics.msaa = step(&MSAA, s.graphics.msaa, d);
                if s.graphics.msaa == 1 { "OFF".into() } else { format!("{}X MSAA", s.graphics.msaa) }
            }, true);
        }
        1 => {
            row(0, "MASTER VOLUME", &mut |s, d| { s.audio.master = stepf(s.audio.master, d, 0.1, 0.0, 1.0); pct(s.audio.master) }, false);
            row(1, "MUSIC VOLUME", &mut |s, d| { s.audio.music = stepf(s.audio.music, d, 0.1, 0.0, 1.0); pct(s.audio.music) }, false);
            row(2, "SOUND FX VOLUME", &mut |s, d| { s.audio.sfx = stepf(s.audio.sfx, d, 0.1, 0.0, 1.0); pct(s.audio.sfx) }, false);
            row(3, "VOICE VOLUME", &mut |s, d| { s.audio.voice = stepf(s.audio.voice, d, 0.1, 0.0, 1.0); pct(s.audio.voice) }, false);
            row(4, "MUTE", &mut |s, d| { if d != 0 { s.audio.mute = !s.audio.mute; } onoff(s.audio.mute) }, false);
        }
        2 => {
            row(0, "STICK DEAD SPOT", &mut |s, d| { s.controls.deadzone = stepf(s.controls.deadzone, d, 0.05, 0.0, 0.95); pct(s.controls.deadzone) }, false);
            row(1, "TRIGGER THRESHOLD", &mut |s, d| { s.controls.trigger_threshold = stepf(s.controls.trigger_threshold, d, 0.05, 0.05, 1.0); pct(s.controls.trigger_threshold) }, false);
            row(2, "MOUSE CAMERA", &mut |s, d| { if d != 0 { s.controls.mouse_camera = !s.controls.mouse_camera; } onoff(s.controls.mouse_camera) }, false);
            row(3, "MOUSE SENSITIVITY", &mut |s, d| { s.controls.mouse_sensitivity = stepf(s.controls.mouse_sensitivity, d, 0.01, 0.01, 0.5); format!("{:.2}", s.controls.mouse_sensitivity) }, false);
            row(4, "INVERT CAMERA Y", &mut |s, d| { if d != 0 { s.controls.invert_y = !s.controls.invert_y; } onoff(s.controls.invert_y) }, false);
            // P34 R8: PC addition (user decision Nov 6), default OFF; engine side = P41
            row(5, "PAUSE ON FOCUS LOSS", &mut |s, d| { if d != 0 { s.game.pause_on_focus_loss = !s.game.pause_on_focus_loss; } onoff(s.game.pause_on_focus_loss) }, false);
            // P41 prototype (default off): PC key names instead of PS2 button icons
            row(6, "PC KEY PROMPTS", &mut |s, d| { if d != 0 { s.game.pc_key_prompts = !s.game.pc_key_prompts; } onoff(s.game.pc_key_prompts) }, false);
        }
        BUTTONS_TAB => {
            let ai = ACTION.load(Relaxed).min(acts.len().saturating_sub(1));
            let cap = CAPTURE.load(Relaxed);
            row(0, "ACTION", &mut |_, d| {
                let i = (ai as i32 + d).rem_euclid(acts.len().max(1) as i32) as usize;
                ACTION.store(i, Relaxed);
                acts.get(i).map_or("-".into(), |a| bind_label(a))
            }, false);
            let act = acts.get(ACTION.load(Relaxed)).cloned().unwrap_or_default();
            let shown = |list: &[(String, Vec<String>)]| list.iter().find(|(a, _)| *a == act).and_then(|(_, n)| n.first()).map_or("-".into(), |n| bind_label(n));
            row(1, "KEY", &mut |s, _| if cap == 1 { "PRESS A KEY".into() } else { shown(&s.controls.keyboard) }, false);
            row(2, "GAMEPAD", &mut |s, _| {
                if scooby_recomp::engine::input::host::button_bit(&act).is_none() {
                    "STICK".into()
                } else if cap == 2 {
                    "PRESS A BUTTON".into()
                } else {
                    shown(&s.controls.gamepad)
                }
            }, false);
            row(3, "RESET BUTTONS", &mut |s, d| {
                if d != 0 {
                    let def = scooby_recomp::engine::input::host::Controls::default();
                    s.controls.keyboard = def.keyboard;
                    s.controls.gamepad = def.gamepad;
                }
                "DEFAULTS".into()
            }, false);
        }
        _ => {
            let inst = remaster_installed();
            row(0, "REMASTER LIGHTING", &mut |s, d| { if d != 0 && inst { s.remaster.lighting = !s.remaster.lighting; } if inst { onoff(s.remaster.lighting) } else { "NOT INSTALLED".into() } }, inst);
            row(1, "TEXTURE PACK", &mut |s, d| { if d != 0 && inst { s.remaster.texture_pack = !s.remaster.texture_pack; } if inst { onoff(s.remaster.texture_pack) } else { "NOT INSTALLED".into() } }, inst);
        }
    }
    out
}

/// after read_pad: open / navigate the page; while it is open the game sees an idle pad
pub fn page_input(page: &mut Page, p: &mut PadState) {
    let opts = scooby_recomp::modules::panel::OPTIONS_OPEN.load(Relaxed);
    let b = p.buttons;
    let edge = |m: u16| b & m != 0 && page.prev & m == 0;
    let stick = |v: u8| if v < 64 { -1i8 } else if v > 192 { 1 } else { 0 };
    let st = (stick(p.lx), stick(p.ly));
    let (sx, sy) = (if st.0 != page.prev_stick.0 { st.0 } else { 0 }, if st.1 != page.prev_stick.1 { st.1 } else { 0 });
    let snd = |id: u32| scooby_recomp::modules::panel::PC_PAGE_SOUND.store(id, Relaxed);
    if !opts && page.open {
        page.open = false; // the panel left the options page (e.g. cancelled by the game)
    }
    if page.open && CAPTURE.load(Relaxed) != 0 {
        // press-to-bind: game_input reads the raw key / button (the pad here is idle)
    } else if page.open {
        let n = rows(&mut get(), page.tab, usize::MAX, 0).len();
        let act_ok = scooby_recomp::engine::input::host::button_bit(get().controls.keyboard.get(ACTION.load(Relaxed)).map_or("", |a| a.0.as_str())).is_some();
        if page.tab == BUTTONS_TAB && edge(pad::CROSS) && (page.sel == 1 || (page.sel == 2 && act_ok)) {
            CAPTURE.store(page.sel as u8, Relaxed);
            snd(0xd);
        } else if page.tab == BUTTONS_TAB && (page.sel == 1 || page.sel == 2) && (edge(pad::LEFT) || edge(pad::RIGHT) || sx != 0) {
            // KEY / GAMEPAD rows change only by press-to-bind
        } else if edge(pad::TRIANGLE) || edge(pad::START) || edge(pad::SELECT) {
            page.open = false;
            page.hold = true;
            snd(0xe);
            if page.dirty {
                save();
                page.dirty = false;
            }
        } else if edge(pad::L1) || edge(pad::R1) {
            let n = ntabs();
            page.tab = (page.tab + if edge(pad::R1) { 1 } else { n - 1 }) % n;
            page.sel = 0;
            snd(10);
        } else if edge(pad::UP) || sy < 0 {
            page.sel = (page.sel + n - 1) % n;
            snd(10);
        } else if edge(pad::DOWN) || sy > 0 {
            page.sel = (page.sel + 1) % n;
            snd(10);
        } else {
            let d = if edge(pad::LEFT) || sx < 0 { -1 } else if edge(pad::RIGHT) || sx > 0 || edge(pad::CROSS) { 1 } else { 0 };
            if d != 0 {
                let (tab, sel) = (page.tab, page.sel);
                update(|s| {
                    rows(s, tab, sel, d);
                });
                page.dirty = true;
                snd(10);
            }
        }
    } else if opts && edge(pad::SELECT) {
        page.open = true;
        page.hold = true;
        snd(0xd);
    }
    // after a press-to-bind the pressed key / button must be released before it acts on the page
    page.prev = scooby_recomp::engine::input::host::page_prev(b, CAPTURE.load(Relaxed) != 0, CAPTURE_END.swap(false, Relaxed));
    page.prev_stick = st;
    if page.hold && !page.open && b == 0 && st == (0, 0) {
        page.hold = false;
    }
    if page.open {
        scooby_recomp::modules::panel::PC_PAGE_ROWS.store(rows(&mut get(), page.tab, usize::MAX, 0).len() as i32, Relaxed);
    }
    scooby_recomp::modules::panel::PC_PAGE_ROW.store(if page.open { page.sel as i32 } else { -1 }, Relaxed);
    if page.open || page.hold {
        *p = PadState { buttons: 0, lx: 128, ly: 128, rx: 128, ry: 128, ..*p };
    }
}

const LABEL: [u8; 4] = [247, 131, 32, 255];
const VALUE: [u8; 4] = [0, 114, 54, 255];

/// HUD commands of the page / the "PC SETTINGS (SELECT)" row, in the panel's own font
pub fn draw(page: &Page, hud: &mut Vec<HudDrawCmd>, fonts: &[Option<EFFont>], screen: (u32, u32)) {
    if !scooby_recomp::modules::panel::OPTIONS_OPEN.load(Relaxed) {
        return;
    }
    let (sw, sh) = (screen.0 as f32, screen.1 as f32);
    let font_cmds: Vec<HudDrawCmd> = hud.iter().filter(|c| matches!(c.tex, HudTex::Font { .. })).cloned().collect();
    let Some(proto) = font_cmds.iter().find(|c| c.colors[0] == LABEL).or(font_cmds.last()).cloned() else { return };
    let HudTex::Font { font: fi, .. } = proto.tex else { return };
    let Some(f) = fonts.get(fi as usize).and_then(|f| f.as_ref()) else { return };
    let label_cols = proto.colors;
    let val_cols = font_cmds.iter().find(|c| c.colors[0] == VALUE).map_or(proto.colors, |c| c.colors);
    // the panel rows: label-coloured glyphs above the help text
    let row_glyphs: Vec<&HudDrawCmd> = font_cmds.iter().filter(|c| c.colors[0] == proto.colors[0] && c.pos[0][1] < sh * 0.75).collect();
    // ascii code 1 = the font's "missing" box (no W, %, <, > in the panel font): W is drawn as two
    // overlapping V glyphs (VV_STEP of a V apart), others use the lowercase glyph (labels avoid Z: its lowercase glyph looks odd); still missing = reported once
    let code = |ch: char| f.ascii.get(ch as usize).copied().filter(|&c| c > 1);
    let vv = |ch: char| ch == 'W' && code('W').is_none();
    let glyph1 = |ch: char| code(ch).and_then(|c| f.Char(c as u32));
    let glyph = |ch: char| {
        let g = glyph1(ch).or_else(|| glyph1(if ch == 'W' { 'V' } else { ch.to_ascii_lowercase() }));
        if g.is_none() && ch != ' ' && MISSING.lock().unwrap().insert(ch) {
            eprintln!("pc settings page: glyph {ch:?} missing in the panel font");
        }
        g
    };
    let med = |mut v: Vec<f32>| { v.sort_by(f32::total_cmp); v.get(v.len() / 2).copied() };
    let q3 = |mut v: Vec<f32>| { v.sort_by(f32::total_cmp); v.get(v.len() * 3 / 4).copied() };
    let drawn = q3(row_glyphs.iter().map(|c| c.pos[1][1] - c.pos[0][1]).collect());
    let nat = med(('A'..='Z').filter_map(glyph1).map(|c| c.sh).filter(|&h| h > 0.0).collect());
    let sc = match (drawn, nat) { (Some(d), Some(n)) => (d / n).clamp(0.25, 2.0), _ => 1.0 };
    // the panel font's digits are small-caps size (sh ~14 vs ~22 for A..Z, bottom-aligned): drawn
    // DIGIT_K larger on the same baseline so numbers match the capitals
    let dk = |ch: char| if ch.is_ascii_digit() { DIGIT_K } else { 1.0 };
    let line_at = |hud: &mut Vec<HudDrawCmd>, parts: &[(&str, [[u8; 4]; 4])], y: f32, scale: f32, at: Option<f32>| -> f32 {
        let adv = |ch: char| scale * glyph(ch).filter(|_| ch != ' ').map_or(f.screen_space, |c| c.sw * dk(ch) * if vv(ch) { 1.0 + VV_STEP } else { 1.0 } + f.screen_spacing);
        let width: f32 = parts.iter().flat_map(|p| p.0.chars()).map(adv).sum();
        let x0 = at.unwrap_or((sw - width) * 0.5);
        let mut x = x0;
        for (text, cols) in parts {
            for ch in text.chars() {
                for (k, c) in glyph(ch).filter(|_| ch != ' ').into_iter().flat_map(|c| std::iter::repeat_n(c, if vv(ch) { 2 } else { 1 })).enumerate() {
                    let (w, h) = (c.sw * scale * dk(ch), c.sh * scale * dk(ch));
                    let yy = y + (c.syoff + c.sh) * scale - h;
                    let x = x + k as f32 * w * VV_STEP;
                    hud.push(HudDrawCmd {
                        tex: HudTex::Font { font: fi, tex: c.tex as i32 as u32 },
                        pos: [[x, yy], [x, yy + h], [x + w, yy], [x + w, yy + h]],
                        uv: [[c.u, c.v], [c.u, c.v + c.dv], [c.u + c.du, c.v], [c.u + c.du, c.v + c.dv]],
                        colors: *cols,
                        priority: proto.priority,
                    });
                }
                x += adv(ch);
            }
        }
        x0
    };
    let line = |hud: &mut Vec<HudDrawCmd>, parts: &[(&str, [[u8; 4]; 4])], y: f32, scale: f32| line_at(hud, parts, y, scale, None);
    let text_w = |t: &str, scale: f32| -> f32 { t.chars().map(|ch| scale * glyph(ch).filter(|_| ch != ' ').map_or(f.screen_space, |c| c.sw * dk(ch) * if vv(ch) { 1.0 + VV_STEP } else { 1.0 } + f.screen_spacing)).sum() };
    let last_row_y = row_glyphs.iter().map(|c| c.pos[1][1]).fold(0.0f32, f32::max);
    if !page.open {
        // entry row under the panel's rows (the solari build's ray-tracing row sits right below them)
        let base = if last_row_y > 0.0 { last_row_y + if cfg!(feature = "solari") { 72.0 } else { 36.0 } } else { sh * 0.585 };
        line(hud, &[("PC SETTINGS  ", label_cols), ("(SELECT)", val_cols)], base - 13.0 * sc.min(1.0), sc);
        if SAVE_FAILED.load(Relaxed) {
            line(hud, &[("SETTINGS COULD NOT BE SAVED", val_cols)], base - 13.0 * sc.min(1.0) + 30.0 * sc, sc * 0.8);
        }
        return;
    }
    // page open: the original rows are hidden (title, panel background + help text stay)
    // all row text (incl. the selected row's colour) below the title
    let top = font_cmds.iter().map(|c| c.pos[0][1]).filter(|&y| y > sh * 0.15 && y < sh * 0.75).fold(f32::MAX, f32::min);
    let top = if top < f32::MAX { top } else { sh * 0.3 };
    let band = |y: f32| y >= top - 8.0 && y < sh * 0.75;
    // selected-row colour of the panel (neither label orange nor value green, inside the rows)
    let sel_cols = font_cmds.iter().find(|c| band(c.pos[0][1]) && c.colors[0] != LABEL && c.colors[0] != VALUE).map_or([[255, 255, 255, 255]; 4], |c| c.colors);
    hud.retain(|c| !(matches!(c.tex, HudTex::Font { .. }) && band(c.pos[0][1])));
    // the original rows' pitch (the flower is placed on the original row spots, see PC_PAGE_ROW)
    let mut ys: Vec<f32> = font_cmds.iter().map(|c| c.pos[0][1]).filter(|&y| band(y)).collect();
    ys.sort_by(f32::total_cmp);
    let min_gap = drawn.unwrap_or(10.0) * 0.6;
    let mut starts: Vec<f32> = Vec::new();
    for y in ys {
        if starts.last().is_none_or(|&l| y - l > min_gap) {
            starts.push(y);
        }
    }
    // the flower steps by the 0x16 -> 0x17 spot distance = the first two original rows
    let orig_pitch = starts.get(1).zip(starts.first()).map_or(f32::MAX, |(b, a)| b - a);
    let pitch = if orig_pitch < f32::MAX { orig_pitch } else { (30.0 * sc.max(0.6)).max(sh * 0.05) };
    let n = rows(&mut get(), page.tab, usize::MAX, 0).len();
    let step = scooby_recomp::modules::panel::pc_row_step(n as i32);
    let row_y = |i: usize| match (starts.get(i.min(2)), i.checked_sub(2).zip(starts.get(1))) {
        (Some(&y), Some((k, &yp))) if starts.len() > 2 => y + k as f32 * (y - yp) * step,
        (Some(&y), _) => y,
        _ => top + pitch * i as f32,
    };
    let tab_txt = format!("- {}", TABS[page.tab]);
    line(hud, &[("PC SETTINGS ", label_cols), (&tab_txt, val_cols)], top - pitch * 1.2, sc);
    // the selection flower (a 3D hud object of the panel) is moved by the panel itself (PC_PAGE_ROW)
    let mut s = get();
    for (i, (l, v, restart)) in rows(&mut s, page.tab, usize::MAX, 0).into_iter().enumerate() {
        let sel = i == page.sel;
        let v = if restart { format!("{v} (R)") } else { v };
        // on the original rows' y (CAMERA .. APPLY CHANGES, unevenly spaced like their flower spots)
        let y = row_y(i);
        // original panel layout: label right-aligned left of the flower column, value left-aligned right of it
        let gap = 30.0 * sc;
        line_at(hud, &[(&l, if sel { sel_cols } else { label_cols })], y, sc, Some(sw * 0.5 - gap - text_w(&l, sc)));
        // value column right of the flower (the flower spans ~sw/2 - 16 .. sw/2 + 20 on the PS2 screen)
        line_at(hud, &[(&v, val_cols)], y, sc, Some(sw * 0.5 + gap + 14.0));
    }
    let note = if page.tab == BUTTONS_TAB {
        if CAPTURE.load(Relaxed) != 0 { "ESC / START CANCEL" } else { "X ON KEY OR GAMEPAD TO CHANGE" }
    } else if rows(&mut get(), page.tab, usize::MAX, 0).iter().any(|r| r.2) {
        "L1/R1 TAB    (R) RESTART"
    } else {
        "L1/R1 TAB"
    };
    // R10/R12: below the last row; a 7th row (CONTROLS) has the flower on it, so a full pitch lower there
    let below = if n > 6 { row_y(n - 1) + pitch * 0.9 } else { row_y(5) + pitch * 0.4 };
    line(hud, &[(note, label_cols)], below, sc * 0.8);
}

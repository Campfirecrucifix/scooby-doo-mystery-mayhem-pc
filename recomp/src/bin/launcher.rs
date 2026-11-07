//! P34: the game launcher ("Scooby-Doo-Mystery-Mayhem.exe", native Win32, no extra GUI deps).
//! R4: home page (title, PLAY, game disc image setup + validation, Settings, Quit, error panel when
//! the game dies at start), the settings tabs Graphics / Audio / Controls / Remaster over settings.toml (engine::settings, the file the game and its in-game
//! Options > PC Settings page use), Play starts game.exe (or the remaster build when installed and
//! enabled) with sound on. "Skip launcher next time" starts the game at once; hold Shift or pass
//! `--settings` to get the launcher back. `--shot FILE.bmp [--tab N]` renders the window offscreen
//! into a BMP and exits (headless screenshot, never shown on screen).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(non_snake_case)]
use scooby_recomp::engine::input::host::Controls;
use scooby_recomp::engine::settings::{self, FPS_CAPS, MSAA, RESOLUTIONS, Settings, WindowMode};
use std::cell::RefCell;
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, GetAsyncKeyState, GetFocus, GetKeyState, SetFocus, VK_CONTROL, VK_SHIFT, VK_TAB};
use windows_sys::Win32::UI::HiDpi::GetDpiForSystem;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// page index of the home page (0..3 = the settings tabs)
const HOME: usize = 10;
/// common items of the settings pages (tab strip, Save, Back, ...)
const SET: usize = 11;

const TABS: [&str; 4] = ["Graphics", "Audio", "Controls", "Remaster"];

/// the shipped build exposes only original-game settings: Remaster only when a remaster build is installed
fn ntabs() -> usize {
    if with(|u| u.remaster.is_some()) { TABS.len() } else { TABS.len() - 1 }
}
const ACTIONS: [&str; 24] = [
    "cross", "circle", "square", "triangle", "l1", "r1", "l2", "r2", "l3", "r3", "start", "select", "up", "down", "left", "right",
    "lstick_up", "lstick_down", "lstick_left", "lstick_right", "rstick_up", "rstick_down", "rstick_left", "rstick_right",
];
const PAD: [&str; 17] = ["South", "East", "North", "West", "LeftTrigger", "RightTrigger", "LeftTrigger2", "RightTrigger2", "Select", "Start", "Mode", "LeftThumb", "RightThumb", "DPadUp", "DPadDown", "DPadLeft", "DPadRight"];
const NONE: &str = "(none)";

fn key_names() -> Vec<String> {
    let mut v: Vec<String> = ('A'..='Z').map(|c| format!("Key{c}")).collect();
    v.extend((0..10).map(|d| format!("Digit{d}")));
    v.extend(["Space", "Enter", "Tab", "Backspace", "ShiftLeft", "ShiftRight", "ControlLeft", "ControlRight", "AltLeft", "AltRight", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].map(String::from));
    v.extend((0..10).map(|d| format!("Numpad{d}")));
    v.extend(["NumpadEnter", "NumpadAdd", "NumpadSubtract", "Comma", "Period", "Slash", "Semicolon", "Quote", "BracketLeft", "BracketRight", "Minus", "Equal", "Backquote", "Backslash", "Insert", "Delete", "Home", "End", "PageUp", "PageDown", "CapsLock"].map(String::from));
    v.extend((1..=10).map(|d| format!("F{d}")));
    v.extend(["Mouse.Left", "Mouse.Right", "Mouse.Middle", "Mouse.Back", "Mouse.Forward"].map(String::from));
    v
}

fn w(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

// control ids
const ID_TAB: i32 = 100;
const ID_PLAY: i32 = 101;
const ID_SAVE: i32 = 102;
const ID_QUIT: i32 = 103;
const ID_SKIP: i32 = 104;
const ID_DEFAULTS: i32 = 105;
const ID_ACTION: i32 = 110;
const ID_RESET: i32 = 111;
const ID_K1: i32 = 112;
const ID_K2: i32 = 113;
const ID_PADB: i32 = 114;
/// "Set..." (press-to-bind) buttons for Key 1, Key 2, Gamepad button
const ID_CAP: i32 = 115;
// graphics
const ID_MODE: i32 = 200;
const ID_RES: i32 = 201;
const ID_VSYNC: i32 = 202;
const ID_FPS: i32 = 203;
const ID_MSAA: i32 = 204;
// audio sliders 300.., value labels 320..
const ID_VOL: i32 = 300;
const ID_MUTE: i32 = 310;
// controls sliders 400.. (deadzone, trigger, sensitivity), labels 420..
const ID_CSL: i32 = 400;
const ID_MOUSE: i32 = 410;
const ID_INVY: i32 = 411;
const ID_PAUSEF: i32 = 412;
const ID_KEYP: i32 = 413;
const ID_RLIGHT: i32 = 500;
const ID_RPACK: i32 = 501;
// home
const ID_SETTINGS: i32 = 600;
const ID_BROWSE: i32 = 601;
const ID_OPENLOG: i32 = 602;
const ID_BUG: i32 = 604;
/// P34 R8/R9 (user decision Nov 6): bug reports go to the project's GitHub Issues page. The URL
/// lives in ONE place: the `Project page (GitHub): <url>` line at the top of README.txt next to the
/// exe (tools/dist/README.txt, placeholder {{GITHUB_URL}} until release; make_dist warns).
/// Returns `<url>/issues`, or None while the placeholder is unfilled / README.txt is missing.
pub fn bug_report_url(readme: &str) -> Option<String> {
    // P40 R11: only the first word after the label (text wrapped onto the same line must not join the URL)
    let url = readme.lines().find_map(|l| l.trim().strip_prefix("Project page (GitHub):"))?.split_whitespace().next()?;
    url.starts_with("http").then(|| format!("{}/issues", url.trim_end_matches('/')))
}
const ID_BACK: i32 = 603;
const TIMER_PAD: usize = 1;
/// R6: the 4 bars of the keyboard / gamepad focus ring
const ID_RING: i32 = 700;
const TIMER_CHILD: usize = 2;

/// the running game (Play): process, start time, crash log size before the start
struct Running {
    child: std::process::Child,
    start: std::time::Instant,
    crash_len: u64,
}

struct Ui {
    main: HWND,
    st: Settings,
    /// P34 R7: the file as last read / written (the base of the three-way merge on Save)
    base: Settings,
    path: PathBuf,
    /// (tab, control)
    pages: Vec<(usize, HWND)>,
    action: usize,
    keys: Vec<String>,
    remaster: Option<PathBuf>,
    texpack: Option<PathBuf>,
    font: HFONT,
    big_font: HFONT,
    /// current page (HOME or a settings tab)
    page: usize,
    /// disc check result: Ok(display line) / Err(reason)
    iso: Result<String, String>,
    /// error panel text (the game exited with an error)
    error: Option<String>,
    running: Option<Running>,
    /// background art (from the disc, cached), see `background`
    /// home art from the user's disc: the game's title logo + the gang (premultiplied DIBs)
    bg: Option<[(HBITMAP, i32, i32); 2]>,
    /// press-to-bind in progress: 0/1 = key 1/2, 2 = gamepad button
    capture: Option<usize>,
    scale: f32,
}

thread_local! {
    static UI: RefCell<Option<Ui>> = const { RefCell::new(None) };
}

fn with<R>(f: impl FnOnce(&mut Ui) -> R) -> R {
    UI.with(|u| f(u.borrow_mut().as_mut().expect("ui")))
}

unsafe fn item(id: i32) -> HWND {
    unsafe { GetDlgItem(with(|u| u.main), id) }
}
unsafe fn send(h: HWND, m: u32, wp: usize, lp: isize) -> isize {
    unsafe { SendMessageW(h, m, wp, lp) }
}
unsafe fn checked(id: i32) -> bool {
    unsafe { send(item(id), BM_GETCHECK, 0, 0) == BST_CHECKED as isize }
}
unsafe fn set_check(id: i32, on: bool) {
    unsafe { send(item(id), BM_SETCHECK, on as usize, 0) };
}
unsafe fn combo_sel(id: i32) -> usize {
    unsafe { send(item(id), CB_GETCURSEL, 0, 0).max(0) as usize }
}
unsafe fn combo_text(id: i32) -> String {
    unsafe {
        let h = item(id);
        let i = send(h, CB_GETCURSEL, 0, 0);
        if i < 0 {
            return String::new();
        }
        let n = send(h, CB_GETLBTEXTLEN, i as usize, 0).max(0) as usize;
        let mut b = vec![0u16; n + 1];
        send(h, CB_GETLBTEXT, i as usize, b.as_mut_ptr() as isize);
        String::from_utf16_lossy(&b[..n])
    }
}
unsafe fn combo_fill(id: i32, items: &[String], sel: &str) {
    unsafe {
        let h = item(id);
        send(h, CB_RESETCONTENT, 0, 0);
        let mut found = 0;
        for (i, s) in items.iter().enumerate() {
            send(h, CB_ADDSTRING, 0, w(s).as_ptr() as isize);
            if s == sel {
                found = i;
            }
        }
        if !items.iter().any(|s| s == sel) && !sel.is_empty() {
            send(h, CB_ADDSTRING, 0, w(sel).as_ptr() as isize);
            found = items.len();
        }
        send(h, CB_SETCURSEL, found, 0);
    }
}
unsafe fn slider(id: i32) -> i32 {
    unsafe { send(item(id), WM_USER, 0, 0) as i32 }
}
unsafe fn set_text(id: i32, s: &str) {
    unsafe { SetWindowTextW(item(id), w(s).as_ptr()) };
}

/// child control on `tab` (None = always visible)
unsafe fn ctl(tab: Option<usize>, class: &str, text: &str, style: u32, x: i32, y: i32, cw: i32, ch: i32, id: i32) -> HWND {
    unsafe {
        let (main, font, k) = with(|u| (u.main, u.font, u.scale));
        let vis = if tab.is_none() { WS_VISIBLE } else { 0 };
        let sc = |v: i32| (v as f32 * k).round() as i32;
        let h = CreateWindowExW(0, w(class).as_ptr(), w(text).as_ptr(), WS_CHILD | vis | style, sc(x), sc(y), sc(cw), sc(ch), main, id as usize as HMENU, GetModuleHandleW(null()), null());
        send(h, WM_SETFONT, font as usize, 1);
        if let Some(t) = tab {
            with(|u| u.pages.push((t, h)));
        }
        h
    }
}

unsafe fn label(tab: usize, text: &str, x: i32, y: i32, cw: i32, id: i32) -> HWND {
    unsafe { ctl(Some(tab), "STATIC", text, 0, x, y + 3, cw, if text.contains('\n') { 84 } else { 20 }, id) }
}
unsafe fn combo(tab: usize, x: i32, y: i32, cw: i32, id: i32) -> HWND {
    unsafe { ctl(Some(tab), "COMBOBOX", "", (CBS_DROPDOWNLIST | WS_VSCROLL as i32) as u32 | WS_TABSTOP, x, y, cw, 300, id) }
}
unsafe fn check(tab: usize, text: &str, x: i32, y: i32, cw: i32, id: i32) -> HWND {
    unsafe { ctl(Some(tab), "BUTTON", text, BS_AUTOCHECKBOX as u32 | WS_TABSTOP, x, y, cw, 22, id) }
}
unsafe fn trackbar(tab: usize, x: i32, y: i32, lo: i32, hi: i32, pos: i32, id: i32) -> HWND {
    unsafe {
        let h = ctl(Some(tab), "msctls_trackbar32", "", TBS_HORZ | WS_TABSTOP, x, y, 240, 26, id);
        send(h, TBM_SETRANGE, 1, ((hi << 16) | lo) as isize);
        send(h, TBM_SETPOS, 1, pos as isize);
        h
    }
}

const SLIDERS_A: [&str; 4] = ["Master volume", "Music", "Sound effects", "Voice / movies"];
const SLIDERS_C: [&str; 3] = ["Stick deadzone", "Trigger threshold", "Mouse sensitivity"];

unsafe fn build() {
    unsafe {
        let st = with(|u| u.st.clone());
        let main = with(|u| u.main);
        // home page (texts + background are painted in WM_PAINT, see paint_home)
        let play = ctl(Some(HOME), "BUTTON", "PLAY", BS_DEFPUSHBUTTON as u32 | WS_TABSTOP, 50, 300, 270, 64, ID_PLAY);
        send(play, WM_SETFONT, with(|u| u.big_font) as usize, 1);
        ctl(Some(HOME), "BUTTON", "Settings", WS_TABSTOP, 50, 376, 130, 36, ID_SETTINGS);
        ctl(Some(HOME), "BUTTON", "Quit", WS_TABSTOP, 190, 376, 130, 36, ID_QUIT);
        ctl(Some(HOME), "BUTTON", "Browse...", WS_TABSTOP, 400, 376, 130, 36, ID_BROWSE);
        ctl(Some(HOME), "BUTTON", "Open log folder", WS_TABSTOP, 580, 470, 150, 30, ID_OPENLOG);
        ctl(Some(HOME), "BUTTON", "Report a bug", WS_TABSTOP, 420, 470, 150, 30, ID_BUG);
        // settings pages
        let tab = ctl(Some(SET), "SysTabControl32", "", WS_CLIPSIBLINGS | WS_TABSTOP, 10, 10, 740, 400, ID_TAB);
        for (i, t) in TABS.iter().take(ntabs()).enumerate() {
            let mut txt = w(t);
            let it = TCITEMW { mask: TCIF_TEXT, dwState: 0, dwStateMask: 0, pszText: txt.as_mut_ptr(), cchTextMax: 0, iImage: -1, lParam: 0 };
            send(tab, TCM_INSERTITEMW, i, &it as *const _ as isize);
        }
        let (x0, x1, mut y) = (30, 190, 50);
        // Graphics
        let g = &st.graphics;
        label(0, "Display mode", x0, y, 150, -1);
        combo(0, x1, y, 200, ID_MODE);
        combo_fill(ID_MODE, &["Windowed".into(), "Borderless fullscreen".into(), "Fullscreen".into()], ["Windowed", "Borderless fullscreen", "Fullscreen"][WindowMode::ALL.iter().position(|&m| m == g.window_mode).unwrap_or(0)]);
        y += 34;
        label(0, "Window size", x0, y, 150, -1);
        combo(0, x1, y, 200, ID_RES);
        let mut res = vec!["Default (1280x720)".to_string()];
        res.extend(RESOLUTIONS.iter().map(|(a, b)| format!("{a}x{b}")));
        combo_fill(ID_RES, &res, &g.resolution.map_or(res[0].clone(), |(a, b)| format!("{a}x{b}")));
        y += 34;
        check(0, "VSync", x0, y, 150, ID_VSYNC);
        set_check(ID_VSYNC, g.vsync);
        y += 34;
        label(0, "FPS cap", x0, y, 150, -1);
        combo(0, x1, y, 200, ID_FPS);
        let fps: Vec<String> = FPS_CAPS.iter().map(|&f| if f == 0 { "Off".into() } else { f.to_string() }).collect();
        combo_fill(ID_FPS, &fps, &fps[FPS_CAPS.iter().position(|&f| f == g.fps_cap).unwrap_or(0)]);
        y += 34;
        label(0, "Anti-aliasing", x0, y, 150, -1);
        combo(0, x1, y, 200, ID_MSAA);
        let aa: Vec<String> = MSAA.iter().map(|&m| if m == 1 { "Off".into() } else { format!("{m}x MSAA") }).collect();
        combo_fill(ID_MSAA, &aa, &aa[MSAA.iter().position(|&m| m == g.msaa).unwrap_or(2)]);
        y += 40;
        label(0, "Display mode, size, VSync and FPS cap also change live in the game\n(Options > PC Settings). Anti-aliasing applies on the next start.", x0, y, 460, -1);
        // Audio
        let a = &st.audio;
        y = 50;
        for (i, (n, v)) in SLIDERS_A.iter().zip([a.master, a.music, a.sfx, a.voice]).enumerate() {
            label(1, n, x0, y, 150, -1);
            trackbar(1, x1, y, 0, 100, (v * 100.0).round() as i32, ID_VOL + i as i32);
            label(1, "", x1 + 250, y, 60, ID_VOL + 20 + i as i32);
            y += 36;
        }
        check(1, "Mute", x0, y, 150, ID_MUTE);
        set_check(ID_MUTE, a.mute);
        label(1, "Applied on top of the game's own Options > Sound volumes.\nChanges apply live in the game (Options > PC Settings).", x0, y + 40, 460, -1);
        // Controls
        let c = &st.controls;
        y = 46;
        for (i, (n, (v, lo, hi))) in SLIDERS_C.iter().zip([((c.deadzone * 100.0) as i32, 0, 95), ((c.trigger_threshold * 100.0) as i32, 5, 100), ((c.mouse_sensitivity * 100.0).round() as i32, 1, 50)]).enumerate() {
            label(2, n, x0, y, 150, -1);
            trackbar(2, x1, y, lo, hi, v, ID_CSL + i as i32);
            label(2, "", x1 + 250, y, 60, ID_CSL + 20 + i as i32);
            y += 32;
        }
        check(2, "Mouse moves the camera", x0, y, 200, ID_MOUSE);
        set_check(ID_MOUSE, c.mouse_camera);
        check(2, "Invert camera Y", x1 + 50, y, 200, ID_INVY);
        set_check(ID_INVY, c.invert_y);
        y += 34;
        label(2, "Action", x0, y, 150, -1);
        combo(2, x1, y, 200, ID_ACTION);
        combo_fill(ID_ACTION, &ACTIONS.map(String::from), "cross");
        y += 32;
        label(2, "Key 1 / Key 2", x0, y, 150, -1);
        combo(2, x1, y, 140, ID_K1);
        combo(2, x1 + 150, y, 140, ID_K2);
        ctl(Some(2), "BUTTON", "Press key 1", WS_TABSTOP, x1 + 300, y, 110, 26, ID_CAP);
        ctl(Some(2), "BUTTON", "Press key 2", WS_TABSTOP, x1 + 420, y, 110, 26, ID_CAP + 1);
        y += 32;
        label(2, "Gamepad button", x0, y, 150, -1);
        combo(2, x1, y, 140, ID_PADB);
        ctl(Some(2), "BUTTON", "Reset controls", WS_TABSTOP, x1 + 150, y, 140, 26, ID_RESET);
        ctl(Some(2), "BUTTON", "Press button", WS_TABSTOP, x1 + 300, y, 110, 26, ID_CAP + 2);
        y += 40;
        check(2, "Pause when the window loses focus", x0, y, 300, ID_PAUSEF);
        set_check(ID_PAUSEF, st.game.pause_on_focus_loss);
        y += 28;
        check(2, "PC key prompts (experimental)", x0, y, 300, ID_KEYP);
        set_check(ID_KEYP, st.game.pc_key_prompts);
        // Remaster
        let (inst, pack) = with(|u| (u.remaster.is_some(), u.texpack.is_some()));
        let r = &st.remaster;
        y = 50;
        let lc = check(3, "Remaster lighting", x0, y, 300, ID_RLIGHT);
        set_check(ID_RLIGHT, r.lighting);
        let pc = check(3, "Remaster texture pack", x0, y + 34, 300, ID_RPACK);
        set_check(ID_RPACK, r.texture_pack);
        EnableWindow(lc, inst as i32);
        EnableWindow(pc, (inst && pack) as i32);
        let note = match (inst, pack) {
            (false, _) => format!("No remaster build next to game.exe. Copy {}\n(+ texpack folder) there to enable these options.", settings::REMASTER_EXE),
            (true, false) => format!("{} found; no texpack folder next to it.\nChanges apply on the next start.", settings::REMASTER_EXE),
            (true, true) => format!("Play starts {} with these options.\nChanges apply on the next start.", settings::REMASTER_EXE),
        };
        label(3, &note, x0, y + 80, 460, -1);
        // bottom row
        ctl(Some(SET), "BUTTON", "Skip launcher next time (hold Shift to get it back)", BS_AUTOCHECKBOX as u32 | WS_TABSTOP, 14, 422, 440, 22, ID_SKIP);
        set_check(ID_SKIP, st.launcher.skip);
        ctl(Some(SET), "BUTTON", "Restore defaults", WS_TABSTOP, 14, 460, 150, 34, ID_DEFAULTS);
        ctl(Some(SET), "BUTTON", "Save", WS_TABSTOP, 500, 460, 120, 34, ID_SAVE);
        ctl(Some(SET), "BUTTON", "Back", WS_TABSTOP, 630, 460, 120, 34, ID_BACK);
        for i in 0..4 {
            ctl(None, "STATIC", "", 0, 0, 0, 0, 0, ID_RING + i);
        }
        load_action();
        refresh_labels();
        let _ = main;
    }
}

unsafe fn refresh_labels() {
    unsafe {
        for i in 0..4 {
            set_text(ID_VOL + 20 + i, &format!("{}%", slider(ID_VOL + i)));
        }
        set_text(ID_CSL + 20, &format!("{}%", slider(ID_CSL)));
        set_text(ID_CSL + 21, &format!("{}%", slider(ID_CSL + 1)));
        set_text(ID_CSL + 22, &format!("{:.2}", slider(ID_CSL + 2) as f32 / 100.0));
    }
}

fn binding<'a>(tbl: &'a [(String, Vec<String>)], a: &str) -> &'a [String] {
    tbl.iter().find(|e| e.0 == a).map_or(&[], |e| &e.1[..])
}

fn set_binding(tbl: &mut Vec<(String, Vec<String>)>, a: &str, v: Vec<String>) {
    match tbl.iter_mut().find(|e| e.0 == a) {
        Some(e) => e.1 = v,
        None => tbl.push((a.to_string(), v)),
    }
}

/// action combobox -> key / pad comboboxes
unsafe fn load_action() {
    unsafe {
        let (a, c, keys) = with(|u| (ACTIONS[u.action], u.st.controls.clone(), u.keys.clone()));
        let kb = binding(&c.keyboard, a);
        let mut list = vec![NONE.to_string()];
        list.extend(keys);
        combo_fill(ID_K1, &list, kb.first().map_or(NONE, |s| s.as_str()));
        combo_fill(ID_K2, &list, kb.get(1).map_or(NONE, |s| s.as_str()));
        let stick = a.contains("stick_");
        let mut pl = vec![NONE.to_string()];
        pl.extend(PAD.map(String::from));
        combo_fill(ID_PADB, &if stick { vec!["(stick: keyboard only)".into()] } else { pl }, binding(&c.gamepad, a).first().map_or(if stick { "(stick: keyboard only)" } else { NONE }, |s| s.as_str()));
        EnableWindow(item(ID_PADB), (!stick) as i32);
    }
}

/// key / pad comboboxes -> settings (keeps extra bindings beyond the shown ones)
unsafe fn store_action() {
    unsafe {
        let (k1, k2, pb) = (combo_text(ID_K1), combo_text(ID_K2), combo_text(ID_PADB));
        with(|u| {
            let a = ACTIONS[u.action];
            let c = &mut u.st.controls;
            let old = binding(&c.keyboard, a).to_vec();
            let mut kb: Vec<String> = [k1, k2].into_iter().filter(|s| s != NONE && !s.is_empty()).collect();
            kb.extend(old.into_iter().skip(2));
            set_binding(&mut c.keyboard, a, kb);
            if !a.contains("stick_") {
                let old = binding(&c.gamepad, a).to_vec();
                let mut pv: Vec<String> = Some(pb).filter(|s| s != NONE && !s.is_empty()).into_iter().collect();
                pv.extend(old.into_iter().skip(1));
                set_binding(&mut c.gamepad, a, pv);
            }
        });
    }
}

/// all controls -> settings
unsafe fn pull() {
    unsafe {
        store_action();
        let (mode, res, fps, aa) = (combo_sel(ID_MODE), combo_sel(ID_RES), combo_sel(ID_FPS), combo_sel(ID_MSAA));
        let vol: Vec<f32> = (0..4).map(|i| slider(ID_VOL + i) as f32 / 100.0).collect();
        let cs: Vec<i32> = (0..3).map(|i| slider(ID_CSL + i)).collect();
        let flags = [ID_VSYNC, ID_MUTE, ID_MOUSE, ID_INVY, ID_RLIGHT, ID_RPACK, ID_SKIP, ID_PAUSEF, ID_KEYP].map(|i| checked(i));
        with(|u| {
            let s = &mut u.st;
            s.graphics.window_mode = WindowMode::ALL[mode.min(2)];
            s.graphics.resolution = if res == 0 { None } else { RESOLUTIONS.get(res - 1).copied().or(s.graphics.resolution) };
            s.graphics.vsync = flags[0];
            s.graphics.fps_cap = FPS_CAPS[fps.min(FPS_CAPS.len() - 1)];
            s.graphics.msaa = MSAA[aa.min(MSAA.len() - 1)];
            (s.audio.master, s.audio.music, s.audio.sfx, s.audio.voice) = (vol[0], vol[1], vol[2], vol[3]);
            s.audio.mute = flags[1];
            s.controls.deadzone = cs[0] as f32 / 100.0;
            s.controls.trigger_threshold = cs[1] as f32 / 100.0;
            s.controls.mouse_sensitivity = cs[2] as f32 / 100.0;
            s.controls.mouse_camera = flags[2];
            s.controls.invert_y = flags[3];
            s.remaster.lighting = flags[4];
            s.remaster.texture_pack = flags[5];
            s.launcher.skip = flags[6];
            s.game.pause_on_focus_loss = flags[7];
            s.game.pc_key_prompts = flags[8];
        });
    }
}

/// the settings file now (None = missing / unreadable)
fn read_disk(p: &std::path::Path) -> Option<Settings> {
    std::fs::read_to_string(p).ok().and_then(|t| Settings::parse(&t).ok())
}

/// Save: re-read the file first and keep what the game's Options menu (or another launcher)
/// changed there meanwhile in the sections this launcher did not touch (P34 R7, no stale
/// overwrite). Returns true when that changed `u.st` (the caller rebuilds the controls).
fn save(u: &mut Ui) -> bool {
    let s = match read_disk(&u.path) {
        Some(disk) => u.st.merged(&u.base, &disk),
        None => u.st.clone(),
    };
    if let Err(e) = s.save(&u.path) {
        msg(&e);
    }
    let changed = s != u.st;
    u.st = s.clone();
    u.base = s;
    changed
}

/// the file was changed by someone else (the game's Options menu while the launcher waited):
/// take those changes in the sections not edited here, then rebuild the controls
unsafe fn reload_from_disk() {
    unsafe {
        pull();
        let changed = with(|u| match read_disk(&u.path) {
            Some(disk) if disk != u.base => {
                let s = u.st.merged(&u.base, &disk);
                let c = s != u.st;
                u.st = s;
                u.base = disk;
                c
            }
            _ => false,
        });
        if changed {
            rebuild();
        }
    }
}

/// destroy + recreate all controls from `st` (keeps the page)
unsafe fn rebuild() {
    unsafe {
        let (main, page) = with(|u| {
            u.pages.clear();
            u.action = 0;
            (u.main, u.page)
        });
        let t = if page == HOME { None } else { Some(send(item(ID_TAB), TCM_GETCURSEL, 0, 0) as usize) };
        loop {
            let c = GetWindow(main, GW_CHILD);
            if c.is_null() {
                break;
            }
            DestroyWindow(c);
        }
        build();
        show_tab(t.unwrap_or(HOME));
        home_state();
    }
}

fn msg(s: &str) {
    unsafe { MessageBoxW(null_mut(), w(s).as_ptr(), w("Scooby-Doo launcher").as_ptr(), MB_OK | MB_ICONWARNING) };
}

/// P34 R12: how long the launcher waits for the disc image to answer (validation on the UI thread)
const DISC_PROBE: std::time::Duration = std::time::Duration::from_secs(4);

/// P34 R12: keep a `ww` x `wh` window with its top-left at (x, y) inside `work` (top-left wins when too big)
fn clamp_to(work: &RECT, x: i32, y: i32, ww: i32, wh: i32) -> (i32, i32) {
    (x.min(work.right - ww).max(work.left), y.min(work.bottom - wh).max(work.top))
}

/// P34 R12: the launcher window's start position: the remembered one, clamped to the work area of the
/// monitor it is on (centred on the main monitor when it was on a monitor that is gone)
unsafe fn place(pos: Option<(i32, i32)>, ww: i32, wh: i32) -> (i32, i32) {
    unsafe {
        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if let Some((x, y)) = pos {
            let mon = MonitorFromRect(&RECT { left: x, top: y, right: x + ww, bottom: y + wh }, MONITOR_DEFAULTTONULL);
            if !mon.is_null() && GetMonitorInfoW(mon, &mut mi) != 0 {
                return clamp_to(&mi.rcWork, x, y, ww, wh);
            }
        }
        let mon = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        if GetMonitorInfoW(mon, &mut mi) == 0 {
            return ((GetSystemMetrics(SM_CXSCREEN) - ww) / 2, (GetSystemMetrics(SM_CYSCREEN) - wh) / 2);
        }
        let w = &mi.rcWork;
        clamp_to(w, w.left + (w.right - w.left - ww) / 2, w.top + (w.bottom - w.top - wh) / 2, ww, wh)
    }
}

fn exe_dir() -> PathBuf {
    std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())).unwrap_or_default()
}

/// start the game: exe next to the launcher (or the remaster build), cwd = its folder, absolute
/// disc + settings paths, stdout/stderr -> log_dir()/game.log, crash log -> log_dir()/crash.log,
/// no console window. The launcher watches the process (TIMER_CHILD).
fn play(st: &Settings, extra: &[String]) -> Result<Running, String> {
    use std::os::windows::process::CommandExt;
    let dir = exe_dir();
    let (exe, env) = settings::launch_plan(st, &dir);
    if !exe.is_file() {
        return Err(format!("{} was not found in\n{}\nThe launcher must sit in the folder that contains game.exe.", exe.file_name().unwrap_or_default().to_string_lossy(), dir.display()));
    }
    let iso = st.game.iso.clone().ok_or_else(|| "No game disc image is set: choose it with Browse... first.".to_string())?;
    // P34 R12: bounded (a removed USB drive / an offline share must not freeze the window)
    if let Err(e) = settings::validate_iso_within(&iso, DISC_PROBE) {
        return Err(format!("The disc image can't be opened right now:
{}
({e})
Reconnect the drive, or choose the disc again with Change...", iso.display()));
    }
    let iso = std::path::absolute(&iso).unwrap_or(iso);
    let logs = settings::log_dir();
    let _ = std::fs::create_dir_all(&logs);
    let crash = logs.join("crash.log");
    let crash_len = std::fs::metadata(&crash).map_or(0, |m| m.len());
    // P39 R8: opened for this game only (shared for reading): a second Play while a game runs no longer
    // truncates the running game's log, it gets this message instead
    let out = scooby_recomp::crashlog::open_log_exclusive(&logs.join("game.log")).map_err(|e| {
        if scooby_recomp::crashlog::log_in_use(&e) {
            "Scooby-Doo! Mystery Mayhem is already running (its game.log is in use). Close the other game window first.".to_string()
        } else {
            format!("cannot write {}: {e}", logs.display())
        }
    })?;
    let err = out.try_clone().map_err(|e| e.to_string())?;
    let cfg = std::path::absolute(settings::paths().0).unwrap_or_default();
    let mut cmd = std::process::Command::new(&exe);
    cmd.current_dir(&dir).arg("--sound").arg("--iso").arg(&iso).args(extra);
    cmd.env("SCOOBY_CRASHLOG", &crash).env("SCOOBY_SETTINGS", &cfg).stdout(out).stderr(err);
    // CREATE_NO_WINDOW: game.exe is a console program; its output goes to game.log
    cmd.creation_flags(0x0800_0000);
    for (k, v) in env {
        match v {
            Some(v) => cmd.env(k, v),
            None => cmd.env_remove(k),
        };
    }
    let child = cmd.spawn().map_err(|e| format!("cannot start {}: {e}", exe.display()))?;
    Ok(Running { child, start: std::time::Instant::now(), crash_len })
}

/// a crash = a non-zero exit code or a new record in the crash log (a panic in a thread that
/// still lets the process exit 0); a normal quit closes the launcher
fn crashed(r: &Running, success: bool) -> bool {
    !success || !new_records(r).is_empty()
}

/// this run's crash records (see settings::crash_records)
fn new_records(r: &Running) -> Vec<String> {
    let crash = std::fs::read(settings::log_dir().join("crash.log")).unwrap_or_default();
    settings::crash_records(&String::from_utf8_lossy(&crash[(r.crash_len as usize).min(crash.len())..]), r.child.id())
}

/// why the game stopped: the panic / exception record the game appended to the crash log, else
/// the last error lines of game.log
fn exit_report(r: &Running, code: Option<i32>) -> String {
    let logs = settings::log_dir();
    // the last crash record of this run, else the last error lines of game.log
    let mut why: Vec<String> = new_records(r).pop().map(|x| x.lines().map(String::from).collect()).unwrap_or_default();
    if why.is_empty() {
        let log = std::fs::read_to_string(logs.join("game.log")).unwrap_or_default();
        let bad: Vec<&str> = log.lines().filter(|l| l.contains("panicked") || l.contains("error") || l.contains("ERROR")).collect();
        why = bad.iter().rev().take(3).rev().map(|l| l.trim().to_string()).collect();
    }
    let how = match code {
        Some(0) | None => String::new(),
        Some(c) => format!(", exit code {c:#x}"),
    };
    let mut s = format!("The game closed unexpectedly (after {:.0} s{how}). Details: Open log folder; Report a bug opens the project's GitHub Issues page.", r.start.elapsed().as_secs_f32());
    for l in why {
        s += "\n";
        s += &l.chars().take(300).collect::<String>();
    }
    s
}

/// Play: save, start, minimize (or stay hidden when `hidden`, skip-launcher mode)
unsafe fn start_game(h: HWND, hidden: bool) {
    unsafe {
        pull();
        let st = with(|u| {
            save(u);
            u.st.clone()
        });
        match play(&st, &[]) {
            Ok(r) => {
                with(|u| {
                    u.running = Some(r);
                    u.error = None;
                });
                SetTimer(h, TIMER_CHILD, 500, None);
                if !hidden {
                    ShowWindow(h, SW_MINIMIZE);
                }
            }
            Err(e) => {
                with(|u| u.error = Some(e));
                show_tab(HOME);
                ShowWindow(h, SW_SHOWNORMAL);
            }
        }
        home_state();
    }
}

/// the disc image: validate, store in settings.toml when good
unsafe fn set_iso(p: PathBuf) {
    unsafe {
        let r = settings::validate_iso_within(&p, DISC_PROBE);
        let main = with(|u| u.main);
        match r {
            Ok(okmsg) => with(|u| {
                u.st.game.iso = Some(p);
                u.iso = Ok(okmsg);
                save(u);
            }),
            Err(e) => {
                let e = format!("{}: {e}", p.file_name().unwrap_or_default().to_string_lossy());
                if with(|u| u.iso.is_ok()) {
                    msg(&format!("{e}\n\nThe current disc image is kept."));
                } else {
                    with(|u| u.iso = Err(e));
                }
            }
        }
        background();
        home_state();
        InvalidateRect(main, null(), 1);
    }
}

/// Browse...: the native open dialog for *.iso / *.bin
unsafe fn browse(h: HWND) {
    use windows_sys::Win32::UI::Controls::Dialogs::*;
    unsafe {
        let mut buf = vec![0u16; 2048];
        let filter: Vec<u16> = "PS2 disc images (*.iso, *.bin, *.img, *.cue)\0*.iso;*.bin;*.img;*.cue\0All files (*.*)\0*.*\0\0".encode_utf16().collect();
        let title = w("Choose your Scooby-Doo! Mystery Mayhem disc image");
        // R13: first run starts in the game folder (where players tend to put the image)
        let start = with(|u| u.st.game.iso.as_ref().and_then(|p| p.parent()).map(|d| d.to_path_buf())).or_else(|| Some(exe_dir())).map(|d| w(&d.display().to_string()));
        let mut of: OPENFILENAMEW = std::mem::zeroed();
        of.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        of.hwndOwner = h;
        of.lpstrFilter = filter.as_ptr();
        of.lpstrFile = buf.as_mut_ptr();
        of.nMaxFile = buf.len() as u32;
        of.lpstrTitle = title.as_ptr();
        if let Some(d) = &start {
            of.lpstrInitialDir = d.as_ptr();
        }
        of.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_HIDEREADONLY;
        if GetOpenFileNameW(&mut of) == 0 {
            return;
        }
        let n = buf.iter().position(|&c| c == 0).unwrap_or(0);
        // P39 R6: the exact wide path (lossy UTF-8 would turn a lone-surrogate file name into another file)
        set_iso(PathBuf::from(<std::ffi::OsString as std::os::windows::ffi::OsStringExt>::from_wide(&buf[..n])));
    }
}

/// the home art, decoded from the user's own disc (nothing of the game ships with the port):
/// INTRO Title_Art (the "Scooby-Doo! Mystery Mayhem" logo) and the gang of the EP1 splash with its
/// blue backdrop keyed out. Cached as raw RGBA next to the logs (keyed by disc path + size), so a
/// launcher start does not re-read the disc. None (no / bad disc) = the painted fallback title.
fn home_art(iso: &std::path::Path) -> Option<[(u32, u32, Vec<u8>); 2]> {
    let size = std::fs::metadata(iso).ok()?.len();
    let cache = settings::log_dir().with_file_name("cache");
    let key = format!("{}|{size}|1", iso.display());
    let files = [cache.join("home_logo.rgba"), cache.join("home_gang.rgba")];
    let read = |f: &PathBuf, w: u32, h: u32| std::fs::read(f).ok().filter(|b| b.len() == (w * h * 4) as usize);
    const DIMS: [(u32, u32); 2] = [(280, 205), (512, 180)];
    if std::fs::read_to_string(cache.join("home.key")).ok().as_deref() == Some(key.as_str()) {
        if let (Some(a), Some(b)) = (read(&files[0], DIMS[0].0, DIMS[0].1), read(&files[1], DIMS[1].0, DIMS[1].1)) {
            return Some([(DIMS[0].0, DIMS[0].1, a), (DIMS[1].0, DIMS[1].1, b)]);
        }
    }
    let find = |file: &str, name: &str| disc_textures(iso, file).into_iter().find(|t| t.name.eq_ignore_ascii_case(name));
    let crop = |t: &scooby_recomp::engine::rw::image::RwTexture, x0: u32, y0: u32, (w, h): (u32, u32)| -> Option<Vec<u8>> {
        if x0 + w > t.width || y0 + h > t.height {
            return None;
        }
        (y0..y0 + h).map(|y| t.rgba.get(((y * t.width + x0) * 4) as usize..((y * t.width + x0 + w) * 4) as usize)).collect::<Option<Vec<_>>>().map(|r| r.concat())
    };
    let logo = crop(&find("LEVE_/INTRO/EN/TEXDIC_0.TXD", "Title_Art")?, 200, 0, DIMS[0])?;
    let mut gang = crop(&find("LEVE_/SPLASH/EN/TEXDIC_0.TXD", "SplashScreen_EP01")?, 0, 300, DIMS[1])?;
    for p in gang.chunks_mut(4) {
        let (r, g, b) = (p[0] as i32, p[1] as i32, p[2] as i32);
        if b > r + 40 && g > r + 20 {
            p.copy_from_slice(&[0; 4]);
        }
    }
    if std::fs::create_dir_all(&cache).is_ok() && std::fs::write(&files[0], &logo).is_ok() && std::fs::write(&files[1], &gang).is_ok() {
        let _ = std::fs::write(cache.join("home.key"), key);
    }
    Some([(DIMS[0].0, DIMS[0].1, logo), (DIMS[1].0, DIMS[1].1, gang)])
}

/// RGBA -> premultiplied top-down 32-bit DIB for AlphaBlend
unsafe fn dib(w: u32, h: u32, rgba: &[u8]) -> HBITMAP {
    unsafe {
        let mut bi: BITMAPINFO = std::mem::zeroed();
        bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bi.bmiHeader.biWidth = w as i32;
        bi.bmiHeader.biHeight = -(h as i32);
        bi.bmiHeader.biPlanes = 1;
        bi.bmiHeader.biBitCount = 32;
        let mut bits: *mut core::ffi::c_void = null_mut();
        let bmp = CreateDIBSection(null_mut(), &bi, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
        if !bmp.is_null() && !bits.is_null() {
            let out = std::slice::from_raw_parts_mut(bits as *mut u8, (w * h * 4) as usize);
            for (o, p) in out.chunks_mut(4).zip(rgba.chunks(4)) {
                let a = p[3] as u32;
                o.copy_from_slice(&[(p[2] as u32 * a / 255) as u8, (p[1] as u32 * a / 255) as u8, (p[0] as u32 * a / 255) as u8, a as u8]);
            }
        }
        bmp
    }
}

/// (re)build the home art for the current disc
unsafe fn background() {
    unsafe {
        let iso = with(|u| u.st.game.iso.clone().filter(|_| u.iso.is_ok()));
        let art = iso.and_then(|p| home_art(&p)).map(|a| a.map(|(w, h, px)| (dib(w, h, &px), w as i32, h as i32)));
        if let Some(old) = with(|u| std::mem::replace(&mut u.bg, art)) {
            for (b, _, _) in old {
                DeleteObject(b as HGDIOBJ);
            }
        }
    }
}

unsafe fn text(hdc: HDC, s: &str, x: i32, y: i32, cw: i32, ch: i32, size: i32, weight: i32, col: u32, flags: u32) {
    unsafe {
        let k = with(|u| u.scale);
        let sc = |v: i32| (v as f32 * k).round() as i32;
        let f = CreateFontW(-sc(size), 0, 0, 0, weight, 0, 0, 0, 1, 0, 0, 5, 0, w("Segoe UI").as_ptr());
        let old = SelectObject(hdc, f as HGDIOBJ);
        SetTextColor(hdc, col);
        let mut r = RECT { left: sc(x), top: sc(y), right: sc(x + cw), bottom: sc(y + ch) };
        let mut t: Vec<u16> = s.encode_utf16().collect();
        DrawTextW(hdc, t.as_mut_ptr(), t.len() as i32, &mut r, flags);
        SelectObject(hdc, old);
        DeleteObject(f as HGDIOBJ);
    }
}

fn rgb(r: u8, g: u8, b: u8) -> u32 {
    r as u32 | (g as u32) << 8 | (b as u32) << 16
}

unsafe fn fill(hdc: HDC, x: i32, y: i32, cw: i32, ch: i32, col: u32) {
    unsafe {
        let k = with(|u| u.scale);
        let sc = |v: i32| (v as f32 * k).round() as i32;
        let b = CreateSolidBrush(col);
        FillRect(hdc, &RECT { left: sc(x), top: sc(y), right: sc(x + cw), bottom: sc(y + ch) }, b);
        DeleteObject(b as HGDIOBJ);
    }
}

/// the home page: background (disc art when available, else a dark gradient), title, disc status,
/// error panel, version
unsafe fn paint_home(h: HWND, hdc: HDC) {
    unsafe {
        let mut rc: RECT = std::mem::zeroed();
        GetClientRect(h, &mut rc);
        let (bg, iso, path, err) = with(|u| (u.bg, u.iso.clone(), u.st.game.iso.clone(), u.error.clone()));
        let (cw, ch) = (rc.right, rc.bottom);
        // dark night-sky gradient (the fallback, and the frame around the art)
        for i in 0..64 {
            let t = i as f32 / 63.0;
            let c = rgb((22.0 + 30.0 * t) as u8, (14.0 + 10.0 * t) as u8, (40.0 + 26.0 * (1.0 - t)) as u8);
            let b = CreateSolidBrush(c);
            FillRect(hdc, &RECT { left: 0, top: ch * i / 64, right: cw, bottom: ch * (i + 1) / 64 + 1 }, b);
            DeleteObject(b as HGDIOBJ);
        }
        SetBkMode(hdc, TRANSPARENT as i32);
        if let Some(art) = bg {
            // logo top-left, the gang running across the right (logical 760x512 layout)
            let k = with(|u| u.scale);
            let sc = |v: i32| (v as f32 * k).round() as i32;
            for ((bmp, bw, bh), (x, y, dw)) in art.into_iter().zip([(24, 8, 330), (318, 70, 430)]) {
                let mdc = CreateCompatibleDC(hdc);
                let old = SelectObject(mdc, bmp as HGDIOBJ);
                SetStretchBltMode(hdc, HALFTONE);
                let bf = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8 };
                GdiAlphaBlend(hdc, sc(x), sc(y), sc(dw), sc(dw * bh / bw), mdc, 0, 0, bw, bh, bf);
                SelectObject(mdc, old);
                DeleteDC(mdc);
            }
        } else {
            let center = DT_CENTER | DT_SINGLELINE | DT_VCENTER;
            for (dx, col) in [(3, rgb(0, 0, 0)), (0, rgb(255, 196, 36))] {
                text(hdc, "Scooby-Doo!", dx, 40 + dx, 760, 110, 84, 900, col, center);
            }
            for (dx, col) in [(2, rgb(0, 0, 0)), (0, rgb(140, 230, 90))] {
                text(hdc, "MYSTERY MAYHEM", dx, 140 + dx, 760, 60, 40, 800, col, center);
            }
        }
        fill(hdc, 390, 280, 350, 136, rgb(16, 12, 26));
        text(hdc, "GAME DISC IMAGE", 400, 286, 330, 22, 14, 700, rgb(200, 190, 230), DT_LEFT | DT_SINGLELINE);
        let (status, col) = match &iso {
            Ok(s) if s.starts_with('!') => (format!("\u{26a0} {}", &s[1..]), rgb(250, 210, 90)),
            Ok(s) => (format!("\u{2713} {s}"), rgb(120, 230, 120)),
            Err(e) => (format!("\u{2717} {e}"), rgb(255, 120, 110)),
        };
        let (status_h, hint) = disc_box(iso.is_ok(), path.is_some());
        text(hdc, &status, 400, 308, 330, status_h, 14, 600, col, DT_LEFT | DT_WORDBREAK | DT_END_ELLIPSIS);
        let p = path.clone().filter(|_| iso.is_ok()).map_or("Browse... for the .iso / .bin copied from your own PS2 disc.".to_string(), |p| p.display().to_string().replace('/', "\\")); // DT_PATH_ELLIPSIS needs backslashes
        if hint {
            text(hdc, &p, 400, 348, 330, 22, 12, 400, rgb(170, 170, 190), DT_LEFT | DT_SINGLELINE | DT_PATH_ELLIPSIS);
        }
        if iso.is_err() {
            text(hdc, "First, choose your disc image (Browse...) to unlock PLAY.", 50, 274, 330, 22, 12, 400, rgb(230, 210, 150), DT_LEFT | DT_SINGLELINE);
        }
        let has_err = err.is_some();
        if let Some(e) = err {
            fill(hdc, 50, 424, 520, 84, rgb(70, 18, 22));
            text(hdc, &e, 58, 428, 506, 78, 13, 400, rgb(255, 230, 230), DT_LEFT | DT_WORDBREAK | DT_END_ELLIPSIS);
        }
        let ver = format!("v{}  \u{00b7}  settings: {}", env!("CARGO_PKG_VERSION"), settings::paths().0.display());
        // (the error panel covers the footer line)
        if !has_err {
            text(hdc, &ver, 10, 512 - 22, 560, 20, 11, 400, rgb(150, 140, 170), DT_LEFT | DT_SINGLELINE | DT_PATH_ELLIPSIS);
        }
    }
}

/// all textures of a texture dictionary file on the user's disc (empty when unreadable)
fn disc_textures(iso: &std::path::Path, file: &str) -> Vec<scooby_recomp::engine::rw::image::RwTexture> {
    let Ok(fs) = scooby_recomp::engine::iso::IsoFs::open(iso) else { return Vec::new() };
    let Ok(b) = fs.read(file) else { return Vec::new() };
    // P39: an untested release / damaged disc must not take the launcher down (this runs inside
    // the window procedure, where a panic aborts): no art = the painted fallback title
    CATCHING.store(true, std::sync::atomic::Ordering::Relaxed);
    let r = std::panic::catch_unwind(|| {
        let mut d = scooby_recomp::engine::rw::image::RwTexDictionary::default();
        scooby_recomp::engine::rw::image::LoadTextureDictionary(&mut scooby_recomp::engine::rw::stream::RwStream::new(&b), &mut d);
        d.textures
    });
    CATCHING.store(false, std::sync::atomic::Ordering::Relaxed);
    r.unwrap_or_default()
}

/// --print-env: what Play would start for each remaster combination (pack x lighting)
fn print_env(st: &Settings) {
    let dir = exe_dir();
    println!("cwd: {} (inherited)", std::env::current_dir().unwrap_or_default().display());
    for (pack, light) in [(false, false), (false, true), (true, false), (true, true)] {
        let mut s = st.clone();
        s.remaster = settings::Remaster { lighting: light, texture_pack: pack };
        let (exe, env) = settings::launch_plan(&s, &dir);
        let env: Vec<String> = env.iter().map(|(k, v)| v.as_ref().map_or(format!("-{k}"), |v| format!("{k}={v}"))).collect();
        println!("pack={} light={}: {} --sound{} [{}]", pack as u8, light as u8, exe.display(), if exe.is_file() { "" } else { " (missing)" }, env.join(" "));
    }
}

/// Restore defaults: the current tab's section back to the defaults, then rebuild the controls
unsafe fn restore_tab() {
    unsafe {
        pull();
        let t = send(item(ID_TAB), TCM_GETCURSEL, 0, 0) as usize;
        with(|u| {
            let d = Settings::default();
            match t {
                0 => u.st.graphics = d.graphics,
                1 => u.st.audio = d.audio,
                2 => {
                    u.st.controls = d.controls;
                    u.st.game.pause_on_focus_loss = d.game.pause_on_focus_loss;
                    u.st.game.pc_key_prompts = d.game.pc_key_prompts;
                }
                _ => u.st.remaster = d.remaster,
            }
        });
        rebuild();
        SetFocus(item(ID_DEFAULTS));
    }
}

thread_local! {
    /// (pad index, buttons) at the previous capture poll (a held button is no press)
    static CAP_PREV: std::cell::Cell<Option<(u32, u32)>> = const { std::cell::Cell::new(None) };
}

/// P41 R7: gamepad press-to-bind: the buttons pressed since the previous poll of the SAME pad,
/// read in the foreground. Nothing counts on the first reading after the capture starts, after
/// the pad is (re)plugged or another pad becomes the first connected one, or after the launcher
/// comes back to the front (what is held then is no press; before, a pad re-plugged during the
/// capture or a button pressed in another window could bind). Sticks are never read (no drift).
fn cap_press(prev: &mut Option<(u32, u32)>, now: Option<(u32, u32)>, fg: bool) -> u32 {
    let now = now.filter(|_| fg);
    match (std::mem::replace(prev, now), now) {
        (Some((i, o)), Some((j, n))) if i == j => n & !o,
        _ => 0,
    }
}

/// the binding name of a key press (Controls key names), None = not bindable
/// the binding name of a mouse button-down message (press-to-bind), None = no button press
fn mouse_name(msg: u32, wp: usize) -> Option<&'static str> {
    match msg {
        WM_LBUTTONDOWN => Some("Mouse.Left"),
        WM_RBUTTONDOWN => Some("Mouse.Right"),
        WM_MBUTTONDOWN => Some("Mouse.Middle"),
        // XBUTTON1 = back, XBUTTON2 = forward (HIWORD of wParam)
        WM_XBUTTONDOWN => Some(if (wp >> 16) & 0xffff == 2 { "Mouse.Forward" } else { "Mouse.Back" }),
        _ => None,
    }
}

fn vk_name(vk: u32, lp: isize) -> Option<String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    let ext = lp & (1 << 24) != 0;
    let scan = (lp >> 16) & 0xff;
    let vk = vk as u16;
    let n = match vk {
        0x41..=0x5a => format!("Key{}", vk as u8 as char),
        0x30..=0x39 => format!("Digit{}", vk as u8 as char),
        0x70..=0x79 => format!("F{}", vk - 0x6f),
        _ if (VK_NUMPAD0..=VK_NUMPAD9).contains(&vk) => format!("Numpad{}", vk - VK_NUMPAD0),
        VK_SPACE => "Space".into(),
        VK_RETURN => if ext { "NumpadEnter" } else { "Enter" }.into(),
        VK_TAB => "Tab".into(),
        VK_BACK => "Backspace".into(),
        VK_SHIFT => if scan == 0x36 { "ShiftRight" } else { "ShiftLeft" }.into(),
        VK_CONTROL => if ext { "ControlRight" } else { "ControlLeft" }.into(),
        VK_MENU => if ext { "AltRight" } else { "AltLeft" }.into(),
        VK_UP => "ArrowUp".into(),
        VK_DOWN => "ArrowDown".into(),
        VK_LEFT => "ArrowLeft".into(),
        VK_RIGHT => "ArrowRight".into(),
        VK_ADD => "NumpadAdd".into(),
        VK_SUBTRACT => "NumpadSubtract".into(),
        VK_OEM_COMMA => "Comma".into(),
        VK_OEM_PERIOD => "Period".into(),
        VK_OEM_2 => "Slash".into(),
        VK_OEM_1 => "Semicolon".into(),
        VK_OEM_7 => "Quote".into(),
        VK_OEM_4 => "BracketLeft".into(),
        VK_OEM_6 => "BracketRight".into(),
        VK_OEM_MINUS => "Minus".into(),
        VK_OEM_PLUS => "Equal".into(),
        VK_OEM_3 => "Backquote".into(),
        VK_OEM_5 => "Backslash".into(),
        VK_INSERT => "Insert".into(),
        VK_DELETE => "Delete".into(),
        VK_HOME => "Home".into(),
        VK_END => "End".into(),
        VK_PRIOR => "PageUp".into(),
        VK_NEXT => "PageDown".into(),
        VK_CAPITAL => "CapsLock".into(),
        _ => return None,
    };
    key_names().contains(&n).then_some(n)
}

/// finish a press-to-bind: select `name` in the slot's list (None = cancelled), restore the button
unsafe fn capture_end(name: Option<&str>) {
    unsafe {
        let Some(slot) = with(|u| u.capture.take()) else { return };
        CAP_PREV.set(None);
        set_text(ID_CAP + slot as i32, ["Press key 1", "Press key 2", "Press button"][slot]);
        if let Some(n) = name {
            let id = [ID_K1, ID_K2, ID_PADB][slot];
            let i = send(item(id), CB_FINDSTRINGEXACT, usize::MAX, w(n).as_ptr() as isize);
            if i >= 0 {
                send(item(id), CB_SETCURSEL, i as usize, 0);
                store_action();
            }
        }
    }
}

/// XInput (first connected pad), polled on a timer while the launcher is the foreground window:
/// D-pad / left stick up-down = focus, left-right = change the focused value (or the tab), A =
/// press / toggle / open-close a list, LB / RB = tab, Start = Play, B = close a list / Quit
unsafe fn pad_poll(main: HWND) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{VK_DOWN, VK_LEFT, VK_RIGHT, VK_UP};
    use windows_sys::Win32::UI::Input::XboxController::*;
    thread_local! {
        static PREV: std::cell::Cell<(Option<u16>, u32)> = const { std::cell::Cell::new((None, 0)) };
    }
    unsafe {
        let mut st: XINPUT_STATE = std::mem::zeroed();
        let idx = (0..4).find(|&i| XInputGetState(i, &mut st) == 0);
        let g = st.Gamepad;
        if with(|u| u.capture == Some(2)) {
            let now = idx.map(|i| (i, g.wButtons as u32 | ((g.bLeftTrigger > 128) as u32) << 16 | ((g.bRightTrigger > 128) as u32) << 17));
            let mut p = CAP_PREV.get();
            let fresh = cap_press(&mut p, now, GetForegroundWindow() == main);
            CAP_PREV.set(p);
            let names: [(u32, &str); 16] = [(XINPUT_GAMEPAD_A as u32, "South"), (XINPUT_GAMEPAD_B as u32, "East"), (XINPUT_GAMEPAD_Y as u32, "North"), (XINPUT_GAMEPAD_X as u32, "West"), (XINPUT_GAMEPAD_LEFT_SHOULDER as u32, "LeftTrigger"), (XINPUT_GAMEPAD_RIGHT_SHOULDER as u32, "RightTrigger"), (1 << 16, "LeftTrigger2"), (1 << 17, "RightTrigger2"), (XINPUT_GAMEPAD_BACK as u32, "Select"), (XINPUT_GAMEPAD_START as u32, "Start"), (XINPUT_GAMEPAD_LEFT_THUMB as u32, "LeftThumb"), (XINPUT_GAMEPAD_RIGHT_THUMB as u32, "RightThumb"), (XINPUT_GAMEPAD_DPAD_UP as u32, "DPadUp"), (XINPUT_GAMEPAD_DPAD_DOWN as u32, "DPadDown"), (XINPUT_GAMEPAD_DPAD_LEFT as u32, "DPadLeft"), (XINPUT_GAMEPAD_DPAD_RIGHT as u32, "DPadRight")];
            if let Some((_, n)) = names.iter().find(|(f, _)| fresh & f != 0) {
                capture_end(Some(n));
            }
            return PREV.set((None, 0));
        }
        if idx.is_none() {
            return PREV.set((None, 0));
        }
        let mut b = g.wButtons as u16;
        let t = 16000;
        for (on, f) in [(g.sThumbLY > t, XINPUT_GAMEPAD_DPAD_UP), (g.sThumbLY < -t, XINPUT_GAMEPAD_DPAD_DOWN), (g.sThumbLX < -t, XINPUT_GAMEPAD_DPAD_LEFT), (g.sThumbLX > t, XINPUT_GAMEPAD_DPAD_RIGHT)] {
            if on {
                b |= f as u16;
            }
        }
        let (prev, held) = PREV.get();
        let dirs = (XINPUT_GAMEPAD_DPAD_UP | XINPUT_GAMEPAD_DPAD_DOWN | XINPUT_GAMEPAD_DPAD_LEFT | XINPUT_GAMEPAD_DPAD_RIGHT) as u16;
        // the first reading (and anything held while the window is in the background) is no press
        let Some(prev) = prev.filter(|_| GetForegroundWindow() == main) else {
            return PREV.set((Some(b), 0));
        };
        // direction auto-repeat: 10 ticks (~0.3 s) then every 4th tick
        let held = if b & dirs != 0 && b & dirs == prev & dirs { held + 1 } else { 0 };
        let rep = held >= 10 && held % 4 == 0;
        let hit = |f: u16| b & f != 0 && (prev & f == 0 || (f & dirs != 0 && rep));
        PREV.set((Some(b), held));
        let focus = GetFocus();
        let mut cls = [0u16; 32];
        let n = GetClassNameW(focus, cls.as_mut_ptr(), 32).max(0) as usize;
        let cls = String::from_utf16_lossy(&cls[..n]).to_ascii_lowercase();
        let dropped = cls == "combobox" && send(focus, CB_GETDROPPEDSTATE, 0, 0) != 0;
        let key = |vk: u16| {
            send(focus, WM_KEYDOWN, vk as usize, 0);
            send(focus, WM_KEYUP, vk as usize, 0);
        };
        let home = with(|u| u.page == HOME);
        let tab = |d: usize| {
            if home {
                return;
            }
            let cur = send(item(ID_TAB), TCM_GETCURSEL, 0, 0) as usize;
            show_tab((cur + d) % ntabs());
            SetFocus(item(ID_TAB));
        };
        let next = |back: bool| {
            let n = GetNextDlgTabItem(main, focus, back as i32);
            if !n.is_null() {
                SetFocus(n);
            }
        };
        for (f, up, vk) in [(XINPUT_GAMEPAD_DPAD_UP as u16, true, VK_UP), (XINPUT_GAMEPAD_DPAD_DOWN as u16, false, VK_DOWN)] {
            if hit(f) {
                if dropped { key(vk) } else { next(up) }
            }
        }
        for (f, d, vk) in [(XINPUT_GAMEPAD_DPAD_LEFT as u16, ntabs() - 1, VK_LEFT), (XINPUT_GAMEPAD_DPAD_RIGHT as u16, 1, VK_RIGHT)] {
            if hit(f) {
                if focus == item(ID_TAB) {
                    tab(d);
                } else if cls != "button" {
                    key(vk);
                }
            }
        }
        if hit(XINPUT_GAMEPAD_A as u16) {
            if cls == "button" {
                send(focus, BM_CLICK, 0, 0);
            } else if cls == "combobox" {
                send(focus, CB_SHOWDROPDOWN, (!dropped) as usize, 0);
            } else if focus == item(ID_TAB) {
                next(false);
            }
        }
        if hit(XINPUT_GAMEPAD_LEFT_SHOULDER as u16) {
            tab(ntabs() - 1);
        }
        if hit(XINPUT_GAMEPAD_RIGHT_SHOULDER as u16) {
            tab(1);
        }
        if hit(XINPUT_GAMEPAD_B as u16) {
            if dropped {
                send(focus, CB_SHOWDROPDOWN, 0, 0);
            } else {
                PostMessageW(main, WM_COMMAND, IDCANCEL as usize, 0);
            }
        }
        if hit(XINPUT_GAMEPAD_START as u16) {
            if dropped {
                send(focus, CB_SHOWDROPDOWN, 0, 0);
            }
            PostMessageW(main, WM_COMMAND, IDOK as usize, 0);
        }
    }
}

/// R6: focus ring colour (orange, COLORREF = 0x00bbggrr)
const RING_RGB: u32 = 0x0000_a0ff;

/// R6: a thick orange ring around the focused control (Tab / arrows / gamepad navigation), kept
/// on top of the tab control; moved only when the focus or its position changes.
unsafe fn focus_ring(main: HWND) {
    unsafe { ring_at(main, GetFocus()) }
}
unsafe fn ring_at(main: HWND, f: HWND) {
    thread_local! {
        static LAST: std::cell::Cell<(usize, [i32; 4])> = const { std::cell::Cell::new((0, [0; 4])) };
    }
    unsafe {
        let id = if f.is_null() { 0 } else { GetDlgCtrlID(f) };
        let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        let show = !f.is_null() && GetParent(f) == main && IsWindowVisible(f) != 0 && !(ID_RING..ID_RING + 4).contains(&id);
        if show {
            GetWindowRect(f, &mut r);
            MapWindowPoints(null_mut(), main, &mut r as *mut RECT as *mut POINT, 2);
        }
        let key = (if show { f as usize } else { 0 }, [r.left, r.top, r.right, r.bottom]);
        if LAST.replace(key) == key {
            return;
        }
        let k = with(|u| u.scale);
        let (t, g) = ((3.0 * k).round() as i32, (2.0 * k).round() as i32);
        let (x0, y0, x1, y1) = (r.left - g - t, r.top - g - t, r.right + g + t, r.bottom + g + t);
        let bars = [(x0, y0, x1 - x0, t), (x0, y1 - t, x1 - x0, t), (x0, y0, t, y1 - y0), (x1 - t, y0, t, y1 - y0)];
        for (i, (x, y, cw, ch)) in bars.into_iter().enumerate() {
            let flags = SWP_NOACTIVATE | if show { SWP_SHOWWINDOW } else { SWP_HIDEWINDOW };
            SetWindowPos(item(ID_RING + i as i32), HWND_TOP, x, y, cw, ch, flags);
        }
    }
}

unsafe extern "system" fn wndproc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        match m {
            WM_COMMAND => {
                let (id, code) = ((wp & 0xffff) as i32, ((wp >> 16) & 0xffff) as u32);
                match (id, code) {
                    (ID_ACTION, CBN_SELCHANGE) => {
                        store_action();
                        let a = combo_sel(ID_ACTION);
                        with(|u| u.action = a);
                        load_action();
                    }
                    (i, BN_CLICKED) if (ID_CAP..ID_CAP + 3).contains(&i) => {
                        capture_end(None);
                        let slot = (i - ID_CAP) as usize;
                        with(|u| u.capture = Some(slot));
                        set_text(i, if slot == 2 { "Press a button" } else { "Press a key" });
                    }
                    (ID_RESET, BN_CLICKED) => {
                        with(|u| u.st.controls = Controls::default());
                        let c = Controls::default();
                        for (i, v) in [(c.deadzone * 100.0) as i32, (c.trigger_threshold * 100.0) as i32, (c.mouse_sensitivity * 100.0).round() as i32].into_iter().enumerate() {
                            send(item(ID_CSL + i as i32), TBM_SETPOS, 1, v as isize);
                        }
                        set_check(ID_MOUSE, c.mouse_camera);
                        set_check(ID_INVY, c.invert_y);
                        load_action();
                        refresh_labels();
                    }
                    (ID_DEFAULTS, BN_CLICKED) => restore_tab(),
                    (ID_SAVE, BN_CLICKED) => {
                        pull();
                        if with(save) {
                            rebuild();
                            SetFocus(item(ID_SAVE));
                        }
                    }
                    (ID_SETTINGS, BN_CLICKED) => {
                        show_tab(with(|u| u.st.launcher.tab as usize).min(ntabs() - 1));
                        SetFocus(item(ID_TAB));
                    }
                    (ID_BACK, BN_CLICKED) => {
                        pull();
                        if with(save) {
                            rebuild();
                        }
                        show_tab(HOME);
                        SetFocus(item(ID_PLAY));
                    }
                    (ID_BROWSE, BN_CLICKED) => browse(h),
                    (ID_BUG, BN_CLICKED) => {
                        let readme = exe_dir().join("README.txt");
                        let url = std::fs::read_to_string(&readme).ok().and_then(|t| bug_report_url(&t));
                        // no URL yet: open README.txt (it says where to report)
                        let _ = std::process::Command::new("explorer").arg(url.unwrap_or_else(|| readme.display().to_string())).spawn();
                    }
                    (ID_OPENLOG, BN_CLICKED) => {
                        let d = settings::log_dir();
                        let _ = std::fs::create_dir_all(&d);
                        let _ = std::process::Command::new("explorer").arg(&d).spawn();
                    }
                    // keyboard: Enter (IDOK, the default button) = Play, Esc (IDCANCEL) = Back / Quit
                    (ID_PLAY, BN_CLICKED) | (IDOK, _) => {
                        if with(|u| u.iso.is_ok() && u.running.is_none()) {
                            start_game(h, false);
                        }
                    }
                    (IDCANCEL, _) if with(|u| u.page != HOME) => {
                        PostMessageW(h, WM_COMMAND, ID_BACK as usize, 0);
                    }
                    (ID_QUIT, BN_CLICKED) | (IDCANCEL, _) => { DestroyWindow(h); }
                    _ => {}
                }
                0
            }
            WM_TIMER if wp == TIMER_CHILD => {
                let done = with(|u| u.running.as_mut().and_then(|r| r.child.try_wait().ok().flatten()));
                if let Some(st) = done {
                    KillTimer(h, TIMER_CHILD);
                    let r = with(|u| u.running.take()).expect("running");
                    if !crashed(&r, st.success()) {
                        DestroyWindow(h);
                    } else {
                        let e = exit_report(&r, st.code());
                        with(|u| u.error = Some(e));
                        reload_from_disk();
                        show_tab(HOME);
                        ShowWindow(h, SW_SHOWNORMAL);
                    }
                }
                0
            }
            WM_TIMER => {
                pad_poll(h);
                focus_ring(h);
                0
            }
            WM_ERASEBKGND if with(|u| u.page == HOME) => 1,
            WM_PAINT => {
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                let hdc = BeginPaint(h, &mut ps);
                if with(|u| u.page == HOME) {
                    paint_home(h, hdc);
                }
                EndPaint(h, &ps);
                0
            }
            WM_HSCROLL => {
                refresh_labels();
                0
            }
            WM_NOTIFY => {
                let n = &*(lp as *const NMHDR);
                if n.idFrom == ID_TAB as usize && n.code == TCN_SELCHANGE {
                    let t = send(n.hwndFrom, TCM_GETCURSEL, 0, 0) as usize;
                    show_tab(t);
                }
                0
            }
            WM_CTLCOLORSTATIC => {
                // the themed tab body colour (labels sit on the tab page)
                static BR: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
                let br = *BR.get_or_init(|| CreateSolidBrush(0x00f9_f9f9) as usize);
                SetBkMode(wp as HDC, TRANSPARENT as i32);
                if (ID_RING..ID_RING + 4).contains(&GetDlgCtrlID(lp as HWND)) {
                    static RING: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
                    return *RING.get_or_init(|| CreateSolidBrush(RING_RGB) as usize) as isize;
                }
                let on_tab = UI.with(|u| u.try_borrow().ok().and_then(|u| u.as_ref().map(|u| u.pages.iter().any(|p| p.1 == lp as HWND)))).unwrap_or(false);
                if on_tab { br as isize } else { GetSysColorBrush(COLOR_WINDOW) as isize }
            }
            // back to the launcher (restored / alt-tab): pick up the game's Options menu changes
            WM_ACTIVATE if wp & 0xffff != 0 => {
                if UI.with(|u| u.try_borrow().ok().and_then(|u| u.as_ref().map(|u| u.running.is_none() && u.capture.is_none()))).unwrap_or(false) {
                    reload_from_disk();
                }
                DefWindowProcW(h, m, wp, lp)
            }
            WM_DESTROY => {
                // remember the last settings tab + the window position (only those keys; unsaved edits stay unsaved)
                let mut wp: WINDOWPLACEMENT = std::mem::zeroed();
                wp.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
                let mut rc: RECT = std::mem::zeroed();
                // minimized (the game is running): the normal-position rect; else the real one
                let pos = if IsIconic(h) != 0 { (GetWindowPlacement(h, &mut wp) != 0).then(|| (wp.rcNormalPosition.left, wp.rcNormalPosition.top)) } else { (GetWindowRect(h, &mut rc) != 0).then_some((rc.left, rc.top)) };
                let pos = pos.filter(|&(x, _)| x > -10000); // the --shot window sits at -20000
                if let Some((p, t)) = UI.with(|u| u.try_borrow().ok().and_then(|u| u.as_ref().filter(|u| u.st.launcher.tab != u.base.launcher.tab || (pos.is_some() && pos != u.base.launcher.pos)).map(|u| (u.path.clone(), u.st.launcher.tab)))) {
                    if let Some(mut d) = read_disk(&p) {
                        d.launcher.tab = t;
                        d.launcher.pos = pos.or(d.launcher.pos);
                        let _ = d.save(&p);
                    }
                }
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(h, m, wp, lp),
        }
    }
}

/// show a page: HOME or settings tab 0..3 (+ the common settings items)
unsafe fn show_tab(t: usize) {
    unsafe {
        let (pages, main) = with(|u| {
            u.page = t;
            if t != HOME {
                u.st.launcher.tab = t as u32;
            }
            (u.pages.clone(), u.main)
        });
        for (tab, c) in pages {
            let on = tab == t || (tab == SET && t != HOME);
            ShowWindow(c, if on { SW_SHOW } else { SW_HIDE });
        }
        if t != HOME {
            send(item(ID_TAB), TCM_SETCURSEL, t, 0);
        }
        home_state();
        InvalidateRect(main, null(), 1);
    }
}

/// home page buttons follow the disc / error state
unsafe fn home_state() {
    unsafe {
        let (ok, err, home, running) = with(|u| (u.iso.is_ok(), u.error.is_some(), u.page == HOME, u.running.is_some()));
        EnableWindow(item(ID_PLAY), (ok && !running) as i32);
        set_text(ID_BROWSE, if ok { "Change..." } else { "Browse..." });
        ShowWindow(item(ID_OPENLOG), if home && err { SW_SHOW } else { SW_HIDE });
        ShowWindow(item(ID_BUG), if home && err { SW_SHOW } else { SW_HIDE });
        disc_tip();
    }
}

thread_local! {
    static TIP: std::cell::Cell<HWND> = const { std::cell::Cell::new(null_mut()) };
}

/// P34 R7: tooltip with the full disc path (the home page shows it shortened) over the path line
/// and the Browse.../Change... button
unsafe fn disc_tip() {
    unsafe {
        let (main, k, path) = with(|u| (u.main, u.scale, u.st.game.iso.clone()));
        let mut tip = TIP.with(|t| t.get());
        if tip.is_null() {
            tip = CreateWindowExW(WS_EX_TOPMOST, w("tooltips_class32").as_ptr(), null(), WS_POPUP | TTS_ALWAYSTIP as u32 | TTS_NOPREFIX as u32, 0, 0, 0, 0, main, null_mut(), GetModuleHandleW(null()), null());
            send(tip, TTM_SETMAXTIPWIDTH, 0, (600.0 * k) as isize);
            TIP.with(|t| t.set(tip));
        }
        let mut text = w(&path.map_or("No disc image chosen yet: Browse... for your Scooby-Doo! Mystery Mayhem disc image (.iso / .bin / .img / .cue)".into(), |p| format!("Disc image: {}", p.display())));
        let sc = |v: i32| (v as f32 * k).round() as i32;
        let mut ti: TTTOOLINFOW = std::mem::zeroed();
        ti.cbSize = std::mem::size_of::<TTTOOLINFOW>() as u32;
        ti.hwnd = main;
        ti.lpszText = text.as_mut_ptr();
        // tool 1: the path / status text (painted), tool 2: the button (re-created by rebuild)
        ti.uFlags = TTF_SUBCLASS;
        ti.uId = 1;
        ti.rect = RECT { left: sc(400), top: sc(300), right: sc(730), bottom: sc(370) };
        send(tip, TTM_DELTOOLW, 0, &ti as *const _ as isize);
        send(tip, TTM_ADDTOOLW, 0, &ti as *const _ as isize);
        ti.uFlags = TTF_SUBCLASS | TTF_IDISHWND;
        ti.uId = item(ID_BROWSE) as usize;
        send(tip, TTM_DELTOOLW, 0, &ti as *const _ as isize);
        send(tip, TTM_ADDTOOLW, 0, &ti as *const _ as isize);
    }
}

/// --shot: render the (offscreen) window into a 24-bit BMP
/// returns the number of distinct colours (a blank capture has a handful)
unsafe fn shot(h: HWND, path: &str) -> Result<usize, String> {
    unsafe {
        let mut r: RECT = std::mem::zeroed();
        GetWindowRect(h, &mut r);
        let (cw, ch) = (r.right - r.left, r.bottom - r.top);
        let sdc = GetDC(null_mut());
        let mdc = CreateCompatibleDC(sdc);
        let bmp = CreateCompatibleBitmap(sdc, cw, ch);
        SelectObject(mdc, bmp as HGDIOBJ);
        if windows_sys::Win32::Storage::Xps::PrintWindow(h, mdc, PW_RENDERFULLCONTENT) == 0 {
            return Err("PrintWindow failed".into());
        }
        let stride = ((cw * 3 + 3) & !3) as usize;
        let mut bits = vec![0u8; stride * ch as usize];
        let mut bi: BITMAPINFO = std::mem::zeroed();
        bi.bmiHeader = BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: cw, biHeight: ch, biPlanes: 1, biBitCount: 24, biCompression: BI_RGB, ..std::mem::zeroed() };
        GetDIBits(mdc, bmp, 0, ch as u32, bits.as_mut_ptr() as *mut _, &mut bi, DIB_RGB_COLORS);
        let mut f = Vec::with_capacity(54 + bits.len());
        f.extend(b"BM");
        f.extend(((54 + bits.len()) as u32).to_le_bytes());
        f.extend(0u32.to_le_bytes());
        f.extend(54u32.to_le_bytes());
        let hdr = std::slice::from_raw_parts(&bi.bmiHeader as *const _ as *const u8, 40);
        f.extend(hdr);
        f.extend(&bits);
        DeleteObject(bmp as HGDIOBJ);
        DeleteDC(mdc);
        ReleaseDC(null_mut(), sdc);
        let colours: std::collections::HashSet<&[u8]> = bits.chunks(3).collect();
        std::fs::write(path, &f).map_err(|e| e.to_string())?;
        Ok(colours.len())
    }
}

/// P39: the launcher is a windows-subsystem exe: a panic would end it without a word. Log it to
/// `<logs>\launcher.log` and tell the player where.
fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |p| {
        use std::io::Write;
        let dir = settings::log_dir();
        let _ = std::fs::create_dir_all(&dir);
        let log = dir.join("launcher.log");
        let rec = format!("=== LAUNCHER PANIC version {}: {p}\n{}\n", scooby_recomp::crashlog::version(), std::backtrace::Backtrace::force_capture());
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&log) {
            let _ = f.write_all(rec.as_bytes());
        }
        prev(p);
        // a panic caught on purpose (disc_textures) is logged, not worth a message box
        if !CATCHING.load(std::sync::atomic::Ordering::Relaxed) {
            msg(&format!("The launcher hit an internal error:\n{p}\n\nDetails: {}", log.display()));
        }
    }));
}

/// set while a panic is caught on purpose (see [`install_panic_hook`])
static CATCHING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn main() {
    install_panic_hook();
    let args = scooby_recomp::crashlog::args();
    let opt = |n: &str| args.iter().position(|a| a == n).and_then(|i| args.get(i + 1)).cloned();
    let shot_path = opt("--shot");
    // --tab home (default) / 0..3 = a settings tab
    let tab: usize = opt("--tab").and_then(|t| t.parse::<usize>().ok()).map_or(HOME, |t| t.min(3));
    let (mut st, path, err) = settings::load();
    let mut early: Option<String> = None;
    if args.iter().any(|a| a == "--print-env") {
        return print_env(&st);
    }
    // debug: --dump-art ISO FILE DIR = every texture of a disc TXD as DIR/<name>_<w>x<h>.rgba
    if let Some(i) = args.iter().position(|a| a == "--dump-art") {
        let (iso, file, dir) = (&args[i + 1], &args[i + 2], &args[i + 3]);
        for t in disc_textures(std::path::Path::new(iso), file) {
            let _ = std::fs::write(format!("{dir}/{}_{}x{}.rgba", t.name, t.width, t.height), &t.rgba);
        }
        return;
    }
    let shift = unsafe { GetAsyncKeyState(VK_SHIFT_I32) } < 0;
    if st.launcher.skip && shot_path.is_none() && !args.iter().any(|a| a == "--settings") && !shift {
        // skip-launcher: start the game; only come up when it dies within 10 s
        match play(&st, &args) {
            Err(e) => early = Some(e),
            Ok(mut r) => loop {
                match r.child.try_wait() {
                    Ok(Some(c)) if crashed(&r, c.success()) => break early = Some(exit_report(&r, c.code())),
                    Ok(None) if r.start.elapsed().as_secs() < 10 => std::thread::sleep(std::time::Duration::from_millis(200)),
                    _ => return,
                }
            },
        }
    }
    let dir = exe_dir();
    // the disc: the stored one, else auto-detect in the usual places (stored when found)
    let iso = match st.game.iso.clone() {
        Some(p) => settings::validate_iso_within(&p, DISC_PROBE),
        None => match settings::find_iso(&dir) {
            Some(p) => {
                let r = settings::validate_iso_within(&p, DISC_PROBE);
                st.game.iso = Some(p);
                if shot_path.is_none() {
                    // P39 R7: atomic (a full disk / a crash mid-write never leaves a cut settings.toml)
                    let _ = st.save(&path);
                }
                r
            }
            None => Err("No disc image chosen yet.".to_string()),
        },
    };
    unsafe {
        SetProcessDPIAware();
        let icc = INITCOMMONCONTROLSEX { dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32, dwICC: ICC_TAB_CLASSES | ICC_BAR_CLASSES | ICC_STANDARD_CLASSES };
        InitCommonControlsEx(&icc);
        let inst = GetModuleHandleW(null());
        let cls = w("ScoobyLauncher");
        let wc = WNDCLASSW { style: 0, lpfnWndProc: Some(wndproc), cbClsExtra: 0, cbWndExtra: 0, hInstance: inst, hIcon: LoadIconW(inst, 1 as _), hCursor: LoadCursorW(null_mut(), IDC_ARROW), hbrBackground: GetSysColorBrush(COLOR_WINDOW), lpszMenuName: null(), lpszClassName: cls.as_ptr() };
        RegisterClassW(&wc);
        let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_CLIPCHILDREN;
        let scale = GetDpiForSystem() as f32 / 96.0;
        let mut r = RECT { left: 0, top: 0, right: (760.0 * scale) as i32, bottom: (512.0 * scale) as i32 };
        AdjustWindowRect(&mut r, style, 0);
        let (ww, wh) = (r.right - r.left, r.bottom - r.top);
        let (x, y, ex) = if shot_path.is_some() { (-20000, -20000, WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE) } else { {
            let (x, y) = place(st.launcher.pos, ww, wh);
            (x, y, 0)
        } };
        let title = w("Scooby-Doo! Mystery Mayhem");
        let main = CreateWindowExW(ex, cls.as_ptr(), title.as_ptr(), style, x, y, ww, wh, null_mut(), null_mut(), inst, null());
        let font = CreateFontW(-(15.0 * scale) as i32, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, w("Segoe UI").as_ptr());
        let big_font = CreateFontW(-(30.0 * scale) as i32, 0, 0, 0, 800, 0, 0, 0, 1, 0, 0, 5, 0, w("Segoe UI").as_ptr());
        let remaster = settings::remaster_exe(&dir);
        let texpack = settings::texpack_dir(&dir);
        UI.with(|u| *u.borrow_mut() = Some(Ui { main, base: st.clone(), st, path, pages: Vec::new(), action: 0, keys: key_names(), remaster, texpack, font, big_font, page: HOME, iso, error: early, running: None, bg: None, capture: None, scale }));
        build();
        background();
        show_tab(tab);
        with(|u| u.st.launcher.tab = u.base.launcher.tab);
        if let Some(e) = err {
            if shot_path.is_none() {
                msg(&format!("settings.toml: {e}\nUsing the defaults (Save overwrites the file)."));
            }
        }
        if let Some(p) = shot_path {
            ShowWindow(main, SW_SHOWNOACTIVATE);
            UpdateWindow(main);
            // --focus N: the ring after N Tab presses from the page's first control (no SetFocus:
            // that could activate the offscreen window)
            if let Some(n) = opt("--focus").and_then(|v| v.parse::<usize>().ok()) {
                let mut f = item(if tab == HOME { ID_PLAY } else { ID_TAB });
                for _ in 0..n {
                    f = GetNextDlgTabItem(main, f, 0);
                }
                ring_at(main, f);
            }
            let mut m: MSG = std::mem::zeroed();
            // PrintWindow right after the first paint sometimes returns a blank frame: retry
            let mut r = Err(String::new());
            // (and a frame can be half painted: always paint a few times, keep the last)
            for i in 0..10 {
                RedrawWindow(main, null(), null_mut(), RDW_INVALIDATE | RDW_UPDATENOW | RDW_ALLCHILDREN);
                for _ in 0..30 {
                    while PeekMessageW(&mut m, null_mut(), 0, 0, PM_REMOVE) != 0 {
                        TranslateMessage(&m);
                        DispatchMessageW(&m);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                r = shot(main, &p);
                if i >= 5 && r.as_ref().is_ok_and(|&colours| colours > 40) {
                    break;
                }
            }
            DestroyWindow(main);
            if let Err(e) = r {
                eprintln!("launcher: shot failed: {e}");
                std::process::exit(1);
            }
            return;
        }
        ShowWindow(main, SW_SHOW);
        SetTimer(main, TIMER_PAD, 30, None);
        let mut m: MSG = std::mem::zeroed();
        // R13: first run (no disc image set yet): start on Browse..., the one thing to do
        // R19: also for a stored disc that is bad / gone (PLAY is disabled then)
        let iso_ok = with(|u| u.iso.is_ok());
        SetFocus(item(home_focus(tab == HOME, iso_ok)));
        while GetMessageW(&mut m, null_mut(), 0, 0) > 0 {
            // press-to-bind: the next key is the binding (Esc cancels; keys are swallowed meanwhile)
            if (m.message == WM_KEYDOWN || m.message == WM_SYSKEYDOWN) && with(|u| u.capture.is_some()) {
                if m.wParam == windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE as usize {
                    capture_end(None);
                } else if with(|u| u.capture != Some(2)) {
                    if let Some(n) = vk_name(m.wParam as u32, m.lParam) {
                        capture_end(Some(&n));
                    }
                }
                continue;
            }
            // ... or the next mouse button (a click anywhere in the launcher, not handed to the control)
            if let Some(n) = mouse_name(m.message, m.wParam).filter(|_| with(|u| matches!(u.capture, Some(0 | 1)))) {
                capture_end(Some(n));
                continue;
            }
            // keyboard-only use: Tab / Shift+Tab walk the controls (creation order: tabs, page,
            // skip, Save, Play, Quit), arrows change the focused control (or the tab), Enter = Play,
            // Esc = Quit, Ctrl+Tab / Ctrl+Shift+Tab switch tabs from anywhere
            if m.message == WM_KEYDOWN && m.wParam == VK_TAB as usize && GetKeyState(VK_CONTROL as i32) < 0 && with(|u| u.page != HOME) {
                let n = ntabs();
                let cur = send(item(ID_TAB), TCM_GETCURSEL, 0, 0) as usize;
                show_tab(if GetKeyState(VK_SHIFT as i32) < 0 { (cur + n - 1) % n } else { (cur + 1) % n });
                SetFocus(item(ID_TAB));
                continue;
            }
            if IsDialogMessageW(main, &m) == 0 {
                TranslateMessage(&m);
                DispatchMessageW(&m);
            }
        }
    }
}

const VK_SHIFT_I32: i32 = 0x10;

/// R19: the disc box on the home page: (status text height, show the hint/path line). A bad stored disc
/// gets 3 lines for its reason (it was cut off and ran into the hint line) and no hint line.
fn disc_box(iso_ok: bool, path_set: bool) -> (i32, bool) {
    if !iso_ok && path_set { (62, false) } else { (40, true) }
}

/// R13/R19: the control focused when the launcher opens: Browse... whenever PLAY is disabled by the disc
fn home_focus(home: bool, iso_ok: bool) -> i32 {
    if !home { ID_TAB } else if iso_ok { ID_PLAY } else { ID_BROWSE }
}

#[cfg(test)]
mod tests {
    #[test]
    fn p34_r19_disc_box_and_focus() {
        use super::*;
        assert_eq!(disc_box(false, true), (62, false));
        assert_eq!(disc_box(false, false), (40, true));
        assert_eq!(disc_box(true, true), (40, true));
        // the 3-line reason ends above the Browse button (y 376)
        assert!(308 + disc_box(false, true).0 <= 376);
        assert_eq!(home_focus(true, false), ID_BROWSE);
        assert_eq!(home_focus(true, true), ID_PLAY);
        assert_eq!(home_focus(false, false), ID_TAB);
    }

    use super::*;

    #[test]
    fn p34_vk_names() {
        // lParam bit 24 = extended key, bits 16-23 = scan code
        assert_eq!(vk_name(0x41, 0).as_deref(), Some("KeyA"));
        assert_eq!(vk_name(0x37, 0).as_deref(), Some("Digit7"));
        assert_eq!(vk_name(0x0d, 0).as_deref(), Some("Enter"));
        assert_eq!(vk_name(0x0d, 1 << 24).as_deref(), Some("NumpadEnter"));
        assert_eq!(vk_name(0x10, 0x2a << 16).as_deref(), Some("ShiftLeft"));
        assert_eq!(vk_name(0x10, 0x36 << 16).as_deref(), Some("ShiftRight"));
        assert_eq!(vk_name(0x11, 1 << 24).as_deref(), Some("ControlRight"));
        assert_eq!(vk_name(0x26, 0).as_deref(), Some("ArrowUp"));
        assert_eq!(vk_name(0x70, 0).as_deref(), Some("F1"));
        // every produced name is one the game's Controls parser knows
        for vk in 0..=0xffu32 {
            for lp in [0isize, 1 << 24, 0x36 << 16] {
                if let Some(n) = vk_name(vk, lp) {
                    assert!(key_names().contains(&n), "{n}");
                }
            }
        }
        assert_eq!(vk_name(0x5b, 0), None); // Windows key: not bindable
    }

    #[test]
    fn p41_cap_press() {
        let a = 0x1000; // XINPUT_GAMEPAD_A
        let mut p = None;
        // capture started with A held (the press that opened it): no bind; release, press = bind
        assert_eq!(cap_press(&mut p, Some((0, a)), true), 0);
        assert_eq!(cap_press(&mut p, Some((0, 0)), true), 0);
        assert_eq!(cap_press(&mut p, Some((0, a)), true), a);
        // pad unplugged during the capture, re-plugged with A held: no bind until a fresh press
        let mut p = Some((0, 0));
        assert_eq!(cap_press(&mut p, None, true), 0);
        assert_eq!(cap_press(&mut p, Some((0, a)), true), 0);
        assert_eq!(cap_press(&mut p, Some((0, a)), true), 0);
        // the first connected pad changes (pad 0 out, pad 1 holding A): no bind
        let mut p = Some((0, 0));
        assert_eq!(cap_press(&mut p, Some((1, a)), true), 0);
        // launcher in the background: presses there never bind, nor what is held on return
        let mut p = Some((0, 0));
        assert_eq!(cap_press(&mut p, Some((0, a)), false), 0);
        assert_eq!(cap_press(&mut p, Some((0, a)), true), 0);
        assert_eq!(cap_press(&mut p, Some((0, 0)), true), 0);
        assert_eq!(cap_press(&mut p, Some((0, a)), true), a);
    }

    #[test]
    fn p34_mouse_names() {
        for (m, wp, n) in [(WM_LBUTTONDOWN, 0, "Mouse.Left"), (WM_RBUTTONDOWN, 0, "Mouse.Right"), (WM_MBUTTONDOWN, 0, "Mouse.Middle"), (WM_XBUTTONDOWN, 1 << 16, "Mouse.Back"), (WM_XBUTTONDOWN, 2 << 16, "Mouse.Forward")] {
            assert_eq!(mouse_name(m, wp), Some(n));
            assert!(key_names().contains(&n.to_string()));
        }
        assert_eq!(mouse_name(WM_LBUTTONUP, 0), None);
        assert_eq!(mouse_name(WM_MOUSEMOVE, 0), None);
    }

    #[test]
    fn p34_home_art_cache() {
        let tmp = std::env::temp_dir().join(format!("p34_art_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        // SAFETY: the only test touching LOCALAPPDATA
        unsafe { std::env::set_var("LOCALAPPDATA", &tmp) };
        let iso = tmp.join("fake.iso");
        std::fs::write(&iso, b"not a disc").unwrap();
        // no cache + not a disc -> None (painted fallback)
        assert!(home_art(&iso).is_none());
        // a valid cache entry for this disc path + size is used without reading the disc
        let cache = settings::log_dir().with_file_name("cache");
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(cache.join("home_logo.rgba"), vec![1u8; 280 * 205 * 4]).unwrap();
        std::fs::write(cache.join("home_gang.rgba"), vec![2u8; 512 * 180 * 4]).unwrap();
        std::fs::write(cache.join("home.key"), format!("{}|10|1", iso.display())).unwrap();
        let art = home_art(&iso).expect("cached art");
        assert_eq!((art[0].0, art[0].1, art[1].0, art[1].1), (280, 205, 512, 180));
        assert!(art[1].2.iter().all(|&b| b == 2));
        // another disc size -> stale key -> re-decode (fails on the fake disc)
        std::fs::write(&iso, b"other").unwrap();
        assert!(home_art(&iso).is_none());
        // a truncated cache file is not used
        std::fs::write(&iso, b"not a disc").unwrap();
        std::fs::write(cache.join("home_gang.rgba"), vec![2u8; 16]).unwrap();
        assert!(home_art(&iso).is_none());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// P34 R12: a remembered launcher position stays inside the monitor's work area
    #[test]
    fn p34_clamp_window() {
        let work = RECT { left: 0, top: 0, right: 1920, bottom: 1040 };
        assert_eq!(clamp_to(&work, 100, 50, 800, 600), (100, 50));
        assert_eq!(clamp_to(&work, 1800, 900, 800, 600), (1120, 440));
        assert_eq!(clamp_to(&work, -300, -20, 800, 600), (0, 0));
        assert_eq!(clamp_to(&work, 5, 5, 2500, 1200), (0, 0));
        let left = RECT { left: -1280, top: 0, right: 0, bottom: 1024 };
        assert_eq!(clamp_to(&left, -100, 10, 800, 600), (-800, 10));
        // R12: README reflow (P40 34425c37) puts more words after the URL on the same line
        assert_eq!(bug_report_url("Project page (GitHub): https://github.com/a/b/ Bug reports and questions").as_deref(), Some("https://github.com/a/b/issues"));
        assert_eq!(bug_report_url("Project page (GitHub): {{GITHUB_URL}} Bug reports"), None);
    }
}

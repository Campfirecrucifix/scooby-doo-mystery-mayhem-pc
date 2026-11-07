//! WP-I1: host input mapping (keyboard / mouse / gamepad -> PS2 [`PadState`]), backend-neutral.
//! Bindings are plain names ("Space", "KeyW", "Mouse.Left", "South", "LeftTrigger2"); the game
//! binary resolves them to Bevy codes. Config file: `controls.toml` (flat TOML subset, std-only).
//!
//! Stick bytes follow libpad / EFPlayerDevice_SP::StickValue: 0..255, 128 = centre, x right = 255,
//! y down = 255. The game itself applies a 40/128 inner deadzone (StickValue); the host radial
//! deadzone only removes pad drift before that.

use super::device::{PadState, pad};

/// PS2 button names in config order (name, pad bit).
pub const BUTTONS: [(&str, u16); 16] = [
    ("cross", pad::CROSS),
    ("circle", pad::CIRCLE),
    ("square", pad::SQUARE),
    ("triangle", pad::TRIANGLE),
    ("l1", pad::L1),
    ("r1", pad::R1),
    ("l2", pad::L2),
    ("r2", pad::R2),
    ("l3", pad::L3),
    ("r3", pad::R3),
    ("start", pad::START),
    ("select", pad::SELECT),
    ("up", pad::UP),
    ("down", pad::DOWN),
    ("left", pad::LEFT),
    ("right", pad::RIGHT),
];

/// Keyboard stick directions (name suffixes in `[keyboard]`).
pub const STICK_DIRS: [&str; 8] = ["lstick_up", "lstick_down", "lstick_left", "lstick_right", "rstick_up", "rstick_down", "rstick_left", "rstick_right"];

#[derive(Clone, Debug, PartialEq)]
pub struct Controls {
    /// radial deadzone 0..1 of the stick magnitude
    pub deadzone: f32,
    /// analog trigger value (0..1) above which L2/R2 count as pressed
    pub trigger_threshold: f32,
    /// mouse motion drives the right stick (camera)
    pub mouse_camera: bool,
    /// stick units per pixel of mouse motion per frame
    pub mouse_sensitivity: f32,
    /// P34: invert the camera (right stick / mouse) Y axis
    pub invert_y: bool,
    /// (action, key names): action = PS2 button name or STICK_DIRS entry
    pub keyboard: Vec<(String, Vec<String>)>,
    /// (PS2 button name, gamepad button names)
    pub gamepad: Vec<(String, Vec<String>)>,
}

fn v(a: &str, b: &[&str]) -> (String, Vec<String>) {
    (a.to_string(), b.iter().map(|s| s.to_string()).collect())
}

impl Default for Controls {
    fn default() -> Self {
        Controls {
            deadzone: 0.15,
            trigger_threshold: 0.3,
            mouse_camera: false,
            mouse_sensitivity: 0.05,
            invert_y: false,
            keyboard: vec![
                v("cross", &["Space", "KeyJ"]),
                v("circle", &["KeyL", "KeyO"]),
                v("square", &["KeyU", "KeyK"]),
                v("triangle", &["KeyI", "Backspace"]),
                v("l1", &["KeyQ"]),
                v("r1", &["KeyE"]),
                v("l2", &["KeyZ"]),
                v("r2", &["KeyC"]),
                v("l3", &["ShiftLeft"]),
                v("r3", &["KeyR", "Mouse.Middle"]),
                v("start", &["Enter"]),
                v("select", &["Tab"]),
                v("up", &["Numpad8", "Digit1"]),
                v("down", &["Numpad2", "Digit2"]),
                v("left", &["Numpad4", "Digit3"]),
                v("right", &["Numpad6", "Digit4"]),
                v("lstick_up", &["KeyW"]),
                v("lstick_down", &["KeyS"]),
                v("lstick_left", &["KeyA"]),
                v("lstick_right", &["KeyD"]),
                v("rstick_up", &["ArrowUp"]),
                v("rstick_down", &["ArrowDown"]),
                v("rstick_left", &["ArrowLeft"]),
                v("rstick_right", &["ArrowRight"]),
            ],
            gamepad: vec![
                v("cross", &["South"]),
                v("circle", &["East"]),
                v("square", &["West"]),
                v("triangle", &["North"]),
                v("l1", &["LeftTrigger"]),
                v("r1", &["RightTrigger"]),
                v("l2", &["LeftTrigger2"]),
                v("r2", &["RightTrigger2"]),
                v("l3", &["LeftThumb"]),
                v("r3", &["RightThumb"]),
                v("start", &["Start"]),
                v("select", &["Select"]),
                v("up", &["DPadUp"]),
                v("down", &["DPadDown"]),
                v("left", &["DPadLeft"]),
                v("right", &["DPadRight"]),
            ],
        }
    }
}

/// Radial deadzone: magnitude below `dz` -> 0, rest rescaled to 0..1 keeping direction; clamped to
/// the unit circle.
pub fn radial_deadzone(x: f32, y: f32, dz: f32) -> (f32, f32) {
    let m = (x * x + y * y).sqrt();
    if !m.is_finite() || m <= dz || dz >= 1.0 {
        return (0.0, 0.0);
    }
    let s = ((m - dz) / (1.0 - dz)).min(1.0) / m;
    (x * s, y * s)
}

/// P27 B4: drift-only deadzone for pad sticks: magnitude <= `dz` -> 0, otherwise the raw axes,
/// NOT rescaled. A DualShock's axes reach the game linearly as bytes and StickValue's 40/128
/// per-axis deadzone + 110 full scale is the only shaping the original applies; rescaling here
/// stacked a second deadzone (0.4 deflection read 0 instead of 0.16, 0.7 read 0.6 instead of 0.7:
/// slower speed and, with DM6 flag 8, a slower turn = a wider arc).
pub fn drift_deadzone(x: f32, y: f32, dz: f32) -> (f32, f32) {
    let m = (x * x + y * y).sqrt();
    if !m.is_finite() || m <= dz { (0.0, 0.0) } else { (x, y) }
}

/// -1..1 -> libpad byte (128 centre).
pub fn stick_byte(v: f32) -> u8 {
    (128.0 + v.clamp(-1.0, 1.0) * 127.0).round() as u8
}

/// Host-side combined input for one frame (all sources merged by the caller).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HostInput {
    pub buttons: u16,
    /// sticks, y up positive, -1..1, deadzone already applied
    pub left: (f32, f32),
    pub right: (f32, f32),
    /// analog trigger values 0..1 (L2, R2)
    pub l2: f32,
    pub r2: f32,
}

impl HostInput {
    /// Triggers over the threshold set L2/R2.
    pub fn apply_triggers(&mut self, threshold: f32) {
        if self.l2 > threshold {
            self.buttons |= pad::L2;
        }
        if self.r2 > threshold {
            self.buttons |= pad::R2;
        }
    }

    /// -> PS2 pad (mode 7 DualShock analog). Pressure bytes are filled (255 when held, triggers
    /// analog); the game only reads them in pressure mode.
    pub fn to_pad(&self) -> PadState {
        let mut p = PadState { buttons: self.buttons, lx: stick_byte(self.left.0), ly: stick_byte(-self.left.1), rx: stick_byte(self.right.0), ry: stick_byte(-self.right.1), ..Default::default() };
        // libpad pressure order: right, left, up, down, triangle, circle, cross, square, L1, R1, L2, R2
        for (i, b) in PRESSURE_BITS.iter().enumerate() {
            p.pressure[i] = if self.buttons & b != 0 { 255 } else { 0 };
        }
        p.pressure[10] = p.pressure[10].max((self.l2.clamp(0.0, 1.0) * 255.0) as u8);
        p.pressure[11] = p.pressure[11].max((self.r2.clamp(0.0, 1.0) * 255.0) as u8);
        p
    }
}

/// P41: focus gate for the host input. The PS2 has no focus; on PC an unfocused window must not
/// read the (global) gamepads or the mouse, and the click that re-focuses the window must not
/// reach the game (the mouse is ignored from focus loss until every mouse button is released).
/// Keyboard keys are released by Bevy itself on focus loss (KeyboardFocusLost -> release_all).
/// P41 R6: the mouse also stays blocked for `GRACE` polls after the re-focus (the activating click's
/// button-down can arrive a frame after the focus event) and then until every button is released.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FocusGate {
    pub mouse_blocked: bool,
    pub grace: u8,
}

impl FocusGate {
    pub const GRACE: u8 = 3;
    /// One frame: returns (read keyboard/gamepad, read mouse buttons).
    pub fn update(&mut self, focused: bool, mouse_held: bool) -> (bool, bool) {
        if !focused {
            self.mouse_blocked = true;
            self.grace = Self::GRACE;
            return (false, false);
        }
        if self.grace > 0 {
            self.grace -= 1;
        } else if self.mouse_blocked && !mouse_held {
            self.mouse_blocked = false;
        }
        (true, !self.mouse_blocked)
    }
}

/// P41 R6: OS cursor for (windowed, gameplay, mouse camera on, focused) -> (visible, confined).
/// Fullscreen: always hidden (P41 R1). Windowed: visible and free in menus / pause / PC SETTINGS /
/// movies and while unfocused; with the mouse camera on, hidden + confined to the window in play (so
/// turning the camera can't drag the cursor out onto the desktop / another monitor).
pub fn cursor_state(windowed: bool, gameplay: bool, mouse_camera: bool, focused: bool) -> (bool, bool) {
    let grab = gameplay && mouse_camera && focused;
    (windowed && !grab, grab)
}

/// P41: port-0 motors (EFPlayerDevice_SP +0x54 small on/off, +0x55 big 0..255, the
/// scePadSetActDirect bytes) -> host rumble (strong, weak) 0..1; none while unfocused (the pad
/// must not buzz while the player is in another window; the motors resume on refocus).
pub fn rumble_intensity(motors: (bool, u8), focused: bool) -> (f32, f32) {
    if !focused {
        return (0.0, 0.0);
    }
    (motors.1 as f32 / 255.0, if motors.0 { 1.0 } else { 0.0 })
}

/// P41 R3: the arrow keys in menus (front end, pause menu, PC SETTINGS) = the d-pad, as on the
/// original PC version's key table (arrows = menu navigation); in gameplay they keep their binding
/// (default: camera stick). `name` = Bevy KeyCode debug name -> d-pad action.
pub fn menu_arrow(name: &str) -> Option<&'static str> {
    match name {
        "ArrowUp" => Some("up"),
        "ArrowDown" => Some("down"),
        "ArrowLeft" => Some("left"),
        "ArrowRight" => Some("right"),
        _ => None,
    }
}

/// P41 R3: an episode level ("EP1/L01", "EP2\L01A") = gameplay; LOADER / INTRO / STATUS / ... =
/// front end (menus, title, episode end screens).
pub fn episode_level(dir: &str) -> bool {
    let d = dir.to_ascii_uppercase();
    let b = d.as_bytes();
    b.len() > 4 && b.starts_with(b"EP") && b[2].is_ascii_digit() && matches!(b[3], b'/' | b'\\' | b'L')
}

/// P41 R3: `[game] pause_on_focus_loss` (PC setting, default off): losing the window focus during
/// gameplay presses START once (the game's own pause path), held for `HOLD` logic steps so the
/// game's edge detection sees one press, then released.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FocusPause {
    /// logic step the START press started at
    pub since: Option<u32>,
}

impl FocusPause {
    pub const HOLD: u32 = 3;
    /// One frame: `lost` = the window lost focus this frame; `gameplay` = a level is being played
    /// (no menu, pause, movie, cut scene or loading). Returns whether START is held this frame.
    pub fn update(&mut self, enabled: bool, lost: bool, gameplay: bool, step: u32) -> bool {
        if self.since.is_none() && enabled && lost && gameplay {
            self.since = Some(step);
        }
        match self.since {
            Some(s) if step.wrapping_sub(s) < Self::HOLD => true,
            Some(_) => {
                self.since = None;
                false
            }
            None => false,
        }
    }
}

/// P41 R4: which connected gamepad drives port 0. `pads` = (id, pressed a button this frame,
/// busy = button held / stick past the deadzone / trigger). A fresh button press takes port 0 at
/// once; otherwise a connected current pad keeps it (busy or idle: a second pad whose stick drifts
/// past the deadzone can't steal it, and the rumble target doesn't hop); with no / an unplugged
/// current pad the first busy one takes it. `None` = no gamepad (keyboard only).
pub fn pick_pad<T: Copy + PartialEq>(cur: Option<T>, pads: &[(T, bool, bool)]) -> Option<T> {
    // the current pad pressing too (same frame) keeps it: no flip by query order
    if let Some(c) = cur.filter(|c| pads.iter().any(|p| p.0 == *c && p.1)) {
        return Some(c);
    }
    if let Some(&(id, _, _)) = pads.iter().find(|p| p.1) {
        return Some(id);
    }
    if let Some(c) = cur.filter(|c| pads.iter().any(|p| p.0 == *c)) {
        return Some(c);
    }
    pads.iter().find(|p| p.2).map(|p| p.0)
}

/// P41 R8: two pads at once. The PS2 game reads port 1 only (a pad in port 2 does nothing); here
/// one pad drives port 0 at a time ([`pick_pad`]) and the inputs of two pads are never merged. While
/// the current pad HOLDS a button (`cur_held`: e.g. the vacuum on R1, a run), a press on another pad
/// does not take port 0 (it would cut the held action mid-way); once the current pad is released, a
/// press hands over as before. Sticks / drift never take over (pick_pad).
pub fn pick_pad_held<T: Copy + PartialEq>(cur: Option<T>, cur_held: bool, pads: &[(T, bool, bool)]) -> Option<T> {
    match cur.filter(|c| cur_held && pads.iter().any(|p| p.0 == *c)) {
        Some(c) => Some(c),
        None => pick_pad(cur, pads),
    }
}

/// P41 R5: the logic ticks at 30 Hz (once per 2 vsyncs) but the host polls every display frame, so
/// a tap pressed and released between two logic ticks was lost (the PS2 IOP samples the pad and
/// the game keeps the latest state; a human tap is longer than one 30 Hz frame there, but a light
/// keyboard tap can be 20-30 ms). `latch` = buttons seen since the last logic tick (reset by the
/// caller when a tick ran): a tap reaches the logic for exactly the next tick, with no added delay.
pub fn latch_taps(latch: &mut u16, p: &mut PadState) {
    *latch |= p.buttons;
    let new = *latch & !p.buttons;
    p.buttons |= *latch;
    for (i, b) in PRESSURE_BITS.iter().enumerate() {
        if new & b != 0 {
            p.pressure[i] = 255;
        }
    }
}

/// P41 R6: the tap window of `latch_taps` = (buttons, logic step it belongs to). A new window per logic
/// tick; while the logic does not tick (level load / warm-up hold: the step count is frozen) the
/// window is emptied every poll, so a START tapped during a load can't pause the next level on its
/// first tick (the original doesn't read the pad while it loads; a held button is still seen held).
/// Returns whether to latch this poll.
pub fn tap_window(tap: &mut (u16, u32), step: u32, ticking: bool) -> bool {
    if !ticking || tap.1 != step {
        *tap = (0, step);
    }
    ticking
}

/// P41 R8/R9: PC SETTINGS page edge memory. Fresh presses this step = `b & !prev`. While a
/// press-to-bind runs, and on the step it ends, every button counts as held (`u16::MAX`), so the key /
/// pad button just bound (e.g. to cross) is not a fresh press until it is released and pressed again.
pub fn page_prev(b: u16, capturing: bool, just_ended: bool) -> u16 {
    if capturing || just_ended { u16::MAX } else { b }
}

/// P41 R7: gamepad press-to-bind (PC SETTINGS): the first fresh button press of a pad that was
/// already connected at the previous poll. `pads` = (id, button just pressed this poll). A pad that
/// was just (re)plugged reports what it holds as new presses on its first reading, which must not
/// bind; sticks are never offered (drift can't bind). `known` = the pads of the previous poll.
pub fn bind_press<T: Copy + PartialEq, B: Copy>(known: &mut Vec<T>, pads: &[(T, Option<B>)]) -> Option<B> {
    let hit = pads.iter().find(|p| p.1.is_some() && known.contains(&p.0)).and_then(|p| p.1);
    *known = pads.iter().map(|p| p.0).collect();
    hit
}

/// libpad pressure order (rdata[8..20]): right, left, up, down, triangle, circle, cross, square, L1, R1, L2, R2
const PRESSURE_BITS: [u16; 12] = [pad::RIGHT, pad::LEFT, pad::UP, pad::DOWN, pad::TRIANGLE, pad::CIRCLE, pad::CROSS, pad::SQUARE, pad::L1, pad::R1, pad::L2, pad::R2];

/// P41 R3 (`pc_key_prompts`): short on-screen label of a keyboard binding name.
pub fn key_label(name: &str) -> String {
    if let Some(m) = name.strip_prefix("Mouse.") {
        return format!("Mouse{m}");
    }
    for (p, r) in [("Key", ""), ("Digit", ""), ("Numpad", "Num"), ("Arrow", "")] {
        if let Some(x) = name.strip_prefix(p) {
            return format!("{r}{x}");
        }
    }
    for (p, r) in [("ShiftLeft", "Shift"), ("ShiftRight", "Shift"), ("ControlLeft", "Ctrl"), ("ControlRight", "Ctrl"), ("AltLeft", "Alt"), ("AltRight", "Alt")] {
        if name == p {
            return r.into();
        }
    }
    name.into()
}

/// P41 R3: the keys shown for the icons 74..77 (Triangle, Circle, Cross, Square): the first keyboard
/// binding of each ("?" when unbound).
pub fn prompt_keys(c: &Controls) -> [String; 4] {
    ["triangle", "circle", "cross", "square"].map(|a| {
        c.keyboard.iter().find(|(n, _)| n == a).and_then(|(_, k)| k.first()).map_or("?".into(), |k| key_label(k))
    })
}

pub fn button_bit(name: &str) -> Option<u16> {
    BUTTONS.iter().find(|b| b.0 == name).map(|b| b.1)
}

fn list(v: &[String]) -> String {
    format!("[{}]", v.iter().map(|s| format!("\"{s}\"")).collect::<Vec<_>>().join(", "))
}

impl Controls {
    pub fn to_toml(&self) -> String {
        let mut s = String::from("# Scooby-Doo! Mystery Mayhem recomp controls (see CONTROLS.md). Delete to regenerate.\n\n");
        s += &format!("deadzone = {}\ntrigger_threshold = {}\nmouse_camera = {}\nmouse_sensitivity = {}\ninvert_y = {}\n", self.deadzone, self.trigger_threshold, self.mouse_camera, self.mouse_sensitivity, self.invert_y);
        s += "\n[keyboard]\n";
        for (a, k) in &self.keyboard {
            s += &format!("{a} = {}\n", list(k));
        }
        s += "\n[gamepad]\n";
        for (a, k) in &self.gamepad {
            s += &format!("{a} = {}\n", list(k));
        }
        s
    }

    /// Parse; unknown/missing entries keep the defaults. Errors name the line.
    pub fn parse(text: &str) -> Result<Controls, String> {
        let mut c = Controls::default();
        let mut sect = String::new();
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        for (n, raw) in text.lines().enumerate() {
            let line = strip_comment(raw).trim();
            if line.is_empty() {
                continue;
            }
            let err = |m: &str| format!("controls.toml line {}: {m}", n + 1);
            if let Some(h) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                sect = h.trim().to_string();
                continue;
            }
            let (k, val) = line.split_once('=').ok_or_else(|| err("expected key = value"))?;
            let (k, val) = (k.trim(), val.trim());
            let num = || val.parse::<f32>().ok().filter(|x| x.is_finite()).ok_or_else(|| err("expected number"));
            match (sect.as_str(), k) {
                ("", "deadzone") => c.deadzone = num()?.clamp(0.0, 0.95),
                ("", "trigger_threshold") => c.trigger_threshold = num()?.clamp(0.0, 1.0),
                ("", "mouse_sensitivity") => c.mouse_sensitivity = num()?,
                ("", "mouse_camera") => c.mouse_camera = val.parse().map_err(|_| err("expected true/false"))?,
                ("", "invert_y") => c.invert_y = val.parse().map_err(|_| err("expected true/false"))?,
                ("keyboard", _) | ("gamepad", _) => {
                    let ok = button_bit(k).is_some() || (sect == "keyboard" && STICK_DIRS.contains(&k));
                    if !ok {
                        return Err(err(&format!("unknown action '{k}'")));
                    }
                    let names = parse_list(val).ok_or_else(|| err("expected [\"Name\", ...]"))?;
                    let tbl = if sect == "keyboard" { &mut c.keyboard } else { &mut c.gamepad };
                    match tbl.iter_mut().find(|e| e.0 == k) {
                        Some(e) => e.1 = names,
                        None => tbl.push((k.to_string(), names)),
                    }
                }
                _ => return Err(err(&format!("unknown key '{k}'"))),
            }
        }
        Ok(c)
    }

    /// Load `path`; write defaults when missing. Parse errors -> defaults + message.
    pub fn load_or_create(path: &std::path::Path) -> (Controls, Option<String>) {
        match std::fs::read_to_string(path) {
            Ok(t) => match Controls::parse(&t) {
                Ok(c) => (c, None),
                Err(e) => (Controls::default(), Some(e)),
            },
            Err(_) => {
                let c = Controls::default();
                let msg = crate::engine::settings::write_atomic(path, c.to_toml().as_bytes()).err().map(|e| format!("cannot write {}: {e}", path.display()));
                (c, msg)
            }
        }
    }
}

fn strip_comment(l: &str) -> &str {
    let mut q = false;
    for (i, ch) in l.char_indices() {
        match ch {
            '"' => q = !q,
            '#' if !q => return &l[..i],
            _ => {}
        }
    }
    l
}

/// `["A", "B"]` or a single `"A"`.
fn parse_list(v: &str) -> Option<Vec<String>> {
    let one = |s: &str| s.trim().strip_prefix('"')?.strip_suffix('"').map(str::to_string);
    if let Some(inner) = v.strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
        if inner.trim().is_empty() {
            return Some(vec![]);
        }
        inner.split(',').filter(|s| !s.trim().is_empty()).map(one).collect()
    } else {
        one(v).map(|s| vec![s])
    }
}

impl Controls {
    /// P41: sanity pass after loading (hand-edited settings.toml / controls.toml). `valid_key` /
    /// `valid_pad` say whether a binding name is a real key / mouse button or gamepad button.
    /// - an unknown name is dropped (it used to be ignored silently, so the action looked bound);
    ///   when ALL names of an action were unknown, the action gets its default bindings back;
    /// - an explicit `[]` stays unbound (documented way to unbind), with a warning for keyboard
    ///   cross / start / triangle (menus need them; Esc still gives START);
    /// - one name on several actions is kept (all of them fire, may be intended), with a warning.
    /// Returns the messages (one line each).
    pub fn validate(&mut self, valid_key: &dyn Fn(&str) -> bool, valid_pad: &dyn Fn(&str) -> bool) -> Vec<String> {
        let mut msgs = Vec::new();
        let d = Controls::default();
        for (sect, tbl, defs, ok) in [("keyboard", &mut self.keyboard, &d.keyboard, valid_key), ("gamepad", &mut self.gamepad, &d.gamepad, valid_pad)] {
            for (action, names) in tbl.iter_mut() {
                let had = names.len();
                for n in names.iter().filter(|n| !ok(n)) {
                    msgs.push(format!("settings [{sect}] {action}: unknown name \"{n}\" ignored"));
                }
                names.retain(|n| ok(n));
                if had > 0 && names.is_empty() {
                    if let Some((_, dn)) = defs.iter().find(|(a, _)| a == action) {
                        *names = dn.clone();
                        msgs.push(format!("settings [{sect}] {action}: no valid binding, default {dn:?} used"));
                    }
                } else if had == 0 && sect == "keyboard" && ["cross", "start", "triangle"].contains(&action.as_str()) {
                    msgs.push(format!("settings [{sect}] {action}: unbound (menus need it)"));
                }
            }
            let mut seen: Vec<(&str, &str)> = Vec::new();
            for (action, names) in tbl.iter() {
                for n in names {
                    match seen.iter().find(|(s, _)| s == n) {
                        Some((_, a)) => msgs.push(format!("settings [{sect}] \"{n}\" is bound to both {a} and {action}")),
                        None => seen.push((n, action)),
                    }
                }
            }
        }
        msgs
    }
}

/// P41: `name` is a Bevy KeyCode / `Mouse.`MouseButton (keyboard) or GamepadButton debug name.
#[cfg(feature = "viewer")]
pub fn binding_name_ok(name: &str, keyboard: bool) -> bool {
    use bevy::reflect::{TypeInfo, Typed};
    fn has<T: Typed>(n: &str) -> bool {
        // unit variants by name; `Other(3)`-style tuple variants by their prefix
        let base = n.split_once('(').map_or(n, |(b, _)| b);
        matches!(T::type_info(), TypeInfo::Enum(e) if e.variant_names().contains(&base))
    }
    if !keyboard {
        return has::<bevy::input::gamepad::GamepadButton>(name);
    }
    match name.strip_prefix("Mouse.") {
        Some(m) => has::<bevy::input::mouse::MouseButton>(m),
        None => has::<bevy::input::keyboard::KeyCode>(name),
    }
}

/// P34 R5 press-to-bind: `name` becomes the first binding of `action` in `list` (a [keyboard] or
/// [gamepad] section; a missing action is added) and is removed from every other action, so one
/// key / button never drives two actions. The action's other bindings are kept.
pub fn rebind(list: &mut Vec<(String, Vec<String>)>, action: &str, name: &str) {
    for (a, names) in list.iter_mut() {
        if a != action {
            names.retain(|n| n != name);
        }
    }
    match list.iter_mut().find(|(a, _)| a == action) {
        Some((_, names)) => {
            names.retain(|n| n != name);
            if names.is_empty() {
                names.push(name.to_string());
            } else {
                names[0] = name.to_string();
            }
        }
        None => list.push((action.to_string(), vec![name.to_string()])),
    }
}

#[cfg(test)]
mod p34_rebind_tests {
    use super::*;

    #[test]
    fn p34_rebind() {
        let mut l = vec![v("Cross", &["Space", "Enter"]), v("Circle", &["KeyE"]), v("Square", &[])];
        rebind(&mut l, "Cross", "KeyE");
        assert_eq!(l, vec![v("Cross", &["KeyE", "Enter"]), v("Circle", &[]), v("Square", &[])]);
        rebind(&mut l, "Square", "Enter");
        assert_eq!(l, vec![v("Cross", &["KeyE"]), v("Circle", &[]), v("Square", &["Enter"])]);
        rebind(&mut l, "Cross", "KeyE");
        assert_eq!(l[0], v("Cross", &["KeyE"]));
        rebind(&mut l, "L1", "KeyQ");
        assert_eq!(l.last(), Some(&v("L1", &["KeyQ"])));
        // defaults: every rebinding keeps the section valid for the parser
        let mut c = Controls::default();
        let a = c.keyboard[0].0.clone();
        rebind(&mut c.keyboard, &a, "F5");
        assert_eq!(c.keyboard[0].1[0], "F5");
        assert_eq!(c.keyboard.iter().flat_map(|(_, n)| n).filter(|n| *n == "F5").count(), 1);
        let g = c.gamepad[0].0.clone();
        rebind(&mut c.gamepad, &g, "RightThumb");
        assert_eq!(Controls::parse(&c.to_toml()), Ok(c));
    }
}

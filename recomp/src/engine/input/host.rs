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
            keyboard: vec![
                v("cross", &["Space", "KeyJ"]),
                v("circle", &["KeyL", "KeyO"]),
                v("square", &["KeyU", "KeyK"]),
                v("triangle", &["KeyI"]),
                v("l1", &["KeyQ"]),
                v("r1", &["KeyE"]),
                v("l2", &["KeyZ"]),
                v("r2", &["KeyC"]),
                v("l3", &["ShiftLeft"]),
                v("r3", &["KeyR", "Mouse.Middle"]),
                v("start", &["Enter"]),
                v("select", &["Tab", "Backspace"]),
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
        let order = [pad::RIGHT, pad::LEFT, pad::UP, pad::DOWN, pad::TRIANGLE, pad::CIRCLE, pad::CROSS, pad::SQUARE, pad::L1, pad::R1, pad::L2, pad::R2];
        for (i, b) in order.iter().enumerate() {
            p.pressure[i] = if self.buttons & b != 0 { 255 } else { 0 };
        }
        p.pressure[10] = p.pressure[10].max((self.l2.clamp(0.0, 1.0) * 255.0) as u8);
        p.pressure[11] = p.pressure[11].max((self.r2.clamp(0.0, 1.0) * 255.0) as u8);
        p
    }
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
        s += &format!("deadzone = {}\ntrigger_threshold = {}\nmouse_camera = {}\nmouse_sensitivity = {}\n", self.deadzone, self.trigger_threshold, self.mouse_camera, self.mouse_sensitivity);
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
            let num = || val.parse::<f32>().map_err(|_| err("expected number"));
            match (sect.as_str(), k) {
                ("", "deadzone") => c.deadzone = num()?.clamp(0.0, 0.95),
                ("", "trigger_threshold") => c.trigger_threshold = num()?.clamp(0.0, 1.0),
                ("", "mouse_sensitivity") => c.mouse_sensitivity = num()?,
                ("", "mouse_camera") => c.mouse_camera = val.parse().map_err(|_| err("expected true/false"))?,
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
                let msg = std::fs::write(path, c.to_toml()).err().map(|e| format!("cannot write {}: {e}", path.display()));
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

//! WP-I1: Bevy side of the host input (keyboard, mouse, gilrs gamepads -> PS2 PadState).
//! Mapping/deadzone/config logic lives in scooby_recomp::engine::input::host (unit tested).
use bevy::ecs::system::SystemParam;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use scooby_recomp::engine::input::PadState;
use scooby_recomp::engine::input::host::{Controls, FocusGate, HostInput, binding_name_ok, button_bit, drift_deadzone, menu_arrow, radial_deadzone};

#[derive(Resource)]
pub struct ControlsRes(pub Controls);

/// P41: the gamepad that drives port 0 (the most recently active one) and the window focus of
/// this frame; pad_rumble sends the port-0 motors to this pad only.
#[derive(Resource, Default)]
pub struct ActivePad {
    pub pad: Option<Entity>,
    pub focused: bool,
    /// P41 R3: the window lost the focus this frame (focused last frame, not now)
    pub lost: bool,
    /// P41 R3: the last button press came from the keyboard / mouse (not a gamepad)
    pub last_kbd: bool,
}

/// P34: the controls part of settings.toml (game_settings; migrated from controls.toml on first run).
pub fn load_controls() -> ControlsRes {
    let mut c = crate::game_settings::get().controls;
    // P41: unknown names (typos in a hand-edited file) used to be ignored silently
    for m in c.validate(&|n| binding_name_ok(n, true), &|n| binding_name_ok(n, false)) {
        eprintln!("game: {m}");
    }
    ControlsRes(c)
}


/// P41 R8 test hook (headless has no keyboard and no window focus): GAME_KEYSCRIPT="Enter@3100,ArrowDown@4320:120,Mouse.Left@5000"
/// holds the named KeyCode / `Mouse.`MouseButton from runtime step N for D updates (default 6) through
/// ButtonInput, so the real bindings, menus and PC SETTINGS see it; with it set the window counts as focused.
pub fn keyscript_on() -> bool {
    static ON: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| std::env::var_os("GAME_KEYSCRIPT").is_some());
    *ON
}

/// keys GAME_KEYSCRIPT can hold (the default bindings + the menu / rebinding keys)
const SCRIPT_KEYS: [KeyCode; 34] = {
    use KeyCode::*;
    [Enter, Space, Tab, Backspace, Escape, ArrowUp, ArrowDown, ArrowLeft, ArrowRight, KeyW, KeyA, KeyS, KeyD, KeyE, KeyQ, KeyX, KeyJ, KeyI, KeyL, KeyO, KeyU, KeyK, KeyZ, KeyC, KeyR, Digit1, Digit2, Digit3, Digit4, Numpad2, Numpad4, Numpad6, Numpad8, ShiftLeft]
};

pub(crate) fn keyscript(game: Res<crate::Game>, mut keys: ResMut<ButtonInput<KeyCode>>, mut mouse: ResMut<ButtonInput<MouseButton>>) {
    let Ok(v) = std::env::var("GAME_KEYSCRIPT") else { return };
    let f = game.steps;
    for it in v.split(',') {
        let Some((name, at)) = it.trim().split_once('@') else { continue };
        let (at, dur) = at.split_once(':').map(|(a, d)| (a, d.parse().unwrap_or(6))).unwrap_or((at, 6u32));
        let Ok(at) = at.parse::<u32>() else { continue };
        let (start, end) = (f == at, f == at + dur);
        if !start && !end {
            continue;
        }
        match name.strip_prefix("Mouse.") {
            Some(m) => match [MouseButton::Left, MouseButton::Right, MouseButton::Middle, MouseButton::Back, MouseButton::Forward].into_iter().find(|b| format!("{b:?}") == m) {
                Some(b) if start => mouse.press(b),
                Some(b) => mouse.release(b),
                None => eprintln!("game: GAME_KEYSCRIPT: unknown {name}"),
            },
            None => match SCRIPT_KEYS.iter().copied().find(|k| format!("{k:?}") == name) {
                Some(k) if start => keys.press(k),
                Some(k) => keys.release(k),
                None => eprintln!("game: GAME_KEYSCRIPT: unknown {name}"),
            },
        }
        if start {
            eprintln!("game: GAME_KEYSCRIPT {name} down at step {f}");
        }
    }
}

#[derive(SystemParam)]
pub struct HostPad<'w, 's> {
    keys: Res<'w, ButtonInput<KeyCode>>,
    mouse: Res<'w, ButtonInput<MouseButton>>,
    motion: Res<'w, AccumulatedMouseMotion>,
    pads: Query<'w, 's, (Entity, &'static Gamepad)>,
    controls: Res<'w, ControlsRes>,
    windows: Query<'w, 's, &'static Window>,
    gate: Local<'s, FocusGate>,
    /// P41 R7: pads connected at the previous poll (press-to-bind ignores a just-plugged pad)
    known: Local<'s, Vec<Entity>>,
    active: ResMut<'w, ActivePad>,
}

fn pad_held(gp: &Gamepad, name: &str, thr: f32) -> bool {
    match name {
        "LeftTrigger2" => gp.get(GamepadButton::LeftTrigger2).unwrap_or(0.0) > thr,
        "RightTrigger2" => gp.get(GamepadButton::RightTrigger2).unwrap_or(0.0) > thr,
        _ => gp.get_pressed().any(|b| format!("{b:?}") == name),
    }
}

impl HostPad<'_, '_> {
    fn key_held(&self, name: &str) -> bool {
        match name.strip_prefix("Mouse.") {
            Some(m) => !self.gate.mouse_blocked && self.mouse.get_pressed().any(|b| format!("{b:?}") == m),
            None => self.keys.get_pressed().any(|k| format!("{k:?}") == name),
        }
    }

    /// P41 R6: controls.toml mouse_camera (cursor capture in play)
    pub fn mouse_camera(&self) -> bool {
        self.controls.0.mouse_camera
    }

    /// P41 R3: the window lost the focus in the last `read`
    pub fn lost(&self) -> bool {
        self.active.lost
    }

    /// P41 R3: the keys for the PC key prompts while the last input came from the keyboard
    pub fn prompt_keys(&self) -> Option<[String; 4]> {
        self.active.last_kbd.then(|| self.prompt_keys_any())
    }

    pub fn prompt_keys_any(&self) -> [String; 4] {
        scooby_recomp::engine::input::host::prompt_keys(&self.controls.0)
    }

    /// `menu`: a menu is open (front end, pause, PC SETTINGS): the arrow keys are the d-pad.
    pub fn read(&mut self, menu: bool) -> PadState {
        // P41: unfocused -> idle pad (gamepads are read globally; alt-tab must not leave the hero
        // running or let another app's pad input drive the game); the re-focus click is swallowed
        let focused = self.windows.iter().any(|w| w.focused) || keyscript_on();
        let (on, mouse_on) = self.gate.update(focused, self.mouse.get_pressed().next().is_some());
        self.active.lost = self.active.focused && !focused;
        self.active.focused = focused;
        if !on {
            return HostInput::default().to_pad();
        }
        // P34 R5: PC SETTINGS press-to-bind: the next key / mouse button (1) or gamepad button (2)
        // becomes the binding; Escape (keyboard) / Start (gamepad) cancels; the game sees an idle pad
        match crate::game_settings::capturing() {
            0 => {}
            mode => {
                let mut got: Option<Option<String>> = None;
                if self.keys.just_pressed(KeyCode::Escape) {
                    got = Some(None);
                } else if mode == 1 {
                    let k = self.keys.get_just_pressed().next().map(|k| format!("{k:?}"));
                    let m = if mouse_on { self.mouse.get_just_pressed().next().map(|b| format!("Mouse.{b:?}")) } else { None };
                    got = k.or(m).map(Some);
                } else {
                    let list: Vec<_> = self.pads.iter().map(|(e, gp)| (e, gp.get_just_pressed().next().copied())).collect();
                    if let Some(b) = scooby_recomp::engine::input::host::bind_press(&mut self.known, &list) {
                        got = Some((b != GamepadButton::Start).then(|| format!("{b:?}")));
                    }
                }
                if keyscript_on() && got.is_some() {
                    eprintln!("game: GAME_KEYSCRIPT capture {got:?}");
                }
                if let Some(g) = got {
                    crate::game_settings::capture_done(g);
                }
                return HostInput::default().to_pad();
            }
        }
        *self.known = self.pads.iter().map(|(e, _)| e).collect();
        if self.keys.get_just_pressed().next().is_some() || (mouse_on && self.mouse.get_just_pressed().next().is_some()) {
            self.active.last_kbd = true;
        } else if self.pads.iter().any(|(_, gp)| gp.get_just_pressed().next().is_some()) {
            self.active.last_kbd = false;
        }
        let c = &self.controls.0;
        let mut h = HostInput::default();
        let mut dirs = [false; 8];
        if menu {
            for k in self.keys.get_pressed() {
                if let Some(a) = menu_arrow(&format!("{k:?}")) {
                    h.buttons |= button_bit(a).unwrap_or(0);
                }
            }
        }
        for (action, names) in &c.keyboard {
            if !names.iter().any(|n| self.key_held(n) && !(menu && action.starts_with("rstick") && menu_arrow(n).is_some())) {
                continue;
            }
            match button_bit(action) {
                Some(bit) => h.buttons |= bit,
                None => {
                    if let Some(i) = scooby_recomp::engine::input::host::STICK_DIRS.iter().position(|d| d == action) {
                        dirs[i] = true;
                    }
                }
            }
        }
        let ax = |n: bool, p: bool| (p as i32 - n as i32) as f32;
        let norm = |x: f32, y: f32| radial_deadzone(x, y, 0.0);
        // P27 B4: keyboard stick keys = full deflection per axis (bytes 0/255 like a PCSX2 key bind,
        // P25 R34); a diagonal is not shrunk to the unit circle (0.714 per axis after StickValue).
        h.left = (ax(dirs[2], dirs[3]), ax(dirs[1], dirs[0]));
        h.right = (ax(dirs[6], dirs[7]), ax(dirs[5], dirs[4]));
        if c.mouse_camera {
            let d = self.motion.delta * c.mouse_sensitivity;
            if d != Vec2::ZERO {
                h.right = norm(h.right.0 + d.x, h.right.1 - d.y);
            }
        }
        // gamepads: hot-plugged by bevy_gilrs; the most recently active pad wins
        let thr = c.trigger_threshold;
        let busy = |gp: &Gamepad| {
            gp.get_pressed().next().is_some()
                || gp.left_stick().length() > c.deadzone
                || gp.right_stick().length() > c.deadzone
                || gp.get(GamepadButton::LeftTrigger2).unwrap_or(0.0) > thr
                || gp.get(GamepadButton::RightTrigger2).unwrap_or(0.0) > thr
        };
        // P41 R4: a button press takes port 0 (pick_pad); unplugged pads drop out of the query
        let list: Vec<_> = self.pads.iter().map(|(e, gp)| (e, gp.get_just_pressed().next().is_some(), busy(gp))).collect();
        // P41 R8: a pad holding a button keeps port 0 (a second pad's press doesn't cut it)
        let held = self.active.pad.and_then(|a| self.pads.get(a).ok()).is_some_and(|(_, gp)| gp.get_pressed().next().is_some() || gp.get(GamepadButton::LeftTrigger2).unwrap_or(0.0) > thr || gp.get(GamepadButton::RightTrigger2).unwrap_or(0.0) > thr);
        self.active.pad = scooby_recomp::engine::input::host::pick_pad_held(self.active.pad, held, &list);
        if let Some((_, gp)) = self.active.pad.and_then(|a| self.pads.get(a).ok()) {
            for (action, names) in &c.gamepad {
                if names.iter().any(|n| pad_held(gp, n, thr)) {
                    h.buttons |= button_bit(action).unwrap_or(0);
                }
            }
            let (ls, rs) = (gp.left_stick(), gp.right_stick());
            let l = drift_deadzone(ls.x, ls.y, c.deadzone);
            let r = drift_deadzone(rs.x, rs.y, c.deadzone);
            if l != (0.0, 0.0) {
                h.left = l;
            }
            if r != (0.0, 0.0) {
                h.right = r;
            }
            h.l2 = gp.get(GamepadButton::LeftTrigger2).unwrap_or(0.0);
            h.r2 = gp.get(GamepadButton::RightTrigger2).unwrap_or(0.0);
        } else {
            self.active.pad = None;
        }
        if c.invert_y {
            h.right.1 = -h.right.1;
        }
        h.to_pad()
    }
}

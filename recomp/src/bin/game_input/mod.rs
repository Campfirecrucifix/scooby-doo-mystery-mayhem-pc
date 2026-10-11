//! WP-I1: Bevy side of the host input (keyboard, mouse, gilrs gamepads -> PS2 PadState).
//! Mapping/deadzone/config logic lives in scooby_recomp::engine::input::host (unit tested).
use bevy::ecs::system::SystemParam;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use scooby_recomp::engine::input::PadState;
use scooby_recomp::engine::input::host::{Controls, HostInput, button_bit, radial_deadzone};

#[derive(Resource)]
pub struct ControlsRes(pub Controls);

/// `controls.toml` in the cwd (else next to the exe when only that one exists); written on first run.
pub fn load_controls() -> ControlsRes {
    let cwd = std::path::PathBuf::from("controls.toml");
    let exe = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("controls.toml")));
    let path = match exe {
        Some(e) if !cwd.exists() && e.exists() => e,
        _ => cwd,
    };
    let (c, err) = Controls::load_or_create(&path);
    if let Some(e) = err {
        eprintln!("controls: {e} (using defaults)");
    }
    ControlsRes(c)
}

#[derive(SystemParam)]
pub struct HostPad<'w, 's> {
    keys: Res<'w, ButtonInput<KeyCode>>,
    mouse: Res<'w, ButtonInput<MouseButton>>,
    motion: Res<'w, AccumulatedMouseMotion>,
    pads: Query<'w, 's, (Entity, &'static Gamepad)>,
    controls: Res<'w, ControlsRes>,
    active: Local<'s, Option<Entity>>,
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
            Some(m) => self.mouse.get_pressed().any(|b| format!("{b:?}") == m),
            None => self.keys.get_pressed().any(|k| format!("{k:?}") == name),
        }
    }

    pub fn read(&mut self) -> PadState {
        let c = &self.controls.0;
        let mut h = HostInput::default();
        let mut dirs = [false; 8];
        for (action, names) in &c.keyboard {
            if !names.iter().any(|n| self.key_held(n)) {
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
        h.left = norm(ax(dirs[2], dirs[3]), ax(dirs[1], dirs[0]));
        h.right = norm(ax(dirs[6], dirs[7]), ax(dirs[5], dirs[4]));
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
        let cur_busy = self.active.and_then(|a| self.pads.get(a).ok()).is_some_and(|(_, gp)| busy(gp));
        if !cur_busy {
            if let Some((e, _)) = self.pads.iter().find(|(_, gp)| busy(gp)) {
                *self.active = Some(e);
            }
        }
        if let Some((_, gp)) = self.active.and_then(|a| self.pads.get(a).ok()) {
            for (action, names) in &c.gamepad {
                if names.iter().any(|n| pad_held(gp, n, thr)) {
                    h.buttons |= button_bit(action).unwrap_or(0);
                }
            }
            let (ls, rs) = (gp.left_stick(), gp.right_stick());
            let l = radial_deadzone(ls.x, ls.y, c.deadzone);
            let r = radial_deadzone(rs.x, rs.y, c.deadzone);
            if l != (0.0, 0.0) {
                h.left = l;
            }
            if r != (0.0, 0.0) {
                h.right = r;
            }
            h.l2 = gp.get(GamepadButton::LeftTrigger2).unwrap_or(0.0);
            h.r2 = gp.get(GamepadButton::RightTrigger2).unwrap_or(0.0);
        } else {
            *self.active = None;
        }
        h.to_pad()
    }
}

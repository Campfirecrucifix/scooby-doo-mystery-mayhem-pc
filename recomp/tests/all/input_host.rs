//! WP-I1: host input mapping (deadzone, stick bytes, pad mapping, controls.toml parse).
use scooby_recomp::engine::input::host::*;
use scooby_recomp::engine::input::pad;

#[test]
fn deadzone_radial() {
    assert_eq!(radial_deadzone(0.1, 0.1, 0.15), (0.0, 0.0));
    let (x, y) = radial_deadzone(1.0, 0.0, 0.15);
    assert!((x - 1.0).abs() < 1e-6 && y == 0.0);
    // direction kept, magnitude rescaled
    let (x, y) = radial_deadzone(0.0, -0.575, 0.15);
    assert!(x == 0.0 && (y + 0.5).abs() < 1e-5);
    // diagonal corner clamped to the unit circle
    let (x, y) = radial_deadzone(1.0, 1.0, 0.1);
    assert!(((x * x + y * y).sqrt() - 1.0).abs() < 1e-5);
    assert_eq!(radial_deadzone(f32::NAN, 0.0, 0.1), (0.0, 0.0));
}

#[test]
fn deadzone_drift_only() {
    assert_eq!(drift_deadzone(0.1, 0.1, 0.15), (0.0, 0.0));
    // outside the drift cut the axes pass unchanged (the game's StickValue does the shaping)
    assert_eq!(drift_deadzone(0.0, -0.4, 0.15), (0.0, -0.4));
    assert_eq!(drift_deadzone(1.0, 1.0, 0.15), (1.0, 1.0));
    assert_eq!(drift_deadzone(f32::NAN, 0.0, 0.1), (0.0, 0.0));
}

#[test]
fn stick_bytes_libpad() {
    assert_eq!(stick_byte(0.0), 128);
    assert_eq!(stick_byte(1.0), 255);
    assert_eq!(stick_byte(-1.0), 1);
    assert_eq!(stick_byte(5.0), 255);
    // y up on the host -> y down = 255 on the pad
    let p = HostInput { left: (0.0, 1.0), right: (1.0, 0.0), ..Default::default() }.to_pad();
    assert_eq!((p.lx, p.ly, p.rx, p.ry), (128, 1, 255, 128));
    assert!(p.connected && p.mode == 7);
}

#[test]
fn triggers_and_pressure() {
    let mut h = HostInput { buttons: pad::CROSS, l2: 0.5, r2: 0.2, ..Default::default() };
    h.apply_triggers(0.3);
    assert_eq!(h.buttons, pad::CROSS | pad::L2);
    let p = h.to_pad();
    assert_eq!(p.pressure[6], 255); // cross
    assert_eq!(p.pressure[10], 255); // L2 held
    assert_eq!(p.pressure[11], 51); // R2 analog 0.2
    assert!(!p.pressure_mode);
}

#[test]
fn config_roundtrip_and_parse() {
    let d = Controls::default();
    assert_eq!(Controls::parse(&d.to_toml()).unwrap(), d);
    // every PS2 button bound on keyboard and gamepad
    for (n, _) in BUTTONS {
        assert!(d.keyboard.iter().any(|e| e.0 == n && !e.1.is_empty()), "{n}");
        assert!(d.gamepad.iter().any(|e| e.0 == n && !e.1.is_empty()), "{n}");
    }
    let c = Controls::parse("deadzone = 0.25 # comment\nmouse_camera = true\n[keyboard]\ncross = [\"KeyX\", \"Mouse.Left\"]\nl3 = \"KeyF\"\n[gamepad]\nr2 = []\n").unwrap();
    assert_eq!(c.deadzone, 0.25);
    assert!(c.mouse_camera);
    assert_eq!(c.keyboard.iter().find(|e| e.0 == "cross").unwrap().1, vec!["KeyX", "Mouse.Left"]);
    assert_eq!(c.keyboard.iter().find(|e| e.0 == "l3").unwrap().1, vec!["KeyF"]);
    assert!(c.gamepad.iter().find(|e| e.0 == "r2").unwrap().1.is_empty());
    assert_eq!(c.trigger_threshold, d.trigger_threshold);
    assert!(Controls::parse("[keyboard]\njump = [\"Space\"]").unwrap_err().contains("line 2"));
    assert!(Controls::parse("[gamepad]\nlstick_up = [\"X\"]").is_err());
    assert!(Controls::parse("deadzone = abc").is_err());
}

#[test]
fn load_or_create_writes_defaults() {
    let p = std::env::temp_dir().join(format!("wp_i1_controls_{}.toml", std::process::id()));
    let _ = std::fs::remove_file(&p);
    let (c, e) = Controls::load_or_create(&p);
    assert!(e.is_none() && c == Controls::default() && p.exists());
    assert_eq!(Controls::load_or_create(&p).0, c);
    let _ = std::fs::remove_file(&p);
}

/// P24: binding names are compared against Bevy debug names and misspellings are silently ignored:
/// every default gamepad/keyboard name must be a real GamepadButton / KeyCode / MouseButton.
#[cfg(feature = "viewer")]
#[test]
fn default_binding_names_exist_in_bevy() {
    use bevy::input::gamepad::GamepadButton as G;
    use bevy::input::keyboard::KeyCode as K;
    use bevy::input::mouse::MouseButton as M;
    let pads = [G::South, G::East, G::North, G::West, G::C, G::Z, G::LeftTrigger, G::LeftTrigger2, G::RightTrigger, G::RightTrigger2, G::Select, G::Start, G::Mode, G::LeftThumb, G::RightThumb, G::DPadUp, G::DPadDown, G::DPadLeft, G::DPadRight];
    let pads: Vec<String> = pads.iter().map(|b| format!("{b:?}")).collect();
    let d = Controls::default();
    for (a, names) in &d.gamepad {
        for n in names {
            assert!(pads.contains(n), "gamepad {a} = {n} is not a GamepadButton");
        }
    }
    let keys = [K::Space, K::KeyJ, K::KeyL, K::KeyO, K::KeyU, K::KeyK, K::KeyI, K::KeyQ, K::KeyE, K::KeyZ, K::KeyC, K::ShiftLeft, K::KeyR, K::Enter, K::Tab, K::Backspace, K::Numpad8, K::Numpad2, K::Numpad4, K::Numpad6, K::Digit1, K::Digit2, K::Digit3, K::Digit4, K::KeyW, K::KeyA, K::KeyS, K::KeyD, K::ArrowUp, K::ArrowDown, K::ArrowLeft, K::ArrowRight];
    let keys: Vec<String> = keys.iter().map(|k| format!("{k:?}")).chain([M::Left, M::Right, M::Middle].iter().map(|m| format!("Mouse.{m:?}"))).collect();
    for (a, names) in &d.keyboard {
        for n in names {
            assert!(keys.contains(n), "keyboard {a} = {n} is not a KeyCode/MouseButton");
        }
    }
}

/// P41: host bytes through the ported EFPlayerDevice_SP stick conversion (|v-128|-40 over 70,
/// per axis): centre reads exactly 0, keyboard full deflection reads 1.0 on each axis (a diagonal
/// = (1,1) like a DualShock 2 corner / a PCSX2 key bind), an XInput unit-circle diagonal ~0.71/axis.
#[test]
fn p41_stick_ranges_vs_c() {
    use scooby_recomp::engine::input::EFPlayerDevice as D;
    let p = HostInput::default().to_pad();
    assert_eq!((p.lx, p.ly, p.rx, p.ry), (128, 128, 128, 128));
    assert_eq!(D::StickValue(p.lx), 0.0);
    let kb = HostInput { left: (1.0, 1.0), right: (-1.0, -1.0), ..Default::default() }.to_pad();
    assert_eq!((kb.lx, kb.ly, kb.rx, kb.ry), (255, 1, 1, 255));
    for b in [kb.lx, kb.ly, kb.rx, kb.ry] {
        assert_eq!(D::StickValue(b).abs(), 1.0);
    }
    let d = std::f32::consts::FRAC_1_SQRT_2;
    let x = D::StickValue(HostInput { left: drift_deadzone(d, d, 0.15), ..Default::default() }.to_pad().lx);
    assert!((x - 0.714).abs() < 0.002, "{x}");
    // small drift below the host deadzone and the C's own 40/128 inner zone both read 0
    assert_eq!(drift_deadzone(0.1, 0.05, 0.15), (0.0, 0.0));
    assert_eq!(D::StickValue(stick_byte(0.3)), 0.0);
    // y up on the host = byte < 128 = C analog y > 0 (the C negates)
    assert!(HostInput { left: (0.0, 1.0), ..Default::default() }.to_pad().ly < 128);
    // ENGINE.INI has no JoystickPressureSensitivity: the C never enters pressure mode
    assert!(!kb.pressure_mode);
}

/// P41: unfocused -> no input; the click that re-focuses the window is swallowed until released.
#[test]
fn p41_focus_gate() {
    let mut g = FocusGate::default();
    assert_eq!(g.update(true, false), (true, true));
    assert_eq!(g.update(false, false), (false, false));
    assert_eq!(g.update(true, true), (true, false)); // focus click held
    assert_eq!(g.update(true, true), (true, false));
    assert_eq!(g.update(true, false), (true, false)); // grace
    assert_eq!(g.update(true, false), (true, true)); // released after the grace -> mouse live again
    assert_eq!(g.update(true, true), (true, true)); // a later click reaches the game
    // P41 R6: the activating click's button-down arrives a frame after the focus event: still swallowed
    assert_eq!(g.update(false, false), (false, false));
    assert_eq!(g.update(true, false), (true, false));
    assert_eq!(g.update(true, true), (true, false));
    assert_eq!(g.update(true, true), (true, false));
    assert_eq!(g.update(true, true), (true, false), "held past the grace: still blocked");
    assert_eq!(g.update(true, false), (true, true));
}

/// P41 R6: cursor visible + free in menus, hidden + confined in play with the mouse camera (windowed)
#[test]
fn p41_cursor_state() {
    // (windowed, gameplay, mouse camera, focused) -> (visible, confined)
    assert_eq!(cursor_state(true, false, true, true), (true, false), "menu / pause: free");
    assert_eq!(cursor_state(true, true, true, true), (false, true), "play + mouse camera: captured");
    assert_eq!(cursor_state(true, true, false, true), (true, false), "mouse camera off: never captured");
    assert_eq!(cursor_state(true, true, true, false), (true, false), "focus lost: released");
    assert_eq!(cursor_state(false, false, true, true), (false, false), "fullscreen: hidden");
    assert_eq!(cursor_state(false, true, true, true), (false, true));
}

/// P41: scePadSetActDirect bytes -> rumble (strong = big motor 0..255, weak = small on/off), off unfocused.
#[test]
fn p41_rumble_intensity() {
    assert_eq!(rumble_intensity((false, 0), true), (0.0, 0.0));
    assert_eq!(rumble_intensity((true, 255), true), (1.0, 1.0));
    assert_eq!(rumble_intensity((false, 51), true), (0.2, 0.0));
    assert_eq!(rumble_intensity((true, 255), false), (0.0, 0.0));
}

/// P41 R2: settings.toml rebinding edge cases: unknown names, all-unknown -> defaults, explicit
/// unbind kept, one key on two actions kept + reported; the defaults are clean.
#[test]
fn p41_validate_bindings() {
    let key_ok = |n: &str| ["Space", "KeyJ", "KeyE", "KeyI", "Enter", "KeyW", "Mouse.Left"].contains(&n) || n.starts_with("Key") && n.len() == 4 || n.starts_with("Digit") || n.starts_with("Numpad") || n.starts_with("Arrow") || ["ShiftLeft", "Tab", "Backspace", "Mouse.Middle"].contains(&n);
    let pad_ok = |n: &str| ["South", "East", "West", "North", "LeftTrigger", "RightTrigger", "LeftTrigger2", "RightTrigger2", "LeftThumb", "RightThumb", "Start", "Select", "DPadUp", "DPadDown", "DPadLeft", "DPadRight"].contains(&n);
    let mut d = Controls::default();
    assert_eq!(d.validate(&key_ok, &pad_ok), Vec::<String>::new());
    assert_eq!(d, Controls::default());
    // menus: Backspace = Triangle (back), the original's own PC key (DIK_BACK 0x0e on the TRI action)
    let kbd = |a: &str| d.keyboard.iter().find(|e| e.0 == a).unwrap().1.clone();
    assert!(kbd("triangle").contains(&"Backspace".to_string()) && !kbd("select").contains(&"Backspace".to_string()));
    let t = "[keyboard]\ncross = [\"Spcae\", \"KeyJ\"]\ncircle = [\"Foo\"]\nsquare = [\"KeyE\"]\nstart = []\n[gamepad]\ntriangle = [\"Y\"]\n";
    let mut c = Controls::parse(t).unwrap();
    let m = c.validate(&key_ok, &pad_ok);
    let kb = |c: &Controls, a: &str| c.keyboard.iter().find(|e| e.0 == a).unwrap().1.clone();
    assert_eq!(kb(&c, "cross"), vec!["KeyJ"]); // typo dropped, the valid one kept
    assert_eq!(kb(&c, "circle"), vec!["KeyL", "KeyO"]); // nothing valid -> default
    assert!(kb(&c, "start").is_empty()); // explicit unbind kept
    assert_eq!(c.gamepad.iter().find(|e| e.0 == "triangle").unwrap().1, vec!["North"]);
    assert!(m.iter().any(|l| l.contains("\"Spcae\"")), "{m:?}");
    assert!(m.iter().any(|l| l.contains("circle") && l.contains("default")), "{m:?}");
    assert!(m.iter().any(|l| l.contains("start") && l.contains("unbound")), "{m:?}");
    assert!(m.iter().any(|l| l.contains("\"KeyE\" is bound to both square and r1")), "{m:?}");
    // the result is a valid settings file again
    assert_eq!(Controls::parse(&c.to_toml()), Ok(c));
}

/// P41 R2: the real name check used by load_controls (Bevy reflection): every default is valid,
/// typos / wrong section are not.
#[cfg(feature = "viewer")]
#[test]
fn p41_binding_name_ok() {
    let d = Controls::default();
    for (_, n) in &d.keyboard {
        assert!(n.iter().all(|n| binding_name_ok(n, true)), "{n:?}");
    }
    for (_, n) in &d.gamepad {
        assert!(n.iter().all(|n| binding_name_ok(n, false)), "{n:?}");
    }
    for bad in ["Spcae", "A", "Mouse.Foo", "South", ""] {
        assert!(!binding_name_ok(bad, true), "{bad}");
    }
    for bad in ["A", "Space", "Y"] {
        assert!(!binding_name_ok(bad, false), "{bad}");
    }
    assert!(binding_name_ok("F5", true) && binding_name_ok("Mouse.Back", true) && binding_name_ok("Mode", false));
    // a validated default set reports nothing
    let mut c = Controls::default();
    assert!(c.validate(&|n| binding_name_ok(n, true), &|n| binding_name_ok(n, false)).is_empty());
}

#[test]
fn p41_menu_arrows() {
    // arrows = d-pad in menus (original PC table); anything else keeps its binding
    assert_eq!(menu_arrow("ArrowUp"), Some("up"));
    assert_eq!(menu_arrow("ArrowLeft"), Some("left"));
    assert_eq!(menu_arrow("KeyW"), None);
    assert_eq!(button_bit(menu_arrow("ArrowDown").unwrap()), Some(pad::DOWN));
    assert_eq!(button_bit(menu_arrow("ArrowRight").unwrap()), Some(pad::RIGHT));
    // default arrows are the camera stick in gameplay
    let c = Controls::default();
    let k = |a: &str| c.keyboard.iter().find(|(n, _)| n == a).unwrap().1.clone();
    assert_eq!(k("rstick_up"), vec!["ArrowUp".to_string()]);
}

#[test]
fn p41_episode_level() {
    for l in ["EP1/L01", "EP2\\L01A", "ep3/l01", "EP1L03A"] {
        assert!(episode_level(l), "{l}");
    }
    for l in ["LOADER", "INTRO", "STATUS", "SPLASH", "EP02E", "EP", ""] {
        assert!(!episode_level(l), "{l}");
    }
}

#[test]
fn p41_focus_pause() {
    let mut f = FocusPause::default();
    // off / in a menu / no focus loss: nothing
    assert!(!f.update(false, true, true, 10));
    assert!(!f.update(true, true, false, 10));
    assert!(!f.update(true, false, true, 10));
    // on + lost in gameplay: START for HOLD logic steps, then released (one press)
    assert!(f.update(true, true, true, 20));
    assert!(f.update(true, false, false, 20)); // pause menu now open: keep the press going
    assert!(f.update(true, false, false, 21 + FocusPause::HOLD - 2));
    assert!(!f.update(true, false, false, 20 + FocusPause::HOLD));
    assert!(!f.update(true, false, false, 30));
}

#[test]
fn p41_settings_game_keys() {
    use scooby_recomp::engine::settings::Settings;
    let s = Settings::default();
    assert!(!s.game.pause_on_focus_loss && !s.game.pc_key_prompts);
    let t = s.to_toml();
    assert!(t.contains("pause_on_focus_loss = false") && t.contains("pc_key_prompts = false"));
    let p = Settings::parse(&t.replace("pause_on_focus_loss = false", "pause_on_focus_loss = true")).unwrap();
    assert!(p.game.pause_on_focus_loss && !p.game.pc_key_prompts);
    assert!(Settings::parse("[game]\npc_key_prompts = maybe\n").is_err());
}

#[test]
fn p41_prompt_keys() {
    assert_eq!(key_label("KeyI"), "I");
    assert_eq!(key_label("Digit1"), "1");
    assert_eq!(key_label("Numpad8"), "Num8");
    assert_eq!(key_label("ArrowUp"), "Up");
    assert_eq!(key_label("ShiftLeft"), "Shift");
    assert_eq!(key_label("Mouse.Middle"), "MouseMiddle");
    assert_eq!(key_label("Space"), "Space");
    // icon order 74..77 = Triangle, Circle, Cross, Square: first keyboard binding of each
    assert_eq!(prompt_keys(&Controls::default()), ["I", "L", "Space", "U"].map(String::from));
    let mut c = Controls::default();
    c.keyboard.iter_mut().find(|(n, _)| n == "cross").unwrap().1.clear();
    assert_eq!(prompt_keys(&c)[2], "?");
}

/// P41 R4: gamepad hot-plug / two pads: which pad drives port 0
#[test]
fn p41_pick_pad() {
    // (id, pressed this frame, busy)
    assert_eq!(pick_pad::<u8>(None, &[]), None);
    assert_eq!(pick_pad(Some(1u8), &[]), None, "the only pad unplugged -> keyboard only, no panic");
    assert_eq!(pick_pad(None, &[(1u8, false, false)]), None, "an idle pad does not take port 0 yet");
    assert_eq!(pick_pad(None, &[(1u8, false, true)]), Some(1));
    assert_eq!(pick_pad(Some(1u8), &[(1, false, false), (2, false, false)]), Some(1), "idle pad 1 keeps port 0 (rumble target)");
    assert_eq!(pick_pad(Some(1u8), &[(1, false, true), (2, false, true)]), Some(1), "a busy current pad keeps it");
    assert_eq!(pick_pad(Some(1u8), &[(1, false, false), (2, false, true)]), Some(1), "a drifting / stick-only pad 2 doesn't steal port 0");
    // pad 2 has port 0 and drifts (always busy): a button press on pad 1 takes it back
    assert_eq!(pick_pad(Some(2u8), &[(1, false, true), (2, false, true)]), Some(2));
    assert_eq!(pick_pad(Some(2u8), &[(1, true, true), (2, false, true)]), Some(1));
    assert_eq!(pick_pad(Some(1u8), &[(1, false, false), (2, false, true)]), Some(1), "and keeps it");
    // pad 1 unplugged mid-game, pad 2 idle: no pad until pad 2 is used; replug = a new id
    assert_eq!(pick_pad(Some(1u8), &[(2, false, false)]), None);
    assert_eq!(pick_pad(None, &[(2u8, true, true)]), Some(2));
    assert_eq!(pick_pad(Some(2u8), &[(2, false, false), (3, true, true)]), Some(3));
}

/// P41 R5: 30 Hz logic, 60 Hz host polls. A tap released before the next logic tick still reaches it (once,
/// with full pressure); a held button is unchanged; a release shows on the next tick (window reset per tick).
#[test]
fn p41_latch_taps() {
    let poll = |b: u16| HostInput { buttons: b, ..Default::default() }.to_pad();
    // tick window 1: two polls, CROSS down on the first, up on the second (a 16 ms tap)
    let mut latch = 0u16;
    let mut p = poll(pad::CROSS);
    latch_taps(&mut latch, &mut p);
    let mut p = poll(0);
    latch_taps(&mut latch, &mut p);
    assert_eq!(p.buttons & pad::CROSS, pad::CROSS, "the logic tick sees the tap");
    assert_eq!(p.pressure[6], 255, "cross pressure follows the latched bit");
    // next window: reset by the caller; nothing pressed -> released at once (no added hold)
    latch = 0;
    let mut p = poll(0);
    latch_taps(&mut latch, &mut p);
    assert_eq!(p.buttons, 0);
    // held button: unchanged
    let mut p = poll(pad::UP | pad::L2);
    let before = p;
    latch_taps(&mut latch, &mut p);
    assert_eq!(p, before);
    // sticks are not latched (analog = latest value, as the IOP sample)
    latch = 0;
    let mut p = poll(0);
    p.lx = 0x20;
    latch_taps(&mut latch, &mut p);
    assert_eq!(p.lx, 0x20);
}

/// P41 R6: latch_taps edge cases. (a) START tapped during a level load (logic step frozen): not queued
/// into the level's first tick. (b) a tap inside the pause menu's 0.25 s debounce reaches exactly one
/// tick (the game ignores it there, as on PS2) and is never carried to a later tick. (c) a key tapped
/// on the frame focus is lost: Bevy releases the keys, the latched tap reaches one tick only; with
/// pause_on_focus_loss on, the START press joins it (one continuous press = one pause), off = nothing.
#[test]
fn p41_latch_edges() {
    let poll = |b: u16| HostInput { buttons: b, ..Default::default() }.to_pad();
    let run = |tap: &mut (u16, u32), step: u32, ticking: bool, b: u16| {
        let mut p = poll(b);
        if tap_window(tap, step, ticking) {
            latch_taps(&mut tap.0, &mut p);
        }
        p.buttons
    };
    // (a) loading: step frozen at 500, START tapped then released, then the level ticks
    let mut tap = (0u16, 0u32);
    run(&mut tap, 500, false, pad::START);
    run(&mut tap, 500, false, 0);
    assert_eq!(run(&mut tap, 500, true, 0), 0, "a START tapped while loading is not queued into the level");
    // ... but a START still held when the load ends is seen held (as the IOP sample)
    let mut tap = (0u16, 0u32);
    run(&mut tap, 500, false, pad::START);
    assert_eq!(run(&mut tap, 500, true, pad::START), pad::START);
    // (b) pause menu open, debounce ticks 10..17: a 16 ms tap at tick 12 reaches tick 12 only
    let mut tap = (0u16, 0u32);
    run(&mut tap, 12, true, pad::START);
    assert_eq!(run(&mut tap, 12, true, 0), pad::START);
    for s in 13..20 {
        assert_eq!(run(&mut tap, s, true, 0), 0, "tick {s}: the tap is not carried");
        assert_eq!(run(&mut tap, s, true, 0), 0);
    }
    // (c) focus lost while CROSS tapped: the release_all poll still carries the tap to this tick, once
    let mut tap = (0u16, 0u32);
    run(&mut tap, 40, true, pad::CROSS);
    assert_eq!(run(&mut tap, 40, true, 0), pad::CROSS);
    assert_eq!(run(&mut tap, 41, true, 0), 0, "no stuck key after focus loss");
    // pause_on_focus_loss on: START tapped on the focus-loss frame + the FocusPause press = one press
    for enabled in [true, false] {
        let mut f = FocusPause::default();
        let mut tap = (0u16, 0u32);
        let mut seen = Vec::new();
        for (s, b, lost) in [(60, pad::START, false), (60, 0, true), (61, 0, false), (61, 0, false), (62, 0, false), (63, 0, false), (64, 0, false)] {
            let mut p = poll(b);
            // the pause menu opens on the START edge of tick 60: no longer gameplay
            if f.update(enabled, lost, s == 60, s) {
                p.buttons |= pad::START;
            }
            if tap_window(&mut tap, s, true) {
                latch_taps(&mut tap.0, &mut p);
            }
            seen.push((s, p.buttons & pad::START != 0));
        }
        let ticks: Vec<bool> = [60, 61, 62, 63, 64].iter().map(|t| seen.iter().rev().find(|x| x.0 == *t).unwrap().1).collect();
        let edges = ticks.windows(2).filter(|w| !w[0] && w[1]).count() + ticks[0] as usize;
        assert_eq!(edges, 1, "enabled {enabled}: one START press {ticks:?}");
    }
}

/// P41 R7: the tap latch around movies / front-end screens: movies step once per poll, so a tap on
/// the last poll of a movie (or of the title / a save prompt tick) is never carried into the next
/// movie or screen; a tap between two 30 Hz ticks of a prompt reaches exactly the next tick.
#[test]
fn p41_latch_movie() {
    let poll = |b: u16| HostInput { buttons: b, ..Default::default() }.to_pad();
    let run = |tap: &mut (u16, u32), step: u32, ticking: bool, b: u16| {
        let mut p = poll(b);
        if tap_window(tap, step, ticking) {
            latch_taps(&mut tap.0, &mut p);
        }
        p.buttons
    };
    // movie 1 ends at step 300 (START tapped on its last poll), movie 2 starts at 301
    let mut tap = (0u16, 0u32);
    assert_eq!(run(&mut tap, 300, true, pad::START), pad::START);
    for s in 301..400 {
        assert_eq!(run(&mut tap, s, true, 0), 0, "step {s}: the tap is not carried into the next movie");
    }
    // movie -> level load (not ticking) -> title screen ticks: nothing queued
    let mut tap = (0u16, 0u32);
    run(&mut tap, 500, true, pad::CROSS);
    run(&mut tap, 501, false, 0);
    assert_eq!(run(&mut tap, 501, true, 0), 0);
    // save prompt (30 Hz ticks, 2 polls each): a one-poll CROSS tap on the off poll reaches the next tick once
    let mut tap = (0u16, 0u32);
    run(&mut tap, 600, true, 0);
    run(&mut tap, 600, true, pad::CROSS);
    assert_eq!(run(&mut tap, 600, true, 0), pad::CROSS, "the tick of step 600 sees the tap");
    assert_eq!(run(&mut tap, 601, true, 0), 0, "and the next prompt (step 601) does not");
    assert_eq!(run(&mut tap, 601, true, 0), 0);
}

/// P41 R7: gamepad press-to-bind in PC SETTINGS ignores a pad on its first poll (hot-plug: a held
/// button reads as a new press) and only takes a fresh press of an already connected pad.
#[test]
fn p41_bind_press() {
    let mut known: Vec<u32> = vec![1];
    assert_eq!(bind_press(&mut known, &[(1, None), (2, Some('a'))]), None, "pad 2 just plugged in");
    assert_eq!(bind_press(&mut known, &[(1, None), (2, None::<char>)]), None);
    assert_eq!(bind_press(&mut known, &[(1, None), (2, Some('b'))]), Some('b'));
    assert_eq!(bind_press(&mut known, &[(1, Some('c'))]), Some('c'), "pad 2 unplugged, pad 1 still binds");
    // unplug + replug pad 1 holding a button
    assert_eq!(bind_press::<u32, char>(&mut known, &[]), None);
    assert_eq!(bind_press(&mut known, &[(1, Some('d'))]), None);
    assert_eq!(known, vec![1]);
}

/// P41 R8: two pads at once: never merged; a press on pad 2 doesn't cut pad 1's held button; same-frame
/// presses keep the current pad; once pad 1 is released, a press on pad 2 hands over.
#[test]
fn p41_two_pads() {
    // (id, pressed this frame, busy)
    assert_eq!(pick_pad_held(Some(1u8), true, &[(1, false, true), (2, true, true)]), Some(1), "pad 1 holds R1: pad 2's press waits");
    assert_eq!(pick_pad_held(Some(1u8), false, &[(1, false, false), (2, true, true)]), Some(2), "pad 1 released: hand-over");
    assert_eq!(pick_pad_held(Some(2u8), false, &[(1, true, true), (2, true, true)]), Some(2), "same-frame presses: the current pad keeps it");
    assert_eq!(pick_pad(Some(2u8), &[(1, true, true), (2, true, true)]), Some(2));
    assert_eq!(pick_pad_held(Some(1u8), true, &[(2, true, true)]), Some(2), "a held pad that was unplugged can't keep it");
    assert_eq!(pick_pad_held(None, true, &[(2u8, false, true)]), Some(2));
}

/// P41 R9: the R8 press-to-bind fix (PC SETTINGS): the key just bound to cross, still held when the
/// capture ends, must not be a fresh cross press (it re-opened the capture and bound the next key).
#[test]
fn p41_page_prev() {
    const X: u16 = 0x4000; // cross
    let edge = |prev: u16, b: u16| b & !prev;
    // capturing: nothing is a fresh press
    let prev = page_prev(X, true, false);
    assert_eq!(edge(prev, X), 0);
    // the step the capture ends, X still held
    let prev = page_prev(X, false, true);
    assert_eq!(edge(prev, X), 0, "held bound key is not fresh after the capture ends");
    // still held on later steps: no edge
    let prev = page_prev(X, false, false);
    assert_eq!(edge(prev, X), 0);
    // released, then pressed again: fresh
    let prev = page_prev(0, false, false);
    assert_eq!(edge(prev, X), X);
}

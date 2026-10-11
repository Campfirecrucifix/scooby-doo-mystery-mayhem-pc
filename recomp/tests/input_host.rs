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

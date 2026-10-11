//! WP-H tests: input edge/mash/analog, camera look-at, spline evaluation.
use scooby_recomp::engine::input::*;
use scooby_recomp::engine::Mem;

fn mgr_with_mash() -> (EFPlayerDeviceMgr, Mem) {
    let mut m = EFPlayerDeviceMgr::new();
    let mut mem = Mem::new();
    // TomeDoomFUNC_Create_Input subset: config 3, action 0x39 = cross on pad 0x10, context 0x12 on push (8)
    m.APIinput_InputConfigAdd(&mut mem, 3);
    m.APIinput_InputActionAdd(&mut mem, 3, 0x39, 0);
    m.APIinput_InputDeviceAdd(&mut mem, 3, 0x39, 0x26, 0x10, 0, 2, 0, 0);
    m.APIinput_InputContextAdd(&mut mem, 0x1234, 0x10, 3, 0x39, 0x12, 8);
    m.APIinput_InputContextSetState(&mut mem, 3, 0x39, 0x12, 0x10);
    // sticks: action 6, left stick X/Y analog + dpad left digital on X
    m.APIinput_InputActionAdd(&mut mem, 3, 6, 0);
    m.APIinput_InputDeviceAdd(&mut mem, 3, 6, 3, 0x10, 0, 0, 0, 0x8000);
    m.APIinput_InputDeviceAdd(&mut mem, 3, 6, 4, 0x10, 0, 1, 0, 0x8001);
    m.APIinput_InputDeviceAdd(&mut mem, 3, 6, 0xb, 0x10, 1, 0, -100, 0xe);
    // keyboard device (id 1) does not exist on PS2: ignored
    m.APIinput_InputDeviceAdd(&mut mem, 3, 6, 0xb, 1, 1, 0, -100, 0xcb);
    m.APIinput_InputConfigSetState(&mut mem, 3, 0x10);
    (m, mem)
}

fn frame(m: &mut EFPlayerDeviceMgr, pad: PadState) {
    m.SetPadState(0, pad);
    m.StartFrame();
    m.Update(1.0 / 60.0);
}

fn press(b: u16) -> PadState {
    PadState { buttons: b, ..Default::default() }
}

#[test]
fn skip_frames_and_edges() {
    let (mut m, mut mem) = mgr_with_mash();
    // first device-mgr update only activates; then 5 EFInputMgr updates are skipped
    for _ in 0..6 {
        frame(&mut m, press(pad::CROSS));
        assert_eq!(m.APIinput_InputActionIsPush(&mut mem, 3, 0x39), 0);
    }
    frame(&mut m, press(pad::CROSS));
    assert_eq!(m.APIinput_InputActionIsPush(&mut mem, 3, 0x39), 1);
    assert_eq!(m.APIinput_InputActionTogglePush(&mut mem, 3, 0x39), 1);
    assert_eq!(m.notifies.iter().filter(|n| n.callback == 0x10).count(), 1);
    frame(&mut m, press(pad::CROSS));
    assert_eq!(m.APIinput_InputActionIsPush(&mut mem, 3, 0x39), 1);
    assert_eq!(m.APIinput_InputActionTogglePush(&mut mem, 3, 0x39), 0);
    frame(&mut m, press(0));
    assert_eq!(m.APIinput_InputActionIsRelease(&mut mem, 3, 0x39), 1);
    assert_eq!(m.APIinput_InputActionToggleRelease(&mut mem, 3, 0x39), 1);
}

#[test]
fn button_mash_count() {
    let (mut m, _mem) = mgr_with_mash();
    for _ in 0..6 {
        frame(&mut m, press(0));
    }
    let mut n = 0;
    for i in 0..20 {
        frame(&mut m, press(if i % 2 == 0 { pad::CROSS } else { 0 }));
        n += m.notifies.iter().filter(|x| x.callback == 0x10).count();
    }
    assert_eq!(n, 10);
    // config state mask: context state 0x10 & config state 0 -> no callback
    let mut mem = Mem::new();
    m.APIinput_InputConfigSetState(&mut mem, 3, 0x40);
    frame(&mut m, press(pad::CROSS));
    assert!(m.notifies.is_empty());
}

#[test]
fn analog_deadzone_and_dpad() {
    let (mut m, mut mem) = mgr_with_mash();
    for _ in 0..6 {
        frame(&mut m, press(0));
    }
    // inside deadzone: idle
    frame(&mut m, PadState { lx: 128 + 30, ..Default::default() });
    assert_eq!(m.APIinput_InputActionGetAnalogX(&mut mem, 3, 6), 0.0);
    assert_eq!(m.APIinput_InputActionGetDigitalInfo(&mut mem, 3, 6), 0);
    // lx=255: (127-40)/70 > 1 -> 1.0; ly=0 (up): -(-1) = +1
    frame(&mut m, PadState { lx: 255, ly: 0, ..Default::default() });
    assert_eq!(m.APIinput_InputActionGetAnalogX(&mut mem, 3, 6), 1.0);
    assert_eq!(m.APIinput_InputActionGetAnalogY(&mut mem, 3, 6), 1.0);
    assert_eq!(m.APIinput_InputActionGetDigitalInfo(&mut mem, 3, 6), 1);
    // lx = 128+75: (75-40)/70 = 0.5
    frame(&mut m, PadState { lx: 128 + 75, ..Default::default() });
    assert_eq!(m.APIinput_InputActionGetAnalogX(&mut mem, 3, 6), 0.5);
    // dpad left on slot 1 -> -1 (sens -100 * 5 clamped), bigger magnitude than slot 0
    frame(&mut m, press(pad::LEFT));
    assert_eq!(m.APIinput_InputActionGetAnalogX(&mut mem, 3, 6), -1.0);
    assert_eq!(EFPlayerDevice::StickValue(128 + 41), 0.014);
}

#[test]
fn activation_is_deferred_and_primed() {
    let (mut m, mut mem) = mgr_with_mash();
    for _ in 0..6 {
        frame(&mut m, press(0));
    }
    m.APIinput_InputConfigActivate(&mut mem, 3, 0);
    assert_eq!(m.APIinput_ConfigIsActive(&mut mem, 3), 0); // pending (flags = 2)
    frame(&mut m, press(pad::CROSS));
    assert_eq!(m.APIinput_ConfigIsActive(&mut mem, 3), 0);
    m.APIinput_InputConfigActivate(&mut mem, 3, 1);
    frame(&mut m, press(pad::CROSS));
    // primed by UpdateActiveFlag: held button gives no push edge
    assert_eq!(m.APIinput_InputActionTogglePush(&mut mem, 3, 0x39), 0);
    assert_eq!(m.APIinput_InputActionIsPush(&mut mem, 3, 0x39), 1);
}

#[test]
fn vibration_envelope() {
    let mut d = EFPlayerDevice::new(0x10, 0, 0);
    d.vibration_enabled = true;
    d.SetVibration(0, 1.0, 1.0, 1.0, 1.0);
    d.Update(0.5);
    assert!((d.outputs[0] - 0.5).abs() < 1e-6);
    d.Update(1.0);
    assert_eq!(d.outputs[0], 1.0);
    d.Update(1.0);
    assert!((d.outputs[0] - 0.5).abs() < 1e-6);
    d.Update(1.0);
    assert!(!d.vibration[0].active);
}

#[test]
fn reset_blocks_held_buttons() {
    let mut d = EFPlayerDevice::new(0x10, 0, 0);
    d.pad = press(pad::CROSS);
    d.Reset();
    d.Update(0.0);
    assert_eq!(d.digital[button::CROSS], 0);
    d.Update(0.0);
    assert_eq!(d.digital[button::CROSS], 0);
    d.pad = press(0);
    d.Update(0.0);
    d.pad = press(pad::CROSS);
    d.Update(0.0);
    assert_eq!(d.digital[button::CROSS], 1);
}

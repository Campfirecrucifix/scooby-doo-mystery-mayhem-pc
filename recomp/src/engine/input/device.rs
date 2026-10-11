//! EFPlayerDevice / EFPlayerDevice_BC / EFPlayerDevice_SP, EFPlayerDeviceMgr(_BC/_SP), EFKeyboard
//! (EXE.1 Classes/EF/EFPlayerDevice*.cpp, EFPlayerDeviceMgr*.cpp, EFKeyboard.cpp).
//!
//! Replaced (Sony SDK): scePad* (libpad DMA buffer, mode switching, actuator alignment, pressure
//! mode negotiation) is replaced by [`PadState`], a snapshot the coordinator fills each frame from
//! gilrs/bevy_input in the libpad layout. Everything that the input layer above observes is kept:
//! the 16 digital slots in EFPlayerDevice_SP order, the 18 analog slots (sticks with the 40/70
//! deadzone and 0.001 quantisation, the 12 pressure values), the "button held at reset stays
//! blocked until released" filter, the vibration envelopes and the two motor outputs.
#![allow(non_snake_case)]

/// Device ids (EFDeviceID_e): pads are `0x10 + port*4 + slot` (InstallSimpleController). Id 1 is the
/// PC keyboard of the shared code base; no such device exists on PS2 so it is never found.
pub const DEVICE_PAD0: u8 = 0x10;
pub const DEVICE_PAD1: u8 = 0x14;
pub const DEVICE_KEYBOARD: u8 = 1;

/// libpad digital button bits of [`PadState::buttons`] (1 = pressed; libpad itself is active low:
/// bits 0..7 = pad byte 2 (rdata[2]), bits 8..15 = byte 3).
pub mod pad {
    pub const SELECT: u16 = 0x0001;
    pub const L3: u16 = 0x0002;
    pub const R3: u16 = 0x0004;
    pub const START: u16 = 0x0008;
    pub const UP: u16 = 0x0010;
    pub const RIGHT: u16 = 0x0020;
    pub const DOWN: u16 = 0x0040;
    pub const LEFT: u16 = 0x0080;
    pub const L2: u16 = 0x0100;
    pub const R2: u16 = 0x0200;
    pub const L1: u16 = 0x0400;
    pub const R1: u16 = 0x0800;
    pub const TRIANGLE: u16 = 0x1000;
    pub const CIRCLE: u16 = 0x2000;
    pub const CROSS: u16 = 0x4000;
    pub const SQUARE: u16 = 0x8000;
}

/// Digital slot indices of an EFPlayerDevice (the button index of InputDeviceAdd's last argument).
pub mod button {
    pub const CROSS: usize = 0;
    pub const CIRCLE: usize = 1;
    pub const SQUARE: usize = 2;
    pub const TRIANGLE: usize = 3;
    pub const L1: usize = 4;
    pub const R1: usize = 5;
    pub const L2: usize = 6;
    pub const R2: usize = 7;
    pub const L3: usize = 8;
    pub const R3: usize = 9;
    pub const SELECT: usize = 10;
    pub const START: usize = 11;
    pub const UP: usize = 12;
    pub const DOWN: usize = 13;
    pub const LEFT: usize = 14;
    pub const RIGHT: usize = 15;
}

/// Replacement for the libpad receive buffer (EFPlayerDevice_SP this+0x34, 0x20 bytes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PadState {
    /// false = scePadRead failed / no controller: the device raises the error notify (0xe).
    pub connected: bool,
    /// libpad mode id (scePadInfoMode): 7 = DualShock analog, 4 = digital. Only 7 reads sticks.
    pub mode: u16,
    /// pressed buttons, [`pad`] bits, 1 = pressed.
    pub buttons: u16,
    /// rdata[4..8]: right stick x, y, left stick x, y; 0..255, 128 = centre, y down = 255.
    pub rx: u8,
    pub ry: u8,
    pub lx: u8,
    pub ly: u8,
    /// rdata[8..20] pressure, libpad order: right, left, up, down, triangle, circle, cross, square,
    /// L1, R1, L2, R2 (0..255).
    pub pressure: [u8; 12],
    /// pressure mode entered (EFPlayerDevice_SP this+0x6d); pressure slots are 0 otherwise.
    pub pressure_mode: bool,
}

impl Default for PadState {
    fn default() -> Self {
        PadState { connected: true, mode: 7, buttons: 0, rx: 128, ry: 128, lx: 128, ly: 128, pressure: [0; 12], pressure_mode: false }
    }
}

/// One vibration envelope (EFPlayerDevice this+0x78, 0x28 bytes each).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VibrationEvent {
    /// +0x00
    pub active: bool,
    /// +0x08 (double) time since SetVibration
    pub time: f64,
    /// +0x10 value at the start of the attack
    pub start: f32,
    /// +0x14 target value
    pub target: f32,
    /// +0x18 current value
    pub current: f32,
    /// +0x1c attack, +0x20 sustain, +0x24 release (seconds)
    pub attack: f32,
    pub sustain: f32,
    pub release: f32,
}

/// EFPlayerDevice (0xb0 bytes; _BC + _SP + EFPlayerDevice fields flattened).
#[derive(Clone, Debug)]
pub struct EFPlayerDevice {
    /// +0x04 EFDeviceID_e
    pub device_id: u8,
    /// +0x0a vibration enabled
    pub vibration_enabled: bool,
    /// +0x0b active
    pub active: bool,
    /// +0x0c acquire count
    pub acquire_count: i16,
    /// +0x10 current digital states (16)
    pub digital: Vec<u8>,
    /// +0x14 previous digital buffer (swapped with +0x10 each frame)
    pub digital_prev: Vec<u8>,
    /// +0x18 "held since reset" block flags
    pub blocked: Vec<u8>,
    /// +0x1c analog values (18)
    pub analog: Vec<f32>,
    /// +0x28 output (motor) values (2)
    pub outputs: Vec<f32>,
    /// +0x2c error-notify object (uid of the EFInstance) and its callback (InputErrorRegisterNotify)
    pub error_notify: Option<(u32, u32)>,
    /// +0x30 reset pending: next update blocks every held button
    pub reset_pending: bool,
    /// +0x5c port, +0x5d slot
    pub port: u8,
    pub slot: u8,
    /// +0x5e actuators aligned (motors usable)
    pub actuators: bool,
    /// +0x5f locked (port closed)
    pub locked: bool,
    /// +0x78 vibration events (one per output)
    pub vibration: Vec<VibrationEvent>,
    /// replacement for scePadRead: filled by the coordinator
    pub pad: PadState,
    /// replacement for scePadSetActDirect: (small motor on, big motor 0..255)
    pub motors: (bool, u8),
}

impl EFPlayerDevice {
    /// EFPlayerDevice(EFDeviceID_e) + EFPlayerDevice_SP::CompilePossibilities (18 analog, 16 digital,
    /// 2 outputs -> SetNbVibrationEvents).
    pub fn new(device_id: u8, port: u8, slot: u8) -> Self {
        EFPlayerDevice {
            device_id,
            vibration_enabled: false,
            active: true,
            acquire_count: 0,
            digital: vec![0; 16],
            digital_prev: vec![0; 16],
            blocked: vec![0; 16],
            analog: vec![0.0; 18],
            outputs: vec![0.0; 2],
            error_notify: None,
            reset_pending: false,
            port,
            slot,
            actuators: true,
            locked: false,
            vibration: vec![VibrationEvent::default(); 2],
            pad: PadState::default(),
            motors: (false, 0),
        }
    }

    pub fn nb_digital(&self) -> u16 {
        self.digital.len() as u16
    }
    pub fn nb_analog(&self) -> u16 {
        self.analog.len() as u16
    }
    pub fn nb_output(&self) -> u16 {
        self.outputs.len() as u16
    }

    /// EFPlayerDevice::Reset (0x0022ad10)
    pub fn Reset(&mut self) {
        self.error_notify = None;
        self.vibration_enabled = true;
        for i in 0..self.nb_output() as u32 {
            self.SetVibration(i, 0.0, 0.0, 0.0, 0.0);
        }
        self.vibration_enabled = false;
        self.reset_pending = true;
    }

    /// EFPlayerDevice::SetVibration(index, value, attack, sustain, release) (0x0022ad90)
    pub fn SetVibration(&mut self, i: u32, value: f32, attack: f32, sustain: f32, release: f32) {
        if self.vibration_enabled && (i as usize) < self.vibration.len() {
            let e = &mut self.vibration[i as usize];
            e.time = 0.0;
            e.start = if e.active { e.current } else { 0.0 };
            e.target = value;
            e.attack = attack;
            e.sustain = sustain;
            e.release = release;
            e.active = true;
        }
    }

    /// EFPlayerDevice::Update(float) (0x0022ae40). `dt` replaces the device's EFTimer
    /// (GetElapsed + Reset at each update).
    pub fn Update(&mut self, dt: f32) -> bool {
        // quirk: the slope/intercept are locals kept across outputs; an event that ends this frame
        // writes `b + a*t` with the previous output's coefficients (0,0 for output 0).
        let mut a = 0.0f32;
        let mut b = 0.0f32;
        if !self.outputs.is_empty() {
            for i in 0..self.outputs.len() {
                let e = &mut self.vibration[i];
                e.time += dt as f64;
                let t = e.time;
                let atk = e.attack;
                if t <= atk as f64 {
                    b = e.start;
                    a = (e.target - b) / atk;
                } else if t > atk as f64 && t <= (atk + e.sustain) as f64 {
                    a = 0.0;
                    b = e.target;
                } else {
                    let end = atk + e.sustain;
                    if t > end as f64 && t <= (e.release + end) as f64 {
                        a = -e.target / e.release;
                        b = e.target - a * end;
                    } else {
                        let keep_t = e.time;
                        *e = VibrationEvent::default();
                        let v = (b as f64 + a as f64 * keep_t) as f32;
                        e.current = v;
                        self.outputs[i] = v;
                        continue;
                    }
                }
                let v = (b as f64 + a as f64 * t) as f32;
                e.current = v;
                self.outputs[i] = v;
            }
        }
        let err = self.DeviceSpecificUpdate();
        if self.reset_pending {
            self.reset_pending = false;
            self.blocked.copy_from_slice(&self.digital);
        }
        for i in 0..self.digital.len() {
            if self.blocked[i] == 1 && self.digital[i] == 1 {
                self.digital[i] = 0;
            } else {
                self.blocked[i] = 0;
            }
        }
        err
    }

    /// Stick conversion of EFPlayerDevice_SP::DeviceSpecificUpdate: |v-128| minus a 40 deadzone,
    /// scaled over 70, clamped to [0,1], truncated to 1/1000, signed.
    pub fn StickValue(raw: u8) -> f32 {
        let d = raw as f64 - 128.0;
        let sign: f32 = if 0.0 <= d as f32 { 1.0 } else { -1.0 };
        let mut f = (d as f32).abs() - 40.0;
        if f < 0.0 {
            f = 0.0;
        } else if f <= 70.0 {
            f /= 70.0;
            if f as f64 > 1.0 {
                f = 1.0;
            }
            if (f as f64) < 0.0 {
                f = 0.0;
            }
        } else {
            f = 1.0;
        }
        sign * (((f * 1000.0) as i32) as f64 * 0.001) as f32
    }

    fn notify_error(&mut self) -> bool {
        self.error_notify.is_some()
    }

    /// EFPlayerDevice_SP::DeviceSpecificUpdate (0x0022b3d0). Returns true when the error notify
    /// (0xe, payload device id) must be raised on `error_notify`.
    pub fn DeviceSpecificUpdate(&mut self) -> bool {
        if self.locked {
            return self.notify_error();
        }
        std::mem::swap(&mut self.digital, &mut self.digital_prev);
        self.digital.iter_mut().for_each(|b| *b = 0);
        self.analog.iter_mut().for_each(|v| *v = 0.0);
        let p = self.pad;
        if !p.connected {
            return self.notify_error();
        }
        let on = |bit: u16| (p.buttons & bit != 0) as u8;
        use pad::*;
        let order = [CROSS, CIRCLE, SQUARE, TRIANGLE, L1, R1, L2, R2];
        for (i, b) in order.iter().enumerate() {
            self.digital[i] = on(*b);
        }
        self.digital[8] = 0;
        self.digital[9] = 0;
        self.digital[10] = on(SELECT);
        self.digital[11] = on(START);
        self.digital[12] = on(UP);
        self.digital[13] = on(DOWN);
        self.digital[14] = on(LEFT);
        self.digital[15] = on(RIGHT);
        if p.mode == 7 {
            self.digital[8] = on(L3);
            self.digital[9] = on(R3);
            self.analog[0] = Self::StickValue(p.lx);
            self.analog[1] = -Self::StickValue(p.ly);
            self.analog[3] = Self::StickValue(p.rx);
            self.analog[4] = -Self::StickValue(p.ry);
            if self.actuators {
                let big = self.outputs[0] * 255.0;
                let big = if big < 2.147_483_6e9 { big as i32 as u8 } else { (big - 2.147_483_6e9) as i32 as u8 };
                self.motors = (0.0 < self.outputs[1], big);
            }
            if p.pressure_mode {
                // pressure bytes -> analog 6..17: cross, circle, square, triangle, L1, R1, L2, R2, up, down, left, right
                let src = [6usize, 5, 7, 4, 8, 9, 10, 11, 2, 3, 1, 0];
                for (k, s) in src.iter().enumerate() {
                    self.analog[6 + k] = p.pressure[*s] as f32 / 255.0;
                }
            }
        }
        false
    }

    /// EFPlayerDevice_SP::LockPlayerDevice
    pub fn LockPlayerDevice(&mut self, lock: bool) {
        if !self.locked && lock {
            self.locked = true;
            self.acquire_count -= 1;
        } else if self.locked && !lock {
            self.locked = false;
            self.acquire_count += 1;
        }
    }

    /// EFPlayerDevice_BC/_SP::ActivatePlayerDevice
    pub fn ActivatePlayerDevice(&mut self, b: bool) {
        self.active = b;
    }

    pub fn digital_at(&self, i: u8) -> bool {
        (i as usize) < self.digital.len() && self.digital[i as usize] != 0
    }
    pub fn analog_at(&self, i: u8) -> f32 {
        if (i as usize) < self.analog.len() { self.analog[i as usize] } else { 0.0 }
    }
}

/// EFKeyboard (this+0x28 of EFPlayerDeviceMgr): only Flush exists on PS2.
#[derive(Clone, Debug, Default)]
pub struct EFKeyboard {
    /// +0x0c, +0x0e
    pub head: u16,
    pub tail: u16,
}
impl EFKeyboard {
    pub fn new() -> Self {
        let mut k = EFKeyboard::default();
        k.Flush();
        k
    }
    pub fn Flush(&mut self) {
        self.head = 0;
        self.tail = 0;
    }
}

/// An error notification the coordinator must deliver (EFInstance::Notify(obj, 0xe, device id, 0)
/// -> registered ExecLogicCallback of `obj`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeviceErrorNotify {
    pub obj_uid: u32,
    pub callback: u32,
    pub device_id: u8,
}

//! EFInputMgr / EFInputConfig / EFInputAction / EFInputContext / EFInputDevice
//! (EXE.1 Classes/EF/EFInput*.cpp, 0x00203c10-0x0020546f). 1:1 port; the singly linked lists
//! (PushFront) are Vecs whose index 0 is the list head, so iteration order is the original's.
#![allow(non_snake_case)]

use super::device::EFPlayerDevice;

/// ActionState {float x; float y; uint flags} (0xc bytes).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ActionState {
    pub x: f32,
    pub y: f32,
    pub flags: u32,
}

/// ActionState flag bits.
pub mod flag {
    /// a digital device fed the slot
    pub const DIGITAL: u32 = 0x1;
    /// analog X / Y fed
    pub const ANALOG_X: u32 = 0x2;
    pub const ANALOG_Y: u32 = 0x4;
    /// push / release edges (TogglePush / ToggleRelease)
    pub const PUSHED: u32 = 0x8;
    pub const RELEASED: u32 = 0x10;
    /// held down / up (IsPush / IsRelease)
    pub const DOWN: u32 = 0x20;
    pub const UP: u32 = 0x40;
    /// analog idle / moved (|x| or |y| > 0.001)
    pub const IDLE: u32 = 0x10000;
    pub const MOVED: u32 = 0x20000;
    /// analog crossed 0.8: +y, -y, -x, +x
    pub const STICK_UP: u32 = 0x100000;
    pub const STICK_DOWN: u32 = 0x200000;
    pub const STICK_LEFT: u32 = 0x400000;
    pub const STICK_RIGHT: u32 = 0x800000;
}

/// Callback request produced by EFInputContext::Process. The original calls
/// EFGameLogicMgr::ExecLogicCallback({obj, callback}, &InputNotify) immediately, InputNotify being
/// the static ActionState at 0x01f72010. The coordinator writes {x, y, flags} into Mem and calls
/// ExecLogicCallback in the same order (the queue keeps the call order).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InputNotify {
    pub obj_uid: u32,
    pub callback: u32,
    pub state: ActionState,
}

/// EFInputContext (0x24 bytes)
#[derive(Clone, Debug, PartialEq)]
pub struct EFInputContext {
    /// +0x00 context id
    pub id: i32,
    /// +0x04 ActionState flags that trigger the callback
    pub trigger: u32,
    /// +0x08 state mask, tested against the owning config's state (+0x0c -> config +0x00)
    pub state: u32,
    /// +0x0c owning config id (the pointer in the original)
    pub config: i32,
    /// +0x10 callback set (ExecLogicCallback)
    pub has_callback: bool,
    /// +0x14 logic callback index
    pub callback: u32,
    /// +0x18 game object uid
    pub obj_uid: u32,
    /// +0x1c object's logic code id (-1 = none)
    pub code_id: i32,
}

impl EFInputContext {
    pub fn EFInputContext() -> Self {
        EFInputContext { id: 0, trigger: 0, state: 0xffff_ffff, config: 0, has_callback: false, callback: 0, obj_uid: 0, code_id: 0 }
    }
    /// EFInputContext::SetCallback: the original fails (-1) for a NULL object or an unknown
    /// callback index (EFLogicCodeBC::GetFunction == 0); object lookup is the coordinator's job,
    /// so here only `obj_uid == 0` fails.
    pub fn SetCallback(&mut self, obj_uid: u32, callback: u32) -> i32 {
        if obj_uid == 0 {
            return -1;
        }
        self.has_callback = true;
        self.callback = callback;
        self.obj_uid = obj_uid;
        self.code_id = 0;
        0
    }
    /// EFInputContext::Process(ActionState*) (0x00204b70)
    pub fn Process(&self, config_state: u32, s: &ActionState, out: &mut Vec<InputNotify>) {
        if s.flags & self.trigger != 0 && self.state & config_state != 0 && self.has_callback && self.code_id != -1 {
            // EFWorld::GetGameObject(obj) != 0 is checked by the coordinator when delivering
            out.push(InputNotify { obj_uid: self.obj_uid, callback: self.callback, state: *s });
        }
    }
}

/// EFInputDevice (0x14 bytes)
#[derive(Clone, Debug, PartialEq)]
pub struct EFInputDevice {
    /// +0x00 button / axis index (low byte of the last InputDeviceAdd arg)
    pub index: u8,
    /// +0x01 slot kind: 0 = analog X, 1 = analog Y, 2 = digital
    pub kind: u8,
    /// +0x02 EFDeviceID_e
    pub device_id: u8,
    /// +0x03 first InputDeviceAdd arg (unused by the engine)
    pub user: u8,
    /// +0x04 slot list index (0..4)
    pub slot: u8,
    /// +0x05 1<<kind, 0x80 = reads an analog value
    pub flags: u8,
    /// +0x08 sensitivity / 100
    pub scale: f32,
}

impl EFInputDevice {
    /// EFInputDevice::Set (0x00204c80). Returns false when the player device does not exist
    /// (AcquirePlayerDevice == 0). `devices` replaces _gpoPlayerDeviceMgr.
    pub fn Set(user: i32, device_id: u8, slot: i32, kind: i32, sens: i32, index: u32, devices: &mut [EFPlayerDevice]) -> Option<Self> {
        let mut d = EFInputDevice {
            index: index as u8,
            slot: slot as u8,
            kind: kind as u8,
            device_id,
            scale: sens as f32 / 100.0,
            user: user as u8,
            flags: 0,
        };
        super::AcquirePlayerDevice(devices, device_id)?;
        if index & 0x8000 != 0 {
            d.flags = 0x80;
        }
        d.flags |= (1u32 << (kind as u32 & 0x1f)) as u8;
        Some(d)
    }

    fn digital_pressed(&self, dev: &EFPlayerDevice) -> bool {
        dev.digital_at(self.index)
    }

    /// EFInputDevice::UpdateAnalogSlot (0x00204d30)
    pub fn UpdateAnalogSlot(&self, dev: &EFPlayerDevice, s: &mut ActionState, axis: usize) {
        let v = if axis == 0 { &mut s.x } else { &mut s.y };
        if self.flags & 0x80 == 0 {
            if self.digital_pressed(dev) {
                *v = (self.scale * 5.0).clamp(-1.0, 1.0);
            } else {
                if 0.0 < *v && 0.0 < self.scale {
                    *v = 0.0;
                }
                if *v < 0.0 && self.scale < 0.0 {
                    *v = 0.0;
                }
            }
        } else {
            let f = dev.analog_at(self.index);
            *v = if 1.0 < f { 1.0 } else if -1.0 <= f { f } else { -1.0 };
        }
    }

    /// EFInputDevice::UpdateDigitalSlot (0x00204ee0)
    pub fn UpdateDigitalSlot(&self, dev: &EFPlayerDevice, s: &mut ActionState) {
        let pressed = if self.flags & 0x80 == 0 { self.digital_pressed(dev) } else { dev.analog_at(self.index) != 0.0 };
        // quirk: AND, so two digital devices on one slot only report a state they agree on
        s.flags &= if pressed { 0x20 } else { 0x40 };
    }

    /// EFInputDevice::Update (0x00204fc0)
    pub fn Update(&self, devices: &[EFPlayerDevice], s: &mut ActionState) {
        let Some(dev) = devices.iter().find(|d| d.device_id == self.device_id) else { return };
        if self.flags & 1 != 0 {
            self.UpdateAnalogSlot(dev, s, 0);
            s.flags |= 2;
        } else if self.flags & 2 != 0 {
            self.UpdateAnalogSlot(dev, s, 1);
            s.flags |= 4;
        } else if self.flags & 4 != 0 {
            self.UpdateDigitalSlot(dev, s);
            s.flags |= 1;
        }
    }
}

/// EFInputAction (0x70 bytes)
#[derive(Clone, Debug, PartialEq)]
pub struct EFInputAction {
    /// +0x00 current state
    pub state: ActionState,
    /// +0x0c previous frame's state
    pub prev: ActionState,
    /// +0x18 action id
    pub id: i32,
    /// +0x54 contexts (head first)
    pub contexts: Vec<EFInputContext>,
    /// +0x58 five device lists (head first)
    pub devices: [Vec<EFInputDevice>; 5],
}

impl EFInputAction {
    pub fn EFInputAction(id: i32) -> Self {
        let mut a = EFInputAction { state: ActionState::default(), prev: ActionState::default(), id, contexts: Vec::new(), devices: Default::default() };
        a.Reset();
        a
    }
    pub fn Reset(&mut self) {
        self.state = ActionState::default();
        self.prev = ActionState::default();
    }

    /// EFInputAction::InputContextAdd (0x00203ce0)
    pub fn InputContextAdd(&mut self, config: i32, id: i32, trigger: u32, obj_uid: u32, callback: u32) -> bool {
        let mut c = EFInputContext::EFInputContext();
        if c.SetCallback(obj_uid, callback) != 0 {
            return false;
        }
        c.id = id;
        c.trigger = trigger;
        c.config = config;
        self.contexts.insert(0, c);
        true
    }
    pub fn InputContextRemoveAll(&mut self) -> bool {
        let had = !self.contexts.is_empty();
        self.contexts.clear();
        had
    }
    /// EFInputAction::InputContextRemove. quirk: the original unlinks the match and then frees the
    /// node that FOLLOWS it (use-after-free); the port removes the matching context.
    pub fn InputContextRemove(&mut self, id: i32) -> bool {
        match self.contexts.iter().position(|c| c.id == id) {
            Some(i) => {
                self.contexts.remove(i);
                true
            }
            None => false,
        }
    }
    pub fn InputContextGet(&mut self, id: i32) -> Option<&mut EFInputContext> {
        self.contexts.iter_mut().find(|c| c.id == id)
    }
    /// EFInputAction::InputDeviceAdd(user, deviceId, slot, kind, sens, index) (0x00203f10)
    pub fn InputDeviceAdd(&mut self, user: i32, device_id: i32, slot: i32, kind: i32, sens: i32, index: i32, devices: &mut [EFPlayerDevice]) -> bool {
        match EFInputDevice::Set(user, (device_id & 0xff) as u8, slot, kind, sens, index as u32, devices) {
            Some(d) => {
                // the original indexes the 5 lists with `slot` unchecked
                self.devices[slot as usize].insert(0, d);
                true
            }
            None => false,
        }
    }
    /// EFInputAction::InputDeviceRemove: empty in EXE.1.
    pub fn InputDeviceRemove(&mut self, _id: i32) {}
    pub fn InputDeviceRemoveAll(&mut self) {
        for l in self.devices.iter_mut() {
            l.clear();
        }
    }

    /// EFInputAction::AddAnalogToogle (0x002040b0)
    pub fn AddAnalogToogle(&mut self, axis: usize, pos: u32, neg: u32) {
        let (v, pv, other) = if axis == 0 { (self.state.x, self.prev.x, self.state.y) } else { (self.state.y, self.prev.y, self.state.x) };
        if 0.8 < v && pv <= 0.8 && other.abs() < v {
            self.state.flags = self.state.flags & 0xff0f_ffff | pos;
        }
        if v < -0.8 && -0.8 <= pv && v < -other.abs() {
            self.state.flags = self.state.flags & 0xff0f_ffff | neg;
        }
    }

    /// EFInputAction::ProcessContext (0x00204210)
    pub fn ProcessContext(&mut self) {
        let f = self.state.flags;
        let p = self.prev.flags;
        if f & !p & 0x20 != 0 {
            self.state.flags |= 8;
        }
        if !f & p & 0x20 != 0 {
            self.state.flags |= 0x10;
        }
        const EPS: f64 = 0.0010000000474974513; // 0x3f50624de0000000
        if (self.state.flags & 2 != 0 && (self.state.x as f64).abs() > EPS) || (self.state.flags & 4 != 0 && (self.state.y as f64).abs() > EPS) {
            self.state.flags |= 0x20000;
        }
        if self.state.flags & 0x20000 == 0 {
            self.state.flags |= 0x10000;
        }
        if self.state.flags & 2 != 0 {
            self.AddAnalogToogle(0, 0x800000, 0x400000);
        }
        if self.state.flags & 4 != 0 {
            self.AddAnalogToogle(1, 0x100000, 0x200000);
        }
    }

    /// EFInputAction::ProcessDevice (0x00204380); `slots` = the static opActionState[5].
    pub fn ProcessDevice(&mut self, slots: &mut [ActionState; 5]) {
        self.state.x = slots[0].x;
        self.state.y = slots[0].y;
        for s in slots.iter_mut() {
            if s.flags & 6 != 0 {
                // both branches of the original are identical
                if s.x.abs() > self.state.x.abs() {
                    self.state.x = s.x;
                }
                if s.y.abs() > self.state.y.abs() {
                    self.state.y = s.y;
                }
            }
            if s.flags & 1 == 0 {
                s.flags &= 0xffff_ff9f;
            }
            self.state.flags |= s.flags;
        }
    }

    /// EFInputAction::UpdateActionState (0x00204570)
    pub fn UpdateActionState(&mut self, devices: &[EFPlayerDevice]) {
        self.state = ActionState::default();
        let mut slots = [ActionState::default(); 5];
        for (i, s) in slots.iter_mut().enumerate() {
            *s = ActionState { x: self.state.x, y: self.state.y, flags: 0x60 };
            if self.devices[i].is_empty() {
                *s = ActionState::default();
            } else {
                for d in &self.devices[i] {
                    d.Update(devices, s);
                }
            }
        }
        self.ProcessDevice(&mut slots);
    }

    /// EFInputAction::Update (0x00204670). The EFDemoLoop record/playback branches
    /// (_gpoEngineController+0x70 bits 1/2) are not ported: demo mode is off.
    pub fn Update(&mut self, devices: &[EFPlayerDevice], config_state: u32, out: &mut Vec<InputNotify>) {
        self.prev = self.state;
        self.UpdateActionState(devices);
        self.ProcessContext();
        for c in &self.contexts {
            c.Process(config_state, &self.state, out);
        }
    }
}

/// EFInputConfig (0x14 bytes)
#[derive(Clone, Debug, PartialEq)]
pub struct EFInputConfig {
    /// +0x00 state bitmask (InputConfigSetState), tested by the contexts' +0x08
    pub state: u32,
    /// +0x04 bit0 active, bit1 change pending, bit2 pending value
    pub active: u32,
    /// +0x08 config id
    pub id: i32,
    /// +0x10 actions (head first)
    pub actions: Vec<EFInputAction>,
}

impl EFInputConfig {
    pub fn EFInputConfig(id: i32) -> Self {
        EFInputConfig { state: 0xffff_ffff, active: 1, id, actions: Vec::new() }
    }
    pub fn Update(&mut self, devices: &[EFPlayerDevice], out: &mut Vec<InputNotify>) {
        let st = self.state;
        for a in self.actions.iter_mut() {
            a.Update(devices, st, out);
        }
    }
    /// EFInputConfig::SetActive (0x00204810): applied at the next StartFrame.
    pub fn SetActive(&mut self, b: bool) {
        if (self.active & 1 != 0) != b {
            self.active = 2;
            if b {
                self.active |= 4;
            }
        }
    }
    pub fn InputActionAdd(&mut self, id: i32) -> &mut EFInputAction {
        self.actions.insert(0, EFInputAction::EFInputAction(id));
        &mut self.actions[0]
    }
    /// quirk: same use-after-free as InputContextRemove in the original; the port removes the match.
    pub fn InputActionRemove(&mut self, id: i32) -> bool {
        match self.actions.iter().position(|a| a.id == id) {
            Some(i) => {
                self.actions.remove(i);
                true
            }
            None => false,
        }
    }
    pub fn InputActionRemoveAll(&mut self) -> bool {
        let had = !self.actions.is_empty();
        self.actions.clear();
        had
    }
    /// EFInputConfig::UpdateActiveFlag (0x002049e0): primes the actions' state on activation so the
    /// first update has no false push edge.
    pub fn UpdateActiveFlag(&mut self, devices: &[EFPlayerDevice]) {
        if self.active & 2 != 0 {
            self.active = (self.active & 4 != 0) as u32;
            if self.active & 1 != 0 {
                for a in self.actions.iter_mut() {
                    a.UpdateActionState(devices);
                }
            }
        }
    }
    pub fn InputActionGet(&mut self, id: i32) -> Option<&mut EFInputAction> {
        self.actions.iter_mut().find(|a| a.id == id)
    }
}

/// EFInputMgr (8 bytes)
#[derive(Clone, Debug, PartialEq)]
pub struct EFInputMgr {
    /// +0x00 configs (head first)
    pub configs: Vec<EFInputConfig>,
    /// +0x04 enabled
    pub enabled: bool,
    /// static _SkipFirstFramePATCH_1454 (moved into the manager: no global state)
    pub skip_first_frame: i32,
}

impl Default for EFInputMgr {
    fn default() -> Self {
        Self::EFInputMgr()
    }
}

impl EFInputMgr {
    pub fn EFInputMgr() -> Self {
        EFInputMgr { configs: Vec::new(), enabled: true, skip_first_frame: 0 }
    }
    pub fn StartFrame(&mut self, devices: &[EFPlayerDevice]) {
        for c in self.configs.iter_mut() {
            c.UpdateActiveFlag(devices);
        }
    }
    /// EFInputMgr::Update (0x00205170): the first 5 updates are skipped.
    pub fn Update(&mut self, devices: &[EFPlayerDevice], out: &mut Vec<InputNotify>) {
        // debug (P20 R137): P17_INLOG=<action id hex> dumps every active config holding that action, each frame
        if let Some(id) = std::env::var("P17_INLOG").ok().and_then(|v| i32::from_str_radix(&v, 16).ok()) {
            for c in self.configs.iter() {
                for a in c.actions.iter().filter(|a| a.id == id) {
                    if c.active & 1 != 0 {
                        eprintln!("INLOG cfg {:#x} active {:#x} state {:#x} act {:#x} flags {:#x} devs {:?} ctx {:?}", c.id, c.active, c.state, a.id, a.state.flags, a.devices, a.contexts);
                    }
                }
            }
        }
        if self.enabled {
            if self.skip_first_frame < 5 {
                self.skip_first_frame += 1;
            } else {
                for c in self.configs.iter_mut() {
                    if c.active & 1 != 0 {
                        c.Update(devices, out);
                    }
                }
            }
        }
    }
    pub fn InputConfigAdd(&mut self, id: i32) -> &mut EFInputConfig {
        self.configs.insert(0, EFInputConfig::EFInputConfig(id));
        &mut self.configs[0]
    }
    pub fn InputConfigRemove(&mut self, id: i32) {
        if let Some(i) = self.configs.iter().position(|c| c.id == id) {
            self.configs.remove(i);
        }
    }
    pub fn InputConfigRemoveAll(&mut self) {
        self.configs.clear();
    }
    pub fn InputConfigGet(&mut self, id: i32) -> Option<&mut EFInputConfig> {
        self.configs.iter_mut().find(|c| c.id == id)
    }
    pub fn InputActionGet(&mut self, cfg: i32, act: i32) -> Option<&mut EFInputAction> {
        self.InputConfigGet(cfg)?.InputActionGet(act)
    }
    pub fn InputContextGet(&mut self, cfg: i32, act: i32, ctx: i32) -> Option<&mut EFInputContext> {
        self.InputActionGet(cfg, act)?.InputContextGet(ctx)
    }
}

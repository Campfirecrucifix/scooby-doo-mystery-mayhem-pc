//! WP-H input: EFPlayerDeviceMgr (owns the EFPlayerDevices and the EFInputMgr, as in EXE.1 where
//! EFPlayerDeviceMgr::Initialize creates the EFInputMgr and Update drives it) + the APIinput /
//! APIsimpleinput host functions (EFApi_Input.cpp, EFApi_SimpleInput.cpp) as plain methods with
//! the api_gen.rs signatures.
//!
//! Callbacks: EFInputContext::Process and the device error notify call into the logic runtime in
//! the original; here they are queued in `notifies` / `error_notifies` in call order and the
//! coordinator delivers them (write the ActionState to the InputNotify block in Mem, then
//! EFGameLogicMgr::ExecLogicCallback) right after `Update`.
#![allow(non_snake_case)]

pub mod config;
pub mod device;
pub mod host;

pub use config::*;
pub use device::*;

use crate::engine::Mem;

/// EFPlayerDeviceMgr::AcquirePlayerDevice(EFDeviceID_e) (GetDevice + acquire count++).
pub fn AcquirePlayerDevice(devices: &mut [EFPlayerDevice], id: u8) -> Option<&mut EFPlayerDevice> {
    let d = devices.iter_mut().find(|d| d.device_id == id)?;
    d.acquire_count += 1;
    // count == 1 -> vtbl 0x1c AcquirePlayerDevice(bool): empty on PS2
    Some(d)
}

/// EFPlayerDeviceMgr (+ _BC + _SP).
#[derive(Clone, Debug)]
pub struct EFPlayerDeviceMgr {
    /// +0x04 EFInputMgr
    pub input: EFInputMgr,
    /// +0x08 DTSingleLList<EFPlayerDevice*> (head first)
    pub devices: Vec<EFPlayerDevice>,
    /// +0x24 active port (InputSetActivePort)
    pub active_port: i32,
    /// +0x28
    pub keyboard: EFKeyboard,
    /// static _SkipFirstFrame_1048 of EFPlayerDeviceMgr_BC::Update
    pub skip_first_frame: bool,
    /// EFInputContext::Process callbacks of the last Update, in call order
    pub notifies: Vec<InputNotify>,
    /// device error notifies (0xe) of the last Update
    pub error_notifies: Vec<DeviceErrorNotify>,
    /// SimpleInput statics: _gSimpleInputData (current config id, starts 0xd903) and the next
    /// action id (0x0034417c)
    pub simple_config: u32,
    pub simple_action: i32,
}

impl Default for EFPlayerDeviceMgr {
    fn default() -> Self {
        Self::new()
    }
}

impl EFPlayerDeviceMgr {
    /// Constructor + Initialize: InitializePs2Input installs one simple controller per port
    /// (no multitap: MultiTapPort1/2 ini keys absent) -> devices 0x10 and 0x14, acquire count 1,
    /// pushed front (list order 0x14, 0x10); EFInputMgr::Create.
    pub fn new() -> Self {
        let mut m = EFPlayerDeviceMgr {
            input: EFInputMgr::EFInputMgr(),
            devices: Vec::new(),
            active_port: -1,
            keyboard: EFKeyboard::new(),
            skip_first_frame: false,
            notifies: Vec::new(),
            error_notifies: Vec::new(),
            simple_config: 0xd903,
            simple_action: 0,
        };
        for port in 0..2u8 {
            let mut d = EFPlayerDevice::new(0x10 + port * 4, port, 0);
            d.acquire_count += 1;
            m.devices.insert(0, d);
        }
        m
    }

    pub fn GetDevice(&mut self, id: u8) -> Option<&mut EFPlayerDevice> {
        self.devices.iter_mut().find(|d| d.device_id == id)
    }

    /// Replacement for scePadRead: the coordinator stores this frame's pad snapshot.
    pub fn SetPadState(&mut self, port: u8, pad: PadState) {
        if let Some(d) = self.devices.iter_mut().find(|d| d.port == port) {
            d.pad = pad;
        }
    }

    /// EFPlayerDeviceMgr::StartFrame -> EFInputMgr::StartFrame
    pub fn StartFrame(&mut self) {
        self.input.StartFrame(&self.devices);
    }

    /// EFPlayerDeviceMgr_SP::Update / _BC::Update(float) (0x0022cb10 / 0x001fc5b0): first call only
    /// activates every device; then updates active acquired devices and the EFInputMgr.
    pub fn Update(&mut self, dt: f32) {
        self.notifies.clear();
        self.error_notifies.clear();
        if !self.skip_first_frame {
            self.skip_first_frame = true;
            for d in self.devices.iter_mut() {
                d.ActivatePlayerDevice(true);
            }
            return;
        }
        for d in self.devices.iter_mut() {
            if d.active && d.acquire_count != 0 {
                if d.Update(dt) {
                    if let Some((obj, cb)) = d.error_notify {
                        self.error_notifies.push(DeviceErrorNotify { obj_uid: obj, callback: cb, device_id: d.device_id });
                    }
                }
            }
        }
        self.input.Update(&self.devices, &mut self.notifies);
    }

    /// EFPlayerDeviceMgr_SP::PostTerminate: reset active acquired devices.
    pub fn PostTerminate(&mut self) {
        for d in self.devices.iter_mut() {
            if d.active && d.acquire_count != 0 {
                d.Reset();
            }
        }
    }

    // ---------------------------------------------------------------- APIinput (EFApi_Input.cpp)

    pub fn APIinput_InputConfigAdd(&mut self, _mem: &mut Mem, a0: i32) -> u32 {
        self.input.InputConfigAdd(a0);
        0
    }
    pub fn APIinput_InputConfigRemove(&mut self, _mem: &mut Mem, a0: i32) -> u32 {
        self.input.InputConfigRemove(a0);
        0
    }
    pub fn APIinput_InputConfigRemoveAll(&mut self, _mem: &mut Mem) -> u32 {
        self.input.InputConfigRemoveAll();
        0
    }
    pub fn APIinput_InputConfigSetState(&mut self, _mem: &mut Mem, a0: i32, a1: i32) -> u32 {
        if let Some(c) = self.input.InputConfigGet(a0) {
            c.state = a1 as u32;
        }
        0
    }
    pub fn APIinput_InputConfigGetState(&mut self, _mem: &mut Mem, a0: i32) -> u32 {
        self.input.InputConfigGet(a0).map_or(0, |c| c.state)
    }
    pub fn APIinput_InputConfigActivate(&mut self, _mem: &mut Mem, a0: i32, a1: i32) -> u32 {
        if let Some(c) = self.input.InputConfigGet(a0) {
            c.SetActive(a1 != 0);
        }
        0
    }
    /// a2 is ignored by EXE.1
    pub fn APIinput_InputActionAdd(&mut self, _mem: &mut Mem, a0: i32, a1: i32, _a2: u32) -> u32 {
        if let Some(c) = self.input.InputConfigGet(a0) {
            c.InputActionAdd(a1);
        }
        0
    }
    pub fn APIinput_InputActionRemove(&mut self, _mem: &mut Mem, a0: i32, a1: i32) -> u32 {
        if let Some(c) = self.input.InputConfigGet(a0) {
            c.InputActionRemove(a1);
        }
        0
    }
    pub fn APIinput_InputActionRemoveAll(&mut self, _mem: &mut Mem, a0: i32) -> u32 {
        if let Some(c) = self.input.InputConfigGet(a0) {
            c.InputActionRemoveAll();
        }
        0
    }
    fn action_state(&mut self, a0: i32, a1: i32) -> Option<ActionState> {
        self.input.InputActionGet(a0, a1).map(|a| a.state)
    }
    pub fn APIinput_InputActionGetAnalogX(&mut self, _mem: &mut Mem, a0: i32, a1: i32) -> f32 {
        self.action_state(a0, a1).map_or(0.0, |s| s.x)
    }
    pub fn APIinput_InputActionGetAnalogY(&mut self, _mem: &mut Mem, a0: i32, a1: i32) -> f32 {
        self.action_state(a0, a1).map_or(0.0, |s| s.y)
    }
    pub fn APIinput_InputActionGetDigitalInfo(&mut self, _mem: &mut Mem, a0: i32, a1: i32) -> u8 {
        self.action_state(a0, a1).is_some_and(|s| s.flags & 0x20000 == 0x20000) as u8
    }
    pub fn APIinput_InputActionIsPush(&mut self, _mem: &mut Mem, a0: i32, a1: i32) -> u8 {
        self.action_state(a0, a1).is_some_and(|s| s.flags & 0x20 != 0) as u8
    }
    pub fn APIinput_InputActionIsRelease(&mut self, _mem: &mut Mem, a0: i32, a1: i32) -> u8 {
        self.action_state(a0, a1).is_some_and(|s| s.flags & 0x40 != 0) as u8
    }
    pub fn APIinput_InputActionTogglePush(&mut self, _mem: &mut Mem, a0: i32, a1: i32) -> u8 {
        self.action_state(a0, a1).is_some_and(|s| s.flags & 8 != 0) as u8
    }
    pub fn APIinput_InputActionToggleRelease(&mut self, _mem: &mut Mem, a0: i32, a1: i32) -> u8 {
        self.action_state(a0, a1).is_some_and(|s| s.flags & 0x10 != 0) as u8
    }
    /// InputContextAdd(objUid, callback, config, action, contextId, triggerFlags). The caller must
    /// pass a resolved object uid (SELF 0x2a000000 -> current object's uid) and drop the call if
    /// EFWorld::GetGameObject fails (the original then fails SetCallback and adds nothing).
    pub fn APIinput_InputContextAdd(&mut self, _mem: &mut Mem, a0: u32, a1: u32, a2: i32, a3: i32, a4: i32, a5: i32) -> u32 {
        if self.input.InputConfigGet(a2).is_some() {
            if let Some(a) = self.input.InputActionGet(a2, a3) {
                a.InputContextAdd(a2, a4, a5 as u32, a0, a1);
            }
        }
        0
    }
    pub fn APIinput_InputContextRemove(&mut self, _mem: &mut Mem, a0: i32, a1: i32, a2: i32) -> u32 {
        if let Some(a) = self.input.InputActionGet(a0, a1) {
            a.InputContextRemove(a2);
        }
        0
    }
    pub fn APIinput_InputContextRemoveAll(&mut self, _mem: &mut Mem, a0: i32, a1: i32) -> u32 {
        if let Some(a) = self.input.InputActionGet(a0, a1) {
            a.InputContextRemoveAll();
        }
        0
    }
    pub fn APIinput_InputContextSetState(&mut self, _mem: &mut Mem, a0: i32, a1: i32, a2: i32, a3: u32) -> u32 {
        if let Some(c) = self.input.InputContextGet(a0, a1, a2) {
            c.state = a3;
        }
        0
    }
    pub fn APIinput_InputContextGetState(&mut self, _mem: &mut Mem, a0: i32, a1: i32, a2: i32) -> u32 {
        self.input.InputContextGet(a0, a1, a2).map_or(0, |c| c.state)
    }
    /// identical to SetState in EXE.1 (both write context+0x08)
    pub fn APIinput_InputContextSetFlags(&mut self, mem: &mut Mem, a0: i32, a1: i32, a2: i32, a3: u32) -> u32 {
        self.APIinput_InputContextSetState(mem, a0, a1, a2, a3)
    }
    pub fn APIinput_InputContextGetFlags(&mut self, mem: &mut Mem, a0: i32, a1: i32, a2: i32) -> u32 {
        self.APIinput_InputContextGetState(mem, a0, a1, a2)
    }
    /// empty in EXE.1
    pub fn APIinput_InputContextSetCallback(&mut self, _mem: &mut Mem, _a0: u32, _a1: u32, _a2: i32, _a3: i32, _a4: i32) -> u32 {
        0
    }
    /// InputDeviceAdd(config, action, user, deviceId, slot, kind(0 X/1 Y/2 digital), sens%, index(|0x8000 analog))
    pub fn APIinput_InputDeviceAdd(&mut self, _mem: &mut Mem, a0: i32, a1: i32, a2: i32, a3: i32, a4: i32, a5: i32, a6: i32, a7: i32) -> u32 {
        let devices = &mut self.devices;
        if let Some(a) = self.input.InputActionGet(a0, a1) {
            a.InputDeviceAdd(a2, a3, a4, a5, a6, a7, devices);
        }
        0
    }
    /// EFInputAction::InputDeviceRemove is empty in EXE.1
    pub fn APIinput_InputDeviceRemove(&mut self, _mem: &mut Mem, a0: i32, a1: i32, a2: i32) -> u32 {
        if let Some(a) = self.input.InputActionGet(a0, a1) {
            a.InputDeviceRemove(a2);
        }
        0
    }
    pub fn APIinput_InputDeviceRemoveAll(&mut self, _mem: &mut Mem, a0: i32, a1: i32) -> u32 {
        if let Some(a) = self.input.InputActionGet(a0, a1) {
            a.InputDeviceRemoveAll();
        }
        0
    }
    /// note: acquires the device (count++) every call, as the original
    pub fn APIinput_InputEnableVibration(&mut self, _mem: &mut Mem, a0: i32, a1: i32) -> u32 {
        if let Some(d) = AcquirePlayerDevice(&mut self.devices, a0 as u8) {
            d.vibration_enabled = true;
            if a1 < 1 {
                for i in 0..d.nb_output() as u32 {
                    d.SetVibration(i, 0.0, 0.0, 0.0, 0.0);
                }
                d.vibration_enabled = false;
            }
        }
        0
    }
    pub fn APIinput_InputSetVibration(&mut self, _mem: &mut Mem, a0: i32, a1: u32, a2: f32, a3: f32, a4: f32, a5: f32) -> u32 {
        if let Some(d) = AcquirePlayerDevice(&mut self.devices, a0 as u8) {
            d.SetVibration(a1, a2, a3, a4, a5);
        }
        0
    }
    pub fn APIinput_ConfigIsActive(&mut self, _mem: &mut Mem, a0: i32) -> u8 {
        self.input.InputConfigGet(a0).is_some_and(|c| c.active & 1 != 0) as u8
    }
    /// InputErrorRegisterNotify(_, objUid, callback): every pad 0x10..0x17 present gets the object
    /// as error target (the original also registers notify 0xe with ExecLogicCallback; here the
    /// notify is queued in `error_notifies`). `a1` must be a resolved uid.
    pub fn APIinput_InputErrorRegisterNotify(&mut self, _mem: &mut Mem, _a0: i32, a1: u32, a2: u32) -> u32 {
        for id in 0x10u8..0x18 {
            if let Some(d) = AcquirePlayerDevice(&mut self.devices, id) {
                d.error_notify = Some((a1, a2));
            }
        }
        0
    }
    pub fn APIinput_InputSetActivePort(&mut self, _mem: &mut Mem, a0: i32) -> u32 {
        self.active_port = a0;
        0
    }

    // ------------------------------------------------- APIsimpleinput (EFApi_SimpleInput.cpp)

    pub fn APIsimpleinput_SimpleInput_ConfigCreate(&mut self, _mem: &mut Mem) -> i32 {
        self.simple_config = self.simple_config.wrapping_add(1);
        self.simple_action = 0;
        self.input.InputConfigAdd(self.simple_config as i32);
        self.simple_config as i32
    }
    /// The original dereferences the config unchecked (crash on a bad id); the port ignores it.
    pub fn APIsimpleinput_SimpleInput_ConfigEnable(&mut self, _mem: &mut Mem, a0: u32) -> u32 {
        if let Some(c) = self.input.InputConfigGet(a0 as i32) {
            c.SetActive(true);
        }
        0
    }
    pub fn APIsimpleinput_SimpleInput_ConfigDisable(&mut self, _mem: &mut Mem, a0: u32) -> u32 {
        if let Some(c) = self.input.InputConfigGet(a0 as i32) {
            c.SetActive(false);
        }
        0
    }
    pub fn APIsimpleinput_SimpleInput_ConfigIsEnabled(&mut self, _mem: &mut Mem, a0: u32) -> u8 {
        self.input.InputConfigGet(a0 as i32).is_some_and(|c| c.active & 1 != 0) as u8
    }
    pub fn APIsimpleinput_SimpleInput_ConfigSetState(&mut self, _mem: &mut Mem, a0: u32, a1: u32) -> u32 {
        if let Some(c) = self.input.InputConfigGet(a0 as i32) {
            c.state = a1;
        }
        0
    }
    pub fn APIsimpleinput_SimpleInput_ConfigGetState(&mut self, _mem: &mut Mem, a0: u32) -> u32 {
        self.input.InputConfigGet(a0 as i32).map_or(0, |c| c.state)
    }
    /// writes the head context's state (+0x54 -> +0x08), unchecked in the original
    pub fn APIsimpleinput_SimpleInput_ActionSetState(&mut self, _mem: &mut Mem, a0: u32, a1: u32, a2: u32) -> u32 {
        if let Some(c) = self.input.InputActionGet(a0 as i32, a1 as i32).and_then(|a| a.contexts.first_mut()) {
            c.state = a2;
        }
        0
    }
    pub fn APIsimpleinput_SimpleInput_ActionGetState(&mut self, _mem: &mut Mem, a0: u32, a1: u32) -> u32 {
        self.input.InputActionGet(a0 as i32, a1 as i32).and_then(|a| a.contexts.first()).map_or(0, |c| c.state)
    }

    /// SimpleInput_AddJoystickCallback(cb, left, idle, deviceId). The callbacks go to SELF
    /// (0x2a000000), which the coordinator resolves when delivering `notifies`.
    /// Ghidra lost the index argument of the two InputDeviceAdd calls; the port uses the stick axes
    /// of EFPlayerDevice_SP: left = analog 0/1 (0x8000/0x8001), right = 3/4 (0x8003/0x8004).
    pub fn SimpleInput_AddJoystickCallback(&mut self, cb: u32, left: u32, idle: u32, device: u32) {
        let cfg = self.simple_config as i32;
        let act = self.simple_action;
        let Some(c) = self.input.InputConfigGet(cfg) else { return };
        // quirk: the action is added twice (the first one stays in the list, unused)
        c.InputActionAdd(act);
        c.InputActionAdd(act);
        let base = if left != 0 { 0x8000 } else { 0x8003 };
        let devices = &mut self.devices;
        let c = self.input.InputConfigGet(cfg).unwrap();
        let a = &mut c.actions[0];
        a.InputDeviceAdd(0, device as i32, 0, 0, 0, base, devices);
        a.InputDeviceAdd(0, device as i32, 0, 1, 0, base + 1, devices);
        let trig = if idle == 0 { 0x20000 } else { 0x10000 };
        a.InputContextAdd(cfg, 0, trig, crate::engine::SELF, cb);
        self.simple_action += 1;
    }
    /// SimpleInput_AddButtonCallback(cb, button, trigger, deviceId)
    pub fn SimpleInput_AddButtonCallback(&mut self, cb: u32, button: i32, trigger: u32, device: u32) {
        let cfg = self.simple_config as i32;
        let act = self.simple_action;
        let devices = &mut self.devices;
        let Some(c) = self.input.InputConfigGet(cfg) else { return };
        let a = c.InputActionAdd(act);
        a.InputDeviceAdd(0, device as i32, 0, 2, 0, button, devices);
        a.InputContextAdd(cfg, 0, trigger, crate::engine::SELF, cb);
        self.simple_action += 1;
    }
    /// SimpleInput_AddKeyboardCallback: device 1 (keyboard) does not exist on PS2, so only the
    /// action and the context are created.
    pub fn SimpleInput_AddKeyboardCallback(&mut self, cb: u32, key: i32, trigger: u32) {
        self.SimpleInput_AddButtonCallback(cb, key, trigger, DEVICE_KEYBOARD as u32);
    }

    pub fn APIsimpleinput_SimpleInput_AddLeftJoystickCallback(&mut self, _m: &mut Mem, a0: u32) -> u32 {
        self.SimpleInput_AddJoystickCallback(a0, 1, 0, 0x10);
        0
    }
    pub fn APIsimpleinput_SimpleInput_AddLeftJoystickIdleCallback(&mut self, _m: &mut Mem, a0: u32) -> u32 {
        self.SimpleInput_AddJoystickCallback(a0, 1, 1, 0x10);
        0
    }
    pub fn APIsimpleinput_SimpleInput_AddRightJoystickCallback(&mut self, _m: &mut Mem, a0: u32) -> u32 {
        self.SimpleInput_AddJoystickCallback(a0, 0, 0, 0x10);
        0
    }
    pub fn APIsimpleinput_SimpleInput_AddRightJoystickIdleCallback(&mut self, _m: &mut Mem, a0: u32) -> u32 {
        self.SimpleInput_AddJoystickCallback(a0, 0, 1, 0x10);
        0
    }
    pub fn APIsimpleinput_SimpleInput_AddButtonDownCallback(&mut self, _m: &mut Mem, a0: u32, a1: i32) -> u32 {
        self.SimpleInput_AddButtonCallback(a0, a1, 0x20, 0x10);
        0
    }
    pub fn APIsimpleinput_SimpleInput_AddButtonUpCallback(&mut self, _m: &mut Mem, a0: u32, a1: i32) -> u32 {
        self.SimpleInput_AddButtonCallback(a0, a1, 0x40, 0x10);
        0
    }
    pub fn APIsimpleinput_SimpleInput_AddButtonPushedCallback(&mut self, _m: &mut Mem, a0: u32, a1: i32) -> u32 {
        self.SimpleInput_AddButtonCallback(a0, a1, 8, 0x10);
        0
    }
    pub fn APIsimpleinput_SimpleInput_AddButtonReleasedCallback(&mut self, _m: &mut Mem, a0: u32, a1: i32) -> u32 {
        self.SimpleInput_AddButtonCallback(a0, a1, 0x10, 0x10);
        0
    }
    pub fn APIsimpleinput_SimpleInput_AddKeyboardDownCallback(&mut self, _m: &mut Mem, a0: u32, a1: i32) -> u32 {
        self.SimpleInput_AddKeyboardCallback(a0, a1, 0x20);
        0
    }
    pub fn APIsimpleinput_SimpleInput_AddKeyboardUpCallback(&mut self, _m: &mut Mem, a0: u32, a1: i32) -> u32 {
        self.SimpleInput_AddKeyboardCallback(a0, a1, 0x40);
        0
    }
    pub fn APIsimpleinput_SimpleInput_AddKeyboardPushedCallback(&mut self, _m: &mut Mem, a0: u32, a1: i32) -> u32 {
        self.SimpleInput_AddKeyboardCallback(a0, a1, 8);
        0
    }
    pub fn APIsimpleinput_SimpleInput_AddKeyboardReleasedCallback(&mut self, _m: &mut Mem, a0: u32, a1: i32) -> u32 {
        self.SimpleInput_AddKeyboardCallback(a0, a1, 0x10);
        0
    }
    pub fn APIsimpleinput_SimpleInput_AddLeftJoystickCallback_Port(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 {
        self.SimpleInput_AddJoystickCallback(a0, 1, 0, a1);
        0
    }
    pub fn APIsimpleinput_SimpleInput_AddLeftJoystickIdleCallback_Port(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 {
        self.SimpleInput_AddJoystickCallback(a0, 1, 1, a1);
        0
    }
    pub fn APIsimpleinput_SimpleInput_AddRightJoystickCallback_Port(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 {
        self.SimpleInput_AddJoystickCallback(a0, 0, 0, a1);
        0
    }
    pub fn APIsimpleinput_SimpleInput_AddRightJoystickIdleCallback_Port(&mut self, _m: &mut Mem, a0: u32, a1: u32) -> u32 {
        self.SimpleInput_AddJoystickCallback(a0, 0, 1, a1);
        0
    }
    pub fn APIsimpleinput_SimpleInput_AddButtonDownCallback_Port(&mut self, _m: &mut Mem, a0: u32, a1: i32, a2: u32) -> u32 {
        self.SimpleInput_AddButtonCallback(a0, a1, 0x20, a2);
        0
    }
    pub fn APIsimpleinput_SimpleInput_AddButtonUpCallback_Port(&mut self, _m: &mut Mem, a0: u32, a1: i32, a2: u32) -> u32 {
        self.SimpleInput_AddButtonCallback(a0, a1, 0x40, a2);
        0
    }
    pub fn APIsimpleinput_SimpleInput_AddButtonPushedCallback_Port(&mut self, _m: &mut Mem, a0: u32, a1: i32, a2: u32) -> u32 {
        self.SimpleInput_AddButtonCallback(a0, a1, 8, a2);
        0
    }
    pub fn APIsimpleinput_SimpleInput_AddButtonReleasedCallback_Port(&mut self, _m: &mut Mem, a0: u32, a1: i32, a2: u32) -> u32 {
        self.SimpleInput_AddButtonCallback(a0, a1, 0x10, a2);
        0
    }
}

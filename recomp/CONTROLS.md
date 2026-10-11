# Controls (recomp `game`)

On first run the game writes `controls.toml` to the current directory (if absent there but present
next to `game.exe`, that one is used). Delete it to regenerate defaults. Parse errors are printed
and the defaults are used.

## Defaults

| PS2 | Keyboard / mouse | Gamepad (Xbox / PlayStation / Switch Pro) |
|---|---|---|
| Cross | Space, J | South (A / Cross / B*) |
| Circle | L, O | East (B / Circle / A*) |
| Square | U, K | West (X / Square / Y*) |
| Triangle | I | North (Y / Triangle / X*) |
| L1 / R1 | Q / E | LB / RB (L1 / R1, L / R) |
| L2 / R2 | Z / C | LT / RT analog, pressed above `trigger_threshold` |
| L3 / R3 | Left Shift / R, middle mouse | stick clicks |
| Start / Select | Enter / Tab, Backspace | Start (Menu, Options, +) / Select (View, Share, -) |
| D-pad | Numpad 8/2/4/6, 1/2/3/4 | D-pad |
| Left stick (move) | W A S D | left stick |
| Right stick (camera) | arrow keys (+ mouse if `mouse_camera = true`) | right stick |
| quit | Esc (fixed) | — |

\* positional mapping by gilrs (Switch Pro labels are swapped vs. Xbox).

## Format

```toml
deadzone = 0.15            # radial stick deadzone (0..0.95)
trigger_threshold = 0.3    # LT/RT analog value counted as L2/R2
mouse_camera = false       # mouse motion -> right stick
mouse_sensitivity = 0.05   # stick units per pixel per frame

[keyboard]                 # action = ["Name", ...]
cross = ["Space", "KeyJ"]
lstick_up = ["KeyW"]       # lstick_/rstick_ up/down/left/right: keyboard only
r3 = ["KeyR", "Mouse.Middle"]

[gamepad]
cross = ["South"]
l2 = ["LeftTrigger2"]
```

Actions: `cross circle square triangle l1 r1 l2 r2 l3 r3 start select up down left right`
(+ `lstick_*`, `rstick_*` in `[keyboard]`). Missing entries keep defaults; `[]` unbinds.

Names are Bevy debug names: keys = `KeyCode` (`KeyA`, `Digit1`, `Space`, `ShiftLeft`, `ArrowUp`,
`Numpad8`, `F5`, ...), mouse = `Mouse.Left|Right|Middle|Back|Forward`, gamepad = `GamepadButton`
(`South East North West LeftTrigger RightTrigger LeftTrigger2 RightTrigger2 Select Start Mode
LeftThumb RightThumb DPadUp DPadDown DPadLeft DPadRight`). Misspelled names are silently ignored.

## Gamepads

gilrs (XInput, DualShock 4/DualSense, Switch Pro, SDL-mapped generic pads), hot-plug supported.
With several pads the one that was used most recently is read. Sticks: radial deadzone, rescaled,
mapped to libpad bytes (0..255, centre 128, y down = 255); the game applies its own 40/128 inner
deadzone on top (EFPlayerDevice_SP::StickValue). Keyboard and pad inputs are merged (pad sticks
override keyboard when outside the deadzone). Pressure bytes are filled (255 held, triggers analog)
but the game only reads them in pressure mode, which it never enables.

# Controls (recomp `game`)

P34: controls live in `settings.toml` (current directory; if absent there but present next to
`game.exe`, that one is used), together with the graphics / audio / remaster / launcher settings.
On the first run without settings.toml an existing `controls.toml` is migrated into it (the old
file is left alone and no longer read). The format below is unchanged (an old controls.toml copied
to settings.toml works). Edit it with the launcher (`launcher.exe`, Controls tab: sliders + key /
pad rebinding) or in game: pause > Options > SELECT = PC Settings (deadzone, trigger threshold,
mouse camera / sensitivity, invert camera Y). Delete it to regenerate defaults. Parse errors are
printed and the defaults are used.

## Defaults

| PS2 | Keyboard / mouse | Gamepad (Xbox / PlayStation / Switch Pro) |
|---|---|---|
| Cross | Space, J | South (A / Cross / B*) |
| Circle | L, O | East (B / Circle / A*) |
| Square | U, K | West (X / Square / Y*) |
| Triangle | I, Backspace (= back in menus, as the original's own PC key table: DIK_BACK) | North (Y / Triangle / X*) |
| L1 / R1 | Q / E | LB / RB (L1 / R1, L / R) |
| L2 / R2 | Z / C | LT / RT analog, pressed above `trigger_threshold` |
| L3 / R3 | Left Shift / R, middle mouse | stick clicks |
| Start / Select | Enter / Tab | Start (Menu, Options, +) / Select (View, Share, -) |
| D-pad | Numpad 8/2/4/6, 1/2/3/4; arrow keys in menus (front end, pause, PC SETTINGS), as the original PC key table | D-pad |
| Left stick (move) | W A S D | left stick |
| Right stick (camera) | arrow keys in gameplay (+ mouse if `mouse_camera = true`) | right stick |
| Start (pause) | Esc (fixed, in addition to the `start` keys) | — |
| fullscreen | F11 (fixed, borderless toggle) | — |
| quit | close the window (Alt+F4); in game: pause -> Quit returns to the main menu | — |

\* positional mapping by gilrs (Switch Pro labels are swapped vs. Xbox).

## Format

```toml
deadzone = 0.15            # pad stick drift cut (0..0.95), not rescaled
trigger_threshold = 0.3    # LT/RT analog value counted as L2/R2
mouse_camera = false       # mouse motion -> right stick
mouse_sensitivity = 0.05   # stick units per pixel per frame
invert_y = false           # P34: invert the camera (right stick / mouse) Y axis

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
LeftThumb RightThumb DPadUp DPadDown DPadLeft DPadRight`). Unknown names are dropped with a message on stderr / in game.log
(an action left with no valid name gets its defaults back; `[]` still unbinds). One key on two
actions is allowed (both fire) and reported.

## Gamepads

gilrs (XInput, DualShock 4/DualSense, Switch Pro, SDL-mapped generic pads), hot-plug supported.
With several pads the one that was used most recently is read. Sticks: drift cut (below `deadzone`
-> centre, otherwise raw, not rescaled), mapped to libpad bytes (0..255, centre 128, y down = 255); the game applies its own 40/128 inner
deadzone on top (EFPlayerDevice_SP::StickValue). Keyboard and pad inputs are merged (pad sticks
override keyboard when outside the deadzone). Pressure bytes are filled (255 held, triggers analog)
but the game only reads them in pressure mode, which it never enables.

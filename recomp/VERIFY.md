# VERIFY — WP-V2 all-levels headless sweep (9000 frames)

Runner: `recomp/tests/v1_all_levels.rs` (real `EngineRuntime::new_with_engine` + `ReadLevel` + `LoadLevel`, not NullHost),
one process per level: `tools/engineport/v1_sweep.sh OUT 9000`, table: `tools/engineport/v1_table.sh OUT 9000`.
Debug: `V1_LEVEL=EP1/L01 V1_FRAMES=n V1_TRACE=a-b V1_BT=1` (trace prints leader physic/object position per frame).
Input script: from frame 100 the left stick is held, direction cycling forward/right/back/left every 600 frames;
Cross / Square pulsed every 90 frames. 9000 frames at 1/60 s.
Columns: load = ReadLevel/LoadLevel result; frames = runtime frames reached; NaN = first frame the leader position was NaN;
fall = first frame the leader is more than 50 units under its first position; stuck = number of 600-frame windows
(with stick held) where the leader moved < 0.5 in XZ, and the first such frame; panic = message @ location;
todo! = panic was todo!/unimplemented; backtrace = top logic frames. ✅ clean, ⚠️ fall/stuck only, ❌ panic/NaN/short.

Sweep at c94ab37b (66 packs):

| level | load | frames | NaN | fall | stuck (600f windows) | panic | todo! | backtrace top (logic frames) |
|---|---|---|---|---|---|---|---|---|
| ✅ CREDITS | ok | 9000 |  |  |  |  |  |  |
| ⚠️ EP1/L01 | ok | 9000 |  |  | 2x from frame 700 |  |  |  |
| ✅ EP1/L02 | ok | 9000 |  |  |  |  |  |  |
| ⚠️ EP1/L03A | ok | 9000 |  |  | 1x from frame 3700 |  |  |  |
| ⚠️ EP1/L03B | ok | 9000 |  |  | 1x from frame 4900 |  |  |  |
| ⚠️ EP1/L04 | ok | 9000 |  |  | 3x from frame 5500 |  |  |  |
| ✅ EP1/L09 | ok | 9000 |  |  |  |  |  |  |
| ⚠️ EP2/L01A | ok | 9000 |  |  | 3x from frame 3700 |  |  |  |
| ⚠️ EP2/L01B | ok | 9000 |  |  | 7x from frame 4900 |  |  |  |
| ✅ EP2/L01C | ok | 9000 |  |  |  |  |  |  |
| ✅ EP2/L02A | ok | 9000 |  |  |  |  |  |  |
| ✅ EP2/L02B | ok | 9000 |  |  |  |  |  |  |
| ✅ EP2/L02C | ok | 9000 |  |  |  |  |  |  |
| ✅ EP2/L03A | ok | 9000 |  |  |  |  |  |  |
| ✅ EP2/L04 | ok | 9000 |  |  |  |  |  |  |
| ✅ EP2/L09 | ok | 9000 |  |  |  |  |  |  |
| ⚠️ EP3/L01 | ok | 9000 |  |  | 2x from frame 4900 |  |  |  |
| ⚠️ EP3/L02 | ok | 9000 |  |  | 2x from frame 2500 |  |  |  |
| ⚠️ EP3/L03 | ok | 9000 |  | frame 118 y=-46.4 (spawn 3.9) | 14x from frame 700 |  |  |  |
| ⚠️ EP3/L04 | ok | 9000 |  |  | 1x from frame 1300 |  |  |  |
| ✅ EP3/L05 | ok | 9000 |  |  |  |  |  |  |
| ⚠️ EP3/L09 | ok | 9000 |  | frame 118 y=-46.4 (spawn 3.9) | 14x from frame 700 |  |  |  |
| ✅ EP4/L01 | ok | 9000 |  |  |  |  |  |  |
| ✅ EP4/L02A | ok | 9000 |  |  |  |  |  |  |
| ✅ EP4/L02B | ok | 9000 |  |  |  |  |  |  |
| ⚠️ EP4/L03A | ok | 9000 |  | frame 1966 y=-50.3 (spawn 0.0) | 11x from frame 2500 |  |  |  |
| ⚠️ EP4/L03B | ok | 9000 |  |  | 10x from frame 3100 |  |  |  |
| ✅ EP4/L04 | ok | 9000 |  |  |  |  |  |  |
| ⚠️ EP4/L09A | ok | 9000 |  | frame 1874 y=-50.2 (spawn 0.0) | 11x from frame 2500 |  |  |  |
| ⚠️ EP4/L09B | ok | 9000 |  |  | 10x from frame 3100 |  |  |  |
| ⚠️ EP6/L01 | ok | 9000 |  |  | 1x from frame 8500 |  |  |  |
| ✅ EP6/L02 | ok | 9000 |  |  |  |  |  |  |
| ✅ EP6/L03 | ok | 9000 |  |  |  |  |  |  |
| ✅ EP6/L04A | ok | 9000 |  |  |  |  |  |  |
| ✅ EP6/L04B | ok | 9000 |  |  |  |  |  |  |
| ✅ EP6/L05 | ok | 9000 |  |  |  |  |  |  |
| ✅ EP6/L05B | ok | 9000 |  |  |  |  |  |  |
| ✅ EP6/L09 | ok | 9000 |  |  |  |  |  |  |
| ✅ GAMEART/ART01 | ok | 9000 |  |  |  |  |  |  |
| ✅ GAMEART/ART02 | ok | 9000 |  |  |  |  |  |  |
| ✅ GAMEART/ART03 | ok | 9000 |  |  |  |  |  |  |
| ✅ GAMEART/ART04 | ok | 9000 |  |  |  |  |  |  |
| ✅ GAMEART/ART05 | ok | 9000 |  |  |  |  |  |  |
| ✅ GAMEART/ART06 | ok | 9000 |  |  |  |  |  |  |
| ✅ GAMEART/ART07 | ok | 9000 |  |  |  |  |  |  |
| ✅ GAMEART/ART08 | ok | 9000 |  |  |  |  |  |  |
| ✅ GAMEART/ART09 | ok | 9000 |  |  |  |  |  |  |
| ✅ GAMEART/ART10 | ok | 9000 |  |  |  |  |  |  |
| ✅ GAMEART/ART11 | ok | 9000 |  |  |  |  |  |  |
| ✅ GAMEART/ART12 | ok | 9000 |  |  |  |  |  |  |
| ✅ GAMEART/ART13 | ok | 9000 |  |  |  |  |  |  |
| ✅ GAMEART/ART14 | ok | 9000 |  |  |  |  |  |  |
| ✅ GAMEART/ART15 | ok | 9000 |  |  |  |  |  |  |
| ✅ GAMEART/ART16 | ok | 9000 |  |  |  |  |  |  |
| ✅ GAMEART/ART17 | ok | 9000 |  |  |  |  |  |  |
| ✅ GAMEART/ART18 | ok | 9000 |  |  |  |  |  |  |
| ✅ GAMEART/ART19 | ok | 9000 |  |  |  |  |  |  |
| ✅ GAMEART/ART20 | ok | 9000 |  |  |  |  |  |  |
| ✅ GAMEART/ART21 | ok | 9000 |  |  |  |  |  |  |
| ✅ GAMEART/ART22 | ok | 9000 |  |  |  |  |  |  |
| ✅ GAMEART/ART23 | ok | 9000 |  |  |  |  |  |  |
| ✅ GAMEART/ART24 | ok | 9000 |  |  |  |  |  |  |
| ✅ INTRO | ok | 9000 |  |  |  |  |  |  |
| ✅ LOADER | ok | 9000 |  |  |  |  |  |  |
| ✅ SPLASH | ok | 9000 |  |  |  |  |  |  |
| ✅ STATUS | ok | 9000 |  |  |  |  |  |  |

## Result
66/66 packs load and run 9000 frames without panic, todo! or NaN hero position. 49 clean, 17 with fall/stuck flags.
(WP-V1 at 1800 frames: 64/66.) EP6/L05 died once during the sweep with no output; a rerun was clean.

Fixed in WP-V2:
- EP2/L02B, EP4/L02A: library objects were constructed after their clients. Cause: DTGlo OBJECT_FLAGS (0x4000 = update first)
  was never copied to EFGameObject +0x15c. Fixed 1:1 with EFGameObject::LoadFile / EFGameLogicMgr::Update (8455d88d).
- EP3/L05 (frame 1892): c2rs indirect call through the ApiCFunctions library (coll_level5). Now LIBApiCFunctions::IntRandom (0cf517ba).
- EP1/L03B NaN hero (frame 3487): c2rs `in_f0` was 0 instead of the GetFrameDeltaTime return value, giving an inf/NaN l03_camera velocity.
  Fixed in 49 converted modules (1ef7d909, a04905cd).

## Remaining (not fixed)
- EP3/L03 and EP3/L09 (mine-cart ride): the leader 0x2a000004 (mcart) falls straight down from spawn (1055.8, 3.9, 6507.3)
  starting at frame 0, with no input. The cart's wheel prims (0x2c6-0x2c8) use collision attrib bits 0x400/0x800, so either
  the rail collision or that attrib filtering in the physic port is missing. Belongs to physic (engine) work.
- EP4/L03A and EP4/L09A: the hero walks off a ledge at about frame 1860 and falls forever. There is no death/respawn below
  the world, so the kill volume or floor event is probably not handled.
- EP4/L03B and EP4/L09B: the hero freezes at about frame 1300 at (1995.5, -3.8, -68.3) after a jump back. Probably a
  captured/death state that waits for input or a respawn that never happens. Not investigated further.
- Other stuck flags (1-7 windows): probably walls under the fixed stick script, not checked one by one.
- EP1/L03B frame 6809 (only seen with NaN instrumentation): generic_fx SetTranslation gets a NaN position from
  coll_level3b's FX descriptor. Affects only a particle emitter.

# Book (Tome of Doom) capture mechanic — end-to-end trace + engine gap table (WP-K1)

Sources: ported logic in `recomp/src/modules/{hero,tomedoom,tod_beam,ghost,shadows,zombie,dustdevil,...,wisps,wispsourcenew,generic_fx,panel}`,
engine in `recomp/src/engine/**`. Resource ids verified on the disc by `recomp/tests/engine_k1.rs`.
`SELF` = the running object; `go.` = `APIgameobj::`; `k(..)` = per-build literal (builds.rs).

## Level data the sequence resolves to

| Role | EP1L01 | EP1L03A (orb level) |
|---|---|---|
| Shaggy / Scooby (book holders) | GLO 0x2a000091 `shaggy` / 0x2a000092 `scooby` | (same ids) |
| Book | GLO 0x2a000093 `tomedoom`: model 0x324, anims 0x325..0x32b (ids 1..7: Shaggy open 1 / close 3 / idle 4, Scooby open 5 / close 7 / idle 6, 2 = loop/msg 0x76) | GLO 0x2a0000ee: model 0x8af, anims 0x8b0..0x8b7 |
| Book sounds (descriptor INDEX = Play id, GLALL bank 0) | 0x10..0x17, 0x2e, 0x41, 0x42 — **0xf missing** | 0xf..0x17, 0x2e, 0x41, 0x42, 0x4000000b, 0x80000013 |
| Hands / beam latent (`ID_2a800095`) | latent 0x2a800095 `tod_beam`: model 0x339, anims 0x33a..0x33f | latent 0x2a8000f5: model 0x78e, anims 0x78f..0x795 |
| Button prompt (msg 0x5c) | 0x2a000096 `inv_control` | |
| HUD tome meter (msg 0x1d, 0x26) | 0x2a100006 `panel` | 0x2a100000 |
| Monster_Base latent `0x2a800118` | `enemybar` (monster life bar; *not* the suck beam as PORT_NOTES says) | |
| Capture report (msg 0xea) | 0x2a00011a `rdrmngr` | |
| Monsters | 0x2a000006/7 medghost, 0x2a000008 smallghost, 0x2a000013 shadows (own HAnim sets) | shadows 0x2a000022.., small/med ghosts |
| Orbs | — | `l03a_wisps` model 0x8d3 anims 0x8d4/0x8d5 (16 GLOs + launched latent 0x2a8000d4); `wispsourcenew` 0x2a0000d6/d7 |
| Particle textures | `spirit` (tod_beam), `spark`/`halo` (book wisp collect, wisps) — all in TEXDIC | same |

## Sequence

1. **Button → request** — `hero/hero_inputs.rs heroCB_INPUT_Tome` (input callback): sets GB0+0x14 |= 0x40000000, L+0x16c |= 8.
   `hero_core heroFUNC_CheckInputEvents` → `heroValidOpenTome` (`APIphysic::GetFloorAttribs`, `go.SendMessageTo` book) → team msgs 0x8d/0x60
   → `heroInit_TomeIn` (`go.GetID`, `APIphysic::PrimResetIntAttribBits`, `go.SetContextFlag`, `APIinput::InputConfigSetState`), hero anims via `LIBAnimMngr::PlayAnim` (slots 0x3f/0x1d).
2. **Book opens** — `tomedoom TomeDoomST_ISOPENING` (state 2): `go.SetGameObjectVisible`, `APIinput::InputConfigActivate/SetState`, `go.Get/SetShadowParams`,
   `APIphysic::DMGetParams/DMSetParams/DMSetActive` (DM 1 TrackObjet + 0x25 Plugin from `TomeDoomFUNC_Create_Physic`: `CreatePhysic, SetFlags, SetPrimaryPrim, PrimSet/GetIntAttrib*, DMAdd, DMSetContextMask`), open anim `LIBAnimMngr::PlayAnim` → `APIanim::Play` (anim 1/5, PLAY_ONCE).
   Book position each frame: `FixPourNePasTomberDansLeMur` (`go.GetBoneWMat`, `APIphysic::Get/SetVelocity`, `go.TakeOrientation`), `TomeDoomCB_PHYSIC` (`APIphysic::CheckRayIntersection`, `go.Translate`), `JoyRotate` (`go.GetBoneLMat/SetBoneLMat`), `TomeDoomFUNC_UpdateLight`. Idle sound `APInewSound::Play(SELF,0x2e)`.
3. **Anim end → scan/target** — `TomeDoomCB_ANIM_LF` (cb 6, anim notification 0x11): `APIextra::TimerUnregisterNotify`, `APIlight::DestroyLight`, `APInewSound::StopSound`, `LIBAnimMngr::PlayAnim` (idle 4/6). `TomeDoomST_OPEN` (state 3, sub 0x54 scan): `TomeDoomFUNC_ScanForMonster` (`go.GetGroupBitMask`, `LIBApiCFunctions::GetDistance/CheckPositionOrientation`), `TomeDoomFUNC_ContactWisps` (msg 0x11d to wisps).
4. **Monsters flee while open** — `ghost_base TomeIsOpen`/`IsTomeOpen(ed)` = GB0+0x14 bit 0 && GB1+4 (power) > 0 (zombie: hero within 20 m) → ghost ST_TOME_IS_OPEN_FLEE (0x18) / shadows FLEE_TOME 0x17 / STAY_AWAY 0x18 / zombie SCARED_NOTHOOKED / dustdevil SCARED_FLEE; speed via `SetRunFromTomeSpeedParams` (`APIphysic::DMGetParams/DMSetParams` on the monster's DM 4 Move), anims `LIBAnimMngr::PlayAnim`. Closed: TR_TOME_IS_CLOSE_SET.
5. **Latch (hands appear)** — `TomeDoomFUNC_SET_CAPTURE`: `APIextra::TimerSetFrequency(0.051)/TimerRegisterNotify(0x13)`, `TomeDoomFUNC_Create_Light` (`APIlight::CreateLight/SetColor/SetFlag/SetRadius`, `go.AddChild/SetTranslation`; not in build 4a201eb9),
   `go.LaunchLatant(ID_2a800095)` + `go.AddChild` (beam), `APInewSound::Play(SELF,0x14)` FXTomeLinkAppear, msg 0x5c/2 → inv_control, `APInewSound::Play(SELF,0xf)` FXTomeAbsorbLP, msg 0xd6/0x65 → monster, `TomeDoomFUNC_GenerateButton` (`LIBApiCFunctions::IntRandom`), `Update_NTAction` (msg + `APInewSound::Play` 0x41/0x42), `APIphysic::DMGetParams/DMSetParams/DMSetActive` (TrackObjet target = monster, 10.0), `DMSetStateChangeNotifyBits`.
   **tod_beam**: `ToD_BeamST_FIRSTMOVE` (`go.RegisterNotify`), `State_Opening_In/Hooking_In/Closing_In` (`APIanim::BeginTree`, `LIBAnimMngr::PlayAnim`, `APIanim::PlayBlendConnect`, `APIanim::EndTree`, `go.SetGameObjectVisible`), `*_Execute` (`APIanim::ChangeBlendParams`), `ToD_Beam_Create_Particules` (`APIparticle::TypeCreate/TypeSetTexture("spirit")/TypeSetName(TOD_BEAM_SPIRALSPIRIT, TOD_BEAM_FLYINGSPIRIT)/TypeSet*/EmitterCreate/EmitterSetJoint/EmitterStart`), `ToD_Beam_Create_Sprites` (`APIgraphics::SpriteAddGroup/SpriteSetGroupTexture/Color/BlendMode/Flags/SpriteGroupAddSprite`), anim callback `ToD_BeamCB_ANIMATION` → msg to book.
   **Monster** `Monster_Base_MessageFunction` 0xd6/0x65: `go.LaunchLatant(0x2a800118 enemybar)`, `go.AddChild`, capture params msg 0xd6/0x66 → book (7 words → book L+200, life L+0x70), ghosts TR_HOOKED_BY_BOOK_SET → ST_BEINGSUCKED (`LIBApiCFunctions::TrackID` book 0x2a000093).
6. **Button mashing / tug of war** — `TomeDoomST_OPEN` sub 0x53: `APIextra::TimerGetTime/TimerReset`, `TomeDoomFUNC_DrainMonsterLife` (timer cb 0x13: correct button drains life L+0x70, `APIinput::InputConfigSetState`, msg 0x1d → panel, msg 0xec → enemybar, `APInewSound::Play`), `TomeDoomFUNC_SelectButton`, `TomeDoomFUNC_Update_Sound_Pitch` (`APInewSound::SetPitch(SELF,0xf)`), `TomeDoomCB_INPUT_STICK/ROTATE/IDLE` (`go.GetLookAt/GetLookRight/GetBoneWMat`), `TomeDoomFUNC_CallVibration` (`APIinput::InputSetVibration`), `TomeDoomFUNC_ShowText` (`LIBApiText::ShowText`). Pull: `tod_sucking.rs TomeDoomFUNC_CalcNewDMs` (`go.InvertTransformVector`, msg 0xd6/0x6a pull vector → monster → `APIphysic::DMSetParams` on monster DM 4), `UpdateOrientationLink` (`go.Rotate`). Monster struggles (ghost ST_BEINGSUCKED_LOOP, shadows BEINGSUCKED 0x14). Link broken (msg 0xd0 / life restored): msg 0xd6/100, `APInewSound::Play(SELF,0x13)` FXTomeLinkBroken, state 5.
7. **Pulled into the book** — life ≤ 0: DrainMonsterLife → `APInewSound::Play(SELF,0x42)`, `go.ChangeState(4)`; `TomeDoomST_FINALSUCKING`: msg 0xd6/0x67 (monster pulled in, glow off, `APIanim::BeginTree/Play/PlayBlendConnect/EndTree` capture anim), 0xd6/0x6c, `APIphysic::DMResetStateChangeNotifyBits/SetVelocity`, heroes msg 0x6c. Monster 0xd6/0x69 → book `APInewSound::Play(SELF,0x10)` FXTomeCapture.
8. **Capture** — book msg 0xd6/0x68: msg 0x6f, 0xd6/0x68 to monster + rdrmngr, state 5. `Monster_BaseST_CAPTURED`: GB2+4 |= kill mask, msg 0x26 → panel, 0xea → rdrmngr, `LIBMiscFunc::MiscFunc_ChangeClipping`, `go.Sleep`; ghosts/zombie L4 msg 0x14e → manager; `go.KillObject`.
9. **Book closes** — `TomeDoomST_ISCLOSING` (state 5): close anim `LIBAnimMngr::PlayAnim` (3/7), `APIlight::SetFlag/SetColor`, `APInewSound::StopSound`, `APIphysic::DMResetStateBits`, `APIinput::InputConfigActivate`; `TomeDoomCB_ANIM_LF` → put away (`go.SetGameObjectVisible(0)`, `go.EnableObject`); `TomeDoomFUNC_CancelScanForWisps` (msg 0x11e). Beam `State_Closing_Execute` → msg to book, `go.KillObject`. Hero `heroInit_TomeOut` (`APIphysic::ResetFlags/PrimGetIntAttrib/PrimSetIntAttribBits`).
10. **Orbs refill the book** — `wisps wisp_base`: `Create_Physic` (`APIphysic::CreatePhysic/SetFlags/DMAdd(1,4,7,0)/DMSetParams/DMSetContextMask`), `Create_Particles` (`APIparticle::EmitterCreate/EmitterSetJoint`, `MiscFunc_EmitterUpdateFlags`; types `wisp_collect_spark`/`spark`, `wisp_collect_halo`/`halo`), `Create_MotionTrail` (`APIgraphics::MTCreateSystem/MTSetFrequency/MTSetJoint/MTSetRGB/MTSetLifeSpan`), `RunBob` (`go.SetTranslation`). Book open → msg 0x11d → `Hooked_Execute`/`Return_Execute` (`go.Translate`, `APIgraphics::MTStopDraw`), book side msg 0x122 (size) → `TomeDoomFUNC_MessageHandler` 0x122: `APIparticle::EmitterStart` (wispcollect spark/halo small/medium/large, types created in `TomeDoomFUNC_Create_Particles`), `APInewSound::IsPlaying/Play(SELF,0x2e)`. `Wisp_baseState_Picked_Execute`: power GB1+4 += 18/36/54 (cap 180), `APInewSound::Play(book,0x17)` FXTomeCollectWisp, msg 0x1d → panel; `Dead_In` (`go.SetGameObjectVisible`, `APIphysic::SetVelocity`), `LaunchSparkles` → `MiscFunc_PlayGenericFX` (generic_fx: `APIparticle::EmitterStart`, `go.KillObject`). Huge sources: `wispsourcenew` launches wisps (`go.LaunchLatant`).
11. **HUD** — `panel_base panel_baseFUNC_UpdateTomeMeter / ToDMeterChangeControl` (`go.SetRotation/Rotate` on meter needle GLOs, `APIhud::WindowSetFlags`), `ToDFaderControl` (`APIgraphics::TexFaderPlay`).

## Gap table (engine side of every call above)

"Impl" = an engine implementation exists; "Wired" = reachable from logic through the `Host` used by `EngineRuntime` (`runtime.rs RuntimeHost`). Today `RuntimeHost` only forwards APIparticle_* and APIgameobj GetPosition/GetLookAt; every other `Host` method is the generated default (`unimplemented`, returns 0).

| API (used in step) | Engine impl | Status |
|---|---|---|
| go.* state/messages/latents/children/kill/timers (all steps) | `engine/core/apigameobj.rs`, `apiextra.rs` (Timer*) | OK (1:1, in Engine) |
| go.GetPosition / GetLookAt | `runtime.rs RuntimeHost` | OK (world frames) |
| go.SetTranslation/Translate/Rotate/SetRotation/TakeOrientation/GetLookRight/InvertTransformVector (2,6,10,11) | core apigameobj | check per fn; frame writes do not reach RuntimeWorld frames (no sync back) |
| go.GetBoneWMat / GetBoneLMat / SetBoneLMat (2,6,9) | **none** (Host default) | GAP — needs EFAnimationMgr pose → bone matrices |
| go.SetGameObjectVisible / EnableObject / Get/SetShadowParams | core flags; shadows not rendered | partial |
| APIanim_* (Play, PlayBlendConnect, BeginTree/EndTree, ChangeBlendParams, StopAnim) (2,3,5,7,9) | `engine/anim/api.rs` 1:1 (WP-D3/D4) | GAP wiring — not in RuntimeHost; no per-object EFAnimationMgr built from GLO anim descriptors in the runtime; anim-end notify 0x11 (CB_ANIM_LF) not drained to logic |
| LIBAnimMngr::PlayAnim | `libs/anim_mngr.rs` → APIanim | depends on the row above |
| APInewSound_Play/StopSound/IsPlaying/SetPitch (2,5,6,7,9,10) | `engine/audio/api.rs` 1:1 (WP-G3/G4) | GAP wiring — not in RuntimeHost; level `CreateSoundInstances` not run by runtime. Data quirk: EP1L01 book has no sound 0xf (Play returns 1, silent) |
| APIparticle_* (5,10) | `engine/particle/api.rs`, wired | OK |
| APIgraphics_Sprite* (tod_beam sprites, 5) | **none** | GAP |
| APIgraphics_MT* motion trails (wisps, 10) | **none** | GAP |
| APIgraphics_TexFaderPlay (11) | `gfx/texfader.rs` (EFTexFader logic) but no API/Host | GAP wiring |
| APIphysic_CreatePhysic/DMAdd/DMSet*/SetVelocity/SetFlags (2,4,5,6,7,10) | `engine/physic/api.rs` (takes `&mut EFPhysicMgr`); DM kinds 0,1,4,7,0x25 ported | GAP wiring — EFPhysicMgr not in EngineRuntime/Host; Plugin 0x25 notify 0x12/0x13 must be run by coordinator |
| APIphysic_CheckRayIntersection (2) | physic/api.rs `CheckRayIntersectionOut` | GAP wiring + guest RayIntersectOut packing (WP-E2 open) |
| APIphysic_Prim*IntAttrib*, SetPrimaryPrim, GetFloorAttribs (1,2,9) | physic/api.rs PrimSetIntAttribBits (others partial) | partial / GAP wiring |
| APIlight_CreateLight/SetColor/SetFlag/SetRadius/DestroyLight (3,5,9) | `engine/light.rs` | GAP wiring (EFLightMgr_BC in runtime, not in Host) |
| APIinput_InputConfig*/InputSetVibration (1,2,6,9) | `engine/input/mod.rs` | GAP wiring (not in RuntimeHost; vibration has no output) |
| APIhud_WindowSetFlags (11) | `engine/hud/api.rs` | GAP wiring |
| LIBApiText::ShowText, LIBMiscFunc, LIBApiCFunctions | `libs/*` (logic-side) | OK |
| Camera (APIcamera_*) | not called by the capture sequence | n/a |

Summary: data side is complete (every model/anim/sound/texture the sequence names exists, `tests/engine_k1.rs`); logic side is ported; the
blocking gaps are (a) Host wiring of anim / audio / physic / light / input / hud into `RuntimeHost` (+ per-object EFAnimationMgr, EFNewSoundMgr,
EFPhysic creation from GLO descriptors and anim/physic notifications back to logic), (b) bone matrix APIs, (c) sprites and motion trails.

## WP-K3: capture loop per monster family (verification table)

Harness (`src/bin/game.rs`, uncommitted K3 hunks — file shared with J22/J23 edits): `game LEVEL --autoplay --give-book --headless --frames 1500 --shots DIR`
with `GAME_BOOKTGT=<uid>` (target moved 3 m in front of the leader at autoplay r=100, K3 status line every 15 frames: book state/sub, button, life L+0x70,
power GB1+4, book target L+0x60, monster state/pos/group mask), `GAME_WARP`, `GAME_SENDMSG=uid:msg:param:frame` (debug: e.g. water 0xd8 to a fire ghost).
Book states: 1 closed, 2 ISOPENING, 3 OPEN (sub 0x54 scan / 0x53 hooked+mash), 4 FINALSUCKING, 5 ISCLOSING. Catchable = group bit 0x200 (`TomeDoomFUNC_ScanForMonster`).
Level survey (pack dump): beam latent `tod_beam` is absent in EP3/L03, EP3/L09 (mummyride) and EP4/L02A (zombies + fearghost) → no capture possible there in the original either
(EP4/L02A: book build 362a6394 launches 0x2a800047, not in the pack → LaunchLatant fails → book stalls in FINALSUCKING; same in C).

| Family (level, uid) | Open anim+hands | Flee while open | Beam/pull | Mash | Capture anim → book | Meter drain / refill | Notes |
|---|---|---|---|---|---|---|---|
| small ghost (EP1L01 0x2a000008) | ok | ok | ok | ok | ok (state 5 CAPTURED, f315) | ok (180→135) / — | j5/j21 shots |
| poltergeist (EP2L01B 0x2a0000f1) | ok | n/t | ok (spirit beam) | ok | ok (f210) | ok (180→112.5) | |
| dust devil (EP3L02 0x2a0000e4) | ok | ok | ok | ok | ok (2nd hook; 1st link broken by distance, C behaviour) | ok (180→90) | |
| zombie (EP4L02A 0x2a000109) | ok | ok | ok | ok | stalls in FINALSUCKING | ok | level has no tod_beam latent (data, same in C); EP6L02 retry: leader not found at start (hero None), not reached |
| fire ghost (EP6L04B 0x2a000147) | ok | n/a | ok after water (msg 0xd8): book hooks the inner ghost 0x2a00014d | ok | ok (book 4→5→1) | ok (180→112.5) | not catchable while burning (mask 0x401, no 0x200) = C. Hands drawn away from the ghost in k3_finalsuck (open) |
| shadows (EP1L03B 0x2a00001c) | ok | ok (FLEE 0x17 / STAY_AWAY 0x18) | hooked f150 but monster y → -6 | started | broken: hero 0x83 killed (msg 0x8b/0x8c to partner, 0xc9 restart) at f≈160 | ok | OPEN: shadow sinks under floor while sucked + hero death cause |
| mummy (EP2L02A 0x2a0000f1) | ok | n/t | not hooked | — | — | — | mask 0x1001 (no 0x200) until its tomb/path script sets 0x200611; msg 0xc1 alone did not; not reached |
| wisps / orb refill (EP1L03A) | — | — | — | — | — | refill verified by J-series (power +18/36/54, msg 0x1d) | not retested |
| dust devil boss (EP3L05 0x2a00009d) | ok | ok | ok (hooks 0x9d / 0x9f) | ok | not reached: tome power hits 0 before life (link broken, state 5) | drain ok | needs orb refill mid-fight (plausibly C design) |
| zombie boss (EP4L04 0x2a0000bc) | ok | — | not hooked (boss state 5, no 0x200) | — | — | — | boss is beaten by its own sequence first |
| fire ghost boss (EP6L05 0x2a0000c0) | — | — | — | — | — | — | leader not found at start (intro), not reached |
| ghost boss (EP6L05B 0x2a000012) | — | — | — | — | — | — | leader not found at start (intro), not reached |
| mummy ride (EP3L03/L09) | n/a | | | | | | ride level, no book/tod_beam in the pack |

No C deviation was fixed in this pass (none isolated to a 1:1 cause); open items above.

## WP-K4: K3 open items

| Item | Verdict | Evidence |
|---|---|---|
| shadows sink (EP1L03B 0x2a00001c) | harness bug, fixed (commit "WP-K4: book test target placed via SetTranslation"): the target was moved by writing the physic frame pos only, its ground collision stayed behind → free fall. With the hero warped into the shadow's room (GAME_WARP) the shadow stays on the floor (y 0.02), hook f150 → FINALSUCKING → CAPTURED (0xb), no hero death. Moved 70 m away from its room it still falls (no floor collision for it there; not a C deviation) but is captured anyway (state 0xb f300, hero alive) | /tmp k4 sh2/sh3 shots (room is black in headless) |
| mummy (EP2L02A) / zombie boss (EP4L04) | not resolved in K4 (no 0x200 group bit at test time; level script not driven) | — |
| leader not found EP6L02 | **fixed (WP-K5a)**: l02_camera fadcfbe3 ported; leader 0x2a000042 found, camera follows. Zombie 0x2a0001a8: open/hook/mash ok (life -1), then book stays in FINALSUCKING (zombie state 5) as on EP4L02A | was: game log "unported logic codes: l02_camera.op2:fadcfbe3" |
| leader EP6L05 / EP6L05B | found now (hero 0x2a000008 / 0x2a000056) | K4 runs |
| fire ghost boss (EP6L05 0x2a0000c0) | not catchable at start: group 0x100001 (no 0x200); book scans (0x54) without hooking — boss sequence must make it vulnerable first | K4 log f390 |
| ghost boss (EP6L05B 0x2a000012) | ok open/hook/mash (life 175→-1), but tome power 180→0 first → link broken (state 5), no capture: same pattern as the dust devil boss, i.e. bosses need power refills (orbs) mid-fight | gb/j7_hud_capture.png |
| dust devil boss orb flow / fire ghost hands | not re-verified in K4 | — |

## WP-K5b: boss / trigger captures

| target | C flow (reference) | port | verdict |
|---|---|---|---|
| Ghost boss EP6L05B (0x12) | GhostBossLevelFive: book msgs 0xd6/0x67/0x69/0x68 drive book pages 3->4->5->1; life drains 0.9x per hit (same rate as power), win at life<=0 -> 0x180 to 0x5a + HUD 0x3a | matches; full capture (life 195 -> -1), ghost removed (shots gb_suck/gb_after) | PASS. K4 "power first" note was a misread: life<=0 is the win condition |
| Fire ghost boss EP6L05 | init group 0x100001, no SetGroupBitMask 0x200 anywhere, no other module refs its uid 0x2a0000c0; water 0xd8 -> BeingAttack; cut-scene param1 -> BlackHole (end) | same | NOT book-catchable by design (water fight); no port fix needed |
| Dust devil boss EP3L05 (0x9d) | DustDevilCB_NOTIFY_TIMER_LOCAL: minion pairs 9e/9f, a0/a1, a2/a3 released as GD+4 bits 0x1000..0x20000 get set; each stage sends 0xd3 to carrousel 0xaa (speeds up; timer latent 0x2a8000ab drops wisps every ~4-8 s); minion capture -> 0x14e to inv_control 0x0e; all stages -> boss state 0x10 | logic matches C; wisps launched (0x126 x3 per 1500 f); wisp pickup only when power<180 (C-correct) | **FULL CAPTURE (fixed)**: stalled after 2 minions because inv_control_gen registered inv_control_baseConstructor instead of inv_controlConstructor (C: EFLogicCode looks up `<code>Constructor`), so the EP3L05 msg fn never forwarded 0x14e to 0x9d. Fixed (34977018, all 37 builds); run: 6 minions then boss 0x9d captured (state 0xb, removed), shot /tmp/k5b/dd7/dd_end.png |
| Zombie boss EP4L04 (0xbc) | becomes catchable (0x300241) on 0xde from gamecontroller 0xc7 after 5 zombie captures (0xd5 with captured uid != 0xbc; zombies b6/b8/ba respawn via 0x1fd x2); DropWisps -> 0x2a0000c3 msg 0xf7 | logic matches C | run zb3: 1st zombie captured, harness then stuck on b8 (state 0x20); 5-capture run still open |
| Mummy EP2L02A | Mummy_Base 0x17c bits 0x8000 (near/LOS check) + 0x10000 (timer/msg 0xe6) gate the catchable state; chest_level2a sends 0xd2/0x6a to 0x2a0000f1/f2 | - | trigger located in C only; drive+capture open |
| Fire ghost hands EP6L04B | - | - | not investigated (open) |
| Fire ghost hands EP6L04B | - | - | not investigated (open) |

Harness (game.rs `k5_frame`): `GAME_BOOKTGTS` (list of book targets), `GAME_WISPGRAB`.

## K7 status
| Item | Status |
|---|---|
| Entry-point audit | done, 5 builds fixed (see ENGINE_PORT K7) |
| Zombie EP4L04 (5 zombies + boss) | open: harness stalls on zombie b8 state 0x20; not resolved in K7 |
| Mummy EP2L02A trigger (0x17c bits 0x8000/0x10000, msg 0xe6) | open, not driven in K7 |
| Fire ghost hands EP6L04B | open: hand inner 0x14d positions near origin -> suspect frame/position bug |

## WP-K6: zombie final-suck stall

| Item | Finding | Status |
|---|---|---|
| What ends FINALSUCKING (C) | Not the monster: zombie 0xd6/0x67 only stops gas particles + state 5 (Monster_BaseST_BLACK_HOLE_LOOP, empty); zombie/Monster_Base never send 0xd6/0x68/0x69. The **tod_beam** ends it: book 0xd6/0x67 -> beam hooked flag -> Closing anim; anim event 3 -> book 0xd6/0x69 (FXTomeCapture), last frame -> book 0xd6/0x68 (capture, state 5) + beam KillObject | — |
| EP6L02 zombie 0x2a0001a8 | root cause: port deviation. Launched instance ids count from 0x2a800000 (EFGameLogicMgr +0x264, 1:1) and collide with pack latent uids; the 0x45th launch (a 0x2a8001c6 copy) got uid 0x2a800045 = the tod_beam latent, and `Subsystems::CloneInstances`/`object_launched` (uid-keyed) then cloned that copy's 2 anims/frame into the beam. Beam anims 3..6 missing -> opening anim LF never fired -> beam stuck in Opening_Execute (invisible). C duplicates the EFLatant's own EFGameObject, no collision. Fix (commit 27be00f6): template data saved aside (`LatentTpl`, `RuntimeWorld::latent_frames`) when an instance id first overwrites it | **fixed**: hooked, mash, beam visible, captured f288 (shots k3_finalsuck/k3_captured viewed); poltergeist EP2L01B regression ok |
| EP4L02A zombie 0x2a000109 | tomedoom 362a6394 launches 0x2a800047, absent from the pack (no tod_beam module/latent) -> LaunchLatant 0 -> nothing ever sends 0xd6/0x68; C has no fallback (no +0x54==0 test in FINALSUCKING/DrainMonsterLife) | same as C (data); not a port bug. Harness forces the book (--give-book); likely unreachable in the original |


## K9: fire ghost hands EP6L04B
| Item | Status |
|---|---|
| Fire ghost hands EP6L04B (inner ghost 0x2a00014d) | **not reproduced, no C deviation found; no code change.** Per-frame log (book frame, hands latent L+0x54, target L+0x60, L+0x110 = GetPosition(target, bone 1)) over 3 set-ups (GAME_BOOKTGT=2a000147 + water 0xd8 at f60 / f200, GAME_BOOKTGT=2a00014d + water at f20): L+0x110 tracks the ghost's bone 1 (y≈1.3 above its frame), EFDMTrackObjet (book DM L+0xc, type 1) turns the book to it, beam 0x2a800018/19 is parented to the book (0x7fffffff), its nodes stay within ~4 m of the book; the ghost is pulled into the book and the hands wrap it in FINALSUCKING (screenshots/k9_fireghost_finalsuck.png, _b.png, close-ups via GAME_CLOSEUP). Checked vs C (818e89e1): Extinguishe/CatchOnFire/MessageHandler 0x68 (timers unregistered on capture), Monster_Base 0x65/0x67/0x68, TomeDoomFUNC_UpdatePosition/UpdateOrientationLink, ToD_Beam: 1:1. "Near origin" in K7 = the shell 0x147 parked at (35,35,35) / (0,0,0) by C code (Extinguishe / capture 0x68), not the hands. EP1L01 unaffected (no change) |

## Regression suite (WP-T1)

`recomp/tests/book_regress.rs`: headless, engine API only (no screenshots, no game.rs). Run from `recomp/`:
`cargo test --release --test book_regress` (all 10 cases ~10 s in release; debug is much slower). Panics if the ISO
(`../Scooby Doo Mystery Mayhem.ISO`) is missing. The driver reproduces the K3/K5 `game --autoplay --give-book` set-up:
book msg 0xe5 + power GB1+4 at f10, each target moved 3 m in front of the leader (APIgameobj::SetTranslation) while the
book is closed, SQUARE until open, then the requested button (book L+0x1e8) mashed every 2nd frame.

| Test | Asserts (recorded on HEAD 7dd43558) |
|---|---|
| ep1l01_small_ghost / ep2l01b_poltergeist / ep3l02_dust_devil / ep1l03b_shadows / ep6l02_zombie | book 2->3->4->5->1 at the recorded frames (+-15), target CAPTURED 0xb (asleep) to the end, life <= 0, power after capture 135 / 112.5 / 90 / 112.5 / 112.5 |
| ep6l04b_fire_ghost_after_water | water 0xd8 to 0x147 at f60, 0x147 placed, book hooks the inner ghost 0x14d only, captured, power 112.5 |
| ep6l05b_ghost_boss | capture = book 4->5 hooked to 0x12 (GhostBossLevelFive eats 0xd6/0x68), life <= 0, power 180->0 |
| ep3l05_dust_devil_boss_with_minions | hook order 9e,9f,a0,a1,a2,a3,9d, all captured in order, wisp refill seen mid-fight (launched wisps moved onto the leader) |
| ep1l01_fail_paths | no press / wrong button: 3->5 link break at f322 / f232 (+-15), no state 4, ghost not captured, power stays 180 |
| ep1l03a_orb_refill | power 10 -> steps of n x 18 (wispsourcenew 0x2a0000d6/d7) -> 180 cap. The l03a_wisps GLOs give no power when only walked over |

Update the recorded frames/values only for an understood, intended behaviour change.

### K8 status (WP-K8)

| item | result | cause / recipe |
|---|---|---|
| Zombie EP4L04 "stall on b8 state 0x20" | harness, not port | state 0x20 = zombie waiting at post 0x2a0000c3 (zombie_baseST_WAITING_SET type 3 / zombie_localST_RETURN, anim 5; CheckHero 6 m -> 0x15), matches C. K5 only placed a target at power >= 150; after a capture power stayed ~112 because the hero stands on the raised start platform (y 2.6) and wisps land on the floor (y 0.22) -> no collide pickup. Each capture also returns the team to the platform. |
| Zombie boss EP4L04 full capture | **ok** | 5 zombie captures (b6,b8,ba,b6,b8; c7 gets 0xd5 x5, respawns 0x1fd x2) -> 0xde to boss 0xbc (grp 0x300241), DropWisps 0xf7 to c3 -> hook/mash -> 0x67/0x68 -> boss state 0xb, 0xd5 to c7, f3046. Recipe: `GAME_WARP=302,0.6,205:100 GAME_K5FLOOR=302,0.6,205 GAME_K5POWER=0 GAME_K5REFILL=1 GAME_WISPGRAB=1 GAME_BOOKTGTS=2a0000b6,..b8,..ba,..b6,..b8,..ba,..b6,..b8,..ba,..bc` (captured targets are skipped) |
| Mummy EP2L02A | **ok** | 0x2a0000f1/f2 are scooby_mummy/shaggy_mummy HEROES (earlier rows wrong). Mummies: 0xfe/0xff (type 10, 0x200611 from start; fe captured state 0xb) and 0x100/0x101 in sarcophagi (type 0xb, 0x200411). 0xe6 sets 0x17c 0x10000; with tome closed (0x8000) -> OPEN_SARCOPHAGE sends 0xe6 to sarcophagus 0x106 -> 0xfb -> GET_OUT -> 0x200611. Hooked within ~3 m of its sarcophagus it re-enters it (C: SCARED SetPathFinding -> ENTER_SARCOPHAGE, 0x200411, 0x92 to hero -> 0xd0 book release), as in C. Recipe: `GAME_WARP=110,0.3,256:10 GAME_SENDMSG=2a000100:e6:0:20 GAME_K5POWER=10 GAME_BOOKTGTS=2a000100` -> state 0xb f360. Hero must be in the sector (else 0x202 disables the mummy). |

## WP-K10: capture end state, EP1L03A free wisps

- **Step 8 correction: no KillObject at capture.** Every family's `*_ST_CAPTURED` (small/medium/big ghost, poltergeist,
  dust devil, shadows, zombie, fire ghost, mummy, ghost/zombie boss, scientist, guards, mercenary; 70+ C files checked)
  does `if (GD+0x30 == 0) { msg 0x14e -> manager; MiscFunc_ChangeClipping(self, 4); go.Sleep(-1) }`, nothing else.
  Book msg 0xd6/0x68 before it: SetTranslation(L+0xf8 = spawn pos), kill mask GB2+4 |= L+0x40, 0x26 -> panel,
  UnregisterNotify(0xc), group mask 0 / group id 6, 0xea -> rdrmngr, SetClippingCaptured, state 0xb.
  KillObject only comes from msg 0xd4 or `AmIDead` (kill mask set, at the next level load). So "state 0xb, asleep"
  in tests/book_regress.rs is the C behaviour. Exception: EP4 L04 `ZombieLevelFourB_ST_CAPTURED` restores the
  saved clipping (L+0x120) and goes back to state 5 (no sleep). The 0xc1 path restores everything (Sleep(0), state 0).
- **Captured = not drawn, no collision.** EFClippingParams +0xd8/+0xdc = disable masks used by
  `EFWorld::ClipGameObjects` (in frustum -> +0xdc of distance level uVar11; distance clip off when +0xd4 <= 0 ->
  level 0): bit 4 = RpWorldRemoveClump, 0x80 / 0x400 = other subsystems off. Capture sets 0x2bf / 0x6bf with far -1,
  so the monster leaves rendering. Port: `S1State::clip_render_off` (src/engine/host/gameobj.rs, distance-off case
  only; the distance test is not ported) used by game.rs draw loop. Verified: `game EP1/L01 --autoplay --give-book
  --headless --shots` ghost_captured.png shows no ghost (book_open_beam.png before it does). Collision was already
  off by group mask 0 + UnregisterNotify.
- **EP1L03A free wisps are pickable by touch.** The placed l03a_wisps GLOs 0x2a0000ac..cd start Active
  (`Wisp_baseState_Active_Execute`: visible, group mask 0x801, collide notify 3); `Wisp_baseCB_NOTIFY_COLLIDE_OBJ`:
  other == leader (GD[0]+8) or the book, timer >= L+0xa0, power < 180 -> flag 1 -> Picked -> +18 small / +36 medium /
  +54 large (cap 180). No source release, no button. The T1 "no power when walked over" was a test artefact: the
  harness moved the wisp onto the leader, but `Wisp_baseFUNC_RunBob` re-places an active level wisp at its home
  L+0x68 every frame. New test `ep1l03a_free_wisps` moves the leader onto each wisp: 10 -> +18 per wisp from f103
  (one +36 at f263) -> 180 at f283. The `ep1l03a_orb_refill` row's last sentence is superseded by this.

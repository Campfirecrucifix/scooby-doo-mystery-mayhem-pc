# P22 — opt-in raytraced GI (bevy_solari 0.19.1)

## What
- Cargo feature `solari` (`["viewer", "dep:bevy_solari"]`, never default). Code: `src/engine/gfx/rtgi.rs` (`RtgiPlugin`), hooks in `src/bin/game.rs` (`setup_cameras`, `main`) and `src/bin/viewer.rs`.
- Enable: settings option "Global Ray Tracing" = `global_ray_tracing = true` in `settings.toml` (next to controls.toml; cwd, else exe dir). Dev override `GAME_RTGI=1|0`. F9 toggles at runtime and persists the setting. The plugin is only registered when enabled at startup (SolariPlugins switches the opaque renderer to deferred), so RTGI-off == raster path (verified pixel-identical: C:/Temp/p22/rtgi0_EP1L01.png vs raster_EP1L01.png).
- Device: Bevy's default `WgpuSettingsPriority::Functionality` already requests every adapter feature incl. `SolariPlugins::required_wgpu_features()`; forcing them would panic on adapters without ray query, so instead `RtgiPlugin::finish` checks the RenderDevice and logs a warning + keeps raster when missing.
- Camera: `SolariLighting`, `Msaa::Off`, `CameraMainTextureUsages` + STORAGE_BINDING, Hdr, neutral `Tonemapping::None`, no bloom; the overlay/HUD cameras sharing the target get Hdr + Msaa::Off.
- Meshes: world-layer (layer 0) Opaque/Mask StandardMaterial meshes get `RaytracingMesh3d` with a stripped copy (POSITION/NORMAL/UV_0/TANGENT, U32 indices; smooth normals / mikktspace tangents generated when missing, fallback constant tangent). Raster `Mesh3d` keeps the prelit ATTRIBUTE_COLOR, so the G-buffer albedo = texture x prelit colour. Blended/additive, HUD/overlay layers stay raster only.
- Art style (R2 option b): RT materials become matte (roughness 1, metallic 0, reflectance 0, unlit off; originals restored on F9-off); one soft sun (13000 lux, `GAME_RTGI_SUN` override); fog/alpha/unlit layers untouched. Option (a) (prelit base x clamped RT ratio) needs a custom pass over Solari's lighting texture — not done.

## Validation
- `cargo build --release --features game` and `--features game,solari` OK; `cargo test --profile wfast --all-features` green (266 passed) in R2.
- EP1/L01 frame 200 (R2): raster mean luma 58.7; RT 8000 lux 42.1, 12000 lux 51.4 (-12%).
- KNOWN BUG: later RTGI runs (same binary/config) produce fully black frames incl. HUD (C:/Temp/p22/rtgi1_EP1L01.png, sbs_EP1L01.png), no wgpu warnings. Not root-caused (suspects: HDR main texture + STORAGE_BINDING with 3 cameras sharing an Rgba8 image target in headless; Solari temporal history NaN). RTGI stays opt-in/default off.

## Next
- Root-cause black output (try windowed, single camera, reset each frame), then hybrid ratio pass (clamp 0.6..1.3), PS2 options-menu entry for the setting, BLAS refresh for CPU-skinned meshes, DLSS-RR denoise.

## R4 — black frames FIXED
- Root cause (bisected with `GAME_RTGI_DBG=nosolari,nomesh,nomat` + camera switches): any of Hdr / Msaa::Off / STORAGE_BINDING main-texture usage on the main camera alone blacked the image. The overlay/HUD cameras share the same render target with `ClearColorConfig::None`; when their main texture differs from the RT camera's they get their own empty main texture and overwrite the target with black. `rtgi::companion()` now also sets `CameraMainTextureUsages` + STORAGE_BINDING (it already had Hdr + Msaa::Off). Solari, meshes and materials were fine.
- Calibrated: sun 8000 lux, emissive lift = 0.01 x base colour (keeps the shadowed interiors from going black; the emissive also feeds the RT GI).
- EP1/L01 frame 200: raster mean luma 58.7 -> RT 67.1 (+14%); EP1/L02: 30.2 -> 33.7 (+12%). Slightly above the ~10% target. RTGI off is pixel-identical to raster.
- Shots: C:/Temp/p22/sbs_EP1L01.png, sbs_EP1L02.png (left raster, right RT), raster_*.png, rtgi1_*.png, rtgi0_EP1L01.png.
- Gate (`cargo test --profile wfast`, default features): 255 + 4 passed, 0 failed.
- Look: palette kept (albedo = texture x prelit colour), adds sun shadows/bounce; sunlit open areas read slightly brighter and flatter than the PS2 prelit look. Hybrid ratio pass (option a) still TODO.

## R5
- Sun calibrated to 5000 lux (emissive lift 0.01). Mean luma RT vs raster, frame 200: EP1/L01 -4.9%, EP1/L02 -0.3%, EP2/L01A +1.1% (all within ±8%). Side-by-sides (left raster, right RT): C:/Temp/p22/sbs_EP1L01.png, sbs_EP1L02.png, sbs_EP2L01A.png.
- Fixed: under Solari the default opaque method is deferred, and the HUD camera has no deferred prepass, so opaque HUD clumps vanished. HUD/overlay StandardMaterials are now set to Forward (compass is back).
- Open: the HUD character portrait (EFHud3DWindow) still renders black in RTGI mode. It probably shares a material with a world mesh that was made matte/lit, and the HUD layer has no light. Tried giving HUD entities their own forward copy of the material: the whole HUD disappeared, so reverted. Next: find the portrait's material path in the adapter.
- Not done: entry in the PS2 options menu (setting is in settings.toml + F9), and the hybrid prelit × clamp(RT, 0.6..1.3) pass.
- Gates: `cargo test --profile wfast` 255+4 passed; `--all-features` (with solari) 266+4 passed.

## R6
- HUD portrait black FIXED: the adapter caches StandardMaterials by DrawMaterialKey, so the HUD portrait clump shared a material with world meshes, and the in-place matte (lit) conversion darkened it on the unlit HUD layer. World RT meshes now get a private matte copy (`RtgiState.clones/orig`), swapped in by `attach_rt_meshes` in `Last` (the adapter re-inserts MeshMaterial3d every frame). HUD/overlay keep the original material (forced Forward). The HUD now matches raster (C:/Temp/p22/sbs_EP2L01A.png).
- Mean luma vs raster: EP1/L01 -4.9%, EP1/L02 -0.3%, EP2/L01A +1.9%. Shots: C:/Temp/p22/sbs_EP1L01.png, sbs_EP1L02.png, sbs_EP2L01A.png.
- Options-menu entry NOT added. The options screen is decompiled PS2 game logic (src/modules/panel/panel_base_options.rs: OPTIONS_SET copies fixed GBP option slots and the HUD windows come from the ISO layout), so a new row means changing game logic/data, which this task forbids. The Bevy build also has no bevy_ui/bevy_text for an overlay. The setting stays: settings.toml `global_ray_tracing` + F9 (persists) + GAME_RTGI. Next: an overlay drawn through the engine HUD font (EFHud text windows) as a host-side layer.
- Hybrid pass: not started.
- Gates: `cargo test --profile wfast` 255+4 passed; `--all-features` 266+4 passed.

## R7 — hybrid look
- `src/engine/gfx/rtgi_hybrid.rs` (`RtgiHybridPlugin`, marker `RtgiHybrid` on the Solari camera): a fullscreen pass in Core3d MainPass, after `main_opaque_pass_3d` and before `main_transparent_pass_3d`. For every pixel with G-buffer data it outputs `A * clamp(lum(RT)/lum(A)/REF, 0.6, 1.3)`, where A is the G-buffer base colour (texture x baked prelit vertex colour = the original raster pixel) and RT is the Solari result. Palette and baked lighting composition stay the original's; RT only modulates intensity (contact shadows, bounce). Sky, forward, blended and HUD pixels pass through. REF = 1.0 (`GAME_RTGI_REF` override); `GAME_RTGI_DBG=nohybrid` shows the plain Solari image.
- Mean luma vs raster: EP1/L01 -6.5%, EP1/L02 -3.7%, EP2/L01A -1.3%. Side-by-sides: C:/Temp/p22/sbs_hybrid_EP1L01.png, sbs_hybrid_EP1L02.png, sbs_hybrid_EP2L01A.png (left raster, right hybrid RT).
- Limitation: fog is not re-applied over the modulated pixels (A is pre-fog).
- Not done: options-panel overlay toggle (time).
- Gates: `cargo test --profile wfast` 255+4 passed; `--all-features` 266+4 passed.

## R8
- Fog: the hybrid pass now re-applies the camera's DistanceFog (`RtgiHybrid.fog`, synced from DistanceFog + Projection by `sync_fog`; linear/exp/exp2 like bevy_pbr fog.wgsl) to the modulated pixels. Distance comes from the prepass depth (reverse-Z infinite perspective: z = near/depth, scaled by the per-pixel ray length). Not visually verified on a fogged level: none of the 3 test levels showed visible fog at frame 200 (luma unchanged: EP1/L01 -6.5%, EP1/L02 -3.6%, EP2/L01A -1.3%).
- Characters: CPU-skinned meshes (adapter marks them NoFrustumCulling) are excluded from the BLAS, so there is no stale-pose ghost shadow. They are still in the G-buffer, so the hybrid pass modulates them from RT light (they read slightly darker than raster in shadowed spots).
- Options-panel overlay row: NOT done (time). Plan: a host-side EFHud text window drawn via `draw_hud`'s HudDrawCmd path while panel_base is in its OPTIONS state, read-only from the game-object state (no logic change).
- Shots: C:/Temp/p22/sbs_hybrid_EP1L01.png, sbs_hybrid_EP1L02.png, sbs_hybrid_EP2L01A.png.
- Gates: `cargo test --profile wfast` 255+4 passed; `--all-features` lib 4 passed + `--test all` 266 passed (rerun separately after the combined run hit the 295 s command timeout).

## Round 9
- Options panel: port-side row "GLOBAL RAY TRACING ON/OFF (R)" drawn under the original options panel
  (pause -> Game Options) with the game's HUD font glyphs via the HUD draw-command path
  (src/bin/game.rs `rtgi_options_row`). R toggles (same path as F9), persisted to settings.toml;
  if Solari wasn't loaded at start (setting off) it edits settings.toml and shows "(RESTART)".
  Hidden on GPUs without ray query; absent in non-solari builds. Open-detection: port-side static
  `OPTIONS_OPEN` (panel_base_options.rs, set in ST_OPTIONS_SET, cleared on the return to pause);
  no game logic changed. Shot: C:/Temp/p22/options_rtgi_row.png.
- Known: the row's glyphs come out larger/greyer than the panel rows (copies the last font glyph's
  font/colours); the row also shows on the Sound Options sub-page.
- Not done (time): skinned-mesh pass-through in the hybrid pass; fogged-level side-by-side.
- Gates: default wfast 255+4 ok; --all-features --test all 266 ok (first run failed once, rerun ok, flaky).

## Round 10
- Options row: uses the panel's own row style (font 0, unselected orange label, green value, glyph scale from the rows' drawn height), placed one row step (36) below the last row, centred. Shown only on Game Options (OPTIONS_OPEN cleared on Sound page, set on return). Screens: C:/Temp/p22/opts3.png, sound.png.
- Skinned (CPU-skinned, NoFrustumCulling) meshes: material forced Forward -> no G-buffer -> hybrid pass leaves them as raster. sbs: C:/Temp/p22/sbs_skin_EP1L02.png (match).
- Fog side-by-side: not done (no fogged level identified in time).
- Flaky all-features test: not reproduced (--test all 266 passed twice, lib 4 passed).
- Gates: default 255+4 ok; all-features 266+4 ok.

## R11 — fog verification
- Probed fog state (temp GAME_FOGLOG in ApplyFog) on 20 loadable levels (EP1-4, EP6, 150 frames): no level enables fog at start; fog is only toggled by camera floor events (camera local builds 0e2da84f/1ac1b1b0/2500ac27/5257bb9b..., linear, plane 0.1-5).
- Finding: `bevy_adapter::ApplyFog` / `fog_to_distance_fog` has NO caller, so the port never puts DistanceFog on the camera: fog is rendered neither in raster nor in RTGI. A forced fog (temp GAME_FOGFORCE) never reached the camera either. Same image RT on/off: C:/Temp/p22/sbs_fog_EP1L02.png (means 35.2/30.8/21.2 vs 33.6/29.2/20.0).
- So the hybrid cannot lose or double fog today; its fog re-apply (sync_fog reads DistanceFog) activates automatically once someone wires ApplyFog. No code change (temp probes reverted), gates not rerun.
## R12 — fog wired
- `src/bin/game.rs` (render_frame_inner, camera-0 sync): every frame `g.gmgr.SetRenderState(c)` (EFGraphicsMgr_SP::SetRenderState: fog enable/colour/type/density from the camera) then `ApplyFog(world, GameCam, &gmgr.fog, &c.rw)` -> DistanceFog inserted/removed on the game camera. Linear = camera fog plane (SetFogDistance) to the far plane; colour = the packed SetFogColor word (incl. its no-x255 quirk); exp/exp2 use the density. No game logic changed.
- Fogged levels (probe, reverted): EP4/L01 (camera build fae3f0bb) fog on from frame 2: linear, plane 1, far 40, colour 0x152231; EP4/L04 (78a0064b): plane 1, far 125, colour 0x7d6028. Others toggle fog only from camera floor events (e.g. 49127a51 room 2).
- Side-by-side EP4/L01 f200, RT off | RT on: C:/Temp/p22/sbs_fog_real.png. Fog visible in both (distance fades to dark blue).
- Double fog: none. The hybrid output is A*ratio with A = pre-fog G-buffer albedo, fogged once by the hybrid pass. Far-region mean RGB: raster 18.7/51.5/60.8 vs hybrid 18.1/52.4/62.0. Double fog would pull it toward the fog colour (21/34/49). Near ground: 22/77/61 vs 26/87/69 (the RT modulation, as in R7).
- Gates: `cargo test --profile wfast` 255+4 ok; `--all-features` --test all 266 + --lib 4 ok.

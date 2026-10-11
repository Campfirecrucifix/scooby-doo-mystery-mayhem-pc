# WP-G1 — aspect ratios + `--rt`

## Aspect (Hor+)
- `game [LEVEL] --aspect 4:3|16:9|21:9|32:9|auto` (also any `W:H` or a float), `--res WxH` (window or headless target size).
- The PS2 engine has no in-game widescreen option (only the demo-launcher `gaspect` parameter in main.cpp, unused by the engine), so Hor+ is new behaviour:
  - game camera: vertical FOV from the RW view window (`2*atan(vw.y)`) is kept; Bevy derives the aspect from the camera viewport, so the view widens horizontally.
  - engine culling: `engine::camera::FRUSTUM_HSCALE` (default 1.0 = 1:1) widens the x extent of `EFFrustum::SetFromCameraParams` in `EFCamera::refresh_frustum` / `UpdateGuardBand` by `vw.y*aspect/vw.x` (game.rs `apply_aspect`; cameras marked frustum-dirty on change). Note this also widens `IsSphereInFrustrum`/`ProjectOntoFrustum` for scripts and particle emitter culling.
  - viewports: fixed aspects are fitted (letter/pillarboxed) into the window/target; `auto` = whole window (resizable).
  - HUD: EFHud3DWindow camera + FMV camera are a centered 4:3 viewport (not stretched). EFSpriteMgr overlay spans the game view in unscaled PS2 units (centered 4:3), and screen-wide sprites (x 0..640: fades, bars) are widened to the full view.
- Default: windowed = auto Hor+; headless without `--aspect/--res` = unchanged legacy 1280x896 full-target output (tests/screenshots unaffected).
- Screenshots: `screenshots/g1_4x3.png g1_16x9.png g1_21x9.png g1_32x9.png g1_def.png (legacy headless) g1_rt.png (16:9 --rt)`.

## `--rt` (cargo feature `rt`)
Bevy Solari (0.19.1) was evaluated and is NOT used, because:
- `bevy_solari::scene::blas::is_mesh_raytracing_compatible` requires the vertex attributes to be exactly `[POSITION, NORMAL, UV_0, TANGENT]` + `Indices::U32` + TriangleList. All port meshes carry `ATTRIBUTE_COLOR` (RW prelit / CPU-lit vertex colours = the game's entire baked lighting); stripping it removes the lighting/colour.
- Solari lighting shades the deferred G-buffer of lit `StandardMaterial`s; the port draws `unlit` (prelit) materials, many alpha-blended (forward only), so they would not be lit/would be skipped.
- The levels have no real-time light sources (lighting is baked), so a Solari scene would need invented lights → not the original look.
Closest working option implemented (`rt_camera` in game.rs): HDR + Bloom + FXAA + contrast-adaptive sharpening on the game camera (overlay/HUD cams made HDR too so they composite). Without `--rt` (or without the feature) nothing changes.

## Limitations
- Cut-scene letterbox bars are 3D objects of finite width: they cover the view up to ~2:1; at 21:9/32:9 the outer sides are not covered.
- World-anchored sprites (layer 0) use the engine projection and are unaffected; HUD elements are centered 4:3, not edge-anchored.

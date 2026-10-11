# Roadmap

## Current goals
- Full scripted play-through of every episode (EP1–EP6) on the 1:1 port (`--route`).
- Test suite green (`cargo test --release --all-features`).
- PC features: widescreen Hor+ (`--aspect`, branch `gfx`), keyboard/mouse/controller with rebinding (`controls.toml`, branch `input`).

## Future goal: "Remastered" render mode with ray tracing (optional, opt-in)
The 1:1 port draws the original PS2 look: lighting is baked into vertex colours, materials are unlit/prelit
(many alpha-blended), and there are almost no runtime lights. Bevy's ray tracer (Solari) cannot use that
(see GFX_G1.md). A remaster layer on top of the port — not a change to the 1:1 logic — would add:

1. **Materials:** convert prelit materials to lit PBR (`StandardMaterial`), derive normals/tangents, keep the
   original textures; alpha-blended surfaces handled separately (forward pass).
2. **Lights:** real lights from the game's own light data (`src/engine/light.rs`, `LightObject`/SetLogicFlags 0x100),
   plus a per-level sun/moon/ambient; tune per level.
3. **Baked colours:** remove from lighting, or reuse as an ambient-occlusion/tint layer.
4. **Ray tracing:** enable Solari (`RaytracingMesh3d`, Solari-compatible mesh attributes) behind `--remaster --rt`;
   fallback (`--remaster` without RT) uses standard PBR + shadows + SSAO.
5. **Optional extras:** texture upscaling, higher-poly replacement assets via a mod folder.

Constraints: default build and `--rt` post-process mode stay unchanged; game logic stays 1:1; remaster is a
rendering-only layer. Expect per-level art tuning; this is an art pipeline project, not a toggle.
Hardware target: RTX-class GPU for RT (developed on RTX 4080 SUPER).

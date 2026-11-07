//! WP-S6: custom Bevy materials for the PS2 GS effects StandardMaterial cannot express.
//! - [`ShadowSubMaterial`]: projected shadow, GS ALPHA_1 0xa1 = Cd - Cs (ReverseSubtract blend).
//! - [`DistortMaterial`]: EFFBEffect_FrameDistortion APPROXIMATION: redraw of the effect object's
//!   meshes sampling Bevy's opaque-pass copy (view_transmission_texture, the GrabRaster stand-in)
//!   at the fragment's screen position + an offset = distortion matrix * view-space normal. The
//!   original's VU1 micro code (UV generation from the matrix) is not in the decompiled source.
use bevy::asset::{uuid_handle, Asset, Handle};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::{Shader, ShaderRef};

const SHADOW_SHADER: Handle<Shader> = uuid_handle!("6a1f0c52-3c0e-4c55-9f77-1c0b5d0a5a01");
const DISTORT_SHADER: Handle<Shader> = uuid_handle!("6a1f0c52-3c0e-4c55-9f77-1c0b5d0a5a02");
const DECODE_SHADER: Handle<Shader> = uuid_handle!("6a1f0c52-3c0e-4c55-9f77-1c0b5d0a5a03");
const HUD_DEPTH_SHADER: Handle<Shader> = uuid_handle!("6a1f0c52-3c0e-4c55-9f77-1c0b5d0a5a04");
const STENCIL_READ_SHADER: Handle<Shader> = uuid_handle!("6a1f0c52-3c0e-4c55-9f77-1c0b5d0a5a05");
const MASK_OCCLUDER_SHADER: Handle<Shader> = uuid_handle!("6a1f0c52-3c0e-4c55-9f77-1c0b5d0a5a06");
const OCC_SQUARE_SHADER: Handle<Shader> = uuid_handle!("6a1f0c52-3c0e-4c55-9f77-1c0b5d0a5a07");
const OCC_READ_SHADER: Handle<Shader> = uuid_handle!("6a1f0c52-3c0e-4c55-9f77-1c0b5d0a5a08");

/// P26 R26: the HUD clumps (EFHud3DWindow EF3dObjs on the HUD camera) are drawn into the PS2's
/// 16-bit Z buffer (ENGINE.INI has no DeepZBuffer -> zBufferDepth 16): depths closer than one Z step
/// (255.9 / 65535 = 0.0039 for the HUD camera's 0.1..256 range) tie, and GEQUAL lets the later draw
/// win. The compass dial ping ring sits 0.0016-0.0033 in front of the disk (same mesh), within one Z
/// step, and the original never shows it over the disk. hud_depth.wgsl snaps the
/// fragment depth to that grid; HUD clump materials only (world and Im2D HUD quads untouched).
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug, Default)]
pub struct HudDepth {}
impl bevy::pbr::MaterialExtension for HudDepth {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(HUD_DEPTH_SHADER)
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    // EFHudMgr_SP::Update keeps ZWRITE on for every HUD draw (RwRenderStateSet 8 = 1), blended clumps
    // included; Bevy's Blend pipelines drop depth writes.
    fn specialize(
        _p: &bevy::pbr::MaterialExtensionPipeline,
        d: &mut RenderPipelineDescriptor,
        _l: &MeshVertexBufferLayoutRef,
        _k: bevy::pbr::MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(ds) = d.depth_stencil.as_mut() {
            ds.depth_write_enabled = Some(true);
            // ties: earlier draw wins. The port's mat-0 strip of the dial clump 0x718 draws the disk
            // (20 tris) before the ring (80 tris); PCSX2 never shows the ring, i.e. the disk wins the
            // tie on the PS2 (draw order there unverified, see logs/P26.md R26).
            ds.depth_compare = Some(bevy::render::render_resource::CompareFunction::Greater);
        }
        Ok(())
    }
}
pub type HudMaterial = bevy::pbr::ExtendedMaterial<StandardMaterial, HudDepth>;

/// P40 R3 (B#9): GS ALPHA 0xa1 = (A Cd - B Cs) * C FIX + D 0, FIX = 0x80 = 1.0: Cd - Cs, clamped at 0
/// (COLCLAMP), source alpha ignored. PLEmitter::Render picks it for particle types with flag 0x80
/// (EP2/L02C "broue", EP6 fireball smoke 0x81). The port approximated it with a Multiply blend by
/// (1 - Cs) (= Cd - Cs*Cd: too light, and the inverted vertex colour times the texel was not 1 - Cs).
/// Same StandardMaterial inputs (unlit texel * vertex colour); only the colour blend becomes
/// ReverseSubtract (dst*1 - src*1); the frame buffer holds display values (display-space mode), so
/// the subtraction is in the GS's space. Alpha channel (mask image) left as is.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug, Default)]
pub struct GsSubtract {}
pub fn gs_subtract_blend() -> BlendState {
    BlendState {
        color: BlendComponent { src_factor: BlendFactor::One, dst_factor: BlendFactor::One, operation: BlendOperation::ReverseSubtract },
        alpha: BlendComponent { src_factor: BlendFactor::Zero, dst_factor: BlendFactor::One, operation: BlendOperation::Add },
    }
}
impl bevy::pbr::MaterialExtension for GsSubtract {
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn specialize(
        _p: &bevy::pbr::MaterialExtensionPipeline,
        d: &mut RenderPipelineDescriptor,
        _l: &MeshVertexBufferLayoutRef,
        _k: bevy::pbr::MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(f) = d.fragment.as_mut() {
            for t in f.targets.iter_mut().flatten() {
                t.blend = Some(gs_subtract_blend());
            }
        }
        Ok(())
    }
}
pub type GsSubtractMaterial = bevy::pbr::ExtendedMaterial<StandardMaterial, GsSubtract>;

/// P26 R28: PS2 "stencil" = frame-buffer destination alpha. EFGraphicsMgr::RenderCamera clears it
/// (ClearStencil) and masks alpha writes (EnableStencilWrite(0)); only EF3dObjs whose owner has
/// logic flag 0x8000 open the alpha byte (EnableStencilWrite(1), FBMSK alpha) and leave their alpha
/// there. Sprite groups with flag 0x10 draw with EnableStencilRead(1) = GS TEST DATE on, DATM 0:
/// a pixel is written only where the frame alpha bit 7 is 0 (e.g. the collectable glow never covers
/// its snack box). Port: the 0x8000 objects are also drawn by a mask camera into an Rgba8 image
/// (cleared to alpha 0, written alpha = the material alpha, PS2 0x80 = 1.0); stencil_read.wgsl
/// discards where that alpha has bit 7 set.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug, Default)]
pub struct StencilRead {
    #[texture(100)]
    pub mask: Handle<Image>,
}
impl bevy::pbr::MaterialExtension for StencilRead {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(STENCIL_READ_SHADER)
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
}
pub type StencilReadMaterial = bevy::pbr::ExtendedMaterial<StandardMaterial, StencilRead>;

/// P26 R29: the PS2 alpha write of an EnableStencilWrite object happens only where its pixels pass
/// the Z test. The stencil-mask camera therefore also draws the opaque world with this material
/// (depth written, colour/alpha 0): a stencil object behind a wall leaves no mask there.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug, Default)]
pub struct MaskOccluderExt {}
impl bevy::pbr::MaterialExtension for MaskOccluderExt {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(MASK_OCCLUDER_SHADER)
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
}
/// base = the occluding entity's own StandardMaterial (alpha-tested cut-outs stay holes)
pub type MaskOccluder = bevy::pbr::ExtendedMaterial<StandardMaterial, MaskOccluderExt>;
/// P26 R21: the end of a display-space frame (bevy_adapter::display_space). The cameras render
/// display values into Bevy's sRGB main texture as if they were linear, so every blend happens on
/// the display value like the GS frame buffer; this pass on the LAST camera of the frame (the
/// order-2 HUD camera) decodes sRGB -> linear once, so the final sRGB target write re-encodes the
/// original display value.
#[derive(Component, bevy::render::extract_component::ExtractComponent, Clone, Copy, bevy::render::render_resource::ShaderType, Default)]
pub struct DisplayDecode {
    /// unused (uniform padding; WebGL2 structs must be 16 bytes)
    pub pad: Vec4,
}
impl bevy::core_pipeline::fullscreen_material::FullscreenMaterial for DisplayDecode {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(DECODE_SHADER)
    }
}

/// occurrences one [`ShadowSubMaterial`] holds (EFShadowMgr::Begin casts at most 8 shadows a frame)
pub const SHADOW_MAX: usize = 8;
/// atlas slot size (largest shadow raster, 0x100); slots are laid out 4 x 2
pub const SHADOW_SLOT: u32 = 256;

/// P26 R4: ONE material shared by every shadow patch of the frame. GS: each patch is drawn with
/// ALPHA_1 0xa1 = Cd - Cs clamped, so where two patches overlap the frame buffer is darkened twice.
/// The port reads Cd from the opaque-pass copy (sRGB subtraction, P26 B2), so a patch cannot see the
/// previous patch's result; instead every patch fragment evaluates ALL of the frame's occurrences at
/// its world position and subtracts the sum: max(max(Cd - a, 0) - b, 0) == max(Cd - (a + b), 0), so
/// overlapping draws write the same, stacked value.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct ShadowSubMaterial {
    /// per occurrence i, 6 vec4: [6i] right.xyz + view window, [6i+1] up.xyz + parallel (1/0),
    /// [6i+2] at.xyz + raster size / slot size, [6i+3] projector pos.xyz + colour (Cs = mask * colour),
    /// [6i+4] near plane, [6i+5] far plane
    #[uniform(0)]
    pub occ: [Vec4; 6 * SHADOW_MAX],
    /// x = occurrence count
    #[uniform(3)]
    pub count: Vec4,
    /// the shadow rasters, slot i at ((i % 4) * 256, (i / 4) * 256) of a 1024 x 512 atlas
    #[texture(1)]
    #[sampler(2)]
    pub mask: Handle<Image>,
}

impl Material for ShadowSubMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(SHADOW_SHADER)
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }
    /// P26 B2: Cd is read from the opaque-pass copy (shadow.wgsl subtracts in sRGB space)
    fn reads_view_transmission_texture(&self) -> bool {
        true
    }
    fn depth_bias(&self) -> f32 {
        60.0
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn specialize(_p: &MaterialPipeline, d: &mut RenderPipelineDescriptor, _l: &MeshVertexBufferLayoutRef, _k: MaterialPipelineKey<Self>) -> Result<(), SpecializedMeshPipelineError> {
        d.primitive.cull_mode = None;
        if let Some(f) = d.fragment.as_mut() {
            for t in f.targets.iter_mut().flatten() {
                t.blend = None;
            }
        }
        if let Some(ds) = d.depth_stencil.as_mut() {
            ds.depth_write_enabled = Some(false);
        }
        Ok(())
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct DistortMaterial {
    /// the effect's uv matrix (EFFBEffect_FrameDistortion params +0xc RwMatrix, identity if unset)
    #[uniform(0)]
    pub uv_matrix: Mat4,
    /// x = screen offset scale
    #[uniform(1)]
    pub strength: Vec4,
}

impl Material for DistortMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(DISTORT_SHADER)
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }
    fn reads_view_transmission_texture(&self) -> bool {
        true
    }
    fn enable_shadows() -> bool {
        false
    }
}

/// P33 R3: EFSpriteGroup_SP::RenderSpriteNewOcc (0x002780a0) occlusion. GS: (1) the small sample
/// square around the sprite centre is drawn into the frame alpha (FRAME mask 0xffffff, ZMSK) with
/// alpha 0 (ZTST ALWAYS), (2) again with alpha 0x80 and ZTST GEQUAL at the sprite's own Z, so the
/// alpha is 0x80 only where the sprite centre is not hidden by the world, (3) that alpha is copied
/// and spread over the sprite rect, (4) the sprite is drawn with ALPHA 0x98 Cs*Ad / 0x58 Cs*Ad+Cd /
/// 0x91 (Cd-Cs)*Ad: its colour scales with the visible part of the sample square. Port: (1)+(2) =
/// [`OccSquare`] quads on the stencil-mask camera (depth-tested against the MaskOccluder world,
/// green channel only: the stencil alpha stays untouched), (3)+(4) = [`OccRead`] on the overlay
/// sprite: colour and alpha * the mean green over the sample square.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug, Default)]
pub struct OccSquareExt {}
impl bevy::pbr::MaterialExtension for OccSquareExt {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(OCC_SQUARE_SHADER)
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn specialize(
        _p: &bevy::pbr::MaterialExtensionPipeline,
        d: &mut RenderPipelineDescriptor,
        _l: &MeshVertexBufferLayoutRef,
        _k: bevy::pbr::MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        d.primitive.cull_mode = None;
        if let Some(f) = d.fragment.as_mut() {
            for t in f.targets.iter_mut().flatten() {
                t.blend = None;
                t.write_mask = bevy::render::render_resource::ColorWrites::GREEN;
            }
        }
        if let Some(ds) = d.depth_stencil.as_mut() {
            ds.depth_write_enabled = Some(false);
        }
        Ok(())
    }
}
/// base: unlit, AlphaMode::Blend (transparent phase = after the opaque MaskOccluder world)
pub type OccSquare = bevy::pbr::ExtendedMaterial<StandardMaterial, OccSquareExt>;
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug, Default)]
pub struct OccRead {
    #[texture(100)]
    pub mask: Handle<Image>,
    /// sample square in screen uv: x0, y0, x1, y1
    #[uniform(101)]
    pub rect: Vec4,
}
impl bevy::pbr::MaterialExtension for OccRead {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(OCC_READ_SHADER)
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
}
pub type OccReadMaterial = bevy::pbr::ExtendedMaterial<StandardMaterial, OccRead>;

pub struct GameFxPlugin;
impl Plugin for GameFxPlugin {
    fn build(&self, app: &mut App) {
        let mut shaders = app.world_mut().resource_mut::<Assets<Shader>>();
        let _ = shaders.insert(SHADOW_SHADER.id(), Shader::from_wgsl(include_str!("shadow.wgsl"), "game_fx/shadow.wgsl"));
        let _ = shaders.insert(DISTORT_SHADER.id(), Shader::from_wgsl(include_str!("distort.wgsl"), "game_fx/distort.wgsl"));
        let _ = shaders.insert(DECODE_SHADER.id(), Shader::from_wgsl(include_str!("decode.wgsl"), "game_fx/decode.wgsl"));
        let _ = shaders.insert(HUD_DEPTH_SHADER.id(), Shader::from_wgsl(include_str!("hud_depth.wgsl"), "game_fx/hud_depth.wgsl"));
        let _ = shaders.insert(STENCIL_READ_SHADER.id(), Shader::from_wgsl(include_str!("stencil_read.wgsl"), "game_fx/stencil_read.wgsl"));
        let _ = shaders.insert(MASK_OCCLUDER_SHADER.id(), Shader::from_wgsl(include_str!("mask_occluder.wgsl"), "game_fx/mask_occluder.wgsl"));
        let _ = shaders.insert(OCC_SQUARE_SHADER.id(), Shader::from_wgsl(include_str!("occ_square.wgsl"), "game_fx/occ_square.wgsl"));
        let _ = shaders.insert(OCC_READ_SHADER.id(), Shader::from_wgsl(include_str!("occ_read.wgsl"), "game_fx/occ_read.wgsl"));
        app.add_plugins((MaterialPlugin::<OccSquare>::default(), MaterialPlugin::<OccReadMaterial>::default()));
        app.add_plugins((
            MaterialPlugin::<ShadowSubMaterial>::default(),
            MaterialPlugin::<DistortMaterial>::default(),
            MaterialPlugin::<HudMaterial>::default(),
            MaterialPlugin::<GsSubtractMaterial>::default(),
            MaterialPlugin::<StencilReadMaterial>::default(),
            MaterialPlugin::<MaskOccluder>::default(),
            bevy::core_pipeline::fullscreen_material::FullscreenMaterialPlugin::<DisplayDecode>::default(),
        ));
    }
}

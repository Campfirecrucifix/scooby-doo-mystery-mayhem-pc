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

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct ShadowSubMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
    #[texture(1)]
    #[sampler(2)]
    pub mask: Handle<Image>,
}

impl Material for ShadowSubMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(SHADOW_SHADER)
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
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
                t.blend = Some(BlendState {
                    color: BlendComponent { src_factor: BlendFactor::One, dst_factor: BlendFactor::One, operation: BlendOperation::ReverseSubtract },
                    alpha: BlendComponent { src_factor: BlendFactor::Zero, dst_factor: BlendFactor::One, operation: BlendOperation::Add },
                });
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

pub struct GameFxPlugin;
impl Plugin for GameFxPlugin {
    fn build(&self, app: &mut App) {
        let mut shaders = app.world_mut().resource_mut::<Assets<Shader>>();
        let _ = shaders.insert(SHADOW_SHADER.id(), Shader::from_wgsl(include_str!("shadow.wgsl"), "game_fx/shadow.wgsl"));
        let _ = shaders.insert(DISTORT_SHADER.id(), Shader::from_wgsl(include_str!("distort.wgsl"), "game_fx/distort.wgsl"));
        app.add_plugins((MaterialPlugin::<ShadowSubMaterial>::default(), MaterialPlugin::<DistortMaterial>::default()));
    }
}

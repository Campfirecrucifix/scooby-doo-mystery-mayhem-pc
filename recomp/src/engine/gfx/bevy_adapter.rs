//! Bevy adapter (feature `bevy_render`): RW plain-data assets -> Bevy `Mesh` / `Image` /
//! `StandardMaterial`, and entity spawning per EF3dObj (frame tree as a Transform hierarchy,
//! one mesh entity per atomic material split, `SkinnedMesh` bound to the HAnim node frames) and
//! for the level's static world (EFStatic3dObj sectors). Not a port: the replacement of the PS2
//! RenderWare pipelines (RxPipelineExecute / RpWorldRender).
//!
//! Conventions: RW matrices are used as-is (column-major, `Mat4::from_cols_array`); no
//! handedness flip is applied (open question for the coordinator's camera). Prelit colours become
//! vertex colours; material colour is `base_color`; world sectors are unlit (lighting is baked
//! in the prelit colours on PS2), clumps are lit.
#![allow(non_snake_case)]

use super::hierarchy::{SkinInverseMatrix, IDENTITY};
use super::render::{BlendMode, EF3dObj_SP};
use super::EFStatic3dObj;
use crate::engine::rw::clump::RpClump;
use crate::engine::rw::geometry::{RpGeometry, RpMaterial};
use crate::engine::rw::image::{rw_name_eq, RwTexDictionary, RwTexture};
use crate::engine::rw::world::RpWorld;
use bevy_app::{App, Plugin};
use bevy_asset::{AssetApp, Assets, Handle, RenderAssetUsages};
use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_image::Image;
use bevy_material::AlphaMode;
use bevy_math::Mat4;
use bevy_mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy_mesh::{Indices, Mesh, Mesh3d, PrimitiveTopology, VertexAttributeValues};
use bevy_pbr::{MeshMaterial3d, StandardMaterial};
use bevy_transform::components::{GlobalTransform, Transform};
use wgpu_types::{Extent3d, TextureDimension, TextureFormat};

/// Registers the asset types the adapter needs when the full render stack is absent
/// (headless tests); harmless next to `PbrPlugin`.
pub struct GfxAdapterPlugin;
impl Plugin for GfxAdapterPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_asset::AssetPlugin>() {
            app.add_plugins(bevy_asset::AssetPlugin::default());
        }
        // init_asset re-inserts Assets<T>: only register what DefaultPlugins did not (re-registering
        // Image would drop the tonemapping LUTs ImagePlugin users already added)
        if !app.world().contains_resource::<Assets<Mesh>>() {
            app.init_asset::<Mesh>();
        }
        if !app.world().contains_resource::<Assets<Image>>() {
            app.init_asset::<Image>();
        }
        if !app.world().contains_resource::<Assets<StandardMaterial>>() {
            app.init_asset::<StandardMaterial>();
        }
        if !app.world().contains_resource::<Assets<SkinnedMeshInverseBindposes>>() {
            app.init_asset::<SkinnedMeshInverseBindposes>();
        }
        app.init_resource::<GfxCache>();
    }
}

/// Texture cache: RW texture name (case-insensitive) -> Bevy image + "has alpha".
#[derive(Resource, Default)]
pub struct GfxCache {
    pub textures: Vec<(String, Handle<Image>, bool)>,
}

/// Marks the root entity of an EF3dObj instance.
#[derive(Component, Debug, Clone, Copy)]
pub struct EF3dObjEntity {
    pub owner: u32,
    pub res_uid: u32,
}
/// A clump frame entity (index in the clump frame list).
#[derive(Component, Debug, Clone, Copy)]
pub struct RwFrameEntity(pub usize);
/// A mesh entity drawn for one clump atomic (index in `RpClump::atomics`) / material.
#[derive(Component, Debug, Clone, Copy)]
pub struct RwAtomicEntity {
    pub atomic: usize,
    pub material: usize,
}
/// Root of the static level world.
#[derive(Component, Debug, Clone, Copy)]
pub struct StaticWorldEntity(pub u32);

pub fn rw_mat(m: &[f32; 16]) -> Mat4 {
    Mat4::from_cols_array(m)
}

/// RwTexture (RGBA8 already decoded by rw::image) -> Image (sRGB).
/// The sampler comes from RwTexture +0x50 filterAddressing (WP-C4: the default Bevy sampler is
/// clamp-to-edge, but RW textures default to rwTEXTUREADDRESSWRAP and the game's UVs run far
/// outside 0..1 — e.g. the book cover 0.38..8.03 — so every surface sampled one edge texel).
pub fn texture_to_image(t: &RwTexture) -> Image {
    let mut img = Image::new(
        Extent3d { width: t.width, height: t.height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        t.rgba.clone(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    img.sampler = rw_sampler(t.filter_addressing);
    img
}

/// RwTextureFilterMode (low byte) / RwTextureAddressMode U (bits 8-11), V (bits 12-15) -> sampler.
/// Address: 1 wrap, 2 mirror, 3 clamp, 4 border (GS has no border colour: REGION_CLAMP, mapped
/// to clamp). Filter: 1 nearest, 2 linear, 3/4 mip nearest/linear-mip-nearest, 5/6 mip-linear.
pub fn rw_sampler(fa: u32) -> bevy_image::ImageSampler {
    use bevy_image::{ImageAddressMode as A, ImageFilterMode as F, ImageSampler, ImageSamplerDescriptor};
    let addr = |v: u32| match v & 0xf {
        2 => A::MirrorRepeat,
        3 | 4 => A::ClampToEdge,
        _ => A::Repeat,
    };
    let (mag, mip) = match fa & 0xff {
        1 | 3 => (F::Nearest, F::Nearest),
        5 => (F::Nearest, F::Linear),
        2 | 4 => (F::Linear, F::Nearest),
        _ => (F::Linear, F::Linear),
    };
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: addr(fa >> 8),
        address_mode_v: addr(fa >> 12),
        address_mode_w: A::Repeat,
        mag_filter: mag,
        min_filter: mag,
        mipmap_filter: mip,
        ..Default::default()
    })
}

impl GfxCache {
    /// Image handle for a texture name looked up in the dictionary (RwTexDictionaryFindNamedTexture).
    pub fn texture(&mut self, name: &str, dict: &RwTexDictionary, images: &mut Assets<Image>) -> Option<(Handle<Image>, bool)> {
        if let Some(t) = self.textures.iter().find(|t| rw_name_eq(&t.0, name)) {
            return Some((t.1.clone(), t.2));
        }
        let t = dict.Get(name)?;
        // WP-J6: `alpha` = the texture has PARTIAL alpha (needs blending). Textures whose alpha is
        // only 0 / 255 (scooby: 13 cut-out texels) are drawn alpha-tested with z-write (Mask):
        // the PS2 GS blends with ZMSK off + alpha test, which Bevy's Blend (no depth write,
        // per-entity sort) cannot express => Scooby's far side drew over his near side.
        let alpha = t.rgba.chunks_exact(4).any(|p| p[3] > 0 && p[3] < 250);
        let h = images.add(texture_to_image(t));
        self.textures.push((name.to_string(), h.clone(), alpha));
        Some((h, alpha))
    }
}

/// RpMaterial -> StandardMaterial.
pub fn material_to_standard(m: &RpMaterial, tex: Option<(Handle<Image>, bool)>, unlit: bool, blend: BlendMode) -> StandardMaterial {
    let c = m.color;
    let has_alpha = c[3] < 255 || tex.as_ref().is_some_and(|t| t.1);
    StandardMaterial {
        base_color: Color::srgba_u8(c[0], c[1], c[2], c[3]),
        base_color_texture: tex.map(|t| t.0),
        unlit,
        double_sided: true,
        cull_mode: None,
        alpha_mode: match blend {
            BlendMode::Additive | BlendMode::AdditiveFix => AlphaMode::Add,
            BlendMode::Subtractive => AlphaMode::Multiply,
            BlendMode::Normal if has_alpha => AlphaMode::Blend,
            BlendMode::Normal if m.texture.is_some() => AlphaMode::Mask(0.5),
            BlendMode::Normal => AlphaMode::Opaque,
        },
        ..Default::default()
    }
}

fn colors(c: &[[u8; 4]]) -> Vec<[f32; 4]> {
    c.iter().map(|&c| rwrgba_to_linear(c)).collect()
}

/// RpGeometry -> one triangle-list Mesh per material split: (material index, mesh).
pub fn geometry_to_meshes(g: &RpGeometry) -> Vec<(u32, Mesh)> {
    let Some(mt) = g.morph_targets.first() else { return Vec::new() };
    let n = mt.verts.len();
    let mut out = Vec::new();
    for sm in g.submeshes() {
        if sm.indices.is_empty() || sm.indices.iter().any(|&i| i as usize >= n) {
            continue;
        }
        let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, mt.verts.clone());
        if mt.normals.len() == n {
            m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, mt.normals.clone());
        }
        match g.uvs.first() {
            Some(uv) if uv.len() == n => m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv.clone()),
            _ => m.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0f32; 2]; n]),
        }
        if g.prelit.len() == n {
            m.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors(&g.prelit));
        }
        if let Some(s) = &g.skin {
            if s.indices.len() == n {
                let idx: Vec<[u16; 4]> = s.indices.iter().map(|i| [i[0] as u16, i[1] as u16, i[2] as u16, i[3] as u16]).collect();
                m.insert_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, VertexAttributeValues::Uint16x4(idx));
                m.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, s.weights.clone());
            }
        }
        m.insert_indices(Indices::U32(sm.indices));
        out.push((sm.material, m));
    }
    out
}

/// RpWorld sectors -> (material index, mesh) per sector mesh (PS2 native data decoded by rw::world).
pub fn world_to_meshes(w: &RpWorld) -> Vec<(u32, Mesh)> {
    let mut out = Vec::new();
    for s in &w.sectors {
        for sm in &s.meshes {
            let n = sm.mesh.positions.len();
            if n == 0 || sm.indices.is_empty() {
                continue;
            }
            let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
            m.insert_attribute(Mesh::ATTRIBUTE_POSITION, sm.mesh.positions.clone());
            if sm.mesh.normals.len() == n {
                m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, sm.mesh.normals.clone());
            }
            let uv = if sm.mesh.uvs.len() == n { sm.mesh.uvs.clone() } else { vec![[0.0; 2]; n] };
            m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
            if sm.mesh.colors.len() == n {
                // PS2 prelit alpha 0x80 = 1.0 (FixAlphaSectorCallBack forces 0xff); colour 0x80 = 1.0
                let c: Vec<[f32; 4]> = sm.mesh.colors.iter().map(|&c| { let mut l = gs_to_linear(c); l[3] = 1.0; l }).collect();
                m.insert_attribute(Mesh::ATTRIBUTE_COLOR, c);
            }
            m.insert_indices(Indices::U32(sm.indices.clone()));
            out.push((sm.material, m));
        }
    }
    out
}

/// Spawns an EF3dObj instance: root (owner transform) -> frame entities -> atomic mesh entities.
/// Returns the root entity.
pub fn spawn_3dobj(world: &mut World, owner: u32, owner_ltm: &[f32; 16], o: &EF3dObj_SP, clump: &RpClump, dict: &RwTexDictionary) -> Entity {
    let root = world
        .spawn((EF3dObjEntity { owner, res_uid: o.res_uid }, Transform::from_matrix(rw_mat(owner_ltm)), GlobalTransform::default()))
        .id();
    let frames: Vec<Entity> = (0..o.frames.local.len())
        .map(|i| world.spawn((RwFrameEntity(i), Transform::from_matrix(rw_mat(&o.frames.local[i])), GlobalTransform::default())).id())
        .collect();
    for (i, &e) in frames.iter().enumerate() {
        let p = o.frames.parent[i];
        let parent = if p >= 0 && (p as usize) < frames.len() { frames[p as usize] } else { root };
        world.entity_mut(parent).add_child(e);
    }
    let hier = o.hier.hierarchy.as_ref();
    for a in &o.atomics {
        if a.flags & super::render::rpATOMICRENDER == 0 {
            continue;
        }
        let Some(g) = clump.geometries.get(a.geometry) else { continue };
        let skinned = match (&g.skin, hier) {
            (Some(s), Some(h)) => {
                let joints: Vec<Entity> =
                    (0..s.num_bones as usize).map(|i| h.node_frames.get(i).copied().flatten().map(|f| frames[f]).unwrap_or(root)).collect();
                let ib: Vec<Mat4> =
                    (0..s.num_bones as usize).map(|i| rw_mat(&s.inverse_matrices.get(i).map(SkinInverseMatrix).unwrap_or(IDENTITY))).collect();
                let ibh = world.resource_mut::<Assets<SkinnedMeshInverseBindposes>>().add(SkinnedMeshInverseBindposes::from(ib));
                Some(SkinnedMesh { inverse_bindposes: ibh, joints })
            }
            _ => None,
        };
        let parent = frames.get(a.frame).copied().unwrap_or(root);
        for (mi, mesh) in geometry_to_meshes(g) {
            let mat = &g.materials[mi as usize];
            let tex = mat.texture.as_ref().and_then(|t| {
                world.resource_scope(|w, mut cache: Mut<GfxCache>| {
                    let mut images = w.resource_mut::<Assets<Image>>();
                    cache.texture(&t.name, dict, &mut images)
                })
            });
            let sm = material_to_standard(mat, tex, false, o.blend_mode);
            let mh = world.resource_mut::<Assets<Mesh>>().add(mesh);
            let math = world.resource_mut::<Assets<StandardMaterial>>().add(sm);
            let mut e = world.spawn((
                RwAtomicEntity { atomic: a.atomic, material: mi as usize },
                Mesh3d(mh),
                MeshMaterial3d(math),
                Transform::IDENTITY,
                GlobalTransform::default(),
            ));
            if let Some(s) = &skinned {
                e.insert(s.clone());
            }
            let id = e.id();
            // skinned meshes are posed by their joints: parent them to the root
            world.entity_mut(if skinned.is_some() { root } else { parent }).add_child(id);
        }
    }
    root
}

/// Spawns the level's static world (EFWorld_SP::GetCurrentWorld) as unlit meshes.
pub fn spawn_static_world(world: &mut World, s: &EFStatic3dObj, dict: &RwTexDictionary) -> Option<Entity> {
    let w = s.world.as_ref()?;
    let root = world.spawn((StaticWorldEntity(s.uid), Transform::IDENTITY, GlobalTransform::default())).id();
    for (mi, mesh) in world_to_meshes(w) {
        let Some(mat) = w.materials.get(mi as usize) else { continue };
        let tex = mat.texture.as_ref().and_then(|t| {
            world.resource_scope(|wd, mut cache: Mut<GfxCache>| {
                let mut images = wd.resource_mut::<Assets<Image>>();
                cache.texture(&t.name, dict, &mut images)
            })
        });
        let sm = material_to_standard(mat, tex, true, BlendMode::Normal);
        let mh = world.resource_mut::<Assets<Mesh>>().add(mesh);
        let math = world.resource_mut::<Assets<StandardMaterial>>().add(sm);
        let e = world.spawn((Mesh3d(mh), MeshMaterial3d(math), Transform::IDENTITY, GlobalTransform::default())).id();
        world.entity_mut(root).add_child(e);
    }
    Some(root)
}

/// Per-frame sync: frame tree local matrices (after EF3dObj UpdateHierarchy) -> frame Transforms,
/// owner LTM -> root Transform.
pub fn sync_3dobj(world: &mut World, root: Entity, owner_ltm: &[f32; 16], o: &EF3dObj_SP) {
    if let Some(mut t) = world.get_mut::<Transform>(root) {
        *t = Transform::from_matrix(rw_mat(owner_ltm));
    }
    let mut q = world.query::<(&RwFrameEntity, &mut Transform, &bevy_ecs::hierarchy::ChildOf)>();
    let ids: Vec<Entity> = descendants(world, root);
    for e in ids {
        if let Ok((f, mut t, _)) = q.get_mut(world, e) {
            if let Some(m) = o.frames.local.get(f.0) {
                *t = Transform::from_matrix(rw_mat(m));
            }
        }
    }
}

fn descendants(world: &World, root: Entity) -> Vec<Entity> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(e) = stack.pop() {
        if let Some(c) = world.get::<bevy_ecs::hierarchy::Children>(e) {
            for x in c.iter() {
                out.push(x);
                stack.push(x);
            }
        }
    }
    out
}

// =============================================================================================
// WP-C3: draw-list renderer. The 1:1 path: EF3dObj_SP::Render (LOD test, rpATOMICRENDER, blend
// colour callbacks, tex-fader passes, stencil/z-write state) emits `DrawCmd`s into
// `EFGraphicsMgr::draws`; `ApplyDrawList` mirrors exactly that list into Bevy entities each frame.
// Nothing is drawn that Render did not emit, so LOD switching is the same predicate as
// EF3dObj::Render / EF3dObj_SP::LodGetAtomic (ApGetAtomicLod(atomic) == lod & 7).
//
// Axis convention (verified, see tests/engine_c3.rs `rw_camera_looks_at_level`): RW and Bevy are
// both right-handed with +Y up, so world/clump geometry is used unchanged. Only the camera differs:
// an RW camera looks down its frame's +at (+Z) and its +right (+X) is SCREEN-LEFT (EFFrustum::
// SetFromCameraParams: plane +0x20 "left" has normal (+1,0,-vw)); a Bevy camera looks down -Z
// with +X screen-right. Bevy camera transform = RW camera LTM * rotY(180°) = LTM * diag(-1,1,-1).
// =============================================================================================

use super::hierarchy::Mat4 as RwMat4;
use super::mgr::FogState;
use super::render::DrawCmd;
use crate::engine::camera::{DTMatrix, RwCamera};
use bevy_camera::visibility::Visibility;
use bevy_camera::{PerspectiveProjection, Projection};
use bevy_pbr::{DistanceFog, FogFalloff};
use std::collections::HashMap;

/// RW camera frame (EFCamera +0x14 LTM) -> Bevy camera Transform (see the axis note above).
pub fn rw_camera_transform(ltm: &DTMatrix) -> Transform {
    let (r, u, a, p) = (ltm.right, ltm.up, ltm.at, ltm.pos);
    let m = Mat4::from_cols_array(&[r[0], r[1], r[2], 0., u[0], u[1], u[2], 0., a[0], a[1], a[2], 0., p[0], p[1], p[2], 1.]);
    Transform::from_matrix(m * Mat4::from_scale(bevy_math::Vec3::new(-1.0, 1.0, -1.0)))
}

/// RwCamera view window / clip planes -> Bevy perspective. RW view window = tan(half fov) (y
/// component for the vertical fov); aspect = vw.x / vw.y.
pub fn rw_camera_projection(c: &RwCamera) -> Projection {
    let vy = if c.view_window[1] > 0.0 { c.view_window[1] } else { 0.5 };
    let vx = if c.view_window[0] > 0.0 { c.view_window[0] } else { vy * 4.0 / 3.0 };
    Projection::Perspective(PerspectiveProjection {
        fov: 2.0 * vy.atan(),
        aspect_ratio: vx / vy,
        near: c.near.max(0.001),
        far: c.far.max(c.near + 1.0),
        ..Default::default()
    })
}

/// EFGraphicsMgr::SetRenderState fog (RwRenderStateSet FOGENABLE/FOGCOLOR/FOGTYPE/FOGDENSITY) ->
/// DistanceFog for the camera entity. Colour is the packed EFCamera +0x2c 0xAARRGGBB word.
/// rwFOGTYPELINEAR (1): from the camera fog plane (+0x88) to the far plane, as the PS2 GS fog
/// coefficient is computed per vertex; EXPONENTIAL (2) / EXPONENTIAL2 (3) use the density.
pub fn fog_to_distance_fog(f: &FogState, cam: &RwCamera) -> Option<DistanceFog> {
    if !f.enabled {
        return None;
    }
    let c = f.color;
    let color = Color::srgba_u8((c >> 16) as u8, (c >> 8) as u8, c as u8, 255);
    let falloff = match f.fog_type {
        2 => FogFalloff::Exponential { density: f.density },
        3 => FogFalloff::ExponentialSquared { density: f.density },
        _ => FogFalloff::Linear { start: cam.fog_plane, end: cam.far.max(cam.fog_plane + 1.0) },
    };
    Some(DistanceFog { color, falloff, ..Default::default() })
}

/// Inserts / removes the camera's DistanceFog from the graphics manager's fog state.
pub fn ApplyFog(world: &mut World, camera: Entity, f: &FogState, cam: &RwCamera) {
    match fog_to_distance_fog(f, cam) {
        Some(d) => {
            world.entity_mut(camera).insert(d);
        }
        None => {
            world.entity_mut(camera).remove::<DistanceFog>();
        }
    }
}

/// The material state a DrawCmd material is drawn with (RW material colour after
/// BlendMaterialColorCB, texture after tex-fader / texture-anim swaps, fader plugin alpha, owner
/// blend mode). Used as the StandardMaterial cache key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DrawMaterialKey {
    pub texture: Option<String>,
    /// RGBA8 used as base colour (white when the geometry does not modulate material colour)
    pub color: [u8; 4],
    /// tex-fader plugin alpha bits when the 0x80000000 plugin flag is set
    pub fade_alpha: Option<u32>,
    /// BlendMode as u8 (0 Normal, 1 Additive, 2 AdditiveFix, 3 Subtractive)
    pub blend: u8,
}

impl DrawMaterialKey {
    pub fn from_draw(d: &DrawCmd, mi: usize) -> Self {
        let m = &d.materials[mi];
        // rpGEOMETRYMODULATEMATERIALCOLOR: RW multiplies by the material colour only when the
        // geometry flag is set (EF3dObj::Render sets it while blending, flag 0x400)
        let color = if d.geometry_flags & super::render::rpGEOMETRYMODULATEMATERIALCOLOR != 0 { m.color } else { [255; 4] };
        // EFTexFader_SP::FadeAtomic draws the atomic twice with the 0x69690000 plugin flags
        // |0xc0000000 (source, alpha A) / |0x80000000 (destination, alpha B): bit 31 = use the
        // plugin alpha for the material.
        let fade_alpha = (m.fade.0 & 0x8000_0000 != 0).then(|| m.fade.1.clamp(0.0, 1.0).to_bits());
        DrawMaterialKey { texture: m.texture.as_ref().map(|t| t.to_ascii_lowercase()), color, fade_alpha, blend: d.blend as u8 }
    }

    pub fn blend_mode(&self) -> BlendMode {
        match self.blend {
            1 => BlendMode::Additive,
            2 => BlendMode::AdditiveFix,
            3 => BlendMode::Subtractive,
            _ => BlendMode::Normal,
        }
    }

    /// GS ALPHA register -> Bevy AlphaMode / colour.
    /// Normal: (Cs-Cd)*As+Cd = AlphaMode::Blend (Opaque when nothing has alpha < 1).
    /// Additive 0x44 (Cs*As+Cd) = AlphaMode::Add. AdditiveFix 0x68 (Cs*FIX+Cd, FIX=0x80=1.0) =
    /// AlphaMode::Add with base alpha 1. Subtractive 0xa1 ((Cd-Cs)*FIX): no subtractive blend in
    /// StandardMaterial; approximated by AlphaMode::Multiply with the inverted colour
    /// (Cd*(1-Cs) == Cd - Cs*Cd).
    pub fn to_standard(&self, tex: Option<(Handle<Image>, bool)>, unlit: bool) -> StandardMaterial {
        let c = self.color;
        let mut a = c[3] as f32 / 255.0;
        if let Some(f) = self.fade_alpha {
            a *= f32::from_bits(f);
        }
        let tex_alpha = tex.as_ref().is_some_and(|t| t.1);
        let (rgb, alpha_mode, a) = match self.blend_mode() {
            BlendMode::Additive => ([c[0], c[1], c[2]], AlphaMode::Add, a),
            BlendMode::AdditiveFix => ([c[0], c[1], c[2]], AlphaMode::Add, 1.0),
            BlendMode::Subtractive => ([255 - c[0], 255 - c[1], 255 - c[2]], AlphaMode::Multiply, a),
            BlendMode::Normal if a < 1.0 || tex_alpha || self.fade_alpha.is_some() => ([c[0], c[1], c[2]], AlphaMode::Blend, a),
            BlendMode::Normal if self.texture.is_some() => ([c[0], c[1], c[2]], AlphaMode::Mask(0.5), a),
            BlendMode::Normal => ([c[0], c[1], c[2]], AlphaMode::Opaque, a),
        };
        StandardMaterial {
            base_color: Color::srgba(rgb[0] as f32 / 255.0, rgb[1] as f32 / 255.0, rgb[2] as f32 / 255.0, a),
            base_color_texture: tex.map(|t| t.0),
            unlit,
            double_sided: true,
            cull_mode: None,
            alpha_mode,
            ..Default::default()
        }
    }
}

/// CPU copy of a skinned geometry for palette skinning of DrawCmd::skin.
struct SkinSrc {
    verts: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<[u8; 4]>,
    weights: Vec<[f32; 4]>,
}

/// Linear-blend skinning with the DrawCmd palette (atomic-local matrices, hierarchy::SkinMatrices).
fn skin_vertices(s: &SkinSrc, pal: &[RwMat4]) -> (Vec<[f32; 3]>, Vec<[f32; 3]>) {
    let mut p = Vec::with_capacity(s.verts.len());
    let mut n = Vec::with_capacity(s.verts.len());
    for i in 0..s.verts.len() {
        let (v, w, ix) = (s.verts[i], s.weights[i], s.indices[i]);
        let nn = s.normals.get(i).copied().unwrap_or([0.0; 3]);
        let (mut op, mut on) = ([0.0f32; 3], [0.0f32; 3]);
        for k in 0..4 {
            if w[k] == 0.0 {
                continue;
            }
            let Some(m) = pal.get(ix[k] as usize) else { continue };
            for r in 0..3 {
                op[r] += w[k] * (m[r] * v[0] + m[4 + r] * v[1] + m[8 + r] * v[2] + m[12 + r]);
                on[r] += w[k] * (m[r] * nn[0] + m[4 + r] * nn[1] + m[8 + r] * nn[2]);
            }
        }
        p.push(op);
        n.push(on);
    }
    (p, n)
}

/// One mesh entity of the draw list.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DrawListEntity {
    pub owner: u32,
    pub atomic: usize,
    pub material: u32,
    /// n-th DrawCmd for the same atomic this frame (the tex fader draws an atomic twice)
    pub pass: u32,
}

/// Retained state of the draw-list renderer (entity pool, mesh/material caches).
#[derive(Resource, Default)]
pub struct DrawListState {
    /// (res uid, geometry, material) -> rest-pose mesh
    meshes: HashMap<(u32, usize, u32), Handle<Mesh>>,
    skins: HashMap<(u32, usize), SkinSrc>,
    /// F1: (res uid, geometry) -> geometry_to_meshes output, built once (was rebuilt for every
    /// draw every frame: ~14 ms/frame of EP4L01's render)
    split: HashMap<(u32, usize), std::sync::Arc<Vec<(u32, Mesh)>>>,
    /// F1: inputs of the last CPU skin/light of a slot (res, geometry, material, ltm bits, geometry
    /// flags, hash of the skin matrices); the lights are the level's static ones, so equal inputs =
    /// equal mesh
    lit_key: HashMap<DrawListEntity, (u32, usize, u32, [u32; 16], u32, u64)>,
    /// (key, unlit = CPU-lit by RwLights)
    materials: HashMap<(DrawMaterialKey, bool), Handle<StandardMaterial>>,
    entities: HashMap<DrawListEntity, (Entity, Option<Handle<Mesh>>)>,
    /// entities shown by the last ApplyDrawList
    pub visible: usize,
}

impl DrawListState {
    /// Pooled entity of a draw-list slot.
    pub fn entity(&self, id: &DrawListEntity) -> Option<Entity> {
        self.entities.get(id).map(|e| e.0)
    }
    /// Cached material of a key.
    pub fn material(&self, k: &DrawMaterialKey) -> Option<Handle<StandardMaterial>> {
        self.materials.get(&(k.clone(), false)).or_else(|| self.materials.get(&(k.clone(), true))).cloned()
    }
}

/// Mirrors `draws` (EFGraphicsMgr::draws after this frame's EF3dObj Render calls) into Bevy:
/// one entity per (owner, atomic, material, pass), world Transform = DrawCmd::ltm, skinned draws
/// CPU-skinned with DrawCmd::skin; every pooled entity not drawn this frame is hidden.
/// `clump_of(owner)` returns the instance's (res uid, clump).
pub fn ApplyDrawList<'a>(world: &mut World, draws: &[DrawCmd], clump_of: &dyn Fn(u32) -> Option<(u32, &'a RpClump)>, dict: &RwTexDictionary) {
    if !world.contains_resource::<DrawListState>() {
        world.init_resource::<DrawListState>();
    }
    if !world.contains_resource::<GfxCache>() {
        world.init_resource::<GfxCache>();
    }
    let lights = world.get_resource::<RwLights>().cloned();
    world.resource_scope(|world, mut st: Mut<DrawListState>| {
        let mut seen: HashMap<(u32, usize, u32), u32> = HashMap::new();
        let mut used = std::collections::HashSet::new();
        for d in draws {
            let Some((res, clump)) = clump_of(d.owner) else { continue };
            let Some(g) = clump.geometries.get(d.geometry) else { continue };
            let split = st.split.entry((res, d.geometry)).or_insert_with(|| std::sync::Arc::new(geometry_to_meshes(g))).clone();
            for (mi, mesh) in split.iter() {
                let mi = *mi;
                if mi as usize >= d.materials.len() {
                    continue;
                }
                let key = (res, d.geometry, mi);
                let pass = {
                    let c = seen.entry((d.owner, d.atomic, mi)).or_insert(0);
                    *c += 1;
                    *c - 1
                };
                let id = DrawListEntity { owner: d.owner, atomic: d.atomic, material: mi, pass };
                let skinned = d.skin.is_some() && g.skin.is_some();
                let skin_hash = d.skin.as_ref().filter(|_| skinned).map_or(0, |m| {
                    use std::hash::{Hash, Hasher};
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    m.iter().for_each(|x| x.map(f32::to_bits).hash(&mut h));
                    h.finish()
                });
                let lk = (res, d.geometry, mi, d.ltm.map(f32::to_bits), d.geometry_flags, skin_hash);
                let reuse = st.entities.get(&id).and_then(|e| e.1.clone()).filter(|_| st.lit_key.get(&id) == Some(&lk));
                let mh = if let Some(h) = reuse {
                    // F1: unchanged inputs: skip the CPU skin/relight + GPU re-upload
                    h
                } else if skinned || lights.is_some() {
                    st.lit_key.insert(id, lk);
                    // per-entity mesh: CPU skinning and/or CPU RW lighting rewrite it every frame
                    let mut m = mesh.clone();
                    let (p, n) = if skinned {
                        let src = st.skins.entry((res, d.geometry)).or_insert_with(|| {
                            let s = g.skin.as_ref().unwrap();
                            let mt = &g.morph_targets[0];
                            SkinSrc { verts: mt.verts.clone(), normals: mt.normals.clone(), indices: s.indices.clone(), weights: s.weights.clone() }
                        });
                        skin_vertices(src, d.skin.as_ref().unwrap())
                    } else {
                        let mt = &g.morph_targets[0];
                        (mt.verts.clone(), mt.normals.clone())
                    };
                    if let Some(l) = &lights {
                        let mat = &g.materials[mi as usize];
                        let c = l.LightVertices(&d.ltm, &p, &n, &g.prelit, d.geometry_flags, mat.ambient, mat.diffuse);
                        m.insert_attribute(Mesh::ATTRIBUTE_COLOR, c);
                    }
                    let nv = p.len();
                    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, p);
                    if m.attribute(Mesh::ATTRIBUTE_NORMAL).is_some() && n.len() == nv {
                        m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, n);
                    }
                    m.remove_attribute(Mesh::ATTRIBUTE_JOINT_INDEX);
                    m.remove_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT);
                    let prev = st.entities.get(&id).and_then(|e| e.1.clone());
                    let mut meshes = world.resource_mut::<Assets<Mesh>>();
                    match prev {
                        Some(h) => {
                            let _ = meshes.insert(h.id(), m);
                            h
                        }
                        None => meshes.add(m),
                    }
                } else if let Some(h) = st.meshes.get(&key) {
                    h.clone()
                } else {
                    let mut m = mesh.clone();
                    m.remove_attribute(Mesh::ATTRIBUTE_JOINT_INDEX);
                    m.remove_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT);
                    let h = world.resource_mut::<Assets<Mesh>>().add(m);
                    st.meshes.insert(key, h.clone());
                    h
                };
                let per_entity = skinned || lights.is_some();
                let mk = DrawMaterialKey::from_draw(d, mi as usize);
                let unlit = lights.is_some();
                let math = match st.materials.get(&(mk.clone(), unlit)) {
                    Some(h) => h.clone(),
                    None => {
                        let tex = mk.texture.clone().and_then(|t| {
                            world.resource_scope(|w, mut cache: Mut<GfxCache>| {
                                let mut images = w.resource_mut::<Assets<Image>>();
                                cache.texture(&t, dict, &mut images)
                            })
                        });
                        let h = world.resource_mut::<Assets<StandardMaterial>>().add(mk.to_standard(tex, unlit));
                        st.materials.insert((mk, unlit), h.clone());
                        h
                    }
                };
                let tr = Transform::from_matrix(rw_mat(&d.ltm));
                let e = match st.entities.get(&id) {
                    Some(&(e, _)) => {
                        world.entity_mut(e).insert((Mesh3d(mh.clone()), MeshMaterial3d(math), tr, Visibility::Visible));
                        e
                    }
                    None => {
                        let e = world.spawn((id, Mesh3d(mh.clone()), MeshMaterial3d(math), tr, GlobalTransform::default(), Visibility::Visible)).id();
                        // WP-J6: CPU-skinned meshes are rewritten every frame, but Bevy computes the
                        // Aabb once (first pose) and never refreshes it: limbs outside the spawn-pose
                        // box got frustum-culled. The PS2 culls skinned atomics by the HAnim bounding
                        // sphere, not per material split: skip Bevy's per-mesh culling for them.
                        if skinned {
                            world.entity_mut(e).insert(bevy_camera::visibility::NoFrustumCulling);
                        }
                        e
                    }
                };
                st.entities.insert(id, (e, per_entity.then_some(mh)));
                used.insert(id);
            }
        }
        for (id, (e, _)) in st.entities.iter() {
            if !used.contains(id) {
                if let Ok(mut em) = world.get_entity_mut(*e) {
                    em.insert(Visibility::Hidden);
                }
            }
        }
        st.visible = used.len();
    });
}

// =============================================================================================
// WP-C4: RW lighting on the CPU, PS2 colour scale, particles / billboards.
// =============================================================================================

use crate::engine::particle::{PLDrawBatch, PLPrims};
use crate::engine::rw::clump::RpLight as RwStreamLight;
use crate::engine::rw::geometry::{rpGEOMETRYLIGHT, rpGEOMETRYPRELIT};

/// sRGB-encoded 0..1 -> linear (Bevy vertex colours are linear; PS2 colours are display values,
/// so a PS2 `texel * vertex` product in display space == Bevy's linear product re-encoded, up to
/// the sRGB curve's deviation from a pure power law).
pub fn srgb_to_linear(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

/// RwRGBA 0..255 (generic geometry stream, RpGeometry prelit) -> linear RGBA. The PS2 RW pipelines
/// upload these as GS colours scaled 255 -> 0x80 (= 1.0), so 255 is full intensity.
pub fn rwrgba_to_linear(c: [u8; 4]) -> [f32; 4] {
    [srgb_to_linear(c[0] as f32 / 255.0), srgb_to_linear(c[1] as f32 / 255.0), srgb_to_linear(c[2] as f32 / 255.0), c[3] as f32 / 255.0]
}

/// PS2 GS colour (0x80 = 1.0; native world prelit, particle colours) -> linear RGBA. Values
/// above 1.0 saturate (the GS clamps the modulated texel, TEX0.TFX MODULATE).
pub fn gs_to_linear(c: [u8; 4]) -> [f32; 4] {
    [srgb_to_linear(c[0] as f32 / 128.0), srgb_to_linear(c[1] as f32 / 128.0), srgb_to_linear(c[2] as f32 / 128.0), (c[3] as f32 / 128.0).min(1.0)]
}

/// One RpLight in the world (static level light: EFStaticLight, colour/type/radius + frame).
#[derive(Debug, Clone, PartialEq)]
pub struct RwLightInst {
    /// 1 directional, 2 ambient, 0x80 point, 0x81 spot, 0x82 soft spot
    pub ty: u8,
    /// rpLIGHTLIGHTATOMICS 1 | rpLIGHTLIGHTWORLD 2
    pub flags: u8,
    pub color: [f32; 3],
    pub radius: f32,
    pub minus_cos_angle: f32,
    /// frame +at (the light direction for directional/spot lights)
    pub at: [f32; 3],
    pub pos: [f32; 3],
}

/// Lights applied to atomics. Replacement of the PS2 RW atomic lighting (VU1 microcode of the
/// skinned/unskinned DFF mapper pipelines): evaluated per vertex on the CPU in ApplyDrawList when
/// this resource exists, with RW's generic lighting model:
///   colour = prelit + ka * Σ ambient + kd * Σ (directional/point/spot) max(0, n·L) * atten,
/// clamped to 1, then × material colour (rpGEOMETRYMODULATEMATERIALCOLOR) × texel in the
/// material. Point attenuation is RW's linear (1 - d/radius); spot = point inside the cone
/// (cos > -minusCosAngle), soft spot fades linearly towards the cone edge. Geometries without
/// rpGEOMETRYLIGHT get prelit only (white without prelit). The materials are then unlit.
#[derive(Resource, Debug, Clone, Default)]
pub struct RwLights {
    pub lights: Vec<RwLightInst>,
}

impl RwLights {
    /// From the level's static lights (EFStatic3dObj::lights: RpLight + 12-real frame
    /// right, up, at, pos). Only lights with rpLIGHTLIGHTATOMICS light clumps.
    pub fn from_static(ls: &[super::EFStaticLight]) -> Self {
        let lights = ls
            .iter()
            .filter(|l| l.light.flags & 1 != 0)
            .map(|l| {
                let m = &l.matrix;
                Self::inst(&l.light, [m[6], m[7], m[8]], [m[9], m[10], m[11]])
            })
            .collect();
        RwLights { lights }
    }

    fn inst(l: &RwStreamLight, at: [f32; 3], pos: [f32; 3]) -> RwLightInst {
        RwLightInst { ty: l.light_type, flags: l.flags, color: l.color, radius: l.radius, minus_cos_angle: l.minus_cos_angle, at, pos }
    }

    /// Per-vertex colours (linear RGBA) for atomic-local positions/normals transformed by `ltm`.
    #[allow(clippy::too_many_arguments)]
    pub fn LightVertices(&self, ltm: &RwMat4, p: &[[f32; 3]], n: &[[f32; 3]], prelit: &[[u8; 4]], gflags: u32, ka: f32, kd: f32) -> Vec<[f32; 4]> {
        let m = ltm;
        let xf = |v: [f32; 3], w: f32| {
            [
                m[0] * v[0] + m[4] * v[1] + m[8] * v[2] + m[12] * w,
                m[1] * v[0] + m[5] * v[1] + m[9] * v[2] + m[13] * w,
                m[2] * v[0] + m[6] * v[1] + m[10] * v[2] + m[14] * w,
            ]
        };
        let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let lit = gflags & rpGEOMETRYLIGHT != 0;
        let has_prelit = gflags & rpGEOMETRYPRELIT != 0 && prelit.len() == p.len();
        let mut amb = [0.0f32; 3];
        for l in self.lights.iter().filter(|l| l.ty == 2) {
            for k in 0..3 {
                amb[k] += l.color[k] * ka;
            }
        }
        (0..p.len())
            .map(|i| {
                let mut c = if has_prelit {
                    let q = prelit[i];
                    [q[0] as f32 / 255.0, q[1] as f32 / 255.0, q[2] as f32 / 255.0, q[3] as f32 / 255.0]
                } else if lit {
                    [0.0, 0.0, 0.0, 1.0]
                } else {
                    [1.0; 4]
                };
                if lit {
                    let wp = xf(p[i], 1.0);
                    let mut wn = xf(n.get(i).copied().unwrap_or([0.0, 1.0, 0.0]), 0.0);
                    let ln = dot(wn, wn).sqrt();
                    if ln > 1e-6 {
                        wn = [wn[0] / ln, wn[1] / ln, wn[2] / ln];
                    }
                    for k in 0..3 {
                        c[k] += amb[k];
                    }
                    for l in &self.lights {
                        let f = match l.ty {
                            1 => (-dot(wn, l.at)).max(0.0),
                            0x80..=0x82 => {
                                let d = [l.pos[0] - wp[0], l.pos[1] - wp[1], l.pos[2] - wp[2]];
                                let dist = dot(d, d).sqrt();
                                if dist >= l.radius || dist < 1e-6 {
                                    0.0
                                } else {
                                    let ld = [d[0] / dist, d[1] / dist, d[2] / dist];
                                    let mut f = dot(wn, ld).max(0.0) * (1.0 - dist / l.radius);
                                    if l.ty != 0x80 {
                                        let cosv = -dot(ld, l.at);
                                        let cosc = -l.minus_cos_angle;
                                        if cosv < cosc {
                                            f = 0.0;
                                        } else if l.ty == 0x82 && cosc < 1.0 {
                                            f *= ((cosv - cosc) / (1.0 - cosc)).clamp(0.0, 1.0);
                                        }
                                    }
                                    f
                                }
                            }
                            _ => 0.0,
                        };
                        for k in 0..3 {
                            c[k] += l.color[k] * kd * f;
                        }
                    }
                }
                [srgb_to_linear(c[0]), srgb_to_linear(c[1]), srgb_to_linear(c[2]), c[3].clamp(0.0, 1.0)]
            })
            .collect()
    }
}

/// One particle / billboard batch entity (index in the frame's batch list; `billboard` = from
/// EFBillboardMgr).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParticleBatchEntity {
    pub billboard: bool,
    pub index: usize,
}

#[derive(Resource, Default)]
pub struct ParticleDrawState {
    entities: HashMap<ParticleBatchEntity, (Entity, Handle<Mesh>)>,
    materials: HashMap<(String, u32), Handle<StandardMaterial>>,
    pub visible: usize,
}

/// PLDrawBatch::blend (RpSkyRenderStateSet(2, x): GS ALPHA register A|B<<2|C<<4|D<<6, FIX=0x80)
/// -> Bevy AlphaMode. 0x29: A=Cd B=0 C=FIX D=Cs -> Cd + Cs (additive, source alpha ignored) =
/// AlphaMode::Add with vertex alpha 1. 0x44: (Cs-Cd)*As+Cd = AlphaMode::Blend. 0xa1: (Cd-Cs)*FIX
/// (subtractive): approximated by AlphaMode::Multiply with the inverted vertex colour.
/// J23: 0x29 (Cd*FIX+Cs) / 0x68 (Cs*FIX+Cd) with FIX = 0x80 (RpSkyRenderStateSet(2,
/// 0x80_0000xx)) are pure ONE/ONE adds that ignore vertex AND texture alpha. AlphaMode::Add
/// multiplies rgb by alpha; Premultiplied (ONE, ONE_MINUS_SRC_ALPHA) with the vertex alpha set
/// to 0 (see particle_batch_mesh) gives exactly Cs + Cd.
pub fn particle_alpha_mode(blend: u32) -> AlphaMode {
    match blend & 0xff {
        0x44 => AlphaMode::Blend,
        0xa1 => AlphaMode::Multiply,
        0x29 | 0x68 => AlphaMode::Premultiplied,
        _ => AlphaMode::Add,
    }
}

/// Camera-facing quads of one batch (replacement of the PLEmitter::RenderBatch VU1 expansion).
/// `right`/`up` are the world-space screen axes of the camera (RW camera: screen-right =
/// -LTM.right, screen-up = LTM.up). Sprite: centre pos, half-size `size`, rotated by
/// rot_index*2π/128 in the screen plane; colour 0x80 = 1.0. Quads (3D types) are world space.
pub fn particle_batch_mesh(b: &PLDrawBatch, right: [f32; 3], up: [f32; 3]) -> Mesh {
    let (mut pos, mut uv, mut col, mut idx) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let tr = |c: [u8; 4]| {
        let mut l = gs_to_linear(c);
        match b.blend & 0xff {
            0xa1 => l = [1.0 - l[0], 1.0 - l[1], 1.0 - l[2], 1.0],
            0x44 => {}
            0x29 | 0x68 => l[3] = 0.0,
            _ => l[3] = 1.0,
        }
        l
    };
    match &b.prims {
        PLPrims::Sprites(v) => {
            for s in v {
                let a = s.rot_index as f32 * std::f32::consts::TAU / 128.0;
                let (sn, cs) = a.sin_cos();
                let base = pos.len() as u32;
                for (cx, cy, u, w) in [(-1.0f32, 1.0f32, 0.0, 0.0), (1.0, 1.0, 1.0, 0.0), (1.0, -1.0, 1.0, 1.0), (-1.0, -1.0, 0.0, 1.0)] {
                    let x = (cx * cs - cy * sn) * s.size;
                    let y = (cx * sn + cy * cs) * s.size;
                    pos.push([s.pos[0] + right[0] * x + up[0] * y, s.pos[1] + right[1] * x + up[1] * y, s.pos[2] + right[2] * x + up[2] * y]);
                    uv.push([u, w]);
                    col.push(tr(s.rgba));
                }
                idx.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
            }
        }
        PLPrims::Quads(v) => {
            for q in v {
                let base = pos.len() as u32;
                for k in 0..4 {
                    pos.push(q.verts[k]);
                    uv.push(q.uvs[k]);
                    col.push(tr(q.rgba));
                }
                idx.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
            }
        }
    }
    let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    m.insert_attribute(Mesh::ATTRIBUTE_COLOR, col);
    m.insert_indices(Indices::U32(idx));
    m
}

/// Mirrors RuntimeFrame::particles / ::billboards (batches of one camera slot) into pooled unlit
/// entities. PLEmitter::BeginRender state: z-write off (Bevy transparent passes do not write
/// depth), fog off. Not reproduced: stencil_read (EnableStencilRead destination-alpha test).
pub fn ApplyParticles(world: &mut World, particles: &[PLDrawBatch], billboards: &[PLDrawBatch], cam_ltm: &DTMatrix, dict: &RwTexDictionary) {
    if !world.contains_resource::<ParticleDrawState>() {
        world.init_resource::<ParticleDrawState>();
    }
    if !world.contains_resource::<GfxCache>() {
        world.init_resource::<GfxCache>();
    }
    let right = [-cam_ltm.right[0], -cam_ltm.right[1], -cam_ltm.right[2]];
    let up = cam_ltm.up;
    world.resource_scope(|world, mut st: Mut<ParticleDrawState>| {
        let mut used = std::collections::HashSet::new();
        for (bb, list) in [(false, particles), (true, billboards)] {
            for (i, b) in list.iter().enumerate() {
                let id = ParticleBatchEntity { billboard: bb, index: i };
                let mesh = particle_batch_mesh(b, right, up);
                let mk = (b.texture.to_ascii_lowercase(), b.blend & 0xff);
                let mat = match st.materials.get(&mk) {
                    Some(h) => h.clone(),
                    None => {
                        let tex = world.resource_scope(|w, mut cache: Mut<GfxCache>| {
                            let mut images = w.resource_mut::<Assets<Image>>();
                            cache.texture(&b.texture, dict, &mut images)
                        });
                        let sm = StandardMaterial {
                            base_color: Color::WHITE,
                            base_color_texture: tex.map(|t| t.0),
                            unlit: true,
                            fog_enabled: false,
                            double_sided: true,
                            cull_mode: None,
                            alpha_mode: particle_alpha_mode(b.blend),
                            ..Default::default()
                        };
                        let h = world.resource_mut::<Assets<StandardMaterial>>().add(sm);
                        st.materials.insert(mk, h.clone());
                        h
                    }
                };
                match st.entities.get(&id).cloned() {
                    Some((e, mh)) => {
                        let _ = world.resource_mut::<Assets<Mesh>>().insert(mh.id(), mesh);
                        world.entity_mut(e).insert((MeshMaterial3d(mat), Visibility::Visible));
                    }
                    None => {
                        let mh = world.resource_mut::<Assets<Mesh>>().add(mesh);
                        let e = world
                            .spawn((id, Mesh3d(mh.clone()), MeshMaterial3d(mat), Transform::IDENTITY, GlobalTransform::default(), Visibility::Visible))
                            .id();
                        st.entities.insert(id, (e, mh));
                    }
                }
                used.insert(id);
            }
        }
        for (id, (e, _)) in st.entities.iter() {
            if !used.contains(id) {
                if let Ok(mut em) = world.get_entity_mut(*e) {
                    em.insert(Visibility::Hidden);
                }
            }
        }
        st.visible = used.len();
    });
}

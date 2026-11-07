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
        tex_format(),
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
        base_color: display_color(c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, c[3] as f32 / 255.0),
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
/// `lights`: the rpLIGHTLIGHTWORLD lights (RwLights::from_static_world). P26 R17:
/// RpWorldSectorPS2AllLightingSetupFunc (0x17a950) lights every sector of a world with rpWORLDLIGHT
/// (0x20) with the world's global lights + the sector's local lights that pass
/// _rpWorldSectorPS2AllDoApplyLight (0x17a6e8: light flags & rpLIGHTLIGHTWORLD 2), ka = kd = 1.0
/// (RxPipelineNodePS2AllApplyLight(1.0, 1.0, ..)), on top of the prelit colour. The world lights
/// are the static lights (EFStatic3dObj::ReadLights RpWorldAddLight); the logic may recolour them
/// at run time, so game.rs re-bakes through relight_static_world. EP4/L03A, L03B, L09A (world-only
/// ambient: the blue night cast) and EP2/L09 (a world spot) are the only levels with such lights.
pub fn world_to_meshes(w: &RpWorld, lights: &RwLights) -> Vec<(u32, [u32; 2], Mesh)> {
    let mut out = Vec::new();
    for (si, s) in w.sectors.iter().enumerate() {
        for (mj, sm) in s.meshes.iter().enumerate() {
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
            if let Some(c) = world_sector_colors(w, si, mj, lights) {
                m.insert_attribute(Mesh::ATTRIBUTE_COLOR, c);
            }
            m.insert_indices(Indices::U32(sm.indices.clone()));
            out.push((sm.material, [si as u32, mj as u32], m));
        }
    }
    out
}

/// Vertex colours of world sector `si` mesh `mj` (None without prelit colours).
/// P26 R15 (P25 R75 GS dump): the PS2 world pipeline sends prelit*128/255 to the GS, so 255 =
/// 1.0 (was gs_to_linear c/128: everything >= 128 saturated, the baked sconce light cones
/// vanished and the world was ~1.8x too bright). Alpha: FixAlphaSectorCallBack forces 0xff.
pub fn world_sector_colors(w: &RpWorld, si: usize, mj: usize, lights: &RwLights) -> Option<Vec<[f32; 4]>> {
    let sm = w.sectors.get(si)?.meshes.get(mj)?;
    if sm.mesh.colors.len() != sm.mesh.positions.len() {
        return None;
    }
    let lit = w.format & rpGEOMETRYLIGHT != 0 && !lights.lights.is_empty();
    Some(if lit {
        let mut c = lights.LightVertices(&IDENTITY, &sm.mesh.positions, &sm.mesh.normals, &sm.mesh.colors, rpGEOMETRYLIGHT | rpGEOMETRYPRELIT, 1.0, 1.0);
        c.iter_mut().for_each(|v| v[3] = 1.0);
        c
    } else {
        sm.mesh.colors.iter().map(|&c| { let mut l = rwrgba_to_linear(c); l[3] = 1.0; l }).collect()
    })
}

/// A static world sector mesh entity: (sector, mesh) index into the RpWorld (for relighting).
#[derive(Component, Debug, Clone, Copy)]
pub struct WorldSectorMesh(pub [u32; 2]);

/// P37 R2: world-space bounds (min, max) of a [`WorldSectorMesh`]'s vertices (relight culling)
#[derive(Component, Debug, Clone, Copy)]
pub struct WorldSectorBounds(pub [f32; 3], pub [f32; 3]);

/// sphere (centre, radius) meets the box (min, max)
fn sphere_meets_box(c: [f32; 3], r: f32, lo: [f32; 3], hi: [f32; 3]) -> bool {
    let d2: f32 = (0..3).map(|k| (c[k] - c[k].clamp(lo[k], hi[k])).powi(2)).sum();
    d2 <= r * r
}

/// P37 R2: the spheres a world light set change can reach, or None = every sector. Only when the
/// set has the same lights and every changed one is a point/spot light (type 0x80..) both before and
/// after: such a light lights nothing at distance >= radius (LightVertices), so a sector outside
/// both its old and its new sphere keeps its colours (a moving dynamic point light no longer
/// re-bakes the whole world).
fn relight_spheres(prev: &WorldLightKey, key: &[i32], lights: &[RwLightInst]) -> Option<Vec<([f32; 3], f32)>> {
    if prev.0.len() != key.len() || prev.1.len() != lights.len() || key.len() != lights.len() * 10 {
        return None;
    }
    let mut v = Vec::new();
    for (i, (a, b)) in prev.0.chunks(10).zip(key.chunks(10)).enumerate() {
        if a == b {
            continue;
        }
        let (o, n) = (&prev.1[i], &lights[i]);
        if o.ty & 0x80 == 0 || n.ty & 0x80 == 0 {
            return None;
        }
        v.push((o.pos, o.radius.max(0.0)));
        v.push((n.pos, n.radius.max(0.0)));
    }
    Some(v)
}

/// P26 R17: re-evaluates the world sector lighting with the current rpLIGHTLIGHTWORLD lights. The
/// world's static lights are the light manager's EFLights (EFStatic3dObj::ReadLights), whose colour
/// the level logic changes at run time (EP4/L03B: ambient 0x12800002 (0.2, 0.3, 0.6), pulsing), and
/// the PS2 lights the sectors every frame (RpWorldSectorPS2AllLightingSetupFunc). The port bakes the
/// result into the vertex colours, so it re-bakes only when the world light set changed
/// (`WorldLightKey`, colours quantised to 1/255 = one GS colour step) and only for rpWORLDLIGHT worlds.
pub fn relight_static_world(world: &mut World, s: &EFStatic3dObj, lights: &RwLights) {
    let Some(w) = s.world.as_ref() else { return };
    if w.format & rpGEOMETRYLIGHT == 0 {
        return;
    }
    let q = |x: f32| (x * 255.0).round() as i32;
    let key: Vec<i32> = lights.lights.iter().flat_map(|l| [l.ty as i32, q(l.color[0]), q(l.color[1]), q(l.color[2]), q(l.at[0]), q(l.at[1]), q(l.at[2]), q(l.pos[0]), q(l.pos[1]), q(l.pos[2])]).collect();
    let spheres = match world.get_resource::<WorldLightKey>() {
        Some(k) if k.0 == key => return,
        Some(k) => relight_spheres(k, &key, &lights.lights),
        None => None,
    };
    world.insert_resource(WorldLightKey(key, lights.lights.clone()));
    let ents: Vec<(Handle<Mesh>, [u32; 2], Option<WorldSectorBounds>)> =
        world.query::<(&Mesh3d, &WorldSectorMesh, Option<&WorldSectorBounds>)>().iter(world).map(|(m, i, b)| (m.0.clone(), i.0, b.copied())).collect();
    let mut meshes = world.resource_mut::<Assets<Mesh>>();
    for (h, [si, mj], b) in ents {
        if let (Some(sp), Some(b)) = (&spheres, b) {
            if !sp.iter().any(|&(c, r)| sphere_meets_box(c, r, b.0, b.1)) {
                continue;
            }
        }
        let (Some(c), Some(mut m)) = (world_sector_colors(w, si as usize, mj as usize, lights), meshes.get_mut(&h)) else { continue };
        m.insert_attribute(Mesh::ATTRIBUTE_COLOR, c);
    }
}

/// The world light set the static world vertex colours were last baked with (relight_static_world).
#[derive(Resource, Default)]
pub struct WorldLightKey(pub Vec<i32>, pub Vec<RwLightInst>);

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
            if o.blend_mode == BlendMode::Subtractive {
                subblend_log("object", mat.texture.as_ref().map_or("-", |t| t.name.as_str()));
            }
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
    let lights = RwLights::from_static_world(&s.lights);
    // P37 R13/R14 (default on, GAME_WORLD_MERGE=0 = off): one StandardMaterial per distinct world material key instead of
    // one per sector split (same inputs => identical material => pixel-identical output)
    let share = world_merge();
    let mut shared: HashMap<WorldMatKey, Handle<StandardMaterial>> = HashMap::new();
    for (mi, idx, mesh) in world_to_meshes(w, &lights) {
        let Some(mat) = w.materials.get(mi as usize) else { continue };
        let key = world_mat_key(mat);
        let math = match shared.get(&key) {
            Some(h) if share => h.clone(),
            _ => {
                let tex = mat.texture.as_ref().and_then(|t| {
                    world.resource_scope(|wd, mut cache: Mut<GfxCache>| {
                        let mut images = wd.resource_mut::<Assets<Image>>();
                        cache.texture(&t.name, dict, &mut images)
                    })
                });
                let sm = material_to_standard(mat, tex, true, BlendMode::Normal);
                let h = world.resource_mut::<Assets<StandardMaterial>>().add(sm);
                if share {
                    shared.insert(key, h.clone());
                }
                h
            }
        };
        let mh = world.resource_mut::<Assets<Mesh>>().add(mesh);
        let bounds = w.sectors.get(idx[0] as usize).and_then(|sc| sc.meshes.get(idx[1] as usize)).map(|sm| {
            sm.mesh.positions.iter().fold(WorldSectorBounds([f32::MAX; 3], [f32::MIN; 3]), |b, p| {
                WorldSectorBounds([b.0[0].min(p[0]), b.0[1].min(p[1]), b.0[2].min(p[2])], [b.1[0].max(p[0]), b.1[1].max(p[1]), b.1[2].max(p[2])])
            })
        });
        let mut e = world.spawn((Mesh3d(mh), MeshMaterial3d(math), WorldSectorMesh(idx), Transform::IDENTITY, GlobalTransform::default()));
        if let Some(b) = bounds {
            e.insert(b);
        }
        let e = e.id();
        world.entity_mut(root).add_child(e);
    }
    let q = |x: f32| (x * 255.0).round() as i32;
    let key = lights.lights.iter().flat_map(|l| [l.ty as i32, q(l.color[0]), q(l.color[1]), q(l.color[2]), q(l.at[0]), q(l.at[1]), q(l.at[2]), q(l.pos[0]), q(l.pos[1]), q(l.pos[2])]).collect();
    world.insert_resource(WorldLightKey(key, lights.lights.clone()));
    Some(root)
}

/// P37 R13: everything `material_to_standard(m, tex, true, Normal)` reads from a world material:
/// RwRGBA colour + texture name (the GfxCache texture, incl. its alpha flag, is keyed by name)
type WorldMatKey = ([u8; 4], Option<String>);

fn world_mat_key(m: &RpMaterial) -> WorldMatKey {
    (m.color, m.texture.as_ref().map(|t| t.name.clone()))
}

/// P37 R14: default ON (all-level pixel sweep identical); `GAME_WORLD_MERGE=0` is the escape hatch
fn world_merge() -> bool {
    world_merge_from(std::env::var("GAME_WORLD_MERGE").ok().as_deref())
}

fn world_merge_from(v: Option<&str>) -> bool {
    v.map_or(true, |v| v.trim() != "0")
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
    let color = display_color(((c >> 16) & 0xff) as f32 / 255.0, ((c >> 8) & 0xff) as f32 / 255.0, (c & 0xff) as f32 / 255.0, 1.0);
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
    /// P26 R36: the geometry is prelit with partial vertex alpha (the GS blends with As = vertex *
    /// texel alpha; EP3/L05 dust-devil cone res 0x4a5, prelit alpha 5..115, opaque texture): Blend,
    /// not the alpha-tested Mask(0.5) cut-out path (which discarded the whole cone)
    pub vert_alpha: bool,
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
        DrawMaterialKey { texture: m.texture.as_ref().map(|t| t.to_ascii_lowercase()), color, fade_alpha, blend: d.blend as u8, vert_alpha: false }
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
            // P26 R20: FIX ignores As (vertex / prelit alpha too): AlphaMode::Add multiplied the
            // colour by base * vertex * texel alpha (EP4/L02A searchlight cone res 0x5a4, prelit
            // alpha 0..204, was near invisible) -> Premultiplied with alpha 0 = ONE/ONE, as the
            // 0x68 sprites / particles
            BlendMode::AdditiveFix => ([c[0], c[1], c[2]], AlphaMode::Premultiplied, 0.0),
            BlendMode::Subtractive => ([255 - c[0], 255 - c[1], 255 - c[2]], AlphaMode::Multiply, a),
            BlendMode::Normal if a < 1.0 || tex_alpha || self.vert_alpha || self.fade_alpha.is_some() => ([c[0], c[1], c[2]], AlphaMode::Blend, a),
            BlendMode::Normal if self.texture.is_some() => ([c[0], c[1], c[2]], AlphaMode::Mask(0.5), a),
            BlendMode::Normal => ([c[0], c[1], c[2]], AlphaMode::Opaque, a),
        };
        StandardMaterial {
            base_color: display_color(rgb[0] as f32 / 255.0, rgb[1] as f32 / 255.0, rgb[2] as f32 / 255.0, a),
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
/// P37 R11: true when `a` and `b` carry byte-identical vertex attributes (same set, same data)
fn same_vertex_data(a: &Mesh, b: &Mesh) -> bool {
    a.attributes().count() == b.attributes().count() && b.attributes().all(|(id, v)| a.attribute(id.id).is_some_and(|o| o.get_bytes() == v.get_bytes()))
}

fn mesh_dirty_check() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("GAME_MESH_DIRTY").is_ok_and(|v| v == "1"))
}

/// P37 R10: GAME_PROF sub-timers of [`ApplyDrawList`] (per frame, ms; drained by the game's stage log)
pub static DL_PROF: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub static DL_SUB: std::sync::Mutex<Vec<(&'static str, f64)>> = std::sync::Mutex::new(Vec::new());

pub fn ApplyDrawList<'a>(world: &mut World, draws: &[DrawCmd], clump_of: &dyn Fn(u32) -> Option<(u32, &'a RpClump)>, dict: &RwTexDictionary) {
    let prof = DL_PROF.load(std::sync::atomic::Ordering::Relaxed);
    // (skin, light, upload, entity, hide) ms
    let mut acc = [0f64; 5];
    let tick = |on: bool| on.then(std::time::Instant::now);
    let tock = |a: &mut f64, t: Option<std::time::Instant>| {
        if let Some(t) = t {
            *a += t.elapsed().as_secs_f64() * 1e3;
        }
    };
    if !world.contains_resource::<DrawListState>() {
        world.init_resource::<DrawListState>();
    }
    if !world.contains_resource::<GfxCache>() {
        world.init_resource::<GfxCache>();
    }
    let lights = world.get_resource::<RwLights>().cloned();
    let hud = world.get_resource::<RwHudLights>().cloned();
    world.resource_scope(|world, mut st: Mut<DrawListState>| {
        let mut seen: HashMap<(u32, usize, u32), u32> = HashMap::new();
        let mut used = std::collections::HashSet::new();
        for d in draws {
            let Some((res, clump)) = clump_of(d.owner) else { continue };
            let Some(g) = clump.geometries.get(d.geometry) else { continue };
            // P26 R17: HUD clumps are lit by the HUD world's lights only (RwHudLights)
            let lights = match &hud {
                Some(h) if d.owner >= h.from_owner => Some(&h.lights),
                _ => lights.as_ref(),
            };
            let split = st.split.entry((res, d.geometry)).or_insert_with(|| std::sync::Arc::new(geometry_to_meshes(g))).clone();
            // P37 R6: the skinned positions/normals depend on the draw only, and the CPU lighting on the
            // draw + the material's (ambient, diffuse): compute them once per draw, not once per
            // material split (every split carries the whole vertex list). Same values, less work.
            let mut pn: Option<(Vec<[f32; 3]>, Vec<[f32; 3]>)> = None;
            let mut lit_cache: Vec<((u32, u32), Vec<[f32; 4]>)> = Vec::new();
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
                    let t_s = tick(prof);
                    if pn.is_none() {
                        pn = Some(if skinned {
                            let src = st.skins.entry((res, d.geometry)).or_insert_with(|| {
                                let s = g.skin.as_ref().unwrap();
                                let mt = &g.morph_targets[0];
                                SkinSrc { verts: mt.verts.clone(), normals: mt.normals.clone(), indices: s.indices.clone(), weights: s.weights.clone() }
                            });
                            skin_vertices(src, d.skin.as_ref().unwrap())
                        } else {
                            let mt = &g.morph_targets[0];
                            (mt.verts.clone(), mt.normals.clone())
                        });
                    }
                    let (p, n) = pn.clone().unwrap();
                    tock(&mut acc[0], t_s);
                    let t_l = tick(prof);
                    if let Some(l) = lights {
                        let mat = &g.materials[mi as usize];
                        let ck = (mat.ambient.to_bits(), mat.diffuse.to_bits());
                        let c = match lit_cache.iter().find(|(k, _)| *k == ck) {
                            Some((_, c)) => c.clone(),
                            None => {
                                let c = l.LightVertices(&d.ltm, &p, &n, &g.prelit, d.geometry_flags, mat.ambient, mat.diffuse);
                                lit_cache.push((ck, c.clone()));
                                c
                            }
                        };
                        m.insert_attribute(Mesh::ATTRIBUTE_COLOR, c);
                    }
                    tock(&mut acc[1], t_l);
                    let t_u = tick(prof);
                    let nv = p.len();
                    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, p);
                    if m.attribute(Mesh::ATTRIBUTE_NORMAL).is_some() && n.len() == nv {
                        m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, n);
                    }
                    m.remove_attribute(Mesh::ATTRIBUTE_JOINT_INDEX);
                    m.remove_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT);
                    let prev = st.entities.get(&id).and_then(|e| e.1.clone());
                    let mut meshes = world.resource_mut::<Assets<Mesh>>();
                    let h = match prev {
                        // R11: dirty check: the re-skin/relight produced the same vertex data as the
                        // uploaded mesh: keep it (no asset Modified event, no GPU re-upload).
                        // Opt-in (GAME_MESH_DIRTY=1): R11 A/B was pixel-identical but not a measurable win (P37.md R11).
                        Some(h) if mesh_dirty_check() && meshes.get(h.id()).is_some_and(|o| same_vertex_data(o, &m)) => h,
                        Some(h) => {
                            let _ = meshes.insert(h.id(), m);
                            h
                        }
                        None => meshes.add(m),
                    };
                    tock(&mut acc[2], t_u);
                    h
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
                let mut mk = DrawMaterialKey::from_draw(d, mi as usize);
                // GAME_VERTALPHA=0: debug opt-out (pre-R36 Mask path)
                mk.vert_alpha = d.geometry_flags & 0x8 != 0 && g.prelit.iter().any(|c| c[3] < 250) && !std::env::var("GAME_VERTALPHA").is_ok_and(|v| v == "0");
                let unlit = lights.is_some();
                if mk.blend == 3 {
                    subblend_log("object", mk.texture.as_deref().unwrap_or("-"));
                }
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
                let t_e = tick(prof);
                let tr = Transform::from_matrix(rw_mat(&d.ltm));
                let e = match st.entities.get(&id) {
                    Some(&(e, _)) => {
                        // P37 R4: write only what changed. Re-inserting identical components every frame
                        // marked every draw-list entity changed, so Bevy re-extracted / re-specialized all
                        // of them each frame (render-world cost on object-heavy levels).
                        let mut em = world.entity_mut(e);
                        if em.get::<Mesh3d>().is_none_or(|m| m.0 != mh) {
                            em.insert(Mesh3d(mh.clone()));
                        }
                        if em.get::<MeshMaterial3d<StandardMaterial>>().is_none_or(|m| m.0 != math) {
                            em.insert(MeshMaterial3d(math));
                        }
                        if em.get::<Transform>() != Some(&tr) {
                            em.insert(tr);
                        }
                        if em.get::<Visibility>() != Some(&Visibility::Visible) {
                            em.insert(Visibility::Visible);
                        }
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
                tock(&mut acc[3], t_e);
            }
        }
        let t_h = tick(prof);
        for (id, (e, _)) in st.entities.iter() {
            if !used.contains(id) {
                if let Ok(mut em) = world.get_entity_mut(*e)
                    && em.get::<Visibility>() != Some(&Visibility::Hidden)
                {
                    em.insert(Visibility::Hidden);
                }
            }
        }
        st.visible = used.len();
        tock(&mut acc[4], t_h);
    });
    if prof {
        DL_SUB.lock().unwrap().extend([("dl:skin", acc[0]), ("dl:light", acc[1]), ("dl:upload", acc[2]), ("dl:entity", acc[3]), ("dl:hide", acc[4])]);
    }
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
/// P26 R21: the GS blends (and fogs, and modulates) in frame-buffer = display space. Display-space
/// mode feeds every texel / vertex / material / fog colour to Bevy unconverted (textures
/// Rgba8Unorm, no sRGB decode), so Bevy's "linear" blending is the GS's display-space blending;
/// the game binary then decodes the frame once at the end (game_fx::DisplayDecode) so the sRGB
/// target re-encodes it to the original values. Off (old linear-light path) for the viewer /
/// unit tests; the game turns it on unless GAME_LINEAR_BLEND is set.
static DISPLAY_SPACE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn set_display_space(on: bool) {
    DISPLAY_SPACE.store(on, std::sync::atomic::Ordering::Relaxed);
}
pub fn display_space() -> bool {
    DISPLAY_SPACE.load(std::sync::atomic::Ordering::Relaxed)
}
/// display-space 0..1 colour -> Bevy Color (unconverted in display-space mode, sRGB otherwise)
pub fn display_color(r: f32, g: f32, b: f32, a: f32) -> Color {
    if display_space() { Color::linear_rgba(r, g, b, a) } else { Color::srgba(r, g, b, a) }
}
/// RwTexture image format: raw display values in display-space mode, sRGB-decoded otherwise
pub fn tex_format() -> TextureFormat {
    if display_space() { TextureFormat::Rgba8Unorm } else { TextureFormat::Rgba8UnormSrgb }
}

pub fn srgb_to_linear(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if display_space() {
        return c;
    }
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

/// RwRGBA 0..255 (generic geometry stream, RpGeometry prelit) -> linear RGBA. The PS2 RW pipelines
/// upload these as GS colours scaled 255 -> 0x80 (= 1.0), so 255 is full intensity.
pub fn rwrgba_to_linear(c: [u8; 4]) -> [f32; 4] {
    [srgb_to_linear(c[0] as f32 / 255.0), srgb_to_linear(c[1] as f32 / 255.0), srgb_to_linear(c[2] as f32 / 255.0), c[3] as f32 / 255.0]
}

/// PS2 GS colour (0x80 = 1.0; particle colours) -> linear RGBA. P26 R40: colour values above
/// 1.0 are kept (the GS clamps the modulated texel Cf*Ct>>7, TEX0.TFX MODULATE, not Cf: the
/// snack-box glow sprite colour 0x5f,0,0xbf doubles the blue of its texel); alpha still saturates.
static GSCLAMP: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
pub fn gs_to_linear(c: [u8; 4]) -> [f32; 4] {
    let ch = |v: u8| {
        let x = v as f32 / 128.0;
        if x <= 1.0 || *GSCLAMP.get_or_init(|| std::env::var("GAME_GSCLAMP").is_ok_and(|v| v == "1")) { srgb_to_linear(x) } else if display_space() { x } else { ((x + 0.055) / 1.055).powf(2.4) }
    };
    [ch(c[0]), ch(c[1]), ch(c[2]), (c[3] as f32 / 128.0).min(1.0)]
}

/// P26 R41: [`gs_to_linear`] for a GS ALPHA blend. The GS clamps Cs = Cf*Ct>>7 to 1.0 before
/// blending; the port has no per-pixel clamp (StandardMaterial). Only the pure adds 0x68 / 0x29
/// (Cs*FIX+Cd, FIX = 0x80) are exact with an unclamped Cf (the frame-buffer write clamps the sum
/// as COLCLAMP does); every other mode (0x44 uses As, 0xa1 inverts Cs, default Add) keeps the
/// Cf <= 1.0 clamp so nothing over-brightens or goes negative.
pub fn gs_to_linear_blend(c: [u8; 4], blend: u32) -> [f32; 4] {
    match blend & 0xff {
        0x29 | 0x68 => gs_to_linear(c),
        _ => gs_to_linear(c.map(|v| v.min(0x80))).map(|v| v.min(1.0)),
    }
}

/// P26 R17: the lights of the HUD world (EFHudMgr_SP +0x10c0, its own RpWorld). EFHudMgr_SP::Update
/// renders the EFHud3DWindow clumps with a camera in that world after EnableHudLights; only the
/// HUD lights (EFHudMgr_SP::AddLight: APIlight_CreateLight with bit 31, EFLight kind 0x40000) are
/// in it, so the level's lights never reach the HUD 3D objects (the compass dial, ...). Draw-list
/// owners >= `from_owner` are HUD objects.
#[derive(Resource, Clone, Default)]
pub struct RwHudLights {
    pub from_owner: u32,
    pub lights: RwLights,
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

    /// P26 R17: the static lights as the light manager has them now (EFStatic3dObj::ReadLights:
    /// the rendered RpLight of static light i is EFLight (i + 0x800000) | 0x12000000, colour /
    /// flags / radius / cone copied at load, then changed by the logic). Lights not in the world
    /// (IsVisible) are skipped; uids missing from the manager keep the streamed values. All flags:
    /// filter with `only(1)` (atomics) / `only(2)` (world).
    pub fn from_static_rt(ls: &[super::EFStaticLight], rt: &[crate::engine::light::EFLight]) -> Self {
        let mut lights = Vec::new();
        for l in ls {
            let m = &l.matrix;
            let mut i = Self::inst(&l.light, [m[6], m[7], m[8]], [m[9], m[10], m[11]]);
            if let Some(e) = rt.iter().find(|e| e.uid == l.uid) {
                let Some(r) = e.rp.as_ref().filter(|_| e.IsVisible()) else { continue };
                (i.ty, i.flags, i.color, i.radius, i.minus_cos_angle) = (r.ty, r.flags, [r.color[0], r.color[1], r.color[2]], r.radius, r.minus_cos_angle);
            }
            lights.push(i);
        }
        RwLights { lights }
    }

    /// The lights with any of `flags` (rpLIGHTLIGHTATOMICS 1 / rpLIGHTLIGHTWORLD 2).
    pub fn only(&self, flags: u8) -> Self {
        RwLights { lights: self.lights.iter().filter(|l| l.flags & flags != 0).cloned().collect() }
    }

    /// The static lights with rpLIGHTLIGHTWORLD (2): the ones that light world sectors.
    pub fn from_static_world(ls: &[super::EFStaticLight]) -> Self {
        let lights = ls
            .iter()
            .filter(|l| l.light.flags & 2 != 0)
            .map(|l| {
                let m = &l.matrix;
                Self::inst(&l.light, [m[6], m[7], m[8]], [m[9], m[10], m[11]])
            })
            .collect();
        RwLights { lights }
    }

    /// P26 R16: + the EFLightMgr dynamic world lights (APIlight_CreateLight, kind 0x10000) that
    /// are in the RpWorld (SetVisible 1). RpWorldAddLight puts them in the world's global
    /// (ambient/directional) or sector-local (point/spot) lists, which
    /// RpAtomicPS2AllLightingSetupFunc walks for every lit atomic just like the static ones
    /// (rpLIGHTLIGHTATOMICS checked there; R17: all flags kept, filter with `only`). EP1/L04 (ligth_l04) and EP1/L03B (level03b_*) light
    /// their characters only this way (0 static lights in EP1/L04: Scooby was black).
    pub fn with_dynamic(&self, ls: &[crate::engine::light::EFLight]) -> Self {
        let mut out = self.clone();
        for l in ls.iter().filter(|l| l.kind == 0x10000 && l.IsVisible()) {
            let Some(r) = l.rp.as_ref() else { continue };
            let (at, pos) = l.frame.map_or(([0.0; 3], [0.0; 3]), |f| (f.at, f.pos));
            out.lights.push(RwLightInst { ty: r.ty, flags: r.flags, color: [r.color[0], r.color[1], r.color[2]], radius: r.radius, minus_cos_angle: r.minus_cos_angle, at, pos });
        }
        out
    }

    /// P26 R17: the HUD world's lights: EFLights of kind 0x40000 added by EFHudMgr_SP::AddLight.
    pub fn from_hud(ls: &[crate::engine::light::EFLight]) -> Self {
        let lights = ls
            .iter()
            .filter(|l| l.kind == 0x40000 && l.in_hud)
            .filter_map(|l| {
                let r = l.rp.as_ref()?;
                let (at, pos) = l.frame.map_or(([0.0; 3], [0.0; 3]), |f| (f.at, f.pos));
                Some(RwLightInst { ty: r.ty, flags: r.flags, color: [r.color[0], r.color[1], r.color[2]], radius: r.radius, minus_cos_angle: r.minus_cos_angle, at, pos })
            })
            .filter(|l| l.flags & 1 != 0)
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

/// P40 R3 (B#9): the app draws GS ALPHA 0xa1 ((Cd-Cs)*FIX, FIX 0x80 = 1.0) batches with a real
/// ReverseSubtract blend (game: game_fx::GsSubtract swapped in by `sub_particles`). Off (viewer /
/// unit tests without that material): the old Multiply-by-(1-Cs) approximation.
static SUB_BLEND_HW: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn set_sub_blend_hw(on: bool) {
    SUB_BLEND_HW.store(on, std::sync::atomic::Ordering::Relaxed);
}
pub fn sub_blend_hw() -> bool {
    SUB_BLEND_HW.load(std::sync::atomic::Ordering::Relaxed)
}

/// P40 R4: the game's frame number for the GAME_SUBBLEND_LOG=frames detail lines (set by the game loop).
pub static SUBBLEND_FRAME: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// debug (P40 R3, B#9): GAME_SUBBLEND_LOG=1 logs (once per source + texture) every draw that uses
/// the GS subtractive blend ALPHA 0xa1 ((Cd-Cs)*FIX): particles, sprites, transparent objects.
pub fn subblend_log(src: &str, tex: &str) {
    use std::sync::{Mutex, OnceLock};
    static SEEN: OnceLock<Mutex<std::collections::HashSet<String>>> = OnceLock::new();
    if std::env::var_os("GAME_SUBBLEND_LOG").is_none() {
        return;
    }
    let k = format!("{src} {tex}");
    if SEEN.get_or_init(Default::default).lock().unwrap().insert(k.clone()) {
        eprintln!("game: subblend 0xa1 {k}");
    }
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
    // P33 R6 debug: GAME_BLEND_LOG=1 reports particle batches with the subtractive GS mode 0xa1 (once)
    if blend & 0xff == 0xa1 && std::env::var_os("GAME_BLEND_LOG").is_some() {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| eprintln!("P33 blend 0xa1 particle batch"));
    }
    match blend & 0xff {
        0x44 => AlphaMode::Blend,
        // P40 R3 (B#9): with a hardware subtract material the batch goes to the transparent phase
        // as Blend; the game swaps the pipeline's blend to ReverseSubtract (game_fx::GsSubtract)
        0xa1 if sub_blend_hw() => AlphaMode::Blend,
        0xa1 => AlphaMode::Multiply,
        0x29 | 0x68 => AlphaMode::Premultiplied,
        _ => AlphaMode::Add,
    }
}

/// P40 R8: GAME_SPRITE_VU1=1 = the VU1TestCode sprite shape (sprite::Vu1SpriteCorners: corners `size` from the
/// centre, side size*sqrt2, turned 45 deg). Default off (the pre-R7 axis-aligned square of half-side `size`) until a
/// PCSX2 frame confirms it (P40.md R8).
fn sprite_vu1() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("GAME_SPRITE_VU1").is_some_and(|v| v == "1"))
}

/// Camera-facing quads of one batch (replacement of the PLEmitter::RenderBatch VU1 expansion).
/// `right`/`up` are the world-space screen axes of the camera (RW camera: screen-right =
/// -LTM.right, screen-up = LTM.up). Sprite: centre pos, half-size `size`, rotated by
/// rot_index*2π/128 in the screen plane; colour 0x80 = 1.0. Quads (3D types) are world space.
pub fn particle_batch_mesh(b: &PLDrawBatch, right: [f32; 3], up: [f32; 3]) -> Mesh {
    particle_batch_mesh_shape(b, right, up, sprite_vu1())
}

/// [`particle_batch_mesh`] with the sprite shape chosen: `vu1` = VU1TestCode corners, else the axis-aligned square
pub fn particle_batch_mesh_shape(b: &PLDrawBatch, right: [f32; 3], up: [f32; 3], vu1: bool) -> Mesh {
    let (mut pos, mut uv, mut col, mut idx) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let tr = |c: [u8; 4]| {
        let mut l = gs_to_linear_blend(c, b.blend);
        match b.blend & 0xff {
            // P40 R3 (B#9): hardware subtract = Cd - Cs exactly (Cs = MODULATE texel * vertex colour)
            0xa1 if sub_blend_hw() => l[3] = 1.0,
            0xa1 => l = [1.0 - l[0], 1.0 - l[1], 1.0 - l[2], 1.0],
            0x44 => {}
            0x29 | 0x68 => l[3] = 0.0,
            _ => l[3] = 1.0,
        }
        l
    };
    match &b.prims {
        PLPrims::Sprites(v) if vu1 => {
            // P40 R7: the VU1TestCode corners (sprite::Vu1SpriteCorners; R = LTM.right = -right) and its
            // 4-vertex triangle strip (0,1,2),(1,2,3)
            let r = [-right[0], -right[1], -right[2]];
            for s in v {
                let base = pos.len() as u32;
                let c = crate::engine::gfx::sprite::Vu1SpriteCorners(r, up, s.pos, s.size, s.rot_index);
                for (k, st) in [[0.0f32, 0.0f32], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]].into_iter().enumerate() {
                    pos.push(c[k]);
                    uv.push(st);
                    col.push(tr(s.rgba));
                }
                idx.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
            }
        }
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
                if b.blend & 0xff == 0xa1 {
                    subblend_log(if bb { "billboard" } else { "particle" }, &mk.0);
                    if std::env::var_os("GAME_SUBBLEND_LOG").is_some_and(|v| v == "frames") {
                        // P40 R4: per-frame detail (frame, sprite/quad count, max vertex colour) to find emitting frames
                        let (n, mx) = match &b.prims {
                            crate::engine::particle::PLPrims::Sprites(v) => (v.len(), v.iter().map(|s| s.rgba[0].max(s.rgba[1]).max(s.rgba[2])).max().unwrap_or(0)),
                            crate::engine::particle::PLPrims::Quads(v) => (v.len(), v.iter().map(|q| q.rgba[0].max(q.rgba[1]).max(q.rgba[2])).max().unwrap_or(0)),
                        };
                        eprintln!("game: subblend f{} {} n {n} max_rgb {mx:#x}", SUBBLEND_FRAME.load(std::sync::atomic::Ordering::Relaxed), mk.0);
                    }
                }
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
                if let Ok(mut em) = world.get_entity_mut(*e)
                    && em.get::<Visibility>() != Some(&Visibility::Hidden)
                {
                    em.insert(Visibility::Hidden);
                }
            }
        }
        st.visible = used.len();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(pos: [f32; 3], radius: f32) -> RwLightInst {
        RwLightInst { ty: 0x80, flags: 2, color: [1.0, 0.5, 0.2], radius, minus_cos_angle: 0.0, at: [0.0, 0.0, 1.0], pos }
    }
    fn key(ls: &[RwLightInst]) -> Vec<i32> {
        let q = |x: f32| (x * 255.0).round() as i32;
        ls.iter().flat_map(|l| [l.ty as i32, q(l.color[0]), q(l.color[1]), q(l.color[2]), q(l.at[0]), q(l.at[1]), q(l.at[2]), q(l.pos[0]), q(l.pos[1]), q(l.pos[2])]).collect()
    }

    // P37 R13: world material dedupe key = exactly the material_to_standard inputs (colour, texture name)
    #[test]
    fn p37_world_mat_key() {
        use crate::engine::rw::geometry::RpTextureRef;
        let mk = |c: [u8; 4], t: Option<&str>, f: u32| RpMaterial {
            flags: 0,
            color: c,
            ambient: 1.0,
            specular: 0.0,
            diffuse: 1.0,
            texture: t.map(|n| RpTextureRef { filter_addressing: f, name: n.into(), ..Default::default() }),
            ext: Default::default(),
        };
        let a = world_mat_key(&mk([255; 4], Some("wall"), 1));
        assert_eq!(a, world_mat_key(&mk([255; 4], Some("wall"), 2)));
        assert_ne!(a, world_mat_key(&mk([255; 4], Some("floor"), 1)));
        assert_ne!(a, world_mat_key(&mk([255, 255, 255, 128], Some("wall"), 1)));
        assert_ne!(a, world_mat_key(&mk([255; 4], None, 1)));
        let keys: std::collections::HashSet<_> = [mk([1; 4], Some("a"), 0), mk([1; 4], Some("a"), 0), mk([1; 4], None, 0)].iter().map(world_mat_key).collect();
        assert_eq!(keys.len(), 2);
        // R14: default on, "0" is the escape hatch
        assert!(world_merge_from(None) && world_merge_from(Some("1")));
        assert!(!world_merge_from(Some("0")));
    }

    // P37 R11: the mesh dirty check skips only byte-identical vertex data
    #[test]
    fn p37_same_vertex_data() {
        let mk = |p: [f32; 3], c: Option<[f32; 4]>| {
            let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
            m.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![p, [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);
            if let Some(c) = c {
                m.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![c; 3]);
            }
            m
        };
        let a = mk([0.0; 3], Some([1.0; 4]));
        assert!(same_vertex_data(&a, &mk([0.0; 3], Some([1.0; 4]))));
        assert!(!same_vertex_data(&a, &mk([0.0, 0.0, 1e-6], Some([1.0; 4]))));
        assert!(!same_vertex_data(&a, &mk([0.0; 3], Some([1.0, 1.0, 0.99, 1.0]))));
        assert!(!same_vertex_data(&a, &mk([0.0; 3], None)));
        assert!(!same_vertex_data(&mk([0.0; 3], None), &a));
    }

    // P37 R2: a moving point light re-bakes only the sectors its old/new sphere reaches; anything else = all
    #[test]
    fn p37_relight_culling() {
        let amb = RwLightInst { ty: 2, ..pt([0.0; 3], 0.0) };
        let old = vec![amb.clone(), pt([0.0, 0.0, 0.0], 5.0)];
        let prev = WorldLightKey(key(&old), old.clone());
        let moved = vec![amb.clone(), pt([1.0, 0.0, 0.0], 5.0)];
        let sp = relight_spheres(&prev, &key(&moved), &moved).expect("point light move is local");
        assert_eq!(sp, vec![([0.0, 0.0, 0.0], 5.0), ([1.0, 0.0, 0.0], 5.0)]);
        // a sector box 3 units away is reached, one 10 units away is not
        assert!(sp.iter().any(|&(c, r)| sphere_meets_box(c, r, [4.0, -1.0, -1.0], [6.0, 1.0, 1.0])));
        assert!(!sp.iter().any(|&(c, r)| sphere_meets_box(c, r, [10.0, -1.0, -1.0], [12.0, 1.0, 1.0])));
        // the ambient light changed colour -> every sector
        let recol = vec![RwLightInst { color: [0.2, 0.2, 0.2], ..amb.clone() }, old[1].clone()];
        assert!(relight_spheres(&prev, &key(&recol), &recol).is_none());
        // a light appeared -> every sector
        let more = vec![amb, old[1].clone(), pt([9.0, 9.0, 9.0], 1.0)];
        assert!(relight_spheres(&prev, &key(&more), &more).is_none());
    }
}

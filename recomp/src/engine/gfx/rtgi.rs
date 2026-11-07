//! P22: opt-in raytraced global illumination (bevy_solari 0.19), cargo feature `solari`.
//! Runtime opt-in: `GAME_RTGI=1` (game + viewer); toggle in-window with F9.
//! Without the env var nothing here is added (SolariPlugins switches the default opaque renderer
//! to deferred, so it must not be registered on the normal path).
//! Solari only accepts meshes with exactly POSITION/NORMAL/UV_0/TANGENT + U32 indices, while the
//! port meshes carry the RW prelit ATTRIBUTE_COLOR, so a stripped copy is built per mesh for the
//! BLAS (`RaytracingMesh3d`) and the original stays on `Mesh3d` for the raster G-buffer.
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::CameraMainTextureUsages;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::TextureUsages;
use bevy::render::renderer::RenderDevice;
use bevy::render::RenderApp;
use bevy_solari::prelude::*;

/// `settings.toml` next to controls.toml (cwd, else next to the exe when only that one exists).
pub fn settings_path() -> std::path::PathBuf {
    let cwd = std::path::PathBuf::from("settings.toml");
    match std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("settings.toml"))) {
        Some(e) if !cwd.exists() && e.exists() => e,
        _ => cwd,
    }
}

/// "Global Ray Tracing: On/Off" = `global_ray_tracing = true|false` in settings.toml (default off).
pub fn setting() -> bool {
    std::fs::read_to_string(settings_path()).ok().is_some_and(|t| {
        t.lines().filter_map(|l| l.split_once('=')).any(|(k, v)| k.trim() == "global_ray_tracing" && v.trim() == "true")
    })
}

/// Persist the setting (other lines of settings.toml are kept).
pub fn save_setting(on: bool) {
    let p = settings_path();
    let old = std::fs::read_to_string(&p).unwrap_or_default();
    let mut out: Vec<String> = old.lines().filter(|l| l.split_once('=').is_none_or(|(k, _)| k.trim() != "global_ray_tracing")).map(String::from).collect();
    out.push(format!("global_ray_tracing = {on}"));
    if let Err(e) = std::fs::write(&p, out.join("
") + "
") {
        warn!("rtgi: cannot write {}: {e}", p.display());
    }
}

/// Dev override `GAME_RTGI=1|0`, else the settings.toml option.
pub fn requested() -> bool {
    match std::env::var("GAME_RTGI") {
        Ok(v) => !v.is_empty() && v != "0",
        Err(_) => setting(),
    }
}

/// Options-panel row "Global Ray Tracing": handles the R key and returns the value to show
/// (None = hidden: GPU without ray query). Without the plugin loaded (setting off at start) the
/// row edits settings.toml only (`restart` = the change applies on the next start).
pub fn options_row(world: &mut World) -> Option<(bool, bool)> {
    let r = world.get_resource::<ButtonInput<KeyCode>>().is_some_and(|k| k.just_pressed(KeyCode::KeyR));
    if let Some(mut s) = world.get_resource_mut::<RtgiState>() {
        if !s.supported {
            return None;
        }
        if r {
            s.req = true;
        }
        return Some((s.on != r, false));
    }
    let mut on = setting();
    if r {
        on = !on;
        save_setting(on);
    }
    Some((on, on))
}

/// sun illuminance (lux), `GAME_RTGI_SUN` override (calibrated so the mean luminance matches raster)
fn sun_lux() -> f32 {
    std::env::var("GAME_RTGI_SUN").ok().and_then(|v| v.parse().ok()).unwrap_or(SUN_LUX)
}
const SUN_LUX: f32 = 5_000.0;

/// Marker for the camera that gets `SolariLighting` (the bins insert it on the 3D game camera).
#[derive(Component)]
pub struct RtgiCamera;


#[derive(Resource, Default)]
struct RtgiState {
    on: bool,
    supported: bool,
    rt_meshes: HashMap<AssetId<Mesh>, Option<Handle<Mesh>>>,
    /// materials switched to matte lit for the RT path -> original material
    mats: HashMap<AssetId<StandardMaterial>, StandardMaterial>,
    /// matte copy -> original material id, original id -> matte copy
    orig: HashMap<AssetId<StandardMaterial>, AssetId<StandardMaterial>>,
    clones: HashMap<AssetId<StandardMaterial>, Handle<StandardMaterial>>,
    /// toggle requested by the options-panel row (R key)
    req: bool,
}

pub struct RtgiPlugin;

impl Plugin for RtgiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((SolariPlugins, super::rtgi_hybrid::RtgiHybridPlugin))
            .insert_resource(RtgiState { on: true, ..default() })
            .add_systems(Startup, spawn_sun)
            .add_systems(PostUpdate, toggle)
            .add_systems(Last, attach_rt_meshes);
    }

    fn finish(&self, app: &mut App) {
        let ok = app
            .get_sub_app(RenderApp)
            .and_then(|r| r.world().get_resource::<RenderDevice>())
            .is_some_and(|d| d.features().contains(SolariPlugins::required_wgpu_features()));
        if !ok {
            warn!("rtgi: GPU lacks ray query / binding array features; running the raster path");
        } else {
            info!("rtgi: Solari raytraced lighting enabled (F9 toggles)");
        }
        let mut s = app.world_mut().resource_mut::<RtgiState>();
        s.supported = ok;
        s.on = ok;
    }
}

/// Camera components required by `SolariLighting` (Msaa off, storage-bindable main texture, HDR).
/// Cameras sharing the render target must be HDR / Msaa::Off too: use `companion` for those.
pub fn camera() -> impl Bundle {
    (
        RtgiCamera,
        Msaa::Off,
        CameraMainTextureUsages::default().with(TextureUsages::STORAGE_BINDING),
        bevy::camera::Hdr,
        // neutral (same as the raster path), no bloom
        Tonemapping::None,
    )
}

/// debug bisect switches: GAME_RTGI_DBG=nosolari,nomesh,nomat
pub fn dbg(k: &str) -> bool {
    std::env::var("GAME_RTGI_DBG").is_ok_and(|v| v.split(',').any(|x| x == k))
}

pub fn companion() -> impl Bundle {
    // must match the RT camera's main texture (HDR, Msaa off, STORAGE_BINDING usage): a camera with a
    // different main texture gets its own (empty) one and its ClearColorConfig::None pass blacks the target
    (Msaa::Off, bevy::camera::Hdr, CameraMainTextureUsages::default().with(TextureUsages::STORAGE_BINDING))
}

fn spawn_sun(mut commands: Commands) {
    // the levels have no real-time lights (lighting is baked into the prelit colours): a soft sun
    commands.spawn((
        DirectionalLight { illuminance: sun_lux(), shadow_maps_enabled: false, ..default() },
        Transform::from_xyz(0.3, 1.0, 0.4).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn toggle(keys: Option<Res<ButtonInput<KeyCode>>>, mut s: ResMut<RtgiState>, mut mats: ResMut<Assets<StandardMaterial>>, mut commands: Commands, cams: Query<(Entity, Has<SolariLighting>), With<RtgiCamera>>) {
    if s.supported && (keys.is_some_and(|k| k.just_pressed(KeyCode::F9)) || std::mem::take(&mut s.req)) {
        s.on = !s.on;
        info!("rtgi: {}", if s.on { "on" } else { "off" });
        save_setting(s.on);
        let on = s.on;
        for (id, orig) in s.mats.iter() {
            if let Some(mut m) = mats.get_mut(*id) {
                if on { matte(&mut m) } else { *m = orig.clone() }
            }
        }
    }
    for (e, has) in &cams {
        if s.on && !has && !dbg("nosolari") {
            if dbg("nohybrid") { commands.entity(e).insert(SolariLighting::default()); } else { commands.entity(e).insert((SolariLighting::default(), super::rtgi_hybrid::RtgiHybrid::default())); }
        } else if !s.on && has {
            commands.entity(e).remove::<(SolariLighting, super::rtgi_hybrid::RtgiHybrid)>();
        }
    }
}

/// Art-style option (b): the G-buffer albedo already is texture x prelit vertex colour (the raster
/// mesh keeps ATTRIBUTE_COLOR); make it a matte lit surface (no specular) with a small emissive lift
/// so the baked dark corners don't go black. Fog/alpha/unlit-layer materials are untouched.
fn matte(m: &mut StandardMaterial) {
    m.unlit = false;
    m.perceptual_roughness = 1.0;
    m.metallic = 0.0;
    m.reflectance = 0.0;
    let lift = std::env::var("GAME_RTGI_LIFT").ok().and_then(|v| v.parse().ok()).unwrap_or(EMISSIVE_LIFT);
    let c = m.base_color.to_linear() * lift;
    m.emissive = LinearRgba::rgb(c.red, c.green, c.blue);
}
const EMISSIVE_LIFT: f32 = 0.01;

/// Stripped copy for the BLAS: POSITION/NORMAL/UV_0/TANGENT + U32 indices (normals/tangents generated
/// when missing). None for non-triangle-list or unindexed meshes.
pub fn rt_mesh(m: &Mesh) -> Option<Mesh> {
    if m.primitive_topology() != PrimitiveTopology::TriangleList {
        return None;
    }
    let pos = m.attribute(Mesh::ATTRIBUTE_POSITION)?.clone();
    let n = pos.len();
    let idx: Vec<u32> = match m.indices()? {
        Indices::U32(v) => v.clone(),
        Indices::U16(v) => v.iter().map(|&i| i as u32).collect(),
    };
    if n == 0 || idx.len() < 3 || idx.iter().any(|&i| i as usize >= n) {
        return None;
    }
    let mut r = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    r.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    r.insert_indices(Indices::U32(idx));
    match m.attribute(Mesh::ATTRIBUTE_NORMAL) {
        Some(v @ VertexAttributeValues::Float32x3(_)) => r.insert_attribute(Mesh::ATTRIBUTE_NORMAL, v.clone()),
        _ => r.compute_smooth_normals(),
    }
    let uv = match m.attribute(Mesh::ATTRIBUTE_UV_0) {
        Some(v @ VertexAttributeValues::Float32x2(_)) => v.clone(),
        _ => VertexAttributeValues::Float32x2(vec![[0.0, 0.0]; n]),
    };
    r.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    if r.generate_tangents().is_err() {
        r.insert_attribute(Mesh::ATTRIBUTE_TANGENT, vec![[1.0f32, 0.0, 0.0, 1.0]; n]);
    }
    Some(r)
}

/// World (layer 0) opaque/masked StandardMaterial meshes get a `RaytracingMesh3d` and a private matte
/// copy of their material (the adapter caches materials by key, so HUD clumps share them: mutating the
/// shared one turned the HUD portrait black). Blended meshes and the HUD/overlay layers keep the
/// original material, forced to forward (no deferred prepass on those cameras). Runs in `Last` because
/// the adapter re-inserts Mesh3d/MeshMaterial3d every frame. Converted once per mesh asset
/// (CPU-skinned meshes keep their first-frame BLAS).
fn attach_rt_meshes(
    mut commands: Commands,
    mut s: ResMut<RtgiState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    q: Query<(Entity, &Mesh3d, &MeshMaterial3d<StandardMaterial>, Option<&RenderLayers>, Has<RaytracingMesh3d>, Has<bevy::camera::visibility::NoFrustumCulling>), Or<(Changed<Mesh3d>, Changed<MeshMaterial3d<StandardMaterial>>)>>,
) {
    if !s.supported || dbg("nomesh") {
        return;
    }
    for (e, mh, ma, layers, has_rt, skinned) in &q {
        let mid = ma.0.id();
        let own = s.mats.contains_key(&mid);
        let orig_id = if own { s.orig.get(&mid).copied().unwrap_or(mid) } else { mid };
        let Some(orig) = mats.get(orig_id).cloned() else { continue };
        let world_layer = layers.is_none_or(|l| l.iter().all(|i| i == 0));
        // CPU-skinned characters (adapter marks them NoFrustumCulling) stay raster-only: their BLAS would
        // keep the first pose (ghost shadows); they pass through the hybrid pass with the original look
        if skinned || !world_layer || !matches!(orig.alpha_mode, AlphaMode::Opaque | AlphaMode::Mask(_)) {
            if has_rt {
                commands.entity(e).remove::<RaytracingMesh3d>();
            }
            // forward = no G-buffer write, so the hybrid pass leaves those pixels as the original raster
            if (skinned || !world_layer) && orig.opaque_render_method != bevy::material::OpaqueRendererMethod::Forward {
                if let Some(mut m) = mats.get_mut(orig_id) {
                    m.opaque_render_method = bevy::material::OpaqueRendererMethod::Forward;
                }
            }
            continue;
        }
        let id = mh.0.id();
        let h = match s.rt_meshes.get(&id) {
            Some(h) => h.clone(),
            None => {
                let Some(m) = meshes.get(&mh.0) else { continue };
                let h = rt_mesh(m).map(|r| meshes.add(r));
                s.rt_meshes.insert(id, h.clone());
                h
            }
        };
        let Some(h) = h else { continue };
        if own {
            commands.entity(e).insert(RaytracingMesh3d(h));
            continue;
        }
        let clone = match s.clones.get(&orig_id) {
            Some(c) => c.clone(),
            None => {
                let mut m = orig.clone();
                if s.on && !dbg("nomat") {
                    matte(&mut m);
                }
                let c = mats.add(m);
                s.mats.insert(c.id(), orig);
                s.orig.insert(c.id(), orig_id);
                s.clones.insert(orig_id, c.clone());
                c
            }
        };
        commands.entity(e).insert((MeshMaterial3d(clone), RaytracingMesh3d(h)));
    }
}

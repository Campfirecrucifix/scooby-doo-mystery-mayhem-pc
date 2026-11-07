//! P22 R7 hybrid look: after the opaque pass of the Solari camera, every G-buffer pixel is rewritten as
//! `A * clamp(lum(RT) / lum(A) / REF, 0.6, 1.3)` where A = G-buffer base colour (= texture x baked prelit
//! vertex colour, i.e. the original raster pixel) and RT = Solari's lit result. The palette and the
//! baked lighting composition stay the original's; RT only modulates intensity (contact shadows, bounce).
//! Pixels without G-buffer data (sky, forward/HUD/blended) pass through untouched.
use bevy::core_pipeline::core_3d::{main_opaque_pass_3d, main_transparent_pass_3d};
use bevy::core_pipeline::prepass::ViewPrepassTextures;
use bevy::core_pipeline::schedule::{Core3d, Core3dSystems};
use bevy::core_pipeline::FullscreenShader;
use bevy::prelude::*;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy::render::render_resource::binding_types::texture_2d;
use bevy::render::render_resource::*;
use bevy::render::renderer::{RenderContext, ViewQuery};
use bevy::render::view::ViewTarget;
use bevy::render::{RenderApp, RenderStartup};
use bevy::shader::Shader;

const SRC: &str = r#"
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var gbuf: texture_2d<u32>;
@group(0) @binding(2) var depth: texture_depth_2d;
struct Fog { color: vec4<f32>, params: vec4<f32>, proj: vec4<f32> };
@group(0) @binding(3) var<uniform> fog: Fog;
// DistanceFog re-applied over the modulated pixels (A is pre-fog): same falloffs as bevy_pbr fog.wgsl,
// distance rebuilt from the reverse-Z infinite perspective depth (z_view = near / depth)
fn apply_fog(col: vec3<f32>, p: vec2<i32>) -> vec3<f32> {
    let mode = fog.params.x;
    if (mode < 0.5) { return col; }
    let d = textureLoad(depth, p, 0);
    if (d <= 0.0) { return col; }
    let size = vec2<f32>(textureDimensions(src));
    let ndc = (vec2<f32>(p) + 0.5) / size * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0);
    let z = fog.proj.x / d;
    let dist = z * length(vec3<f32>(ndc.x * fog.proj.y * fog.proj.z, ndc.y * fog.proj.y, 1.0));
    var f = 0.0;
    if (mode < 1.5) {
        f = 1.0 - clamp((fog.params.z - dist) / max(fog.params.z - fog.params.y, 1e-4), 0.0, 1.0);
    } else if (mode < 2.5) {
        f = 1.0 - 1.0 / exp(dist * fog.params.w);
    } else {
        let dd = dist * fog.params.w;
        f = 1.0 - 1.0 / exp(dd * dd);
    }
    return mix(col, fog.color.rgb, f * fog.color.a);
}
const LUM = vec3<f32>(0.2126, 0.7152, 0.0722);
@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let p = vec2<i32>(in.position.xy);
    let c = textureLoad(src, p, 0);
    let g = textureLoad(gbuf, p, 0);
    if (g.x == 0u && g.y == 0u && g.w == 0u) { return c; }
    let a = pow(unpack4x8unorm(g.x).rgb, vec3<f32>(2.2));
    let la = dot(a, LUM);
    if (la < 1e-4) { return c; }
    let f = clamp(dot(c.rgb, LUM) / la / REF, 0.6, 1.3);
    return vec4<f32>(apply_fog(a * f, p), c.a);
}
"#;

/// Marker on the RT camera (inserted/removed together with SolariLighting).
#[derive(Component, Clone, Default, ExtractComponent)]
pub struct RtgiHybrid {
    /// [fog colour rgba (a = 1 when fog on)], [mode 0 none/1 linear/2 exp/3 exp2, start, end, density], [near, tan(fov/2), aspect, 0]
    pub fog: [[f32; 4]; 3],
}

/// main world: copy the camera's DistanceFog + projection into the extracted marker
fn sync_fog(mut q: Query<(&mut RtgiHybrid, Option<&bevy::pbr::DistanceFog>, &Projection)>) {
    for (mut h, fog, proj) in &mut q {
        let mut v = [[0.0; 4]; 3];
        if let Projection::Perspective(p) = proj {
            v[2] = [p.near, (p.fov * 0.5).tan(), p.aspect_ratio, 0.0];
        }
        if let Some(f) = fog {
            let c = f.color.to_linear();
            v[0] = [c.red, c.green, c.blue, c.alpha];
            v[1] = match f.falloff {
                bevy::pbr::FogFalloff::Linear { start, end } => [1.0, start, end, 0.0],
                bevy::pbr::FogFalloff::Exponential { density } => [2.0, 0.0, 0.0, density],
                bevy::pbr::FogFalloff::ExponentialSquared { density } => [3.0, 0.0, 0.0, density],
                _ => [0.0; 4],
            };
        }
        if h.fog != v {
            h.fog = v;
        }
    }
}

#[derive(Resource, Clone)]
struct HybridShader(Handle<Shader>);

#[derive(Resource)]
struct HybridPipeline {
    layout: BindGroupLayoutDescriptor,
    id: CachedRenderPipelineId,
}

pub struct RtgiHybridPlugin;

/// REF = the average RT/raster ratio (normalises an average-lit surface to 1.0); `GAME_RTGI_REF` override.
fn reference() -> f32 {
    std::env::var("GAME_RTGI_REF").ok().and_then(|v| v.parse().ok()).unwrap_or(1.0)
}

impl Plugin for RtgiHybridPlugin {
    fn build(&self, app: &mut App) {
        let src = SRC.replace("REF", &format!("{:?}", reference()));
        let h = app.world_mut().resource_mut::<Assets<Shader>>().add(Shader::from_wgsl(src, "rtgi_hybrid.wgsl"));
        app.add_plugins(ExtractComponentPlugin::<RtgiHybrid>::default()).add_systems(PostUpdate, sync_fog);
        let Some(r) = app.get_sub_app_mut(RenderApp) else { return };
        r.insert_resource(HybridShader(h))
            .add_systems(RenderStartup, init_pipeline)
            .add_systems(Core3d, hybrid.after(main_opaque_pass_3d).before(main_transparent_pass_3d).in_set(Core3dSystems::MainPass));
    }
}

fn init_pipeline(mut commands: Commands, fs: Res<FullscreenShader>, cache: Res<PipelineCache>, shader: Res<HybridShader>) {
    let layout = BindGroupLayoutDescriptor::new(
        "rtgi_hybrid_layout",
        &BindGroupLayoutEntries::sequential(ShaderStages::FRAGMENT, (
                texture_2d(TextureSampleType::Float { filterable: false }),
                texture_2d(TextureSampleType::Uint),
                bevy::render::render_resource::binding_types::texture_depth_2d(),
                bevy::render::render_resource::binding_types::uniform_buffer_sized(false, None),
            )),
    );
    let id = cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("rtgi_hybrid".into()),
        layout: vec![layout.clone()],
        vertex: fs.to_vertex_state(),
        fragment: Some(FragmentState {
            shader: shader.0.clone(),
            targets: vec![Some(ColorTargetState { format: ViewTarget::TEXTURE_FORMAT_HDR, blend: None, write_mask: ColorWrites::ALL })],
            ..default()
        }),
        ..default()
    });
    commands.insert_resource(HybridPipeline { layout, id });
}

fn hybrid(view: ViewQuery<(&ViewTarget, &ViewPrepassTextures, &RtgiHybrid)>, p: Res<HybridPipeline>, cache: Res<PipelineCache>, mut ctx: RenderContext) {
    let (target, prepass, h) = view.into_inner();
    let Some(pipeline) = cache.get_render_pipeline(p.id) else { return };
    let Some(gbuf) = prepass.deferred_view() else { return };
    let Some(depth) = prepass.depth_view() else { return };
    let data: Vec<u8> = h.fog.iter().flatten().flat_map(|f| f.to_le_bytes()).collect();
    let buf = ctx.render_device().create_buffer_with_data(&BufferInitDescriptor { label: Some("rtgi_hybrid_fog"), contents: &data, usage: BufferUsages::UNIFORM });
    let post = target.post_process_write();
    let bg = ctx.render_device().create_bind_group(None, &cache.get_bind_group_layout(&p.layout), &BindGroupEntries::sequential((post.source, gbuf, depth, buf.as_entire_binding())));
    let desc = RenderPassDescriptor {
        label: Some("rtgi_hybrid"),
        color_attachments: &[Some(RenderPassColorAttachment { view: post.destination, depth_slice: None, resolve_target: None, ops: Operations::default() })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    };
    let mut pass = ctx.command_encoder().begin_render_pass(&desc);
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &bg, &[]);
    pass.draw(0..3, 0..1);
}

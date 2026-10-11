//! WP-C5: sprites (EFSpriteMgr_BC), motion trails (EFMotionTrailMgr_BC) and the APIgraphics_*
//! table. Drives the API with the exact call sequences of the logic modules (tod_beam
//! ToD_Beam_Create_Sprites, wisp_base Create_MotionTrail, wispsourcenew) using the real
//! EP1L01 / EP1L03A texture dictionaries.
use scooby_recomp::engine::camera::DTMatrix;
use scooby_recomp::engine::gfx::api::*;
use scooby_recomp::engine::gfx::motion_trail::*;
use scooby_recomp::engine::gfx::sprite::*;
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::runtime::EngineRuntime;
use scooby_recomp::engine::{Mem, SELF};
use std::cell::Cell;

const ISO: &str = "../Scooby Doo Mystery Mayhem.ISO";

fn at(p: [f32; 3]) -> DTMatrix {
    DTMatrix { pos: p, ..IDENTITY }
}

/// SELF = 0x2a000123; joint n of it sits at (n, 0, 10 + t)
struct W {
    t: Cell<f32>,
}
impl GfxWorld for W {
    fn ResolveGameObject(&self, uid: u32) -> Option<u32> {
        match uid {
            SELF | 0x2a00_0123 => Some(0x2a00_0123),
            _ => None,
        }
    }
    fn GetObjectFrame(&self, obj: u32, bone: u32) -> Option<DTMatrix> {
        (obj == 0x2a00_0123).then(|| at([bone as f32, 0.0, 10.0 + self.t.get()]))
    }
    fn GetObjectPosDataFrame(&self, _: u32) -> Option<DTMatrix> {
        None
    }
    fn GetJointLTM(&self, obj: u32, j: u32) -> Option<DTMatrix> {
        self.GetObjectFrame(obj, j)
    }
}

fn level_textures(dir: &str, file: &str) -> Option<Vec<String>> {
    let fs = IsoFs::open(ISO).ok()?;
    let (mut e, mut rt) = EngineRuntime::new_with_engine();
    let res = EngineRuntime::ReadLevel(&fs, dir, file).ok()?;
    rt.LoadLevel(&mut e, res);
    Some(rt.gfx.tex_dict.textures.iter().map(|t| t.name.clone()).collect())
}

fn cam() -> SpriteCamera {
    SpriteCamera { ltm: IDENTITY, view_window: [0.5, 0.5], near: 0.1, far: 1000.0, width: 640, height: 448, z_scale: 1.0, z_shift: 0.0 }
}

#[test]
fn sprite_manager_lists_and_ids() {
    let mut mem = Mem::new();
    let w = W { t: Cell::new(0.0) };
    let mut g = EFGraphicsApi::new();
    let a = g.APIgraphics_SpriteAddGroup(&mut mem);
    let b = g.APIgraphics_SpriteAddGroup(&mut mem);
    assert_eq!((a, b), (1, 2));
    assert_eq!(g.sprites.groups[0].id, 2, "head insertion");
    let s0 = g.APIgraphics_SpriteGroupAddSprite(&mut mem, a, SELF, SPRITE_OWN_FRAME, 0.45, &w);
    let s1 = g.APIgraphics_SpriteGroupAddSprite(&mut mem, a, SELF, 3, 0.45, &w);
    assert_eq!((s0, s1), (0x4c80_0000, 0x4c80_0001));
    assert_eq!(g.APIgraphics_SpriteGroupAddSprite(&mut mem, 99, SELF, 3, 0.45, &w), 0);
    assert!(matches!(g.sprites.GetSprite(s1).unwrap().frame, SpriteFrame::ObjectFrame { obj: 0x2a00_0123, bone: 3 }));
    g.APIgraphics_SpriteSetGroupFlags(&mut mem, a, 6);
    assert_eq!(g.APIgraphics_SpriteGetGroupFlags(&mut mem, a), 6);
    assert_eq!(g.APIgraphics_SpriteGetGroupFlags(&mut mem, 99), 0);
    g.APIgraphics_SpriteGroupRemoveSprite(&mut mem, a, s0);
    assert_eq!(g.sprites.GetSpriteGroup(a).unwrap().sprites.len(), 1);
    g.APIgraphics_SpriteRemoveGroup(&mut mem, a);
    assert!(g.sprites.GetSpriteGroup(a).is_none());
    // ids are global and never reused
    let s2 = g.APIgraphics_SpriteGroupAddSprite(&mut mem, b, SELF, 1, 1.0, &w);
    assert_eq!(s2, 0x4c80_0002);
    g.Terminate();
    assert!(g.sprites.groups.is_empty());
}

#[test]
fn tod_beam_spirit_sprites_ep1l01() {
    let Some(tex) = level_textures("EP1/L01", "EP1L01.P2R") else { return };
    assert!(tex.iter().any(|t| t.eq_ignore_ascii_case("spirit")), "EP1L01 has 'spirit': {tex:?}");
    let mut mem = Mem::new();
    let w = W { t: Cell::new(0.0) };
    let mut g = EFGraphicsApi::new();
    g.textures = tex;
    // ToD_Beam_Create_Sprites (tod_beam 0x00011a60)
    let gr = g.APIgraphics_SpriteAddGroup(&mut mem);
    let name = mem.cstr("spirit");
    g.APIgraphics_SpriteSetGroupTexture(&mut mem, gr, name);
    g.APIgraphics_SpriteSetGroupColor(&mut mem, gr, 0.0, f32::from_bits(0x3d4c_cccd), f32::from_bits(0x3ecc_cccd), 0.0);
    g.APIgraphics_SpriteSetGroupBlendMode(&mut mem, gr, 1);
    g.APIgraphics_SpriteSetGroupFlags(&mut mem, gr, 6);
    for i in 0..0xeu32 {
        let s = g.APIgraphics_SpriteGroupAddSprite(&mut mem, gr, SELF, SPRITE_OWN_FRAME, f32::from_bits(0x3ee6_6666), &w);
        // APIgameobj::AddChild(SELF, 0xe + i, sprite) -> coordinator hook
        assert!(g.sprites.AttachSprite(s, 0x2a00_0123, 0xe + i));
    }
    let grp = g.sprites.GetSpriteGroup(gr).unwrap();
    assert_eq!(grp.texture.as_deref().map(str::to_ascii_lowercase).as_deref(), Some("spirit"));
    assert_eq!(grp.rgba, [0, 12, 102, 0]);
    // flags 6: transparent pass only
    assert!(g.RenderSprites(SpritePass::Normal, &w, &cam()).is_empty());
    assert!(g.RenderSprites(SpritePass::Occluded, &w, &cam()).is_empty());
    let d = g.RenderSprites(SpritePass::Transparent, &w, &cam());
    assert_eq!(d.len(), 1);
    let SpriteDraw::Batch(b) = &d[0] else { panic!() };
    assert_eq!(b.blend, Some(0x68));
    assert!(b.zwrite_off && !b.ztest_off && !b.stencil_read);
    assert_eq!(b.verts.len(), 14 * 6);
    // sprite of joint 0xe at x=14, z=10: view x = 0.5*10 - 0.5*14/0.5 = -9 -> centre -0.9*640
    let v = b.verts[b.verts.len() - 6]; // list is head-first: the first-added sprite is last
    assert!((v.recip_z - 0.1).abs() < 1e-6);
    let sz = f32::from_bits(0x3ee6_6666);
    assert!((v.x - 0.1 * 640.0 * (-9.0 + sz)).abs() < 1e-3, "{v:?}");
    assert_eq!(v.rgba, [0.0, 12.0, 102.0, 0.0]);
    // removal like ToD_Beam teardown
    g.APIgraphics_SpriteRemoveGroup(&mut mem, gr);
    assert!(g.RenderSprites(SpritePass::Transparent, &w, &cam()).is_empty());
}

#[test]
fn sprite_sort_clip_and_occluded() {
    let mut mem = Mem::new();
    let w = W { t: Cell::new(0.0) };
    let mut g = EFGraphicsApi::new();
    let gr = g.APIgraphics_SpriteAddGroup(&mut mem);
    g.APIgraphics_SpriteSetGroupFlags(&mut mem, gr, 1);
    for (i, z) in [5.0f32, 20.0, 2000.0, 10.0].iter().enumerate() {
        let s = g.APIgraphics_SpriteGroupAddSprite(&mut mem, gr, SELF, SPRITE_OWN_FRAME, 0.1, &w);
        let SpriteFrame::Own { local, .. } = &mut g.sprites.GetSprite(s).unwrap().frame else { panic!() };
        local.pos = [0.0, 0.0, *z + i as f32 * 0.0];
    }
    let d = g.RenderSprites(SpritePass::Normal, &w, &cam());
    let SpriteDraw::Batch(b) = &d[0] else { panic!() };
    assert_eq!(b.blend, Some(0xa8));
    // far clip drops z=2000; back to front
    let zs: Vec<f32> = b.verts.chunks(6).map(|c| 1.0 / c[0].recip_z).collect();
    assert_eq!(zs, vec![20.0, 10.0, 5.0]);
    // occluded group: centre must be inside the hard-coded 512x448 screen
    let o = g.APIgraphics_SpriteAddGroup(&mut mem);
    g.APIgraphics_SpriteSetGroupFlags(&mut mem, o, 0x20);
    let s = g.APIgraphics_SpriteGroupAddSprite(&mut mem, o, SELF, SPRITE_OWN_FRAME, 0.05, &w);
    let SpriteFrame::Own { local, .. } = &mut g.sprites.GetSprite(s).unwrap().frame else { panic!() };
    local.pos = [-1.0, -1.0, 10.0]; // view (5+1, 5+1)/10 -> (0.6*640, 0.6*448)
    let d = g.RenderSprites(SpritePass::Occluded, &w, &cam());
    assert_eq!(d.len(), 1);
    let SpriteDraw::Occluded(oc) = &d[0] else { panic!() };
    assert!((oc.center[0] - 384.0).abs() < 1e-3 && (oc.center[1] - 268.8).abs() < 1e-3);
    assert_eq!(oc.rgba, [0x40, 0x40, 0x40, 0x7f]);
    assert_eq!(oc.blend, Some(0x98));
}

#[test]
fn wisp_motion_trail_ep1l03a() {
    // EP1L03A wisps: glow particle textures spark/halo exist in the level dictionary
    if let Some(tex) = level_textures("EP1/L03A", "EP1L03A.P2R") {
        for n in ["spark", "halo", "spirit"] {
            assert!(tex.iter().any(|t| t.eq_ignore_ascii_case(n)), "EP1L03A has '{n}'");
        }
    }
    let mut mem = Mem::new();
    let w = W { t: Cell::new(0.0) };
    let mut g = EFGraphicsApi::new();
    // wisp_base Create_MotionTrail
    let mt = g.APIgraphics_MTCreateSystem(&mut mem, 2, 5, 0);
    assert_eq!(mt, 0);
    g.APIgraphics_MTSetFrequency(&mut mem, mt, 0.6);
    g.APIgraphics_MTSetJoint(&mut mem, mt, 0, SELF, 6, &w);
    g.APIgraphics_MTSetRGB(&mut mem, mt, 0, 0.2, 0.6, 1.0);
    g.APIgraphics_MTSetLifeSpan(&mut mem, mt, 0, 0.5);
    g.APIgraphics_MTSetJoint(&mut mem, mt, 1, SELF, 7, &w);
    g.APIgraphics_MTSetRGB(&mut mem, mt, 1, 0.0, 0.0, 0.0);
    g.APIgraphics_MTSetLifeSpan(&mut mem, mt, 1, 0.5);
    assert_eq!(g.trails.Get(mt).unwrap().cps[0].obj, 0x2a00_0123, "SELF resolved");
    assert!(g.trails.Render().is_empty(), "not started");
    g.APIgraphics_MTStart(&mut mem, mt);
    let dt = 0.25;
    let mut strips = Vec::new();
    for f in 0..12 {
        w.t.set(f as f32);
        g.trails.Update(dt, &w);
        strips = g.trails.Render();
    }
    let s = &g.trails.Get(mt).unwrap();
    assert_ne!(s.head, s.tail);
    assert_eq!(strips.len(), 1);
    let st = &strips[0];
    assert!(st.verts.len() >= 4 && st.verts.len() % 2 == 0);
    // newest row (head) is at the current joint position with full colour
    let n = st.verts.len();
    assert_eq!(st.verts[n - 2].pos, [6.0, 0.0, 21.0]);
    assert_eq!(st.verts[n - 2].rgba, [51, 153, 255, 255]);
    // oldest rows are faded (age clamped to the 0.5 s life span -> black)
    assert_eq!(st.verts[0].rgba, [0, 0, 0, 255]);
    // strip indices: row r = [k, k, k+n, k+1, k+1+n, last] ...
    assert_eq!(&st.indices[..6], &[0, 0, 2, 1, 3, 3]);
    // Hooked/Return: MTStopDraw stops both tracking and drawing
    g.APIgraphics_MTStopDraw(&mut mem, mt);
    assert!(g.trails.Render().is_empty());
    g.APIgraphics_MTDestroySystem(&mut mem, mt);
    assert!(g.trails.Get(mt).is_none());
    assert_eq!(g.APIgraphics_MTCreateSystem(&mut mem, 3, 0x19, 0), 1, "ids never reused");
}

#[test]
fn motion_trail_quirks() {
    let mut mem = Mem::new();
    let w = W { t: Cell::new(0.0) };
    let mut g = EFGraphicsApi::new();
    // DTMap order: id 37 hashes to bucket 0 ahead of id 1
    for _ in 0..38 {
        g.trails.CreateSystem(1, 2, 0x1000);
    }
    let order: Vec<u32> = g.trails.Systems().map(|s| s.id).take(3).collect();
    assert_eq!(order, vec![37, 0, 1]);
    // fixed position systems (flag 0x1000) + MTSetPosition
    let p = mem.malloc(12);
    for (i, v) in [1.0f32, 2.0, 3.0].iter().enumerate() {
        mem.w32(p + 4 * i as u32, v.to_bits());
    }
    g.APIgraphics_MTSetPosition(&mut mem, 5, 0, p);
    g.APIgraphics_MTStart(&mut mem, 5);
    g.trails.Update(0.1, &w);
    assert_eq!(g.trails.Get(5).unwrap().verts[0].pos, [1.0, 2.0, 3.0]);
    // unknown object: tracking stops with -1
    let id = g.trails.CreateSystem(1, 2, 0);
    g.APIgraphics_MTStart(&mut mem, id);
    assert_eq!(EFMotionTrailMgr_BC::UpdateSystem(g.trails.Get(id).unwrap(), 0.1, &w), 0xffff_ffff);
    assert_eq!(g.trails.Get(id).unwrap().flags & 3, 0);
    // unknown ids are no-ops
    g.APIgraphics_MTSetFrequency(&mut mem, 999, 1.0);
    g.APIgraphics_MTDestroySystem(&mut mem, 999);
}

#[test]
fn texfader_and_misc_api() {
    let mut mem = Mem::new();
    let w = W { t: Cell::new(0.0) };
    let mut g = EFGraphicsApi::new();
    // _sTexFaderParams in module memory
    let p = mem.malloc(0x34);
    let a = mem.cstr("meter_a");
    let b = mem.cstr("meter_b");
    mem.w32(p, 1);
    mem.w32(p + 4, 0.5f32.to_bits());
    mem.w32(p + 8, a);
    mem.w32(p + 0x14, b);
    mem.w32(p + 0x24, 1.0f32.to_bits());
    mem.w32(p + 0x30, 0x77);
    g.APIgraphics_TexFaderPlay(&mut mem, SELF, p, &w);
    g.APIgraphics_TexFaderPlay(&mut mem, 0x2a00_0999, p, &w); // missing object: nothing
    g.APIgraphics_TexFaderPause(&mut mem, SELF, 1, &w);
    g.APIgraphics_SetGameObjBlendColor(&mut mem, SELF, 0, 1.0, 0.5, 0.25, 1.0, &w);
    g.APIgraphics_DoCrappyXboxSpewHackThingy(&mut mem, SELF, 0, 1, &w);
    g.APIgraphics_DoStupidXboxShadowHack(&mut mem, 1);
    g.APIgraphics_LensFlareActivate(&mut mem, 1);
    g.APIgraphics_LensFlareSetParams(&mut mem, 0x1ff, 3);
    assert_eq!(g.deferred.len(), 4);
    match &g.deferred[0] {
        GfxDeferred::TexFaderPlay { game_obj, obj3d_index, params } => {
            assert_eq!((*game_obj, *obj3d_index), (0x2a00_0123, 0));
            assert_eq!(params.src[0].as_deref(), Some("meter_a"));
            assert_eq!(params.dst[0].as_deref(), Some("meter_b"));
            assert_eq!(params.src[1], None);
            assert_eq!((params.flags, params.duration, params.a_to, params.notify), (1, 0.5, 1.0, 0x77));
        }
        d => panic!("{d:?}"),
    }
    assert!(g.xbox_shadow_hack && g.lens_flare.active);
    assert_eq!(g.lens_flare.params, Some((0xff, 3)));
}

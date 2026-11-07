//! WP-C3: model identification (level data + logic link) and the Bevy draw-list adapter
//! (LOD, tex-fader alpha, blend modes, fog, RW->Bevy camera convention). ISO tests skip without
//! the disc image.
use scooby_recomp::engine::dt::descriptors::{DT3dObjectDescriptor, EFIDescriptor};
use scooby_recomp::engine::gfx::*;
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::logic_link::LogicCodeEntry;
use scooby_recomp::engine::pack::{EFRessourcesMgr, RawInstanciator};
use scooby_recomp::engine::rw::image::RwTexDictionary;

use crate::common::ISO;

pub struct Level {
    pub dict: RwTexDictionary,
    pub res: Vec<EF3dObjRes>,
    pub stat: Option<EFStatic3dObj>,
    /// (GLO uid, logic code name, 3D descriptor PUID = EF3dObjRes uid, GLO pos, look_at, look_up)
    pub glos: Vec<(u32, String, Option<u32>, [f32; 3], [f32; 3], [f32; 3])>,
}

pub fn load(dir: &str) -> Option<Level> {
    let fs = IsoFs::open(ISO).ok()?;
    let mut m = EFRessourcesMgr::default();
    m.level_file_name = Some(format!("{}.P2R", dir.replace('/', "")));
    assert_eq!(m.LoadLevel(&fs, "", dir, "EN", &mut RawInstanciator, |_, _| {}).unwrap(), 0);
    let mut dict = RwTexDictionary::default();
    for (p, b) in &m.side_files {
        if p.to_ascii_uppercase().contains("TEXDIC_") {
            EFTextureDict::LoadPropertiesBinary(b, &mut dict);
        }
    }
    let (mut res, mut stat, mut codes) = (Vec::new(), None, std::collections::HashMap::new());
    for o in &m.objects {
        let Some(r) = o.as_raw() else { continue };
        match r.class_id {
            0x27 | 0x45 => {
                if let Some(e) = LogicCodeEntry::parse(r.class_id, &r.data) {
                    codes.insert(e.uid, e.name.clone());
                }
            }
            0x5b => {
                let mut x = EF3dObjRes::default();
                x.LoadPropertiesBinary(&r.data).unwrap();
                res.push(x);
            }
            0x18 if stat.is_none() => {
                let mut s = EFStatic3dObj::default();
                s.LoadPropertiesBinary(&r.data).unwrap();
                stat = Some(s);
            }
            _ => {}
        }
    }
    let glos = m
        .game_objects
        .iter()
        .map(|g| {
            // EF3dObj IDescriptor (class 3): its PUID is the EF_3DOBJ_RES uid of the clump
            let puid = g.glo.object_descriptors.objects.iter().find_map(|d| d.as_any().downcast_ref::<EFIDescriptor<DT3dObjectDescriptor>>().map(|x| x.desc.puid as u32));
            (g.uid, codes.get(&g.glo.glo_file_id).cloned().unwrap_or_default(), puid, g.glo.pos, g.glo.look_at, g.glo.look_up)
        })
        .collect();
    Some(Level { dict, res, stat, glos })
}

impl Level {
    pub fn res(&self, uid: u32) -> Option<&EF3dObjRes> {
        self.res.iter().find(|r| r.res.uid == uid)
    }
    /// Resource uids of the objects whose logic module name starts with `code`.
    pub fn by_code(&self, code: &str) -> Vec<u32> {
        let mut v: Vec<u32> = self.glos.iter().filter(|g| g.1.starts_with(code)).filter_map(|g| g.2).collect();
        v.dedup();
        v
    }
}

/// (logic code, GLO uid or 0, res uid, a texture name of the clump)
type Expect = (&'static str, u32, u32, &'static str);

fn check(lv: &Level, exp: &[Expect]) {
    for &(code, glo, uid, tex) in exp {
        assert!(lv.by_code(code).contains(&uid), "{code} -> res {uid:#x} (got {:x?})", lv.by_code(code));
        if glo != 0 {
            assert!(lv.glos.iter().any(|g| g.0 == glo && g.1.starts_with(code) && g.2 == Some(uid)), "{code} GLO {glo:#x}");
        }
        let r = lv.res(uid).unwrap_or_else(|| panic!("res {uid:#x} missing"));
        let c = r.clump.as_ref().unwrap();
        let names: Vec<String> = c.geometries.iter().flat_map(|g| g.materials.iter().filter_map(|m| m.texture.as_ref().map(|t| t.name.clone()))).collect();
        assert!(names.iter().any(|n| n.eq_ignore_ascii_case(tex)), "{code}: texture {tex} not in {names:?}");
        for n in &names {
            assert!(lv.dict.Get(n).is_some(), "{code}: texture {n} not in the level TXD");
        }
        #[cfg(feature = "bevy_render")]
        {
            use scooby_recomp::engine::gfx::bevy_adapter::*;
            let mut n = 0;
            for g in &c.geometries {
                for (mi, m) in geometry_to_meshes(g) {
                    assert!(m.count_vertices() > 0 && m.indices().is_some_and(|i| i.len() > 0), "{code}: empty mesh");
                    if let Some(t) = &g.materials[mi as usize].texture {
                        let img = texture_to_image(lv.dict.Get(&t.name).unwrap());
                        assert!(img.width() > 0 && img.height() > 0);
                    }
                    n += 1;
                }
            }
            assert!(n > 0, "{code}: no Bevy mesh");
        }
    }
}

/// EP1L01: the Tome of Doom, the hands out of the book (tod_beam latent object 0x2a800095,
/// TomeDoom "hands out of the book"), Scooby, Shaggy, the three monster families. EP1L01 has no
/// wisp (orb) module: orbs are checked on EP1L03A / EP2L09 below.
#[test]
fn ep1l01_models_identified() {
    let Some(lv) = load("EP1/L01") else { return };
    check(
        &lv,
        &[
            ("tomedoom", 0x2a00_0093, 0x324, "cover"),
            ("tod_beam", 0x2a80_0095, 0x339, "white"),
            ("scooby.op2", 0x2a00_0092, 0x6d6, "Scooby"),
            ("shaggy.op2", 0x2a00_0091, 0x756, "SHAGGY3"),
            ("smallghostlevelone", 0x2a00_0008, 0x18c, "smal_ghost"),
            ("medghostlevelone", 0, 0x6c6, "FANTOM"),
            ("shadowslevelone", 0x2a00_0013, 0x1ae, "tornadeb00100"),
            ("scooby_ghost", 0, 0x7e9, "scooby_ghost"),
            ("shaggy_ghost", 0, 0x7f2, "shaggy_ghost"),
        ],
    );
    assert!(!lv.glos.iter().any(|g| g.1.contains("wisp")), "EP1L01 has no wisps module");
    // the book and the heroes are skinned (HAnim)
    for uid in [0x324, 0x6d6, 0x756, 0x18c] {
        assert!(lv.res(uid).unwrap().clump.as_ref().unwrap().geometries.iter().any(|g| g.skin.is_some()));
    }
}

/// Boss level EP2L09 (mummy boss) + the orbs (l09_wisps) and the book there.
#[test]
fn ep2l09_boss_and_orbs_identified() {
    let Some(lv) = load("EP2/L09") else { return };
    check(
        &lv,
        &[
            ("mummylevelnine", 0x2a00_0191, 0x6f1, "Mummy"),
            ("mummy_binding", 0x2a80_018e, 0x6eb, "Mummy"),
            ("l09_wisps", 0x2a80_00b0, 0x53f, "white"),
        ],
    );
    assert!(!lv.by_code("tomedoom").is_empty() && !lv.by_code("scooby.op2").is_empty());
    // the wisp glow is a particle sprite (wisp_base Init_Particle: textures "halo"/"spirit")
    assert!(lv.dict.Get("halo").is_some() && lv.dict.Get("spirit").is_some());
}

#[test]
fn ep1l03a_orbs_identified() {
    let Some(lv) = load("EP1/L03A") else { return };
    let w = lv.by_code("l03a_wisps");
    assert_eq!(w, vec![0x8d3]);
    let c = lv.res(0x8d3).unwrap().clump.as_ref().unwrap();
    assert!(!c.geometries.is_empty());
}

#[cfg(feature = "bevy_render")]
mod adapter {
    use super::*;
    use bevy_app::App;
    use bevy_color::Alpha;
    use bevy_asset::Assets;
    use bevy_camera::visibility::Visibility;
    use bevy_material::AlphaMode;
    use bevy_pbr::{FogFalloff, StandardMaterial};
    use scooby_recomp::engine::camera::{DTMatrix, EFCamera};
    use scooby_recomp::engine::gfx::bevy_adapter::*;
    use scooby_recomp::engine::gfx::hierarchy::{IDENTITY, transform_point};
    use scooby_recomp::engine::gfx::mgr::*;
    use scooby_recomp::engine::gfx::render::*;

    fn visible(app: &mut App) -> usize {
        let w = app.world_mut();
        w.query::<(&DrawListEntity, &Visibility)>().iter(w).filter(|(_, v)| **v == Visibility::Visible).count()
    }

    /// LOD switching: the draw list holds exactly the atomics EF3dObj::Render selects, i.e. the
    /// ones LodGetAtomic finds for that LOD.
    #[test]
    fn lod_switch_matches_lodgetatomic() {
        let Some(mut lv) = load("EP1/L01") else { return };
        // a 2-atomic clump (clues_off / clues_on); put atomic #1 on LOD 1
        let r = lv.res.iter_mut().find(|r| r.res.uid == 0x64b).unwrap();
        let clump = r.clump.as_mut().unwrap();
        let mut o = EF3dObj_SP::Initialize(0x64b, clump, 0);
        o.SetVisible(1);
        o.atomics[1].ap_flags |= 1 << 16;
        assert_eq!(o.LodGetAtomic(0), Some(0));
        assert_eq!(o.LodGetAtomic(1), Some(1));
        let owner = OwnerInfo { uid: 0x2a00_0001, ltm: IDENTITY, ..Default::default() };
        let mut app = App::new();
        app.add_plugins(GfxAdapterPlugin);
        let mut mgr = EFGraphicsMgr::new(640, 448);
        let mut per_lod = Vec::new();
        for lod in [0u16, 1, 2, 0] {
            mgr.draws.clear();
            o.Render(lod, clump, &owner, &mut mgr, &mut NoTexAnim);
            let c: &RpClumpRef = clump;
            ApplyDrawList(app.world_mut(), &mgr.draws, &|_| Some((0x64b, c)), &lv.dict);
            app.update();
            let want: usize = mgr.draws.iter().map(|d| c.geometries[d.geometry].submeshes().len()).sum();
            assert_eq!(mgr.draws.len(), o.LodGetAtomic(lod as u32).map_or(0, |_| 1));
            assert_eq!(visible(&mut app), want);
            per_lod.push((mgr.draws.first().map(|d| d.atomic), want));
        }
        assert_ne!(per_lod[0].0, per_lod[1].0);
        assert_eq!(per_lod[2].1, 0);
        assert_eq!(per_lod[3], per_lod[0]);
    }
    type RpClumpRef = scooby_recomp::engine::rw::clump::RpClump;

    fn cmd(fade: (u32, f32), blend: BlendMode, flags: u32, color: [u8; 4]) -> DrawCmd {
        DrawCmd {
            owner: 1,
            atomic: 0,
            geometry: 0,
            ltm: IDENTITY,
            geometry_flags: flags,
            materials: vec![DrawMaterial { color, texture: None, fade }],
            pipeline: AtomicPipeline::DFFMapper,
            skin: None,
            stencil_write: false,
            zwrite_forced: false,
            fade_pass: 0,
            blend,
            lod: 0,
        }
    }

    #[test]
    fn texfader_alpha_and_blend_modes() {
        let m = |d: &DrawCmd| DrawMaterialKey::from_draw(d, 0).to_standard(None, false);
        // tex-fader first pass (plugin flags | 0xc0000000, alpha A)
        let s = m(&cmd((0xc000_0001, 0.25), BlendMode::Normal, 0, [255; 4]));
        assert_eq!(s.alpha_mode, AlphaMode::Blend);
        assert!((s.base_color.alpha() - 0.25).abs() < 1e-6);
        // plugin flags without bit 31: ignored
        let s = m(&cmd((0x4000_0000, 0.25), BlendMode::Normal, 0, [255; 4]));
        assert_eq!(s.alpha_mode, AlphaMode::Opaque);
        // material colour applies only with rpGEOMETRYMODULATEMATERIALCOLOR
        let s = m(&cmd((0, 0.0), BlendMode::Normal, 0x40, [255, 0, 0, 128]));
        assert_eq!(s.alpha_mode, AlphaMode::Blend);
        assert!((s.base_color.alpha() - 128.0 / 255.0).abs() < 1e-6);
        let s = m(&cmd((0, 0.0), BlendMode::Normal, 0, [255, 0, 0, 128]));
        assert_eq!(s.alpha_mode, AlphaMode::Opaque);
        assert_eq!(m(&cmd((0, 0.0), BlendMode::Additive, 0, [255; 4])).alpha_mode, AlphaMode::Add);
        let s = m(&cmd((0, 0.0), BlendMode::AdditiveFix, 0x40, [255, 255, 255, 10]));
        assert_eq!((s.alpha_mode, s.base_color.alpha()), (AlphaMode::Premultiplied, 0.0));
        assert_eq!(m(&cmd((0, 0.0), BlendMode::Subtractive, 0, [255; 4])).alpha_mode, AlphaMode::Multiply);
    }

    /// Two DrawCmds of one atomic in a frame (tex-fader passes) -> two entities, two materials.
    #[test]
    fn texfader_two_passes_two_entities() {
        let Some(lv) = load("EP1/L01") else { return };
        let c = lv.res(0x396).unwrap().clump.as_ref().unwrap();
        let mut app = App::new();
        app.add_plugins(GfxAdapterPlugin);
        let a = cmd((0xc000_0000, 0.75), BlendMode::Normal, 0, [255; 4]);
        let b = cmd((0x8000_0000, 0.25), BlendMode::Normal, 0, [255; 4]);
        ApplyDrawList(app.world_mut(), &[a, b], &|_| Some((0x396, c)), &lv.dict);
        app.update();
        assert_eq!(visible(&mut app), 2);
        let w = app.world_mut();
        let mut alphas: Vec<f32> = {
            let mats: Vec<_> = w.query::<&bevy_pbr::MeshMaterial3d<StandardMaterial>>().iter(w).map(|m| m.0.clone()).collect();
            let a = w.resource::<Assets<StandardMaterial>>();
            mats.iter().map(|h| a.get(h).unwrap().base_color.alpha()).collect()
        };
        alphas.sort_by(f32::total_cmp);
        assert_eq!(alphas, vec![0.25, 0.75]);
    }

    #[test]
    fn fog_state_to_distance_fog() {
        let mut c = EFCamera::new(0, DTMatrix::default(), Some((640, 448)));
        c.rw.fog_plane = 10.0;
        c.rw.far = 100.0;
        c.fog_enabled = 1;
        c.fog_color = 0xff20_4060;
        let mut m = EFGraphicsMgr::new(640, 448);
        m.SetRenderState(&c);
        let f = fog_to_distance_fog(&m.fog, &c.rw).unwrap();
        assert!(matches!(f.falloff, FogFalloff::Linear { start: 10.0, end: 100.0 }));
        let s = f.color.to_srgba();
        assert_eq!(((s.red * 255.0).round(), (s.green * 255.0).round(), (s.blue * 255.0).round()), (32.0, 64.0, 96.0));
        c.fog_type = 2;
        m.SetRenderState(&c);
        assert!(matches!(fog_to_distance_fog(&m.fog, &c.rw).unwrap().falloff, FogFalloff::Exponential { .. }));
        c.fog_enabled = 0;
        m.SetRenderState(&c);
        assert!(fog_to_distance_fog(&m.fog, &c.rw).is_none());
    }

    /// RW -> Bevy camera convention. An EFCamera placed at the hero spawn looking along the GLO
    /// look-at: every static-world vertex with RW camera-space z>0 (in front) must have Bevy
    /// view-space z<0, and RW camera-space +x (screen-left, EFFrustum left plane) must map to Bevy
    /// view-space -x. Geometry in front must exist (level is visible).
    #[test]
    fn rw_camera_looks_at_level() {
        let Some(lv) = load("EP1/L01") else { return };
        let g = lv.glos.iter().find(|g| g.1.starts_with("scooby.op2")).unwrap();
        let (at, up) = (g.4, g.5);
        let right = [up[1] * at[2] - up[2] * at[1], up[2] * at[0] - up[0] * at[2], up[0] * at[1] - up[1] * at[0]];
        let pos = [g.3[0] - at[0] * 6.0, g.3[1] + 2.0, g.3[2] - at[2] * 6.0];
        let cam = EFCamera::new(0, DTMatrix { right, up, at, pos }, Some((640, 448)));
        let t = rw_camera_transform(&cam.ltm);
        let view = t.to_matrix().inverse();
        let w = lv.stat.as_ref().unwrap().world.as_ref().unwrap();
        let (mut front, mut n) = (0, 0);
        for s in &w.sectors {
            for sm in &s.meshes {
                for p in &sm.mesh.positions {
                    let d = [p[0] - pos[0], p[1] - pos[1], p[2] - pos[2]];
                    let rz = d[0] * at[0] + d[1] * at[1] + d[2] * at[2];
                    let rx = d[0] * right[0] + d[1] * right[1] + d[2] * right[2];
                    let ry = d[0] * up[0] + d[1] * up[1] + d[2] * up[2];
                    let v = view.transform_point3(bevy_math::Vec3::from(*p));
                    if rz.abs() > 1e-3 {
                        assert_eq!(rz > 0.0, v.z < 0.0);
                    }
                    assert!((v.x + rx).abs() < 1e-2 && (v.y - ry).abs() < 1e-2);
                    if rz > 0.0 && (rx / rz).abs() < 1.0 && (ry / rz).abs() < 0.75 {
                        front += 1;
                    }
                    n += 1;
                }
            }
        }
        assert!(n > 0 && front > 100, "{front}/{n} vertices in view");
        // and the point straight ahead is centred on screen
        let ahead = transform_point(&IDENTITY, [pos[0] + at[0] * 5.0, pos[1] + at[1] * 5.0, pos[2] + at[2] * 5.0]);
        let v = view.transform_point3(bevy_math::Vec3::from(ahead));
        assert!(v.x.abs() < 1e-3 && v.y.abs() < 1e-3 && v.z < 0.0);
    }

    /// The whole EP1L01 draw list of every placed 3D object (Render at LOD 0) through the adapter.
    #[test]
    fn ep1l01_draw_list() {
        let Some(mut lv) = load("EP1/L01") else { return };
        let mut app = App::new();
        app.add_plugins(GfxAdapterPlugin);
        let mut mgr = EFGraphicsMgr::new(640, 448);
        let glos: Vec<_> = lv.glos.iter().filter_map(|g| g.2.map(|r| (g.0, r))).collect();
        let mut owners = Vec::new();
        for (uid, res) in glos {
            let Some(r) = lv.res.iter_mut().find(|r| r.res.uid == res) else { continue };
            let Some(c) = r.clump.as_mut() else { continue };
            let mut o = EF3dObj_SP::Initialize(res, c, 0);
            o.SetVisible(1);
            o.SyncFrames(IDENTITY);
            o.Render(0, c, &OwnerInfo { uid, ltm: IDENTITY, ..Default::default() }, &mut mgr, &mut NoTexAnim);
            owners.push((uid, res));
        }
        assert!(mgr.draws.iter().any(|d| d.skin.is_some()), "skinned draws present");
        let res = &lv.res;
        let look = |o: u32| owners.iter().find(|x| x.0 == o).and_then(|x| res.iter().find(|r| r.res.uid == x.1)).and_then(|r| r.clump.as_ref()).map(|c| (owners.iter().find(|x| x.0 == o).unwrap().1, c));
        ApplyDrawList(app.world_mut(), &mgr.draws, &look, &lv.dict);
        app.update();
        let n = visible(&mut app);
        assert!(n >= mgr.draws.len(), "{n} entities for {} draws", mgr.draws.len());
    }
}

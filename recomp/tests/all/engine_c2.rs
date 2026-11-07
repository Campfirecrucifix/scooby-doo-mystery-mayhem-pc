//! WP-C2: render-side logic (EF3dObj_SP, hierarchy, EFGraphicsMgr, EFWorld queue) and the Bevy
//! adapter against EP1L01 (ISO tests skip without the disc image).
use scooby_recomp::engine::camera::{DTMatrix, EFCamera};
use scooby_recomp::engine::gfx::hierarchy::IDENTITY;
use scooby_recomp::engine::gfx::mgr::*;
use scooby_recomp::engine::gfx::render::*;
use scooby_recomp::engine::gfx::*;
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::rw::image::RwTexDictionary;

use crate::common::ISO;

fn entries(pk: &[u8], want: u32) -> Vec<Vec<u8>> {
    let u = |o: usize| u32::from_le_bytes(pk[o..o + 4].try_into().unwrap());
    let n = u(4) as usize;
    let base = 8 + 16 * n;
    (0..n)
        .filter(|&i| u(20 + 16 * i) == want)
        .map(|i| pk[base + u(16 + 16 * i) as usize..][..u(12 + 16 * i) as usize].to_vec())
        .collect()
}

#[allow(dead_code)]
struct Level {
    dict: RwTexDictionary,
    res: Vec<EF3dObjRes>,
    stat: EFStatic3dObj,
}

fn load() -> Option<Level> {
    let fs = IsoFs::open(ISO).ok()?;
    let mut dict = RwTexDictionary::default();
    assert_eq!(EFTextureDict::LoadPropertiesBinary(&fs.read("LEVEL/EP1/L01/EN/TEXDIC_0.TXD").unwrap(), &mut dict), 0);
    let pk = fs.read("LEVEL/EP1/L01/GEN/EP1L01.P2R").unwrap();
    let res = entries(&pk, 0x5b)
        .iter()
        .map(|b| {
            let mut r = EF3dObjRes::default();
            r.LoadPropertiesBinary(b).unwrap();
            r
        })
        .collect();
    let mut stat = EFStatic3dObj::default();
    stat.LoadPropertiesBinary(&entries(&pk, 0x18)[0]).unwrap();
    Some(Level { dict, res, stat })
}

struct Rec(Vec<RenderPass>);
impl RenderSeam for Rec {
    fn pass(&mut self, _: &mut EFGraphicsMgr, p: RenderPass) {
        self.0.push(p);
    }
}

#[test]
fn graphics_mgr_pass_order() {
    let mut m = EFGraphicsMgr::new(640, 448);
    let mut c = EFCamera::new(0, DTMatrix::default(), Some((640, 448)));
    c.flags = 1;
    c.render_flag = 0x20 | 0x40 | 4;
    m.cameras.cameras[0] = Some(c);
    let mut r = Rec(Vec::new());
    m.Render(0.033, &mut r);
    let p = &r.0;
    let pos = |x: RenderPass| p.iter().position(|&y| y == x).unwrap();
    assert!(pos(RenderPass::RenderMap(0)) < pos(RenderPass::RenderObjects(0)));
    assert!(pos(RenderPass::RenderObjects(0)) < pos(RenderPass::RenderObjectsTransparent(0)));
    assert!(p.contains(&RenderPass::Particles));
    assert_eq!(*p.last().unwrap(), RenderPass::Flip);
    assert_eq!(m.stencil.clears, 1);
    m.Pause(1);
    let mut r = Rec(Vec::new());
    m.Render(0.033, &mut r);
    assert!(r.0.is_empty());
}

#[test]
fn transparent_queue_sorted_back_to_front() {
    let o = |f, b, z: f32| RenderObjInfo { render_type: 1, has_3dobj: true, obj_flags: f, bucket: b, pos: [0., 0., z], ..Default::default() };
    let objs = vec![o(2, 0x20, 1.0), o(0x10, 0x20, 2.0), o(0x10, 0x20, 5.0), o(0x200, 3, 1.0)];
    let mut q = EFWorldRenderQueue::default();
    let mut cl = Vec::new();
    let now = q.RenderObjects(&objs, &IDENTITY, 0, &mut cl);
    assert_eq!(now.len(), 1);
    assert_eq!(cl.len(), 4);
    let t = q.RenderObjectsTransparent(&objs, 0);
    assert_eq!(t.iter().map(|x| x.0).collect::<Vec<_>>(), vec![3, 2, 1]);
    assert_eq!(t[0].2, BlendMode::Subtractive);
    assert_eq!(t[1].2, BlendMode::Additive);
}

#[test]
fn ep1l01_render_all_clumps() {
    let Some(mut lv) = load() else { return };
    let mut m = EFGraphicsMgr::new(640, 448);
    let (mut skinned, mut drawn) = (0, 0);
    for r in lv.res.iter_mut() {
        let Some(c) = r.clump.as_mut() else { continue };
        let mut o = EF3dObj_SP::Initialize(r.res.uid, c, 0);
        if o.hier.hierarchy.is_some() {
            skinned += 1;
            assert!(o.base.flags & 4 != 0);
        }
        o.SetVisible(1);
        let owner = OwnerInfo { uid: 0x2a00_0001, ltm: IDENTITY, ..Default::default() };
        m.draws.clear();
        o.Render(0, c, &owner, &mut m, &mut NoTexAnim);
        drawn += m.draws.len();
        for d in &m.draws {
            assert!(d.ltm.iter().chain(d.skin.iter().flatten().flatten()).all(|x| x.is_finite()));
        }
    }
    assert!(drawn > 0);
    assert!(skinned > 0, "hero/monster clumps have HAnim hierarchies");
    let mut w = EFWorld_SP::default();
    w.SetStatic3dObj(Some(std::mem::take(&mut lv.stat)));
    assert!(w.GetCurrentWorld().is_some());
}

#[cfg(feature = "bevy_render")]
#[test]
fn ep1l01_into_headless_bevy_app() {
    use bevy_app::App;
    use scooby_recomp::engine::gfx::bevy_adapter::*;
    let Some(lv) = load() else { return };
    let mut app = App::new();
    app.add_plugins(GfxAdapterPlugin);
    let w = app.world_mut();
    assert!(spawn_static_world(w, &lv.stat, &lv.dict).is_some());
    let mut n = 0;
    for (i, r) in lv.res.iter().enumerate() {
        let Some(c) = r.clump.as_ref() else { continue };
        let mut o = EF3dObj_SP::Initialize(r.res.uid, c, 0);
        o.SetVisible(1);
        let root = spawn_3dobj(w, 0x2a00_0000 + i as u32, &IDENTITY, &o, c, &lv.dict);
        sync_3dobj(w, root, &IDENTITY, &o);
        n += 1;
    }
    app.update();
    let w = app.world_mut();
    assert_eq!(w.query::<&EF3dObjEntity>().iter(w).count(), n);
    assert!(w.query::<&RwAtomicEntity>().iter(w).count() > 0);
}

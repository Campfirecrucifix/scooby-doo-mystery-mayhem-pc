//! WP-D4: real TomeDoom (book) anims on EP1L01, BlendPrimColl, EFMorphAnimNode/ApMorphAnimation.
use scooby_recomp::engine::anim::coll_blend::*;
use scooby_recomp::engine::anim::morph::*;
use scooby_recomp::engine::anim::{api::*, mgr::*, nodes::*, res::*, AnimInstanciator};
use scooby_recomp::engine::dt::descriptors::{DTAnimDescriptor, EFIDescriptor};
use scooby_recomp::engine::rw::stream::RwStream;
use scooby_recomp::engine::{iso::IsoFs, logic_link::LogicCodeEntry, pack::*};
use std::collections::HashMap;
use std::sync::Arc;

const ISO: &str = "../Scooby Doo Mystery Mayhem.ISO";

/// EP1L01 book GLO 0x2a000093 (logic code "tomedoom"): OBJECT_DESCRIPTORS = [3D_OBJECT_DESCR,
/// 7 x DT_ANIM_NODE_DESCR]. EFGameObject::CreateInstances: anim id = descriptor index, manager
/// base (+0x60) = first anim descriptor index (1), node resource = GetRessource(descriptor PUID).
/// TomeDoomFUNC_SetMasterParams (EP1L01 build 802a33e5): Shaggy (0x2a000091) holds -> open 1,
/// close 3, open idle 4; Scooby (0x2a000092) -> open 5, close 7, open idle 6; id 2 = CB_ANIM_LF /
/// msg 0x76 anim (looped).
#[test]
fn ep1l01_book_real_open_close() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let mut rm = EFRessourcesMgr::default();
    rm.level_file_name = Some("EP1L01.P2R".into());
    rm.LoadLevel(&fs, "", "EP1/L01", "EN", &mut AnimInstanciator::default(), |_, _| {}).unwrap();
    let mut codes = HashMap::new();
    let mut hanims: HashMap<u32, EFHAnimRes> = HashMap::new();
    let mut additive = Vec::new();
    for o in rm.objects.iter_mut() {
        if let Some(r) = o.as_raw() {
            if let Some(e) = LogicCodeEntry::parse(r.class_id, &r.data) {
                codes.insert(e.uid, e.base_name());
            }
        }
        if let Some(d) = o.as_dt() {
            if let Some(h) = d.as_any().downcast_ref::<EFHAnimRes>() {
                let a = h.base.anim.as_ref().unwrap();
                if a.unkd[1] == 2 {
                    additive.push(h.base.hdr.uid);
                }
                hanims.insert(h.base.hdr.uid, h.clone());
            }
        }
    }
    // DTAnimHdr+0xe == 2 (additive marker, EFAnimationMgr::ConnectHAnim 0x001ea3a4 lbu 0xe / ==2)
    eprintln!("additive {additive:x?}");
    assert_eq!(additive.len(), 2, "{additive:x?}");
    let book = rm.game_objects.iter().find(|g| codes.get(&g.glo.glo_file_id).map(|s| s.as_str()) == Some("tomedoom")).expect("book GLO");
    assert_eq!(book.uid, 0x2a00_0093);
    // CreateInstances
    let descs = &book.glo.object_descriptors.objects;
    let base = descs.iter().position(|d| d.GetClassName() == "DT_ANIM_NODE_DESCR").unwrap() as u32;
    assert_eq!(base, 1);
    let mut m = EFAnimationMgr::new(base, 0);
    m.owner_uid = book.uid;
    let mut puids = Vec::new();
    for (i, d) in descs.iter().enumerate() {
        let Some(a) = d.as_any().downcast_ref::<EFIDescriptor<DTAnimDescriptor>>() else { continue };
        let res = Arc::new(hanims[&(a.desc.puid as u32)].clone());
        let mut n = EFHAnimNode::new(res, i as u32);
        n.obj = 1;
        m.AddAnim(n, i as u32);
        puids.push(a.desc.puid as u32);
    }
    assert_eq!(puids, (0x325..=0x32b).collect::<Vec<u32>>());
    for id in 1..=7u32 {
        assert!(!additive.contains(&puids[id as usize - 1]));
    }
    // Shaggy holds the book: TomeDoom ST_OPEN plays L+0x230 = 1, ST_CLOSE L+0x234 = 3 (PLAY_ONCE)
    let run = |m: &mut EFAnimationMgr, id: u32| {
        APIanim_Play(Some(m), id, state::PLAY_ONCE, 0.0);
        assert_eq!(APIanim_IsAnimPlaying(Some(m), id), 1);
        let dur = APIanim_GetDuration(Some(m), id);
        m.Update(0.0);
        let p0 = m.GetPose(1).unwrap();
        for _ in 0..20 {
            m.Update(dur / 10.0);
        }
        let p1 = m.GetPose(1).unwrap();
        APIanim_StopAnim(Some(m), id);
        (dur, p0, p1)
    };
    let diff = |a: &[scooby_recomp::engine::anim::res::DTTransfo], b: &[scooby_recomp::engine::anim::res::DTTransfo]| {
        a.iter().zip(b).map(|(x, y)| x.f.iter().zip(y.f).map(|(u, v)| (u - v).abs()).fold(0.0f32, f32::max)).fold(0.0f32, f32::max)
    };
    let (d_open, o0, o1) = run(&mut m, 1);
    let (d_close, c0, c1) = run(&mut m, 3);
    assert!((d_open - 0.36666667).abs() < 1e-6 && (d_close - 0.36666667).abs() < 1e-6);
    assert!(diff(&o0, &o1) > 1e-3, "open anim does not move");
    assert!(diff(&c0, &c1) > 1e-3, "close anim does not move");
    // open ends where close starts (and vice versa)
    assert!(diff(&o1, &c0) < diff(&o0, &o1), "open end != close start");
    assert!(diff(&c1, &o0) < diff(&c0, &c1), "close end != open start");
    // Scooby's set (5 open, 7 close) also animates
    let (_, s0, s1) = run(&mut m, 5);
    assert!(diff(&s0, &s1) > 1e-3);
}

fn prim(ty: u8, tag: u32, c: [f32; 3], r: f32) -> EFAnimBaseCollPrim {
    let mut body = vec![0u32; 0x48 / 4];
    body[0] = c[0].to_bits();
    body[1] = c[1].to_bits();
    body[2] = c[2].to_bits();
    body[3] = r.to_bits();
    EFAnimBaseCollPrim { ty, tag, flags: 0, size: 0x5c, body }
}

#[test]
fn blend_prim_coll_merge_and_spheres() {
    let mut b = EFAnimBlendNode::new(2, 1.0);
    let a = CollPrimList { count: 3, prims: vec![prim(0, 1, [0.0; 3], 1.0), prim(2, 3, [0.0; 3], 0.0), prim(2, 7, [0.0; 3], 0.0)], is_obj_list: false };
    let c = CollPrimList { count: 3, prims: vec![prim(0, 1, [4.0, 0.0, 0.0], 1.0), prim(2, 5, [0.0; 3], 0.0), prim(2, 7, [0.0; 3], 0.0)], is_obj_list: false };
    let mut called = 0;
    // cnt 0.5 of dur 1 -> w = 0.5
    let (out, set) = b.BlendPrimColl(true, 0.5, &a, &c, &mut |_, _| called += 1).unwrap();
    assert!(!set);
    assert_eq!(called, 0);
    assert_eq!(out.prims.iter().map(|p| p.tag).collect::<Vec<_>>(), vec![1, 3, 5, 7]);
    assert_eq!(out.count, 4);
    // enclosing sphere of the two unit spheres 4 apart: centre 2, radius 3
    let h = &out.prims[0];
    assert_eq!(f32::from_bits(h.body[0]), 2.0);
    assert_eq!(f32::from_bits(h.body[3]), 3.0);
    assert!(b.BlendPrimColl(false, 0.5, &a, &c, &mut |_, _| {}).is_none());
    // additive: the first list is taken as is
    b.additive = true;
    let (out, set) = b.BlendPrimColl(true, 0.5, &CollPrimList { is_obj_list: true, ..a.clone() }, &c, &mut |_, _| {}).unwrap();
    assert!(set);
    assert_eq!(out.prims, a.prims);
}

/// Synthetic ApMorphAnimation blob (no 0x13 MORPH_ANIM_INST entry exists in any shipped P2R):
/// one atomic, vertex channel over vertex 1, keys t=0 (+0) / t=1 (+2 on x) / t=2 (+0).
fn morph_chunk() -> Vec<u8> {
    let mut b = vec![0u8; 0x200];
    let w = |b: &mut Vec<u8>, o: usize, v: u32| b[o..o + 4].copy_from_slice(&v.to_le_bytes());
    let f = |b: &mut Vec<u8>, o: usize, v: f32| b[o..o + 4].copy_from_slice(&v.to_bits().to_le_bytes());
    w(&mut b, 0, 0x20); // table
    b[4] = 1;
    b[8..10].copy_from_slice(&0x28a8u16.to_le_bytes());
    b[0xc] = 1;
    w(&mut b, 0x20, 0x30); // atomic 0 -> AnimEx
    w(&mut b, 0x30, 0x40); // vertex channel
    let ch = 0x40;
    b[ch + 4..ch + 6].copy_from_slice(&3u16.to_le_bytes());
    b[ch + 8..ch + 10].copy_from_slice(&1u16.to_le_bytes());
    w(&mut b, ch + 0xc, 0x80);
    b[0x80..0x82].copy_from_slice(&1u16.to_le_bytes());
    w(&mut b, ch + 0x10, 0x90);
    w(&mut b, ch + 0x18, 0xa0);
    w(&mut b, ch + 0x1c, 0xc0);
    for k in 0..3usize {
        let p = 0xc0 + k * 0xc;
        f(&mut b, p, k as f32);
        w(&mut b, p + 4, if k < 2 { (0xc0 + (k + 1) * 0xc) as u32 } else { 0xc0 });
        w(&mut b, p + 8, (0x100 + k * 0xc) as u32);
        f(&mut b, 0x100 + k * 0xc, if k == 1 { 2.0 } else { 0.0 });
    }
    let mut c = Vec::new();
    c.extend_from_slice(&0x1bad006u32.to_le_bytes());
    c.extend_from_slice(&(b.len() as u32).to_le_bytes());
    c.extend_from_slice(&0x3400_0000u32.to_le_bytes());
    c.extend(b);
    c
}

#[test]
fn morph_anim_node_moves_vertices() {
    let c = morph_chunk();
    let anim = ApMorphAnimationStreamRead(&mut RwStream::new(&c)).unwrap().unwrap();
    assert_eq!(ApMorphAnimationGetDuration(&anim), 2.0);
    let mut clump = MorphClump { atomics: vec![MorphAtomic { verts: vec![[0.0; 3], [1.0, 0.0, 0.0]], normals: vec![[0.0, 1.0, 0.0]; 2], nb_geom_morph_targets: 1, ..Default::default() }] };
    let mut n = EFMorphAnimNode::new(anim, &mut clump, 5);
    assert_eq!((n.duration, n.nb_sets), (2.0, 1));
    n.Play(&mut clump, state::PLAY_ONCE, 0.0);
    n.Update(&mut clump, 0.5);
    assert!((clump.atomics[0].verts[1][0] - 2.0).abs() < 1e-5, "{:?}", clump.atomics[0].verts);
    assert_eq!(clump.atomics[0].verts[0], [0.0; 3]);
    n.Update(&mut clump, 0.5);
    assert!((clump.atomics[0].verts[1][0] - 3.0).abs() < 1e-5);
    n.Update(&mut clump, 5.0); // clamps at the end, notify 1
    assert!((clump.atomics[0].verts[1][0] - 1.0).abs() < 1e-5);
    assert_eq!(n.notifications, vec![(1, 5)]);
    assert!(n.state & state::FINISHED != 0);
    n.Stop();
    assert_eq!(n.notifications.last(), Some(&(0, 5)));
    // no chunk -> NULL
    assert!(ApMorphAnimationStreamRead(&mut RwStream::new(&[])).unwrap().is_none());
}

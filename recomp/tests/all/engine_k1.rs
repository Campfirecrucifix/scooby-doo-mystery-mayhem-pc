//! WP-K1: book capture sequence audit (see recomp/BOOK_MECHANICS.md). Asserts that every level
//! resource the ported capture logic resolves to exists on the disc: book / beam / monster / orb
//! models + anims, the book's sound events (with PCM), particle textures, and the GLOs the book
//! messages. ISO tests skip without the disc image.
use scooby_recomp::engine::audio::{handlers::*, mgr::*};
use scooby_recomp::engine::dt::descriptors::*;
use scooby_recomp::engine::gfx::EFTextureDict;
use scooby_recomp::engine::rw::image::RwTexDictionary;
use scooby_recomp::engine::{iso::IsoFs, logic_link::LogicCodeEntry, pack::*};
use std::collections::HashMap;
use crate::common::ISO;

struct Lvl {
    rm: EFRessourcesMgr,
    /// resource uid -> class id (every pack object)
    cls: HashMap<u32, u32>,
    /// logic code uid -> module base name
    codes: HashMap<u32, String>,
    dict: RwTexDictionary,
}
fn load(fs: &IsoFs, dir: &str) -> Lvl {
    let mut rm = EFRessourcesMgr::default();
    rm.level_file_name = Some(format!("{}.P2R", dir.replace('/', "")));
    assert_eq!(rm.LoadLevel(fs, "", dir, "EN", &mut RawInstanciator, |_, _| {}).unwrap(), 0);
    let (mut cls, mut codes) = (HashMap::new(), HashMap::new());
    for o in &rm.objects {
        let Some(r) = o.as_raw() else { continue };
        cls.insert(r.uid, r.class_id);
        if let Some(e) = LogicCodeEntry::parse(r.class_id, &r.data) {
            codes.insert(e.uid, e.base_name());
        }
    }
    let mut dict = RwTexDictionary::default();
    for (p, b) in &rm.side_files {
        if p.to_ascii_uppercase().contains("TEXDIC_") {
            EFTextureDict::LoadPropertiesBinary(b, &mut dict);
        }
    }
    Lvl { rm, cls, codes, dict }
}
impl Lvl {
    /// (logic code, model PUIDs, anim PUIDs (descriptor order), sound ids)
    fn glo(&self, uid: u32) -> (String, Vec<u32>, Vec<u32>, Vec<u32>) {
        let g = self.rm.game_objects.iter().find(|g| g.uid == uid).unwrap_or_else(|| panic!("GLO {uid:#x} missing"));
        let (mut m, mut a, mut s) = (vec![], vec![], vec![]);
        for d in &g.glo.object_descriptors.objects {
            let x = d.as_any();
            if let Some(v) = x.downcast_ref::<EFIDescriptor<DT3dObjectDescriptor>>() {
                m.push(v.desc.puid as u32)
            } else if let Some(v) = x.downcast_ref::<EFIDescriptor<DTAnimDescriptor>>() {
                a.push(v.desc.puid as u32)
            } else if let Some(v) = x.downcast_ref::<EFIDescriptor<DTSoundDescriptor>>() {
                s.push(v.desc.index as u32)
            }
        }
        (self.codes.get(&g.glo.glo_file_id).cloned().unwrap_or_default(), m, a, s)
    }
    /// Asserts code name, that every model is an EF_3DOBJ_RES and every anim an anim resource.
    fn check(&self, uid: u32, code: &str, anims: &[u32]) -> Vec<u32> {
        let (c, m, a, s) = self.glo(uid);
        assert!(c.starts_with(code), "{uid:#x}: code {c} != {code}");
        assert!(!m.is_empty(), "{uid:#x} has no model");
        for r in &m {
            assert_eq!(self.cls.get(r), Some(&0x5b), "{uid:#x} model {r:#x}");
        }
        if !anims.is_empty() {
            assert_eq!(a, anims, "{uid:#x} anim PUIDs");
        }
        for r in &a {
            let k = self.cls.get(r).copied();
            assert!(matches!(k, Some(0x4f) | Some(0x49) | Some(0x13)), "{uid:#x} anim {r:#x} class {k:x?}");
        }
        s
    }
    fn tex(&self, name: &str) -> bool {
        self.dict.textures.iter().any(|t| t.name.eq_ignore_ascii_case(name))
    }
}

/// Book sound ids used by TomeDoom / wisp_base (APInewSound::Play/IsPlaying/StopSound(SELF, id)).
const BOOK_SOUNDS: [u32; 11] = [0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x2e, 0x41, 0x42];

#[test]
fn ep1l01_capture_resources() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let l = load(&fs, "EP1/L01");
    // heroes holding the book (TomeDoom k literals ID_2a000091 / 0x92)
    l.check(0x2a00_0091, "shaggy", &[]);
    l.check(0x2a00_0092, "scooby", &[]);
    // book GLO 0x2a000093: model 0x324, anims 0x325..0x32b (WP-D4 ids 1..7)
    let s = l.check(0x2a00_0093, "tomedoom", &(0x325..=0x32b).collect::<Vec<_>>());
    for id in BOOK_SOUNDS {
        assert!(s.contains(&id), "book sound {id:#x}");
    }
    // quirk: FXTomeAbsorbLP 0xf (SET_CAPTURE Play(SELF,0xf)) is NOT registered on the EP1L01 book
    assert!(!s.contains(&0xf));
    // tod_beam hands latent ID_2a800095 (LaunchLatant in SET_CAPTURE): model 0x339, anims 0x33a..0x33f
    assert_eq!(0x2a80_0095 & 0x80_0000, 0x80_0000);
    l.check(0x2a80_0095, "tod_beam", &(0x33a..=0x33f).collect::<Vec<_>>());
    // message targets: inv_control (0x5c button prompt), panel (0x1d/0x26 meter), rdrmngr (0xea),
    // Monster_Base latent 0x2a800118 = enemybar (monster life bar, NOT a beam)
    assert!(l.glo(0x2a00_0096).0.starts_with("inv_control"));
    assert!(l.glo(0x2a10_0006).0.starts_with("panel"));
    assert!(l.glo(0x2a00_011a).0.starts_with("rdrmngr"));
    l.check(0x2a80_0118, "enemybar", &[]);
    // monsters with book code: hooked/sucked/captured anims come from their own descriptors
    l.check(0x2a00_0006, "medghost", &[]);
    l.check(0x2a00_0008, "smallghost", &[]);
    l.check(0x2a00_0013, "shadows", &[]);
    // particle textures: tod_beam "spirit", book wisp-collect "spark"/"halo"
    for t in ["spirit", "spark", "halo"] {
        assert!(l.tex(t), "texture {t} not in EP1L01 TXD");
    }
}

#[test]
fn ep1l03a_orb_resources() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let l = load(&fs, "EP1/L03A");
    let s = l.check(0x2a00_00ee, "tomedoom", &(0x8b0..=0x8b7).collect::<Vec<_>>());
    for id in BOOK_SOUNDS.iter().chain([0xfu32].iter()) {
        assert!(s.contains(id), "book sound {id:#x}");
    }
    l.check(0x2a80_00f5, "tod_beam", &(0x78f..=0x795).collect::<Vec<_>>());
    // orbs: l03a_wisps (model 0x8d3, anims 0x8d4/0x8d5), launched latent 0x2a8000d4, huge sources
    for u in [0x2a00_00ac, 0x2a00_00cd, 0x2a80_00d4] {
        l.check(u, "l03a_wisps", &[0x8d4, 0x8d5]);
    }
    l.check(0x2a00_00d6, "wispsourcenew", &[]);
    for t in ["spirit", "spark", "halo"] {
        assert!(l.tex(t), "texture {t} not in EP1L03A TXD");
    }
}

/// Every book sound id resolves to a GLALL event with non-empty PCM (bank 0 = id >> 30).
#[test]
fn book_sounds_have_pcm() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    for (dir, book) in [("EP1/L01", 0x2a00_0093u32), ("EP1/L03A", 0x2a00_00ee)] {
        let mut res = EFRessourcesMgr::default();
        res.level_file_name = Some(format!("{}.P2R", dir.replace('/', "")));
        let mut inst = AudioInstanciator::default();
        assert_eq!(res.LoadLevel(&fs, "", dir, "EN", &mut inst, |_, _| {}).unwrap(), 0);
        let mut m = EFNewAudioMgr::new();
        inst.env.lock().unwrap()[0].Apply(&mut m);
        let lang = "EN";
        assert!(m.LoadSoundBanks(dir, lang, &mut |p| fs.read(p).ok()) >= 1);
        let go = res.game_objects.iter().find(|g| g.uid == book).unwrap();
        let regs = CreateSoundInstances(&mut m, go);
        for id in BOOK_SOUNDS {
            assert!(regs.iter().any(|r| r.1 == id), "{dir} {id:#x} not registered");
            m.voices.clear();
            assert_ne!(m.Play(id, book), 0, "{dir} {id:#x}");
            let v = m.voices.iter().find(|v| v.sample_rate > 0).unwrap_or_else(|| panic!("{dir} {id:#x} no sample"));
            let ok = v.stream.is_some() || m.banks[v.bank as usize].as_ref().unwrap().pcm[v.sample as usize].as_ref().map_or(false, |p| !p.samples.is_empty());
            assert!(ok, "{dir} {id:#x} empty pcm");
            m.StopSound(id, book);
        }
    }
}

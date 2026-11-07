//! WP-X4: ModuleDef::init literal tables replaced by words read from the module ELFs on the disc.
//! For every module of Decompiled Source/INDEX-modules.tsv that maps to a ported ModuleDef, builds the
//! initial data image exactly as `load_module` does (`init`, then `build_init`, in a fresh Mem) and
//! dumps its non-zero words. X4_DUMP=<dir> (old tree) writes <dir>/<sha8>.txt, X4_EXPECT=<dir>
//! compares; without either it only checks that every image builds.
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::logic_link::{FindPortedModule, LogicCodeEntry};
use scooby_recomp::engine::Mem;

use crate::common::ISO;

fn image(def: &'static scooby_recomp::logic::ModuleDef, b: usize) -> String {
    let mut m = Mem::new();
    let data = m.malloc(def.data_size);
    for &(addr, words) in def.init {
        for (i, &w) in words.iter().enumerate() {
            m.w32(data + (addr - 0x10000) + i as u32 * 4, w);
        }
    }
    if let Some(f) = def.build_init {
        f(&mut m, data, b);
    }
    let mut s = format!("{} {}\n", def.name, b);
    for o in (0..def.data_size & !3).step_by(4) {
        let w = m.r32(data + o);
        if w != 0 {
            s += &format!("{:x} {:08x}\n", 0x10000 + o, w);
        }
    }
    s
}

#[test]
fn x4_module_images() {
    if IsoFs::open(ISO).is_err() {
        return;
    }
    scooby_recomp::engine::gamedata::set_iso_path(std::path::Path::new(ISO));
    let dump = std::env::var("X4_DUMP").ok();
    let expect = std::env::var("X4_EXPECT").ok();
    let idx = std::fs::read_to_string("../Decompiled Source/INDEX-modules.tsv").expect("INDEX-modules.tsv");
    let (mut n, mut bad) = (0, Vec::new());
    for line in idx.lines().skip(1) {
        let c: Vec<&str> = line.split('\t').collect();
        let e = LogicCodeEntry { class_id: 0x27, uid: 0, arg: 0, name: c[1].to_string(), sha8: c[4][..8].to_string() };
        let Some((def, b)) = FindPortedModule(&e) else { continue };
        if scooby_recomp::engine::gamedata::module_elf_opt(&e.sha8).is_none() {
            continue; // module not on this disc image
        }
        let s = image(def, b);
        n += 1;
        if let Some(d) = &dump {
            std::fs::create_dir_all(d).unwrap();
            std::fs::write(format!("{d}/{}.txt", e.sha8), &s).unwrap();
        }
        if let Some(d) = &expect {
            let old = std::fs::read_to_string(format!("{d}/{}.txt", e.sha8)).unwrap_or_default();
            if old != s {
                let diff = old.lines().zip(s.lines()).find(|(a, b)| a != b).map(|(a, b)| format!("{a} -> {b}"));
                bad.push(format!("{} {}: {:?}", c[0], e.sha8, diff));
            }
        }
    }
    eprintln!("x4: {n} module images");
    assert!(n > 100, "only {n} module images");
    assert!(bad.is_empty(), "{} images differ:\n{}", bad.len(), bad.join("\n"));
}

/// WP-X4: panel image::syms (ELF symbol table) for every build (equal to the deleted SYMS table for all 37
/// builds when it was replaced).
#[test]
fn x4_panel_syms() {
    if IsoFs::open(ISO).is_err() {
        return;
    }
    scooby_recomp::engine::gamedata::set_iso_path(std::path::Path::new(ISO));
    use scooby_recomp::modules::panel::{builds::BUILDS, image};
    for b in 0..BUILDS.len() {
        let s = image::syms(b);
        assert!(s.0 > 0 && s.1[image::S_pGlobalData_BasePanel] != 0, "panel build {b} {}", BUILDS[b].sha8);
    }
}

/// WP-X4: per-build tables converted with tools/rustport/x3_gen.py (rows `{:?}`, X4_DUMP / X4_EXPECT as above).
#[test]
fn x4_tables() {
    if IsoFs::open(ISO).is_err() {
        return;
    }
    scooby_recomp::engine::gamedata::set_iso_path(std::path::Path::new(ISO));
    use scooby_recomp::modules::*;
    macro_rules! rows {
        ($t:expr) => {
            (0..$t.len()).map(|b| format!("{:?}", $t[b])).collect::<Vec<String>>()
        };
    }
    let all: Vec<(&str, Vec<String>)> = vec![
        ("rdrmngr", rows!(rdrmngr::BUILDS)),
        ("enmyspot", rows!(enmyspot::BUILDS)),
        ("herospot", rows!(herospot::BUILDS)),
        ("enemybar", rows!(enemybar::BUILDS)),
        ("fearwave", rows!(fearwave::BUILDS)),
    ];
    let (dump, expect) = (std::env::var("X4_DUMP").ok(), std::env::var("X4_EXPECT").ok());
    let mut bad = Vec::new();
    for (name, rows) in &all {
        let s = rows.join("\n") + "\n";
        if let Some(d) = &dump {
            std::fs::create_dir_all(d).unwrap();
            std::fs::write(format!("{d}/table_{name}.txt"), &s).unwrap();
        }
        if let Some(d) = &expect {
            if std::fs::read_to_string(format!("{d}/table_{name}.txt")).unwrap_or_default() != s {
                bad.push(*name);
            }
        }
    }
    assert!(bad.is_empty(), "tables differ: {bad:?}");
}

/// WP-X4: a missing-module panic must not poison gamedata's locks (it used to poison ISO_PATH through a
/// temporary guard in the panic message, failing every later test of the shared binary with PoisonError).
#[test]
fn x4_missing_module_panic_does_not_poison() {
    if IsoFs::open(ISO).is_err() {
        return;
    }
    let r = std::panic::catch_unwind(|| scooby_recomp::engine::gamedata::module_elf("00000000"));
    assert!(r.is_err(), "module 00000000 should not exist");
    scooby_recomp::engine::gamedata::set_iso_path(std::path::Path::new(ISO));
    assert!(!scooby_recomp::engine::gamedata::module_elf("1e3ebcb2").is_empty());
}

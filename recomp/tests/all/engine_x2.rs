//! WP-X2: the per-build literal tables (`builds::BUILDS[b].k`) are read from the module ELFs on the disc.
//! Reads every used entry of every build of every family (each must decode). With X2_EXPECT=<dir>
//! (tools/rustport/x2_gen.py writes <dir>/<family>.tsv: "build entry value" of the table it replaced)
//! the values are compared with the old compiled-in tables.
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::modules;

use crate::common::ISO;

macro_rules! fam {
    ($($m:ident),*) => {
        vec![$((stringify!($m), modules::$m::builds::BUILDS.len(), Box::new(|b: usize, i: usize| modules::$m::builds::BUILDS[b].k[i]) as Box<dyn Fn(usize, usize) -> u32>, modules::$m::builds::K.entries.len())),*]
    };
}

#[test]
fn x2_builds_literals() {
    if IsoFs::open(ISO).is_err() {
        return;
    }
    let expect = std::env::var("X2_EXPECT").ok();
    for (name, nb, k, ne) in fam!(dustdevil, fireghost, ghost, hero_ghost, mummy, poltergeist, scientist, shadows, tod_beam, tomedoom, wisps, wispsourcenew, zombie, panel, hero) {
        let mut n = 0;
        for b in 0..nb {
            for i in 0..ne {
                let _ = k(b, i);
            }
        }
        if let Some(d) = &expect {
            let Ok(t) = std::fs::read_to_string(format!("{d}/{name}.tsv")) else { continue };
            for l in t.lines() {
                let v: Vec<u32> = l.split(' ').map(|x| x.parse().unwrap()).collect();
                assert_eq!(k(v[0] as usize, v[1] as usize), v[2], "{name} build {} entry {}", v[0], v[1]);
                n += 1;
            }
        }
        println!("{name}: {nb} builds x {ne} entries, {n} compared");
    }
}

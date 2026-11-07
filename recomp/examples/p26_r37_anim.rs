//! P26 R37 scratch: HAnim channels of a level's EFHAnimRes uids (ARG2.. hex): per bone, channel target + min/max.
use scooby_recomp::engine::{iso::IsoFs, runtime::EngineRuntime};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let fs = IsoFs::open("../Scooby Doo Mystery Mayhem.ISO").expect("iso");
    let mut pack = EngineRuntime::ReadLevel(&fs, &a[1], &format!("{}.P2R", a[1].replace('/', ""))).unwrap();
    let want: Vec<u32> = a[2..].iter().map(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()).collect();
    for o in pack.objects.iter_mut() {
        let Some(d) = o.as_dt() else { continue };
        let Some(h) = d.as_any().downcast_ref::<scooby_recomp::engine::anim::res::EFHAnimRes>() else { continue };
        if !want.contains(&h.base.hdr.uid) { continue; }
        let Some(an) = &h.base.anim else { continue };
        println!("res {:#x} dur {} sets {}", h.base.hdr.uid, an.duration, an.sets.len());
        for s in &an.sets {
            let mut v = Vec::new();
            for c in &s.channels {
                let (mut lo, mut hi) = (f32::MAX, f32::MIN);
                for k in 0..=20 { let x = c.curve.Evaluate(an.duration * k as f32 / 20.0); lo = lo.min(x); hi = hi.max(x); }
                if (c.id & 0xffffff) <= 2 || (c.id & 0xffffff) == 9 { v.push(format!("t{} {:.2}..{:.2}", c.id & 0xffffff, lo, hi)); }
            }
            if !v.is_empty() { println!("  bone {:#x} fl {:#x} {}", s.bone, s.flags, v.join(" ")); }
        }
    }
}

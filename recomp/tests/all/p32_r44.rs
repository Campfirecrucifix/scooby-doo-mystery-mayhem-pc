//! P32 R44: Ghidra prints `GetTime(anim1); x = in_f0; GetTime(anim2); if (in_f0 != x) SetTime(anim1, x)`
//! (one SSA input `in_f0`, so `x` and `in_f0` look interchangeable); the disasm stores anim1 -> fp+8,
//! anim2 -> fp+0xc and passes `lwc1 $f12, 0xc($fp)` = anim2's time to SetTime. Seven c2rs builds kept
//! Ghidra's `x` (= anim1's own time, a no-op SetTime), so the walk/run blend never re-synced anim1 to
//! anim2: heroFUNC_UpdateBlend_Others (scooby/shaggy scientist + security_guard EP6/L01, scooby_snorkel
//! EP4/L02B) and heroFUNC_UpdateBlend_Scooby (scooby EP1/L09, EP6/L09). The hero_ghost hand port already
//! passed anim2 (`SetTime(e, SELF, 1, b)`).
use std::path::Path;

/// Every generated `x = in_f0; in_f0 = GetTime(..); if (in_f0 != x) { .. SetTime(.., x) }` must pass `in_f0`.
#[test]
fn p32_r44_blend_settime_uses_anim2_time() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/modules");
    let (mut sites, mut bad) = (0, vec![]);
    let mut stack = vec![root];
    while let Some(d) = stack.pop() {
        for ent in std::fs::read_dir(&d).unwrap() {
            let p = ent.unwrap().path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            if p.extension().map_or(true, |x| x != "rs") {
                continue;
            }
            let src = std::fs::read_to_string(&p).unwrap();
            let lines: Vec<&str> = src.lines().collect();
            for i in 0..lines.len().saturating_sub(3) {
                let Some(x) = lines[i].trim().strip_suffix(" = in_f0;") else { continue };
                if !lines[i + 1].contains("in_f0 = APIanim::GetTime(")
                    || lines[i + 2].trim() != format!("if (in_f0 != {x}) {{")
                {
                    continue;
                }
                sites += 1;
                for l in &lines[i + 3..(i + 8).min(lines.len())] {
                    if l.contains("APIanim::SetTime(") && l.contains(&format!(", {x});")) {
                        bad.push(format!("{}:{}", p.display(), i + 1));
                    }
                }
            }
        }
    }
    assert!(sites >= 7, "expected the 7 blend sites, found {sites}");
    assert!(bad.is_empty(), "SetTime passes the stale anim1 time: {bad:?}");
}

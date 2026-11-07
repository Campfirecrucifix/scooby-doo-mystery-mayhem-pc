//! P32 R50: implicit float -> undefined4 stores. Ghidra shows `uVar1 = (*pFunctions->GetVelocity)(..)` /
//! `auStack_60[0] = (*pAnim->GetTime)(..)` / `uVar2 = (*pAnim->GetDuration)(..)` (EngineAPI.h types the
//! returns undefined4); the disasm stores the float bits (`swc1 $f0`), c2rs converted numerically
//! (`as i64 as u32`): hero velocity (+0x2e0, read back as a float by the blend / team-mode code) became a
//! denormal, heroST_Panic_Loop sent a truncated anim time in msg 138, rotatingdoors stored an int duration
//! that fstate_ROTATING compares as a float.
use std::path::Path;

fn walk(d: &Path, out: &mut Vec<(String, String)>) {
    for e in std::fs::read_dir(d).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() { walk(&p, out) } else if p.extension().map_or(false, |x| x == "rs") {
            out.push((p.display().to_string(), std::fs::read_to_string(&p).unwrap()));
        }
    }
}

#[test]
fn p32_r50_float_returns_stored_as_bits() {
    let mut files = Vec::new();
    walk(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/modules"), &mut files);
    let (mut vel, mut time, mut dur) = (0, 0, 0);
    for (p, s) in &files {
        for l in s.lines() {
            for f in ["GetVelocity(e, 0x2a000000i32, 7u32)", "APIanim::GetTime(e,", "APIanim::GetDuration(e,"] {
                if let Some(i) = l.find(f) {
                    let tail = &l[i..];
                    let end = tail.find(';').unwrap_or(tail.len());
                    assert!(!tail[..end].contains(") as i64 as u32"), "float return converted numerically: {p}: {l}");
                }
            }
            if l.contains("(LIBApiCFunctions::GetVelocity(e, 0x2a000000i32, 7u32)).to_bits()") { vel += 1 }
            if l.contains("(APIanim::GetTime(e, 0x2a000000u32, ") && l.contains(".to_bits()") && l.contains("frame + 0x30") { time += 1 }
            if l.contains("APIanim::GetDuration(e, 0x2a000000u32, a2) }).to_bits()") { dur += 1 }
        }
    }
    assert_eq!((vel, time, dur), (29, 29, 3), "heroFUNC_GetVelocity x29, heroST_Panic_Loop x29, rotatingdoors x3");
}

#[test]
fn p32_r50_lightening_frequency_is_a_float_slot() {
    // fstate_BASICMOVE_SET 0x10e3c `swc1 $f0, 0($v0)` / 0x10e8c `lwc1 $f12, 0($a1)` -> TimerSetFrequency
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/modules/ligthening_controller/b_e3eb64ce.rs");
    let s = std::fs::read_to_string(&p).unwrap();
    assert!(s.contains("as f32) + 2.0)).to_bits())"), "store must keep the float bits");
    assert!(s.contains("let a2 = e.rf(e.r32(g(e, 0x10024))); APIextra::TimerSetFrequency"), "read must be a float load");
}

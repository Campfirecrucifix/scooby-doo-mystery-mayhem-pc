//! P32 R45: guard for the Ghidra pseudo-register class (R43 unbound `in_f0`, R44 stale `in_f0` copy).
//! Audit of every compiled module (C:/Temp/p32/r45): `extraout_*` / `unaff_*` / `in_a*` / `in_v*` reads are
//! all benign and checked against the disasm - destructors pass `in_a0_lo` to a base destructor that ignores
//! its argument, frankenstein `extraout_v0_lo` = the bound `Calculate_Squared_Distance_On_Track` result (fp+0x24),
//! coll `extraout_a1..t0` were replaced by the asm-recovered $f12..$f15 glow floats, floor events
//! (`in_a1_lo`) are hand ported to single-argument ManageEvent*. `in_f0` read after an unbinding statement
//! call only in heroFUNC_MultiBits (null_printf), cameraST_CAMERA_FOLLOW (frame delta stack copy) and the
//! race controllers (bound TimerGetTime). New ones must be disasm-checked before being allowlisted.
use std::path::{Path, PathBuf};

fn compiled_module_files() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/modules");
    let modrs = std::fs::read_to_string(root.join("mod.rs")).unwrap();
    let mut stack: Vec<PathBuf> = modrs
        .lines()
        .filter_map(|l| l.trim().strip_prefix("pub mod ")?.strip_suffix(';').map(str::to_string))
        .flat_map(|m| [root.join(&m), root.join(format!("{m}.rs"))])
        .filter(|p| p.exists())
        .collect();
    let mut out = vec![];
    while let Some(p) = stack.pop() {
        if p.is_dir() {
            stack.extend(std::fs::read_dir(&p).unwrap().map(|e| e.unwrap().path()));
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
    out
}

fn word_at(l: &str, pre: &str) -> bool {
    l.match_indices(pre).any(|(i, _)| i == 0 || !l.as_bytes()[i - 1].is_ascii_alphanumeric() && l.as_bytes()[i - 1] != b'_')
}

#[test]
fn p32_r45_no_unaudited_pseudo_register_reads() {
    let files = compiled_module_files();
    assert!(files.len() > 300, "found only {} module files", files.len());
    let bad: Vec<String> = files.iter().flat_map(|p| scan_pseudo(&p.display().to_string(), &std::fs::read_to_string(p).unwrap())).collect();
    assert!(bad.is_empty(), "unaudited Ghidra pseudo-register reads: {bad:#?}");
}

fn scan_pseudo(name: &str, src: &str) -> Vec<String> {
    let mut bad = vec![];
    {
        for (n, l) in src.lines().enumerate() {
            let t = l.trim();
            if t.starts_with("//") || t.starts_with("let mut ") {
                continue;
            }
            let hit = ["extraout_", "unaff_", "in_a0", "in_a1", "in_a2", "in_a3", "in_v0", "in_v1", "in_t"]
                .iter()
                .any(|w| word_at(t, w));
            let ok = t.contains("baseDestructor(e, in_a0_lo)") || t.contains("extraout_v0_lo");
            if hit && !ok {
                bad.push(format!("{name}:{}: {t}", n + 1));
            }
        }
    }
    bad
}

#[test]
fn p32_r45_in_f0_rebound_after_statement_calls() {
    let bad: Vec<String> = compiled_module_files().iter().flat_map(|p| scan_f0(&p.display().to_string(), &std::fs::read_to_string(p).unwrap())).collect();
    assert!(bad.is_empty(), "in_f0 read after a call that did not rebind it: {bad:#?}");
}

fn scan_f0(name: &str, src: &str) -> Vec<String> {
    const AUDITED: &[&str] = &[
        "heroFUNC_MultiBits",
        "cameraST_CAMERA_FOLLOW",
        "racecontrollerST_BASICMOVE_LOOP",
        "inv_racecontrollerST_BASICMOVE_LOOP",
    ];
    let mut bad = vec![];
    {
        let (mut cur, mut last_call) = (String::new(), None::<usize>);
        for (n, l) in src.lines().enumerate() {
            let t = l.trim();
            if let Some(r) = t.strip_prefix("pub fn ").or_else(|| t.strip_prefix("fn ")) {
                cur = r.split('(').next().unwrap().to_string();
                last_call = None;
            } else if t.starts_with("in_f0 = ") {
                last_call = None;
            } else if t.starts_with("let _ = ") && t.contains("(e") {
                last_call = Some(n);
            } else if let Some(c) = last_call {
                if word_at(t, "in_f0") && !t.contains("null_printf") && !AUDITED.contains(&cur.as_str()) {
                    bad.push(format!("{name}:{} {cur}: call L{} then {t}", n + 1, c + 1));
                    last_call = None;
                }
            }
        }
    }
    bad
}

/// the scanners flag the R43 (unbound) / R44-shaped (discarded call, then read) cases and pseudo-register data reads
#[test]
fn p32_r45_scanners_flag_known_bad_shapes() {
    let f0 = "pub fn heroFUNC_UpdateBlend_Others(e: &mut Engine) {
    let _ = APIanim::GetTime(e, 0x2a000000u32, 5u32);
    if (in_f0 != fVar1) {
";
    assert_eq!(scan_f0("x", f0).len(), 1);
    assert!(scan_f0("x", &f0.replace("let _ = APIanim", "in_f0 = APIanim")).is_empty());
    assert_eq!(scan_pseudo("x", "    let _ = Foo(e, extraout_a1_lo, unaff_s0);
    let mut extraout_t0_lo: u32 = 0;
").len(), 1);
}

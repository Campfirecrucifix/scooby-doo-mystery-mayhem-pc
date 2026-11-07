//! P32 R43: c2rs left Ghidra's `in_f0` unbound where a float API return sits in a comma expression
//! (`((*pExtra->TimerGetTime)(t), 5.5 < in_f0)`), so the value compared was always 0.0. TITLE panel
//! build 81d6342a (INTRO): PanelST_BASICLOGIC's 5.5 s idle timer (random title character anims, msg 222 +
//! TimerReset) and PanelCB_NOINPUT's 1 s NOCONTROLLER window never fired. Disasm 0x2af60..0x2af70 /
//! 0x2b67c..0x2b68c: `jalr TimerGetTime; c.olt.s 5.5|1.0, $f0`.
use crate::common::*;
use scooby_recomp::engine::iso::IsoFs;

#[test]
fn p32_r43_intro_panel_idle_timer_resets() {
    if !iso_or_skip("p32_r43_intro_panel_idle_timer_resets") {
        return;
    }
    let fs = IsoFs::open(ISO).unwrap();
    let (mut e, mut rt) = load_carried(&fs, "INTRO", &[]).unwrap();
    let (mut armed, mut max_t) = (0u32, 0f32);
    for f in 0..1500 {
        frame(&mut e, &mut rt, 0);
        let Some(c) = e.rt.codes.iter().position(|c| c.def.name == "panel") else { continue };
        let data = e.rt.codes[c].data;
        let l = e.mem.r32(data + 0x13e5c); // pPanelLocalData (0x23e5c)
        if l == 0 {
            continue;
        }
        let (s380, s384, h) = (e.mem.r32(l + 0x380), e.mem.r32(l + 0x384), e.mem.r32(l + 0x3b4));
        if s380 == 2 || s384 == 20 {
            armed += 1;
            let t = scooby_recomp::engine::core::apiextra::TimerGetTime(&mut e, h);
            max_t = max_t.max(t);
            if f % 100 == 0 {
                eprintln!("f{f} 380={s380} 384={s384} t={t}");
            }
        }
    }
    eprintln!("armed {armed} frames, max idle timer {max_t}");
    assert!(armed > 600, "INTRO panel never in the idle state (armed {armed})");
    assert!(max_t < 6.0, "idle timer {max_t} > 5.5 s never reset (in_f0 unbound)");
}

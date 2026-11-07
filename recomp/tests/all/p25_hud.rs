//! P25 R58: port side of the original-vs-port HUD timelines (B8 costume ring / X prompt, B9 sandwich window + pickup HUD hide).
//! 30 Hz logic (HZ 30, one `frame` = one logic frame, as the original). Logs one line per logic frame whenever anything changes:
//! leader, GB0+0x14, GB2+0xc, courage, HUD cmd count, and for every HUD Object3D draw (go, res) its first screen vertex + colour.
//! P25_MODE=b9: Triangle at P25_PRE (default 900). pick: warp leader onto the tomato 0xbd (205.49,3.08,-1.54) at P25_PRE.
//! cos: warp to the library chest (-65.15,-3.31) for 60 f after P25_PRE, Cross 6 f. N = P25_N (default 700).
use crate::common::*;
use scooby_recomp::engine::iso::IsoFs;

#[test]
#[ignore]
fn p25_r58_hud() {
    if !iso_or_skip("p25_r58_hud") { return; }
    let _g = serial();
    let fs = IsoFs::open(ISO).unwrap();
    let (mut e, mut rt) = load_carried(&fs, "EP1/L01", &[]).unwrap();
    HZ.store(30, std::sync::atomic::Ordering::Relaxed); // after load_carried (it resets HZ to 60)
    let env = |k: &str, d: u32| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    let mode = std::env::var("P25_MODE").unwrap_or("b9".into());
    let (pre, n) = (env("P25_PRE", 900), env("P25_N", 700));
    let gb = |e: &scooby_recomp::engine::Engine, i: u32| e.rt.game_buffers.get(&i).map_or(0, |b| b.0);
    let mut last = String::new();
    for f in 0..pre + n {
        let rel = f as i64 - pre as i64;
        let mut b = 0u16;
        match mode.as_str() {
            "b9" if (0..6).contains(&rel) => b = 0x1000,
            "pick" if rel == 0 => warp(&mut e, &rt, [205.49, 3.2, -1.54]),
            "cos" if (0..60).contains(&rel) => warp(&mut e, &rt, [-65.15, 0.3, -3.31]),
            "cos" if (60..66).contains(&rel) => b = 0x4000,
            _ => {}
        }
        // P26 R10 pickw: as the original R58 run (stick held throughout): walk into the tomato
        if mode == "pickw" {
            if rel == -12 { warp(&mut e, &rt, [203.49, 3.2, -1.54]); }
            let st = if rel >= -10 { std::env::var("P25_STICK").ok().map(|v| { let mut i = v.split(','); (i.next().unwrap().parse().unwrap(), i.next().unwrap().parse().unwrap()) }).unwrap_or((255u8, 128u8)) } else { (128, 128) };
            frame_stick(&mut e, &mut rt, b, st);
        } else {
            frame(&mut e, &mut rt, b);
        }
        if rel < -5 { continue; }
        let (g0, g1, g2) = (gb(&e, 0), gb(&e, 1), gb(&e, 2));
        let mut s = format!("lead {:x} g14 {:x} gb2c {:x} cool {:.2} hud {}", leader(&e) & 0xfff, e.mem.r32(g0 + 0x14),
            if g2 != 0 { e.mem.r32(g2 + 0xc) } else { 0 }, e.mem.rf(g1), rt.frame.hud.len());
        let mut objs: Vec<String> = rt.frame.hud.iter().filter_map(|c| if let scooby_recomp::engine::hud::HudTex::Object3D(a, r) = c.tex {
            Some(format!("{:x}/{:x} {:.0},{:.0}", a & 0xffffff, r, c.pos[0][0], c.pos[0][1])) } else { None }).collect();
        objs.dedup();
        if std::env::var_os("P25_ALL").is_some() {
            for c in rt.frame.hud.iter().filter(|c| !matches!(c.tex, scooby_recomp::engine::hud::HudTex::Object3D(..))) {
                let k = format!("{:?}", c.tex); s += &format!(" # {} {:.0},{:.0}", &k[..k.len().min(14)], c.pos[0][0], c.pos[0][1]);
            }
        }
        let p = rt.particles.lock().unwrap();
        for o in objs {
            s += " | "; s += &o;
            let mut it = o.split(|c| c == '/' || c == ' ');
            let (a, r) = (u32::from_str_radix(it.next().unwrap(), 16).unwrap() | 0x2a000000, u32::from_str_radix(it.next().unwrap(), 16).unwrap());
            if let Some(pose) = p.sub.PoseOf(a, r) { for (i, t) in pose.iter().enumerate().take(3) { s += &format!(" t{i} {:.3},{:.3},{:.3}", t.f[3], t.f[4], t.f[5]); } }
        }
        drop(p);
        for d in &rt.frame_gfx_deferred {
            if let scooby_recomp::engine::gfx::api::GfxDeferred::SetBlendingColor { game_obj, obj3d_index, rgba } = d { s += &format!(" || blend {game_obj:x}/{obj3d_index} {rgba:?}"); }
        }
        if s != last { println!("rel{rel} {s}"); last = s; }
    }
}

//! P28: playtest HUD bugs (B8/B10) probes.
use crate::common::*;
use scooby_recomp::engine::iso::IsoFs;

#[test]
#[ignore]
fn p28_probe_hud() {
    if !iso_or_skip("p28_probe_hud") { return; }
    let _g = serial();
    let fs = IsoFs::open(ISO).unwrap();
    let (mut e, mut rt) = load_carried(&fs, "EP1/L01", &[]).unwrap();
    let pre: u32 = std::env::var("P28_PRE").ok().and_then(|v| v.parse().ok()).unwrap_or(300);
    for _ in 0..pre { frame(&mut e, &mut rt, 0); }
    {
        let p = rt.particles.lock().unwrap();
        let mut k: Vec<_> = p.world.frames.keys().filter(|&&u| u >> 24 == 0xf).copied().collect();
        k.sort();
        println!("0xf frames {k:x?}");
        for (o, l) in p.sub.obj3d_list.iter().filter(|(_, l)| l.len() > 3) { println!("obj3d_list {o:#x} {l:x?}"); }
    }
    {
        let p = rt.particles.lock().unwrap();
        for (o, i) in &p.hud3d { println!("hud3d {o:#x} idx {i} obj3d {:x?} hidden {} n3d {:?}", p.sub.obj3d.get(o), p.world.hidden.contains(o), p.sub.obj3d_list.get(o).map(|l| l.len())); }
        let mut d: Vec<_> = p.sub.obj3d_desc.iter().collect(); d.sort();
        println!("desc {d:x?}");
    }
    println!("hud cmds {}", rt.frame.hud.len());
    for c in &rt.frame.hud { if let scooby_recomp::engine::hud::HudTex::Object3D(a, b) = c.tex { println!("obj3d {a:#x} {b:#x}"); } }
    let gb0 = e.rt.game_buffers.get(&0).unwrap().0;
    let gb1 = e.rt.game_buffers.get(&1).unwrap().0;
    println!("gb0+14 {:#x} courage {}", e.mem.r32(gb0 + 0x14), e.mem.rf(gb1));
    e.mem.wf(gb1, 0.3);
    let needle = |rt: &scooby_recomp::engine::runtime::EngineRuntime| { let p = rt.particles.lock().unwrap(); let mut v: Vec<_> = p.world.frames.iter().filter(|(u, _)| *u >> 24 == 0xf).map(|(u, f)| format!("{u:#x} {f:?}")).collect(); v.sort(); v };
    println!("B10 needle courage 1.0: {:?}", needle(&rt));
    for _ in 0..120 { frame(&mut e, &mut rt, 0); println!("defer {:?}", rt.frame_gfx_deferred.len()); }
    println!("B10 needle courage 0.3: {:?}", needle(&rt));
}

/// B9: sandwich ingredient windows: blend colours, window objects, anims around a Triangle press.
#[test]
#[ignore]
fn p28_probe_b9() {
    if !iso_or_skip("p28_probe_b9") { return; }
    let _g = serial();
    let fs = IsoFs::open(ISO).unwrap();
    let (mut e, mut rt) = load_carried(&fs, "EP1/L01", &[]).unwrap();
    let dump = |e: &scooby_recomp::engine::Engine, rt: &scooby_recomp::engine::runtime::EngineRuntime, f: u32| {
        for d in &rt.frame_gfx_deferred {
            if let scooby_recomp::engine::gfx::api::GfxDeferred::SetBlendingColor { .. } = d { println!("f{f} {d:x?}"); }
        }
        let v: Vec<_> = rt.frame.hud.iter().filter_map(|c| if let scooby_recomp::engine::hud::HudTex::Object3D(a, b) = c.tex { Some((a, b, c.pos[0])) } else { None }).collect();
        let mut p = rt.particles.lock().unwrap();
        for a in p.sub.log.anim_play.drain(..) { if a.0 >> 20 == 0x2a1 || a.0 == 0x2a00007e || a.0 == 0x2a000001 { println!("f{f} anim_play {a:x?}"); } }
        if f % 50 == 0 { let mut h: Vec<_> = p.hud3d.iter().map(|(a, b)| (*a, *b)).collect(); h.sort(); println!("f{f} hud {} hud3d {h:x?}", rt.frame.hud.len()); }
        if std::env::var_os("P28_Y").is_some() && f >= 2395 && f <= 2700 {
            let mut ys = vec![];
            for (a, b, _) in &v { if let Some(pose) = p.sub.PoseOf(*a, *b) { ys.push(format!("{a:x}/{b:x}:{}", pose.iter().map(|t| format!("{:.3}", t.f[4])).collect::<Vec<_>>().join(","))); } }
            println!("Y f{f} {}", ys.join(" "));
        }
        if f % 10 == 0 {
            println!("f{f} obj3d {:x?}", v);
            for (a, b, _) in &v {
                if let Some(pose) = p.sub.PoseOf(*a, *b) { println!("   pose {a:#x} {b:#x} n{} t0 {:?}", pose.len(), pose.first().map(|t| t.f)); }
            }
        }
        let _ = e;
    };
    {
        let p = rt.particles.lock().unwrap();
        for (o, l) in p.sub.obj3d_list.iter().filter(|(_, l)| l.len() >= 5) { println!("obj3d_list {o:#x} {l:x?}"); }
    }
    let pre: u32 = std::env::var("P28_PRE").ok().and_then(|v| v.parse().ok()).unwrap_or(900);
    for f in 0..pre { frame(&mut e, &mut rt, 0); dump(&e, &rt, f); }
    for f in pre..pre + 6 { frame(&mut e, &mut rt, 0x1000); dump(&e, &rt, f); }
    for f in pre + 6..pre + 300 { frame(&mut e, &mut rt, 0); dump(&e, &rt, f); }
}

/// B11: shadow monster 0x2a000013 chase: hero warped into its corridor, then to points outside.
#[test]
#[ignore]
fn p28_probe_b11() {
    if !iso_or_skip("p28_probe_b11") { return; }
    let _g = serial();
    let fs = IsoFs::open(ISO).unwrap();
    let (mut e, mut rt) = load_carried(&fs, "EP1/L01", &[]).unwrap();
    const SH: u32 = 0x2a00_0013;
    let pre: u32 = std::env::var("P28_PRE").ok().and_then(|v| v.parse().ok()).unwrap_or(900);
    for _ in 0..pre { frame(&mut e, &mut rt, 0); }
    let s0 = pos(&mut e, &rt, SH).unwrap();
    println!("leader {:#x} cs {:?}", leader(&e), rt.particles.lock().unwrap().cutscene.as_ref().map(|c| (c.current, c.flags)));
    println!("shadow start {s0:?}");
    let spots: Vec<[f32; 3]> = std::env::var("P28_SPOTS").ok().map(|v| v.split(';').map(|t| { let a: Vec<f32> = t.split(',').map(|x| x.parse().unwrap()).collect(); [a[0], s0[1], a[1]] }).collect())
        .unwrap_or(vec![[310.0, s0[1], 1.5], [298.6, s0[1], 2.6], [313.9, s0[1], 0.1], [320.0, s0[1], 0.0], [330.0, s0[1], 0.0], [290.0, s0[1], 0.0], [280.0, s0[1], 0.0]]);
    let mut f = 0;
    let n: usize = std::env::var("P28_N").ok().and_then(|v| v.parse().ok()).unwrap_or(240);
    for sp in spots {
        for i in 0..n {
            warp(&mut e, &rt, sp);
            frame(&mut e, &mut rt, 0);
            f += 1;
            if i % 30 == 29 {
                let s = pos(&mut e, &rt, SH).unwrap_or([0.0; 3]);
                let ld = leader(&e);
                let h = pos(&mut e, &rt, ld).unwrap_or([0.0; 3]);
                let st = e.rt.map.find(SH).map(|_| scooby_recomp::engine::core::apigameobj::GetCurrentStateID(&mut e, SH));
                let d = ((s[0] - h[0]).powi(2) + (s[2] - h[2]).powi(2)).sqrt();
                println!("f{f} hero {:?} shadow {:?} d {d:.2} state {st:x?}", h, s);
            }
        }
    }
}

/// B8: chest 0x13f costume put-on in the library: panel act prompt (Put on / Remove) messages.
#[test]
#[ignore]
fn p28_probe_cos() {
    if !iso_or_skip("p28_probe_cos") { return; }
    let _g = serial();
    let fs = IsoFs::open(ISO).unwrap();
    let (mut e, mut rt) = load_carried(&fs, "EP1/L01", &[]).unwrap();
    for _ in 0..1600 { frame(&mut e, &mut rt, 0); }
    for _ in 0..60 { warp(&mut e, &rt, [-65.15, 0.3, -3.31]); frame(&mut e, &mut rt, 0); }
    let gb0 = e.rt.game_buffers.get(&0).unwrap().0;
    let gb2 = e.rt.game_buffers.get(&2).map(|b| b.0);
    let mut last = u32::MAX;
    for f in 0..700 {
        let b = if (40..46).contains(&f) { 0x4000 } else { 0 };
        frame(&mut e, &mut rt, b);
        if f % 20 == 0 { eprintln!("f{f} g14 {:#x} leader {:#x}", e.mem.r32(gb0 + 0x14), leader(&e)); }
        if let Some(g2) = gb2 {
            let v = e.mem.r32(g2) & 0x20000;
            if v != last { eprintln!("f{f} GB2 prompt bit {v:#x} leader {:#x}", leader(&e)); last = v; }
        }
    }
}

/// B12: ghost bob (Ghost_base Waving, per logic frame): y per logic frame at real-time pacing (dt 2 vsyncs),
/// peak spacing in frames / seconds for every object that bobs within +-0.15 of a base.
#[test]
#[ignore]
fn p28_probe_b12() {
    if !iso_or_skip("p28_probe_b12") { return; }
    let _g = serial();
    let fs = IsoFs::open(ISO).unwrap();
    let lv = std::env::var("P28_LEVEL").unwrap_or("EP1/L01".into());
    let (mut e, mut rt) = load_carried(&fs, &lv, &[]).unwrap();
    HZ.store(30, std::sync::atomic::Ordering::Relaxed);
    for _ in 0..100 { frame(&mut e, &mut rt, 0); }
    let mut ys: std::collections::BTreeMap<u32, Vec<f32>> = Default::default();
    for _ in 0..300 {
        frame(&mut e, &mut rt, 0);
        let p = rt.particles.lock().unwrap();
        for (&u, f) in &p.world.frames { ys.entry(u).or_default().push(f.pos[1]); }
    }
    for (u, y) in &ys {
        if y.len() < 300 { continue; }
        let (lo, hi) = y.iter().fold((f32::MAX, f32::MIN), |(a, b), &v| (a.min(v), b.max(v)));
        if !(0.05..0.32).contains(&(hi - lo)) { continue; }
        let pk: Vec<usize> = (1..y.len() - 1).filter(|&i| y[i] > y[i - 1] && y[i] >= y[i + 1]).collect();
        let sp: Vec<usize> = pk.windows(2).map(|w| w[1] - w[0]).collect();
        let mean = if sp.is_empty() { 0.0 } else { sp.iter().sum::<usize>() as f32 / sp.len() as f32 };
        println!("B12 {u:#x} range {:.3} peaks {} spacing {sp:?} mean {mean:.1} f = {:.2} s", hi - lo, pk.len(), mean * 2.0 * 1001.0 / 60000.0);
        if *u >> 24 == 0x2a || pk.len() > 2 { println!("B12 {u:#x} y {:?}", &y[..60].iter().map(|v| (v * 1000.0).round() / 1000.0).collect::<Vec<_>>()); }
    }
}

/// B11 R8: shadow monsters of a level (P28_LEVEL) + chase of P28_SH with the hero pinned at P28_SPOT "x,z" (y of the
/// shadow) for P28_N frames after P28_PRE; logs every P28_EVERY frames: monster pos, distance, state.
#[test]
#[ignore]
fn p28_probe_b11b() {
    if !iso_or_skip("p28_probe_b11b") { return; }
    let _g = serial();
    let fs = IsoFs::open(ISO).unwrap();
    let lv = std::env::var("P28_LEVEL").unwrap_or("EP1/L01".into());
    let (mut e, mut rt) = load_carried(&fs, &lv, &[]).unwrap();
    let env = |k: &str, d: u32| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    for _ in 0..env("P28_PRE", 300) { frame(&mut e, &mut rt, 0); }
    let ids: Vec<(u32, String)> = e.rt.objects.iter().flatten().filter_map(|o| {
        let n = o.inst.code.map(|k| e.rt.codes[k].def.name.to_string())?;
        n.starts_with("shadows").then_some((o.id, n))
    }).collect();
    for (u, n) in &ids { let p = pos(&mut e, &rt, *u); println!("shadow {u:#x} {n} pos {p:?}"); }
    let Some(sh) = std::env::var("P28_SH").ok().and_then(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16).ok()) else { return };
    let s0 = pos(&mut e, &rt, sh).unwrap();
    let sp: Vec<f32> = std::env::var("P28_SPOT").unwrap().split(',').map(|x| x.parse().unwrap()).collect();
    let to = [sp[0], if sp.len() > 2 { sp[2] } else { s0[1] }, sp[1]];
    let every = env("P28_EVERY", 15);
    for f in 0..env("P28_N", 600) {
        warp(&mut e, &rt, to);
        frame(&mut e, &mut rt, 0);
        if f % every == 0 {
            let s = pos(&mut e, &rt, sh).unwrap_or([0.0; 3]);
            let ld = leader(&e);
            let h = pos(&mut e, &rt, ld).unwrap_or([0.0; 3]);
            let st = scooby_recomp::engine::core::apigameobj::GetCurrentStateID(&mut e, sh);
            let d = ((s[0] - h[0]).powi(2) + (s[2] - h[2]).powi(2)).sqrt();
            println!("B11 f{f} hero ({:.2},{:.2},{:.2}) shadow ({:.2},{:.2},{:.2}) d {d:.2} state {st:#x}", h[0], h[1], h[2], s[0], s[1], s[2]);
        }
    }
}

/// R9 B6/B7 movement probe: P28_LEVEL, P28_PRE idle frames, then P28_SCRIPT steps separated by ';':
/// `c` Circle (6 f), `x` Cross (6 f), `t` Triangle, `i:N` idle N, `w:x,y,z` warp leader (+buddy 1 m east),
/// `s:lx,ly,N` stick for N frames, `sx:lx,ly,N` stick + Cross held. Logs both heroes (pos, up, GB0+14, leader)
/// every frame where GB0+14 / leader changes, else every P28_EVERY frames.
#[test]
#[ignore]
fn p28_probe_move() {
    if !iso_or_skip("p28_probe_move") { return; }
    let _g = serial();
    let fs = IsoFs::open(ISO).unwrap();
    let lv = std::env::var("P28_LEVEL").unwrap_or("EP1/L01".into());
    let (mut e, mut rt) = load_carried(&fs, &lv, &[]).unwrap();
    let env = |k: &str, d: u32| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    HZ.store(env("P28_HZ", 60), std::sync::atomic::Ordering::Relaxed);
    for _ in 0..env("P28_PRE", 600) { frame(&mut e, &mut rt, 0); }
    let gb0 = e.rt.game_buffers.get(&0).unwrap().0;
    let every = env("P28_EVERY", 10);
    let (h1, h2) = (e.mem.r32(gb0 + 8), e.mem.r32(gb0 + 0xc));
    println!("heroes leader {h1:#x} buddy {h2:#x}");
    let mut f = 0u32;
    let mut last = (0u32, 0u32);
    let mut step = |e: &mut scooby_recomp::engine::Engine, rt: &mut scooby_recomp::engine::runtime::EngineRuntime, b: u16, st: (u8, u8), tag: &str| {
        frame_stick(e, rt, b, st);
        f += 1;
        let k = (e.mem.r32(gb0 + 0x14), e.mem.r32(gb0 + 8));
        if k != last || f % every == 0 {
            last = k;
            let fr = |rt: &scooby_recomp::engine::runtime::EngineRuntime, u: u32| rt.particles.lock().unwrap().world.frames.get(&u).map(|m| (m.pos, m.up));
            let (a, c) = (fr(rt, h1), fr(rt, h2));
            let s = |x: Option<([f32; 3], [f32; 3])>| x.map(|(p, u)| format!("({:.3},{:.3},{:.3}) up({:.2},{:.2},{:.2})", p[0], p[1], p[2], u[0], u[1], u[2])).unwrap_or_default();
            let b = e.mem.alloca(0x10);
            rt.particles.lock().unwrap().logic = true;
            let hr = scooby_recomp::engine::APIgameobj::GetPosition(e, k.1, 0x250_0000, b);
            rt.particles.lock().unwrap().logic = false;
            let hand = if hr != 0 { let v = e.mem.rv3(b); format!("hand({:.3},{:.3},{:.3})", v[0], v[1], v[2]) } else { "hand-".into() };
            println!("MV f{f} {tag} g14 {:#x} leader {:#x} {h1:#x} {} {h2:#x} {} {hand}", k.0, k.1, s(a), s(c));
        }
    };
    for st in std::env::var("P28_SCRIPT").unwrap_or_default().split(';').filter(|s| !s.is_empty()) {
        let (op, arg) = st.split_once(':').unwrap_or((st, ""));
        let n: Vec<f32> = arg.split(',').filter(|s| !s.is_empty()).map(|x| x.parse().unwrap()).collect();
        match op {
            "c" | "x" | "t" => { let b = match op { "c" => 0x2000, "x" => 0x4000, _ => 0x1000 }; for _ in 0..6 { step(&mut e, &mut rt, b, (128, 128), st); } }
            "i" => for _ in 0..n[0] as u32 { step(&mut e, &mut rt, 0, (128, 128), st); },
            "w" => { warp(&mut e, &rt, [n[0], n[1], n[2]]); step(&mut e, &mut rt, 0, (128, 128), st); }
            // wf: warp the follower (GB0+0xc) alone
            "wf" => {
                let fl = e.mem.r32(gb0 + 0xc);
                let p = e.mem.alloca(0x10);
                for i in 0..3 { e.mem.w32(p + 4 * i as u32, n[i].to_bits()); }
                rt.particles.lock().unwrap().logic = true;
                scooby_recomp::engine::APIgameobj::SetTranslation(&mut e, fl, p);
                rt.particles.lock().unwrap().logic = false;
                step(&mut e, &mut rt, 0, (128, 128), st);
            }
            "s" | "sx" => { let b = if op == "sx" { 0x4000 } else { 0 }; for _ in 0..n[2] as u32 { step(&mut e, &mut rt, b, (n[0] as u8, n[1] as u8), st); } }
            _ => panic!("bad step {st}"),
        }
    }
}

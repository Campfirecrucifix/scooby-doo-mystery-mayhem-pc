//! WP-P11: Episode 2 scripted play. Runs the per-level step routes of src/bin/routes_ep2.rs through
//! real level switches (game buffers carried as the C EFGameMemoryMgr does) and reports the frame of
//! each level request. Step machinery copied from tests/engine_r7.rs (plus `c UID`: walk-free pickup =
//! leader onto the object for 30 frames). `cargo test --release --test engine_p11 -- --ignored --nocapture`
//! P11_FROM=<level> starts the chain later (fresh buffers, or R10_CARRY-style dump in P11_CARRY);
//! P11_STEPS="<level>|<steps>" runs one ad-hoc script.
#[path = "../src/bin/routes_ep2.rs"]
#[allow(dead_code)]
mod routes_ep2;
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::runtime::*;
use scooby_recomp::engine::APIgameobj;

const ISO: &str = "../Scooby Doo Mystery Mayhem.ISO";
type Eng = scooby_recomp::engine::Engine;

fn leader(e: &Eng) -> u32 {
    e.rt.game_buffers.get(&0).map(|b| e.mem.r32(b.0 + 8)).unwrap_or(0)
}

fn pos(e: &mut Eng, rt: &EngineRuntime, id: u32) -> Option<[f32; 3]> {
    if let Some(m) = rt.particles.lock().unwrap().world.frames.get(&id) {
        return Some(m.pos);
    }
    let b = e.mem.alloca(0x10);
    rt.particles.lock().unwrap().logic = Some(&e.rt as *const _);
    let r = APIgameobj::GetPosition(e, id, 0, b);
    rt.particles.lock().unwrap().logic = None;
    (r != 0).then(|| e.mem.rv3(b))
}

fn warp(e: &mut Eng, rt: &EngineRuntime, to: [f32; 3]) {
    let l = leader(e);
    let p = e.mem.alloca(0x10);
    for (i, c) in to.iter().enumerate() {
        e.mem.w32(p + 4 * i as u32, c.to_bits());
    }
    rt.particles.lock().unwrap().logic = Some(&e.rt as *const _);
    APIgameobj::SetTranslation(e, l, p);
    // the buddy (GB0+0xc) comes along, 1 m aside (as engine_r3): doors need both heroes (GB0+0x34 0x110)
    let o = e.rt.game_buffers.get(&0).map(|b| e.mem.r32(b.0 + 0xc)).unwrap_or(0);
    if o != 0 && o != l {
        e.mem.w32(p, (to[0] + 1.0).to_bits());
        APIgameobj::SetTranslation(e, o, p);
    }
    rt.particles.lock().unwrap().logic = None;
}

fn send(e: &mut Eng, rt: &EngineRuntime, uid: u32, m: u32, p2: u32, data: &[u32]) {
    let d = if data.is_empty() { 0 } else {
        let p = e.mem.alloca(0x40.max(4 * data.len() as u32));
        for (i, w) in data.iter().enumerate() {
            e.mem.w32(p + 4 * i as u32, *w);
        }
        p
    };
    rt.particles.lock().unwrap().logic = Some(&e.rt as *const _);
    APIgameobj::SendMessageTo(e, uid, 0, m, p2, d);
    rt.particles.lock().unwrap().logic = None;
}

fn cutscene_update(e: &mut Eng, rt: &mut EngineRuntime) {
    use scooby_recomp::engine::script::RuntimeCutSceneHost;
    let (cs, reqs) = {
        let mut p = rt.particles.lock().unwrap();
        (p.cutscene.take(), std::mem::take(&mut p.cutscene_req))
    };
    let Some(mut cs) = cs else { return };
    let mut h = RuntimeCutSceneHost::new(e, rt);
    for q in reqs {
        match q {
            CutSceneReq::Play(id, snd, pos) => cs.Play(&mut h, id, snd, pos),
            CutSceneReq::Stop => cs.Stop(&mut h),
            CutSceneReq::Skip => cs.Skip(&mut h),
        }
    }
    cs.Update(&mut h, 1.0 / 60.0);
    rt.particles.lock().unwrap().cutscene = Some(cs);
}

fn frame(e: &mut Eng, rt: &mut EngineRuntime, buttons: u16) -> Option<scooby_recomp::engine::host::game::LevelRequest> {
    use scooby_recomp::engine::input::PadState;
    rt.particles.lock().unwrap().devices.SetPadState(0, PadState { connected: true, mode: 7, rx: 128, ry: 128, lx: 128, ly: 128, buttons, ..Default::default() });
    rt.Frame(e, 1.0 / 60.0);
    cutscene_update(e, rt);
    rt.particles.lock().unwrap().game.take_level_request()
}

fn carry(e: &Eng) -> Vec<(u32, Vec<u8>)> {
    e.rt.game_buffers.iter().map(|(&id, &(p, _))| (id, (0..e.mem.r32(p - 0x10)).map(|i| e.mem.r8(p + i)).collect())).collect()
}

fn load_carried(fs: &IsoFs, dir: &str, bufs: &[(u32, Vec<u8>)]) -> Result<(Eng, EngineRuntime), String> {
    let (mut e, mut rt) = EngineRuntime::new_with_engine();
    let file = format!("{}.P2R", dir.rsplit('/').next().filter(|_| !dir.starts_with("EP")).map(str::to_string).unwrap_or_else(|| dir.replace('/', "")));
    let res = EngineRuntime::ReadLevel(fs, dir, &file)?;
    let cs = scooby_recomp::engine::script::EFCutScenePlayer::FromLevelObjects(&res.objects).ok();
    for (id, b) in bufs {
        let p = scooby_recomp::engine::host::gameobj::GetGameBuffer(&mut e, *id, b.len() as u32, 1);
        for (i, v) in b.iter().enumerate() {
            e.mem.w8(p + i as u32, *v);
        }
    }
    rt.LoadLevel(&mut e, res);
    rt.particles.lock().unwrap().cutscene = cs;
    Ok((e, rt))
}

fn norm(l: &str) -> String {
    let d = l.replace(char::from(0x5c), "/").trim_matches('/').to_string();
    if d.contains('/') || d.len() < 6 || !d.starts_with("EP") { d } else { format!("{}/{}", &d[..3], &d[3..]) }
}

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or_else(|_| panic!("bad hex {s}"))
}

/// object uid: hex, or `@code` = first object whose logic code name starts with `code`
fn uid(e: &Eng, s: &str) -> u32 {
    match s.strip_prefix('@') {
        Some(c) => e.rt.objects.iter().flatten().find(|o| o.inst.code.is_some_and(|k| e.rt.codes[k].def.name.to_lowercase().starts_with(c))).unwrap_or_else(|| panic!("no object with code {c}")).id,
        None => hex(s),
    }
}

/// Step script, ';'-separated:
/// `f N` frames; `x N` frames pressing X (3 of every 20), re-warping to the last warp target each 60;
/// `p BTN N` frames holding pad bits BTN (hex); `m UID MSG P2 [W..]` SendMessageTo (data words);
/// `warp UID` leader onto the object; `l UID OFF VAL` write local+OFF of UID; `g ID OFF VAL` write
/// game buffer ID +OFF; `dbg UID` print local words; `dm ADDR LEN` dump memory. Runs until a level request (stops the script).
fn run(e: &mut Eng, rt: &mut EngineRuntime, steps: &str) -> Option<(String, u32)> {
    let (e, rt) = (e, rt);
    let mut f = 0u32;
    let mut at: Option<[f32; 3]> = None;
    let mut got = None;
    'outer: for st in steps.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        let a: Vec<&str> = st.split_whitespace().collect();
        let (btn, n) = match a[0] {
            "f" => (0u16, a[1].parse::<u32>().unwrap()),
            "x" => (scooby_recomp::engine::input::pad::CROSS, a[1].parse().unwrap()),
            "p" => (hex(a[1]) as u16, a[2].parse().unwrap()),
            "m" => {
                let d: Vec<u32> = a[4..].iter().map(|w| if w.starts_with('@') { uid(e, w) } else { hex(w) }).collect();
                let u = uid(e, a[1]);
                send(e, rt, u, hex(a[2]), hex(a[3]), &d);
                (0, 0)
            }
            "warp" => {
                let u = uid(e, a[1]);
                at = pos(e, rt, u);
                println!("P11  warp {u:#x} -> {at:?} leader {:#x}", leader(e));
                if let Some(p) = at { warp(e, rt, p) }
                (0, 0)
            }
            "l" => {
                let u = uid(e, a[1]);
                let lo = e.rt.objects.iter().flatten().find(|o| o.id == u).map_or(0, |o| o.inst.local);
                e.mem.w32(lo + hex(a[2]), hex(a[3]));
                (0, 0)
            }
            "g" => {
                let b = e.rt.game_buffers.get(&hex(a[1])).map(|b| b.0).expect("game buffer");
                e.mem.w32(b + hex(a[2]), hex(a[3]));
                (0, 0)
            }
            "ls" => {
                for o in e.rt.objects.iter().flatten() {
                    if let Some(k) = o.inst.code {
                        let n = &e.rt.codes[k].def.name;
                        if n.to_lowercase().contains(a[1]) {
                            println!("P11  obj {:#x} {n} build {} local {:#x}", o.id, e.rt.codes[k].build, o.inst.local);
                        }
                    }
                }
                (0, 0)
            }
            "gd" => {
                let b = e.rt.game_buffers.get(&hex(a[1])).map(|b| b.0).expect("game buffer");
                let w: Vec<String> = (0..hex(a[2]) / 4).map(|i| format!("{:x}", e.mem.r32(b + 4 * i))).collect();
                println!("P11  f{f} gb{} {}", a[1], w.join(" "));
                (0, 0)
            }
            "play" => {
                // EFCutScenePlayer::Play(id) as APIgame::PlayCutScene (no sound): the scripted cut-scene
                rt.particles.lock().unwrap().cutscene_req.push(CutSceneReq::Play(hex(a[1]), 0xffff_ffff, 0x7fff_ffff));
                (0, 0)
            }
            "cs" => {
                let g = rt.particles.lock().unwrap();
                println!("P11  f{f} cutscene {:?}", g.cutscene.as_ref().map(|c| (c.current, c.flags, c.time, c.duration, c.scene(c.current).map(|s| s.actors.clone()))));
                (0, 0)
            }
            "wxyz" => {
                // warp the leader to a world position (re-applied by `x` like `warp`)
                at = Some([a[1].parse().unwrap(), a[2].parse().unwrap(), a[3].parse().unwrap()]);
                warp(e, rt, at.unwrap());
                (0, 0)
            }
            "put" => {
                // move object A onto object B (hot spot onto a vehicle whose spline overrides warps)
                let (a1, b1) = (uid(e, a[1]), uid(e, a[2]));
                if let Some(to) = pos(e, rt, b1) {
                    let p = e.mem.alloca(0x10);
                    for (i, c) in to.iter().enumerate() {
                        e.mem.w32(p + 4 * i as u32, c.to_bits());
                    }
                    rt.particles.lock().unwrap().logic = Some(&e.rt as *const _);
                    APIgameobj::SetTranslation(e, a1, p);
                    rt.particles.lock().unwrap().logic = None;
                }
                (0, 0)
            }
            "phys" => {
                let u = uid(e, a[1]);
                let g = rt.particles.lock().unwrap();
                let id = g.sub.physic.PhysicOf(u);
                match g.sub.physic.Physic(id) {
                    Some(p) => println!("P11  f{f} phys {u:#x} id {id} flags {:#x} state {:#x} notify {:#x} masks {:x?} prims {:?}", p.flags, p.state, p.notify_bits, p.model.masks, p.model.prims.iter().map(|q| q.as_ref().map(|q| format!("{:?}", q.base))).collect::<Vec<_>>()),
                    None => println!("P11  f{f} phys {u:#x}: no physic"),
                }
                (0, 0)
            }
            "dbg" => {
                let u = uid(e, a[1]);
                let lo = e.rt.objects.iter().flatten().find(|o| o.id == u).map_or(0, |o| o.inst.local);
                let w: Vec<String> = (0..hex(a.get(2).unwrap_or(&"20")) / 4).map(|i| format!("{:x}", e.mem.r32(lo + 4 * i))).collect();
                println!("P11  f{f} {u:#x} local {}", w.join(" "));
                (0, 0)
            }
            "dm" => {
                // dump memory: `dm ADDR LEN` (hex), e.g. a hero's pLocalDataHero (local word 0)
                let w: Vec<String> = (0..hex(a[2]) / 4).map(|i| format!("{:x}", e.mem.r32(hex(a[1]) + 4 * i))).collect();
                println!("P11  f{f} mem {} {}", a[1], w.join(" "));
                (0, 0)
            }
            "pos" => {
                let u = uid(e, a[1]);
                let b = e.mem.alloca(0x10);
                rt.particles.lock().unwrap().logic = Some(&e.rt as *const _);
                let r = APIgameobj::GetPosition(e, u, 0, b);
                rt.particles.lock().unwrap().logic = None;
                let wf = rt.particles.lock().unwrap().world.frames.get(&u).map(|m| m.pos);
                println!("P11  f{f} pos {u:#x} api {r} {:?} frame {wf:?}", e.mem.rv3(b));
                (0, 0)
            }
            "nt" => {
                let u = uid(e, a[2]);
                println!("P11  f{f} notify {} {u:#x}: {:?}", a[1], e.rt.notify.maps[hex(a[1]) as usize].get(u).map(|l| l.iter().map(|n| (n.owner, n.data.cb)).collect::<Vec<_>>()));
                (0, 0)
            }
            "pl" => {
                let l = leader(e);
                println!("P11  f{f} leader {l:#x} at {:?}", pos(e, rt, l));
                (0, 0)
            }
            "c" => {
                // pickup: leader onto the object, 30 frames (no re-warp)
                let u = uid(e, a[1]);
                if let Some(p) = pos(e, rt, u) { warp(e, rt, p) }
                at = None;
                (0, 30)
            }
            s => panic!("bad step {s}"),
        };
        for i in 0..n {
            let b = if a[0] == "x" && i % 20 >= 3 { 0 } else { btn };
            if a[0] == "x" && i % 60 == 59 {
                // only when pushed away (> 3 m, as engine_r3): an unconditional re-warp puts the
                // leader in the air right before the next X press (X is ignored while falling)
                let l = leader(e);
                if let Some(p) = at.filter(|p| pos(e, rt, l).is_some_and(|q| (0..3).map(|i| (q[i] - p[i]).powi(2)).sum::<f32>() > 9.0)) { warp(e, rt, p) }
            }
            f += 1;
            if let Some(r) = frame(e, rt, b) {
                got = Some((norm(&r.level), f));
                break 'outer;
            }
        }
    }
    got
}

fn load_dump(path: &str) -> Vec<(u32, Vec<u8>)> {
    let d = std::fs::read(path).expect("carry dump");
    let mut v = vec![];
    let mut i = 0;
    while i + 8 <= d.len() {
        let id = u32::from_le_bytes(d[i..i + 4].try_into().unwrap());
        let n = u32::from_le_bytes(d[i + 4..i + 8].try_into().unwrap()) as usize;
        v.push((id, d[i + 8..i + 8 + n].to_vec()));
        i += 8 + n;
    }
    v
}

#[test]
#[ignore]
fn p11_ep2_chain() {
    let fs = IsoFs::open(ISO).expect("ISO missing");
    if let Ok(s) = std::env::var("P11_STEPS") {
        let (dir, steps) = s.split_once('|').expect("P11_STEPS=<level>|<steps>");
        let bufs = std::env::var("P11_CARRY").map(|p| load_dump(&p)).unwrap_or_default();
        let (mut e, mut rt) = load_carried(&fs, dir, &bufs).unwrap();
        println!("P11 adhoc {dir}: {:?}", run(&mut e, &mut rt, steps));
        return;
    }
    let from = std::env::var("P11_FROM").ok();
    let mut bufs = std::env::var("P11_CARRY").map(|p| load_dump(&p)).unwrap_or_default();
    let mut started = from.is_none();
    let mut total = 0u32;
    for (lvl, steps, want) in routes_ep2::EP2_ROUTE {
        started |= from.as_deref() == Some(*lvl);
        if !started { continue }
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let (mut e, mut rt) = load_carried(&fs, lvl, &bufs).unwrap();
            let r = run(&mut e, &mut rt, steps);
            (r, carry(&e))
        }));
        match r {
            Ok((Some((l, f)), b)) if l.eq_ignore_ascii_case(want) => {
                total += f;
                println!("P11 {lvl}: -> {l} at f{f} (chain f{total})");
                bufs = b;
            }
            Ok((Some((l, f)), _)) => { println!("P11 {lvl}: WRONG -> {l} at f{f} (want {want})"); break }
            Ok((None, _)) => { println!("P11 {lvl}: not fired (want {want})"); break }
            Err(_) => { println!("P11 {lvl}: PANIC"); break }
        }
    }
}

//! WP-P17: Episodes 3, 4, 6 scripted play (there is no EP5 on the ISO). Runs the per-level step routes of
//! src/bin/routes_ep3.rs .. routes_ep6.rs through real level switches (game buffers carried as the C
//! EFGameMemoryMgr does) and reports the frame of each level request. Step machinery = tests/engine_p11.rs.
//! `cargo test --release --test engine_p17 -- --ignored --nocapture` (P17_EP=3|4|5|6 one episode,
//! P17_FROM=<level> starts later, P17_CARRY dump, P17_STEPS="<level>|<steps>" ad hoc).
//!
//! Vertical / non-flat movement (P20 R110; all real pad input, no new engine code). The C heroes have NO jump
//! button: everything off the flat plane is an edge action of EFDMEdgeDetect (DM 0x19) on env polygons whose
//! +4 edge mask is set, gated by the hero prim's mask words (+0x20/+0x24):
//! - ledge climb (tag 0x66, poly mat[1] & 0x8000000): walk into the ledge face and press X: `go X Z N X`
//!   (X pulses while steering; both Scooby and Shaggy). Proven EP2/L01A (280-286, 224-230).
//! - crawl / crouch through a low doorway (Scooby only, tag 0x67, poly mat[1] & 0x20000000, e.g. the
//!   doorway plugs mat 0x20000003): leader must be Scooby (`p 2000 30` = CIRCLE leader switch; needs both
//!   heroes on the ground and the timer >= 0.75 s), approach square to the plug (GB0+0x14 0x4000 = edge seen,
//!   0x8000 = in front + facing within 1.22 rad), then `go <beyond> N X`: GB0+0x14 -> 0x1004204 (crouch) and
//!   the hs_door behind the plug hops the hero through. Proven EP2/L03A doors 0x111/0x112 (key 0x49 room).
//! - ladders: walk to the ladder foot and `go X Z N X` (R38 game.rs 'J').
//! - drops / ramps: plain `go`/`goto` (walking off an edge falls under gravity).
//! `ledges X0 Z0 X1 Z1` lists the edge-flagged polys (mask, normal, mat words) in a box: no entry with
//! mat[1] & 0x8000000 = no climb anywhere there (EP2/L03A has none; its only edges are the 0x111/0x112 plugs).
//! Rides (EP4/L03A/B trail bike 0x2a00000d): `rideauto N [GAZ [DUTY]]` holds GAZ (pad 8000, 1 of DUTY frames) while fuel (bike local+0x98,
//! max 180, -0.22/f under gaz, +50 per coll_level3 can 0x40000011 hit by the bike = msg 0x17b) > 25 and pulses
//! the stick (continuous lateral speed, mirrored vs walking) away from hs_rideobst hotspots ahead (hit = 0x17d
//! accident) or towards the next can within 4.5 m laterally; `rideauto N GAZ DUTY JUMP` also presses JUMP near hotspots.
//! Fire (EP6/L05, P24 R7): `dodge N X Z DX DZ [F]` holds the hero on (X,Z) for N frames and sidesteps to (X+DX,Z+DZ)
//! while a thrown fireball/fireballbig is within 12 m, so it lands next to the stand and burns shrubs there (0x1ec);
//! F also stays off the stand while a fireballbig_expl burns within 2.5 m and steps away from a `ghost` < 3 m (contact
//! = scare 0xc9, which regrows shrubs and resets the valve count). `until UID OFF !VAL N body` waits for "differs".
//! `bone UID` prints the object's bone frames 0..12.
// R16 route `setenv` switches are per test thread (the gate runs p17_chains and p21_real_carry_episodes in
// parallel in one process; a process env var set by one route leaked into the other's L03B run).
thread_local! { static RENV: std::cell::RefCell<std::collections::HashMap<String, String>> = Default::default(); }
fn penv(k: &str) -> Option<String> { RENV.with(|m| m.borrow().get(k).cloned()).or_else(|| std::env::var(k).ok()) }
#[path = "../../src/bin/routes_ep2.rs"]
#[allow(dead_code)]
pub(crate) mod routes_ep2;
#[path = "../../src/bin/routes_animsnd.rs"]
#[allow(dead_code)]
mod routes_animsnd;
#[path = "../../src/bin/routes_ep3.rs"]
#[allow(dead_code)]
pub(crate) mod routes_ep3;
#[path = "../../src/bin/routes_ep4.rs"]
#[allow(dead_code)]
pub(crate) mod routes_ep4;
#[path = "../../src/bin/routes_ep5.rs"]
#[allow(dead_code)]
mod routes_ep5;
#[path = "../../src/bin/routes_ep6.rs"]
#[allow(dead_code)]
pub(crate) mod routes_ep6;
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::runtime::*;
use scooby_recomp::engine::{APIgameobj, APIphysic};

use crate::common::{ISO, leader, pos, warp, send, cutscene_update, frame, frame_stick, carry, load_carried, norm, hex, uid, load_dump};
type Eng = scooby_recomp::engine::Engine;

/// Step script, ';'-separated:
/// `f N` frames; `x N` frames pressing X (3 of every 20), re-warping to the last warp target each 60;
/// `p BTN N` frames holding pad bits BTN (hex); `m UID MSG P2 [W..]` SendMessageTo (data words);
/// `warp UID` leader onto the object; `l UID OFF VAL` write local+OFF of UID; `g ID OFF VAL` write
/// game buffer ID +OFF; `dbg UID` print local words; `dm ADDR LEN` dump memory. Runs until a level request (stops the script).
/// WP-P21 R46/R47: left stick (lx, ly) that walks from `p` towards `t`, relative to the active camera (game.rs route_pad)
fn steer(rt: &EngineRuntime, p: [f32; 3], t: [f32; 3]) -> (u8, u8) {
    let (dx, dz) = (t[0] - p[0], t[2] - p[2]);
    let d = (dx * dx + dz * dz).sqrt().max(1e-6);
    let cat = rt.gfx.cameras.cameras.iter().flatten().next().map_or([0.0, 0.0, 1.0], |c| c.ltm.at);
    let fl = (cat[0] * cat[0] + cat[2] * cat[2]).sqrt().max(1e-6);
    let fw = [cat[0] / fl, cat[2] / fl];
    // right = up x fwd = (fwd.z, -fwd.x)
    let rg = [fw[1], -fw[0]];
    let (ux, uz) = (dx / d, dz / d);
    let st = |v: f32| (128.0 + v.clamp(-1.0, 1.0) * 127.0) as u8;
    (st(-(ux * rg[0] + uz * rg[1])), st(-(ux * fw[0] + uz * fw[1])))
}

pub(crate) fn run(e: &mut Eng, rt: &mut EngineRuntime, steps: &str) -> Option<(String, u32)> {
    let (e, rt) = (e, rt);
    let mut f = 0u32;
    let mut at: Option<[f32; 3]> = None;
    let mut got = None;
    let mut q: std::collections::VecDeque<String> = steps.split(';').map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect();
    'outer: while let Some(st) = q.pop_front() {
        let a: Vec<&str> = st.split_whitespace().collect();
        if std::env::var_os("P17_STEPLOG").is_some() { println!("P17 step f{f} `{st}` hero {:?} pw {}", pos(e, rt, leader(e)), e.rt.game_buffers.get(&1).map_or(0.0, |b| if b.0 != 0 { e.mem.rf(b.0 + 4) } else { 0.0 })) }
        let (btn, n) = match a[0] {
            // R77 `gw UID [N] [X|T]`: `goto` to the object's current position (sweep replacement for warps)
            "gw" => {
                let u = uid(e, a[1]);
                if let Some(p) = pos(e, rt, u) {
                    q.push_front(format!("goto {} {} {} {}", p[0], p[2], a.get(2).unwrap_or(&"1500"), a.get(3).unwrap_or(&"")));
                }
                (0, 0)
            }
            // R105 `gnear NAME TOTAL [CHUNK]`: for TOTAL frames, repeatedly `goto` (CHUNK frames, default 40) the
            // nearest visible object of module NAME (EP3/L01 food race: eaten food hides for 15 s)
            // R113 optional 5th arg: `v` = EP3/L01 food value weighting (0xde..0xe3 = 200, 0xe4..0xe7 = 500 pts,
            // Food.c), else a RIVAL uid: skip objects nearer to RIVAL than to the leader
            "gnear" => {
                let (tot, ch): (u32, u32) = (a[2].parse().unwrap(), a.get(3).map_or(40, |s| s.parse().unwrap()));
                let me = pos(e, rt, leader(e)).unwrap_or([0.0; 3]);
                // P20 R147: 5th arg = `/`-separated list; `!X,Z,R` = skip objects within R of X,Z (EP3/L05: wisps by the
                // obj_launcher_level5 throw spot 268.6,239.3, each throw costs 0.3 courage; R72: `!X:Z:R` too, for nested ifs); several RIVAL uids allowed
                let t5: Vec<&str> = a.get(4).map_or(vec![], |s| s.split('/').collect());
                let ex: Vec<Vec<f32>> = t5.iter().filter_map(|s| s.strip_prefix('!')).map(|s| s.split([',', ':']).map(|v| v.parse().unwrap()).collect()).collect();
                let rv: Vec<[f32; 3]> = t5.iter().filter(|s| **s != "v" && !s.starts_with('!')).filter_map(|s| pos(e, rt, uid(e, s))).collect();
                let d = |p: &[f32; 3], m: &[f32; 3]| (p[0] - m[0]).powi(2) + (p[2] - m[2]).powi(2);
                let ids: Vec<u32> = e.rt.objects.iter().flatten().filter(|o| o.inst.code.is_some_and(|k| &*e.rt.codes[k].def.name == a[1])).map(|o| o.id).collect();
                let hid = rt.particles.lock().unwrap().world.hidden.clone();
                let vw = t5.contains(&"v"); // P32 R12: `v` may be combined with RIVAL uids (`v/2a00002c`)
                let w = |u: u32| if !vw { 1.0 } else { match u & 0xfff { 0xe4..=0xe7 => 25.0, 0xde..=0xe3 => 4.0, _ => 1.0 } };
                let best = ids.into_iter().filter(|u| !hid.contains(u)).filter_map(|u| pos(e, rt, u).map(|p| (p, w(u)))).filter(|(p, _)| rv.iter().all(|r| d(p, &me) <= d(p, r))).filter(|(p, _)| ex.iter().all(|x| (p[0] - x[0]).hypot(p[2] - x[1]) > x[2])).min_by(|(p, a), (q, b)| (d(p, &me) / a).total_cmp(&(d(q, &me) / b))).map(|(p, _)| p);
                // P27 R9 `gnear KIND TOT CH UID e`: end as soon as no KIND is left on this side (instead of idling to TOT)
                let end = a.get(5) == Some(&"e") && best.is_none();
                if tot > 0 && !end {
                    let c = ch.min(tot);
                    q.push_front(format!("gnear {} {} {} {} {}", a[1], tot - c, ch, a.get(4).map_or("", |s| s), a.get(5).map_or("", |s| s)));
                    match best {
                        Some(p) => q.push_front(format!("goto {} {} {}", p[0], p[2], c)),
                        None => q.push_front(format!("f {c}")),
                    }
                }
                (0, 0)
            }
            // R102 `pr UID`: debug print of the object's and the leader's positions
            // P24 R7 `bone UID`: GetObjectFrame(UID, b) positions for bones 0..12 (y relative to the object)
            // P23 R12 `nodes UID,UID`: also publish these objects' joint frames in this level (as game.rs does)
            "nodes" => {
                crate::common::add_nodes(&a[1].split(',').map(|h| uid(e, h)).collect::<Vec<_>>());
                (0, 0)
            }
            "bone" => {
                let u = uid(e, a[1]);
                let g = rt.particles.lock().unwrap();
                let o = g.world.frames.get(&u).map_or([0.0; 3], |m| m.pos);
                let w: Vec<String> = (0..13).map(|b| scooby_recomp::engine::particle::PLWorld::GetObjectFrame(&g.world, u, b).map_or("-".into(), |m| format!("{b}:{:.2},{:.2},{:.2}", m.pos[0] - o[0], m.pos[1] - o[1], m.pos[2] - o[2]))).collect();
                println!("P17  f{f} bone {u:#x} n {} {}", g.world.nodes.get(&u).map_or(0, |n| n.len()), w.join(" "));
                println!("P17  f{f} bone {u:#x} ids {:x?}", g.world.node_ids.get(&u));
                (0, 0)
            }
            "pr" => {
                let u = uid(e, a[1]);
                let l = e.rt.game_buffers.get(&0).map(|b| e.mem.r32(b.0 + 8)).unwrap_or(0);
                // R128: + partner (GB0+0xc) and courage (GB1+0)
                let pt = e.rt.game_buffers.get(&0).map(|b| e.mem.r32(b.0 + 0xc)).unwrap_or(0);
                let cg = e.rt.game_buffers.get(&1).map_or(0.0, |b| e.mem.rf(b.0));
                println!("P17 pr {u:#x} {:?} leader {:?} partner {:?} courage {cg}", pos(e, rt, u), pos(e, rt, l), pos(e, rt, pt));
                (0, 0)
            }
            "f" => (0u16, a[1].parse::<u32>().unwrap()),
            "x" => (scooby_recomp::engine::input::pad::CROSS, a[1].parse().unwrap()),
            "p" => (hex(a[1]) as u16, a[2].parse().unwrap()),
            // `s LX LY N [PAD]`: N frames with the left stick at (LX, LY) (hex bytes, 80 = centre),
            // optionally holding pad buttons PAD (hex, e.g. 4000 = CROSS)
            "s" => (a.get(4).map_or(0, |w| hex(w) as u16), a[3].parse().unwrap()),
            "m" => {
                let d: Vec<u32> = a[4..].iter().map(|w| if w.starts_with('@') { uid(e, w) } else { hex(w) }).collect();
                let u = uid(e, a[1]);
                send(e, rt, u, hex(a[2]), hex(a[3]), &d);
                (0, 0)
            }
            "warp" => {
                let u = uid(e, a[1]);
                at = pos(e, rt, u);
                println!("P17  warp {u:#x} -> {at:?} leader {:#x}", leader(e));
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
            // WP-P21 R62 `gor B O V`: OR V into game buffer B at +O (a fake that keeps the real bits)
            "gor" => {
                let b = e.rt.game_buffers.get(&hex(a[1])).map(|b| b.0).expect("game buffer");
                let v = e.mem.r32(b + hex(a[2]));
                e.mem.w32(b + hex(a[2]), v | hex(a[3]));
                (0, 0)
            }
            "ls" => {
                let hits: Vec<(u32, String, usize, u32)> = e.rt.objects.iter().flatten().filter_map(|o| o.inst.code.map(|k| (o.id, e.rt.codes[k].def.name.to_string(), e.rt.codes[k].build as usize, o.inst.local))).filter(|h| h.1.to_lowercase().contains(a.get(1).copied().unwrap_or(""))).collect();
                for (id, n, b, lo) in hits {
                    let w: Vec<String> = if lo != 0 { (0..6).map(|i| format!("{:x}", e.mem.r32(lo + 4 * i))).collect() } else { vec![] };
                    println!("P17  obj {id:#x} {n} build {b} local {lo:#x} [{}] pos {:?}", w.join(" "), pos(e, rt, id));
                }
                (0, 0)
            }
            // `lsw VAL`: objects whose first 0x100 local bytes hold the word VAL (hex)
            "lsw" => {
                let v = u32::from_str_radix(a[1], 16).unwrap();
                let hits: Vec<(u32, String, u32)> = e.rt.objects.iter().flatten().filter_map(|o| o.inst.code.map(|k| (o.id, e.rt.codes[k].def.name.to_string(), o.inst.local))).filter(|h| h.2 != 0).collect();
                for (id, n, lo) in hits {
                    if let Some(i) = (0..64).find(|i| e.mem.r32(lo + 4 * i) == v) { println!("P17  lsw {id:#x} {n} +{:#x}", 4 * i) }
                }
                (0, 0)
            }
            "gd" => {
                let b = e.rt.game_buffers.get(&hex(a[1])).map(|b| b.0).expect("game buffer");
                let w: Vec<String> = (0..hex(a[2]) / 4).map(|i| format!("{:x}", e.mem.r32(b + 4 * i))).collect();
                println!("P17  f{f} gb{} {}", a[1], w.join(" "));
                (0, 0)
            }
            "play" => {
                // EFCutScenePlayer::Play(id) as APIgame::PlayCutScene (no sound): the scripted cut-scene
                rt.particles.lock().unwrap().cutscene_req.push(CutSceneReq::Play(hex(a[1]), 0xffff_ffff, 0x7fff_ffff));
                (0, 0)
            }
            "cs" => {
                let g = rt.particles.lock().unwrap();
                println!("P17  f{f} cutscene {:?}", g.cutscene.as_ref().map(|c| (c.current, c.flags, c.time, c.duration, c.scene(c.current).map(|s| s.actors.clone()))));
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
                    rt.particles.lock().unwrap().logic = true;
                    APIgameobj::SetTranslation(e, a1, p);
                    rt.particles.lock().unwrap().logic = false;
                }
                (0, 0)
            }
            "front" => {
                // book_regress placement: object 3 m in front of the leader (its frame `at`), +1 m up
                let t = uid(e, a[1]);
                let l = leader(e);
                let fr = rt.particles.lock().unwrap().world.frames.get(&l).cloned();
                if let Some(fr) = fr {
                    let to = [fr.pos[0] + fr.at[0] * 3.0, fr.pos[1] + 1.0, fr.pos[2] + fr.at[2] * 3.0];
                    let p = e.mem.alloca(0x10);
                    for (i, c) in to.iter().enumerate() {
                        e.mem.w32(p + 4 * i as u32, c.to_bits());
                    }
                    rt.particles.lock().unwrap().logic = true;
                    APIgameobj::SetTranslation(e, t, p);
                    rt.particles.lock().unwrap().logic = false;
                }
                (0, 0)
            }
            // P23 R6 `prox UID`: EFGameObject proximity clipping entry (params, inside, +0xcc)
            "prox" => {
                let u = uid(e, a[1]);
                let v = rt.particles.lock().unwrap().proximity.get(&u).cloned();
                println!("P17  f{f} prox {u:#x} {v:x?}");
                (0, 0)
            }
            "phys" => {
                let u = uid(e, a[1]);
                let g = rt.particles.lock().unwrap();
                let id = g.sub.physic.PhysicOf(u);
                match g.sub.physic.Physic(id) {
                    Some(p) => println!("P17  f{f} phys {u:#x} id {id} flags {:#x} state {:#x} notify {:#x} masks {:x?} prims {:?}", p.flags, p.state, p.notify_bits, p.model.masks, p.model.prims.iter().map(|q| q.as_ref().map(|q| format!("{:?} {:?}", q.base.center, q.ext))).collect::<Vec<_>>()),
                    None => println!("P17  f{f} phys {u:#x}: no physic"),
                }
                (0, 0)
            }
            // P23 R17 `floor UID`: GetFloorAttribs words (word 2 = attr flags: 0x20 camera floor, 0x2000 EP6/L02 alarm reset)
            "floor" => {
                let u = uid(e, a[1]);
                let b = e.mem.alloca(0x20);
                let r = APIphysic::GetFloorAttribs(e, u, b);
                let w: Vec<String> = (0..8).map(|i| format!("{:x}", e.mem.r32(b + 4 * i))).collect();
                println!("P17  f{f} floor {u:#x} r {r} {}", w.join(" "));
                (0, 0)
            }
            // P22 `pia UID TAG`: prim int attribs 0..10
            "pia" => {
                let u = uid(e, a[1]);
                let w: Vec<String> = (0..11).map(|i| format!("{:x}", APIphysic::PrimGetIntAttrib(e, u, hex(a[2]), i))).collect();
                println!("P17  f{f} pia {u:#x} tag {} {}", a[2], w.join(" "));
                (0, 0)
            }
            // P22 `ride X Z R N [PAD]`: trailbike steering driver: each frame LEFT/RIGHT toward (X,Z) from the
            // bike heading (trailbike local+0x158 degrees, 0 = -z, LEFT = +); PAD (hex) is held too; ends within R
            // (xz) or after N frames; PAD is pulsed 3 of every 30 frames (X = jump the ride obstacles); HOLD (hex) held but released 1 of 30 frames (8000 = SQUARE = gas: it only latches on a press and a landing clears it)
            "ride" => {
                let (tx, tz, r, n): (f32, f32, f32, u32) = (a[1].parse().unwrap(), a[2].parse().unwrap(), a[3].parse().unwrap(), a[4].parse().unwrap());
                let extra = a.get(5).map_or(0, |s| hex(s)) as u16;
                let u = 0x2a00000d;
                let p = pos(e, rt, u).unwrap_or([0.0; 3]);
                let (dx, dz) = (tx - p[0], tz - p[2]);
                let lo = e.rt.objects.iter().flatten().find(|o| o.id == u).map_or(0, |o| o.inst.local);
                let d = ((-dx).atan2(-dz).to_degrees() - e.mem.rf(lo + 0x158) + 540.0).rem_euclid(360.0) - 180.0;
                if dx.hypot(dz) <= r || n == 0 || d.abs() > 100.0 {
                    println!("P17  f{f} ride {tx} {tz} end at {p:?} (left {n})");
                    (0, 0)
                } else {
                    let lo = e.rt.objects.iter().flatten().find(|o| o.id == u).map_or(0, |o| o.inst.local);
                    let h = e.mem.rf(lo + 0x158);
                    let want = (-dx).atan2(-dz).to_degrees();
                    let d = (want - h + 540.0).rem_euclid(360.0) - 180.0;
                    if std::env::var("P22_RIDE").is_ok() { println!("P17  f{f} ride h {h} want {want} d {d} at {p:?} fuel {} oog {} spd {} crash {:#x}", e.mem.rf(lo + 0x98), e.mem.r32(lo + 0x9c), e.mem.rf(lo + 0x138), e.mem.r32(lo + 0xa4)); }
                    q.push_front(format!("ride {tx} {tz} {r} {} {} {}", n - 1, a.get(5).unwrap_or(&"0"), a.get(6).unwrap_or(&"0")));
                    let st = if d > 2.0 { scooby_recomp::engine::input::pad::LEFT } else if d < -2.0 { scooby_recomp::engine::input::pad::RIGHT } else { 0 };
                    (st | if n % 30 == 15 { 0 } else { a.get(6).map_or(0, |s| hex(s)) as u16 } | if n % 30 < 3 { extra } else { 0 }, 1)
                }
            }
            // P24 R12 `ridew X Z R N JD`: trail-bike waypoint (fuel can line): steer to X,Z with gas, JUMP (c000) while the
            // reach circle is within JD..JD-2.5 m (0 = no jump); ends on reach (<= R), target behind (> 100 deg), crash
            // (bike local+0xa4 != 0) or N frames (R < 0: radius |R|, no `behind` end, to turn around); prints `ridew X Z <reach|behind|crash|time> at P fuel F`.
            // R13 `ridewo ...` = ridew + hs_rideobst handling (below)
            // R16 `setenv K V`: set a route-helper env switch mid-route (P24_J1 / P24_OJ / P24_DW per segment; thread-local, see penv)
            "setenv" => {
                RENV.with(|m| m.borrow_mut().insert(a[1].to_string(), a[2].to_string()));
                (0, 0)
            }
            // P27 R24 `unsetenv K`: drop a route-helper env switch again (thread-local only)
            "unsetenv" => {
                RENV.with(|m| m.borrow_mut().remove(a[1]));
                (0, 0)
            }
            "ridew" | "ridewo" => {
                let (tx, tz, r, n, jd): (f32, f32, f32, u32, f32) = (a[1].parse().unwrap(), a[2].parse().unwrap(), a[3].parse().unwrap(), a[4].parse().unwrap(), a[5].parse().unwrap());
                let u = 0x2a00000d;
                let p = pos(e, rt, u).unwrap_or([0.0; 3]);
                let lo = e.rt.objects.iter().flatten().find(|o| o.id == u).map_or(0, |o| o.inst.local);
                let (dx, dz) = (tx - p[0], tz - p[2]);
                let dist = dx.hypot(dz);
                let h = if lo == 0 { 0.0 } else { e.mem.rf(lo + 0x158) };
                let d = ((-dx).atan2(-dz).to_degrees() - h + 540.0).rem_euclid(360.0) - 180.0;
                let crash = lo == 0 || e.mem.r32(lo + 0xa4) != 0;
                let end = if crash { Some("crash") } else if dist <= r.abs() { Some("reach") } else if d.abs() > 100.0 && r > 0.0 { Some("behind") } else if n == 0 { Some("time") } else { None };
                if std::env::var_os("P24_TRACE").is_some() && f % 5 == 0 && lo != 0 { println!("P17  f{f} trace at {p:?} h {h} fuel {} st {:#x} cls {} win {} {}", e.mem.rf(lo + 0x98), e.mem.r32(lo + 4), e.mem.r32(lo + 0x128), e.mem.rf(lo + 0x11c), e.mem.rf(lo + 0x118)) }
                if let Some(why) = end {
                    println!("P17  f{f} ridew {tx} {tz} {why} at {p:?} d {dist} fuel {} code {:#x} st {:#x}", if lo == 0 { 0.0 } else { e.mem.rf(lo + 0x98) }, if lo == 0 { 0 } else { e.mem.r32(lo + 0xa4) }, if lo == 0 { 0 } else { e.mem.r32(lo + 4) });
                    (0, 0)
                } else {
                    // R13 `ridew X Z R N JD JH`: JH > 0 = JUMP held JH frames once pressed (0 = only inside the window): an early JUMP release divides the
                    // rising velocity by 1.7 (trailBikeST_Jump_Execute flag 0x400), i.e. a short hop
                    let jh: u32 = a.get(6).map_or(0, |s| s.parse().unwrap());
                    let mut hold: u32 = a.get(7).map_or(0, |s| s.parse().unwrap());
                    let win = jd > 0.0 && dist - r.abs() <= jd && dist - r.abs() > jd - 2.5;
                    if jh > 0 && win && hold == 0 { hold = jh + 1; }
                    let jw = if jh == 0 { win } else { hold >= 2 }; // hold: 0 not fired, >= 2 holding, 1 done
                    let mut j = false;
                    // R13: hs_rideobst ahead (prim attrib 9, as P22 rideauto): 0x10 -> JUMP X at OJ m (env P24_OJ, default 12),
                    // 2 (overhead) -> duck CIRCLE; both with a neutral stick (Standing_Execute: a lean wins over duck/jump)
                    let mut duck = false;
                    if a[0] == "ridewo" {
                        let oj: f32 = penv("P24_OJ").ok_or(()).map_or(12.0, |s| s.parse().unwrap());
                        let hr = h.to_radians();
                        let (fx, fz) = (-hr.sin(), -hr.cos());
                        let obst: Vec<u32> = e.rt.objects.iter().flatten().filter(|o| o.inst.code.is_some_and(|k| &*e.rt.codes[k].def.name == "hs_rideobst")).map(|o| o.id).collect();
                        for u in obst {
                            let Some(b) = pos(e, rt, u) else { continue };
                            let t = APIphysic::PrimGetIntAttrib(e, u, 0, 9);
                            let (bx, bz) = (b[0] - p[0], b[2] - p[2]);
                            let fd = bx * fx + bz * fz;
                            let lat = (bx * fz - bz * fx).abs();
                            if (b[1] - p[1]).abs() > 5.0 || lat > 6.0 { continue }
                            if std::env::var("P24_ODBG").is_ok() { println!("P17  f{f} obst {u:#x} t {t:#x} fd {fd} lat {lat} at {p:?} fl {:#x} st {:#x} jmp {} phys {:#x}", e.mem.r32(lo), e.mem.r32(lo + 4), e.mem.r32(lo + 0xe4), { let g = rt.particles.lock().unwrap(); let id = g.sub.physic.PhysicOf(0x2a00000d); g.sub.physic.Physic(id).map_or(0, |p| p.state) }) }
                            // JUMP only registers on a press edge while the physic state & 5 (on floor) (trailBikeCB_INPUT_JUMP):
                            // press on floor frames while Standing (0x12ef0), hold it once the jump state runs (release = /1.7)
                            // R15: the 0x10 obstacles are raised capsules (swept sphere r 1, centre +2.25, 6 m long): duck
                            // under them (P24_DUCK10=1); attrib 9 = 2 obstacles are ground walls to jump
                            if t == 0x10 && penv("P24_DUCK10").is_some() {
                                let w: Vec<f32> = penv("P24_DW").ok_or(()).map_or(vec![-8.0, 12.0], |s| s.split(',').map(|x| x.parse().unwrap()).collect());
                                if fd > w[0] && fd < w[1] { duck = true }
                                continue;
                            }
                            // R15 (P24_DUCK10): attrib 2 = ground wall -> JUMP at OJ m like the old 0x10 handling
                            let jt = if penv("P24_DUCK10").is_some() { 2 } else { 0x10 };
                            // R16 (P24_J1): attrib 1 (flat log, e.g. 0x1b3/0x1d4..) -> JUMP like attrib 2
                            let t = if t == 1 && jt == 2 && penv("P24_J1").is_some() { 2 } else { t };
                            if t == jt && fd > 1.0 && fd < oj + 2.0 {
                                let ps = { let g = rt.particles.lock().unwrap(); let id = g.sub.physic.PhysicOf(0x2a00000d); g.sub.physic.Physic(id).map_or(0, |p| p.state) };
                                j = if e.mem.r32(lo + 4) == 0x12ef0 { ps & 5 != 0 } else { true };
                            }
                            if t == jt && fd > -6.0 && fd <= 1.0 && e.mem.r32(lo + 4) != 0x12ef0 { j = true }
                            if t == 2 && jt != 2 && fd > -1.0 && fd < 12.0 { duck = true }
                        }
                    }
                    // R16: 9th arg G = gas duty (gas held G of every 10 frames; default 10)
                    let g: u32 = a.get(8).map_or(10, |s| s.parse().unwrap());
                    q.push_front(format!("{} {tx} {tz} {r} {} {jd} {jh} {} {g}", a[0], n - 1, if hold >= 2 { hold - 1 } else { hold }));
                    let st = if d > 2.0 { scooby_recomp::engine::input::pad::LEFT } else if d < -2.0 { scooby_recomp::engine::input::pad::RIGHT } else { 0 };
                    let st = if j || duck { 0 } else { st };
                    (st | if n % 30 == 15 || n % 10 >= g { 0 } else { 0x8000 } | if j || jw { 0xc000 } else { 0 } | if duck { 0x2000 } else { 0 }, 1)
                }
            }
            // P24 R11 `ridef N BIAS [JAT]`: trailbike autopilot: N frames steering to the centre of the current floor
            // heading window (trailBikeFUNC_SetFloorOrientation: class local+0x128 1..8 -> 0,315,270,227.5,180,132.5,90,45)
            // + BIAS degrees, gas (8000) held; JUMP (c000) pressed 6 frames from frame JAT (counted from the end, -1 = none).
            "ridef" => {
                let (n, bias): (u32, f32) = (a[1].parse().unwrap(), a[2].parse().unwrap());
                let jat: i64 = a.get(3).map_or(-1, |s| s.parse().unwrap());
                let u = 0x2a00000d;
                let p = pos(e, rt, u).unwrap_or([0.0; 3]);
                let lo = e.rt.objects.iter().flatten().find(|o| o.id == u).map_or(0, |o| o.inst.local);
                if n == 0 || lo == 0 {
                    println!("P17  f{f} ridef end at {p:?}");
                    (0, 0)
                } else {
                    let cls = e.mem.r32(lo + 0x128);
                    let c = [0.0f32, 0.0, 315.0, 270.0, 227.5, 180.0, 132.5, 90.0, 45.0][(cls as usize).min(8)];
                    let h = e.mem.rf(lo + 0x158);
                    let mut want = c + bias;
                    // fuel cans (coll_level3 +0x14 0x40000011) within 30 m and 35 deg of the floor direction pull the bike
                    if a.get(4).is_none_or(|s| *s != "n") {
                        let cans: Vec<u32> = e.rt.objects.iter().flatten().filter(|o| o.inst.code.is_some_and(|k| &*e.rt.codes[k].def.name == "coll_level3") && e.mem.r32(o.inst.local + 0x14) == 0x40000011).map(|o| o.id).collect();
                        let best = cans.into_iter().filter_map(|u| pos(e, rt, u)).filter_map(|b| { let (dx, dz) = (b[0] - p[0], b[2] - p[2]); let w = (-dx).atan2(-dz).to_degrees(); let dd = (w - c + 540.0).rem_euclid(360.0) - 180.0; (dx.hypot(dz) < 30.0 && dx.hypot(dz) > 1.0 && dd.abs() < 35.0 && (b[1] - p[1]).abs() < 5.0).then_some((dx.hypot(dz), w)) }).min_by(|x, y| x.0.total_cmp(&y.0));
                        if let Some((_, w)) = best { want = w; }
                    }
                    if std::env::var("P24_CANS").is_ok() && n == 1 {
                        let cans: Vec<u32> = e.rt.objects.iter().flatten().filter(|o| o.inst.code.is_some_and(|k| &*e.rt.codes[k].def.name == "coll_level3")).map(|o| o.id).collect();
                        for u in cans { let l = e.rt.objects.iter().flatten().find(|o| o.id == u).map_or(0, |o| o.inst.local); println!("P17  can {u:#x} {:#x} {:?}", e.mem.r32(l + 0x14), pos(e, rt, u)); }
                        // P27 R24: also the ride obstacles (prim attrib 9)
                        let obst: Vec<u32> = e.rt.objects.iter().flatten().filter(|o| o.inst.code.is_some_and(|k| &*e.rt.codes[k].def.name == "hs_rideobst")).map(|o| o.id).collect();
                        for u in obst { let t = APIphysic::PrimGetIntAttrib(e, u, 0, 9); println!("P17  obst {u:#x} t {t:#x} {:?}", pos(e, rt, u)); }
                    }
                    let d = (want - h + 540.0).rem_euclid(360.0) - 180.0;
                    if n % 10 == 0 && std::env::var("P22_RIDE").is_ok() { println!("P17  f{f} ridef cls {cls} h {h} at {p:?} fuel {} spd {} crash {:#x}", e.mem.rf(lo + 0x98), e.mem.rf(lo + 0x138), e.mem.r32(lo + 0xa4)); }
                    q.push_front(format!("ridef {} {bias} {jat} {}", n - 1, a.get(4).unwrap_or(&"y")));
                    let st = if d > 2.0 { scooby_recomp::engine::input::pad::LEFT } else if d < -2.0 { scooby_recomp::engine::input::pad::RIGHT } else { 0 };
                    let j = jat >= 0 && (n as i64) <= jat && (n as i64) > jat - 6;
                    (st | if n % 30 == 15 { 0 } else { 0x8000 } | if j { 0xc000 } else { 0 }, 1)
                }
            }
            // R85 `gb B OFF`: print game buffer B word at OFF
            // P32 R72 `setc V`: TEST-ONLY courage refill (GB1+0 = V), no C equivalent; validates later route parts only
            "setc" => {
                let b = e.rt.game_buffers.get(&1).map(|b| b.0).expect("game buffer 1");
                e.mem.wf(b, a[1].parse().unwrap());
                println!("P17  f{f} setc courage {}", a[1]);
                (0, 0)
            }
            "gb" => {
                let b = e.rt.game_buffers.get(&hex(a[1])).map(|b| b.0).expect("game buffer");
                println!("P17  f{f} gb {} +{} = {:#x}", a[1], a[2], e.mem.r32(b + hex(a[2])));
                (0, 0)
            }
            // P22 `until UID OFF VAL N s1, s2, ..`: run the ','-separated steps until local+OFF of UID == VAL (hex), at most N times
            "until" => {
                let u = uid(e, a[1]);
                let lo = e.rt.objects.iter().flatten().find(|o| o.id == u).map_or(0, |o| o.inst.local);
                let (v, n): (u32, u32) = (e.mem.r32(lo + hex(a[2])), a[4].parse().unwrap());
                // P24 R7: VAL `!X` waits until the word differs from X
                let hit = match a[3].strip_prefix('!') { Some(x) => v != hex(x), None => v == hex(a[3]) };
                if !hit && n > 0 {
                    let body = a[5..].join(" ");
                    q.push_front(format!("until {} {} {} {} {body}", a[1], a[2], a[3], n - 1));
                    // P20 R129: '|' in a body step becomes ',' (one level of nested `until`)
                    for s in body.split(',').rev() {
                        q.push_front(s.trim().replace('|', ","));
                    }
                } else {
                    println!("P17  f{f} until {u:#x} +{} = {v:#x} (left {n})", a[2]);
                }
                (0, 0)
            }
            // P22 R33 `untilg BUF OFF VAL N s1, s2, ..`: `until` on game buffer BUF +OFF (hex) instead of an object
            // local (`!X`: until it differs from X). EP4 cut-scenes: hero lock GB0+0x14 = 0x1400008 while a cut runs
            "untilg" => {
                let b = e.rt.game_buffers.get(&hex(a[1])).map(|b| b.0).expect("game buffer");
                let (v, n): (u32, u32) = (e.mem.r32(b + hex(a[2])), a[4].parse().unwrap());
                // P20 R144: VAL `MASK&X` compares only the MASK bits (`!MASK&X`: until they differ); EP2/L04 cut-scene hero
                // lock = GB0+0x14 bit 0x1000000 (0x1410008 / 0x1430008 depending on the other bits)
                let eq = |x: &str| match x.split_once('&') { Some((m, x)) => v & hex(m) == hex(x), None => v == hex(x) };
                let hit = match a[3].strip_prefix('!') { Some(x) => !eq(x), None => eq(a[3]) };
                if !hit && n > 0 {
                    let body = a[5..].join(" ");
                    q.push_front(format!("untilg {} {} {} {} {body}", a[1], a[2], a[3], n - 1));
                    for s in body.split(',').rev() {
                        q.push_front(s.trim().to_string());
                    }
                } else {
                    println!("P17  f{f} untilg {} +{} = {v:#x} (left {n})", a[1], a[2]);
                }
                (0, 0)
            }
            // P22 R14 `untild UID R N s1, s2, ..`: run the steps until the xz distance UID <-> leader is >= R
            // (`<R`: until it is < R), at most N times (EP4/L02A: hide from the circling alligator 0x10c)
            "untild" => {
                let u = uid(e, a[1]);
                let (lt, r) = match a[2].strip_prefix('<') { Some(x) => (true, x.parse::<f32>().unwrap()), None => (false, a[2].parse::<f32>().unwrap()) };
                let n: u32 = a[3].parse().unwrap();
                let (p, h) = (pos(e, rt, u).unwrap_or([0.0; 3]), pos(e, rt, leader(e)).unwrap_or([0.0; 3]));
                let d = ((p[0] - h[0]).powi(2) + (p[2] - h[2]).powi(2)).sqrt();
                if (d < r) != lt && n > 0 {
                    let body = a[4..].join(" ");
                    q.push_front(format!("untild {} {} {} {body}", a[1], a[2], n - 1));
                    for s in body.split(',').rev() {
                        q.push_front(s.trim().to_string());
                    }
                } else {
                    println!("P17  f{f} untild {u:#x} d {d:.2} (left {n})");
                }
                (0, 0)
            }
            // P23 R42 `untilf UID OFF LO HI N s1, s2, ..`: run the steps until LO <= f32 local+OFF of UID <= HI, at most N times.
            // UID/LO/HI may be '+'-lists (all must hold), LO > HI wraps. EP6/L02 arcs: beam angle local+0x30 (re-rolled every 4.6 s)
            "untilfs" => {
                // P23 R51 `untilfs UID OFF S N`: waits (5-frame steps, at most N frames) until UID's float at local+OFF
                // has not changed for S frames (EP6/L02 arc 0x18d: each switch-on re-rolls the beam angle at +0x30 and
                // the hero crossing the diagonal gap within ~100 f of a switch-on is struck; once the arc settles the
                // gap is safe)
                let u = uid(e, a[1]);
                let lo = e.rt.objects.iter().flatten().find(|o| o.id == u).map_or(0, |o| o.inst.local);
                let v = f32::from_bits(e.mem.r32(lo + hex(a[2])));
                let (s, n): (i32, i32) = (a[3].parse().unwrap(), a[4].parse().unwrap());
                let st = match (a.get(5).map(|t| t.parse::<f32>().unwrap()), a.get(6).map(|t| t.parse::<i32>().unwrap())) {
                    (Some(p), Some(k)) if p == v => k + 5,
                    _ => 0,
                };
                if st >= s || n <= 0 {
                    println!("P17  f{f} untilfs {} +{} = {v} stable {st} (left {n})", a[1], a[2]);
                } else {
                    q.push_front(format!("untilfs {} {} {s} {} {v} {st}", a[1], a[2], n - 5));
                    q.push_front("f 5".to_string());
                }
                (0, 0)
            }
            "untilf" => {
                let n: u32 = a[5].parse().unwrap();
                let (us, los, his): (Vec<&str>, Vec<&str>, Vec<&str>) = (a[1].split('+').collect(), a[3].split('+').collect(), a[4].split('+').collect());
                let vs: Vec<f32> = us.iter().map(|s| {
                    let u = uid(e, s);
                    let lo = e.rt.objects.iter().flatten().find(|o| o.id == u).map_or(0, |o| o.inst.local);
                    f32::from_bits(e.mem.r32(lo + hex(a[2])))
                }).collect();
                // LO > HI: wrapped range (v >= LO or v <= HI)
                let ok = vs.iter().enumerate().all(|(i, v)| {
                    let (l, h) = (los[i].parse::<f32>().unwrap(), his[i].parse::<f32>().unwrap());
                    if l <= h { *v >= l && *v <= h } else { *v >= l || *v <= h }
                });
                if !ok && n > 0 {
                    let body = a[6..].join(" ");
                    q.push_front(format!("untilf {} {} {} {} {} {body}", a[1], a[2], a[3], a[4], n - 1));
                    for s in body.split(',').rev() {
                        q.push_front(s.trim().to_string());
                    }
                } else {
                    println!("P17  f{f} untilf {} +{} = {vs:?} (left {n})", a[1], a[2]);
                }
                (0, 0)
            }
            // P22 R30 `untilp UID X Z R N s1, s2, ..`: watch a guard: run the steps while UID is within R (xz) of the point
            // X Z (`>R`: while it is farther than R), at most N times (EP4/L02A roof mercs 0x119/0x11a: go once the guard turned away)
            "untilp" => {
                let u = uid(e, a[1]);
                let (x, z): (f32, f32) = (a[2].parse().unwrap(), a[3].parse().unwrap());
                let (gt, r) = match a[4].strip_prefix('>') { Some(v) => (true, v.parse::<f32>().unwrap()), None => (false, a[4].parse::<f32>().unwrap()) };
                let n: u32 = a[5].parse().unwrap();
                let p = pos(e, rt, u).unwrap_or([0.0; 3]);
                let d = (p[0] - x).hypot(p[2] - z);
                if (if gt { d > r } else { d < r }) && n > 0 {
                    let body = a[6..].join(" ");
                    q.push_front(format!("untilp {} {} {} {} {} {body}", a[1], a[2], a[3], a[4], n - 1));
                    for s in body.split(',').rev() {
                        q.push_front(s.trim().to_string());
                    }
                } else {
                    println!("P17  f{f} untilp {u:#x} at ({:.1},{:.1}) d {d:.2} (left {n})", p[0], p[2]);
                }
                (0, 0)
            }
            "ifns" | "ifad" | "ifca" | "ifem" => {
                // P23 R27 `ifns 1|0 s1, s2..`: run the comma steps only under P17_NODESET=all (1) / not (0) (EP6/L04A
                // ghost 0x1d2: the hook needs `bookgb` on the default node set, `bookgr` under NODESET)
                // P23 R54 `ifad 1|0 s1, s2..`: same, on the engine anim mode (GAME_ANIMDIS on = default (1) / =0 (0); EP6/L02
                // 0x1aa/0x1ab refill: the wisps drift differently while the book opens)
                // P20 R170 `ifca 1|0 s1, s2..`: same, on the flex-camera anim channels (Subsystems::cam_anim: on = default (1),
                // P17_CAMANIM=0 (0); EP2/L02B crate climb: the camera-relative steer differs with the static camera)
                // P23 R60 `ifem 1|0 s1, s2..`: same, on GAME_EMCHILD (on = default (1), GAME_EMCHILD=0 (0); EP6/L02 hideout: with the em_child emitters the
                // scientist 0x100 fearwave 0x2a800008 flies on instead of bursting early and hits the waiting hero at < 0.99 m)
                let ns = if a[0] == "ifem" {
                    std::env::var("GAME_EMCHILD").as_deref() != Ok("0")
                } else if a[0] == "ifca" {
                    crate::common::camanim_on()
                } else if a[0] == "ifad" {
                    std::env::var("GAME_ANIMDIS").as_deref() != Ok("0")
                } else {
                    std::env::var("P17_NODESET").as_deref() == Ok("all")
                };
                println!("P17  f{f} {} {} on {ns}", a[0], a[1]);
                if ns == (a[1] == "1") {
                    // '|' in a step becomes ',' (one nested `until` body, as `until` does)
                    for s in a[2..].join(" ").split(',').rev() {
                        q.push_front(s.trim().replace('|', ","));
                    }
                }
                (0, 0)
            }
            // P32 R16 `iflc s1, s2..`: run the comma steps only under GAME_LOGICCLIP=1 (same test as host/gameobj.rs)
            // so clip-only route waits leave the default-mode timings untouched
            // P32 R87 `ifnlc s1, s2..`: the inverse (only without GAME_LOGICCLIP=1)
            // P38 R4: '|' in a body step becomes ',' (one level of nesting, as `until`/`ifst`/`ifcap`)
            "iflc" | "ifnlc" => {
                if matches!(std::env::var("GAME_LOGICCLIP").as_deref().map(str::trim), Ok("1")) == (a[0] == "iflc") {
                    for s in a[1..].join(" ").split(',').rev() {
                        q.push_front(s.trim().replace('|', ","));
                    }
                }
                (0, 0)
            }
            "ifc" => {
                // P24 R28: `ifc LO HI s1, s2..` runs the comma steps only when courage (GB1+0) is in [LO,HI] (EP4/L01:
                // the deck-snack stairs walk needs some courage; at 0 one ghost touch kills the hero)
                let (lo, hi): (f32, f32) = (a[1].parse().unwrap(), a[2].parse().unwrap());
                let c = e.rt.game_buffers.get(&1).map_or(f32::NAN, |b| e.mem.rf(b.0));
                println!("P17  f{f} ifc courage {c} in [{lo},{hi}]");
                if c >= lo && c <= hi {
                    for s in a[3..].join(" ").split(',').rev() {
                        q.push_front(s.trim().to_string());
                    }
                }
                (0, 0)
            }
            "ifnear" => {
                // P32 R69 `ifnear UID R s1, s2..`: run the comma steps only when UID is within R (xz) of the hero
                // (EP3/L04 GAME_LOGICCLIP: dustdevil 0x152, frozen off-view during the 0x151 fight as C, waits on the path)
                let (p, h) = (pos(e, rt, uid(e, a[1])), pos(e, rt, leader(e)));
                let r: f32 = a[2].parse().unwrap();
                let d = match (p, h) { (Some(p), Some(h)) => (p[0] - h[0]).hypot(p[2] - h[2]), _ => f32::INFINITY };
                let near = d < r;
                println!("P17  f{f} ifnear {} {r} {near} (d {d})", a[1]);
                if near {
                    for s in a[3..].join(" ").split(',').rev() {
                        q.push_front(s.trim().replace('|', ","));
                    }
                }
                (0, 0)
            }
            "ifpw" => {
                // P20 R166: `ifpw LO HI s1, s2..` runs the comma steps only when the book power (GB1+4) is in [LO,HI]
                // (EP3/L05: the wisp walk between 0x9e and 0x9f only when the 0x9e fight left enough power); '|' -> ','
                let (lo, hi): (f32, f32) = (a[1].parse().unwrap(), a[2].parse().unwrap());
                let pw = e.rt.game_buffers.get(&1).map_or(f32::NAN, |b| e.mem.rf(b.0 + 4));
                println!("P17  f{f} ifpw power {pw} in [{lo},{hi}]");
                if pw >= lo && pw <= hi {
                    for s in a[3..].join(" ").split(',').rev() {
                        q.push_front(s.trim().replace('|', ","));
                    }
                }
                (0, 0)
            }
            "ifx" | "ifz" => {
                // P24 R24: `ifx UID LO HI s1, s2..` runs the comma steps only when UID's x is in [LO,HI] (EP4/L01:
                // under NODESET the 0x103 east push leaves the hero east of the pipe; he walks around it to the south)
                // P23 R58 `ifz`: same on z (EP6/L04A ANIMSND=0: 0x1d1 is sucked with the hero north of the 646 room corner)
                let (lo, hi): (f32, f32) = (a[2].parse().unwrap(), a[3].parse().unwrap());
                // P38 R4: UID `L` = the current leader (EP6/L04A clip-on respawn recovery `iflc ifx L 1280 1340 ..`)
                let u = if a[1] == "L" { leader(e) } else { uid(e, a[1]) };
                let x = pos(e, rt, u).map_or(f32::NAN, |p| if a[0] == "ifz" { p[2] } else { p[0] });
                println!("P17  f{f} {} {} v {x} in [{lo},{hi}]", a[0], a[1]);
                if x >= lo && x <= hi {
                    for s in a[4..].join(" ").split(',').rev() {
                        q.push_front(s.trim().to_string());
                    }
                }
                (0, 0)
            }
            "ifst" => {
                // P23 R58 `ifst UID STATE s1, s2..`: run the comma steps only when UID's current state ID is STATE ('|' -> ',';
                // EP6/L04A ANIMSND=0: a SQUARE pulse on the frame the book closed after sucking 0x1cf re-opened it empty ->
                // the hero stood rooted in tomedoom state 3, out of 0x1cd's range: `ifst @tomedoom 3 p 8000 10` closes it)
                let t = uid(e, a[1]);
                let st = scooby_recomp::engine::core::apigameobj::GetCurrentStateID(e, t);
                println!("P17  f{f} ifst {} state {st}", a[1]);
                if st == a[2].parse::<u32>().unwrap() {
                    for s in a[3..].join(" ").split(',').rev() {
                        q.push_front(s.trim().replace('|', ","));
                    }
                }
                (0, 0)
            }
            "ifcap" => {
                // P23 R56 `ifcap UID 1|0 s1, s2..`: run the comma steps only when UID is captured (gone or state 0xb, as the book
                // `c` check) (1) / not (0) ('|' -> ','; EP6/L02 NODESET: refill the book before 0x1a9 only when the first
                // capture did not suck it in)
                let t = uid(e, a[1]);
                let cap = e.rt.map.find(t).is_none() || scooby_recomp::engine::core::apigameobj::GetCurrentStateID(e, t) == 0xb;
                println!("P17  f{f} ifcap {} captured {cap}", a[1]);
                if cap == (a[2] == "1") {
                    for s in a[3..].join(" ").split(',').rev() {
                        q.push_front(s.trim().replace('|', ","));
                    }
                }
                (0, 0)
            }
            "waitfar" => {
                // P24 R21: `waitfar X Z R N UID..` waits (10-frame steps, at most N frames) until every UID is at
                // least R (xz) from (X,Z) (gates a leg on the patrolling ghosts, e.g. EP4/L01 0x18b/0x18c)
                let (x, z, r, n): (f32, f32, f32, i32) = (a[1].parse().unwrap(), a[2].parse().unwrap(), a[3].parse().unwrap(), a[4].parse().unwrap());
                let near = a[5..].iter().any(|s| pos(e, rt, uid(e, s)).is_some_and(|p| (p[0] - x).hypot(p[2] - z) < r));
                if near && n > 0 {
                    q.push_front(format!("waitfar {} {}", a[1..5].iter().enumerate().map(|(i, s)| if i == 3 { (n - 10).to_string() } else { s.to_string() }).collect::<Vec<_>>().join(" "), a[5..].join(" ")));
                    q.push_front("f 10".to_string());
                } else {
                    println!("P17  f{f} waitfar {x} {z} (left {n})");
                }
                (0, 0)
            }
            "waitx" => {
                // P22 R11: `waitx UID LO HI N` waits (10-frame steps, at most N frames) until UID's x is in
                // [LO, HI] (syncs to a patrolling enemy's phase, e.g. the EP4/L01 pipe-room ghost 0x18b)
                let u = uid(e, a[1]);
                let (lo, hi, n): (f32, f32, i32) = (a[2].parse().unwrap(), a[3].parse().unwrap(), a[4].parse().unwrap());
                let x = pos(e, rt, u).map_or(f32::NAN, |p| p[0]);
                if !(lo..=hi).contains(&x) && n > 0 {
                    q.push_front(format!("waitx {} {} {} {}", a[1], a[2], a[3], n - 10));
                    q.push_front("f 10".to_string());
                } else {
                    println!("P17  f{f} waitx {u:#x} x {x} (left {n})");
                }
                (0, 0)
            }
            "waitfa" => {
                // P23 R51: `waitfa UID LO HI N` waits (10-frame steps, at most N frames) until UID's facing angle
                // atan2(at.z, at.x) in degrees is in [LO, HI] (EP6/L02 NODESET: the sweeping videocamhead 0x170 spots
                // the hero at the robotcrawl 0x173 push and the l02_camera cuts to state 3, freezing him)
                // P23 R52: `waitfa UID LO HI N MOD [UID2 LO2 HI2 ..]`: angles taken modulo MOD (LO > HI = wrapped
                // window) and every UID/window must hold at once (EP6/L02 square arc boxes 0x18d/0x191, which get a
                // random relative rotation on each switch-on: wait for a switch-on that leaves the diagonal gap open)
                let n: i32 = a[4].parse().unwrap();
                let m: f32 = a.get(5).map_or(0.0, |s| s.parse().unwrap());
                let mut ok = true;
                let mut msg = String::new();
                let mut tri = vec![(a[1], a[2], a[3])];
                tri.extend(a.get(6..).unwrap_or(&[]).chunks(3).filter(|c| c.len() == 3).map(|c| (c[0], c[1], c[2])));
                for (us, los, his) in tri {
                    let u = uid(e, us);
                    let (lo, hi): (f32, f32) = (los.parse().unwrap(), his.parse().unwrap());
                    let at = rt.particles.lock().unwrap().world.frames.get(&u).map(|fr| fr.at);
                    let mut d = at.map_or(f32::NAN, |v| v[2].atan2(v[0]).to_degrees());
                    if m > 0.0 { d = d.rem_euclid(m) }
                    ok &= if lo <= hi { (lo..=hi).contains(&d) } else { d >= lo || d <= hi };
                    msg += &format!(" {u:#x} angle {d}");
                }
                if !ok && n > 0 {
                    q.push_front(format!("waitfa {} {} {} {} {}", a[1], a[2], a[3], n - 10, a[5..].join(" ")));
                    q.push_front("f 10".to_string());
                } else {
                    println!("P17  f{f} waitfa{msg} (left {n})");
                }
                (0, 0)
            }
            "waitst" => {
                // P20 R135: `waitst UID STATE N` waits (10-frame steps, at most N frames) until UID's current
                // state ID (GetCurrentStateID) is STATE (game-state gate instead of a blind delay)
                let u = uid(e, a[1]);
                let (st, n): (u32, i32) = (a[2].parse().unwrap(), a[3].parse().unwrap());
                let s = scooby_recomp::engine::core::apigameobj::GetCurrentStateID(e, u);
                if s != st && n > 0 {
                    q.push_front(format!("waitst {} {} {}", a[1], a[2], n - 10));
                    q.push_front("f 10".to_string());
                } else {
                    println!("P17  f{f} waitst {u:#x} state {s} (left {n})");
                }
                (0, 0)
            }
            // P24 R27 `lsmod NAME`: every object of module NAME: uid, position, hidden flag, local words 0..8
            "lsmod" => {
                let hid = rt.particles.lock().unwrap().world.hidden.clone();
                let os: Vec<(u32, u32)> = e.rt.objects.iter().flatten().filter(|o| o.inst.code.is_some_and(|k| &*e.rt.codes[k].def.name == a[1])).map(|o| (o.id, o.inst.local)).collect();
                for (u, lo) in os {
                    let w: Vec<String> = (0..8).map(|i| format!("{:x}", e.mem.r32(lo + 4 * i))).collect();
                    println!("P17  f{f} lsmod {u:#x} {:?} hid {} local {}", pos(e, rt, u).map(|p| p.map(|v| (v * 100.0).round() / 100.0)), hid.contains(&u), w.join(" "));
                }
                (0, 0)
            }
            "dbg" => {
                let u = uid(e, a[1]);
                let lo = e.rt.objects.iter().flatten().find(|o| o.id == u).map_or(0, |o| o.inst.local);
                // `dbg UID [LEN [OFF]]`
                let lo = lo + a.get(3).map_or(0, |s| hex(s));
                let w: Vec<String> = (0..hex(a.get(2).unwrap_or(&"20")) / 4).map(|i| format!("{:x}", e.mem.r32(lo + 4 * i))).collect();
                println!("P17  f{f} {u:#x} local {}", w.join(" "));
                (0, 0)
            }
            // `dl UID OFF LEN`: dump an object's local data (inst.local + OFF), hex
            "dl" => {
                let u = uid(e, a[1]);
                let b = e.rt.objects.iter().flatten().find(|o| o.id == u).map_or(0, |o| o.inst.local);
                let w: Vec<String> = (0..hex(a[3]) / 4).map(|i| format!("{:x}", e.mem.r32(b + hex(a[2]) + 4 * i))).collect();
                println!("P17  f{f} dl {u:#x}+{} {}", a[2], w.join(" "));
                (0, 0)
            }
            "dm" => {
                // `dm @NAME OFF LEN`: OFF relative to module NAME's data base (0x10000 = start)
                if a[1].starts_with('@') {
                    let base = e.rt.codes.iter().find(|c| &*c.def.name == &a[1][1..]).map_or(0, |c| c.data);
                    let a0 = base + hex(a[2]) - 0x10000;
                    let w: Vec<String> = (0..hex(a[3]) / 4).map(|i| format!("{:x}", e.mem.r32(a0 + 4 * i))).collect();
                    println!("P17  f{f} mem {}+{} {}", a[1], a[2], w.join(" "));
                } else {
                    // dump memory: `dm ADDR LEN` (hex), e.g. a hero's pLocalDataHero (local word 0)
                    let w: Vec<String> = (0..hex(a[2]) / 4).map(|i| format!("{:x}", e.mem.r32(hex(a[1]) + 4 * i))).collect();
                    println!("P17  f{f} mem {} {}", a[1], w.join(" "));
                }
                (0, 0)
            }
            "pos" => {
                // P23 R58: `pos L` = the current leader (hero)
                let u = if a[1] == "L" { leader(e) } else { uid(e, a[1]) };
                let b = e.mem.alloca(0x10);
                rt.particles.lock().unwrap().logic = true;
                // `pos UID [BONE]` (P22 R10: the bone form = the hero's lever snap GetPosition(item, 5))
                let bone = a.get(2).map_or(0, |t| hex(t));
                let r = APIgameobj::GetPosition(e, u, bone, b);
                rt.particles.lock().unwrap().logic = false;
                let wf = rt.particles.lock().unwrap().world.frames.get(&u).map(|m| m.pos);
                println!("P17  f{f} pos {u:#x} api {r} {:?} frame {wf:?}", e.mem.rv3(b));
                (0, 0)
            }
            "grp" => {
                // group bit mask (TomeDoomFUNC_ScanForMonster needs 0x200) + state
                let u = uid(e, a[1]);
                rt.particles.lock().unwrap().logic = true;
                let g = APIgameobj::GetGroupBitMask(e, u);
                rt.particles.lock().unwrap().logic = false;
                println!("P17  f{f} group {u:#x} {g:#x} state {}", scooby_recomp::engine::core::apigameobj::GetCurrentStateID(e, u));
                (0, 0)
            }
            "nt" => {
                let u = uid(e, a[2]);
                println!("P17  f{f} notify {} {u:#x}: {:?}", a[1], e.rt.notify.maps[hex(a[1]) as usize].get(u).map(|l| l.iter().map(|n| (n.owner, n.data.cb)).collect::<Vec<_>>()));
                (0, 0)
            }
            "hz" => {
                // `hz N`: logic step 1/N s from here on (original: 30, see common::HZ)
                crate::common::HZ.store(a[1].parse().unwrap(), std::sync::atomic::Ordering::Relaxed);
                (0, 0)
            }
            "near" => {
                // `near X Z R`: objects within R (xz) of X Z: uid, module, state, position
                let (x, z, r): (f32, f32, f32) = (a[1].parse().unwrap(), a[2].parse().unwrap(), a[3].parse().unwrap());
                let ids: Vec<(u32, Option<usize>)> = e.rt.objects.iter().flatten().map(|o| (o.id, o.inst.code)).collect();
                for (u, k) in ids {
                    if let Some(p) = pos(e, rt, u) {
                        if (p[0] - x).hypot(p[2] - z) <= r {
                            let m = k.map(|k| e.rt.codes[k].def.name.to_string());
                            let at = rt.particles.lock().unwrap().world.frames.get(&u).map(|fr| fr.at);
                            println!("P17  f{f} near {u:#x} {m:?} state {} at {p:?} facing {at:?}", scooby_recomp::engine::core::apigameobj::GetCurrentStateID(e, u));
                        }
                    }
                }
                (0, 0)
            }
            // P20 R69 `refs UID..`: static trigger finder: every object whose local data (level custom data +
            // runtime state) holds one of the UIDs as a 32-bit word: object, module, state, word offset
            "refs" => {
                let want: Vec<u32> = a[1..].iter().map(|t| uid(e, t)).collect();
                let objs: Vec<(u32, Option<usize>, u32, u32)> = e.rt.objects.iter().flatten().map(|o| (o.id, o.inst.code, o.inst.local, o.inst.local_size)).collect();
                for (u, k, l, n) in objs {
                    if l == 0 { continue }
                    for off in (0..n & !3).step_by(4) {
                        let v = e.mem.r32(l + off);
                        if want.contains(&v) && v != u {
                            let m = k.map(|k| e.rt.codes[k].def.name.to_string());
                            println!("P17  f{f} refs {v:#x} <- {u:#x} {m:?} state {} local+{off:#x} at {:?}", scooby_recomp::engine::core::apigameobj::GetCurrentStateID(e, u), pos(e, rt, u));
                        }
                    }
                }
                (0, 0)
            }
            // P20 R69 `doors`: every hs_door: position, local[0..4] and the position of local[0] (destination locator)
            "doors" => {
                let objs: Vec<(u32, u32)> = e.rt.objects.iter().flatten().filter(|o| o.inst.code.is_some_and(|k| e.rt.codes[k].def.name.starts_with("hs_door"))).map(|o| (o.id, o.inst.local)).collect();
                for (u, l) in objs {
                    let w: Vec<u32> = (0..5).map(|i| if l == 0 { 0 } else { e.mem.r32(l + 4 * i) }).collect();
                    println!("P17  f{f} door {u:#x} at {:?} local {w:x?} -> {:?}", pos(e, rt, u), pos(e, rt, w[0]));
                }
                (0, 0)
            }
            // P32 R91 `rng`: the shared libc rand() state (MyRand / particles / audio)
            "rng" => {
                { let v = rt.rand.0.lock().unwrap().0; println!("P17  f{f} rng {v:#x}"); }
                (0, 0)
            }
            "srand" => {
                // `srand N`: set the shared libc rand() state (the PS2's state at a level entry
                // depends on everything played before it; this samples other entry states)
                let v: u64 = a[1].parse().unwrap();
                rt.rand.0.lock().unwrap().0 = v;
                (0, 0)
            }
            "who" => {
                // `who UID..`: module name, state and position of objects
                for t in &a[1..] {
                    let u = uid(e, t);
                    let m = e.rt.objects.iter().flatten().find(|o| o.id == u).and_then(|o| o.inst.code).map(|k| e.rt.codes[k].def.name.to_string());
                    let lf = e.rt.objects.iter().flatten().find(|o| o.id == u).map_or(0, |o| o.logic_flags);
                    // P32 R88: + the GAME_LOGICCLIP clip word (s1.clip_cur, None when clip is off)
                    let cw = { let g = rt.particles.lock().unwrap(); (g.s1.clip_cur.get(&u).copied(), rt.logic_clip.as_ref().map(|l| l.contains(&u)), g.sub.physic.clip_off.contains(&u), g.sub.anim_clip_off.contains(&u)) };
                    // P32 R90: + the module stack pointer (a leak between frames shows as a growing sp)
                    println!("P17  f{f} who {u:#x} {m:?} state {} lflags {lf:#x} clip(word,logic,physoff,animoff) {cw:x?} sp {:#x} at {:?}", scooby_recomp::engine::core::apigameobj::GetCurrentStateID(e, u), e.mem.sp(), pos(e, rt, u));
                }
                (0, 0)
            }
            // R81 `contacts N [LX LY]`: N frames (stick LX LY hex, default centre); each frame prints the leader's
            // collision-stage contacts: other object uid / prim index, or env map prim (word id, plane), normal
            "contacts" => {
                let n: u32 = a[1].parse().unwrap();
                let st = (a.get(2).map_or(128, |s| hex(s) as u8), a.get(3).map_or(128, |s| hex(s) as u8));
                for _ in 0..n {
                    f += 1;
                    if let Some(r) = frame_stick(e, rt, 0, st) { got = Some((norm(&r.level), f)); break 'outer }
                    let l = std::env::var("P17_CONTUID").ok().map_or_else(|| leader(e), |s| u32::from_str_radix(&s, 16).unwrap());
                    let g = rt.particles.lock().unwrap();
                    let me = g.sub.physic.PhysicOf(l);
                    if std::env::var_os("P17_CONTDBG").is_some() { println!("P17  contacts me {me} stages {:?}", g.sub.collision.stages.iter().map(|s| (s.dyn_ref, s.col_prims.clone(), s.contacts.len())).collect::<Vec<_>>()) }
                    for s in &g.sub.collision.stages {
                        if !s.col_prims.iter().any(|&(_, pid)| pid == me) { continue }
                        let c: Vec<String> = s.contacts.iter().map(|c| match (&c.prim, &c.env) {
                            (Some((u, i)), _) => format!("obj {u:#x}/prim {i} n {:.2?}", c.n),
                            (_, Some(v)) => format!("env #{} mat {:x?} plane {:.2?} d {:.2} n {:.2?}", v.id, &v.mat[..2], v.normal, v.d, c.n),
                            _ => format!("other phys {} n {:.2?}", c.other, c.n),
                        }).collect();
                        println!("P17  f{f} contacts {}: {}", c.len(), c.join(" | "));
                    }
                }
                (0, 0)
            }
            // R123 `prims UID..`: the object's collision prims (kind, tag, radius, world centre, flag words 0..12)
            "prims" => {
                for t in &a[1..] {
                    let u = uid(e, t);
                    let g = rt.particles.lock().unwrap();
                    let id = g.sub.physic.PhysicOf(u);
                    if let Some(ph) = g.sub.physic.Physic(id) {
                        for (i, pr) in ph.model.prims.iter().enumerate() {
                            if let Some(pr) = pr { println!("P17  f{f} prims {u:#x} #{i} kind {} tag {} r {:.2} c {:.2?} j {:x} ltm {:.2?} w {:x?}", pr.base.kind, pr.base.tag, pr.base.radius, pr.base.center, pr.base.joint, pr.base.ltm, &pr.base.words[..12]); if let scooby_recomp::engine::physic::model::PrimExt::BBox { half, m, .. } = &pr.ext { println!("P17   bbox half {half:.2?} m {:.3?}", &m[..3]) } else if let scooby_recomp::engine::physic::model::PrimExt::SweptSphere { world_vec, sradius, .. } = &pr.ext { println!("P17   swept {world_vec:.2?} r {sradius:.2}") } }
                        }
                    } else { println!("P17  f{f} prims {u:#x} no physic") }
                }
                (0, 0)
            }
            // R124 `alive [TAG]`: soft-lock probe. pushes the stick in 4 directions (45 f each); if the leader never moved
            // 0.3 m, taps X for 120 f (text boxes) and pushes again,
            // then Triangle (puzzle / camera exit);
            // prints `ALIVE TAG ok|FROZEN moved M flags GB0+0x14 leader` (FROZEN = leader moved < 0.3 m in all four)
            "alive" => {
                let l0 = leader(e);
                let p0 = pos(e, rt, l0).unwrap_or([0.0; 3]);
                let mut best = 0f32;
                // pass 0: stick only; pass 1 (only if still frozen): X taps first (text boxes / prompts), then stick
                for pass in 0..3 {
                    if best >= 0.3 { break }
                    if pass > 0 {
                        // pass 1 taps X (text boxes / prompts), pass 2 taps Triangle (exit puzzle / camera views)
                        let k = if pass == 1 { scooby_recomp::engine::input::pad::CROSS } else { scooby_recomp::engine::input::pad::TRIANGLE };
                        for i in 0..120 {
                            f += 1;
                            let b = if i % 20 < 3 { k } else { 0 };
                            if let Some(r) = frame_stick(e, rt, b, (128, 128)) { got = Some((norm(&r.level), f)); break 'outer }
                        }
                    }
                    for st in [(255u8, 128u8), (128, 255), (0, 128), (128, 0)] {
                        let s0 = pos(e, rt, leader(e)).unwrap_or([0.0; 3]);
                        for _ in 0..45 {
                            f += 1;
                            if let Some(r) = frame_stick(e, rt, 0, st) { got = Some((norm(&r.level), f)); break 'outer }
                        }
                        let s1 = pos(e, rt, leader(e)).unwrap_or([0.0; 3]);
                        best = best.max((s1[0] - s0[0]).hypot(s1[2] - s0[2]));
                    }
                }
                let fl = e.rt.game_buffers.get(&0).map_or(0, |b| e.mem.r32(b.0 + 0x14));
                println!("P17  f{f} ALIVE {} {} moved {best:.2} flags {fl:#x} leader {:#x} at {:?} (from {:?})", a.get(1).unwrap_or(&"-"), if best < 0.3 { "FROZEN" } else { "ok" }, leader(e), pos(e, rt, leader(e)), p0);
                (0, 0)
            }
            // R125 `gbdump PATH`: write all game buffers (P17_CARRY format) to PATH (diff a save point against its load)
            "gbdump" => {
                // P20 R131: not carry(e): its TerminateGameBuffers frees the non-persistent buffers (ApiSectorClipping's
                // client lists, GB 40000) of the running level -> later crash in api_list. Persistent buffers only, no free.
                let gbs: Vec<(u32, Vec<u8>)> = e.rt.game_buffers.iter().filter(|&(&id, &(p, _))| !(p >= 0x14 && e.mem.r32(p - 0x14) == id && e.mem.r32(p - 4) == p && e.mem.r8(p - 0x14 + 0xc) == 0))
                    .map(|(&id, &(p, _))| (id, (0..e.mem.r32(p - 0x10)).map(|i| e.mem.r8(p + i)).collect())).collect();
                let o: Vec<u8> = gbs.into_iter().flat_map(|(id, v)| [id.to_le_bytes().to_vec(), (v.len() as u32).to_le_bytes().to_vec(), v].concat()).collect();
                std::fs::write(a[1], o).unwrap();
                println!("P17  f{f} gbdump {}", a[1]);
                (0, 0)
            }
            // R126 `pausequit`: Start -> pause menu (continue / game options / episode file / quit) -> 3x down to quit -> X
            // -> yes/no prompt (default no; stick left/right toggles) -> left -> X: fade, then the level request for INTRO
            // (panel_baseFUNC_MainMenuConfirm 0xc, panel_baseFUNC_YesNoConfirm 0x32). The run ends at that request.
            "pausequit" => {
                for s in "p 8 3; f 100; s 80 ff 10; f 30; s 80 ff 10; f 30; s 80 ff 10; f 30; p 4000 3; f 100; s 0 80 10; f 30; p 4000 3; f 900".rsplit(';') { q.push_front(s.trim().into()) }
                (0, 0)
            }
            // R126 `menuload`: on INTRO, the title menu Load of the first save on the card (as the player does)
            "menuload" => {
                for s in "f 100; p 8 3; f 100; s 80 ff 10; f 30; p 4000 3; f 200; p 4000 3; f 200; s 80 ff 10; f 30; p 4000 3; f 300; p 4000 3; f 300; p 4000 3; f 600".rsplit(';') { q.push_front(s.trim().into()) }
                (0, 0)
            }
            // R127 `save`: the checkpoint save dialog as a player drives it (stand on the checkpoint first). X opens it;
            // on an empty card: "create?" (stick down = yes, X) -> slot list -> X on slot 1 -> "overwrite?" (down = yes) -> X.
            // When the card already holds the save file (a second save, or the STATUS saves of earlier episodes) there
            // is no "create?" prompt: X -> slot list -> X on slot 1 -> "overwrite?" -> down -> X (slot 1 is overwritten).
            "save" => {
                let has = std::env::var("GAME_SAVEDIR").is_ok_and(|d| std::path::Path::new(&d).join("BASLUS-20701/BASLUS-20701").is_file());
                let s = if has { "f 90; p 4000 3; f 400; p 4000 3; f 300; s 80 ff 10; f 60; p 4000 3; f 400" } else { "f 90; p 4000 3; f 400; s 80 ff 10; f 60; p 4000 3; f 400; p 4000 3; f 300; s 80 ff 10; f 60; p 4000 3; f 400" };
                // P22 R29: a tutorial text on screen (GB0+0x14 bit 0x1000000, e.g. EP1/L01 "cameras are used to save")
                // holds the team and takes the first X: dismiss it first, as the player does
                let tut = e.rt.game_buffers.get(&0).map_or(0, |b| e.mem.r32(b.0 + 0x14)) & 0x1000000 != 0;
                println!("P17  f{f} save ({}{})", if has { "overwrite" } else { "new card file" }, if tut { ", dismiss tutorial text" } else { "" });
                for s in s.rsplit(';') { q.push_front(s.trim().into()) }
                if tut { q.push_front("f 120".into()); q.push_front("p 4000 3".into()) }
                (0, 0)
            }
            // R125 `card`: files under GAME_SAVEDIR (memory card dir) with sizes
            "card" => {
                fn walk(d: &std::path::Path, out: &mut Vec<String>) {
                    for en in std::fs::read_dir(d).into_iter().flatten().flatten() {
                        let p = en.path();
                        if p.is_dir() { walk(&p, out) } else { out.push(format!("{} {}", p.display(), en.metadata().map_or(0, |m| m.len()))) }
                    }
                }
                let mut v = vec![];
                if let Ok(d) = std::env::var("GAME_SAVEDIR") { walk(std::path::Path::new(&d), &mut v) }
                println!("P17  f{f} card {} files: {}", v.len(), v.join(" | "));
                (0, 0)
            }
            "pl" => {
                let l = leader(e);
                println!("P17  f{f} leader {l:#x} at {:?}", pos(e, rt, l));
                (0, 0)
            }
            "c" => {
                // pickup: leader onto the object, 30 frames (no re-warp)
                let u = uid(e, a[1]);
                // (collected wisps are parked at the origin / y ~1000 until they respawn: skip those)
                if let Some(p) = pos(e, rt, u).filter(|p| p[1] < 500.0 && p[0].abs() + p[2].abs() > 1.0) { warp(e, rt, p) }
                at = None;
                (0, 30)
            }
            // R86 `cwg [N [MOD [K]]]` (R87: K = nearest K only): real pickup walk: queues `goto` (N f, default 300) to every live object of module
            // MOD (default wisps; e.g. coll_level5 = L05 snacks), nearest first
            // P32 R4 `cwgp TOTAL T`: state-based wisp refill: `cwg 300 wisps 3` + 30 f rounds while the book power
            // (GB1+4) is below T, for up to ~TOTAL frames (the wispsourcenew dispenser now layers its additive
            // release anims as the C does, so its wisps drift around the skull and one `cwg` pass may miss some)
            "cwgp" => {
                let (tot, t): (i64, f32) = (a[1].parse().unwrap(), a[2].parse().unwrap());
                let pw = e.rt.game_buffers.get(&1).map_or(0.0, |b| if b.0 != 0 { e.mem.rf(b.0 + 4) } else { 0.0 });
                if pw < t && tot > 0 {
                    q.push_front(format!("cwgp {} {t}", tot - 360));
                    q.push_front("f 30".into());
                    q.push_front("cwg 300 wisps 3".into());
                }
                (0, 0)
            }
            "cwg" => {
                let n = a.get(1).copied().unwrap_or("300");
                let md = a.get(2).copied().unwrap_or("wisps");
                let me = pos(e, rt, leader(e)).unwrap_or([0.0; 3]);
                let mut ws: Vec<[f32; 3]> = e.rt.objects.iter().flatten().filter(|o| o.inst.code.is_some_and(|k| &*e.rt.codes[k].def.name == md)).map(|o| o.id).collect::<Vec<_>>().into_iter().filter_map(|u| pos(e, rt, u)).filter(|p| p[1] < 500.0 && p[0].abs() + p[2].abs() > 1.0).collect();
                ws.sort_by(|a, b| ((a[0] - me[0]).hypot(a[2] - me[2])).total_cmp(&(b[0] - me[0]).hypot(b[2] - me[2])));
                ws.truncate(a.get(3).and_then(|k| k.parse().ok()).unwrap_or(usize::MAX));
                for p in ws.iter().rev() {
                    q.push_front("f 10".into());
                    q.push_front(format!("goto {} {} {n}", p[0], p[2]));
                }
                (0, 0)
            }
            "cw" => {
                // pick up every live wisp (EP4/L04 truck drops; parked ones skipped), 30 frames each
                let ws: Vec<u32> = e.rt.objects.iter().flatten().filter(|o| o.inst.code.is_some_and(|k| &*e.rt.codes[k].def.name == "wisps")).map(|o| o.id).collect();
                for u in ws {
                    let Some(p) = pos(e, rt, u).filter(|p| p[1] < 500.0 && p[0].abs() + p[2].abs() > 1.0) else { continue };
                    if std::env::var("P17_CWDBG").is_ok() { println!("P17  f{f} cw {u:#x} {p:?}") }
                    warp(e, rt, p);
                    for _ in 0..30 {
                        f += 1;
                        if let Some(r) = frame(e, rt, 0) { got = Some((norm(&r.level), f)); break 'outer }
                    }
                }
                at = None;
                (0, 0)
            }
            // WP-P21 R59 `farm N [T]`: up to N frames, pick up live wisps (as `cw`) until the book power
            // (GB1+4) is >= T (default 130); the truck drops a new batch every ~1200 f
            "farm" => {
                let n: u32 = a[1].parse().unwrap();
                let thr: f32 = a.get(2).map_or(130.0, |s| s.parse().unwrap());
                let power = |e: &Eng| e.rt.game_buffers.get(&1).map_or(0.0, |b| if b.0 != 0 { e.mem.rf(b.0 + 4) } else { 0.0 });
                let back = pos(e, rt, leader(e));
                let mut i = 0;
                while i < n && power(e) < thr {
                    let ws: Vec<u32> = e.rt.objects.iter().flatten().filter(|o| o.inst.code.is_some_and(|k| &*e.rt.codes[k].def.name == "wisps")).map(|o| o.id).collect();
                    let mut took = false;
                    for u in ws {
                        let Some(p) = pos(e, rt, u).filter(|p| p[1] < 3.0 && p[0].abs() + p[2].abs() > 1.0) else { continue };
                        warp(e, rt, p);
                        took = true;
                        for _ in 0..30 {
                            f += 1;
                            i += 1;
                            if let Some(r) = frame(e, rt, 0) { got = Some((norm(&r.level), f)); break 'outer }
                        }
                        if power(e) >= thr { break }
                    }
                    if !took {
                        f += 1;
                        i += 1;
                        if let Some(r) = frame(e, rt, 0) { got = Some((norm(&r.level), f)); break 'outer }
                    }
                }
                println!("P17  f{f} farm power {} after {i} f", power(e));
                if let Some(p) = back { warp(e, rt, p) }
                at = None;
                (0, 0)
            }
            "book" | "bookw" | "bookz" | "bookzg" | "bookg" | "bookgr" | "bookgn" | "bookgs" | "bookgb" => {
                // R108 `bookgs`: `bookgr`, but once the target is within 5.5 m (book range 5.7) the leader stands
                // still and mashes SQUARE (the book opening takes ~20 f; a dust devil touching the hero meanwhile
                // cancels it and costs 0.3 cool meter)
                // R109 `bookgb`: `bookgs` + backing off while the book is closed and the target is < 3.5 m away
                let backoff = a[0] == "bookgb";
                let stand = a[0] == "bookgs" || backoff;
                let a: Vec<&str> = a.iter().map(|w| if *w == "bookgs" || *w == "bookgb" { "bookgr" } else { *w }).collect();
                // P24 R4 `bookzg N D UID..`: `bookz` target picking, but the leader WALKS (bookgr chase) instead of warps
                let zmode = a[0] == "bookz" || a[0] == "bookzg";
                // P32 R34: the R32 empty-book close-outs below run only under GAME_LOGICCLIP=1 (default routes unchanged:
                // unconditional they moved the EP3/L05 default route, gate 290/2)
                let clip_on = matches!(std::env::var("GAME_LOGICCLIP").as_deref().map(str::trim), Ok("1"));
                let a: Vec<&str> = a.iter().map(|w| if *w == "bookzg" { "bookgr" } else { *w }).collect();
                // R95 `bookgn`: `bookgr`, ignoring target positions > 60 m away (EP6/L04B phasing ghosts
                // 0x14d/0x14e blink to the parked spot near the origin between appearances)
                let near60 = a[0] == "bookgn";
                let a: Vec<&str> = a.iter().map(|w| if *w == "bookgn" { "bookgr" } else { *w }).collect();
                // R83 `bookg`: as `book`, but the leader WALKS to the target (planner path, replanned every
                // 60 f, stick steering) instead of being warped beside it
                // R84 `bookgr`: `bookg` chasing harder (replan every 15 f, straight steer within 6 m)
                let walk = a[0] == "bookg" || a[0] == "bookgr";
                let (rep, near) = if a[0] == "bookgr" { (15, 36.0) } else { (60, 0.0) };
                let (mut wpath, mut wscr): (Vec<[f32; 3]>, u32) = (vec![], 0);
                // WP-P21 R59 `bookw`: as `book`, but while the book is closed and the power (GB1+4) is below a
                // full drain (< 80) no SQUARE (an empty open burns power): the leader collects live wisps instead
                let farmw = a[0] != "book" && (!walk || zmode);
                // `bookz N D UID..`: as `bookw`, target = the first UID that is out (state != 5 parked, not at the
                // origin), re-picked every 20 f, D m to the side; runs N f (EP4/L04: the 3 zombies must all be
                // parked at once before the zombieboss leaves its seat)
                // (`r:UID` among them: the recall hot-spot, the leader is warped onto it every 600 f while low)
                let zs: Vec<u32> = if zmode { a[3..].iter().filter(|t| !t.starts_with("r:") && !t.starts_with("t:") && !t.starts_with("k:")).map(|t| uid(e, t)).collect() } else { vec![] };
                let recall = a.iter().find_map(|t| t.strip_prefix("r:")).map(|t| uid(e, t));
                // P32 R91 (`t:UID`, bookzg): empty book, no wisps -> walk up to UID (EP4/L04 truck 0x2a0000c3, which drops a
                // wisp every 3 s only while its logic runs = while on screen, clip word 0x6a7 as C truckFUNC_SetClipping)
                // and wait there (stepping off zombies) instead of walking into a zombie, as a player would
                let truck = a.iter().find_map(|t| t.strip_prefix("t:")).map(|t| uid(e, t));
                // P32 R92 (`k:X:Z`, bookzg): empty book, no wisps -> walk to the kite spot (X, Z) and wait there (stepping
                // off zombies): a chasing zombie loses the hero beyond 6 m and goes back to shake the truck (the wisp source)
                let kite: Option<[f32; 2]> = a.iter().find_map(|t| t.strip_prefix("k:")).map(|t| { let v: Vec<f32> = t.split(':').map(|x| x.parse().unwrap()).collect(); [v[0], v[1]] });
                // (and, as a player watching the meter would, no hook below one capture's cost: 67.5 power at L04;
                // `k:X:Z:MIN` = no hook below MIN power)
                let kmin = if kite.is_some() { a.iter().find_map(|t| t.strip_prefix("k:")).and_then(|t| t.split(':').nth(2)).map_or(67.0, |m| m.parse().unwrap()) } else { 0.0 };
                // real book use: SQUARE pulses (10 of every 60 f) while the book is closed (state 1), the
                // requested button (book L+0x1e8) mashed every 2nd frame while it drains (state 3), as
                // tests/book_regress.rs. Stops after N frames or when `book N UID` target reached 0xb/removed.
                let bk = uid(e, "@tomedoom");
                let mut tgt = if zs.is_empty() { a.get(2).map(|t| uid(e, t)) } else { None };
                let n: u32 = a[1].parse().unwrap();
                // `book N UID D[f]`: D m to the side; `f` = keep following while hooked/draining too
                let d: f32 = a.get(if zs.is_empty() { 3 } else { 2 }).map_or(0.0, |s| s.trim_end_matches('f').parse().unwrap());
                let follow = zs.is_empty() && a.get(3).is_some_and(|s| s.ends_with('f'));
                let mut last = u32::MAX;
                let mut since = 0u32;
                // P22 R39 bookzg: a wisp the leader could not get closer than 1 m to (no 0.5 m progress in 120 f, e.g.
                // walking into a tomb), or not collected within 600 f (circling a hovering one), is skipped for 900 f
                let mut skip: Vec<(u32, u32)> = vec![];
                let (mut anc, mut anc_i, mut wuid, mut w_i) = ([0f32; 3], 0u32, None::<u32>, 0u32);
                // (and the walk to the boss/zombie blocked the same way: 45 f sidestep, alternating sides;
                // an empty book next to a skipped wisp: open it, the 0x54 scan pulls wisps within 5.6 m)
                let (mut sanc, mut sanc_i, mut szu, mut zskip) = ([0f32; 3], 0u32, None::<u32>, Vec::<(u32, u32)>::new());
                let (mut banc, mut banc_i, mut side_to, mut side_s, mut pull_to) = ([0f32; 3], 0u32, 0u32, 1f32, 0u32);
                for i in 0..n {
                    // P23 R54 (opt-in, trailing `c`): target already captured (by the previous book step's suck-in) -> no SQUARE: the
                    // old post-frame check opened an empty book first (EP6/L02 0x1a9, GAME_ANIMDIS=0: the hero stood
                    // rooted in tomedoom state 3 for ~900 f)
                    if let Some(t) = tgt.filter(|_| zs.is_empty() && i == 0 && (a.last() == Some(&"c") || std::env::var_os("P17_BOOKC").is_some())) {
                        if e.rt.map.find(t).is_none() || scooby_recomp::engine::core::apigameobj::GetCurrentStateID(e, t) == 0xb {
                            println!("P17  f{f} book target {t:#x} captured");
                            break;
                        }
                    }
                    let bs = scooby_recomp::engine::core::apigameobj::GetCurrentStateID(e, bk);
                    if bs != last {
                        let l = e.rt.map.find(bk).map_or(0, |h| e.rt.objects[h].as_ref().unwrap().inst.local);
                        let gb1 = e.rt.game_buffers.get(&1).map_or(0, |b| b.0);
                        println!("P17  f{f} book state {bs} life {} hook {:#x} lock {:#x} power {}", if l != 0 { e.mem.rf(l + 0x70) } else { 0.0 }, if l != 0 { e.mem.r32(l + 0x60) } else { 0 }, if l != 0 { e.mem.r32(l + 0x54) } else { 0 }, if gb1 != 0 { e.mem.rf(gb1 + 4) } else { 0.0 });
                        last = bs;
                        since = i;
                    }
                    let l = e.rt.map.find(bk).map_or(0, |h| e.rt.objects[h].as_ref().unwrap().inst.local);
                    // chase: the target wanders (boss-fight dust devils) -> leader back onto it while not hooked
                    // (`book N UID D`: D m to the side of it instead, out of its contact damage)
                    if std::env::var("P17_BOOKDBG").is_ok_and(|v| i % v.parse::<u32>().unwrap_or(300).max(1) == 0) {
                        let st: Vec<u32> = zs.iter().chain(tgt.iter()).map(|&u| scooby_recomp::engine::core::apigameobj::GetCurrentStateID(e, u)).collect();
                        let pw = e.rt.game_buffers.get(&1).map_or(0.0, |b| if b.0 != 0 { e.mem.rf(b.0 + 4) } else { 0.0 });
                        let cool = e.rt.game_buffers.get(&1).map_or(0.0, |b| if b.0 != 0 { e.mem.rf(b.0) } else { 0.0 });
                        let gc: Vec<(u32, u32)> = e.rt.objects.iter().flatten().filter(|o| o.inst.code.is_some_and(|k| &*e.rt.codes[k].def.name == "gamecontroller") && o.inst.local != 0).map(|o| (e.mem.r32(o.inst.local + 8), e.mem.r32(o.inst.local + 12))).collect();
                        println!("P17  f{f} bookz dbg book {bs} states {st:?} power {pw} cool {cool} gc {gc:?} leader {:?} boss {:?}", pos(e, rt, leader(e)), zs.last().or(tgt.as_ref()).and_then(|&u| pos(e, rt, u)));
                        // P23 R55 P17_BOOKDBGU=uid,uid..: positions of extra objects too
                        for u in std::env::var("P17_BOOKDBGU").unwrap_or_default().split(',').filter_map(|h| u32::from_str_radix(h, 16).ok()) {
                            println!("P17  f{f} bookz dbg {u:#x} {:?} st {}", pos(e, rt, u), scooby_recomp::engine::core::apigameobj::GetCurrentStateID(e, u));
                        }
                    }
                    if !zs.is_empty() && i % 20 == 0 {
                        let mut out = |u: u32| scooby_recomp::engine::core::apigameobj::GetCurrentStateID(e, u) != 5 && pos(e, rt, u).is_some_and(|p| p[0].abs() + p[2].abs() > 1.0);
                        let t = zs.iter().copied().find(|&u| out(u));
                        if t != tgt { println!("P17  f{f} bookz target {t:x?}") }
                        tgt = t;
                    }
                    let low = farmw && (bs == 1 || bs == 5) && e.rt.game_buffers.get(&1).map_or(0.0, |b| if b.0 != 0 { e.mem.rf(b.0 + 4) } else { 0.0 }) < if walk { 170.0 } else { 60.0 };
                    // (bookzg: while power < 170, WALK to the nearest live wisp (not within 3 m of a zombie)
                    // instead of the warps below; SQUARE still allowed at >= 60)
                    let me0 = pos(e, rt, leader(e)).unwrap_or([0.0; 3]);
                    let wp = if walk && low {
                        let ids: Vec<u32> = e.rt.objects.iter().flatten().filter(|o| o.inst.code.is_some_and(|k| &*e.rt.codes[k].def.name == "wisps")).map(|o| o.id).collect();
                        let mut best: Option<[f32; 3]> = None;
                        let mut any: Option<([f32; 3], u32)> = None;
                        let mut bu = None;
                        // P22 R59: a collected launched wisp (Picked -> Dead_In hides it -> Removed_In) is only
                        // killed once GD+0xc > 1, until then it lingers invisible at its pickup spot: skip hidden ones
                        let hid = rt.particles.lock().unwrap().world.hidden.clone();
                        for u in ids {
                            if hid.contains(&u) || skip.iter().any(|&(s, t)| s == u && i < t + 900) { continue }
                            let st = scooby_recomp::engine::core::apigameobj::GetCurrentStateID(e, u);
                            if std::env::var_os("P24_WST").is_some() { println!("P17 wisp {u:#x} st {st} {:?}", pos(e, rt, u)) }
                            let Some(p) = pos(e, rt, u).filter(|p| p[1] < 3.0 && p[0].abs() + p[2].abs() > 1.0) else { continue };
                            if any.is_none_or(|(q, _)| (p[0] - me0[0]).hypot(p[2] - me0[2]) < (q[0] - me0[0]).hypot(q[2] - me0[2])) { any = Some((p, u)) }
                            if zmode && zs.iter().filter_map(|&z| pos(e, rt, z)).any(|z| z[0].abs() + z[2].abs() > 1.0 && (z[0] - p[0]).hypot(z[2] - p[2]) < 3.0) { continue }
                            if best.is_none_or(|q| (p[0] - me0[0]).hypot(p[2] - me0[2]) < (q[0] - me0[0]).hypot(q[2] - me0[2])) { best = Some(p); bu = Some(u) }
                        }
                        // P22 R39: empty book and every live wisp next to a zombie -> go for the nearest one anyway
                        // (the flee rule below still steps off a zombie within 2.5 m)
                        if best.is_none() && zmode && e.rt.game_buffers.get(&1).map_or(0.0, |b| if b.0 != 0 { e.mem.rf(b.0 + 4) } else { 0.0 }) < 60.0 {
                            if let Some((p, u)) = any { best = Some(p); bu = Some(u) }
                        }
                        if bu != wuid { wuid = bu; anc = me0; anc_i = i; w_i = i }
                        best
                    } else { None };
                    // P22 R40: empty book and no wisp left to walk to -> let a zombie/the boss catch the hero: the
                    // caught fade (hero 0x4f with GB0+0x14 0x8000000) sends 0xc1 to group 6 = every collected level
                    // wisp (Wisp_baseState_Dead_Execute respawn); that is the original's only wisp refill here
                    let starve = zmode && walk && (bs == 1 || bs == 5) && wp.is_none() && e.rt.game_buffers.get(&1).map_or(0.0, |b| if b.0 != 0 { e.mem.rf(b.0 + 4) } else { 0.0 }) <= kmin;
                    let truck_wait = starve && truck.is_some_and(|t| pos(e, rt, t).is_some());
                    let truck_wait = truck_wait || (starve && kite.is_some());
                    let wp = if let Some(k) = kite.filter(|_| truck_wait) {
                        if i % 300 == 0 { println!("P17  f{f} bookz empty book, no wisps: kite at {k:?}") }
                        let dd = (k[0] - me0[0]).hypot(k[1] - me0[2]);
                        Some(if dd > 1.0 { [k[0], me0[1], k[1]] } else { me0 })
                    } else if truck_wait {
                        let p = pos(e, rt, truck.unwrap()).unwrap();
                        if i % 300 == 0 { println!("P17  f{f} bookz empty book, no wisps: wait at truck {:#x} {p:?}", truck.unwrap()) }
                        let dd = (p[0] - me0[0]).hypot(p[2] - me0[2]);
                        Some(if dd > 6.0 { p } else { me0 })
                    } else if starve {
                        if i % 300 == 0 { println!("P17  f{f} bookz empty book, no wisps: walk into a zombie (caught -> wisps respawn)") }
                        // P22 R60: a zombie that does not catch the hero (idle/returning: the hero stands pressed against
                        // it, no progress for 240 f) is skipped for 600 f -> walk into the next one
                        if (sanc[0] - me0[0]).hypot(sanc[2] - me0[2]) > 0.5 { sanc = me0; sanc_i = i }
                        else if i - sanc_i > 240 { if let Some(z) = szu { println!("P17  f{f} bookz zombie {z:#x} does not catch, next"); zskip.push((z, i)) } sanc_i = i }
                        let zz = zs.iter().copied().filter(|z| !zskip.iter().any(|&(s, t)| s == *z && i < t + 600)).filter_map(|z| pos(e, rt, z).map(|p| (z, p))).filter(|(_, p)| p[0].abs() + p[2].abs() > 1.0).min_by(|(_, x), (_, y)| (x[0] - me0[0]).hypot(x[2] - me0[2]).total_cmp(&(y[0] - me0[0]).hypot(y[2] - me0[2])));
                        szu = zz.map(|(z, _)| z);
                        zz.map(|(_, p)| p)
                    } else { sanc = me0; sanc_i = i; wp };
                    // bookzg: a zombie touching the hero scares it (hero_scared 0xc9 -> L04 gamecontroller Reset:
                    // capture count back to 0) -> while the book is closed, step away from any zombie within 2.5 m
                    let flee = if walk && zmode && (!starve || truck_wait) && (bs == 1 || bs == 5) {
                        zs.iter().filter_map(|&u| pos(e, rt, u).map(|p| (p, 2.5f32))).filter(|(p, _)| p[0].abs() + p[2].abs() > 1.0).map(|(p, r)| (p, (p[0] - me0[0]).hypot(p[2] - me0[2]), r)).filter(|&(_, dd, r)| dd < r).map(|(p, dd, r)| ((p, r), dd)).min_by(|x, y| x.1.total_cmp(&y.1)).map(|((p, fr), dd)| { let k = (fr + 1.5) / dd.max(0.1); [me0[0] + (me0[0] - p[0]) * k, me0[1], me0[2] + (me0[2] - p[2]) * k] })
                    } else { None };
                    let stuck = if flee.is_some() || wp.is_none_or(|q| (q[0] - me0[0]).hypot(q[2] - me0[2]) < 1.0) || (anc[0] - me0[0]).hypot(anc[2] - me0[2]) > 0.5 { anc = me0; anc_i = i; false } else { i - anc_i > 120 };
                    if wp.is_some() && flee.is_none() && (stuck || i - w_i > 600) { if let Some(u) = wuid { println!("P17  f{f} bookz wisp {u:#x} unreachable, skipped"); skip.push((u, i));
                        if zmode && e.rt.game_buffers.get(&1).map_or(0.0, |b| if b.0 != 0 { e.mem.rf(b.0 + 4) } else { 0.0 }) <= 0.0 && bs == 1 && wp.is_some_and(|q| (q[0] - me0[0]).hypot(q[2] - me0[2]) < 5.0) { pull_to = i + 10 } wuid = None; anc_i = i; w_i = i } }
                    let wp = flee.or(wp);
                    let low = low && !walk;
                    if let Some(r) = recall.filter(|_| low && i % 600 == 300) {
                        if let Some(p) = pos(e, rt, r) { warp(e, rt, p) }
                    }
                    if low && i % 30 == 0 && i % 600 != 300 {
                        let w = e.rt.objects.iter().flatten().filter(|o| o.inst.code.is_some_and(|k| &*e.rt.codes[k].def.name == "wisps")).map(|o| o.id).collect::<Vec<_>>();
                        if let Some(p) = w.into_iter().filter_map(|u| pos(e, rt, u)).find(|p| p[1] < 3.0 && p[0].abs() + p[2].abs() > 1.0) { warp(e, rt, p) }
                    }
                    let mut stick = (128u8, 128u8);
                    // R85: bookgr, SQUARE only once within 5 m of the target (an open book with nothing in
                    // range stays in state 3: the hero can't walk up to it any more)
                    let mut far = false;
                    let mut inr = false;
                    if walk {
                        if let (Some(t), Some(me)) = (tgt.filter(|_| l != 0 && (follow || e.mem.r32(l + 0x60) == 0)), pos(e, rt, leader(e))) {
                            if let Some(p) = wp.or_else(|| pos(e, rt, t).filter(|p| !near60 || (p[0] - me[0]).hypot(p[2] - me[2]) < 60.0)) {
                                let goal = if wp.is_some() { p } else { [p[0] + d, p[1], p[2]] };
                                let d2 = |q: [f32; 3]| (q[0] - me[0]).powi(2) + (q[2] - me[2]).powi(2);
                                far = a[0] == "bookgr" && d2(p) > 25.0;
                                inr = stand && wp.is_none() && bs == 1 && d2(p) < 30.25;
                                if !inr && d2(goal) > if wp.is_some() { 0.04 } else { 2.25 } {
                                    if i % rep == 0 || wpath.is_empty() {
                                        if wscr == 0 { wscr = e.mem.malloc(0x80) }
                                        let j2 = scooby_recomp::engine::runtime::J2(&rt.particles);
                                        wpath = scooby_recomp::engine::nav::nav_path(&mut e.mem, &j2, wscr, me, [goal[0], me[1], goal[2]], 15.0);
                                    }
                                    let mut tp = goal;
                                    if d2(goal) < near {} else if let Some(k) = (0..wpath.len()).min_by(|&u, &v| d2(wpath[u]).total_cmp(&d2(wpath[v]))) {
                                        tp = wpath[k..].iter().find(|&&q| d2(q) > 1.0).copied().unwrap_or(goal);
                                    }
                                    stick = steer(rt, me, tp);
                                    if zmode && wp.is_none() && far {
                                        if (banc[0] - me[0]).hypot(banc[2] - me[2]) > 0.5 { banc = me; banc_i = i }
                                        else if i - banc_i > 120 && i >= side_to { println!("P17  f{f} bookz blocked at {me:?}, sidestep"); side_to = i + 45; side_s = -side_s; banc_i = i }
                                        if i < side_to { stick = steer(rt, me, [me[0] - (goal[2] - me[2]) * side_s, me[1], me[2] + (goal[0] - me[0]) * side_s]) }
                                    } else { banc = me; banc_i = i }
                                }
                                // R109 bookgb: closed book and the target closer than 3.5 m (just released / missed
                                // hook): back off straight away from it, re-hook from range
                                if backoff && wp.is_none() && bs == 1 && d2(p) < 12.25 {
                                    inr = false;
                                    stick = steer(rt, me, [2.0 * me[0] - p[0], me[1], 2.0 * me[2] - p[2]]);
                                }
                            }
                        }
                    } else if let Some(t) = tgt.filter(|_| !low && i % 20 == 0 && l != 0 && (follow || e.mem.r32(l + 0x60) == 0)) {
                        if let Some(p) = pos(e, rt, t) { warp(e, rt, [p[0] + d, p[1], p[2]]) }
                    }
                    let mut b = 0u16;
                    if bs == 1 && !low && flee.is_none() && (wp.is_none() || e.rt.game_buffers.get(&1).map_or(0.0, |b| if b.0 != 0 { e.mem.rf(b.0 + 4) } else { 0.0 }) >= 60.0) && !far && i % 60 < 10 && (zs.is_empty() || i - since > 120) { b = scooby_recomp::engine::input::pad::SQUARE }
                    if inr && flee.is_none() && i % 2 == 0 { b = scooby_recomp::engine::input::pad::SQUARE }
                    // P22 R38 bookzg: an open book still scanning (L+0x58 0x54, nothing hooked) after 2 s -> SQUARE
                    // again = heroCB_INPUT_Tome (>= 0.75 s in the tome) -> event_ACTION_TOME_OUT -> 0xd0, book put
                    // away, as a player would; the C has no give-up in the 0x54 scan (TomeDoomST_OPEN), so the bot
                    // stood with the open book forever when the boss had moved out of range
                    // (also an open book with 0 power: TomeDoomFUNC_MessageHandler 0xd1 skips the scan, nothing hooked)
                    if zmode && walk && bs == 3 && l != 0 && e.mem.r32(l + 0x60) == 0 && (e.mem.r32(l + 0x54) == 0 || e.mem.r32(l + 0x58) == 0x54) && i - since > 120 && i % 60 < 10 { b = scooby_recomp::engine::input::pad::SQUARE }
                    // P32 R32: every book mode: an open book with 0 power and nothing hooked scans nothing
                    // (TomeDoomFUNC_MessageHandler 0xd1) and stays open until SQUARE -> put it away, as a player would
                    // (R30 EP2/L02C: `bookgs` left it open and the hero stood frozen for the rest of the level)
                    // P38 R5: (clip only) also an open book with power left but nothing hooked for > 300 f (EP6/L04B: opened with
                    // no target in range at 1810,213 and the hero stood rooted) -> SQUARE, as a player would
                    if clip_on && !zmode && bs == 3 && l != 0 && e.mem.r32(l + 0x60) == 0 && (e.rt.game_buffers.get(&1).map_or(1.0, |b| if b.0 != 0 { e.mem.rf(b.0 + 4) } else { 1.0 }) <= 0.0 || i - since > 300) && i - since > 120 && i % 60 < 10 { b = scooby_recomp::engine::input::pad::SQUARE }
                    // R38: bookzg, no SQUARE on an empty book (power 0: nothing to capture with)
                    if zmode && walk && bs == 1 && e.rt.game_buffers.get(&1).map_or(0.0, |b| if b.0 != 0 { e.mem.rf(b.0 + 4) } else { 0.0 }) <= kmin { b = 0 }
                    if i < pull_to && bs == 1 { b = scooby_recomp::engine::input::pad::SQUARE }
                    if bs == 3 && l != 0 && i % 2 == 0 {
                        use scooby_recomp::engine::input::pad;
                        b |= match e.mem.r32(l + 0x1e8) { 2 => pad::CROSS, 4 => pad::CIRCLE, 8 => pad::TRIANGLE, _ => 0 };
                    }
                    if std::env::var("P22_BOTDBG").is_ok() && i % 30 == 0 { println!("P17  f{f} botdbg bs {bs} tgt {tgt:x?} low {low} wp {wp:?} flee {flee:?} far {far} inr {inr} stick {stick:?} b {b:#x} hook {:#x}", if l != 0 { e.mem.r32(l + 0x60) } else { 0 }) }
                    f += 1;
                    if let Some(r) = frame_stick(e, rt, b, stick) {
                        got = Some((norm(&r.level), f));
                        break 'outer;
                    }
                    if let Some(t) = tgt.filter(|_| zs.is_empty()) {
                        if e.rt.map.find(t).is_none() || scooby_recomp::engine::core::apigameobj::GetCurrentStateID(e, t) == 0xb {
                            println!("P17  f{f} book target {t:#x} captured");
                            break;
                        }
                    }
                }
                // P32 R32: step over with the book open, empty (power 0) and nothing hooked -> SQUARE until it is put
                // away (<= 90 f); the next walking step otherwise starts with the hero rooted in the tome
                for i in 0..if clip_on { 90 } else { 0 } {
                    let l = e.rt.map.find(bk).map_or(0, |h| e.rt.objects[h].as_ref().unwrap().inst.local);
                    let pw = e.rt.game_buffers.get(&1).map_or(1.0, |b| if b.0 != 0 { e.mem.rf(b.0 + 4) } else { 1.0 });
                    // P38 R5: any power (an unhooked open book roots the hero whatever its power)
                    let _ = pw;
                    if scooby_recomp::engine::core::apigameobj::GetCurrentStateID(e, bk) != 3 || l == 0 || e.mem.r32(l + 0x60) != 0 { break }
                    f += 1;
                    if let Some(r) = frame(e, rt, if i % 2 == 0 { scooby_recomp::engine::input::pad::SQUARE } else { 0 }) { got = Some((norm(&r.level), f)); break 'outer }
                }
                (0, 0)
            }
            // `press UID OFF [DIR]`: hero-action on an object whose X input is armed by collision with the
            // leader (button/switch local+OFF != 0): warp onto it, walk DIR (pad bits, default 0x20) until
            // armed (retries), then one CROSS press and 60 f
            "press" => {
                let t = uid(e, a[1]);
                let off = hex(a[2]);
                let dir = a.get(3).map_or(0x20, |s| hex(s) as u16);
                let armed = |e: &Eng| e.rt.map.find(t).map_or(false, |h| { let l = e.rt.objects[h].as_ref().unwrap().inst.local; l != 0 && e.mem.r32(l + off) != 0 });
                let mut ok = false;
                'tries: for k in 0..12u32 {
                    if let Some(p) = pos(e, rt, t) { warp(e, rt, p) }
                    for i in 0..10 + 4 + 2 * (k % 4) {
                        f += 1;
                        if let Some(r) = frame(e, rt, if i >= 10 { dir } else { 0 }) { got = Some((norm(&r.level), f)); break 'outer }
                        if i >= 10 && armed(e) { ok = true; break 'tries }
                    }
                }
                println!("P17  f{f} press {t:#x} armed {ok}");
                for i in 0..65 {
                    f += 1;
                    let b = if ok && (2..5).contains(&i) { scooby_recomp::engine::input::pad::CROSS } else { 0 };
                    if let Some(r) = frame(e, rt, b) { got = Some((norm(&r.level), f)); break 'outer }
                }
                (0, 0)
            }
            // WP-P21 R46 `go X Z [N] [X]`: real walk (no teleport): left stick towards (X, Z) relative to the
            // active camera (as game.rs route_pad) until within 0.6 m or N frames (default 600); `X` = also pulse
            // CROSS every 20 f (take a door / hot-spot on the way)
            // WP-P21 R47 `goto X Z [N] [X]`: the same along an A* path (engine::nav::nav_path over the collision
            // map), replanned every 240 f or when the leader made < 0.5 m progress in 90 f
            // WP-P21 R50 `ray X Z`: debug, horizontal rays (map + objects) from the leader towards (X, Z) at
            // 0.3 / 0.8 / 1.4 m, prints the raw hit words
            // `hmap X0 Z0 X1 Z1 STEP Y`: floor heights (down rays from Y, len Y+5) on a grid; '.' = no hit
            // P22 R48 `envq X Y Z R M1 M2`: EFPhysicMgr::GetPrimitivesInSphere per map (stage env gather); `envq X Y Z R M1 M2 A`
            // = scan every primitive of every map whose vertex bbox comes within R of (X, Z) (any y), no octree/masks
            "envq" => {
                use scooby_recomp::engine::physic::collision_map::EFGetPrimitivesInSphereData;
                let v: Vec<f32> = a[1..5].iter().map(|s| s.parse().unwrap()).collect();
                let (m1, m2) = (hex(a[5]), hex(a[6]));
                let mut g = rt.particles.lock().unwrap();
                let mgr = &mut g.sub.physic;
                mgr.query_stamp = mgr.query_stamp.wrapping_add(1);
                let stamp = mgr.query_stamp;
                for (mi, m) in mgr.maps.iter_mut().enumerate() {
                    let list: Vec<usize> = if a.get(7) == Some(&"A") {
                        m.Primitives().into_iter().filter(|&p| { let n = m.PrimCount(p) & 0x7fff; let vs: Vec<_> = (0..n).map(|i| m.PrimVertex(p, i)).collect();
                            let (x0, x1) = vs.iter().fold((f32::MAX, f32::MIN), |a, q| (a.0.min(q[0]), a.1.max(q[0]))); let (z0, z1) = vs.iter().fold((f32::MAX, f32::MIN), |a, q| (a.0.min(q[2]), a.1.max(q[2])));
                            n > 0 && v[0] + v[3] >= x0 && v[0] - v[3] <= x1 && v[2] + v[3] >= z0 && v[2] - v[3] <= z1 }).collect()
                    } else {
                        let mut d = EFGetPrimitivesInSphereData { center: [v[0], v[1], v[2]], radius: v[3], mask1: m1, mask2: m2, out: Vec::new(), capacity: 64 };
                        m.GetPrimitivesInSphere(&mut d, stamp);
                        d.out
                    };
                    for p in list {
                        let mat = m.PrimMaterial(p);
                        let n = m.PrimCount(p);
                        let vs: Vec<String> = (0..n & 0x7fff).map(|i| format!("{:.1?}", m.PrimVertex(p, i))).collect();
                        println!("P17  f{f} envq map {mi} prim {p} mat {:x} {:x} {:x} n {n:#x} nrm {:.2?} d {:.2} v {}", m.MaterialWord(mat, 0), m.MaterialWord(mat, 1), m.MaterialWord(mat, 2), m.PrimNormal(p), m.PrimD(p), vs.join(" "));
                    }
                }
                (0, 0)
            }
            "hmap" => {
                let b: Vec<f32> = a[1..7].iter().map(|s| s.parse().unwrap()).collect();
                let scr = e.mem.alloca(0x80);
                let j2 = scooby_recomp::engine::runtime::J2(&rt.particles);
                let mut z = b[1];
                while z <= b[3] {
                    let mut row = format!("P17  hmap z {z:.0}:");
                    let mut x = b[0];
                    while x <= b[2] {
                        let w = [x, b[5], z, 0.0, -(b[5] + std::env::var("P24_HDEEP").map_or(5.0, |s| s.parse().unwrap())), 0.0].map(f32::to_bits);
                        for (i, v) in w.iter().chain(&[u32::MAX, u32::MAX, 3, 0, leader(e)]).enumerate() { e.mem.w32(scr + 4 * i as u32, *v) }
                        let r = j2.ray_intersect(&mut e.mem, scr, scr + 0x30);
                        let y = f32::from_bits(e.mem.r32(scr + 0x34));
                        // P24 R13 `hmap ... A`: height + trail-bike floor class (material word 2 bits, 1..8; '-' none) or 'X' (word 1 bit 31 = accident floor)
                        if a.get(7) == Some(&"A") {
                            let (w1, w2) = (e.mem.r32(scr + 0x30 + 8 * 4), e.mem.r32(scr + 0x30 + 9 * 4));
                            let c = if r != 2 { '?' } else if w1 & 0x8000_0000 != 0 { 'X' } else { (0..8).find(|i| w2 & (0x101 << i) & 0x0002_00ff | w2 & (0x400 << i) != 0).map_or('-', |i| char::from(b'1' + i as u8)) };
                            row += &if r != 0 { format!(" {y:3.0}{c}") } else { "    .".into() };
                        } else {
                        row += &if r != 0 { format!(" {y:4.1}") } else { "    .".into() };
                        }
                        x += b[4];
                    }
                    println!("{row}");
                    z += b[4];
                }
                (0, 0)
            }
            "ray" => {
                let (x, z): (f32, f32) = (a[1].parse().unwrap(), a[2].parse().unwrap());
                let l = leader(e);
                if let Some(p) = pos(e, rt, l) {
                    let scr = e.mem.alloca(0x80);
                    let j2 = scooby_recomp::engine::runtime::J2(&rt.particles);
                    for hy in [0.3f32, 0.8, 1.4] {
                        // `ray X Z D`: down rays at (X, Z) from leader y + 3 * hy instead
                        let w = if a.get(3) == Some(&"D") { [x, p[1] + 3.0 * hy, z, 0.0, -6.0, 0.0] } else { [p[0], p[1] + hy, p[2], x - p[0], 0.0, z - p[2]] }.map(f32::to_bits);
                        for (i, v) in w.iter().chain(&[u32::MAX, u32::MAX, 3, 0, l]).enumerate() { e.mem.w32(scr + 4 * i as u32, *v) }
                        let r = j2.ray_intersect(&mut e.mem, scr, scr + 0x30);
                        let o: Vec<String> = (0..12).map(|i| { let v = e.mem.r32(scr + 0x30 + 4 * i); format!("{v:#x}/{:.2}", f32::from_bits(v)) }).collect();
                        let hit = e.mem.r32(scr + 0x48);
                        let code = e.rt.objects.iter().flatten().find(|o| o.id == hit).and_then(|o| o.inst.code.map(|c| e.rt.codes[c].def.name.to_string()));
                        println!("P17  f{f} ray {hy} from {p:?}: {r} {code:?} {o:?}");
                    }
                }
                (0, 0)
            }
            // P23 R5 `drive UID X Z N [S]`: tank-steer a remote-controlled object (EP6/L02 robot: stick left/right
            // rotates, up/down drives, robot_selfcontrol.c robotCB_INPUT_STICK) onto X Z using its world frame `at`;
            // S = -1 flips the rotate direction. Stops within 0.5 m.
            "drive" => {
                let u = uid(e, a[1]);
                let (x, z): (f32, f32) = (a[2].parse().unwrap(), a[3].parse().unwrap());
                let n: u32 = a.get(4).map_or(600, |s| s.parse().unwrap());
                let sg: f32 = a.get(5).map_or(1.0, |s| s.parse().unwrap());
                let mut done = false;
                for _ in 0..n {
                    let fr = rt.particles.lock().unwrap().world.frames.get(&u).cloned();
                    let Some(fr) = fr else { break };
                    let (dx, dz) = (x - fr.pos[0], z - fr.pos[2]);
                    let d = dx.hypot(dz);
                    if d < 0.5 { done = true; break }
                    let cr = fr.at[0] * dz - fr.at[2] * dx;
                    let dt = (fr.at[0] * dx + fr.at[2] * dz) / d.max(1e-6);
                    let stick = if dt > 0.97 { (128, 0) } else if cr * sg > 0.0 { (255, 128) } else { (0, 128) };
                    if std::env::var_os("P17_DRIVEDBG").is_some() && f % 10 == 0 { println!("P17  f{f} drive pos {:.2?} at {:.2?} dt {dt:.2} cr {cr:.2} stick {stick:?}", fr.pos, fr.at) }
                    f += 1;
                    if let Some(r) = frame_stick(e, rt, 0, stick) { got = Some((norm(&r.level), f)); break 'outer }
                }
                if !done { println!("P17  f{f} drive {x} {z} STUCK at {:?}", pos(e, rt, u)) }
                (0, 0)
            }
            "go" | "goto" => {
                let (x, z): (f32, f32) = (a[1].parse().unwrap(), a[2].parse().unwrap());
                let n: u32 = a.get(3).map_or(600, |s| s.parse().unwrap());
                // (R84: `TX` = both flags)
                let fl = a.get(4).copied().unwrap_or("");
                let px = fl.contains('X');
                // R72 `goto X Z N T`: stop when the leader is teleported (> 5 m in one frame: an hs_door)
                let tp = fl.contains('T');
                // R101 `P`: partner-safe legs: every 120 walk frames, hold still (<= 150 f) until the partner
                // (GB0+0xc) is within 3 m (C EFDMFollow drops a partner the leader outruns)
                let pw = fl.contains('P');
                // P22 R9 `H<hex>` flag: also hold these pad bits while steering (e.g. H800 = R1, snorkel hide/submerge)
                let hold: u16 = fl.find('H').map_or(0, |k| u16::from_str_radix(&fl[k + 1..], 16).unwrap_or(0));
                let mut leg = 0u32;
                let mut last: Option<[f32; 3]> = None;
                let mut done = false;
                let mut path: Vec<[f32; 3]> = vec![];
                let (mut plan_f, mut chk, mut scr) = (0u32, (0u32, [0f32; 3]), 0u32);
                for i in 0..n {
                    let l = leader(e);
                    let Some(p) = pos(e, rt, l) else { break };
                    let d2 = |q: [f32; 3]| (q[0] - p[0]).powi(2) + (q[2] - p[2]).powi(2);
                    if d2([x, 0.0, z]) < 0.36 { done = true; break }
                    if tp && last.is_some_and(|q| d2(q) > 25.0) { done = true; break }
                    last = Some(p);
                    leg += 1;
                    if pw && leg >= 120 {
                        leg = 0;
                        let pu = e.rt.game_buffers.get(&0).map(|b| e.mem.r32(b.0 + 0xc)).unwrap_or(0);
                        for _ in 0..150 {
                            let Some(pp) = pos(e, rt, pu) else { break };
                            if (pp[0] - p[0]).hypot(pp[2] - p[2]) < 3.0 { break }
                            f += 1;
                            if let Some(r) = frame_stick(e, rt, 0, (128, 128)) { got = Some((norm(&r.level), f)); break 'outer }
                        }
                        last = None;
                    }
                    let mut t = [x, 0.0, z];
                    if a[0] == "goto" {
                        if i >= chk.0 + 90 {
                            if d2(chk.1) < 0.25 { plan_f = 0 }
                            chk = (i, p);
                        }
                        if path.is_empty() || i >= plan_f + 240 || plan_f == 0 {
                            if scr == 0 { scr = e.mem.malloc(0x80) }
                            let j2 = scooby_recomp::engine::runtime::J2(&rt.particles);
                            path = scooby_recomp::engine::nav::nav_path(&mut e.mem, &j2, scr, p, [x, p[1], z], 15.0);
                            plan_f = i.max(1);
                            if std::env::var_os("P17_NAVDBG").is_some() { println!("P17  f{f} goto plan {} pts, end {:?}", path.len(), path.last()) }
                            if std::env::var("P17_NAVDBG").is_ok_and(|v| v == "2") { println!("P17  plan {:?}", path.iter().step_by(4).map(|q| ((q[0] * 10.0).round() / 10.0, (q[2] * 10.0).round() / 10.0)).collect::<Vec<_>>()) }
                        }
                        // next path point >= 1 m ahead of the nearest one
                        if let Some(k) = (0..path.len()).min_by(|&u, &v| d2(path[u]).total_cmp(&d2(path[v]))) {
                            t = path[k..].iter().find(|&&q| d2(q) > 1.0).copied().unwrap_or(t);
                        }
                    }
                    let stick = steer(rt, p, t);
                    // P38 R7 debug (P17_CLIPLOG=1): every 100 walk frames, leader/partner pos + state + stick
                    if i % 100 == 0 && std::env::var_os("P17_CLIPLOG").is_some() {
                        let pt = e.rt.game_buffers.get(&0).map(|b| e.mem.r32(b.0 + 0xc)).unwrap_or(0);
                        // P38 R8: the hs_door gates (hs_door_base.c CB_CollisionDetection / InputAction_X):
                        // gb0+8 leader, +0x14 & 0x800090d, +0x34 (0x110 / 0x1400), +0x7c; gb2[0] & 0x20000
                        let g0 = e.rt.game_buffers.get(&0).map(|b| b.0).unwrap_or(0);
                        let g2 = e.rt.game_buffers.get(&2).map(|b| b.0).unwrap_or(0);
                        if g0 != 0 {
                            let g2w = if g2 != 0 { e.mem.r32(g2) } else { 0 };
                            println!("P17  f{f} doorgate gb0+8 {:#x} +14 {:#x} +34 {:#x} +7c {:#x} gb2[0] {g2w:#x}", e.mem.r32(g0 + 8), e.mem.r32(g0 + 0x14), e.mem.r32(g0 + 0x34), e.mem.r32(g0 + 0x7c));
                        }
                        println!("P17  f{f} st L {l:#x} {:?} s{} P {pt:#x} {:?} s{} stick {stick:?}", p, scooby_recomp::engine::core::apigameobj::GetCurrentStateID(e, l), pos(e, rt, pt), scooby_recomp::engine::core::apigameobj::GetCurrentStateID(e, pt));
                    }
                    let b = if px && i % 20 < 2 { scooby_recomp::engine::input::pad::CROSS } else { 0 } | hold;
                    f += 1;
                    if let Some(r) = frame_stick(e, rt, b, stick) { got = Some((norm(&r.level), f)); break 'outer }
                }
                if !done { println!("P17  f{f} {} {x} {z} STUCK at {:?}", a[0], pos(e, rt, leader(e))) }
                // P38 R5 debug: P17_CLIPLOG=1 -> the stuck leader's clip state (word, logic list, state id)
                if !done && std::env::var_os("P17_CLIPLOG").is_some() {
                    let u = leader(e);
                    let w = rt.particles.lock().unwrap().s1.clip_word.get(&u).copied();
                    let lg = rt.logic_clip.as_ref().map(|l| l.contains(&u));
                    println!("P17  f{f} cliplog leader {u:#x} word {w:x?} logic {lg:?} state {}", scooby_recomp::engine::core::apigameobj::GetCurrentStateID(e, u));
                }
                at = None;
                (0, 0)
            }
            // P24 R5 `rideauto N [GAZ]`: EP4/L03 trail-bike driver (bike 0x2a00000d). Holds GAZ (pad bits, default
            // 8000 = TrailBike action 0x42) and steers: 3.2 m clear of an hs_rideobst ahead within 14 m / 2.6 m
            // of the bike's line (its NotifyColide sends 0x17d -> accident), else onto the next fuel can ahead.
            // Logs respawns (position jumps > 5 m = accident / out of fuel). P17_RIDEDBG=1 prints decisions.
            // P24 R7 `dodge N X Z DX DZ`: EP6/L05 fire-ghost boss. Holds the hero on (X, Z); while a thrown
            // fireball/fireballbig (> 1.5 m from the boss 0x2a0000c0) is within 12 m, walks to (X+DX, Z+DZ) so the
            // ball flies past and hits the shrubs behind (C: fireball NotifyColide prim 0xd8 -> shrub 0x1ec burn).
            // `dodge N X Z DX DZ F`: also stays off (X, Z) while a fireballbig_expl fire burns within 2.5 m of it, and steps away from a medium `ghost` closer than 3 m.
            "dodge" => {
                let n: u32 = a[1].parse().unwrap();
                let v: Vec<f32> = a[2..6].iter().map(|s| s.parse().unwrap()).collect();
                // P23 R27 `H<hex>` (any later arg): also hold these pad bits (H800 = R1 hides from the L05 fire ghosts)
                let hold: u16 = a[6..].iter().find_map(|s| s.strip_prefix('H')).map_or(0, |h| u16::from_str_radix(h, 16).unwrap_or(0));
                for _ in 0..n {
                    let l = leader(e);
                    let p = pos(e, rt, l).unwrap_or([0.0; 3]);
                    let b = pos(e, rt, 0x2a0000c0).unwrap_or([0.0; 3]);
                    let balls: Vec<u32> = e.rt.objects.iter().flatten().filter(|o| o.inst.code.is_some_and(|k| matches!(&*e.rt.codes[k].def.name, "fireball" | "fireballbig"))).map(|o| o.id).collect();
                    let near = balls.into_iter().filter_map(|u| pos(e, rt, u)).any(|q| (q[0] - b[0]).hypot(q[2] - b[2]) > 1.5 && (q[0] - p[0]).hypot(q[2] - p[2]) < 12.0 && q[1] > 0.3);
                    // a landed big ball leaves fireballbig_expl fire: touching it scares the hero (0xc9 resets the
                    // inv_control valve count and regrows the shrubs), so stay off the stand while one burns there
                    let fire: Vec<u32> = e.rt.objects.iter().flatten().filter(|o| o.inst.code.is_some_and(|k| &*e.rt.codes[k].def.name == "fireballbig_expl") && o.inst.state == 1).map(|o| o.id).collect();
                    let near = near || (a.get(6) == Some(&"F") && fire.into_iter().filter_map(|u| pos(e, rt, u)).any(|q| (q[0] - v[0]).hypot(q[2] - v[1]) < 2.5));
                    let mut t = if near { [v[0] + v[2], p[1], v[1] + v[3]] } else { [v[0], p[1], v[1]] };
                    // F: a medium `ghost` within 3 m scares on contact too -> step straight away from it
                    if a.get(6) == Some(&"F") {
                        let gh: Vec<u32> = e.rt.objects.iter().flatten().filter(|o| o.inst.code.is_some_and(|k| &*e.rt.codes[k].def.name == "ghost")).map(|o| o.id).collect();
                        if let Some(q) = gh.into_iter().filter_map(|u| pos(e, rt, u)).filter(|q| (q[0] - p[0]).hypot(q[2] - p[2]) < 3.0).min_by(|x, y| (x[0] - p[0]).hypot(x[2] - p[2]).total_cmp(&(y[0] - p[0]).hypot(y[2] - p[2]))) {
                            let (dx, dz) = (p[0] - q[0], p[2] - q[2]);
                            let l = dx.hypot(dz).max(0.01);
                            t = [p[0] + 3.0 * dx / l, p[1], p[2] + 3.0 * dz / l];
                        }
                    }
                    let st = if (t[0] - p[0]).hypot(t[2] - p[2]) > 0.3 { steer(rt, p, t) } else { (128, 128) };
                    f += 1;
                    // P23 R61 `R` (any later arg): release the held pad bits while a ball is near, so the hero runs
                    // (a hiding hero creeps ~0.03 m/frame and an aimed fireballbig needs ~25 frames to arrive)
                    let hold = if near && a[6..].contains(&"R") { 0 } else { hold };
                    if let Some(r) = frame_stick(e, rt, hold, st) { got = Some((norm(&r.level), f)); break 'outer }
                }
                (0, 0)
            }
            "rideauto" => {
                let n: u32 = a[1].parse().unwrap();
                let gaz = a.get(2).map_or(0x8000, |s| hex(s) as u16);
                let duty: u32 = a.get(3).map_or(1, |s| s.parse().unwrap());
                // P22: JUMP `t` = per obstacle type (hs_rideobst prim attrib 9): 2 (overhead) -> duck CIRCLE, 0x10 -> jump X, 1 -> steer around
                let typed = a.get(4).is_some_and(|s| *s == "t");
                let jz: Vec<(f32, f32, f32)> = std::env::var("P17_JZ").map_or(vec![], |w| w.split('/').map(|q| { let v: Vec<f32> = q.split(',').map(|t| t.parse().unwrap()).collect(); (v[0], v[1], *v.get(2).unwrap_or(&1.0)) }).collect()); // extra X jumps within R of x,z,R/..
                let jd: f32 = std::env::var("P17_JD").map_or(12.0, |s| s.parse().unwrap()); // jump distance before a type 0x10 obstacle
                let jump = if typed { 0 } else { a.get(4).map_or(0, |s| hex(s) as u16) };
                let mut jb = 0u16;
                let bike = 0x2a00000du32;
                let obst: Vec<([f32; 3], u32)> = e.rt.objects.iter().flatten().filter(|o| o.inst.code.is_some_and(|k| &*e.rt.codes[k].def.name == "hs_rideobst")).map(|o| o.id).collect::<Vec<_>>().into_iter().filter_map(|u| pos(e, rt, u).map(|p| (p, APIphysic::PrimGetIntAttrib(e, u, 0, 9) as u32))).collect();
                let obs: Vec<[f32; 3]> = obst.iter().filter(|b| !typed || b.1 == 1).map(|b| b.0).collect();
                let cans_of = |e: &mut Eng, rt: &mut EngineRuntime| -> Vec<[f32; 3]> { e.rt.objects.iter().flatten().filter(|o| o.inst.code.is_some_and(|k| &*e.rt.codes[k].def.name == "coll_level3") && e.mem.r32(o.inst.local + 0x14) == 0x40000011).map(|o| o.id).collect::<Vec<_>>().into_iter().filter_map(|u| pos(e, rt, u)).collect() };
                let mut cans = cans_of(e, rt);
                // P22: WP (6th arg) = `x,z/x,z/..` waypoints steered to (after obstacle avoidance) until passed
                let mut wp: Vec<[f32; 2]> = a.get(5).map_or(vec![], |w| w.split('/').map(|q| { let v: Vec<f32> = q.split(',').map(|t| t.parse().unwrap()).collect(); [v[0], v[1]] }).collect());
                let mut hist: std::collections::VecDeque<[f32; 3]> = Default::default();
                for i in 0..n {
                    if i % 30 == 0 { cans = cans_of(e, rt); }
                    let lo = e.rt.objects.iter().flatten().find(|o| o.id == bike).map_or(0, |o| o.inst.local);
                    let fuel = f32::from_bits(e.mem.r32(lo + 0x98));
                    let p = pos(e, rt, bike).unwrap_or([0.0; 3]);
                    if let Some(q) = hist.back() { if (p[0] - q[0]).hypot(p[2] - q[2]) > 5.0 { println!("P17  f{f} ride respawn {q:?} -> {p:?}"); hist.clear() } }
                    hist.push_back(p);
                    if hist.len() > 10 { hist.pop_front(); }
                    let o = hist[0];
                    let (vx, vz) = (p[0] - o[0], p[2] - o[2]);
                    let vl = vx.hypot(vz);
                    let mut stick = (128u8, 128u8);
                    if vl > 0.05 {
                        jb = if obs.iter().any(|b| { let (dx, dz) = (b[0] - p[0], b[2] - p[2]); let fd = (dx * vx + dz * vz) / vl; fd > 0.0 && fd < 5.0 && dx.hypot(dz) < 7.0 && (b[1] - p[1]).abs() < 4.0 }) { jump } else { 0 };
                        if typed {
                            jb = obst.iter().filter(|b| b.1 != 1).find(|(b, t)| { let (dx, dz) = (b[0] - p[0], b[2] - p[2]); let fd = (dx * vx + dz * vz) / vl; let (f0, f1) = if *t == 2 { (-1.0, 12.0) } else { (jd - 2.0, jd + 2.0) }; fd > f0 && fd < f1 && dx.hypot(dz) < f1 + 1.0 && (b[1] - p[1]).abs() < 5.0 }).map_or(0, |b| if b.1 == 2 { std::env::var("P17_T2").map_or(0x2000, |s| hex(&s) as u16) } else { 0x4000 });
                        }
                        let (fx, fz) = (vx / vl, vz / vl);
                        let (rx, rz) = (-fz, fx);
                        let thr = obs.iter().filter_map(|b| { let (dx, dz) = (b[0] - p[0], b[2] - p[2]); let fd = dx * fx + dz * fz; let lat = dx * rx + dz * rz; (fd > 0.0 && fd < 14.0 && lat.abs() < 2.6 && (b[1] - p[1]).abs() < 4.0).then_some((fd, lat)) }).min_by(|x, y| x.0.total_cmp(&y.0));
                        // the C stick CB is continuous (lateral speed local[0x44] = stick * -2.5): steer by the error
                        while wp.first().is_some_and(|w| (w[0] - p[0]) * fx + (w[1] - p[2]) * fz < 1.0) { wp.remove(0); }
                        let side = if let Some((_, lat)) = thr { Some(lat - 3.2 * lat.signum()) } else if let Some(w) = wp.first() { Some((w[0] - p[0]) * rx + (w[1] - p[2]) * rz) } else {
                            cans.iter().filter_map(|b| { let (dx, dz) = (b[0] - p[0], b[2] - p[2]); let fd = dx * fx + dz * fz; let lat = dx * rx + dz * rz; (fd > 1.0 && fd < 60.0 && lat.abs() < 4.5 && (b[1] - p[1]).abs() < 5.0).then_some((fd, lat)) }).min_by(|x, y| x.0.total_cmp(&y.0)).inspect(|c| if std::env::var_os("P17_RIDEDBG").is_some() && i % 10 == 0 { println!("P17  f{f} ride can {c:?} at {p:?}") }).map(|c| c.1)
                        };
                        if let Some(d) = side.filter(|d| d.abs() > 0.3) {
                            let full = steer(rt, p, [p[0] - rx * d.signum(), p[1], p[2] - rz * d.signum()]).0 as f32; // (bike stick is mirrored vs walking)
                            stick = ((128.0 + (full - 128.0) * (d.abs() / 1.5).min(1.0)) as u8, 128);
                            if std::env::var_os("P17_RIDEDBG").is_some() && i % 10 == 0 { println!("P17  f{f} ride steer {d} thr {thr:?} at {p:?}") }
                        }
                    }
                    // (TrailBike Standing_Execute: lean (stick, flags & 6) wins over duck 0x800 / jump 0x200 -> neutral stick)
                    if typed && jz.iter().any(|&(x, z, r)| (p[2] - z).hypot(p[0] - x) < r) { jb = 0x4000 }
                    if typed && jb != 0 { stick = (128, 128) }
                    if i % 300 == 0 || (std::env::var_os("P17_RIDEDBG").is_some() && i % 10 == 0) {
                        println!("P17  f{f} ride at {p:?} flags {:#x} st {:#x} jb {jb:#x} fuel {fuel} cans {}", e.mem.r32(lo), e.mem.r32(lo + 4), cans.len());
                    }
                    f += 1;
                    if let Some(r) = frame_stick(e, rt, (if fuel > 25.0 && i % duty == 0 { gaz } else { 0 }) | jb, stick) { got = Some((norm(&r.level), f)); break 'outer }
                }
                (0, 0)
            }
            // R110 `ledges X0 Z0 X1 Z1`: list the climbable ledge polys (material +4 bit 0x8000000, the
            // EFDMEdgeDetect tag-0x66 mask) in the box, as game.rs GAME_LEDGES
            "ledges" | "polys" => {
                // R115 `polys X0 Z0 X1 Z1`: same scan, every polygon (walls etc.), not only edge-flagged ones
                let allp = a[0] == "polys";
                use scooby_recomp::engine::physic::collision_map::EFGetPrimitivesInSphereData;
                use scooby_recomp::engine::physic::objcoll_check::EnvPrim;
                let b: Vec<f32> = a[1..5].iter().map(|s| s.parse().unwrap()).collect();
                let (cx, cz) = ((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0);
                let r = (b[2] - b[0]).hypot(b[3] - b[1]) / 2.0 + 50.0;
                let out = scooby_recomp::engine::runtime::J2(&rt.particles).physic(|m, _| {
                    let mut d = EFGetPrimitivesInSphereData { center: [cx, 0.0, cz], radius: r, mask1: !0, mask2: !0, out: Vec::new(), capacity: 1_000_000 };
                    m.query_stamp = m.query_stamp.wrapping_add(1);
                    let st = m.query_stamp;
                    let mut v = vec![];
                    let mut all = 0usize;
                    for map in m.maps.iter_mut() {
                        d.out.clear();
                        map.GetPrimitivesInSphere(&mut d, st);
                        all += d.out.len();
                        for &p in &d.out {
                            let e = EnvPrim::from_map(map, p);
                            // any edge-flagged poly (+4 edge mask): bit 0x8000000 = climb (hero prim 0x66), others = crouch (prim 0x67)
                            // `polys` (P20) or P17_ALLPOLY=1 (P24) lists every poly in the box, not only edge-flagged ones
                            if (!allp && e.w4 == 0 && std::env::var_os("P17_ALLPOLY").is_none()) || e.verts.is_empty() { continue }
                            let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
                            for q in &e.verts {
                                for k in 0..3 { lo[k] = lo[k].min(q[k]); hi[k] = hi[k].max(q[k]) }
                            }
                            if hi[0] < b[0] || lo[0] > b[2] || hi[2] < b[1] || lo[2] > b[3] { continue }
                            v.push(format!("ledge x {:.1}..{:.1} y {:.1}..{:.1} z {:.1}..{:.1} edgemask {:#x} n [{:.2},{:.2},{:.2}] mat {:x?}", lo[0], hi[0], lo[1], hi[1], lo[2], hi[2], e.w4, e.normal[0], e.normal[1], e.normal[2], e.mat));
                            // P27 R24 P17_POLYV=1: also the vertices (for floor maps)
                            if std::env::var_os("P17_POLYV").is_some() { v.push(format!("verts {:?}", e.verts)) }
                        }
                    }
                    v.push(format!("{all} polys scanned"));
                    v
                });
                for l in &out { println!("P17  {l}") }
                println!("P17  ledges {} in {:?}", out.len() - 1, b);
                (0, 0)
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
            let stick = if a[0] == "s" { (hex(a[1]) as u8, hex(a[2]) as u8) } else { (128, 128) };
            if let Some(r) = frame_stick(e, rt, b, stick) {
                got = Some((norm(&r.level), f));
                break 'outer;
            }
        }
    }
    // P32 R8 P17_MONLOG=1: at the route end (level exit or last step) list every object in the radar / Tome groups
    // (0x400 radar, 0x200 Tome scan) with module, state, enabled and position (monsters left over after captures)
    if std::env::var_os("P17_MONLOG").is_some() {
        let ids: Vec<(u32, Option<usize>)> = e.rt.objects.iter().flatten().map(|o| (o.id, o.inst.code)).collect();
        for (u, k) in ids {
            rt.particles.lock().unwrap().logic = true;
            let g = APIgameobj::GetGroupBitMask(e, u);
            rt.particles.lock().unwrap().logic = false;
            if g & 0x600 == 0 { continue }
            let dis = rt.particles.lock().unwrap().s1.disabled.contains(&u);
            let m = k.map(|k| e.rt.codes[k].def.name.to_string());
            println!("P17 MON f{f} {u:#x} {m:?} grp {g:#x} state {} disabled {dis} at {:?}", scooby_recomp::engine::core::apigameobj::GetCurrentStateID(e, u), pos(e, rt, u));
        }
    }
    got
}

/// WP-P21 R39: asserts every episode chain reaches its last milestone. KNOWN_BROKEN lists episodes whose
/// chain is known to stop at a given level (asserted, so a fix or a different break shows up too).
/// R65: GAME_SAVEDIR is process-global: the tests that set it (and save to the card) run one at a time
pub(crate) static CARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// P20 R130: a level's steps for the current mode: with ANIMSND on (the default) the `*_ANIMSND` variants
/// (routes_ep2 EP2_*_ANIMSND / EP2_L01A_REAL, routes_animsnd::variant) replace the EPn_ROUTE default steps.
pub(crate) fn route_steps(lvl: &str, steps: &str) -> String {
    // P27 R8 debug: P17_ROUTEFILE=path with `LEVEL=steps` lines overrides a level's route (retune without a rebuild)
    if let Some(t) = std::env::var("P17_ROUTEFILE").ok().and_then(|p| std::fs::read_to_string(p).ok()) {
        if let Some(r) = t.lines().find_map(|l| l.strip_prefix(lvl).and_then(|r| r.strip_prefix('='))) {
            return r.trim().to_string();
        }
    }
    if !crate::common::animsnd_on() {
        return steps.to_string();
    }
    match lvl {
        "EP2/L01A" => routes_ep2::EP2_L01A_REAL,
        "EP2/L01B" => routes_ep2::EP2_L01B_ANIMSND,
        "EP2/L01C" => routes_ep2::EP2_L01C_ANIMSND,
        "EP2/L04" => routes_ep2::EP2_L04_ANIMSND,
        l => routes_animsnd::variant(l).unwrap_or(steps),
    }
    .to_string()
}

#[test]
fn p17_chains() {
    const KNOWN_BROKEN: &[(&str, &str)] = &[];
    crate::common::install_crash_handler();
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let _card = CARD.lock().unwrap_or_else(|e| e.into_inner());
    // EP4/L01 saves at the checkpoint (real save dialog): start from a fresh, empty memory card
    // so the menu flow does not depend on earlier runs
    let card = std::env::temp_dir().join(format!("scooby_p17_card_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&card);
    // P20 R131 P17_CARD=<dir>: keep this card dir as is (earlier saves on the card)
    let card = std::env::var("P17_CARD").map(std::path::PathBuf::from).unwrap_or(card);
    // SAFETY: set once at test start; only memory-card init reads it
    unsafe { std::env::set_var("GAME_SAVEDIR", &card) };
    if let Ok(s) = std::env::var("P17_STEPS") {
        let (dir, steps) = s.split_once('|').expect("P17_STEPS=<level>|<steps>");
        let bufs = std::env::var("P17_CARRY").map(|p| load_dump(&p)).unwrap_or_default();
        // R125: `||` chains segments across level requests: each next segment runs in the requested level with the
        // carried buffers (as the chain does); a segment starting `@LEVEL;` loads LEVEL instead (e.g. `@INTRO` after a quit)
        let (mut e, mut rt) = load_carried(&fs, dir, &bufs).unwrap();
        let mut cur = dir.to_string();
        for (k, seg) in steps.split("||").enumerate() {
            let seg = seg.trim();
            let (seg, at) = match seg.strip_prefix('@') { Some(r) => { let (l, rest) = r.split_once(';').unwrap_or((r, "")); (rest.trim(), Some(l.trim().to_string())) } None => (seg, None) };
            if k > 0 || at.is_some() {
                // `@!LEVEL`: a fresh boot into LEVEL (no carried game buffers, e.g. the title menu Load)
                let b = if at.as_deref().is_some_and(|l| l.starts_with('!')) { vec![] } else { carry(&mut e) };
                let lvl = at.map(|l| l.trim_start_matches('!').to_string()).unwrap_or_else(|| cur.clone());
                println!("P17 adhoc load {lvl} (segment {k})");
                let (e2, rt2) = load_carried(&fs, &lvl, &b).unwrap();
                e = e2;
                rt = rt2;
                cur = lvl;
            }
            let r = run(&mut e, &mut rt, seg);
            println!("P17 adhoc {cur}: {r:?}");
            match r { Some((l, _)) => cur = l, None => if k + 1 < steps.split("||").count() && !steps.split("||").nth(k + 1).is_some_and(|n| n.trim().starts_with('@')) { break } }
        }
        return;
    }
    let ep = std::env::var("P17_EP").ok();
    let mut bad = vec![];
    for (n, route) in [("2", routes_ep2::EP2_ROUTE), ("3", routes_ep3::EP3_ROUTE), ("4", routes_ep4::EP4_ROUTE), ("5", routes_ep5::EP5_ROUTE), ("6", routes_ep6::EP6_ROUTE)] {
        if ep.as_deref().is_some_and(|e| e != n) || route.is_empty() { continue }
        let from = std::env::var("P17_FROM").ok();
        // R65: each episode starts from the real previous-episode end carry (p21_real_carry_episodes:
        // the real end menus; P21_SAVE regenerates them), not from an empty carry + a faked GB2+0 write
        let entry = match n { "2" => "ep1_end_EP2L01A", "3" => "ep2_end_EP3L01", "4" => "ep3_end_EP4L01", _ => "ep4_end_EP6L01" };
        let mut bufs = std::env::var("P17_CARRY").map(|p| load_dump(&p)).unwrap_or_else(|_| if from.is_none() { load_dump(&format!("{}/tests/all/data/{entry}.bin", env!("CARGO_MANIFEST_DIR"))) } else { vec![] });
        let mut started = from.is_none();
        let mut total = 0u32;
        println!("P17 === EP{n}");
        let mut stop: Option<&str> = None;
        for (lvl, steps, want) in route {
            started |= from.as_deref() == Some(*lvl);
            if !started { continue }
            // P17_OVR="<level>|<steps>": replace one level's steps in the chain (ad hoc retuning)
            let ovr = std::env::var("P17_OVR").ok();
            let steps = match ovr.as_deref().and_then(|s| s.split_once('|')) { Some((l, s)) if l == *lvl => s.to_string(), _ => steps.to_string() };
            // P20 R121 ANIMSND: real EP2/L01A (catapult key needs the anim sound table)
            // P22 R48: P17_OVRAS=1 keeps the ANIMSND default (route_steps) for an overridden level
            let animsnd = crate::common::animsnd_on() && (std::env::var_os("P17_OVRAS").is_some() || !ovr.as_deref().is_some_and(|o| o.starts_with(&format!("{lvl}|"))));
            let steps = if animsnd { route_steps(lvl, &steps) } else { steps };
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let (mut e, mut rt) = load_carried(&fs, lvl, &bufs).unwrap();
                let r = run(&mut e, &mut rt, &steps);
                (r, carry(&mut e))
            }));
            // R77 P17_SWEEP=1: try each fake step of this level alone (dropped, or replaced by a real walk)
            // from this level's chain entry carry; prints `SWEEP <lvl> #<i> <variant> PASS|FAIL`
            if std::env::var("P17_SWEEP").is_ok_and(|v| v == "1" || v.split(',').any(|x| x == *lvl)) && matches!(r, Ok((Some((ref l, _)), _)) if l.eq_ignore_ascii_case(want)) {
                let st: Vec<&str> = steps.split(';').map(str::trim).filter(|s| !s.is_empty()).collect();
                for (i, s) in st.iter().enumerate() {
                    let w: Vec<&str> = s.split_whitespace().collect();
                    let vars: Vec<String> = match w[0] {
                        "warp" => vec![String::new(), format!("gw {} 2000", w[1]), format!("gw {} 2000 X", w[1]), format!("gw {} 3000 T; x 120", w[1]), format!("gw {} 3000 TX; f 60", w[1]), format!("gw {} 5000 TXP; f 60", w[1])],
                        "wxyz" => vec![String::new(), format!("goto {} {} 1500", w[1], w[3]), format!("goto {} {} 2500 X", w[1], w[3]), format!("goto {} {} 2500 TX; f 60", w[1], w[3]), format!("goto {} {} 4000 TXP; f 60", w[1], w[3])],
                        "press" => vec![format!("gw {} 1500; x 65", w[1]), format!("gw {} 1500; p {} 14; x 65", w[1], w.get(3).unwrap_or(&"20"))],
                        "c" => vec![String::new(), format!("gw {} 600; f 30", w[1]), format!("gw {} 1500 P; f 30", w[1])],
                        // R83: book warps -> walk (`bookg`), same args
                        // R84: + hard chase with doubled time
                        "book" => vec![s.replacen("book", "bookg", 1), { let mut q = w.clone(); let n2 = (w[1].parse::<u32>().unwrap_or(0) * 2).to_string(); q[0] = "bookgr"; q[1] = &n2; q.join(" ") }],
                        "cw" | "g" | "gor" | "put" | "farm" => vec![String::new()],
                        _ => vec![],
                    };
                    for v in vars {
                        // R101 P17_SWEEPV=<substr>: only variants containing it (e.g. `P`)
                        if std::env::var("P17_SWEEPV").is_ok_and(|k| !v.contains(k.as_str())) { continue }
                        let mut t = st.clone();
                        t[i] = &v;
                        let t = t.join("; ");
                        let ok = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            let (mut e, mut rt) = load_carried(&fs, lvl, &bufs).unwrap();
                            run(&mut e, &mut rt, &t).map(|(l, f)| (l, f))
                        }));
                        let res = match ok { Ok(Some((l, f))) if l.eq_ignore_ascii_case(want) => format!("PASS f{f}"), Ok(Some((l, _))) => format!("FAIL wrong {l}"), Ok(None) => "FAIL none".into(), Err(_) => "FAIL panic".into() };
                        println!("SWEEP {lvl} #{i} `{s}` -> `{v}` {res}");
                    }
                }
            }
            match r {
                Ok((Some((l, f)), b)) if l.eq_ignore_ascii_case(want) => {
                    total += f;
                    println!("P17 {lvl}: -> {l} at f{f} (chain f{total})");
                    bufs = b;
                    // P17_SAVE=<dir>: entry carry of the next level as a P17_CARRY dump (<dir>/EP3_L04.bin)
                    if let Ok(d) = std::env::var("P17_SAVE") {
                        let mut o = vec![];
                        for (id, v) in &bufs {
                            o.extend(id.to_le_bytes());
                            o.extend((v.len() as u32).to_le_bytes());
                            o.extend(v);
                        }
                        // (a repeated visit gets a `_2` suffix instead of overwriting the first entry carry)
                        let mut path = format!("{d}/{}.bin", want.replace('/', "_"));
                        if std::path::Path::new(&path).exists() { path = path.replace(".bin", "_2.bin") }
                        std::fs::write(path, o).unwrap();
                    }
                    // P38 R1 P17_ONE=1: stop after this level (one level per headless slot hold)
                    if std::env::var_os("P17_ONE").is_some() { break }
                }
                Ok((Some((l, f)), _)) => { println!("P17 {lvl}: WRONG -> {l} at f{f} (want {want})"); stop = Some(lvl); break }
                Ok((None, _)) => { println!("P17 {lvl}: not fired (want {want})"); stop = Some(lvl); break }
                Err(_) => { println!("P17 {lvl}: PANIC"); stop = Some(lvl); break }
            }
        }
        if from.is_none() && std::env::var_os("P17_CARRY").is_none() {
            let known = KNOWN_BROKEN.iter().find(|(k, _)| *k == n).map(|(_, l)| *l);
            if stop != known {
                bad.push(format!("EP{n}: stopped at {stop:?}, expected {known:?}"));
            }
        }
    }
    assert!(bad.is_empty(), "P17 chain regressions: {bad:?}");
}

/// WP-P21 R106: EP3/L01 is completable by real play from the real EP2-end carry (routes_ep3::EP3_L01_REAL, no warp)
#[test]
fn p21_ep3_l01_real() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let bufs = load_dump(&format!("{}/tests/all/data/ep2_end_EP3L01.bin", env!("CARGO_MANIFEST_DIR")));
    let (mut e, mut rt) = load_carried(&fs, "EP3/L01", &bufs).unwrap();
    let r = run(&mut e, &mut rt, &route_steps("EP3/L01", routes_ep3::EP3_L01_REAL));
    assert!(matches!(&r, Some((l, _)) if l.eq_ignore_ascii_case("EP3/L02")), "EP3/L01 real: {r:?}");
}

/// WP-P21 R119: walking onto the EP3/L04 vault-counter snacks 0x42..0x44 right after the a7 safe reward fires the panel
/// collision notify with its window index still at the 0x7ffffffe sentinel; the C pointer math wraps (panel.rs
/// cn_windows), the port used to panic with "attempt to add with overflow". Carry = real L03 cart ride end.
#[test]
fn p21_ep3_l04_vault_snacks_panel() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let bufs = load_dump(&format!("{}/tests/all/data/ep3_l04_realcarry.bin", env!("CARGO_MANIFEST_DIR")));
    let (mut e, mut rt) = load_carried(&fs, "EP3/L04", &bufs).unwrap();
    let l04 = routes_ep3::EP3_ROUTE.iter().find(|r| r.0 == "EP3/L04").unwrap().1;
    let cut = "gw 2a0000a7 400; f 60; ";
    let pre = &l04[..l04.find(cut).unwrap() + cut.len()];
    let r = run(&mut e, &mut rt, &format!("{pre}gw 2a000042 200; gw 2a000043 200; gw 2a000044 200; f 200"));
    assert!(r.is_none(), "{r:?}");
    let gb2 = e.rt.game_buffers.get(&2).map(|b| b.0).unwrap();
    assert_eq!(e.mem.r32(gb2) & 0x20, 0x20, "safe reward not taken");
}

/// WP-P21 R44: EP5 has no levels (the 5th game episode is EP6 ShermanTech), so routes_ep5 is empty on purpose.
/// Fails if an EP5 level ever shows up on the ISO or EP5_ROUTE gets entries without a chain expectation.
#[test]
fn p17_no_ep5() {
    assert!(routes_ep5::EP5_ROUTE.is_empty());
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let up: Vec<String> = fs.names().map(|n| n.to_ascii_uppercase().replace('\\', "/")).collect();
    assert!(up.iter().any(|n| n.contains("LEVEL/EP6/")), "ISO level layout changed");
    assert!(!up.iter().any(|n| n.contains("LEVEL/EP5")), "EP5 level on the ISO: write routes_ep5");
}

/// WP-P17 fix: ghost_lock TmpGhostLockMap {id, GB2+0xc bit} = the carved .o's first .text words. EP3/L04
/// ghost lock 0x2a00013d guards the L05 door 0x2a000105 (hs_door lock GB2+0xc 0x4000): local+8 must be 0x4000.
#[test]
fn p17_ghost_lock_bit() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = load_carried(&fs, "EP3/L04", &[]).unwrap();
    for _ in 0..2 {
        frame(&mut e, &mut rt, 0);
    }
    let lo = e.rt.objects.iter().flatten().find(|o| o.id == 0x2a00_013d).map(|o| o.inst.local).expect("ghost lock");
    assert_eq!(e.mem.r32(lo + 4), 0x2a00_013d);
    assert_eq!(e.mem.r32(lo + 8), 0x4000);
}

/// WP-P19 fix: EFGameMemoryMgr::Terminate (EFEngineController::Reset at every LoadLevel) deletes the
/// non-persistent game buffers. ApiSectorClipping's client lists (GetGameBuffer(40000, count*4, 0)) must not
/// reach the next level: EP4/L01 has 4 sectors, EP4/L02A 0x21 -> the stale 16-byte buffer overflowed and
/// L02A's first sector fade faulted in api_list (SectorListInFadeCallback). L01 -> L02A carried must run.
#[test]
fn p19_nonpersistent_buffers_die_at_level_switch() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = load_carried(&fs, "EP4/L01", &[]).unwrap();
    for _ in 0..60 {
        frame(&mut e, &mut rt, 0);
    }
    assert!(e.rt.game_buffers.contains_key(&40000));
    let bufs = carry(&mut e);
    assert!(!bufs.iter().any(|b| b.0 == 40000), "sector clipping client lists carried");
    assert!(bufs.iter().any(|b| b.0 == 0) && bufs.iter().any(|b| b.0 == 2), "persistent GB0/GB2 lost");
    let (mut e, mut rt) = load_carried(&fs, "EP4/L02A", &bufs).unwrap();
    for _ in 0..300 {
        frame(&mut e, &mut rt, 0);
    }
    let l = e.rt.game_buffers[&40000].0;
    assert_eq!(e.mem.r32(l - 0x10), 0x21 * 4);
}

/// WP-P21 R20 fix: heroInit_Action case 2/3 (MSG_PULL_LEVER) anim ids vary per build (EP4/L01 Scooby 7e698dff:
/// 0x13/0x15). A wrong id made StartAnim fail, the one-shot never ended and GB0+0x14 bit 0x100 stayed set,
/// so the EP4/L01 rotating pipes ignored the hero. After a lever pull the bit must clear again.
#[test]
fn p21_pull_lever_action_ends() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = load_carried(&fs, "EP4/L01", &[]).unwrap();
    for _ in 0..60 {
        frame(&mut e, &mut rt, 0);
    }
    let hero = leader(&e);
    rt.particles.lock().unwrap().logic = true;
    APIgameobj::SendMessageTo(&mut e, hero, 0, 0x9a, 1, 0x2a00_010b);
    rt.particles.lock().unwrap().logic = false;
    let gb0 = e.rt.game_buffers[&0].0;
    let mut seen = false;
    for _ in 0..300 {
        frame(&mut e, &mut rt, 0);
        seen |= e.mem.r32(gb0 + 0x14) & 0x100 != 0;
    }
    let lo = e.rt.objects.iter().flatten().find(|o| o.id == hero).map(|o| o.inst.local).unwrap();
    assert_eq!(e.mem.r32(lo + 0x240), 2, "hero did not take the lever action");
    assert!(seen, "action bit never set");
    assert_eq!(e.mem.r32(gb0 + 0x14) & 0x100, 0, "pull-lever action never ended (anim {:#x})", e.mem.r32(lo + 0x218));
}

/// WP-P20 R12: EP1 -> EP2 across the episode boundary. tests/all/data/ep1_end_EP2L01A.bin = the carry game.rs
/// dumped (GAME_CARRY_DUMP) when EP2\L01A loaded at the end of the real chained EP1 run (ep1_chain: EP1/L01 ..
/// L04 -> LOADER -> STATUS, real STATUS cross presses -> SPLASH -> LOADER EP02E/EP02D -> EP2/L01A). The EP2
/// route runs from it without the faked episode-start write `g 2 0 7` and must reach STATUS (EP2 end).
#[test]
fn ep12_real_carry_ep2() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let mut bufs = load_dump(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/all/data/ep1_end_EP2L01A.bin"));
    let mut last = String::new();
    for (lvl, steps, want) in routes_ep2::EP2_ROUTE {
        let (mut e, mut rt) = load_carried(&fs, lvl, &bufs).unwrap();
        let r = run(&mut e, &mut rt, &route_steps(lvl, steps).replace("g 2 0 7;", ""));
        let (l, _) = r.unwrap_or_else(|| panic!("{lvl}: no level request (want {want})"));
        assert!(l.eq_ignore_ascii_case(want), "{lvl}: -> {l} (want {want})");
        bufs = carry(&mut e);
        last = l;
    }
    assert!(last.eq_ignore_ascii_case("STATUS"), "EP2 ends in {last}");
}

/// WP-P21 R60: EP2 -> EP3 -> EP4 -> EP6 with the real previous-episode end carry instead of the faked
/// episode-start writes (`g 2 0 1f` / `g 2 0 3f` / `g 2 0 7f`), starting from the real EP1-end carry.
/// Prints GB2+0 at each episode start (the value the fake wrote).
/// R64: the end menus run for real (STATUS: the savepanel save-status menu needs DOWN then X, as a player
/// picks the second entry; X alone toggles menu <-> games list). EP2 and EP3 run without their faked
/// episode-start write; an episode whose real carry fails is rerun with the fake (logged).
/// R65: green EP2 -> EP6, all four episode-start writes deleted from the routes (~30 s, in the gate).
#[test]
fn p21_real_carry_episodes() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let _card = CARD.lock().unwrap_or_else(|e| e.into_inner());
    // R65: a fresh memory card as p17_chains (the default card dir persists across runs: the EP4/L01
    // checkpoint save dialog then differs and the route stalls)
    let card = std::env::temp_dir().join(format!("scooby_p21_card_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&card);
    // SAFETY: set at test start; only memory-card init reads it
    unsafe { std::env::set_var("GAME_SAVEDIR", &card) };
    let mut bufs = load_dump(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/all/data/ep1_end_EP2L01A.bin"));
    let mut real = vec![];
    // P35 R15 P21_FROM=<3|4|6> + P21_FROM_DIR=<a P21_SAVE dir>: start at that episode from its saved entry
    // carry (EP<n>_start.bin) and card (card_EP<n>)
    let from = std::env::var("P21_FROM").unwrap_or_default();
    let mut skip = !from.is_empty();
    for (n, route) in [("2", routes_ep2::EP2_ROUTE), ("3", routes_ep3::EP3_ROUTE), ("4", routes_ep4::EP4_ROUTE), ("6", routes_ep6::EP6_ROUTE)] {
        if skip {
            if n != from { continue }
            skip = false;
            let d = std::path::PathBuf::from(std::env::var("P21_FROM_DIR").expect("P21_FROM needs P21_FROM_DIR"));
            bufs = load_dump(d.join(format!("EP{n}_start.bin")).to_str().unwrap());
            fn cpd(a: &std::path::Path, b: &std::path::Path) {
                let _ = std::fs::create_dir_all(b);
                for f in std::fs::read_dir(a).into_iter().flatten().flatten() {
                    if f.path().is_dir() { cpd(&f.path(), &b.join(f.file_name())) } else { let _ = std::fs::copy(f.path(), b.join(f.file_name())); }
                }
            }
            cpd(&d.join(format!("card_EP{n}")), &card);
        }
        let gb2 = bufs.iter().find(|b| b.0 == 2).map_or(0, |b| u32::from_le_bytes(b.1[0..4].try_into().unwrap()));
        println!("P21 EP{n} start GB2+0 {gb2:#x}");
        // P20 R131 P21_SAVE: also the card as it is at the episode start (<dir>/card_EP4, for P17_CARD)
        if let Ok(d) = std::env::var("P21_SAVE") {
            fn cp(a: &std::path::Path, b: &std::path::Path) {
                let _ = std::fs::create_dir_all(b);
                for f in std::fs::read_dir(a).into_iter().flatten().flatten() {
                    if f.path().is_dir() { cp(&f.path(), &b.join(f.file_name())) } else { let _ = std::fs::copy(f.path(), b.join(f.file_name())); }
                }
            }
            cp(&card, &std::path::Path::new(&d).join(format!("card_EP{n}")));
        }
        // P21_SAVE=<dir>: the real episode-entry carry as a P17_CARRY dump (<dir>/EP4_start.bin)
        if let Ok(d) = std::env::var("P21_SAVE") {
            let o: Vec<u8> = bufs.iter().flat_map(|(id, v)| [id.to_le_bytes().to_vec(), (v.len() as u32).to_le_bytes().to_vec(), v.clone()].concat()).collect();
            std::fs::write(format!("{d}/EP{n}_start.bin"), o).unwrap();
        }
        let play = |bufs: &Vec<(u32, Vec<u8>)>| -> Result<(Vec<(u32, Vec<u8>)>, String), String> {
            let (mut bufs, mut last) = (bufs.clone(), String::new());
            for (lvl, steps, want) in route {
                // P35 R15 P21_NOCLIP=EP3/L03[,..]: these levels run clip off under GAME_LOGICCLIP=1 (the scripted
                // EP3/L03 cart ride is out of phase clip on), so the later episodes still run clip on
                let clip0 = std::env::var("GAME_LOGICCLIP").ok();
                let noclip = std::env::var("P21_NOCLIP").is_ok_and(|v| v.split(',').any(|x| x.trim().eq_ignore_ascii_case(lvl)));
                // SAFETY: test-only toggle; the engine reads it on this thread only
                if noclip { unsafe { std::env::set_var("GAME_LOGICCLIP", "0") } }
                let (mut e, mut rt) = load_carried(&fs, lvl, &bufs).unwrap();
                let mut steps = route_steps(lvl, steps);
                // R65: the EP4/L01 checkpoint save dialog on a card that already holds the STATUS saves asks
                // one more X (overwrite) than on the fresh card of p17_chains
                // (P22: the checkpoint is now reached with a real `gw 2a00002c 1500`)
                // P20 R131: EP4_L01_ANIMSND ends the save dialog with its own `s 80 ff 10` + X (Yes on the overwrite prompt)
                if *lvl == "EP4/L01" && !crate::common::animsnd_on() { steps = steps.replacen("2a00002c 1500; f 90; p 4000 3; f 400;", "2a00002c 1500; f 90; p 4000 3; f 400; p 4000 3; f 400;", 1).replacen("warp 2a00002c; f 90; p 4000 3; f 400;", "warp 2a00002c; f 90; p 4000 3; f 400; p 4000 3; f 400;", 1) }
                let r = run(&mut e, &mut rt, &steps);
                // SAFETY: as above
                if noclip { if let Some(v) = &clip0 { unsafe { std::env::set_var("GAME_LOGICCLIP", v) } } }
                let Some((l, _)) = r else { return Err(format!("{lvl}: no level request (want {want})")) };
                if !l.eq_ignore_ascii_case(want) { return Err(format!("{lvl}: -> {l} (want {want})")) }
                bufs = carry(&mut e);
                last = l;
            }
            Ok((bufs, last))
        };
        let (b, mut last) = match play(&bufs) {
            Ok(r) => { real.push(n); r }
            Err(m) => { println!("P21 EP{n} real carry FAILED ({m}): stop"); break }
        };
        bufs = b;
        println!("P21 EP{n} -> {last}");
        if n == "6" { break } // the last episode: its end menus (credits) are not chained
        // the real end menus (STATUS/SPLASH/LOADER, X presses as game.rs menu_cross) into the next episode
        for _ in 0..8 {
            if last.to_ascii_uppercase().starts_with("EP") { break }
            let (mut e, mut rt) = load_carried(&fs, &last, &bufs).unwrap();
            let (l, f) = run(&mut e, &mut rt, &"f 282; s 80 ff 6; f 6; p 4000 6; ".repeat(30)).unwrap_or_else(|| panic!("{last}: no level request"));
            println!("P21   {last} -> {l} at f{f}");
            bufs = carry(&mut e);
            last = l;
        }
        // P32 R80: the real end menus must hand over to the next episode's first level (the next route
        // loads its own first level, so a wrong episode handover was not caught) and the STATUS save
        // must have written the card
        let next = match n { "2" => "EP3/L01", "3" => "EP4/L01", _ => "EP6/L01" };
        assert!(last.eq_ignore_ascii_case(next), "EP{n} end menus -> {last} (want {next})");
        assert!(card.join("BASLUS-20701/BASLUS-20701").is_file(), "EP{n} end: no save on the card");
        // P32 R81: the episode unlock the TITLE reads (panel_baseFUNC_bAmILocked_Episode: GB2+0x28 bits
        // 1/0x20/0x400/0x4000/0x100000 = Episodes/Movies items 1..5) is set by the SPLASH controller
        // constructor (Splash Controller.c: ++GB0+0x18, then |= the bit of that episode)
        let ul = bufs.iter().find(|b| b.0 == 2).map_or(0, |b| u32::from_le_bytes(b.1[0x28..0x2c].try_into().unwrap()));
        let bit = match n { "2" => 0x400, "3" => 0x4000, _ => 0x100000 };
        assert!(ul & bit != 0, "EP{n} end: next episode not unlocked (GB2+0x28 {ul:#x}, want bit {bit:#x})");
    }
    println!("P21 real-carry episodes (no episode-start write): {real:?}");
    let want: Vec<&str> = ["2", "3", "4", "6"].into_iter().skip_while(|n| !from.is_empty() && *n != from).collect();
    assert_eq!(real, want, "real carry regressed");
}

/// P21 R24: Pipes_MansionST_BASICMOVE_LOOP decrements its start timer by GetFrameDeltaTime (Ghidra
/// dropped the f1 return; the port subtracted the timer from itself and the pipe never turned).
#[test]
fn p21_pipes_mansion_turns_on_de() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = load_carried(&fs, "EP4/L01", &[]).unwrap();
    for _ in 0..60 {
        frame(&mut e, &mut rt, 0);
    }
    rt.particles.lock().unwrap().logic = true;
    APIgameobj::SendMessageTo(&mut e, 0x2a00_011c, 0, 0xde, 0x2a00_011f, 0);
    rt.particles.lock().unwrap().logic = false;
    for _ in 0..300 {
        frame(&mut e, &mut rt, 0);
    }
    let lo = e.rt.objects.iter().flatten().find(|o| o.id == 0x2a00_011c).map(|o| o.inst.local).unwrap();
    assert_eq!(e.mem.r32(lo + 8), 2, "mansion pipe 0x11c did not turn (angle {})", f32::from_bits(e.mem.r32(lo + 4)));
}

/// WP-P21 R22 regression: WeightPuzzle float locals (weight local+0x24, step +0x28) must be read/written as f32 bits.
/// The port converted int<->float (`as f32` / `as i32`), so the first X clamped the weight to 20 and the
/// 0x172/0x173 combinations (13/15) were unreachable.
#[test]
fn p21_weightpuzzle_adds_float_weight() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = load_carried(&fs, "EP4/L01", &[]).unwrap();
    run(&mut e, &mut rt, "f 60; warp 2a000172; f 60; p 4000 3; f 150; p 4000 1; f 45; p 4000 1; f 45");
    let lo = e.rt.objects.iter().flatten().find(|o| o.id == 0x2a00_016c).map(|o| o.inst.local).unwrap();
    assert_eq!(f32::from_bits(e.mem.r32(lo + 0x24)), 4.0, "weight after two +2 presses");
}

/// WP-P21 R26 regression: HumidFloor's fade alpha (local[0]) is a float. 0xdf -> FADE_In stores 1.0,
/// FADE_Execute subtracts 0.005 per frame and returns to state 1 at <= 0. The port converted the
/// bits 0x3f800000 numerically (~1.065e9), so the fade never ended.
#[test]
fn p21_humidfloor_fade_ends() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = load_carried(&fs, "EP4/L01", &[]).unwrap();
    for _ in 0..30 {
        frame(&mut e, &mut rt, 0);
    }
    APIgameobj::SendMessageTo(&mut e, 0x2a00_0111, 0, 0xdf, 0x2a00_0111, 0);
    for _ in 0..20 {
        frame(&mut e, &mut rt, 0);
    }
    let st = |e: &Eng| e.rt.objects.iter().flatten().find(|o| o.id == 0x2a00_0111).map(|o| (o.inst.state, o.inst.local)).unwrap();
    let (s, lo) = st(&e);
    let a = f32::from_bits(e.mem.r32(lo));
    assert!(s == 3 && a > 0.0 && a < 1.0, "fading: state {s} alpha {a}");
    for _ in 0..400 {
        frame(&mut e, &mut rt, 0);
    }
    let (s, lo) = st(&e);
    assert_eq!(s, 1, "fade did not end (alpha {})", f32::from_bits(e.mem.r32(lo)));
}

/// WP-P21 R26 regression: PowerUp's bob offset local[7] = local[9] + sin * 0.07 is a float; the port
/// stored it via `as i64 as u32`, so it was always 0 (no bobbing).
#[test]
fn p21_powerup_bobs() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (mut e, mut rt) = load_carried(&fs, "EP3/L01", &[]).unwrap();
    let mut seen = Vec::new();
    for f in 0..120 {
        frame(&mut e, &mut rt, 0);
        if f % 30 == 29 {
            let lo = e.rt.objects.iter().flatten().find(|o| o.id == 0x2a00_0114).map(|o| o.inst.local).unwrap();
            let r = |i: u32| f32::from_bits(e.mem.r32(lo + i * 4));
            // R37: FIRSTMOVE/RunBob use int* arithmetic (local+0x12 / +6 words); the port passed byte offsets
            // 18 / 6, so the spawn position was never stored and SetTranslation moved the powerup to junk.
            assert!(r(0x12).abs() > 1.0 && r(0x14).abs() > 1.0, "spawn pos {:?}", (r(0x12), r(0x13), r(0x14)));
            assert!(r(6) == r(0x12) && r(8) == r(0x14), "pos {:?} spawn {:?}", (r(6), r(8)), (r(0x12), r(0x14)));
            seen.push(r(7) - r(0x13));
        }
    }
    assert!(seen.iter().all(|v| v.abs() <= 0.0701) && seen.iter().any(|v| v.abs() > 0.01), "bob offsets {seen:?}");
}

/// WP-P21 R30/R31 regression: STREAM banks (VAG voice lines) had no sample length, so their voices never
/// ended (ObjIsSoundPlaying stuck true; EP3/L01 inv_racecontroller waits for 0x8000000c to end).
/// LoadLevel now derives the length from the VAG header: every VAG stream sample needs a finite length.
#[test]
fn p21_stream_voice_has_length() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let (_e, rt) = load_carried(&fs, "EP3/L01", &[]).unwrap();
    let g = rt.particles.lock().unwrap();
    let mut n = 0;
    for (b, bank) in g.sub.audio.banks.iter().enumerate() {
        let Some(bank) = bank else { continue };
        if !bank.sbh.is_stream() { continue }
        for i in 0..bank.sbh.samples.len() {
            let l = g.sub.sample_len.get(&(b as u32, i as u32)).copied();
            if let Some(l) = l {
                assert!(l > 0.0 && l < 120.0, "stream sample {b}/{i} length {l}");
                n += 1;
            } else {
                // .INT = interleaved music / cutscene streams (not VAG voices), no length by design
                assert!(bank.sbh.samples[i].name.ends_with(".INT"), "stream sample {b}/{i} has no length");
            }
        }
    }
    assert!(n > 0, "no stream samples in EP3/L01");
}

/// P21 R69/R70: the headless harness publishes bone frames (common::publish_nodes, as bin/game.rs
/// draw_objects), so bone-attached coll prims follow their anim: EP6/L01 automaticdoor 0x44's panel prim
/// (joint 3) opens and the hero walks through to the partner door 0x51 (~1110). Frozen prims stopped
/// him at 222.27 in front of the shut panel.
#[test]
fn p21_bone_prims_follow_anim_door() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let bufs = load_dump(&format!("{}/tests/all/data/ep4_end_EP6L01.bin", env!("CARGO_MANIFEST_DIR")));
    let (mut e, mut rt) = load_carried(&fs, "EP6/L01", &bufs).unwrap();
    run(&mut e, &mut rt, "f 60; goto 223.78 202.33 400");
    let l = leader(&e);
    let p = pos(&mut e, &mut rt, l).unwrap();
    assert!(p[0] > 1000.0, "hero did not pass door 0x44: {p:?}");
}

/// P22 R29: player-facing save/load round trip, one camera checkpoint per episode: on a fresh card, stand on the
/// checkpoint, real save dialog (`save`), pause -> quit, title menu Load (`menuload`). The load must land in the saved
/// level with the card file written, and courage (GB1+0), book power, keys/items (GB1) and the GB2 progress bits as saved.
/// P22_SAVELOAD=EPn/Lxx restricts to one case; Prints every differing word of GB0/1/2 (GB0 = level-init/transient, not asserted).
#[test]
fn p22_save_load_roundtrip() {
    // (level, entry carry under tests/all/data, approach): steps that put the hero on a checkpoint; `route:STEP` =
    // the level's chain route up to and including STEP (EP4/L01 2c: a guard near the camera scares a hero warped
    // there early, his POST_COLLISION closes the prompt; the route reaches it after the guards)
    const CASES: &[(&str, &str, &str)] = &[
        // EP1/L01: the first camera 0x9b (X dismisses the start text, then its camera tutorial sets GB0+0x40 bit
        // 0x20000; before that ChkPntCB_NOTIFY_COLLIDE_OBJ sends 0xa3 (tutorial) instead of the save prompt)
        ("EP1/L01", "", "f 200; p 4000 3; f 200; warp 2a00009b; f 200"),
        ("EP2/L01A", "ep1_end_EP2L01A", "f 200; warp @chkpnt"),
        ("EP3/L01", "ep2_end_EP3L01", "f 200; warp @chkpnt"),
        ("EP4/L01", "ep3_end_EP4L01", "route:gw 2a00002c 1500"),
        ("EP6/L02", "ep4_end_EP6L01", "f 200; warp @chkpnt"),
    ];
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let _card = CARD.lock().unwrap_or_else(|e| e.into_inner());
    let only = std::env::var("P22_SAVELOAD").ok();
    let tmp = std::env::temp_dir().join(format!("scooby_p22_sl_{}", std::process::id()));
    let mut bad = vec![];
    for &(lvl, entry, how) in CASES {
        if only.as_deref().is_some_and(|o| o != lvl) { continue }
        let card = tmp.join(lvl.replace('/', "_"));
        let _ = std::fs::remove_dir_all(&card);
        // SAFETY: tests touching GAME_SAVEDIR hold CARD; only memory-card init reads it
        unsafe { std::env::set_var("GAME_SAVEDIR", &card) };
        let bufs = if entry.is_empty() { vec![] } else { load_dump(&format!("{}/tests/all/data/{entry}.bin", env!("CARGO_MANIFEST_DIR"))) };
        let (a, b) = (card.join("a.bin"), card.join("b.bin"));
        let (mut e, mut rt) = load_carried(&fs, lvl, &bufs).unwrap();
        let pre = match how.strip_prefix("route:") {
            Some(m) => {
                let r = route_steps(lvl, routes_ep4::EP4_ROUTE.iter().find(|x| x.0 == lvl).expect("route").1);
                // R88: the first step that IS `m` (not a clip-only `iflc m` step, routes_ep4 EP4/L01 since R87)
                let at = r.match_indices(m).map(|(i, _)| i).find(|&i| i == 0 || r[..i].trim_end().ends_with(';')).expect("route step");
                format!("{}{m}", &r[..at])
            }
            None => how.to_string(),
        };
        let r = run(&mut e, &mut rt, &format!("{pre}; f 60; save; gbdump {}; f 470; pausequit", a.display()));
        let file = card.join("BASLUS-20701/BASLUS-20701");
        println!("P22SL {lvl}: save+quit -> {r:?}, card file {}", file.is_file());
        if !matches!(&r, Some((l, _)) if l == "INTRO") || !file.is_file() { bad.push(format!("{lvl}: save/quit {r:?} file {}", file.is_file())); continue }
        let (mut e, mut rt) = load_carried(&fs, "INTRO", &[]).unwrap();
        let r = run(&mut e, &mut rt, "menuload");
        println!("P22SL {lvl}: menuload -> {r:?}");
        if !matches!(&r, Some((l, _)) if l.eq_ignore_ascii_case(lvl)) { bad.push(format!("{lvl}: load -> {r:?}")); continue }
        let c = carry(&mut e);
        let (mut e, mut rt) = load_carried(&fs, lvl, &c).unwrap();
        let r = run(&mut e, &mut rt, &format!("f 30; gbdump {}", b.display()));
        assert!(r.is_none(), "{lvl}: loaded level left at once {r:?}");
        let (ga, gb) = (load_dump(&a.to_string_lossy()), load_dump(&b.to_string_lossy()));
        for id in [0u32, 1, 2] {
            let (Some(x), Some(y)) = (ga.iter().find(|g| g.0 == id), gb.iter().find(|g| g.0 == id)) else { bad.push(format!("{lvl}: GB{id} missing")); continue };
            // GB2+0 bit 0x20000 = save prompt on screen (panel_base msg 0x31 sets it, panel_baseFUNC_Init clears it): UI state
            let mut x = x.clone();
            if id == 2 { x.1[2] &= !2 }
            let d: Vec<String> = (0..x.1.len().min(y.1.len()) / 4).filter(|&i| x.1[4 * i..4 * i + 4] != y.1[4 * i..4 * i + 4])
                .map(|i| format!("+{:#x} {:08x}->{:08x}", 4 * i, u32::from_le_bytes(x.1[4 * i..4 * i + 4].try_into().unwrap()), u32::from_le_bytes(y.1[4 * i..4 * i + 4].try_into().unwrap()))).collect();
            println!("P22SL {lvl}: GB{id} {} words differ {}", d.len(), d.join(" "));
            if id > 0 && !d.is_empty() { bad.push(format!("{lvl}: GB{id} {}", d.join(" "))) }
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
    assert!(bad.is_empty(), "save/load round trip: {bad:?}");
}

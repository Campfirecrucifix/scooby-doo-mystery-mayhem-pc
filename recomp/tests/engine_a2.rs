//! WP-A2: book-mechanics audio audit. EP1L01 capture (engine_j4 set-up: book handed over, power 180,
//! ghost 3 m in front of Scooby, SQUARE at f120, correct buttons mashed) and the no-press fail path
//! (engine_j22). Logs every APInewSound::Play request and every voice the manager starts/stops
//! (event, bank, playback, sample, rate, L/R, pan, pitch, loop, channel), and renders the mixed
//! output offline (48 kHz stereo, voices' PCM through ADSR, pitch, per-frame L/R) to
//! target WAV files `A2_WAV_DIR` (default: none) for inspection; prints per-second RMS/peak.
use scooby_recomp::engine::audio::adsr::apply_adsr;
use scooby_recomp::engine::audio::mgr::Voice;
use scooby_recomp::engine::core::apigameobj::GetCurrentStateID;
use scooby_recomp::engine::input::{pad, PadState};
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::runtime::*;
use std::collections::HashMap;

const ISO: &str = "../Scooby Doo Mystery Mayhem.ISO";
const BOOK: u32 = 0x2a00_0093;
const GHOST: u32 = 0x2a00_0008;
const SCOOBY: u32 = 0x2a00_0092;
const OUT_RATE: f32 = 48000.0;

type Key = (u32, u32, u32, u32, u32, Option<u8>);
fn key(v: &Voice, _f: u32) -> Key {
    (v.handle, v.source_event, v.playback, v.sample, v.delay.to_bits(), v.channel)
}

struct Run {
    plays: Vec<(u32, u32, u32, u32)>,
    starts: Vec<String>,
    stops: Vec<String>,
    mix: Vec<[f32; 2]>,
    book: Vec<(u32, u32)>,
    ghost: Vec<(u32, u32)>,
    max_voices_per_handle: usize,
}

fn run(fs: &IsoFs, mode: &str, frames: u32) -> Run {
    let (mut e, mut rt) = EngineRuntime::new_with_engine();
    let res = EngineRuntime::ReadLevel(fs, "EP1/L01", "EP1L01.P2R").unwrap();
    rt.LoadLevel(&mut e, res);
    let mut r = Run { plays: vec![], starts: vec![], stops: vec![], mix: vec![], book: vec![], ghost: vec![], max_voices_per_handle: 0 };
    let mut live: HashMap<Key, Voice> = HashMap::new();
    let mut env_cache: HashMap<(u32, u32, u32, u32), Vec<i16>> = HashMap::new();
    let dt = 1.0f32 / 60.0;
    let per = (OUT_RATE * dt) as usize;
    for f in 0..frames {
        if f == 10 {
            rt.particles.lock().unwrap().logic = Some(&e.rt as *const _);
            e.SendLogicMessageTo(BOOK, 0xe5, 0, 0, 0);
            let gb1 = e.rt.game_buffers[&1].0;
            e.mem.w32(gb1 + 4, 180f32.to_bits());
            rt.particles.lock().unwrap().logic = None;
        }
        if f == 100 {
            let mut g = rt.particles.lock().unwrap();
            let ph = g.sub.physic.PhysicOf(SCOOBY);
            let hp = g.sub.physic.Physic(ph).unwrap().frame;
            let gp = g.sub.physic.PhysicOf(GHOST);
            let t = [hp.pos[0] + hp.at[0] * 3.0, hp.pos[1] + 1.0, hp.pos[2] + hp.at[2] * 3.0];
            if let Some(p) = g.sub.physic.PhysicMut(gp) {
                p.frame.pos = t;
            }
            if let Some(fr) = g.world.frames.get_mut(&GHOST) {
                fr.pos = t;
            }
        }
        let l = e.rt.map.find(BOOK).map_or(0, |h| e.rt.objects[h].as_ref().unwrap().inst.local);
        {
            let mut g = rt.particles.lock().unwrap();
            let mut p = PadState { connected: true, mode: 7, rx: 128, ry: 128, lx: 128, ly: 128, ..Default::default() };
            if (120..130).contains(&f) {
                p.buttons = pad::SQUARE;
            }
            if l != 0 && f > 145 && f % 2 == 0 && mode == "capture" {
                p.buttons = match e.mem.r32(l + 0x1e8) {
                    2 => pad::CROSS,
                    4 => pad::CIRCLE,
                    8 => pad::TRIANGLE,
                    _ => 0,
                };
            }
            g.devices.SetPadState(0, p);
        }
        rt.Frame(&mut e, dt);
        let bs = GetCurrentStateID(&mut e, BOOK);
        let gs = GetCurrentStateID(&mut e, GHOST);
        if r.book.last().map(|x| x.1) != Some(bs) {
            r.book.push((f, bs));
            r.starts.push(format!("f{f} --- book state {bs:#x}"));
        }
        if r.ghost.last().map(|x| x.1) != Some(gs) {
            r.ghost.push((f, gs));
            r.starts.push(format!("f{f} --- ghost state {gs:#x}"));
        }
        let mut g = rt.particles.lock().unwrap();
        let drained: Vec<_> = g.sub.log.sound_play.drain(..).collect();
        for (o, id, res) in drained {
            r.plays.push((f, o, id, res));
            // event data: +0x08 3D mode, +0x0c pick type, +0x10..+0x20 distances, +0x28 group, +0x2c instances, +0x34 set, +0x3c name
            if let Some(d) = g.sub.audio.tse.get((id >> 30) as usize).and_then(|t| t.data.get((id & 0x0fff_ffff) as usize)) {
                r.starts.push(format!("f{f} DATA id {id:#x} mode {} pick {} min {:.1}/{:.2} max {:.1}/{:.2} cut {:.1} grp {:#x} inst {} set {:#x} {:?}",
                    d.word(8), d.word(0xc), d.fword(0x10), d.fword(0x14), d.fword(0x18), d.fword(0x1c), d.fword(0x20), d.word(0x28), d.word(0x2c), d.word(0x34), d.name));
            }
        }
        let a = &g.sub.audio;
        let mut seen = HashMap::new();
        let mut per_handle: HashMap<u32, usize> = HashMap::new();
        for v in &a.voices {
            *per_handle.entry(v.handle).or_default() += 1;
            let k = key(v, f);
            if !live.contains_key(&k) {
                let owner = a.handles.get(v.handle as usize).map_or(0, |h| h.owner);
                r.starts.push(format!(
                    "f{f} START h{} owner {owner:#x} ev {:#x} src {:#x} bank {} pb {} smp {} rate {} L {:.3} R {:.3} pan {:.2} pitch {:.3} delay {:.3} loop {} ch {:?} virt {} stream {:?} adsr {:#x}",
                    v.handle, v.event_id, v.source_event, v.bank, v.playback, v.sample, v.sample_rate, v.left, v.right, v.pan, v.pitch, v.delay, v.looped, v.channel, v.virt, v.stream, v.adsr
                ));
            }
            seen.insert(k, v.clone());
        }
        r.max_voices_per_handle = r.max_voices_per_handle.max(per_handle.values().copied().max().unwrap_or(0));
        for (k, v) in &live {
            if !seen.contains_key(k) {
                r.stops.push(format!("f{f} STOP h{} ev {:#x} pb {} after {:.2}s loop {} paused {}", v.handle, v.event_id, v.playback, v.time, v.looped, v.paused));
            }
        }
        // offline mix of this frame's interval [time-dt, time)
        let base = r.mix.len();
        r.mix.resize(base + per, [0.0; 2]);
        for v in seen.values() {
            if v.stream.is_some() || v.paused || v.virt {
                continue;
            }
            let Some(pcm) = a.banks.get(v.bank as usize).and_then(|b| b.as_ref()).and_then(|b| b.pcm.get(v.sample as usize)).and_then(|p| p.as_ref()) else { continue };
            let env = env_cache.entry((v.bank, v.sample, v.adsr, v.pitch.to_bits())).or_insert_with(|| {
                // loops: envelope over 60 s of looped source
                let mut s = pcm.samples.clone();
                if v.looped {
                    let ls = pcm.loop_start.unwrap_or(0);
                    while s.len() < v.sample_rate as usize * 60 && pcm.samples.len() > ls {
                        s.extend_from_slice(&pcm.samples[ls..]);
                    }
                }
                apply_adsr(&s, v.sample_rate, v.pitch, v.adsr, None)
            });
            let step = v.sample_rate as f32 * v.pitch / OUT_RATE;
            for k in 0..per {
                let t = v.time - dt + k as f32 / OUT_RATE - v.delay;
                if t < 0.0 {
                    continue;
                }
                let i = (t * OUT_RATE * step) as usize;
                let Some(&s) = env.get(i) else { break };
                let s = s as f32 / 32768.0;
                r.mix[base + k][0] += s * v.left;
                r.mix[base + k][1] += s * v.right;
            }
        }
        live = seen;
    }
    r
}

fn write_wav(path: &str, mix: &[[f32; 2]]) {
    let mut d = Vec::with_capacity(44 + mix.len() * 4);
    let n = (mix.len() * 4) as u32;
    d.extend_from_slice(b"RIFF");
    d.extend_from_slice(&(36 + n).to_le_bytes());
    d.extend_from_slice(b"WAVEfmt ");
    for w in [16u32, 0x0002_0001, 48000, 48000 * 4, 0x0010_0004] {
        d.extend_from_slice(&w.to_le_bytes());
    }
    d.extend_from_slice(b"data");
    d.extend_from_slice(&n.to_le_bytes());
    for s in mix {
        for c in s {
            d.extend_from_slice(&((c.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
        }
    }
    std::fs::write(path, d).unwrap();
}

#[test]
fn ep1l01_book_audio_audit() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    for (mode, frames) in [("capture", 2400u32), ("none", 900)] {
        let r = run(&fs, mode, frames);
        eprintln!("==== A2 {mode}: book {:x?}\nghost {:x?}", r.book, r.ghost);
        let mut ev: Vec<String> = r.starts.iter().chain(&r.stops).cloned().collect();
        ev.sort_by_key(|s| s[1..].split(' ').next().unwrap().parse::<u32>().unwrap_or(0));
        for p in &r.plays {
            ev.push(format!("f{} PLAY obj {:#x} id {:#x} -> {}", p.0, p.1, p.2, p.3));
        }
        ev.sort_by_key(|s| s[1..].split(' ').next().unwrap().parse::<u32>().unwrap_or(0));
        for s in ev.iter().filter(|s| std::env::var_os("A2_ALL").is_some() || !s.contains("owner 0x0 ")) {
            eprintln!("A2 {mode} {s}");
        }
        for (sec, c) in r.mix.chunks(48000).enumerate() {
            let rms = |ch: usize| (c.iter().map(|s| s[ch] * s[ch]).sum::<f32>() / c.len() as f32).sqrt();
            let peak = c.iter().fold(0f32, |m, s| m.max(s[0].abs()).max(s[1].abs()));
            eprintln!("A2 {mode} mix t{sec}s rmsL {:.4} rmsR {:.4} peak {:.3}", rms(0), rms(1), peak);
        }
        if let Some(dir) = std::env::var_os("A2_WAV_DIR") {
            write_wav(&format!("{}/a2_{mode}.wav", dir.to_string_lossy()), &r.mix);
        }
        eprintln!("A2 {mode} max voices per handle {}", r.max_voices_per_handle);
        assert!(r.plays.iter().any(|p| p.1 == BOOK), "{mode}: no book sound requests");
        // C sequence (BOOK_MECHANICS): open 0x2e/0x12/0x14/0xf/0x41, capture 0x42 then 0x10, close 0x13/0x11
        let book: Vec<u32> = r.plays.iter().filter(|p| p.1 == BOOK).map(|p| p.2).collect();
        let want: &[u32] = if mode == "capture" { &[0x2e, 0x12, 0x14, 0xf, 0x41, 0x42, 0x10, 0x13, 0x11] } else { &[0x2e, 0x12, 0x14, 0xf, 0x41, 0x13, 0x11] };
        assert_eq!(book, want, "{mode}: book sound sequence");
        // one voice per single-instance event (EFSoundEvent_BC +0x2c instances), audio not frozen by the camera pause
        assert_eq!(r.max_voices_per_handle, 1, "{mode}: stacked voices");
        assert!(r.stops.iter().any(|s| s.contains("ev 0x2e ")), "{mode}: book idle loop 0x2e never stopped");
    }
}

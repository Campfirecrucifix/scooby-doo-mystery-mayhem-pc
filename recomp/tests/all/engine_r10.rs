//! WP-R10: level A -> level B with the carried game buffers (as game.rs switch_level), 600 frames.
//! `cargo test --release --test engine_r10 -- --nocapture` (R10_CHAIN="A>B" for an ad-hoc pair).
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::runtime::*;

use crate::common::{ISO, cutscene_update};
type Eng = scooby_recomp::engine::Engine;

fn frame(e: &mut Eng, rt: &mut EngineRuntime) {
    use scooby_recomp::engine::input::PadState;
    rt.particles.lock().unwrap().devices.SetPadState(0, PadState { connected: true, mode: 7, rx: 128, ry: 128, lx: 128, ly: 128, ..Default::default() });
    rt.Frame(e, 1.0 / 60.0);
    cutscene_update(e, rt);
    let _ = rt.particles.lock().unwrap().game.take_level_request();
}

fn load_carried(fs: &IsoFs, dir: &str, bufs: &[(u32, Vec<u8>)]) -> (Eng, EngineRuntime) {
    let (mut e, mut rt) = EngineRuntime::new_with_engine();
    let file = format!("{}.P2R", dir.rsplit('/').next().filter(|_| !dir.starts_with("EP")).map(str::to_string).unwrap_or_else(|| dir.replace('/', "")));
    let res = EngineRuntime::ReadLevel(fs, dir, &file).unwrap();
    let cs = scooby_recomp::engine::script::EFCutScenePlayer::FromLevelObjects(&res.objects).ok();
    for (id, b) in bufs {
        let p = scooby_recomp::engine::host::gameobj::GetGameBuffer(&mut e, *id, b.len() as u32, 1);
        for (i, v) in b.iter().enumerate() {
            e.mem.w8(p + i as u32, *v);
        }
    }
    rt.LoadLevel(&mut e, res);
    rt.particles.lock().unwrap().cutscene = cs;
    (e, rt)
}

fn chain(fs: &IsoFs, a: &str, b: &str) {
    let (mut e, mut rt) = load_carried(fs, a, &[]);
    for _ in 0..120 {
        frame(&mut e, &mut rt);
    }
    let bufs: Vec<(u32, Vec<u8>)> = scooby_recomp::engine::host::gameobj::CarriedGameBuffers(&mut e);
    let (mut e, mut rt) = load_carried(fs, b, &bufs);
    for _ in 0..600 {
        frame(&mut e, &mut rt);
    }
    println!("R10 {a} -> {b}: {} buffers carried, 600 frames ok", bufs.len());
}

#[test]
fn r10_level_chains() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    if let Ok(f) = std::env::var("R10_CARRY") {
        // buffers dumped as (id u32, len u32, bytes) records; load R10_LEVEL with them
        let d = std::fs::read(f).unwrap();
        let mut bufs = Vec::new();
        let mut i = 0;
        while i < d.len() {
            let id = u32::from_le_bytes(d[i..i + 4].try_into().unwrap());
            let n = u32::from_le_bytes(d[i + 4..i + 8].try_into().unwrap()) as usize;
            bufs.push((id, d[i + 8..i + 8 + n].to_vec()));
            i += 8 + n;
        }
        let (mut e, mut rt) = load_carried(&fs, &std::env::var("R10_LEVEL").unwrap_or("EP1/L02".into()), &bufs);
        for _ in 0..600 {
            frame(&mut e, &mut rt);
        }
        return;
    }
    if let Ok(s) = std::env::var("R10_CHAIN") {
        let (a, b) = s.split_once('>').unwrap();
        return chain(&fs, a, b);
    }
    for (a, b) in [("EP1/L01", "EP1/L02"), ("EP1/L02", "EP1/L03A"), ("EP2/L01C", "EP2/L02A"), ("EP3/L05", "LOADER"), ("LOADER", "EP1/L02"), ("EP6/L05B", "LOADER")] {
        chain(&fs, a, b);
    }
}

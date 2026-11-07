//! WP-P3: PSS movie demux/decode + MpegPlay skip/end semantics on the disc movies.
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::video::*;

use crate::common::ISO;

#[test]
fn demux_ep01a() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let d = fs.read("MOVIES/EP01/EP01A.PSS").unwrap();
    let p = demux_pss(&d);
    assert_eq!(p.tracks.len(), 6, "3 stereo pairs of mono tracks");
    assert_eq!(p.track_fmt[&0], (0x10, 48000));
    assert!((video_fps(&p.video_es) - 29.97).abs() < 0.01);
    let (rate, pcm) = decode_audio(&p, 0).unwrap();
    assert_eq!(rate, 48000);
    let secs = pcm.len() as f32 / 2.0 / 48000.0;
    assert!((secs - 34.57).abs() < 0.1, "{secs}");
    assert!(pcm.iter().any(|&s| s.abs() > 1000));
}

fn wait_start(m: &mut MpegPlay) {
    for _ in 0..20000 {
        m.Update(0.0, 0);
        if m.started || m.done {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[test]
fn skip_rules() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let fs = std::sync::Arc::new(fs);
    let v = EFVideoPlayer_SP { name: "MOVIES\\EP01\\EP01A".into(), audio_track: 0 };
    let f2 = fs.clone();
    let mut m = v.Play(0, 1.0, false, move |n| f2.read(&iso_path(n)).ok());
    assert_eq!(m.skip_mask, 0x840);
    wait_start(&mut m);
    assert!(m.started);
    let first = m.take_new_frame().unwrap();
    assert_eq!((first.width, first.height), (720, 480));
    // CROSS (engine 0x4000) held from the start: no rising edge -> no skip; before 60 pictures no skip
    m.Update(1.0 / 60.0, 0x4000);
    assert!(!m.MpgSkip);
    // play past 60 pictures, then press START (engine 0x0008): skipped
    for _ in 0..100000 {
        m.Update(1.0 / 60.0, 0);
        if m.frame_count > 70 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(m.frame_count > 60, "{}", m.frame_count);
    assert!(!m.Update(1.0 / 60.0, 0x0008));
    assert!(m.MpgSkip);

    // missing file: MpegPlay returns 0 at once
    let mut m = MpegPlay::new(1.0, "MOVIES\\NOPE.PSS", 0x840, 0, false, || None);
    wait_start(&mut m);
    assert!(m.done && !m.MpgSkip);
    // LEGAL is unskippable; DEMOS take any button
    assert_eq!(MpegPlay::new(1.0, "MOVIES\\LEGAL.PSS", 0x840, 0, false, || None).skip_mask, 0);
    let d = EFVideoPlayer_SP { name: "MOVIES\\DEMOS\\DEP01".into(), audio_track: 0 };
    assert_eq!(d.Play(0, 1.0, false, |_| None).skip_mask, 0xffff);
    assert_eq!(pad_to_mpeg(0x4008), 0x0840);
}

#[test]
fn decode_speed() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let d = fs.read("MOVIES/EP01/EP01A.PSS").unwrap();
    let p = demux_pss(&d);
    let t = std::time::Instant::now();
    let mut dec = na_mpeg2_decoder::Decoder::new();
    let mut n = 0;
    for ch in p.video_es.chunks(0x8000) {
        match dec.decode(ch, None) { Ok(v) => n += v.len(), Err(e) => { eprintln!("err {e:?} at n={n}"); } }
    }
    n += dec.flush().map(|v| v.len()).unwrap_or(0);
    { eprintln!("es {}", p.video_es.len());
    }
    eprintln!("{n} frames in {:?}", t.elapsed());
}

/// P36: MpegPlay::UpdateClock locks the picture to an absolute (audio) clock: a host hitch of 1 s
/// advances ~30 pictures at once instead of the clamped frame delta (video fell behind the sound).
#[test]
fn p36_clock_sync() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let fs = std::sync::Arc::new(fs);
    let v = EFVideoPlayer_SP { name: r"MOVIES\EP01\EP01A".into(), audio_track: 0 };
    let mut m = v.Play(0, 1.0, false, move |n| fs.read(&iso_path(n)).ok());
    wait_start(&mut m);
    assert!(m.started);
    m.blocking = true;
    m.UpdateClock(0.5, 0);
    let a = m.frame_count;
    assert!((14..=17).contains(&a), "{a}");
    // a 1 s hitch: one call catches up to t = 1.5 s (picture ~46 at 29.97 fps)
    m.UpdateClock(1.5, 0);
    let b = m.frame_count;
    assert!((44..=47).contains(&b), "{b}");
    // the clock never runs backwards
    m.UpdateClock(1.0, 0);
    assert_eq!(m.frame_count, b);
}

/// P36 R7: A/V sync of every disc movie. The picture is locked to the audio clock (UpdateClock: picture
/// floor(t*fps)+1), so drift only happens when the decoder thread has no picture ready. Decode each
/// movie as fast as possible (per-picture cost), then replay the real-time schedule offline with the
/// decoder's bounded channel (6 messages + 1 pending): picture i is due at (i-1)/fps after the first
/// picture; lag = how late it can be shown, in pictures, max per third (start/middle/end). Also prints
/// the soundtrack vs video length (the movie ends when both are done, like `while (!audioDone ...)`).
/// Run: cargo test --release --test all p36_av_sync_all -- --ignored --nocapture
#[test]
#[ignore]
fn p36_av_sync_all() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let fs = std::sync::Arc::new(fs);
    let mut names: Vec<String> = fs.names().filter(|n| n.to_ascii_uppercase().ends_with(".PSS")).map(|s| s.to_string()).collect();
    names.sort();
    let mut worst = 0.0f64;
    for n in &names {
        let (f2, p) = (fs.clone(), n.clone());
        let mut m = MpegPlay::new(1.0, &n.replace('/', "\\"), 0, 0, false, move || f2.read(&p).ok());
        m.blocking = true;
        let mut arr = Vec::new();
        let t0 = std::time::Instant::now();
        let mut last = 0;
        while !m.done {
            m.Update(if m.started { 1.0 / m.fps } else { 0.0 }, 0);
            if m.frame_count != last {
                last = m.frame_count;
                arr.push(t0.elapsed().as_secs_f64());
            }
            if arr.len() > 20000 { break; }
        }
        if arr.len() < 2 {
            eprintln!("avsync {n}: no pictures");
            continue;
        }
        // decode cost per picture (the consumer pulled at once, so the decoder never waited)
        let d: Vec<f64> = std::iter::once(0.0).chain(arr.windows(2).map(|w| w[1] - w[0])).collect();
        let (fps, cap) = (m.fps as f64, 7usize);
        // real-time replay: decoder finishes picture i at fin[i]; it may start i only once picture i-cap was shown
        let mut fin = vec![0.0f64; d.len()];
        let mut shown = vec![0.0f64; d.len()];
        let mut lag = [0.0f64; 3];
        for i in 0..d.len() {
            let free = if i >= cap { shown[i - cap] } else { 0.0 };
            let start = if i == 0 { 0.0 } else { fin[i - 1].max(free) };
            fin[i] = start + d[i];
            let due = fin[0] + i as f64 / fps;
            shown[i] = due.max(fin[i]);
            let l = (fin[i] - due).max(0.0) * fps;
            let k = (i * 3 / d.len()).min(2);
            lag[k] = lag[k].max(l);
        }
        let vs = d.len() as f64 / fps;
        worst = worst.max(lag[0].max(lag[1]).max(lag[2]));
        eprintln!(
            "avsync {n}: {} pictures {:.2} s video, {:.2} s audio, decode {:.2} s ({:.1}x realtime); lag frames start {:.2} mid {:.2} end {:.2}",
            d.len(), vs, m.audio_secs, arr[arr.len() - 1], vs / arr[arr.len() - 1].max(1e-9), lag[0], lag[1], lag[2]
        );
    }
    eprintln!("avsync worst lag {worst:.2} frames over {} movies", names.len());
}

/// P36 R10 (USER_CHECKS D1, headless): output device lost in the middle of a movie. Replays the host's
/// movie loop (game.rs step_runtime: picture on the movie wall clock via UpdateClock; play_movie:
/// device::movie_audio_restart on every new output generation; audio_device_watch: DeviceWatch
/// close / retry / reopen) at 60 Hz on EP01A with a loss at 2.0 s. The picture keeps running while
/// the sound is gone, the soundtrack restarts once at the clock position after the reopen, and from
/// then on the audio position stays within 1 picture of the shown picture. The next movie (on the
/// new stream generation) starts its sound at 0 with its first picture.
#[test]
fn p36_r10_movie_device_loss_sync() {
    use scooby_recomp::engine::audio::device::{movie_audio_restart, DeviceAction, DeviceWatch};
    let Ok(fs) = IsoFs::open(ISO) else { return };
    let fs = std::sync::Arc::new(fs);
    let play = |name: &str| {
        let f = fs.clone();
        let v = EFVideoPlayer_SP { name: name.into(), audio_track: 0 };
        let mut m = v.Play(0, 1.0, false, move |n| f.read(&iso_path(n)).ok());
        wait_start(&mut m);
        assert!(m.started, "{name}");
        m.blocking = true;
        m
    };
    let mut m = play(r"MOVIES\EP01\EP01A");
    let (fps, len) = (m.fps as f64, m.audio_secs as f64);
    assert!(len > 8.0);
    let (mut watch, mut ogen, mut avail) = (DeviceWatch::default(), 0u32, true);
    let mut snd_gen: Option<u32> = None;
    let mut snd: Option<(f64, f64)> = None; // (wall clock at (re)start, soundtrack offset)
    let (mut restarts, mut worst, mut silent_pictures) = (Vec::new(), 0.0f64, 0u32);
    for k in 0..(8 * 60) {
        let t = k as f64 / 60.0;
        let lost = avail && (2.0..2.0 + 1.0 / 60.0).contains(&t);
        match watch.step(t, avail, lost) {
            DeviceAction::Close => {
                avail = false;
                snd = None;
            }
            DeviceAction::Reopen => {
                avail = true;
                watch.reopened(true);
                ogen += 1;
            }
            DeviceAction::None => {}
        }
        m.UpdateClock(t as f32, 0);
        if let Some(at) = movie_audio_restart(snd_gen, ogen, avail, t, len) {
            restarts.push((t, at));
            snd = Some((t, at));
            snd_gen = Some(ogen);
        }
        let pic = (m.frame_count.max(1) - 1) as f64 / fps;
        match snd {
            Some((t0, off)) => worst = worst.max((off + (t - t0) - pic).abs()),
            None => silent_pictures += 1,
        }
    }
    eprintln!("p36 r10 movie loss: restarts {restarts:?}, worst A/V {:.1} ms (1 picture = {:.1} ms), {silent_pictures} silent ticks, reconnects {}", worst * 1e3, 1e3 / fps, watch.reconnects);
    assert_eq!(restarts.len(), 2, "first start + one restart after the reopen: {restarts:?}");
    assert_eq!(restarts[0], (0.0, 0.0));
    let (tr, at) = restarts[1];
    assert!((2.9..3.1).contains(&tr) && (at - tr).abs() < 1e-9, "{restarts:?}");
    assert!(worst <= 1.0 / fps + 1e-6, "A/V off by {:.1} ms", worst * 1e3);
    assert!(m.frame_count as f64 >= 7.9 * fps - 1.0, "picture stalled: {}", m.frame_count);
    // next movie on the new generation: sound from 0 together with its first picture
    let m2 = play(r"MOVIES\EP01\EP01B");
    assert_eq!(movie_audio_restart(None, ogen, true, 0.0, m2.audio_secs as f64), Some(0.0));
    assert_eq!(movie_audio_restart(Some(ogen), ogen, true, 0.5, m2.audio_secs as f64), None);
    assert!(m2.frame_count <= 1);
}

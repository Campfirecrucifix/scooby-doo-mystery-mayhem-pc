//! WP-P3: PSS movie demux/decode + MpegPlay skip/end semantics on the disc movies.
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::engine::video::*;

const ISO: &str = "../Scooby Doo Mystery Mayhem.ISO";

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

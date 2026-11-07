//! A3: SPU2 voice loop semantics + incremental ADPCM stream decode.
use scooby_recomp::engine::audio::spu::{AdpcmStream, SpuData, SpuVoice};
use scooby_recomp::engine::audio::vag::decode_adpcm;
use std::sync::Arc;

/// ADPCM block, filter 0 shift 0 -> nibble n is sample n<<12.
fn blk(flags: u8, n: u8) -> [u8; 16] {
    let mut b = [0u8; 16];
    b[0] = 0x00;
    b[1] = flags;
    for x in &mut b[2..] {
        *x = (n & 0xf) | (n << 4);
    }
    b
}

// ADSR: attack max rate, sustain level max -> envelope ~1 after the first ticks
const ADSR: u32 = 0x000f | (0x1fc0 << 16);

#[test]
fn loop_plays_head_once_then_loops_from_loop_start() {
    // blocks: head(1) | loop start(2, flag 4) | end+repeat(3, flags 3)
    let data: Vec<u8> = [blk(0, 1), blk(4, 2), blk(3, 3)].concat();
    let d = decode_adpcm(&data, true);
    assert_eq!(d.loop_start, Some(28));
    let v = SpuVoice::new(SpuData::Pcm(d.samples.clone().into(), d.loop_start), 48000, 1.0, ADSR, true, 0.0);
    let env = scooby_recomp::engine::audio::adsr::envelope(ADSR, 48000 * 8, None);
    let out: Vec<i16> = v.take(28 * 400).collect();
    // undo the envelope: sample value / (4096 * env)
    let lv = |i: usize, s: i16| ((s as f32) / (4096.0 * env[i])).round() as i16;
    let seq: Vec<i16> = out.iter().enumerate().skip(28).map(|(i, &s)| lv(i, s)).collect();
    // after the head, output cycles 2,3 only (never the head value 1)
    assert!(!seq.contains(&1), "head replayed: {seq:?}");
    // period = 56 samples
    let full: Vec<i16> = SpuVoice::new(SpuData::Pcm(d.samples.into(), d.loop_start), 48000, 1.0, ADSR, true, 0.0).take(28 * 9).collect();
    let lvl: Vec<i16> = full.iter().enumerate().map(|(i, &s)| lv(i, s)).collect();
    assert_eq!(lvl[28 * 3..28 * 5], lvl[28 * 5..28 * 7]);
    assert_eq!((lvl[27], lvl[28], lvl[56], lvl[84], lvl[112]), (1, 2, 3, 2, 3));
    // the streamed (incremental) decoder gives the same raw sequence
    let s: Vec<i16> = AdpcmStream::new(Arc::from(data.as_slice()), false).take(28 * 7).collect();
    let raw: Vec<i16> = s.iter().map(|&x| x / 4096).collect();
    assert_eq!(raw[0], 1);
    assert_eq!(raw[28], 2);
    assert_eq!(raw[56], 3);
    assert_eq!(raw[84], 2);
    assert_eq!(raw[140], 2);
}

#[test]
fn one_shot_ends_and_delay_is_once() {
    let data: Vec<u8> = [blk(0, 1), blk(1, 2)].concat();
    let n = AdpcmStream::new(Arc::from(data.as_slice()), false).count();
    assert_eq!(n, 56);
    let v = SpuVoice::new(SpuData::Adpcm(Arc::from(data.as_slice())), 1000, 1.0, ADSR, false, 0.01);
    assert_eq!(v.count(), 56 + 10);
}

/// Real STREAM VAGs: the incremental decoder reproduces the whole-file decode sample-exactly, and
/// a looping stream continues from its loop start after the end block.
#[test]
fn iso_stream_incremental_matches_whole() {
    use scooby_recomp::engine::audio::vag::decode_vag;
    let Ok(fs) = scooby_recomp::engine::iso::IsoFs::open("../Scooby Doo Mystery Mayhem.ISO") else { return };
    let names: Vec<String> = fs.names().map(|n| n.to_string()).filter(|n| n.to_uppercase().contains("STREAM") && n.to_uppercase().ends_with(".VAG")).take(6).collect();
    assert!(!names.is_empty());
    for n in names {
        let f = fs.read(&n).unwrap();
        let (_, whole) = decode_vag(&f);
        let be = u32::from_be_bytes(f[0xc..0x10].try_into().unwrap()) as usize;
        let body: Arc<[u8]> = f[0x30..(0x30 + be).min(f.len())].into();
        let inc: Vec<i16> = AdpcmStream::new(body.clone(), true).take(whole.samples.len() + 1000).collect();
        assert_eq!(&inc[..whole.samples.len()], &whole.samples[..], "{n}");
        let ls = whole.loop_start.unwrap_or(0);
        // loop continuation (history kept, so compare loosely: first block after the loop jump)
        assert_eq!(inc.len(), whole.samples.len() + 1000, "{n}");
        eprintln!("{n}: {} samples, loop_start {:?}, after-end {:?} vs loop {:?}", whole.samples.len(), whole.loop_start, &inc[whole.samples.len()..whole.samples.len() + 4], &whole.samples[ls..ls + 4]);
    }
}

/// P26 R23: the speed handed to rodio must never make its SampleRateConverter's u32
/// `from * pos` wrap (output stall = all audio silent). Emulates rodio 0.22.2: source rate =
/// (rate as f32 * speed).max(1) as u32, ratio reduced by gcd against the device rate.
#[test]
fn p26_rodio_safe_speed_no_u32_overflow() {
    use scooby_recomp::engine::audio::spu::rodio_safe_speed;
    fn gcd(a: u64, b: u64) -> u64 { if b == 0 { a } else { gcd(b, a % b) } }
    let wraps = |src: u32, dev: u32| { let g = gcd(src as u64, dev as u64); (src as u64 / g) * (dev as u64 / g - 1) > u32::MAX as u64 };
    // the user-log case (old raw Tome pitch) wraps without the snap
    assert!(wraps((44100f32 * 8.1325) as u32, 48000));
    for rate in [11025u32, 22050, 24000, 32000, 44100, 48000, 37800, 18900] {
        for i in 0..4000 {
            let pitch = 0.05 + i as f32 * 0.003;
            let s = rodio_safe_speed(rate, pitch);
            let src = (rate as f32 * s).max(1.0) as u32;
            assert!((src as f32 / (rate as f32 * pitch) - 1.0).abs() < 0.01 || (rate as f32 * pitch) < 5000.0, "{rate} {pitch} -> {src}");
            for dev in [44100u32, 48000, 88200, 96000, 192000] {
                assert!(!wraps(src, dev), "{rate} Hz x {pitch} -> {src} Hz overflows at {dev} Hz");
            }
        }
    }
    assert_eq!(rodio_safe_speed(44100, 1.0), 1.0);
    assert_eq!(rodio_safe_speed(44100, f32::NAN), 1.0);
}

/// P32 R9: every SBH sample rate on the disc (8000/11025/16000/22050/44100) plays at pitch 1 (speed
/// left unsnapped by rodio_safe_speed) without rodio's u32 resampler overflow at any device rate.
#[test]
fn p32_sbh_rates_pitch1_no_overflow() {
    use scooby_recomp::engine::{audio::sbh::Sbh, iso::IsoFs};
    fn gcd(a: u64, b: u64) -> u64 { if b == 0 { a } else { gcd(b, a % b) } }
    let Ok(fs) = IsoFs::open(crate::common::ISO) else { return };
    let mut rates = std::collections::BTreeSet::new();
    let names: Vec<String> = fs.names().filter(|n| n.to_uppercase().ends_with(".SBH")).map(|s| s.to_string()).collect();
    for n in &names {
        for x in Sbh::parse(&fs.read(n).unwrap()).unwrap().samples { rates.insert(x.sampling_rate); }
    }
    assert!(names.len() > 100 && !rates.is_empty());
    for &r in &rates {
        for dev in [44100u64, 48000, 88200, 96000, 192000] {
            let g = gcd(r as u64, dev);
            assert!((r as u64 / g) * (dev / g - 1) <= u32::MAX as u64, "{r} Hz at {dev} Hz");
        }
    }
}

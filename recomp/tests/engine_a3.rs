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

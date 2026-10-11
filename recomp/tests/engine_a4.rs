//! A4: interleaved stereo .INT music (A2M header, 0x6000-byte L/R chunks) decode + loop.
use scooby_recomp::engine::audio::spu::{is_interleaved, AdpcmStream, SpuData, SpuVoice, INT_HEADER, INT_INTERLEAVE};
use scooby_recomp::engine::iso::IsoFs;
use std::sync::Arc;

const ISO: &str = "../Scooby Doo Mystery Mayhem.ISO";

fn corr(a: &[i16], b: &[i16]) -> f64 {
    let (mut ab, mut aa, mut bb) = (0f64, 0f64, 0f64);
    for (&x, &y) in a.iter().zip(b) {
        let (x, y) = (x as f64, y as f64);
        ab += x * y;
        aa += x * x;
        bb += y * y;
    }
    ab / (aa * bb).sqrt().max(1.0)
}

/// mean |s[k]-s[k-1]| at chunk boundaries / mean over all samples
fn boundary_ratio(p: &[i16]) -> f64 {
    let bs = INT_INTERLEAVE / 16 * 28;
    let d = |k: usize| (p[k] as i32 - p[k - 1] as i32).abs() as f64;
    let ks: Vec<usize> = (bs..p.len()).step_by(bs).collect();
    let jb = ks.iter().map(|&k| d(k)).sum::<f64>() / ks.len() as f64;
    let all = (1..p.len()).map(d).sum::<f64>() / (p.len() - 1) as f64;
    jb / all
}

fn wav(path: &str, l: &[i16], r: &[i16], rate: u32) {
    let mut b = Vec::new();
    let n = l.len().min(r.len()) as u32;
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + n * 4).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    for v in [16u32, 0x0002_0001, rate, rate * 4, 0x0010_0004] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(b"data");
    b.extend_from_slice(&(n * 4).to_le_bytes());
    for i in 0..n as usize {
        b.extend_from_slice(&l[i].to_le_bytes());
        b.extend_from_slice(&r[i].to_le_bytes());
    }
    std::fs::write(path, b).unwrap();
}

#[test]
fn int_music_stereo() {
    let Ok(fs) = IsoFs::open(ISO) else { return };
    for name in ["STREA_/EP1/LEVEL1/MUEP1L1.INT", "STREA_/MUMENU.INT"] {
        let f: Arc<[u8]> = fs.read(name).unwrap().into();
        assert!(is_interleaved(&f));
        assert_eq!(u32::from_be_bytes(f[0x10..0x14].try_into().unwrap()), 44100);
        assert_eq!((f.len() - INT_HEADER) % (2 * INT_INTERLEAVE), 0);
        let secs = 8 * 44100;
        let ch: Vec<Vec<i16>> = (0..2u8).map(|c| SpuVoice::new(SpuData::Interleaved(f.clone(), c), 44100, 1.0, 0, true, 0.0).take(secs).collect()).collect();
        let (l, r) = (&ch[0], &ch[1]);
        let c = corr(l, r);
        let (bl, br) = (boundary_ratio(l), boundary_ratio(r));
        let rms = |p: &[i16]| (p.iter().map(|&x| (x as f64).powi(2)).sum::<f64>() / p.len() as f64).sqrt();
        eprintln!("{name}: L/R corr {c:.3}, boundary jump/avg step L {bl:.2} R {br:.2}, rms L {:.0} R {:.0}", rms(l), rms(r));
        assert!(c > 0.4, "L/R uncorrelated");
        assert!(bl < 2.0 && br < 2.0, "discontinuous at chunk boundaries");
        assert!(rms(l) > 300.0 && rms(r) > 300.0);
        if let Ok(dir) = std::env::var("A4_WAV_DIR") {
            wav(&format!("{dir}/{}.wav", name.rsplit('/').next().unwrap()), l, r, 44100);
        }
        // loop: the driver loop flag restarts at the body start, never ends
        let per = (f.len() - INT_HEADER) / 2 / 16 * 28;
        let mut a = AdpcmStream::interleaved(f.clone(), 0, true);
        let first: Vec<i16> = a.by_ref().take(per).collect();
        let again: Vec<i16> = a.by_ref().take(44100).collect();
        assert_eq!(again.len(), 44100);
        assert!(corr(&first[..44100], &again) > 0.95, "loop restarts at the start");
        assert_eq!(AdpcmStream::interleaved(f.clone(), 1, false).count(), per);
    }
}

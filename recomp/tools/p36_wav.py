"""P36 R2: analyse a GAME_AUDIO_WAV capture (48 kHz s16 stereo).
usage: python tools/p36_wav.py capture.wav [click_threshold=0.5] [seam=T ...]
Prints length, overall RMS/peak per channel, clipped samples, silence stretches (> 2 s below
-60 dBFS), and click candidates (sample-to-sample jumps above the threshold, i.e. crackles,
bad loop seams or hard voice cuts), with times."""
import sys, struct, math, array

def main():
    path = sys.argv[1]
    thr = float(sys.argv[2]) if len(sys.argv) > 2 else 0.5
    with open(path, 'rb') as f:
        b = f.read()
    assert b[:4] == b'RIFF' and b[8:12] == b'WAVE'
    rate = struct.unpack_from('<I', b, 24)[0]
    a = array.array('h'); a.frombytes(b[44:])
    n = len(a) // 2
    print(f'{path}: {n / rate:.2f} s @ {rate} Hz')
    for ch in (0, 1):
        x = a[ch::2]
        s2 = sum(v * v for v in x) / max(1, len(x))
        pk = max((abs(v) for v in x), default=0)
        clip = sum(1 for v in x if abs(v) >= 32767)
        print(f'  ch{ch}: rms {10 * math.log10(s2 / 32768**2 + 1e-12):.1f} dBFS peak {pk / 32768:.3f} clipped {clip}')
    # silence stretches (mono, 100 ms windows)
    w = rate // 10
    quiet, start = 0, None
    out = []
    for i in range(0, n - w, w):
        e = sum((a[2 * j] / 32768) ** 2 + (a[2 * j + 1] / 32768) ** 2 for j in range(i, i + w, 8)) / (w / 8) / 2
        db = 10 * math.log10(e + 1e-12)
        if db < -60:
            if start is None: start = i
        else:
            if start is not None and (i - start) / rate > 2: out.append((start / rate, i / rate))
            start = None
    if start is not None and (n - start) / rate > 2: out.append((start / rate, n / rate))
    print('  silence > 2 s:', ', '.join(f'{s:.1f}-{e:.1f}' for s, e in out) or 'none')
    clicks = []
    for ch in (0, 1):
        x = a[ch::2]
        for i in range(1, len(x)):
            if abs(x[i] - x[i - 1]) > thr * 32768:
                clicks.append(i / rate)
    clicks.sort()
    print(f'  click candidates (jump > {thr}): {len(clicks)}', ' '.join(f'{t:.3f}' for t in clicks[:30]))
    # P36 R3: seam=T: max sample jump within +-20 ms of T vs the 99.9th percentile of the 2 s around it
    for arg in sys.argv[3:]:
        if not arg.startswith('seam='):
            continue
        t = float(arg[5:])
        i0, i1 = max(1, int((t - 1) * rate)), min(n, int((t + 1) * rate))
        for ch in (0, 1):
            j = [abs(a[2 * i + ch] - a[2 * (i - 1) + ch]) for i in range(i0, i1)]
            if not j:
                continue
            k = int((t - 0.02) * rate) - i0, int((t + 0.02) * rate) - i0
            near = max(j[max(0, k[0]):max(1, k[1])], default=0)
            p999 = sorted(j)[int(len(j) * 0.999)]
            lvl = math.sqrt(sum((a[2 * i + ch] / 32768) ** 2 for i in range(i0, i1)) / (i1 - i0))
            print(f'  seam {t:.3f} ch{ch}: max jump +-20ms {near / 32768:.4f}, p99.9 of +-1s {p999 / 32768:.4f}, rms {lvl:.4f} -> {"CLICK?" if near > 2 * p999 and near > 0.02 * 32768 else "ok"}')

main()

#!/usr/bin/env python3
"""R26: list int<->float candidates in module code (pattern (a)).

A candidate is a numeric cast `(e.r32(BASE + N*4)) as f32` where BASE is an object
local-data pointer (`e.r32(g(e, X))` or `e.r32((frame + X))`). Evidence that the slot
really holds a float (so the cast should be f32::from_bits) is collected per
(file, base, index):
  +3  the same slot is stored with a float-looking bit constant (e.g. 0x3f800000)
  +3  the same slot is stored with `(... as i32)) as u32` of a float expression
      that reads the slot as f32 (x = (int)((float)x + ...))
  +1  each further `as f32` read of the slot
  -3  the slot is stored with a small integer / incremented as an int
Usage: python tools/p21_intfloat_audit.py [min_score]
"""
import re, sys, pathlib, collections

ROOT = pathlib.Path(__file__).resolve().parent.parent / "src" / "modules"
BASE = r"\(e\.r32\((?:g\(e, (0x[0-9a-f]+)\)|\(frame \+ (0x[0-9a-f]+)\))\)\)"
SLOT = BASE + r"\.wrapping_add\(\((\d+)u32\)\.wrapping_mul\(4\)\)"
BSLOT = BASE + r"\.wrapping_add\((\d+)u32\)"  # byte offset
RF = re.compile(r"e\.rf\(" + BSLOT + r"\)")
BSTORE = re.compile(r"let a0 = " + BSLOT + r"; e\.w32\(a0, (0x[0-9a-f]+)u32\)")
READF = re.compile(r"\(e\.r32\(" + SLOT + r"\)\) as f32")
STORE = re.compile(r"let a0 = " + SLOT + r"; (?:let a1 = (.*?); e\.w32\(a0, a1\)|e\.w32\(a0, (0x[0-9a-f]+|\d+)u32\))")


def floaty(c):
    v = int(c, 16) if c.startswith("0x") else int(c)
    if v == 0:
        return None
    exp = (v >> 23) & 0xff
    return (v & 0xfff) == 0 and 100 < exp < 160


def key(m, off=0):
    return ("g" + m.group(1 + off)) if m.group(1 + off) else ("f" + m.group(2 + off)), int(m.group(3 + off))


def main():
    mn = int(sys.argv[1]) if len(sys.argv) > 1 else 1
    rows = []
    for p in sorted(ROOT.rglob("*.rs")):
        t = p.read_text(encoding="utf-8", errors="replace")
        reads = collections.defaultdict(list)
        for i, line in enumerate(t.splitlines(), 1):
            for m in READF.finditer(line):
                reads[key(m)].append(i)
        if not reads:
            continue
        score = collections.Counter()
        why = collections.defaultdict(list)
        for i, line in enumerate(t.splitlines(), 1):
            for m in STORE.finditer(line):
                k = key(m)
                expr, const = m.group(4), m.group(5)
                if const is not None:
                    f = floaty(const)
                    if f:
                        score[k] += 3; why[k].append(f"L{i} store {const}")
                    elif f is False and int(const, 0) < 0x10000:
                        score[k] -= 3; why[k].append(f"L{i} int {const}")
                elif expr is not None:
                    if "as i32)) as u32" in expr and "as f32" in expr:
                        score[k] += 3; why[k].append(f"L{i} (int)(float expr)")
                    elif "wrapping_add(1" in expr or "wrapping_sub(1" in expr:
                        score[k] -= 3; why[k].append(f"L{i} int inc")
            for m in RF.finditer(line):
                k = key(m); k = (k[0], k[1] // 4)
                score[k] += 3; why[k].append(f"L{i} e.rf")
            for m in BSTORE.finditer(line):
                k = key(m); k = (k[0], k[1] // 4)
                if floaty(m.group(4)):
                    score[k] += 3; why[k].append(f"L{i} store {m.group(4)}")
        for k, ls in reads.items():
            s = score[k] + len(ls)
            if s >= mn:
                rows.append((s, str(p.relative_to(ROOT.parent.parent)), k, ls, why[k]))
    rows.sort(key=lambda r: (-r[0], r[1]))
    for s, f, k, ls, w in rows:
        print(f"{s:3d} {f} {k[0]}[{k[1]}] reads L{','.join(map(str, ls))} | {'; '.join(w)}")
    print(f"# {len(rows)} slots in {len({r[1] for r in rows})} files")


if __name__ == "__main__":
    main()


def fix(path, slots):
    """Rewrite reads/stores of the given float slots [(base, idx)] to bit reinterpretation."""
    p = pathlib.Path(path)
    t = p.read_text(encoding="utf-8")
    n = 0
    for b, i in slots:
        bx = f"g(e, {b[1:]})" if b[0] == "g" else f"(frame + {b[1:]})"
        s = f"(e.r32({bx})).wrapping_add(({i}u32).wrapping_mul(4))"
        sb = f"(e.r32({bx})).wrapping_add({i * 4}u32)"
        old = f"((e.r32({s})) as f32)"
        n += t.count(old)
        t = t.replace(old, f"f32::from_bits(e.r32({s}))")
        out = []
        for line in t.split("\n"):
            for a0 in (s, sb):
                pre = f"let a0 = {a0}; let a1 = "
                j = line.find(pre)
                if j >= 0:
                    k = line.find("; e.w32(a0, a1)", j)
                    a1 = line[j + len(pre):k]
                    if ("f32::from_bits" in a1 or "e.rf(" in a1 or "fVar" in a1) and (a1.endswith(" as i32)) as u32)") or a1.endswith(" as i64 as u32)")):
                        na = a1.replace(" as i64 as u32)", ".to_bits())") if a1.endswith(" as i64 as u32)") else a1[: -len(" as i32)) as u32)")] + ")).to_bits())"
                        line = line[: j + len(pre)] + na + line[k:]
                        n += 1
            out.append(line)
        t = "\n".join(out)
    p.write_text(t, encoding="utf-8")
    return n

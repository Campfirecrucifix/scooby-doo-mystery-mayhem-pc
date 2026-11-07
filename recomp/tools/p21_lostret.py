#!/usr/bin/env python3
"""R27: `let _ = <pure float getter>` sites (Ghidra lost the f0 return). For each, show the first float local
read in the next lines (the stale local the C uses). Usage: p21_lostret.py [--fix]"""
import re, sys, pathlib
ROOT = pathlib.Path(__file__).resolve().parent.parent / "src" / "modules"
GET = r"(?:APIextra::TimerGetTime|GetFrameDeltaTime|FloatRandom|APImath::\w+)"
SITE = re.compile(r"^(\s*)let _ = (.*\b" + GET + r"\(.*);\s*$")
VAR = re.compile(r"\b(fVar\d+|in_f0|fStack_\w+)\b")
fix = "--fix" in sys.argv
n = 0
for p in sorted(ROOT.rglob("*.rs")):
    L = p.read_text(encoding="utf-8").split("\n"); ch = False
    for i, l in enumerate(L):
        m = SITE.match(l)
        if not m: continue
        use = None
        for j in range(i + 1, min(i + 8, len(L))):
            s = L[j]
            a = re.match(r"\s*(fVar\d+) = ", s)
            v = VAR.search(s[a.end():] if a else s)
            if v: use = (j + 1, v.group(1), s.strip()[:120]); break
            if a: break
        dbg = use and "in_f0" == use[1]
        print(f"{p.relative_to(ROOT)}:{i+1} -> {use[1] if use else '-'} {'DEBUG' if dbg else ''} | {use[2] if use else ''}")
        n += 1
        if fix and use and use[1].startswith("fVar"):
            L[i] = f"{m.group(1)}{use[1]} = {m.group(2)};"; ch = True
    if ch: p.write_text("\n".join(L), encoding="utf-8")
print("#", n)

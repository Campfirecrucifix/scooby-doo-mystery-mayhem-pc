#!/usr/bin/env python3
"""WP-P20 R29: connectivity map from a GAME_FLOOD dump (game.rs flood_dump).
usage: p20_flood.py dump.txt step_up jump_up name=x,y,z ...
Components = surface cells joined by walkable steps (|dy| <= step_up, no wall, headroom >= 1.0);
then lists jump links (up to jump_up) between components and which named point lies in which component."""
import sys, collections
f, STEP, JUMP = sys.argv[1], float(sys.argv[2]), float(sys.argv[3])
pts = {}
for a in sys.argv[4:]:
    n, v = a.split('='); pts[n] = tuple(map(float, v.split(',')))
cells = {}
xs = set()
for line in open(f):
    t = line.split()
    if len(t) < 2: continue
    x, z = float(t[0]), float(t[1]); xs.add(x)
    L = [(float(t[i]), int(t[i+1]), int(t[i+2])) for i in range(2, len(t) - 2, 3)]
    cells[(x, z)] = L
xs = sorted(xs); s = xs[1] - xs[0]
nodes = {}
for (x, z), L in cells.items():
    for i, (y, wx, wz) in enumerate(L):
        above = L[i-1][0] if i > 0 else 99
        if above - y >= 1.0:
            nodes[(x, z, i)] = (y, wx, wz)
par = {k: k for k in nodes}
def find(a):
    while par[a] != a:
        par[a] = par[par[a]]; a = par[a]
    return a
jumps = []
for (x, z, i), (y, wx, wz) in nodes.items():
    for dx, dz, wall in ((s, 0, wx), (0, s, wz)):
        nb = (x + dx, z + dz)
        if nb not in cells: continue
        for j, (y2, _, _) in enumerate(cells[nb]):
            k2 = (nb[0], nb[1], j)
            if k2 not in nodes: continue
            d = y2 - y
            if abs(d) <= STEP and not wall:
                par[find((x, z, i))] = find(k2)
            elif abs(d) <= JUMP:
                jumps.append(((x, z, i), k2, d))
comp = collections.Counter(find(k) for k in nodes)
def at(p):
    x, y, z = p
    best = None
    for k, (yy, _, _) in nodes.items():
        if abs(k[0] - x) <= s / 2 + 1e-6 and abs(k[1] - z) <= s / 2 + 1e-6:
            if best is None or abs(yy - y) < abs(nodes[best][0] - y): best = k
    return best
named = {}
for n, p in pts.items():
    k = at(p)
    r = find(k) if k else None
    named[n] = r
    print(f"{n:12s} {p} -> node {k} y {nodes[k][0] if k else None} comp {r} size {comp[r] if r else 0}")
links = collections.defaultdict(list)
for a, b, d in jumps:
    ca, cb = find(a), find(b)
    if ca != cb and d > 0:
        links[(ca, cb)].append((a, b, round(d, 2)))
want = set(v for v in named.values() if v)
print("jump-up links touching named components (from -> to, count, example):")
for (ca, cb), L in sorted(links.items(), key=lambda kv: -len(kv[1])):
    if ca in want or cb in want:
        nm = lambda c: [n for n, v in named.items() if v == c] or [f"size{comp[c]}"]
        print(f"  {nm(ca)} -> {nm(cb)} n={len(L)} e.g. {L[0]}")
# optional: P20_PATH=a,b prints the walkable node path between two named points (every 6th node)
import os
if os.environ.get("P20_PATH"):
    a, b = os.environ["P20_PATH"].split(',')
    src, dst = at(pts[a]), at(pts[b])
    prev = {src: None}; dq = collections.deque([src])
    while dq:
        k = dq.popleft()
        if k == dst: break
        x, z, i = k; y, wx, wz = nodes[k]
        for dx, dz in ((s, 0), (-s, 0), (0, s), (0, -s)):
            nb = (x + dx, z + dz)
            if nb not in cells: continue
            for j, (y2, w2x, w2z) in enumerate(cells[nb]):
                k2 = (nb[0], nb[1], j)
                if k2 in nodes and k2 not in prev and abs(y2 - y) <= STEP:
                    wl = wx if dx > 0 else wz if dz > 0 else (w2x if dx < 0 else w2z)
                    if not wl:
                        prev[k2] = k; dq.append(k2)
    path = []; k = dst
    while k is not None and k in prev:
        path.append(k); k = prev[k]
    path.reverse()
    print("path", a, "->", b, len(path), [(p[0], p[1], nodes[p][0]) for p in path[::6]] + [(dst[0], dst[1], nodes[dst][0])])

//! WP-P21 R47: walk planner for test / harness bots (not game code). A* over a 0.5 m grid on the stage
//! collision map (APIphysic ray tests through runtime_j2 J2::ray_intersect, mask words 0,0,3,0,0 as GAME_MAP),
//! one floor height per cell: a neighbour is walkable when a down ray finds a floor no more than CLIMB above
//! (steps / ramps) or DROP below the current cell, and a body-height side ray (with a body-width margin)
//! between the two cells hits nothing.
use crate::engine::runtime::J2;
use crate::engine::{Mem, Ptr};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

pub const CELL: f32 = 0.5;
pub const CLIMB: f32 = 0.45;
pub const DROP: f32 = 1.2;

/// one ray (origin, delta) -> hit y (out +4); `scr` = 0x80 bytes scratch
fn ray(mem: &mut Mem, j2: &J2, scr: Ptr, o: [f32; 3], dv: [f32; 3]) -> Option<f32> {
    let w = [o[0], o[1], o[2], dv[0], dv[1], dv[2]].map(f32::to_bits);
    for (i, v) in w.iter().chain(&[0, 0, 3, 0, 0]).enumerate() {
        mem.w32(scr + 4 * i as u32, *v);
    }
    (j2.ray_intersect(mem, scr, scr + 0x30) != 0).then(|| f32::from_bits(mem.r32(scr + 0x34)))
}

/// path of cell points (with floor y) from `from` to the cell nearest `goal` (or to the reachable cell
/// closest to it); empty when `from` has no floor. Search window: the from-goal box + `margin` m.
pub fn nav_path(mem: &mut Mem, j2: &J2, scr: Ptr, from: [f32; 3], goal: [f32; 3], margin: f32) -> Vec<[f32; 3]> {
    let x0 = from[0].min(goal[0]) - margin;
    let z0 = from[2].min(goal[2]) - margin;
    let nx = (((from[0].max(goal[0]) + margin - x0) / CELL).ceil() as i32 + 1).min(1200);
    let nz = (((from[2].max(goal[2]) + margin - z0) / CELL).ceil() as i32 + 1).min(1200);
    let idx = |i: i32, j: i32| (i * nz + j) as usize;
    let at = |i: i32, j: i32| [x0 + i as f32 * CELL, z0 + j as f32 * CELL];
    let ci = |p: [f32; 3]| (((p[0] - x0) / CELL).round() as i32, ((p[2] - z0) / CELL).round() as i32);
    let n = (nx * nz) as usize;
    let (mut fy, mut prev, mut gc) = (vec![f32::NAN; n], vec![-1i32; n], vec![u32::MAX; n]);
    let (si, sj) = ci(from);
    // the window is capped at 1200 cells: a far goal (other room) can leave `from` outside it
    if si < 0 || sj < 0 || si >= nx || sj >= nz {
        return vec![];
    }
    let Some(y0) = ray(mem, j2, scr, [at(si, sj)[0], from[1] + 1.0, at(si, sj)[1]], [0.0, -2.5, 0.0]) else { return vec![] };
    let (gi, gj) = ci(goal);
    let h = |i: i32, j: i32| ((((i - gi).pow(2) + (j - gj).pow(2)) as f32).sqrt() * 10.0) as u32;
    let s = idx(si, sj);
    fy[s] = y0;
    gc[s] = 0;
    let mut open = BinaryHeap::new();
    open.push(Reverse((h(si, sj), s)));
    let mut best = (h(si, sj), s);
    while let Some(Reverse((_, c))) = open.pop() {
        let (i, j) = (c as i32 / nz, c as i32 % nz);
        if (i, j) == (gi, gj) {
            best = (0, c);
            break;
        }
        if h(i, j) < best.0 {
            best = (h(i, j), c);
        }
        let p = at(i, j);
        let y = fy[c];
        for (di, dj, k) in [(1, 0, 10), (-1, 0, 10), (0, 1, 10), (0, -1, 10), (1, 1, 14), (1, -1, 14), (-1, 1, 14), (-1, -1, 14)] {
            let (ni, nj) = (i + di, j + dj);
            if ni < 0 || nj < 0 || ni >= nx || nj >= nz {
                continue;
            }
            let nc = idx(ni, nj);
            let g = gc[c] + k;
            if g >= gc[nc] {
                continue;
            }
            let q = at(ni, nj);
            let Some(ny) = ray(mem, j2, scr, [q[0], y + CLIMB + 0.3, q[1]], [0.0, -(CLIMB + 0.3 + DROP), 0.0]) else { continue };
            let by = y.max(ny) + 0.6;
            if ray(mem, j2, scr, [p[0], by, p[1]], [(q[0] - p[0]) * 1.8, 0.0, (q[1] - p[1]) * 1.8]).is_some() {
                continue;
            }
            (fy[nc], prev[nc], gc[nc]) = (ny, c as i32, g);
            open.push(Reverse((g + h(ni, nj), nc)));
        }
    }
    let mut path = vec![];
    let mut c = best.1 as i32;
    while c >= 0 {
        let p = at(c / nz, c % nz);
        path.push([p[0], fy[c as usize], p[1]]);
        c = prev[c as usize];
    }
    path.reverse();
    path
}

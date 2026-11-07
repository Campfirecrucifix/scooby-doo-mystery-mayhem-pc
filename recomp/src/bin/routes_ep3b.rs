//! WP-P21 R28: Episode 3 real-play routes for the game binary (`game EP3/L01 --headless --autoplay --route ep3`;
//! format = game.rs parse_route, see routes_ep1b.rs). Replaces the P17 warps of routes_ep3.rs by walking.
//! EP3/L01 progression (C: PuzzleController.c, inv_racecontroller.c, inv_control__355232f3):
//!   start hub (531,247): poltergeists 0x132/0x133 -> GB2+4 0xc -> inv_control 0x14e -> ghost_lock 0x138 0xdf
//!   (GB2+0xc 0x80000 = door 0x101 at 539.6,249.4); race (inv_racecontroller) -> GB2+0xc 0x10000 = upper doors
//!   0xf9/0xfa (y 7.2) -> puzzle room (785,252); door puzzle order 0x14,0x12,0x11,0x13,0x10,0x15 ->
//!   GB2+0xc 0x20000 = door 0xfd (534.5,259.2) -> (1241.5,321.5) -> door 0xf7 (1222.7,275.6) loads EP3\L02.
//!   R37: the 0xfd room is dark (lantern kills at dmg 0x100000 without lantern GB2+0 0x200 / oil GB4+0): grab the
//!   lantern powerup 0x114 (1241.1,318.9) and oil 0x11d (1240.9,310.9) first.
#![allow(dead_code)]

/// FAKE: `31,0,Q` = GB2+0 |= 0x1f (TITLE EP3 episode select, case 0x40) until a real EP2-end carry exists.
pub const L01_ROUTE: &str = "31,0,Q;538.2,253.4;537.6,253.4;538.8,253.4;535.7,240.2,G;537.4,251.6,G;539.65,249.44,P;541.8,249.44;542.3,249.1,P;542.3,249.1,P;542.3,249.1,P;547.2,248.6,R;549.6,246.5,R;545.2,244.8,R;548.8,240.1,R;551.4,241.2,R;554.9,241.3,R;556.6,241.4,R;559.1,239.2,R;557.2,237.6,R;551.9,237.0,R;546.2,233.5,R;558.8,245.3,R;555.5,249.8,R;556.7,253.9,R;560.1,250.8,R;560.0,258.4,R;556.5,261.1,R;551.7,260.1,R;548.4,260.7,R;546.2,257.5,R;551.4,251.6,R;547.2,248.6,R;549.6,246.5,R;545.2,244.8,R;548.8,240.1,R;551.4,241.2,R;554.9,241.3,R;556.6,241.4,R;559.1,239.2,R;557.2,237.6,R;551.9,237.0,R;546.2,233.5,R;558.8,245.3,R;555.5,249.8,R;556.7,253.9,R;560.1,250.8,R;560.0,258.4,R;556.5,261.1,R;551.7,260.1,R;548.4,260.7,R;546.2,257.5,R;551.4,251.6,R;534,247,D;530.8,246,D;530.8,240,D;530.8,237.5,D;541.4,237.5,D;541.94,238.3,X;785.7,253,P;788.6,257.4,P;788.6,254.6,D;791.7,252.3,P;790,254.6,D;788.7,252,P;788.7,254.6,D;791.6,257.1,P;790,254.6,D;786.5,254.6,D;790,254.6,D;786.5,254.6,D;790,254.6,D;786.5,254.6,D;785.7,251.7,P;786.5,254.6,D;790,254.6,D;786.5,254.6,D;790,254.6,D;786.5,254.6,D;785.7,258.3,P;541.4,257.3,D;531,257.3,D;530.8,252,D;530.8,246,D;534,247,D;534.5,253;534.5,257.5;534.5,259.2,X;1241.1,318.9;1240.9,310.9;1235,300;1221.4,282.7;1222.7,277;1222.7,275.6,X";

/// not a real binary: cargo auto-discovers every src/bin/*.rs as a bin target
fn main() {}

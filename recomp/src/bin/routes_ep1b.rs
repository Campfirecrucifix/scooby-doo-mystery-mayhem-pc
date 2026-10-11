//! WP-P10: Episode 1 game.rs waypoint routes for EP1 L03A / L03B / L04 (format = game.rs parse_route:
//! "x,z[,action];..." actions X door/hot-spot, J jump, W wait for a cut-scene, S hold square, B book
//! capture of the nearest uncaptured monster, P press X once, D direct steer).
//! Wiring (coordinator; game.rs is not edited by this WP): in parse_route add
//!   Some("l03a") => routes_ep1b::L03A_ROUTE, Some("l03b") => routes_ep1b::L03B_ROUTE, Some("l04") => routes_ep1b::L04_ROUTE
//! (e.g. `#[path = "routes_ep1b.rs"] mod routes_ep1b;` in game.rs). level_route already maps EP1/L03A..L04.
//! Until then: `game EP1/L03A --headless --autoplay --route "<string>"` (raw route strings are accepted).
#![allow(dead_code)]

/// EP1/L03A (direct boot, no carried bits needed: Velma hands over the book in the W cut-scene, power 90):
/// clue ledge (J) -> Velma (W) -> captures B x5 (shadows near 418,-8 / 413,-3, room via door 0x2a0000?? at
/// 411.8,-7.75 -> 209.3,-14.6 -> 119,-3.2 -> three B in the west room) -> lock room south -> door at 32.3,-1
/// (X) loads EP1\L03B (~f13400).
pub const L03A_ROUTE: &str = "551.7,0.07,X;417.8,-17.8;413.5,-18,J;413.2,-18;412.9,-17.5;418.8,-17.5;425,-16;442,-16;442.5,-6;441.5,-3;439.8,-3.6,W;444,-3,S;418,-8;416,-8,B;419,-3.5;415,-2.5;413.5,-2.5;413.5,-3,B;413.5,-3;413.5,-7.75;411.8,-7.75,X;207,5;209.3,-13;209.3,-14.6,X;112,-5;119,-4;119,-3.2,X;20,-3;20,-3,B;18,-6;18,-6,B;20,-8;20,-8,B;28,-10;32.2,-1.6;32.3,-1,X";
pub const L03B_ROUTE: &str = "";
pub const L04_ROUTE: &str = "";

/// not a real binary: cargo auto-discovers every src/bin/*.rs as a bin target
fn main() {}

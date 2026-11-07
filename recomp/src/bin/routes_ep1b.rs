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
/// WP-P18: the door room (x 31.2..40.5, z -10..0) is sealed by static walls (map material word1 = 7, the
/// hero masks hit it) from the west room and the chest room; the chest costume (P at 32.3,-13.1 -> scooby_ghost
/// 0x2a0000e8, 15 s) does not pass them either. WP-P20: real entry = from the north corridor (z -17.5) jump (J at
/// 38,-15.2) onto the raised floor x 33..40.5 z -15.5..-10 (map height 2) and walk down into the door room; no
/// FAKE. The door lock bit 0x2000 (GB2+0xc) is set by the real captures.
/// WP-P27: with the camera anim the east-room ghost catches the hero on 419,-3.5 -> 415,-2.5 (courage 0.3 extra
/// hit -> 0.3 at the west room -> scared, game over); `200,0,H` at 419.3,-14 lets it pass (courage 0.7 like main).
// P32 R30: `!` waypoints = GAME_LOGICCLIP=1 only (game.rs parse_route): from the clip L02 carry the hero enters with
// courage 0.2 and the diagonal to 417.8,-17.8 crosses ghost 0x2a00002c (patrols ~419-433,-8..-12.6) -> scared, respawn at
// the J step; the clip detour goes south along x 442 and west along z -17.5 (no hit; reaches wp 27 at 207,5 f12001).
// P35 R4: clip-only detour after the 413.5,-3 B: under GAME_LOGICCLIP=1 the chased shadow flees east and the capture
// ends at 428.3,-4.7; the straight line back west runs Scooby into the pillar at ~426,-4.4 (stuck from f11426, wp 24-26
// skipped). Back east, then the already-walked 423.5,-7.6 / 419.3,-12 / 419,-3.5 / 415,-2.5 legs (clip L03A -> L03B f16561).
pub const L03A_ROUTE: &str = "551.7,0.07,X;!442,-6;!442,-16;!425,-17.5;417.8,-17.8;413.5,-18,J;413.2,-18;412.9,-17.5;418.8,-17.5;425,-16;442,-16;442.5,-6;439.8,-3.6,W;444,-3,S;418,-8;416,-8,B;432.6,-2.7;423.5,-7.6;419.3,-14;200,0,H;419,-3.5;415,-2.5;413.5,-2.5;413.5,-3,B;!432.6,-2.7;!423.5,-7.6;!419.3,-12;!419,-3.5;!415,-2.5;413.5,-3;413.5,-7.75;411.8,-7.75,X;207,5;209.3,-13;209.3,-14.6,X;106,-5.25;112,-5;119,-4;119,-3.2,X;20,-3;20,-3,B;23.1,-7.7;18,-6;18,-6,B;12.8,-8.4;20,-8;20,-8,B;23,-12;23,-17.5;38,-17.5;38,-15.2,J;38,-13;38,-10.6;37,-8;33,-3;32.3,-1,X";
/// WP-P16: EP1/L03B (arrival 380,-21 from L03A): door 0x54 to the rotating-door room (484,-2.25); ghost
/// 0x2a000008 lives there and scares the team (meter 0 -> heroInit_Dead -> respawn at 380: the "snap back")
/// -> capture it first (G; sets GB2+0xc 0x20000 = door 0x4d lock); X on rotatingdoors 0x7f twice (P) carries
/// the team west; door 0x4d (X) -> (233,10.6); room 204..234: capture ghosts 0x0a/0x04 (G)
/// WP-P20: wisps 0x2a0000e0 (214.8,2.5) / 0x2a0000e2 (223.4,-5.7) refill the book (49.5 -> 85.5) before the
/// second capture (power 40 alone cannot hold a 50-life ghost)
/// WP-P20: (after the l03_camera float fix) around the static wall x 471.5..472 z -6..0 via z 1.5 to door 0x4d
/// WP-P20 R4: door 0x4e (234.25,3.25) -> wisps 0xdf/0xd8/0xde/0xd9 (300..307, power 40.5 -> 180) -> back via
/// 0x4c; door 0x4a (204,3.17) -> 141.6,-1.3; ghost 0x07 (G; GB2+4 0x400000 -> inv_control 0x14e sends 0xdf to
/// ghost lock 0x126 -> door 0x46 lock 0x10000); door 0x46 (114.09,-0.47) -> west part (-51.5,0.5)
/// WP-P20 R5: west part: G x2, then circle wispsourcenew 0x2a0000d3 (-65.8,0.1; HugeWisp2 children respawn,
/// power 22.5 -> 130.5), third G (-50,12; GB2+4 bits 2|0x100000|0x200000 -> inv_control 0xdf to lock 0x127 ->
/// GB2+0xc 0x8000), door 0x43 (-77.8,0) X -> LoadLevel EP1\L04 (movie EP01H)
/// P20 R172 (GAME_EMCHILD on in game.exe): `5,2.5,A` before the third west G: wait until ghost 0x2a000005 (flies ~4 m
/// up, wanders between -67.5,-6.1 and -50..-55,-12.5) idles (state 0xf), walk under it (< 2.5 m), then G. Without it the
/// G opened the book at 5 m (ghost at -60,-5.4 out of the scan range) -> flee -> leader stuck on the -55.6,-6.4 step.
pub const L03B_ROUTE: &str = "380,0;379.75,9;379.75,11.85,X;484,-4,G;479,0.9;479,0.9,P;474,0.3;474,0.3,P;473,1.5,D;469.5,1.5,D;469,0.5,D;468.7,-2.3,X;233,10.4;228,10,D;228,7,D;232,3.9;234.25,3.25,X;300.8,2.43;301.75,1.8;306.1,-2.8;307.4,-2.6;297.5,0.4;296.3,0.4,X;206,3.2;217,6,G;220,2,G;206,3.2;204,3.17,X;135,0,G;116,-0.5;114.09,-0.47,X;-60,0;-64,0.1;-65.8,0.9;-66.6,0.1;-65.8,-0.7;-65,0.1;-65.8,0.9;-66.6,0.1;-65.8,-0.7;-59.5,0.5,G;-50,12,G;-64,0.1;-65.8,0.9;-66.6,0.1;-65.8,-0.7;-65,0.1;-65.8,0.9;-66.6,0.1;-65.8,-0.7;-64,0.1;-65.8,0.9;-66.6,0.1;-65.8,-0.7;-65,0.1;-65.8,0.9;-66.6,0.1;-65.8,-0.7;5,2.5,A;-59.5,-10,G;-75,0;-77.8,0,X";
/// WP-P20 R6: EP1/L04 (from L03B door 0x43, start 553.4,0): Scooby starts ALONE (inv_control Start_WithoutShaggy
/// -> 0x7b 0x22 -> TEAMMODE_ALONE, GB0+0x34 |= 1), so rotating doors refuse the push (their collide/X need
/// GB0+0x34 & 0x101 == 0). Walk the door-free path (BFS over static map + closed door panels) to Shaggy's
/// room: hs_found_shaggy 0x2a00000a (593.9,-25.4) sets GB2+8 0x1000000 -> inv_control 0xd1 Start_WithShaggy.
/// L04 has no wisp objects: the tome power comes from the fakeghost 0x2a000033 (surprised within 1 m of the
/// leader, timer >= 5 s -> FakeGhostFUNC_DropWisp), so 'F' chases it to power 180 before each of the five
/// captures (ghosts -> GB2+4 0x3e000000) -> inv_control cut-scene -> LoadLevel LOADER (movie EP01E) = EP1 end.
/// P20 R50: F (fakeghost wisp farm) threshold 70 -> 130: with 70 a capture could drain the tome to 0 and
/// the bot then parked at 555.5,-1 for the rest of the level (chained carry); LOADER from both c4 and chain carries
/// P20 R55: C retrievers + creation order (defaults): walk back to the farm spot 583,-24.5 after the 590,-5 capture
/// (else the F anchor at 581,0 never sees fakeghost 0x33); LOADER from chain + c4 carries (GAME_CLIPORDER=0 not)
/// P20 R126: `583,-20,D` before the farm after the 595,-5 capture: from 583,-24.5 fakeghost 0x33 (590.5,-20.3, behind
/// a wall) was unreachable under ANIMSND (game.exe default) and the leader was scared back to the start; from
/// 583,-20 the farm/captures finish (LOADER with ANIMSND on and off, chained EP1L04 carry)
/// P20 R136 (fixed follower): `583,-20,B` before the farm: the farm leader idles at the anchor while fakeghost 0x33
/// runs off (585.7,0.9), and roaming ghost 0x2a000019 (states 0x11 -> 0x13 attack) drains the courage to 0 -> respawn at the
/// start (power 90 -> 135, every later waypoint unreachable). With power 90 the B captures the nearest ghost first; LOADER.
// P35 R6: clip-only detour before the 3rd F: under GAME_LOGICCLIP=1 the fakeghost 0x2a000033 flees to its spot
// ~585.8,0.9 (>10 m from the 583,-24.5 farm anchor, the F never chases; power stuck at 112.5 < 130). Walk north along
// x 584.5 to -2 so the farm starts next to it (clip L04 from the L03B carry -> LOADER/STATUS f~17360).
pub const L04_ROUTE: &str = "554,-15,D;554,-25,D;570,-25,D;571.7,-24,D;571.7,-1,D;577,-0.5,D;577,-7.5,D;578,-8.5,D;582.5,-8.5,D;582.5,-2,D;584.5,-0.5,D;584.5,-14,D;583,-14,D;583,-24,D;593.5,-24,D;583,-24.5;51,130,F;580,-24.5,G;51,130,F;590,-5,G;583,-24.5,D;!583,-14,D;!584.5,-14,D;!584.5,-8,D;!584.5,-2,D;51,130,F;595,-5,G;583,-20,D;583,-20,B;583,-14,D;584.5,-14,D;584.5,-8,D;51,130,F;583,-14,D;580,-18,D;51,130,F;578,-13,G;51,130,F;560,-17,G;51,130,F;560,-17,G;560,-17,W";

/// not a real binary: cargo auto-discovers every src/bin/*.rs as a bin target
fn main() {}

/// P20 R142: EP2/L01A real play, a waypoint port of routes_ep2::EP2_L01A_REAL (key 0x95 on the y 4 deck,
/// ledge climbs, door 0x11a, ghost 0x145 capture, siege tower, switch 0x164 (`V`), stair zone, exit 0x122).
/// No fakes: the old "1024,0,K" lock bit and the two 'Z' warps are gone.
pub const EP2_L01A_ROUTE: &str = "30,0,H;300,255,N;290,285,N;289.6,291.2,N;289.6,289.7,N;286,289.7,J;282.5,293,N;274,296.5,N;272.6,296.5,N;272.6,302.5,N;268,303.5,N;264.6,304.3,N;272.6,302,N;272.6,296,N;282.5,293,N;284.4,289.7,N;291,289.7,J;291,289.5,N;291,285,N;277,230,N;600,0,H;279,228,D;279.6,224.5,D;281.5,224.5,J;282.3,226.5,J;282.8,229,N;285.7,229.2,N;288,231,N;300,229,N;312.2,226.7,X;562,239,N;580,239,N;591,239.5,N;591.5,243,N;580,243.5,N;574.85,243.61,N;580,243.5,N;591.5,243,N;591,239.5,N;562,239,N;556.5,239.3,N;554.6,239.3,J;200,1,H;298.5,228,N;288.5,230.5,N;286.8,229.5,N;285.3,229.3,J;283.5,229,J;282.5,226.5,N;281.5,224.5,N;279.5,224.5,N;279,228,N;266,229,N;263,229,N;258,229,N;257,233,N;257,239,N;259.5,246,N;261.5,242,N;266,240.5,N;272,240.5,N;273,245.5,N;275,249.5,N;276.5,254,N;278,258.5,N;280,259.5,N;289,259.5,N;295,259.5,N;299.5,261,N;300,263.5,N;301,265.5,N;301.2,266.1,Y;305.4,265.8,N;310.35,249.9,G;310,265,N;310.8,266,N;312.3,266.2,J;200,1,H;560,262,N;566,259.5,N;566.5,254,N;567.5,252,N;570,251.5,N;590.5,251,N;591.5,253,N;591.5,256,N;591,258.5,N;588,258.5,N;584,260.5,N;580,262.5,N;580,268.5,N;582,269.5,N;583.5,271,N;601,271.5,N;603.5,272,N;603.75,276.45,J;200,1,H;816.7,288.5,N;816.7,282,N;822,281,N;830,281,N;842,283,N;856,280,N;858.5,279.5,N;300,0,H;842,283,N;830,281,N;822,281,N;816.7,282,N;816.7,288.5,N;822.6,291,N;822.6,293.5,N;822.6,294.6,J;823,299,J;600,0,H;823,299.6,J;300,0,H;822.7,304,N;60,0,H;822.8,297,N;822.6,295,J;400,0,H;822.6,292,D;815.5,290.6,N;814,290.6,J;200,1,H;600,273,N;587,282,N;570.5,282,N;570.5,297,N;580,302,N;590,306,N;600,310.5,N;603.5,310.5,J;200,1,H;830,326,N;838,328,N;858,328,N;864,327,N;866.5,326.5,N;866.98,325.8,N;300,0,H;866.98,325.9,D;356,84,V;400,0,H;866.98,326,N;200,0,H;875.08,325.8,N;300,0,H;875.08,326.3,P;400,0,H;870,327,N;600,0,H;838,328,N;830,326,N;816.5,325,N;814,324.85,J;200,1,H;600,311,N;590,306,N;580,302,N;570.5,297,N;570.5,282,N;587,282,N;601,271.5,N;603.5,272,N;603.75,276.45,J;200,1,H;816.7,288.5,N;816.7,282,N;822,281,N;830,281,N;842,283,N;860,283,N;862.5,283.5,N;865,287,N;869,288,N;871,290,N;876,292,N;879,292,N;888.5,291.6,N;888.4,291.6,J;600,1,H";

/// P20 R44: EP2/L01B (entered from L01A exit door 0x122; carry tests/all/data/ep2_L01A_end_EP2L01B.bin).
/// Team starts at 1106.5,0,265.5 / 267.5. R62: real (no fakes): buttons e3/e4/e5/e6, rocket from the SW, ride,
/// door 0xda hop, west ramp down, exit door 0xdd -> EP2/L01C (f~6220).
pub const EP2_L01B_ROUTE: &str = "1121.7,272.3;1116,285;1115.9,288.0,P;1116,284;1125,280;1135,280;1136,284;1138.2,286;1138.2,288.4,P;1138,285;1140,283.4;1141.4,283.4,P;1135,280;1125,280;1118,279.3;1114,279.2,P;1120,279.2,D;1130,277,D;1134,275.5,D;1136.6,275.4,P;1136.6,275.4,W;1131,288.3,D;1128.8,289,P;1127.6,292;1120,291;1114,291;1114,296;1127.45,296.5;1127.45,297.7,X";

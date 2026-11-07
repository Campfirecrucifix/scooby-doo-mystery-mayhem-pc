//! CreateEmitterPos data: per monster type, the constant stores into the ghost's local data in
//! source order (joint of glow i at +0x2dc + i*4, its offset at +0x3e8 + i*0xc; the boss also
//! stores sprite sizes at +0x70c + i*4). WP-X3: decoded at run time from CreateEmitterPos of the
//! reference build smallghostlevelone__757255de (the function is the same text in all 31 ghost
//! builds): run i = branch i (`gamedata::store_runs`).
pub use crate::engine::gamedata::StoreRun;

const KEY: &str = "757255de";
const F: &str = "CreateEmitterPos";
/// monster type 0x1 (small ghost): 136 stores
pub static EMITTER_POS_1: StoreRun = StoreRun(KEY, F, 0);
/// monster type 0x2 (medium ghost): 200 stores
pub static EMITTER_POS_2: StoreRun = StoreRun(KEY, F, 1);
/// monster type 0x3 (big ghost): 260 stores
pub static EMITTER_POS_3: StoreRun = StoreRun(KEY, F, 2);
/// monster type 0x13 (ghost boss): 75 stores
pub static EMITTER_POS_13: StoreRun = StoreRun(KEY, F, 3);

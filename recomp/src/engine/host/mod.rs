//! WP-S: host-side implementations of engine APIs that the real runtime still stubbed.
//! runtime.rs keeps one-line delegations into these modules.
pub mod physic;
pub mod gameobj;
pub mod projector;
pub mod fbeffects;
pub mod game;
pub mod extra;
pub mod stdlib;
pub mod loadsave;
pub mod dtmath;

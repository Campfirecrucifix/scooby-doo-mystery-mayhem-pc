//! WP-G audio: PS2 ADPCM decode (replacement), TSC/TSE tables + SBH (1:1), SBB banks,
//! container selection (1:1), path resolution (1:1), EFNewAudioMgr (handles, per-object sound
//! lists, 3D volume/pan, ADSR, SPU channels), APInewSound bindings, descriptor handlers, streams.
pub mod adsr;
pub mod api;
pub mod bank;
pub mod capture;
#[cfg(feature = "bevy_audio")]
pub mod bevy_adapter;
pub mod container;
pub mod device;
pub mod events;
pub mod handlers;
pub mod mgr;
pub mod paths;
pub mod sbh;
pub mod sound3d;
pub mod spu;
pub mod stream;
pub mod tables;
pub mod vag;
pub mod wav;

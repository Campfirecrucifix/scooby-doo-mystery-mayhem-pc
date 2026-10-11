//! RenderWare 3.4 asset loaders (replacement for RenderWare/*, not 1:1): binary stream chunks,
//! the game's image-sequence texture dictionaries, clumps (frames, geometry, materials, skin,
//! HAnim, game plugins), lights, and the BSP world with PS2 native sector data decoded on the CPU.
//! Everything is plain `Send + Sync` data; nothing here touches Bevy rendering.
pub mod clump;
pub mod geometry;
pub mod image;
pub mod ps2;
pub mod stream;
pub mod world;

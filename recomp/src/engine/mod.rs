//! Engine side of the port: emulated module memory, the engine API tables, and the core part of EXE.1
//! that the logic modules depend on (logic runtime, game buffers, object lookup).
//!
//! Memory model (non-obvious, deliberate): the game-logic modules are C code that does pointer
//! arithmetic on engine-allocated blocks (`*(int *)(pLocalDataTome + 0x68)`), keeps pointers inside
//! those blocks (global data -> game buffers) and hands pointers to the engine. To keep the behaviour
//! byte-exact, every such block lives in [`Mem`], a little-endian 32-bit address space like the PS2's,
//! and pointers are plain `u32` addresses ([`Ptr`]). Module locals whose address is never taken are
//! ordinary Rust locals.

mod api_gen;
pub mod anim;
pub mod audio;
pub mod camera;
pub mod core;
mod mem;
pub mod dt;
pub mod error;
pub mod gamedata;
pub mod gfx;
pub mod host;
pub mod hud;
pub mod input;
pub mod iso;
pub mod light;
pub mod level;
pub mod logic_link;
pub mod pack;
pub mod rw;
pub mod spline;
pub mod video;
pub mod physic;

pub use api_gen::*;
pub use mem::{Mem, Ptr, NULL};

use crate::logic::LogicRuntime;

/// The engine as seen by the logic modules: memory + logic runtime + the platform host.
/// Lives in the Bevy world as a `Resource` (see [`crate::plugin`]); there is no global state.
#[derive(bevy_ecs::resource::Resource)]
pub struct Engine {
    pub mem: Mem,
    pub rt: LogicRuntime,
    pub host: Box<dyn Host>,
    /// uRam003439d0: the engine's shared work matrix (CMatGetWorkMatrix)
    pub work_matrix: Ptr,
    /// RW matrix module flags (DAT_002b32f4 + _matrixModule): 0x20000 = identity optimisation on.
    /// Only RwMatrixInvert reads it. Assumed on, which is RenderWare's default.
    pub rw_matrix_module_flags: u32,
    /// EFEngineController's frame-time filter (GetFrameDeltaTime)
    pub clock: crate::logic::frame::FrameClock,
    /// Stands for PS2 address 0 (low kernel RAM, which the game can write without faulting) where
    /// the original writes through a NULL pointer and reads the data back later, e.g. the ghost
    /// cut-scene callbacks' `GetClippingParams(SELF, *(L+0x85c))` / `MiscFunc_EndCutscene(id,
    /// *(L+0x85c))` with L+0x85c never set. Module code maps such NULLs to this block explicitly;
    /// like the real address 0 it is shared by every module. 0x100 bytes.
    pub zero_page: Ptr,
}

impl Engine {
    pub fn new(host: Box<dyn Host>) -> Self {
        let mut e = Engine {
            mem: Mem::new(),
            rt: LogicRuntime::default(),
            host,
            work_matrix: NULL,
            rw_matrix_module_flags: 0x20000,
            clock: Default::default(),
            zero_page: NULL,
        };
        e.zero_page = e.mem.malloc(0x100);
        e.mem.memset(e.zero_page, 0, 0x100);
        e.work_matrix = core::rwmath::RwMatrixCreate(&mut e);
        // EFGameLogicMgr's constructor creates timer 0x90000000 (no owner, never notifies)
        e.CreateTimer();
        e
    }
}

// Inherent forwarders so that `e.w32(f(e), v)` compiles (two-phase borrows do not go through DerefMut).
macro_rules! fwd {
    ($($n:ident(&self $(, $a:ident: $t:ty)*) -> $r:ty;)* ; $($m:ident(&mut self $(, $b:ident: $u:ty)*) -> $s:ty;)*) => {
        impl Engine {
            $(#[inline] #[track_caller] pub fn $n(&self $(, $a: $t)*) -> $r { self.mem.$n($($a),*) })*
            $(#[inline] #[track_caller] pub fn $m(&mut self $(, $b: $u)*) -> $s { self.mem.$m($($b),*) })*
        }
    };
}
fwd! {
    r8(&self, a: Ptr) -> u8; r16(&self, a: Ptr) -> u16; r32(&self, a: Ptr) -> u32; ri32(&self, a: Ptr) -> i32;
    rf(&self, a: Ptr) -> f32; rv3(&self, a: Ptr) -> [f32; 3];
    ;
    w8(&mut self, a: Ptr, v: u8) -> (); w16(&mut self, a: Ptr, v: u16) -> (); w32(&mut self, a: Ptr, v: u32) -> ();
    wi32(&mut self, a: Ptr, v: i32) -> (); wf(&mut self, a: Ptr, v: f32) -> (); wv3(&mut self, a: Ptr, v: [f32; 3]) -> ();
    alloca(&mut self, n: u32) -> Ptr; cstr(&mut self, s: &str) -> Ptr; memset(&mut self, a: Ptr, v: u8, n: u32) -> ();
    memcpy(&mut self, d: Ptr, s: Ptr, n: u32) -> ();
}

// Module code reads/writes memory as `e.r32(a)` / `e.w32(a, v)` (Mem methods through Deref).
impl std::ops::Deref for Engine {
    type Target = Mem;
    fn deref(&self) -> &Mem {
        &self.mem
    }
}
impl std::ops::DerefMut for Engine {
    fn deref_mut(&mut self) -> &mut Mem {
        &mut self.mem
    }
}

/// Object id meaning "the object whose logic is running" (EFWorld::GetGameObject).
pub const SELF: u32 = 0x2a00_0000;
pub mod particle;
pub mod waypoint;
pub mod nav;
pub mod runtime;
pub mod script;
pub mod settings;

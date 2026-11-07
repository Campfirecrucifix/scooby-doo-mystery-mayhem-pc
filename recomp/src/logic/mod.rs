//! Port of the engine's logic runtime (EXE.1 Classes/EF: EFLogicInstance, EFLogicCodeBC, EFLogicStack,
//! EFGameLogicMgr message routing, EFGameMemoryMgr game buffers, EFWorld object lookup).
//!
//! Control flow: the original wraps every state / callback / message / constructor call in `setjmp`,
//! and `ChangeState` / `Break` `longjmp` back to it. Here a logic function returns `Err(Jump)` and the
//! module code propagates it with `?`, which unwinds to the same place: the runtime call that started
//! the logic function, which then behaves as if `setjmp` had returned non-zero (result 0).
//!
//! Threading: the original is single-threaded and so is this (all of it runs inside one exclusive
//! Bevy system); re-entrancy (a message handler sending messages) goes through the explicit logic stack.
#![allow(non_snake_case)]
use std::collections::HashMap;

pub mod frame;
pub mod notify;
pub mod world;

use crate::engine::{Engine, Mem, Ptr, NULL, SELF};

/// The longjmp of `ChangeState`/`Break`: aborts the running logic function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Jump;
pub type LResult<T> = Result<T, Jump>;

/// State / callback entry of a module (`int (*)(void *)`); states are called with 0.
pub type ExtFn = fn(&mut Engine, u32) -> LResult<u64>;
/// Message handler: (message, sub-message/param, data pointer).
pub type MsgFn = fn(&mut Engine, u32, u32, Ptr) -> LResult<u64>;
/// `<module>Constructor(APIcons *api, int instanceCount)`
pub type CtorFn = fn(&mut Engine, Ptr, i32) -> LResult<()>;
/// `<module>Destructor(int instanceCount)`
pub type DtorFn = fn(&mut Engine, i32) -> LResult<()>;
/// `<module>Register(APIcons *api)`: only library modules (EF_LIBRARY_FILE) have one; called once at load.
pub type RegFn = fn(&mut Engine, Ptr);
/// `void F(int pParticleApi, undefined4 type)`: particle-type settings functions whose address one
/// module stores in data and another module calls (wisps -> GENERIC_FX)
pub type ParticleSetFn = fn(&mut Engine, u32, u32) -> LResult<()>;

/// A function whose address the original stores in game data and calls through a pointer from
/// another module. The port hands out a token instead of a code address (see [`Engine::code_ptr`]).
#[derive(Clone, Copy)]
pub enum CodeFn {
    Particle(ParticleSetFn),
    Word(fn(&mut Engine, u32) -> u32),
    Word2(fn(&mut Engine, u32, u32) -> u32),
}

impl CodeFn {
    fn addr(self) -> usize {
        match self {
            CodeFn::Particle(f) => f as usize,
            CodeFn::Word(f) => f as usize,
            CodeFn::Word2(f) => f as usize,
        }
    }
}

/// Tokens returned by [`Engine::code_ptr`] are `CODE_PTR_BASE + index`, far above any [`Mem`] address,
/// so they can never be mistaken for data (and are never dereferenced).
pub const CODE_PTR_BASE: u32 = 0xc0de_0000;

/// Static description of a game-logic module (what EFLogicCode::LoadFile finds in the .op2).
pub struct ModuleDef {
    pub name: &'static str,
    /// size of the module's .data/.bss image (module globals at module-relative 0x10000 + offset)
    pub data_size: u32,
    pub ctor: CtorFn,
    pub dtor: DtorFn,
    /// `<module>Register`, library modules only (EFLogicCodeLib::LoadFile calls it right after loading)
    pub register: Option<RegFn>,
    /// initialised data of the image: (module-relative address, words). Everything else starts zeroed.
    pub init: &'static [(u32, &'static [u32])],
    /// per-build initialised data (modules whose image differs by build, e.g. the wisps' TmpWispMap):
    /// called by `load_module` with the module data base and the build index, after `init`
    pub build_init: Option<fn(&mut Mem, Ptr, usize)>,
}

pub type CodeId = usize;
/// Handle of a game object (index into `LogicRuntime::objects`).
pub type ObjH = usize;

/// EFLogicCodeBC
pub struct LogicCode {
    pub def: &'static ModuleDef,
    /// module data image (address of module-relative 0x10000)
    pub data: Ptr,
    /// +0x1c count / +0x20 table: external functions (states + callbacks)
    pub funcs: Vec<Option<ExtFn>>,
    /// +0x18: the `<module>Register` entry (non-zero for library modules); Run marks instances of such
    /// code with flag 2, so a library object is constructed but never runs states
    pub unk18: u32,
    /// +0x24..: up to 10 registered local-data pointer slots
    pub local_slots: Vec<Ptr>,
    /// +0x4c global data, +0x5c its size, and the slots that receive it
    pub global: Ptr,
    pub global_size: u32,
    pub global_slots: Vec<Ptr>,
    /// +0x50 number of constructed instances
    pub refcount: i32,
    /// which build of the module was loaded (index into the module's generated BUILDS table; one build
    /// per level pack, they differ in per-level literals such as object ids)
    pub build: usize,
}

/// EFLogicInstance (at EFGameObject + 0x2c)
#[derive(Clone, Default)]
pub struct LogicInstance {
    /// +0xc
    pub code: Option<CodeId>,
    /// +0x10 local data (current) / +0x14 allocation / +0x18 size
    pub local: Ptr,
    pub local_alloc: Ptr,
    pub local_size: u32,
    /// +0x1c sleep timer; -1.0 = sleeps until woken
    pub sleep: f32,
    /// +0x20 current state (-1 = none)
    pub state: u32,
    /// +0x24
    pub msg_fn: Option<MsgFn>,
    /// +0x28: 1 = no logic, 2 = see LogicCode::unk18, 8 = constructed
    pub flags: u32,
}

/// The logic-relevant part of an EFGameObject. Everything visual/physical belongs to the host.
#[derive(Clone)]
pub struct GameObject {
    /// +8
    pub id: u32,
    /// +0xc; bit 8: object is ignored by every id lookup
    pub flags: u32,
    /// +0x2c
    pub inst: LogicInstance,
    /// +0x154 / +0x158 / +0x15c
    pub group_id: u32,
    pub group_mask: u32,
    pub logic_flags: u32,
    /// +0x138 (platform-maintained object flags): 0x200 freezes the object's timers, 0x60 skips
    /// the scene-hierarchy update
    pub flags138: u32,
    /// +0xbc: set by (Un)RegisterNotify for type 0x11
    pub notify11: u32,
}

/// Object lifetime events for the platform layer (e.g. to spawn/despawn Bevy entities); drained
/// by the platform, the original has no such queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldEvent {
    Added { id: u32, handle: ObjH },
    Deleted { id: u32, handle: ObjH },
}

const STACK_DEPTH: usize = 10;

/// EFLogicStack: per frame the game object, instance, code and kind (1 state, 2 callback,
/// 4 constructor, 8 destructor, 0x10 message). `top` = number of pushed frames (+0x8b0).
#[derive(Default)]
pub struct LogicStack {
    pub objects: [Option<ObjH>; STACK_DEPTH + 1],
    pub insts: [Option<ObjH>; STACK_DEPTH + 1],
    pub codes: [Option<CodeId>; STACK_DEPTH + 1],
    pub kinds: [u8; STACK_DEPTH + 1],
    pub sps: [u32; STACK_DEPTH + 1],
    pub top: usize,
}

/// EFCustomLibBank node: name (strncpy'd, 0x20 bytes) and the function table MMMalloc'd for it
pub struct LibEntry {
    pub name: [u8; 0x20],
    pub table: Ptr,
    /// +0x28: table size in words
    pub words: u32,
}

// P32 R75: sound host access to the logic runtime. The host (`Box<dyn Host>`, a sibling field of
// `Engine::rt`) reaches the logic objects (SELF resolution, group/logic-flag fields) only while an
// Engine API wrapper lends it `&mut self.rt` through this guard: the raw pointer comes from a live,
// otherwise unused `&mut` (replaces the stale `*const` from `&e.rt` that was written through while
// `&mut Engine` was live = UB).
thread_local! {
    static BOUND: std::cell::Cell<*mut LogicRuntime> = const { std::cell::Cell::new(std::ptr::null_mut()) };
}

/// lends the logic runtime to host code for the guard's lifetime (restores the previous binding)
pub struct LogicBind<'a>(*mut LogicRuntime, std::marker::PhantomData<&'a mut LogicRuntime>);

impl<'a> LogicBind<'a> {
    #[inline]
    pub fn new(rt: &'a mut LogicRuntime) -> Self {
        LogicBind(BOUND.replace(rt as *mut _), std::marker::PhantomData)
    }
}

impl Drop for LogicBind<'_> {
    #[inline]
    fn drop(&mut self) {
        BOUND.set(self.0);
    }
}

/// run `f` on the lent logic runtime (None = not lent / already in use)
pub fn with_bound<R>(f: impl FnOnce(&mut LogicRuntime) -> R) -> Option<R> {
    let p = BOUND.replace(std::ptr::null_mut());
    if p.is_null() {
        return None;
    }
    // SAFETY: `p` comes from the `&mut LogicRuntime` held by a live `LogicBind`, which nothing else
    // uses while the host runs; nulled during `f` so no second `&mut` can be made.
    let r = f(unsafe { &mut *p });
    BOUND.set(p);
    Some(r)
}

pub struct LogicRuntime {
    pub codes: Vec<LogicCode>,
    pub objects: Vec<Option<GameObject>>,
    /// EFWorld+0x4154: id -> object, the original hash table (iteration order is observable)
    pub map: world::ObjMap,
    pub stack: LogicStack,
    /// EFGameMemoryMgr: id -> (buffer, refcount)
    pub game_buffers: HashMap<u32, (Ptr, u32)>,
    /// EFCustomLibBank (0x1f720f0): libraries registered through APIRegLib::RegisterLib, newest first
    pub lib_bank: Vec<LibEntry>,
    /// latent (template) objects LaunchLatant can duplicate: id -> logic
    pub latents: HashMap<u32, Option<CodeId>>,
    /// EFGameLogicMgr+0x264: id given to the next launched latent (constructor: 0x2a800000)
    pub latent_next_id: u32,
    /// functions handed out as code pointers ([`Engine::code_ptr`]); token = CODE_PTR_BASE + index
    pub code_ptrs: Vec<CodeFn>,
    /// EFGameLogicMgr+0x14
    pub notify: notify::NotificationMgr,
    /// EFGameLogicMgr+0x250 / +0x268 / _giTimerCount
    pub timers: frame::TimerMap,
    pub next_timer_id: u32,
    pub timer_count: i32,
    /// EFGameLogicMgr+0x260: dt of the running logic update
    pub logic_dt: f32,
    /// EFGameLogicMgr+0x26c: update every object (set at start and by Terminate) instead of the
    /// unclipped list
    pub update_all: bool,
    /// EFWorld+0x4140: objects to update when not `update_all` (built by PrepareRender)
    pub logic_list: Vec<ObjH>,
    /// EFWorld+0x4150: objects queued by KillObject; the last element is the list head
    pub kill_list: Vec<ObjH>,
    /// EFWorld+0x4120: frame counter
    pub world_frame: u32,
    pub events: Vec<WorldEvent>,
    /// APILoadSave stub: (error, complete) callback code-pointer tokens (EFLoadSaveMgr_BC+0x5b8/+0x5bc)
    pub loadsave_cb: (u32, u32),
    /// WP-M1: goLoadSaveMgr + the emulated memory card (engine/core/apiloadsave.rs)
    pub loadsave: crate::engine::core::apiloadsave::McState,
}

impl Default for LogicRuntime {
    fn default() -> Self {
        LogicRuntime {
            codes: Vec::new(),
            objects: Vec::new(),
            map: world::ObjMap::default(),
            stack: LogicStack::default(),
            game_buffers: HashMap::new(),
            lib_bank: Vec::new(),
            latents: HashMap::new(),
            latent_next_id: 0x2a80_0000,
            code_ptrs: Vec::new(),
            notify: notify::NotificationMgr::default(),
            timers: frame::TimerMap::new(0x25),
            next_timer_id: 0x9000_0000,
            timer_count: 0,
            logic_dt: 0.0,
            update_all: true,
            logic_list: Vec::new(),
            kill_list: Vec::new(),
            world_frame: 0,
            events: Vec::new(),
            loadsave_cb: (0, 0),
            loadsave: Default::default(),
        }
    }
}

impl LogicRuntime {
    /// Object of the running logic (what `0x2a000000` resolves to).
    pub fn current_object(&self) -> Option<ObjH> {
        if self.stack.top == 0 { None } else { self.stack.objects[self.stack.top - 1] }
    }
    pub fn current_instance(&self) -> Option<ObjH> {
        if self.stack.top == 0 { None } else { self.stack.insts[self.stack.top - 1] }
    }
    pub fn current_code(&self) -> Option<CodeId> {
        if self.stack.top == 0 { None } else { self.stack.codes[self.stack.top - 1] }
    }
    /// Module data image of the running module (its globals live at `+ (addr - 0x10000)`).
    pub fn cur_build(&self) -> usize {
        self.codes[self.current_code().expect("no logic running")].build
    }
    pub fn cur_data(&self) -> Ptr {
        self.codes[self.current_code().expect("no logic running")].data
    }
    pub fn obj(&self, h: ObjH) -> &GameObject {
        self.objects[h].as_ref().expect("dead object")
    }
    pub fn obj_mut(&mut self, h: ObjH) -> &mut GameObject {
        self.objects[h].as_mut().expect("dead object")
    }
    pub fn inst(&mut self, h: ObjH) -> &mut LogicInstance {
        &mut self.obj_mut(h).inst
    }
    /// clears the frame about to be pushed (done when a call is refused)
    fn clear_next_frame(&mut self) {
        let t = self.stack.top;
        self.stack.objects[t] = None;
        self.stack.insts[t] = None;
        self.stack.codes[t] = None;
        self.stack.kinds[t] = 0;
    }
}

impl Engine {
    // ---------------------------------------------------------------- setup (EFLogicCode::LoadFile etc.)
    /// Load a module: allocates its data image.
    /// EFLogicCode::LoadFile (+ EFLogicCodeLib::Register for library modules)
    /// The "address" of `f` for storing in game data: the same function always gets the same token, so
    /// pointer comparisons behave like the original's.
    pub fn code_ptr(&mut self, f: CodeFn) -> u32 {
        let a = f.addr();
        let i = match self.rt.code_ptrs.iter().position(|c| c.addr() == a) {
            Some(i) => i,
            None => {
                self.rt.code_ptrs.push(f);
                self.rt.code_ptrs.len() - 1
            }
        };
        CODE_PTR_BASE + i as u32
    }

    /// The function behind a code-pointer token. A value that is not a token is a call through a
    /// wild pointer in the original (it would crash): the port panics.
    pub fn code_fn(&self, token: u32) -> CodeFn {
        match token.checked_sub(CODE_PTR_BASE).and_then(|i| self.rt.code_ptrs.get(i as usize)) {
            Some(&f) => f,
            None => panic!("call through a non-code pointer {token:#x}"),
        }
    }

    pub fn load_module(&mut self, def: &'static ModuleDef, build: usize) -> CodeId {
        let data = self.mem.malloc(def.data_size);
        for &(addr, words) in def.init {
            for (i, &w) in words.iter().enumerate() {
                self.mem.w32(data + (addr - 0x10000) + i as u32 * 4, w);
            }
        }
        if let Some(f) = def.build_init {
            f(&mut self.mem, data, build);
        }
        self.rt.codes.push(LogicCode {
            def,
            data,
            funcs: Vec::new(),
            unk18: def.register.is_some() as u32,
            local_slots: Vec::new(),
            global: NULL,
            global_size: 0,
            global_slots: Vec::new(),
            refcount: 0,
            build,
        });
        let c = self.rt.codes.len() - 1;
        if let Some(r) = def.register {
            // EFLogicCodeLib::Register: FlushCache(0); Register(0x1f71630). Not inside any logic frame.
            r(self, crate::engine::core::apicons::API_CONS);
        }
        c
    }

    /// Add a game object with the given logic (None = no logic). Not constructed until its first Run.
    pub fn add_object(&mut self, id: u32, code: Option<CodeId>) -> ObjH {
        let o = GameObject {
            id,
            flags: 0,
            inst: LogicInstance { code, state: u32::MAX, ..Default::default() },
            group_id: 0,
            group_mask: 1, // EFGameObject ctor: +0x158 = 1
            logic_flags: 2, // EFGameObject ctor: +0x15c = 2
            flags138: 0,
            notify11: 0,
        };
        self.rt.objects.push(Some(o));
        let h = self.rt.objects.len() - 1;
        self.rt.map.insert(id, h);
        self.rt.events.push(WorldEvent::Added { id, handle: h });
        h
    }

    /// EFWorld::GetGameObject
    pub fn get_game_object(&mut self, id: u32, retriever: Ptr) -> Option<ObjH> {
        let h = if id == SELF {
            self.rt.current_object()
        } else if id & 0xff00_0000 == 0xff00_0000 {
            // the original calls through the retriever without checking GetRetriever's result
            self.retriever_get_object(id, retriever)
                .unwrap_or_else(|| panic!("GetGameObject: no retriever {id:#x}"))
        } else {
            self.rt.map.find(id)
        };
        let h = h?;
        if self.rt.objects[h].as_ref()?.flags & 8 != 0 { None } else { Some(h) }
    }

    // ---------------------------------------------------------------- EFLogicInstance
    /// EFLogicInstance::PushLogic
    pub fn PushLogic(&mut self, h: ObjH, kind: u8) {
        let t = self.rt.stack.top;
        assert!(t < STACK_DEPTH, "logic stack overflow");
        self.rt.stack.insts[t] = Some(h);
        self.rt.stack.codes[t] = self.rt.obj(h).inst.code;
        self.rt.stack.kinds[t] = kind;
        self.rt.stack.sps[t] = self.mem.sp();
        self.rt.stack.top = t + 1;
        self.SetCurrent(h);
    }
    /// EFLogicInstance::PopLogic (also releases the C stack of the finished call)
    pub fn PopLogic(&mut self) {
        self.rt.stack.top -= 1;
        let t = self.rt.stack.top;
        self.mem.set_sp(self.rt.stack.sps[t]);
        if t != 0 {
            if let Some(h) = self.rt.stack.insts[t - 1] {
                self.SetCurrent(h);
            }
        }
    }
    /// EFLogicInstance::SetCurrent: point every registered local-data slot of the code at this instance's data
    pub fn SetCurrent(&mut self, h: ObjH) {
        let inst = &self.rt.obj(h).inst;
        if inst.local == NULL {
            return;
        }
        let local = inst.local;
        if let Some(c) = inst.code {
            for i in 0..self.rt.codes[c].local_slots.len() {
                let slot = self.rt.codes[c].local_slots[i];
                self.mem.w32(slot, local);
            }
        }
    }
    /// EFLogicInstance::InitLocalData
    pub fn InitLocalData(&mut self, h: ObjH, slot: Ptr, size: u32) {
        if size != 0 {
            let inst = &self.rt.obj(h).inst;
            let (alloc, old) = (inst.local_alloc, inst.local_size);
            if alloc == NULL {
                let p = self.mem.malloc(size);
                let i = self.rt.inst(h);
                i.local_alloc = p;
                i.local = p;
            } else if (old as i32) < size as i32 {
                let p = self.mem.realloc(alloc, size);
                self.mem.memset(p + old, 0, size - old);
                let i = self.rt.inst(h);
                i.local_alloc = p;
                i.local = p;
            }
        }
        if let Some(c) = self.rt.obj(h).inst.code {
            let slots = &mut self.rt.codes[c].local_slots;
            if !slots.contains(&slot) && slots.len() < 10 {
                slots.push(slot);
            }
        }
        self.SetCurrent(h);
        self.rt.inst(h).local_size = size;
    }
    /// EFLogicCodeBC::InitGlobalData
    pub fn InitGlobalData(&mut self, c: CodeId, slot: Ptr, size: u32) {
        if size == 0 {
            return;
        }
        let (g, old) = (self.rt.codes[c].global, self.rt.codes[c].global_size);
        if g == NULL {
            self.rt.codes[c].global = self.mem.malloc(size);
        } else if old < size {
            let p = self.mem.realloc(g, size);
            self.mem.memset(p + old, 0, size - old);
            self.rt.codes[c].global = p;
        }
        if !self.rt.codes[c].global_slots.contains(&slot) {
            self.rt.codes[c].global_slots.push(slot);
        }
        let g = self.rt.codes[c].global;
        for i in 0..self.rt.codes[c].global_slots.len() {
            let s = self.rt.codes[c].global_slots[i];
            self.mem.w32(s, g);
        }
        self.rt.codes[c].global_size = size;
    }
    /// EFLogicCodeBC::GetFunction
    pub fn GetFunction(&self, c: CodeId, idx: u32) -> Option<ExtFn> {
        self.rt.codes[c].funcs.get(idx as usize).copied().flatten()
    }

    /// EFLogicInstance::Constructor
    pub fn Constructor(&mut self, h: ObjH) {
        let i = self.rt.inst(h);
        i.flags = (i.flags & !1) | 8;
        let Some(c) = i.code else {
            i.flags |= 1;
            self.rt.clear_next_frame();
            return;
        };
        self.PushLogic(h, 4);
        self.rt.codes[c].refcount += 1;
        let n = self.rt.codes[c].refcount;
        let ctor = self.rt.codes[c].def.ctor;
        let _ = ctor(self, crate::engine::core::apicons::API_CONS, n);
        self.PopLogic();
    }
    /// EFLogicInstance::Destructor
    pub fn Destructor(&mut self, h: ObjH) {
        let Some(c) = self.rt.obj(h).inst.code else {
            self.rt.clear_next_frame();
            return;
        };
        self.PushLogic(h, 8);
        self.rt.codes[c].refcount -= 1;
        let n = self.rt.codes[c].refcount;
        self.rt.inst(h).flags &= !8;
        let dtor = self.rt.codes[c].def.dtor;
        let _ = dtor(self, n);
        self.PopLogic();
    }
    /// EFLogicInstance::PreRun
    pub fn PreRun(&mut self, h: ObjH, dt: f32) -> bool {
        let i = self.rt.inst(h);
        let s = i.sleep;
        if s == -1.0 {
            return false;
        }
        if 0.0 < s {
            i.sleep = s - dt;
        }
        if i.sleep <= 0.0 {
            i.sleep = 0.0;
            return true;
        }
        false
    }
    /// EFLogicInstance::Run (the caller has put the object in the next stack frame)
    pub fn Run(&mut self, h: ObjH, dt: f32) {
        let flags = self.rt.obj(h).inst.flags;
        if flags & 8 == 0 {
            self.Constructor(h);
            if let Some(c) = self.rt.obj(h).inst.code {
                if self.rt.codes[c].unk18 != 0 {
                    self.rt.inst(h).flags |= 2;
                }
            }
        } else if flags & 7 == 0 && self.PreRun(h, dt) && self.rt.obj(h).inst.state != u32::MAX {
            self.PushLogic(h, 1);
            let inst = &self.rt.obj(h).inst;
            if let Some(f) = self.GetFunction(inst.code.unwrap(), inst.state) {
                let _ = f(self, 0);
            }
            self.PopLogic();
        } else {
            self.rt.clear_next_frame();
        }
    }
    /// EFLogicInstance::SendMessage
    pub fn SendMessage(&mut self, h: ObjH, msg: u32, sub: u32, data: Ptr) -> u64 {
        if self.rt.obj(h).inst.flags & 1 != 0 {
            self.rt.clear_next_frame();
            return 0;
        }
        self.PushLogic(h, 0x10);
        let r = match self.rt.obj(h).inst.msg_fn {
            Some(f) => f(self, msg, sub, data).unwrap_or(0),
            None => 0,
        };
        self.PopLogic();
        r
    }
    /// EFLogicInstance::CallCB
    pub fn CallCB(&mut self, h: ObjH, idx: u32, arg: u32) -> u64 {
        if self.rt.obj(h).inst.flags & 1 != 0 {
            self.rt.clear_next_frame();
            return 0;
        }
        self.PushLogic(h, 2);
        let c = self.rt.obj(h).inst.code.unwrap();
        // the original calls the table entry without a NULL check
        let f = self.GetFunction(c, idx).unwrap_or_else(|| panic!("CallCB: no function {idx:#x}"));
        let r = f(self, arg).unwrap_or(0);
        self.PopLogic();
        r
    }

    // ---------------------------------------------------------------- EFGameLogicMgr entry points
    /// Run one object's logic for this frame.
    pub fn run_object(&mut self, h: ObjH, dt: f32) {
        let t = self.rt.stack.top;
        self.rt.stack.objects[t] = Some(h);
        self.Run(h, dt);
    }
    /// Deliver an engine callback (timer, input, animation, physics ...) to an object's logic.
    pub fn call_cb(&mut self, h: ObjH, idx: u32, arg: u32) -> u64 {
        let t = self.rt.stack.top;
        self.rt.stack.objects[t] = Some(h);
        self.CallCB(h, idx, arg)
    }
    /// Remove an object immediately the way the kill list does (RemoveGameObject, Terminate, delete):
    /// for level unloading by the platform layer. Logic code uses APIgameobj::KillObject.
    pub fn remove_object(&mut self, h: ObjH) {
        let id = self.rt.obj(h).id;
        self.RemoveGameObject(id);
        self.Terminate(h);
        self.delete_object(h);
    }
    /// EFGameLogicMgr::SendLogicMessageTo: returns the handler's result (single object) or the number
    /// of objects reached (retriever ids 0xffxxxxxx).
    pub fn SendLogicMessageTo(&mut self, id: u32, msg: u32, sub: u32, data: Ptr, retriever: Ptr) -> i32 {
        if id & 0xff00_0000 == 0xff00_0000 {
            let mut params = retriever;
            if params == NULL {
                // auStack_20: a cursor at "start", the other words uninitialised
                params = self.mem.alloca(0x20);
                self.mem.w32(params, world::CURSOR_START);
            }
            let mut n = 0;
            let Some(mut o) = self.retriever_get_object(id, params) else { return 0 };
            while let Some(h) = o {
                let t = self.rt.stack.top;
                self.rt.stack.objects[t] = Some(h);
                self.SendMessage(h, msg, sub, data);
                n += 1;
                o = self.retriever_get_object(id, params).unwrap();
            }
            n
        } else {
            match self.get_game_object(id, NULL) {
                None => 0,
                Some(h) => {
                    let t = self.rt.stack.top;
                    self.rt.stack.objects[t] = Some(h);
                    self.SendMessage(h, msg, sub, data) as i32
                }
            }
        }
    }
    /// EFGameMemoryMgr::GetGameBuffer: buffers are shared by id across all modules; refcounted.
    pub fn GetGameBufferMgr(&mut self, id: u32, size: u32, _flag: u32) -> Ptr {
        if let Some((p, rc)) = self.rt.game_buffers.get_mut(&id) {
            *rc += 1;
            return *p;
        }
        let p = self.mem.malloc(size);
        self.rt.game_buffers.insert(id, (p, 1));
        p
    }
}

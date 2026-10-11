# Rust port working notes (book / Tome of Doom first)

Source of truth: `Decompiled Source/.gamedb/index.sqlite` (+ the `.c` files it indexes).
Tools: `tools/rustport/{apisig,variants,shapes,disasm,statics}.py`
(`disasm.py <module> <fn>` = capstone MIPS disasm of the carved .o, use to settle Ghidra float/f0 ambiguities).

## Modules
- `Game Logic Modules/Globals/Objects/Tome_Of_Doom/tomedoom__<sha8>/` : 30 builds (one per level P2R), all 65 fns
  (TomeDoom.c 50, Tod_Sucking.c 9, wisp_tomeCollect.c 6). Reference build: `tomedoom__802a33e5` (EP1L01).
  Builds differ only by constants (object IDs, e.g. which of 0x2a000091/0x2a000092 is checked first in
  SetMasterParams; DrainMonsterLife 1276 vs 1268 bytes in 288c5fd9). -> per-build constant table.
- Module = ELF32 relocatable MIPS R5900, .text loaded at 0x00010000; entry `tomedoomConstructor/Destructor`,
  looked up by `EFLogicCode::LoadFile` @0x00211c20.

## tomedoom data layout (module globals, `_globals.h`)
0x10000.. API table pointers pAnim,pGame,pGameObj,pInput,pMath,pParticle,pPhysic,pExtra,pLight,pSound,
pGraphics,pStdlib, pAnimMngr(LIB),pDebug,pFunctions(LIBApiCFunctions),pText(LIBApiText),pMiscFunc.
bCollision 0x10044; pLocalDataTome 0x10050; pGlobalDataTome 0x10054; ui32JointHookMasterID 0x10058;
ui32JointHookSlaveID 0x1005c; cvVelocity vec3 0x10060; cvMove vec3 0x10070; cvOldSlaveVelo vec3 0x10080;
cvOldMasterVelo vec3 0x10090; cvJoyDep vec3 0x100a0; cvWallNormal vec3 0x100b0; pcmTomeHook_105 (DTMatrix*).

## Ghidra fixes found with disasm.py
- JoyRotate: real code is `angle = acosf(dot) * (GetFrameDeltaTime() * 5.5)` (Ghidra printed acos*acos*5.5).
- FixPourNePasTomberDansLeMur: `in_f0` = return value of GetFrameDeltaTime() (dt).
- CalcNewDMs: `v.c *= 1.0 / GetFrameDeltaTime()` per component (Ghidra lost the dt results).
- GetTomeVelocity: copies all of cvJoyDep and uses its .y (Ghidra printed `R.y * 0.0` and y = 0 - R.y*d).

## Engine logic runtime (EXE.1 Classes/EF/EFLogicInstance.cpp) -> recomp/src/logic/instance.rs
EFLogicInstance fields: +0xc EFLogicCode* code; +0x10 local data ptr (current); +0x14 local alloc; +0x18 local size;
+0x1c sleep timer f32 (ctor 0); +0x20 state index (ctor -1); +0x24 message fn; +0x28 flags (ctor 0).
flags: bit0 = no code / disabled (set by Constructor when code==0); bit3 = constructed (set at Constructor start,
cleared just before calling module destructor); Run requires (flags & 7)==0; Run sets bit1 if code+0x18 != 0 after ctor.
- Constructor: flags &= ~1; flags |= 8; if code==0 {flags|=1} else {PushLogic(this,4); code.refcount(+0x50)++;
  call module ctor(0x1f71630 /*APIcons table*/, refcount) under setjmp; PopLogic}.
- Destructor: PushLogic(this,8); refcount--; flags &= ~8; call module dtor(refcount) under setjmp; PopLogic.
- Run(dt): if !(flags&8) {Constructor; if code+0x18 {flags|=2}} else if (flags&7)==0 && PreRun(dt) && state!=-1
  {PushLogic(this,1); fn = code.GetFunction(state); if fn: setjmp; fn(0); PopLogic; PostRun()}.
  (Run does NOT run a state on the frame it constructs.)
- SendMessage(inst,msg,data): if !(flags&1) {PushLogic(inst,0x10); r = msgfn ? msgfn(msg,data) under setjmp : 0;
  PopLogic} -> r (0 if longjmp'd).
- ChangeState(s): state=s; longjmp(-1) = abort the current state/cb/msg call (caller sees 0 / returns normally).
- SetNextState(s): state = s (no jump). Break(): longjmp without state change.
- CallCB(inst, idx): if !(flags&1) {PushLogic(inst,2); fn=code.GetFunction(idx); r = setjmp? 0 : fn(); PopLogic}.
- PreRun(dt): sleep=+0x1c; if sleep == -1.0 -> 0 (sleeping forever); if sleep>0 sleep-=dt; if sleep<=0 {sleep=0; ...
- PushLogic/PopLogic: logic stack (current instance, its code, kind 1=state 2=cb 4=ctor 8=dtor 0x10=msg);
  SetCurrent writes the instance's local-data ptr into every registered local-data pointer slot of the code
  (= how `pLocalDataTome` changes per instance: module globals are SHARED by all instances of a module).
- InitLocalData(inst, slotptr, size): alloc/realloc zero-filled local data, register slot, SetCurrent.

## Engine API glue (EXE.1 "Engine API (module interface)"/EFApi_*.cpp)
- SetExternalFunction(i, fn) -> code(top of logic stack).SetFunction(i, fn)  (function table shared per module code).
- SetMessageFunction(fn) -> current instance msg fn. GetLocalData(&slot, size) / GetGlobalData(&slot, size):
  only when stack kind has 4 (constructor) or 0x80 -> InitLocalData / code.InitGlobalData (global = per module code,
  shared by all instances). SetInitialState(s): only in constructor -> SetNextState(s).
- GetApiCustom(name) -> EFCustomLibBank::GetLib(name)  ("AnimMngr","debug","ApiCFunctions","ApiText","MiscFunc").
- gameobj APIs take an object id; EFWorld::GetGameObject(id, retriever) -> object; its EFLogicInstance is at obj+0x2c.
  GetCurrentStateID(id) = instance.state (obj+0x4c) or 0xffffffff if no object. SetNextState(id,s), Sleep(id,t) likewise.
  ChangeState(s) (APIgameobj[0]) = CURRENT instance ChangeState -> longjmp (never returns).
  Break() = longjmp. GetGameBuffer(id,size,flag) -> EFGameMemoryMgr::GetGameBuffer (shared across modules by id!).
  LaunchLatant(id) -> EFGameLogicMgr::LaunchLatant. SendMessageTo(id, retr, msg, sub, data) ->
  EFGameLogicMgr::SendLogicMessageTo(mgr, id, msg, sub, data, retr).
- Object ids: 0x2a000000 = self ("this object"); others are per-level ids (0x2a00008f camera, 0x2a000091/92 heroes,
  0x2a000096, 0x2a100006 ...) -> differ per build -> per-build constants.

## tomedoom constructor/destructor (TomeDoom.c 1907-1995)
ctor(api, instanceCount): if instanceCount==1 { SetExternalFunction: 0 ST_FIRSTMOVE,1 ST_BASICMOVE,2 ST_ISOPENING,
3 ST_OPEN,4 ST_FINALSUCKING,5 ST_ISCLOSING,6 CB_ANIM_LF,7 CB_DMSTATECHANGE,0xb CB_INPUT_STICK,0xc CB_INPUT_IDLE,
0xf CB_INPUT_ROTATE,0x10 CB_INPUT_X,0x11 CB_INPUT_CIRCLE,0x12 CB_INPUT_TRIANGLE,0x13 CB_TIMER,0x14 CB_ANIM,
0x15 CB_POSTLOGIC,0x16 CB_CUTSCENE,0x17 CB_PHYSIC; fetch APIs (custom libs by name); GetGlobalData(&pGlobalDataTome,0xc);
G[0]=GetGameBuffer(0,0x8f8,1); G[1]=GetGameBuffer(1,0x68,1); G[2]=GetGameBuffer(2,0x158,1) }
then always: SetMessageFunction(MessageHandler); GetLocalData(&pLocalDataTome,0x254); RegisterNotify(self,-1,0x15,0x15);
MiscFunc_ChangeClipping(self,2,NULL,1); Create_Input(); Create_Physic(L); Create_GameObject(L); Create_Particles(L);
InitColorLight(L); SetInitialState(0).
dtor(n): TimerDestroy L+0x34..0x4c (7); if L+0x2c DestroyLight; EmitterDestroy L+0x218[6]; TypeDestroy L+0x200[6].
Game buffer 0 (0x8f8 bytes) = shared hero/game state: +8 current hero id, +0xc other hero id, +0x10 input config,
+0x30 captured monster id, +0x34 flags. Buffer 1 (0x68): +4 = book power (float). 

## Rust architecture (built, compiles)
- `src/engine/mem.rs` Mem: LE 32-bit space, base 0x100000; 256KB scratch stack for address-taken C locals
  (`e.alloca(n)`, released by PopLogic); heap malloc/free/realloc (zeroed); `e.cstr("lit")` interns C string literals.
  Engine derefs to Mem: `e.r32(a)`, `e.w32(a,v)`, `e.rf/wf`, `e.rv3/wv3`.
- `src/engine/mod.rs` Engine { mem, rt: LogicRuntime, host: Box<dyn Host> }; SELF = 0x2a000000.
- `src/engine/api_gen.rs` GENERATED by `python tools/rustport/gen_api.py` (re-run after adding core/lib fns):
  trait per table (APIgameobj, LIBAnimMngr, ...) with original slot names; module code calls
  `APIgameobj::SetGroupID(e, SELF, 6)`. Slot implemented by `src/engine/core/<table lower>.rs` `pub fn Name(e: &mut Engine, ..)`
  or `src/libs/<snake lib>.rs` (LIB tables) if such fn exists (signature copied), else forwarded to
  `Host::<Table>_<Name>(&mut mem, ..)` (default: trace log + return 0). Float returns detected from module call sites
  (GetFrameDeltaTime, TimerGetTime, GetTime, GetDuration, InputActionGetAnalogX/Y, MiscFunc_Sin, GetDistance, GetVelocity ...).
- `src/logic/mod.rs` runtime port: LResult<T>=Result<T,Jump> (ChangeState/Break longjmp), ExtFn = fn(&mut Engine,u32)->LResult<u64>,
  MsgFn = fn(&mut Engine,msg,sub,data)->LResult<u64>, ModuleDef{name,data_size,ctor,dtor}; Engine::{load_module, add_object,
  run_object, call_cb, remove_object, SendLogicMessageTo, get_game_object, Push/PopLogic, SetCurrent, InitLocalData, InitGlobalData,
  Constructor, Destructor, PreRun, Run, SendMessage, CallCB}. Module globals: `e.rt.cur_data() + (addr - 0x10000)`.
- Logic stack (EFLogicStack @_gpoLogicStack): objects[] +0, insts[] +0x28, codes[] +0x50, kinds[] +0x78 (bytes), top +0x8b0;
  self = objects[top-1]; SendLogicMessageTo writes objects[top] then SendMessage pushes. SendMessage(inst,msg,sub,data) real
  signature has 4 args (Ghidra dropped a3; confirmed by exedis.py: a3 saved in s1). Refused calls clear frame [top].
- GetGameObject: SELF -> objects[top-1]; 0xff...... -> retriever (Host::retrieve_objects); else id map; result ignored if obj+0xc & 8.
- Core APIgameobj: ChangeState, SetNextState, Sleep, Break, LaunchLatant, GetGameBuffer, GetCurrentStateID, SendMessageTo, GetID.
- Tools: showfn.py <path under Decompiled Source> <fn..> prints bodies; exedis.py <start> <end> disassembles EXE.1.

## Per-build variation (30 tomedoom builds = one per level)
- `python tools/rustport/gen_builds.py "<family dir>" tomedoom__802a33e5 recomp/src/modules/tomedoom/builds.rs --report`
  generates `builds.rs`: `BUILDS[i] = Build{sha8, g:[shape group per multi-shape fn], k:[varying literals]}`.
  Object ids consistent per build -> `ID_<refhex>` (ID_2a00008f camera, ID_2a000093, ID_2a000096, ID_2a100006,
  ID_2a800095 latent, ID_2a800139 latent). Hero ids 0x2a000091/92 swap per build -> per-site `FUNC_<fn>_<grp>_<n>`.
  Module code: `k(e, NAME)` (literal), `g(e, G_<Fn>)` (shape group). Build chosen at load: LogicCode.build.
- Real shape variants (port each group): UpdatePosition g1 17ce1a2c (after 2nd GetPosition: `if L+0x60 == 0x2a0000bc
  {L+0x114 += 0.4}`) / g2 5e71b643 (`if GetGroupBitMask(L+0x60) & 0x20 {GetPosition(L+0x60, 6, L+0x110)}`); SET_CAPTURE g1 4a201eb9 (no Create_Light call); CB_ANIM_LF g1 0a8f4fed, g2 36e3927c;
  CB_ANIM g1 0a8f4fed; Create_GameObject g1 2fb4d6b9 (+RegisterNotify(self,-1,0x16,0x16)).
  Other shape diffs are Ghidra display artifacts (cvMove._0_4_ vs cvMove) normalised away by gen_builds.py.
- Rust conventions: Engine has inherent r32/w32/rf/wf/rv3/wv3/alloca/cstr (two-phase borrow friendly);
  API calls in UFCS form `APIgameobj::SetGroupID(e, SELF, 6)` (several traits share method names); bind
  computed args to locals first. Address-taken C locals -> `e.alloca(size)` at function entry.

## Progress (update as functions land)
- tome_doom.rs (50), tod_sucking.rs (9), wisp_tome_collect.rs (6, GENERATED by tools/rustport/gen_wisp_collect.py):
  all 65 tomedoom fns written. Next: engine/core/apimath.rs (CMatGetPos etc. pure fns),
  libs (AnimMngr PlayAnim, MiscFunc_ChangeClipping), then `pub mod tomedoom;` in modules/mod.rs and cargo build.
- Helpers in tomedoom/mod.rs: G(e,addr) global addr, L(e) local data, GD(e) global data, GB0(e) game buffer 0,
  k(e, IDX) per-build literal, g(e, G_FN) shape group. ALL = 0x7fffffff.
- Object map + retrievers: src/logic/world.rs ports EFWorld's DTMap (37 buckets, key % 37 as int, new nodes
  prepended) so iteration order matches; retriever cursor = ObjectRetrieverParams[0] (1 = start, node ref,
  0x7ffffffe = done). All(0x01)/GroupID(0x08, params[1]==obj+0x154)/GroupMask(0x10, params[1]&obj+0x158) ported;
  nearest/in-range/parent/children -> Host::retriever_get_object. SendLogicMessageTo walks the retriever.

## Tooling gotchas
- Bash tool silently truncates very long commands (~9KB): a heredoc append got cut mid-function. For big
  code chunks use the Write tool to a temp file (recomp/.chunk.rs) then `cat >> target`.
- variants.py usage: `variants.py "<family dir>" <fn> tomedoom__<sha8A> tomedoom__<sha8B>`; disasm.py takes the full
  module name `tomedoom__802a33e5`.

## Step 3 progress (engine math + custom libs)
- DONE src/engine/core/apimath.rs: all 60 APImath slots (EFApi_math.cpp). src/engine/core/rwmath.rs: RwV3dLength/Normalize,
  RwMatrixCreate/Destroy/RotateOneMinusCosineSine/Rotate/Multiply/Invert/MatrixInvertGeneric/Scale, RwV3dTransformPoints/Vectors.
  Facts: VU0 combine 2 = post-concat M*R (row vectors); RW_DEG_TO_RAD=DAT_00343088=0.017453292; CMatInvert(dst,src) clears
  src flags -> always generic; CVectorGetDirectionAndDistanceTo returns length (f0); CMatCopy(src,dst); PS2 FPU sums (x+y)+z.
  Engine fields added: work_matrix, rw_matrix_module_flags (0x20000 assumed), lib_local (HashMap lib->Ptr), libs_registered.
  Host::rw_error(code) added via gen_api.py template.
- LIBRARY MODULES (redone properly): a lib is a ModuleDef with `register: Some(<name>Register)` and `init` (initialised
  image words). `Engine::load_module` copies init, sets code.unk18=1 and calls Register (EFLogicCodeLib::LoadFile).
  The lib's object must be added to the world (add_object with the lib code) and run once: that runs the lib
  Constructor (GetLocalData into the lib's own pLocalData slot), and flag 2 means it never runs states.
  Lib fns are plain calls in the CALLER's logic context but use the lib's globals via `libs::lib_data(e, &MODULE, addr)`.
  EFCustomLibBank = `rt.lib_bank` (newest first) + core/apireglib.rs RegisterLib/UnRegisterLib (malloc table),
  GetApiCustom = bank lookup (strncmp 0x20). Engine.lib_local/libs_registered and rt.libs were removed.
- DONE src/libs/anim_mngr.rs (all 9 fns) and src/libs/misc_func.rs (all 19 fns; preCalcSin table from the .o image,
  same in both builds). Module images: the P2R .o has NO .data section; globals at 0x10000.. live at the start of .text.
  MiscFunc_EmitterUpdateFlags asserts the table arg is pParticle (all shipped callers).
- Lib file names = snake of the table name minus LIB: anim_mngr, api_c_functions, api_text, misc_func (gen_api.py core_fns regex:
  `pub fn Name(e: &mut Engine, ...)`). Re-run `python ../tools/rustport/gen_api.py` from recomp/ after adding fns.
- All lib builds are identical code (only headers/labels differ): port one build each.
- DONE src/libs/api_c_functions.rs (26 fns; 6 builds differ ONLY by the fade-message object id ->
  `BUILDS` table + `libs::lib_build(e, &MODULE)`: lib fns run in the CALLER's context so never use
  rt.cur_build() inside a lib). DONE src/libs/api_text.rs (13 fns). Step 3 complete; crate builds with 0 warnings.
- variants.py/shapes.py accept a glob as root: `variants.py "<...>/Lib/apicfunc__*"`.
- Ghidra folds run-time float products into constants (CheckPositionOrientationPoint: bounds are
  (-PI/8)*3.0 etc. computed with mul.s). When float constants look like products, check disasm.py.
- Libs are unoptimised (-O0, $fp frames, no madd.s): evaluate float expressions in source order.

## Known, deliberate deviations (document in the final report)
- f32 arithmetic is IEEE round-to-nearest with denormals. The EE FPU rounds toward zero, flushes
  denormals, and saturates instead of producing Inf/NaN, so results can differ in the last ulp (and
  at overflow). Emulating that would mean soft-float on every op; not done, for speed.
- DONE src/modules/tod_beam/ (17 fns, builds.rs via gen_builds.py ref tod_beam__01ad949b; only literals differ:
  book id ID_2a000012 + anim indices). Stored code pointers (state fn at L+0x18) = module-relative fn address,
  dispatched by `call_state` match. tests/smoke.rs tod_beam_open_hook_close covers the full sequence.
- tests/smoke.rs: TestHost (identity matrices per (obj,bone), KillObject log), PROBE logic module recording messages.

## Step 4: wisps (orbs that refill the book) -- wisps + generic_fx DONE, tested
- Family: `Game Logic Modules/Globals/Collectables/Wisp/l*_wisps*` (21 builds, ref l01_wisps; 59 fns:
  wisp_base.c 45 + initwisp.c 1 + GlowSettings 3 + SparksSettings 3 + touchCollect 6 + L01_Wisps.c ctor/dtor).
  wispsourcenew__* (16 builds, 29 fns, WispSourceNew.c) NOT started yet.
- Per build: TmpWispMap @0x10038 = N*0x24 bytes initialised (recomp/src/modules/wisps/maps.rs, generated by
  `python tools/rustport/gen_wisp_maps.py`); globals after it at X = align8(0x10038 + N*0x24):
  uiToStatusPointerMapWisp X, pLocalDataWisp_base X+4, pGlobalDataWisp_base X+8. Global data layout/size per
  build = k literals in wisps/builds.rs (gen_builds ref l01_wisps). Shape groups: InitWispData g1 = no loop
  (N=0 builds), Init_LocalSetting g1 = only record 0 (N=1 builds).
- Extra per-level fns: l03a_wisps__44cbb300 (L03A_WispsFUNC_MyMessageFunction msg 0xe7 -> clear 0x10000000 +
  Inactive_In; Wisp_baseCB_CUT_SCENE; ctor adds SetExternalFunction(3,CB_CUT_SCENE) if n==1,
  SetMessageFunction(L03A..), RegisterNotify(self,1,0x16,3)); l04_wisps__ed7e7a13 (Wisp_L04FUNC_MyMessageFunction:
  msg 0xc9 kills the 0x10 launched wisps at GD+0x26c, GD+0x10=0; ctor SetMessageFunction). Others: ctor/dtor just
  call wisp_baseConstructor/Destructor. -> to write in wisps/level.rs (wisps_levelConstructor/Destructor dispatch
  on BUILDS[b].sha8).
- ModuleDef got `build_init: Option<fn(&mut Mem, Ptr, usize)>` (per-build image init; all other defs = None).
- Cross-module code pointers: `Engine::code_ptr(CodeFn::Particle(f)) -> token` (CODE_PTR_BASE 0xc0de0000 + idx,
  deduped by fn address) and `Engine::code_fn(token)`; used for the GENERIC_FX descriptor's particle-settings fn
  pointers (wisp GD + desc(+0x13c in l01) entries of 0xac bytes, fn at entry+0x80 i.e. desc+0x84+i*0xac).
- tools/rustport/gen_particle_settings.py <src.c> <out.rs> <count> (replaces gen_wisp_collect.py) generated
  wisps/wisp_{glow_settings,sparks_settings,touch_collect}.rs and tomedoom/wisp_tome_collect.rs.
- Book power refill = Wisp_baseState_Picked_Execute: power (game buffer 1 +4) += 18 small(1)/36 medium(0,2)/54 large(3),
  cap 180; msg 0x122(size) to the book (ID_2a000025) when sucked (+0xb8), sound Play(book,0x17,0), msg 0x1d to ID_2a100006.
- GENERIC_FX (`Game Logic Modules/Globals/Objects/GENERIC_FX/generic_fx`, 1 build, 13 fns, 280 lines) receives
  0x16f(desc)/0x171/0x170 and calls the descriptor fn pointers -> port it next (needed for collect sparkles).
- DONE: wisps/{wisp_base,level,builds,maps,wisp_*_settings,wisp_touch_collect}.rs + modules/generic_fx.rs (13 fns).
  tests/smoke.rs: wisp_touch_refills_book_power, wisp_sucked_by_book_caps_power (TestHost has time/sounds/GetPosition=origin;
  WISP_PROBE module records (receiver id, msg, sub); GENERIC_FX registered via e.rt.latents.insert(fx_id, Some(code))).
  All 6 smoke tests pass (cargo test), 0 warnings.
- DONE wispsourcenew (15 builds, 29 fns, mod.rs + builds.rs + particle_settings.rs; verified line by line vs the C).
  Test huge_wisp_source_releases_and_replaces (HW_PROBE latent). 7 smoke tests pass.
- NEXT: Bevy plugin (src/plugin.rs is still empty!), then step 5 hero/monster book parts.

## Engine frame / notifications / timers (read from EXE.1, for the runtime + Bevy plugin)
- MainLoop (EXE.1 Classes/EF/EFEngineController.cpp:300-460): ComputeDeltaTime: PrevFrameTimes[10] (0x2ad630, .data
  init 0.02 each), cur=0. Per frame: PrevFrameTimes[cur]=raw frame time; (sync fix if ctrl+0x29c&0x20: video sync, NOT
  ported); clamp that entry: if 0.05<x ->0.05; if x<0.0001 ->0.0001; dt = sequential f32 sum a0..a9 / 10.0; cur=(cur+1)%10;
  ctrl+0x2a4 += dt (total); ctrl+0x2b8 = dt (= GetFrameDeltaTime, EFApi_Game.cpp:178). Order: input(dt);
  EFGameLogicMgr::Update(dt); EFCutScenePlayer::Update; EFWorld::Update(dt); palette; motion trails; Render; audio.
- EFGameLogicMgr ctor (EFGameLogicMgr.cpp:13): +0x264 latent next id = 0x2a800000; +0x268 next timer id = 0x90000000;
  +0x26c update_all = 1 (Terminate:468 sets 1 again); CreateTimer() = global timer 0x90000000 (owner NULL, period 0).
  All DTMaps 0x25 buckets (latents, codes, timers +0x250, notification maps).
- Update(dt) @0x1fcf70: +0x260=dt; NotifyAll(0x14,0); timers in DTMap order: owner=t+0x10; if (owner==0 ||
  ((owner+0x138&0x200)==0 && (owner+0x15c&0x40)==0)) && t+0x14!=1 { t+0x1c += dt; if 0<t+0x18 && t+0x18<=t+0x1c
  { t+0x1c -= t+0x18; EFInstance::Notify(t, 7, data=t.id, 0) } }; if update_all==0 && world+0x4140 list non-empty:
  walk it (next=obj+0x160) UpdateLogic; else 2 passes over world map order (EFAllRetriever): logic_flags&0x4000 first,
  then the rest; update_all=0. NotifyAll(0x15,0).
- UpdateLogic: obj+0xc&8==0 -> stack.objects[top]=obj; Run(dt) else DestroyObject: if obj+0x28 (EFPhysic*)==0:
  RemoveGameObject(id); vtable+0x54 Terminate; delete. (+0xc|=8 only by EFCutScenePlayer.cpp:290.)
- ExecLogicCallback @0x1fd300 (fn of every logic registration): data {obj*, cb}: stack.objects[top]=obj;
  return CallCB(obj+0x2c, cb, payload) (disasm-verified; result passes through).
- LaunchLogicNow = stack.objects[top]=obj; Constructor ONLY (not Run). LaunchLatant: id=+0x264++.
  -> FIX port: core/apigameobj.rs LaunchLatant uses run_object(h,0.0); rt.latent_next_id default must be 0x2a800000.
- Timers (EFGameTimer 0x20: +8 id, +0x10 owner obj, +0x14 paused, +0x18 period, +0x1c elapsed), EFApi_Extra.cpp:20-180:
  TimerCreate: _giTimerCount++; owner=stack.objects[top-1]; id=CreateTimer (insert/replace in map, +0x268++);
  set owner; return id. TimerDestroy: count--; delete + map remove. TimerReset: elapsed=0. TimerGetTime: elapsed.
  TimerRegisterNotify(obj,t,cb): Reg(7, key=t.owner.id, owner=t.id, Exec, {GetGameObject(obj),cb}).
  TimerUnregisterNotify(obj,t): Unreg(7, key=obj.id, owner=t.id). TimerPause(t,f): if t: +0x14=(f!=0).
  TimerSetFrequency(t,f): +0x18=f. TimerGetFrequency: +0x18. Only TimerPause null-checks.
- EFNotificationMgr (EFNotificationMgr.cpp): 0x1b types, map[type] = DTMap(0x25)<key, list head>; node {owner, fn,
  data(owned heap), next}. RegisterNotify(type,key,owner,fn,data): key absent -> new node, map insert (prepend);
  key present: node with owner found -> delete old data, replace fn+data in place; else new node PREPENDED (map value
  replaced). UnregisterNotify(type,key,owner): unlink first node with owner, delete data, remove key if list empty.
  UnregisterAllOwnerNotify(id): every type: drop key id and all its nodes. Notify(type,key,owner,data,deferred):
  !deferred: first node with node.owner==owner: r=fn(node.data,data) (i64); if r>=1 Unregister(type,key,owner);
  deferred: append to FIFO. NotifyAll(type,data): map bucket order, call ONLY the head node of each key; next map
  node read AFTER the call (UB if freed); if r>0 Unregister(type,key,head.owner). FlushNotification: detach FIFO,
  Notify(...,false) each, delete payload if type not 0/1. Called in EFWorld::Update (after anims) + end of PrepareRender.
- EFInstance::Notify(inst,type,data,def) = mgr.Notify(type, key=inst->+0x10->+8, owner=inst->+8, ...).
  EFGameObject +0x10 = this, so object events: key=owner=obj id; sub-instances (timers, anim nodes): key=owning obj id,
  owner=own id. API RegisterNotify(obj,sender,type,cb) (EFApi_GameObj.cpp:201): data {obj,cb}; sender==-1 ->
  key=owner=obj.id else key=obj.id, owner=sender; type 0x11 -> obj+0xbc=1. UnregisterNotify same keys.
  UnregisterAllNotify(obj) -> UnregisterAllOwnerNotify. ~EFGameObject -> UnregisterAllOwnerNotify(id) (EFGameObject.cpp:137).
- EFWorld (EFWorld.cpp): AddGameObject:417 map insert/replace + prepend to all-objects list +0x4128 (always).
  RemoveGameObject:469 unlink + map remove. Update:282: anim pass; FlushNotification; physics; particles; kill list
  +0x4150 (LIFO, KillObject prepends if absent, EFApi_GameObj.cpp:478): DestroyPhysic, RemoveGameObject, Terminate,
  delete; +0x4120++; PrepareRender: if cameras: clip pass over +0x4128 (newest first), lists re-prepended (oldest
  first): logic list +0x4140 = objects whose clip flags lack 0x400; deferred notifies 9/10; FlushNotification.
  No cameras -> lists not rebuilt. EFGameObject::Terminate:397: Destructor; DestroyAttachedEmitters; ...
- Objects with id&0x800000 are latents (LoadFromStream), not added to the world.
- PLAN: port all of the above into the runtime (src/logic/notify.rs, timers, frame.rs); DTMap<V> generic in world.rs;
  host hooks: clip flags per object (None = no cameras), has_physic, object_destroyed; then Bevy plugin.
  Tests in tests/smoke.rs rely on host TimerGetTime/KillObject overrides -> update them when moving these in.
- KeyToHashIndex (both overloads @0x10bd20/0x10bd40) is `divu` = UNSIGNED key % n (Ghidra showed int %).
  world.rs used signed % and panicked on negative keys -> fixed to unsigned (timer ids 0x9xxxxxxx need it).
- EFGameObject::Terminate (kill list) calls EFLogicInstance::Destructor UNCONDITIONALLY (even never-constructed
  objects: refcount-- and module dtor run). Destructor with code==0 clears stack frame [top].
- CreateTimer returns the new id (old +0x268); TimerCreate returns it. GetTimer(id) = map value, NULL deref if
  unknown (only TimerPause checks) -> port panics. TimerGetTime/GetFrequency return the f32 (bits).
- RegisterNotify/UnregisterNotify: nothing if GetGameObject(obj)==NULL; both set obj+0xbc=1 when type==0x11.
- NotifyAll: head node + map node read BEFORE the call; map node's next/key read AFTER the call (possibly freed);
  Unregister(type, key, head.owner) if r>0. Notify: r<1 -> keep, else Unregister.
- EFWorld::Update kill loop pops the head until empty (kills queued during Terminate are processed too):
  DestroyPhysic(obj+0x28), RemoveGameObject(id), Terminate, delete (~EFGameObject: UnregisterAllOwnerNotify).

## Step 4b DONE: runtime frame/notify/timers wired (src/logic/{notify,frame}.rs, core apiextra timers,
apigameobj Register/UnregisterNotify/KillObject, host hooks object_has_physic/destroy_physic/object_terminated).
Global timer 0x90000000 created in Engine::new. tests/smoke.rs uses e.GetTimer(0x9000_0000).elapsed and
e.rt.kill_list + e.ProcessKillList(). cargo test: 7/7 green.
## Step 4c DONE: Bevy plugin src/plugin.rs: ScoobyEnginePlugin::new(Box<dyn Host>) inserts Engine Resource;
EngineSet::{Input,Logic,Animation,PostAnimation,Physics,Finish} chained in Update; systems engine_logic
(clock.ComputeDeltaTime(Time.delta_secs) + GameLogicUpdate), engine_flush_notifications, engine_finish_frame
(ProcessKillList + PrepareRender_Logic(LogicClipList)), sync_game_objects (GameObjectId/GameObjectHandle entities,
GameObjectEntities map). tests/bevy_app.rs green (timer notify every 0.1s, KillObject despawns entity).
NOTE: deleting an object does not remove its timers' notify registrations (key=obj id, owner=timer id); firing
at a deleted object = crash in original, panic in port. Modules unregister in destructors.
## NEXT: plan step 5 = hero (Scooby/Shaggy) book parts + monster capture/flee/suck handlers.

## Step 4b implementation details (historical; all DONE, see above)
- DONE world.rs: generic `DTMap<V>` (new(buckets), find_node/get/get_mut/insert/remove->Option<V>/first/next/
  first_after_bucket_of/chain_next/alive/key/val_ref/val_mut/nodes_in_order; V:Copy -> val/find); ObjMap = DTMap<ObjH>.
- WRITTEN (not yet wired/compiled): src/logic/notify.rs (NotificationMgr{maps: Vec<DTMap<Vec<NotifyNode>>> x0x1b, queue},
  LogicCallback{obj,cb}, Engine::{ExecLogicCallback, NotificationMgr_RegisterNotify/UnregisterNotify/UnregisterAllOwnerNotify,
  Notify(ty,key,owner,payload,deferred), NotifyAll, FlushNotification, ClearAllNotification, notify_object}),
  src/logic/frame.rs (FrameClock::ComputeDeltaTime, GameTimer, TimerMap, Engine::{CreateTimer, DestroyTimer, GetTimer,
  GameLogicUpdate, UpdateLogic, DestroyObject, RemoveGameObject, Terminate, delete_object, ProcessKillList,
  PrepareRender_Logic(Option<&[u32]>), LaunchLogicNow, logic_callback}).
- TODO to compile: logic/mod.rs: `pub mod notify; pub mod frame;`; LogicRuntime fields notify: NotificationMgr,
  timers: TimerMap (DTMap::new(0x25)) + CreateTimer() global timer at init (id 0x90000000, next 0x90000001),
  next_timer_id, timer_count (_giTimerCount i32), logic_dt f32, update_all bool (true), logic_list Vec<ObjH>,
  kill_list Vec<ObjH> (push=prepend, pop=head), events Vec<WorldEvent>, world_frame u32; latent_next_id = 0x2a800000;
  `pub enum WorldEvent { Added{id,handle}, Deleted{id,handle} }` (add_object pushes Added); GameObject fields
  flags138 (EFGameObject+0x138: 0x200 freezes timers) + notify11 (+0xbc); Engine.clock: FrameClock.
  Destructor: code None -> rt.clear_next_frame(). remove_object -> RemoveGameObject+Terminate+delete_object.
  Host hooks (gen_api.py template): object_has_physic(id)->bool (false), object_terminated(mem,id), destroy_physic(mem,id).
  Core APIs: core/apigame.rs GetFrameDeltaTime(e)->f32 {e.clock.dt}; core/apiextra.rs Timer* (TimerCreate: _giTimerCount++,
  owner = stack.objects[top-1], id=CreateTimer; TimerRegisterNotify(obj,t,cb): Reg(7, key=timer.owner_id, owner=t, cb data
  {GetGameObject(obj),cb}); TimerUnregisterNotify(obj,t): Unreg(7, key=GetGameObject(obj).id, owner=t); TimerPause null-checks);
  apigameobj RegisterNotify(obj,sender,type,cb)/UnregisterNotify/UnregisterAllNotify/KillObject (prepend if absent);
  LaunchLatant -> LaunchLogicNow (Constructor only). Then re-run gen_api.py, fix tests (TestHost TimerGetTime/KillObject
  overrides become dead: drive time via timers/GameLogicUpdate), cargo test.
- Then plugin.rs: Engine #[derive(Resource)]; ScoobyEnginePlugin; SystemSets chained EngineSet::{Input, Logic, Animation,
  PostAnimation(FlushNotification), Physics, Finish(ProcessKillList+PrepareRender_Logic)}; frame clock from bevy_time
  Time (add dep bevy_time 0.19.1 default-features=false, in registry); GameObjectId(u32) Component + sync system
  spawning/despawning entities from rt.events.

## Tooling gotcha (context compression)
- compress tool: passing `summaryMaxChars` TRUNCATES summaries to ~200 chars (b41,b42,b45,b46 lost). Never pass it.

## Step 5 scoping (hero + monsters)
- Hero modules: <Ep>/Characters/{Scooby,Shaggy,+costume variants}/<Level>/<scooby|shaggy>__<sha8>/ (Ep1..Ep6, ~50 dirs).
  Ref: Ep1_Library/Characters/Scooby/Level_01/scooby__402420b4: 21 .c files, 7,860 lines, ~190 fns:
  hero_tome.c (271 lines, 9 fns: heroCB_INPUT_Tome, heroInit_Tome/TomeIn/TomeOut/TomeTrack/Unhook,
  heroST_TomeIn_Loop/TomeOut_Loop...), hero_core 934, hero_scared 939, hero_follow 682, hero_msgfunc 663, hero_misc 654...
- Monster book code is shared source compiled into every monster module: Monster_Base{ST_BEINGSUCKED_SET,
  ST_BEINGSUCKED_LOOP, ST_CAPTURED}, SetClippingCaptured, SetRunFromTomeSpeedParams, IsTomeOpen(ed);
  ghosts (Ep1-4 SMALL/MEDIUM/Big_Ghost): Ghost_Base{ST_BEINGSUCKED_LOOP, ST_TOME_IS_OPEN_FLEE, TR_HOOKED_BY_BOOK_SET,
  TR_TOME_IS_CLOSE_SET, TR_TOME_IS_OPEN_SET}, HeroInHookAttackRange; Shadows (Ep1), Zombie (Ep4/6), DustDevil (Ep3),
  Poltergeist, FIRE_GHOST. HUD: Globals/Panel panel_baseFUNC_{ToDFaderControl,ToDMeterChangeControl,UpdateTomeMeter}.

## Step 5 progress: hero (written to disk; READ THIS after any context compression)
- Family = the 57 hero builds that contain hero_tome.c: tools/rustport/hero_family.txt (paths). The other 33
  hero builds (costume/no book) are out of scope for now. Reference build scooby__402420b4 (EP1L01 Scooby).
- Generated: recomp/src/modules/hero/builds.rs (gen_builds.py over hero_family.txt, 87 multi-shape fns,
  739 literals; consts `G_<fn>` shape-group index, `<fn>_<grp>_<n>` literal index; doc comments show context).
  The 4 main shape groups: 402420b4 Scooby(23), 2b32b17e old Scooby(6), 0117f115 Shaggy(24), 1730c849 old Shaggy(4).
- Generated: hero/skeleton.rs + one stub file per .c (hero_*.rs, scooby.rs) by tools/rustport/gen_skeleton.py
  (`gen_skeleton.py <build dir> <outdir>`): `addr::<fn>` module-relative code addrs, `call_code(e, f, a0)`
  dispatcher, `INIT` image words, `DATA_SIZE` 0x3bc0, `globals::*` (pLocalDataHero 0x104f0, pGlobalDataHero
  0x104f4, pDebug 0x101c4...). Stubs `todo!("<fn>")` -> unported fns panic with their name (no silent omission).
- Generated: hero/tables.rs by tools/rustport/gen_hero_tables.py: VALIDE_CURRENT_SOUND per build (switch
  table), CHAR_NAME, IS_SHAGGY.
- hero/mod.rs written: MODULE (ModuleDef "scooby"), build_index, helpers G/L/GD/GB0/GB2/has_debug/k/g/unported_shape.
  `pub mod hero;` in modules/mod.rs; cargo build OK at that point.
- gen_builds.py now has `--partial` (functions only some builds have get slots; builds without them form a
  last "<absent>" shape group; doc says "(code of <sha8>)" when the fn's reference is not the family ref).
  Hero builds.rs regenerated: `python tools/rustport/gen_builds.py @tools/rustport/hero_family.txt
  scooby__402420b4 recomp/src/modules/hero/builds.rs --partial` -> 199 G_ fns, 906 slots.
  Non-reference shape groups: literals ALIGNED with the reference use the `_0_` slots, only unaligned ones get
  `_<g>_` slots -> for groups != 0 run `gen_builds.py ... --partial --annotate <fn> <sha8>` to see which slot
  each literal maps to (unslotted literals are constant in that group).
- tools: `hdiff.py <fn> <sha8A> [<sha8B>] [-n CTX]` (print/diff a hero fn), `kvals.py <builds.rs> NAME|prefix*`
  (value distribution of a slot or G_ over builds).
- tables.rs also has DEBUG_BUILD (47 debug, 10 release: release = empty heroFUNC_DebugTime + no pDebug prints).
- mod.rs helpers: debug_build, dprint (= debug build && pDebug != 0), char_name ("Scoobydoo"/"Shaggy" cstr),
  debug_info(e,msg), debug_info_value(e,msg,ptr,kind), kf (f32 slot), `pub mod tables;`.
- Hero shape-group axes: debug vs release (prints, DebugTime args), Scooby vs Shaggy (char name, UpdateBlend_Scooby
  vs UpdateBlend_Others, StartBlend structure), per-build literals. Module global addresses differ per build
  (Shaggy pLocalDataHero 0x10470, pDebug 0x10144): irrelevant, the image is module-private; port uses ref layout.
- pGlobalDataHero is `undefined4` (byte arithmetic): `**(float**)(pGlobalDataHero+4)` = rf(r32(GD+4)) (buffer 1,
  float @0); `*(uint*)pGlobalDataHero[2]` = r32(GB2); `*(int*)(*pGlobalDataHero+8)` = r32(GB0+8) = leader hero id;
  GB0+0x14 = flags (0x40000000 input consumed, 0x100 no-tome, 0x40000004 block tome).
- EABI: float args in $f12.. independently of int $a0..: StartBlend(f32 time=f12, int action=a0, a1, a2). The stub's
  `param_1: u32` is really f32. All hero code is -O0 ($fp frames): evaluate floats in source order, no madd.
- DONE (written, not yet compiled): hero_misc.rs UpdateTimerOutOfTime / IsOutOfTime / heroFUNC_DebugTime
  (+ `debug_time(e, "lit", p2, p3)` helper); hero_sounds.rs heroFUNC_ValideCurrentSound (tables-driven).
- heroFUNC_StartAnim (same in all builds): ValideCurrentSound(L, p2); return PlayAnim(p1,p2,p3,p4) as i64.
- heroFUNC_UpdateBlend_Scooby (G: 0 = 402420b4 x23, 1 = 2b32b17e x6 release, 2 absent), disasm-verified (Ghidra
  swapped the GetTime results): prologue `if p2==1 && x(L+0x2e0) < half(L+0x70*0.5) {p2=0} else if p2==0 &&
  half <= x {p2=1}` (c.olt/c.ole); if r32(p1+0x230) != p2 -> p1+0x218 = StartBlend(0.1, p2, 2, 0), return.
  Else: if p1+0x2b8 != 0 { a=GetTime(SELF,k_0_1); b=GetTime(SELF,k_0_2); if a != b { null_printf("-1.anim1
  %6.4f   ->  anim2 %6.4f  =  %6.4f\n"); SetTime(SELF,k_0_3,b); p1+0x2b8 = (p1+0x2b8==0) } }. p2==1: x=0x2e0,
  h=0x70*0.5; if x < 0x70*0.6(0x3f19999a) && p1+0x158==heroST_Move_Loop {UpdateTimerOutOfTime(2.0)};
  0x2ac=(x-h)/(0x70*0.95(0x3f733333)-h); if 1.0<v {1.0} else if v<0 {0}; SetSpeed(SELF,k_0_4,1.0);
  SetSpeed(SELF,k_0_5,1.0); ChangeBlendParams(SELF,p1+0x2a4). p2==0: if 0x158==Move_Loop {UpdateTimerOutOfTime(2.0)}.
  p2==3: nothing. else DebugTime("Failed to update blend",0,0). Return value unused by all 3 callers (garbage).
- heroFUNC_StartBlend G: 0 = 402420b4 x23, 1 = 2b32b17e x6, 2 = 0117f115 x24 (Shaggy), 3 = 1730c849 x4.
  g0: same prologue (on L), L+0x230=a. a==1: L+0x2a4=0xbb000002,+0x2a8=0x3e19999a,+0x2b0=0,+0x2ac=0x3dcccccd,
  +0x2b4=0x3f000000; BeginTree(SELF); r=StartAnim(f(0x3e09374c),k_0_1,2,0); PlayBlendConnect(SELF,k_0_2,k_0_3,10,
  L+0x2a4); SetTime(SELF,k_0_4,0.075); SetTime(SELF,k_0_5,0.075); L+0x2b8=1; EndTree; ret r. (int)a<2: a==0 ->
  ret StartAnim(p1,k_0_6,p3,p4). a==2: GetFloorAttribs(SELF,buf); if buf+8 & 1 -> ret StartAnim(p1,k_0_7,p3,p4);
  else params[5]={0xbb000003,0,1.0-rf(r32(GD+4)),0,0.5}; BeginTree; r=StartAnim(p1,k_0_8,p3,p4);
  PlayBlendConnect(SELF,k_0_9,k_0_10,p3,&params); EndTree; ret r. a==k_0_11 (3 x51, 0 x6): params id 0xbb000004;
  r=StartAnim(p1,k_0_12,..); PlayBlendConnect(SELF,k_0_13,k_0_14,2,&params); EndTree; ret r. Fallthrough: debug
  print "heroFUNC_InitBlend -> invalid eBlendAction" (&a,1); ret a.
  g1 (release Scooby): same logic, no print; unaligned slots 1_1 (case2 PlayBlendConnect a1), 1_2 (case2 anim),
  1_3 (floor-set anim), 1_4 (case3 anim) -> use --annotate for the rest.
  g2 (Shaggy, disasm): NO prologue adjust; L+0x230=p2; switch p2 { 2: params 0xbb000003; BeginTree;
  r=StartAnim(p1,0xf,p3,p4); PlayBlendConnect(SELF,0x34,0xf,p3,&params); EndTree; ret r | 3: 0xbb000004;
  StartAnim(p1,0x33,..); PlayBlendConnect(SELF,0x35,0x33,2,&params) | 1: L+0x2a4.. as g0; StartAnim(0x3e09374c,
  0x2b,2,0); PlayBlendConnect(SELF,0x2a,0x2b,10,L+0x2a4); SetTime 0x2b,0x2a 0.075; L+0x2b8=1 | default (incl 0):
  debug print "heroFUNC_InitBlend -> invalid eBlendAction" (&p2,1); ret p2 } (literals per --annotate).
  g3 (release Shaggy): as g2 without the print; anims case2 6/0x28, case1 0x20/0x1f, case3 0x26/0x27.
- heroFUNC_UpdateBlend_Others (Shaggy only; G 0 = 0117f115 x24, 1 = 1730c849 x4, 2 absent; NO stub exists yet ->
  add to hero_blend.rs): if p1+0x2b8 != 0 { a=GetTime(k_0_1 [0x2b]); b=GetTime(k_0_2 [0x2a]); if a != b
  { null_printf("0.anim1 ..."); SetTime(k_0_3, b); flip 0x2b8 } } (verify GetTime order with disasm). p2==1:
  t=0x70*0.333; x=0x2e0; Move_Loop timer check as Scooby; if x < t { 0x2ac=0; if x < t*0.5 {x=t*0.5};
  s=(x/t)*0.6666667; SetSpeed(k_0_4,s); SetSpeed(k_0_5,s); ChangeBlendParams } else { 0x2ac=(x-t)/(0x70*0.95-t);
  clamp; s=0x2ac*0.5833333+0.6666667; SetSpeed(k_0_6,s); SetSpeed(k_0_7,s); ChangeBlendParams }. else if p2!=0 &&
  p2!=3: debug print "FUNC_UpdateBlend_Others -> invalid eBlendAction" (&p2,1).
- Tome fn groups + slots: heroInit_TomeIn g1 = release (DebugTime()); slots 0_1 [0x3f] leader anim, 0_2 [0x1d]
  follower anim, 0_3/0_4 [0x2a000092] GetID/PrimReset, 0_5/0_6 [0x2a000091]. heroInit_Tome slot 0_1 [0x2d].
  heroST_Tome_Loop g1 = c7b040f3 (1 build, negative anim ids -0x23/-0x33, StartAnim 0xffffffcd/0xffffffdd; hardcode);
  slots 0_1 [0x1e] cmp, 0_2 [0x2d] cmp, 0_3 [0x2d] anim, 0_4 [0x1e] anim. heroInit_TomeOut slots 0_1 [0x40],
  0_2 [0x1c], 0_3..5 [0x2a000092] (GetID,PrimGetIntAttrib,PrimSetIntAttribBits), 0_6..8 [0x2a000091].
  heroInit_TomeTrack (4 groups) slots 0_1 [0x2a000091] leader cmp, 0_2/0_3 [0x2a000093] GetPosition bone 10/9;
  release: no "Init_TomeTrack - time" print. heroST_TomeTrack_Loop (4 groups) slots 0_1 [0x2a000091],
  0_2 [0x2a000093] GetPosition, 0_3 [0x2a000093] SendMessageTo 0x76; Shaggy -> UpdateBlend_Others; release: no
  "\"ST_TOME -> IN RANGE\"" print. heroValidOpenTome (g0 x23, g1 release x10, g2 Shaggy x24) slot 0_1 [0x2a00008f].
  heroCB_INPUT_Tome g1 release; slots 0_1 [0x2a000091], 0_2 [0x2a000092], 0_3 [0x2a000093]. Code: l=L;
  t=TimerGetTime(r32(l+0x20c)); if 0.75 <= t { if (l+0x17c==k1 || ==k2) && GetID(k3,0) != 0x7ffffffe &&
  GB0+0x14 & 0x40000004 == 0 { DebugTime("CB_INPUT_Tome",0,0); GB0+0x14 |= 0x40000000; l+0x16c |= 8 } else
  DebugTime("CB_INPUT_Tome wrong uiMyID", l+0x17c, 1) } ret 0.
- DONE + compiling: hero_blend.rs (StartAnim, StartBlend g0-3, UpdateBlend_Scooby g0-1, UpdateBlend_Others g0-1
  [added fn, Shaggy only]), hero_tome.rs (all 9; heroST_Tome_Loop g1 c7b040f3 hard-coded -0x23/-0x33/
  0xffffffcd/0xffffffdd), hero_inputs.rs heroCB_INPUT_Tome. StartBlend param_1 is now f32. Helpers in
  hero_blend.rs: blend_params, scooby_action, start_run_blend, start_tree. Group axes verified with hdiff:
  tome fns differ only by slots + debug prints (handled by debug_info/debug_time) + Shaggy UpdateBlend_Others.
- Tome entry points in hero (ref file lines): hero_core.c:94 heroFUNC_CheckInputEvents (0x1d1a0, 1052 b):
  local+0x16c bit 8 (tome request) & !(0x170 & 4): if GB0+0x14 & 1 == 0 -> if !(0x170&0x4000) &&
  heroValidOpenTome -> clear 0x16c bit 8, 0x170 |= 0x4000, IsInTeamMode -> msg 0x8d / 0x60(0x4000) to the
  partner (local+0x180); else (book open) if !(0x170 & 0x2000) -> 0x170 |= 0x2000, team msgs 0x8d/0x60(0x2000).
  hero_core.c:297-328 heroFUNC_CheckActionEvents (0x1d900, 4088 b) assigns heroInit_TomeOut/TomeIn/TomeTrack
  to local+0x154. hero_msgfunc.c heroFUNC_MessageHandler (0x25f48, 9172 b) checks heroST_Tome_Loop (line 226).
- NEXT: port hero_core.c CheckInputEvents + CheckActionEvents + their callees, then heroFUNC_MessageHandler,
  (whole functions, 1:1), then monster book states (Monster_Base / Ghost_Base).
  Lesson: in bash loops quote paths with spaces ("$p").

## Step 5b: ghost family (monster side of the book) -- recomp/src/modules/ghost/ (READ after compression)
- Family = 31 builds (tools/rustport/ghost_family.txt; make with tools/rustport/mk_family.py <fn> <ref> <out>).
  Ref smallghostlevelone__757255de (EP1L01). Builds whose dir has no __sha8 are keyed by dir name
  (bigghostlevelfour, medghostlevelthreeold, ghostbosslevel5, medghostlevel4a/4b/5, ...): BUILDS[b].sha8.
- Generated: builds.rs (gen_builds.py --partial, G_<fn>/<fn>_<g>_<n>/ID_<hex> slots), maps.rs
  (GO_CUSTOM_DATA_GHOST, TMP_MONSTER_MAP per build, written by ghost_build_init), emitter_pos.rs
  (tools/rustport/gen_emitter_pos.py: CreateEmitterPos constant stores per monster type 1/2/3/0x13).
- Hand-written: mod.rs (one global layout for all builds: goCustomDataGhost 0x10000 [200 b], TmpMonsterMap
  0x100c8 [300 b], API pointers 0x10200.., pLocalDataMonster_Base 0x10240, pGlobalDataMonster_Base 0x10244,
  pLocalDataGhost_Base 0x10248, pGlobalDataGhost_Base 0x1024c, DATA_SIZE 0x300; helpers G/L/LM/GDM/GDG/k/g/
  build_key/unported_shape/call_external_now/or32), monster_base.rs (Monster_Base.c, 26 fns), ghost_base.rs
  (Ghost_base.c, 53 fns), glow.rs (Glow.c, 6), ghost_glow_init.rs (GhostGlowInit.c), level.rs (every level +
  episode file of the 31 builds; ghost_levelConstructor/Destructor dispatch on build_key; per-build
  functions with a name clash carry a `_<key>` suffix).
- Book protocol seen by the ghost: msg 0xd6 from the book/beam: 100 link broken (ev 0x4000), 0x65 hooked
  (ev 0x1000; Monster_Base launches latent 0x2a800118 = the suck beam, msg 0xec params {L+0xb0,L+0x5c,
  L+0xb4,L+0x68,L+0x6c,L+0x3c,L+0x58} and replies 0xd6/0x66 to the sender), 0x67 pulled in (ev 0x2000,
  BLACK_HOLE state 5 + TrackID book 0x2a000093), 0x68 captured (game buffer 2 +4 |= kill mask L+0x40,
  msg 0x26 to 0x2a100006, 0xea to 0x2a00011a, state 0xb), 0x69 suck sound, 0x6a pull vector -> DM L+0xc.
  0xc9/0xc1 restart (if captured before: back out of the book), 0xcd new leader, 0xd4 kill.
  Book open = game buffer 0 +0x14 bit 0 && game buffer 1 +4 (power) > 0 (IsTomeOpened).
- Every shape group of every ghost fn is ported. Group 1 of the Ghost_Base states = ghostbosslevel5 (Ep6 boss):
  no TOME_IS_OPEN checks (never flees the book), ROAMING -> search mode when L+0x850, SEARCH_MODE forces
  ev 0x100 (chase), Initialisation keeps 0x70/0x74/0x78/0x850; shorter timers (1.3/2.5/1.0/0.5).
- Engine: Engine::zero_page is now zero-filled (level.rs saves clipping params through NULL L+0x85c, as the
  PS2 original writes to address 0).
- DONE + compiling + tested (tests/smoke.rs every_ghost_build_runs: all 31 builds construct and run
  closed/open/hooked; ghost_flees_open_book_and_is_captured: EP1L01 roam -> 0x18 flee -> 0x15 hooked
  (0xd6/0x66 to the book) -> 0xb captured (kill mask in gb2+4, 0x26 to HUD, 0x14e to manager)).
- NEXT: other monster families with book code (Shadows Ep1, Zombie Ep4/6, DustDevil Ep3, Poltergeist,
  FIRE_GHOST: Monster_Base + their own base), HUD panel tome meter (Globals/Panel panel_baseFUNC_
  {ToDFaderControl,ToDMeterChangeControl,UpdateTomeMeter}), then hero_core CheckInputEvents/CheckActionEvents
  + heroFUNC_MessageHandler.

## Step 5c: shadows family (Ep1) -- recomp/src/modules/shadows/ (READ after compression)
- 4 builds (tools/rustport/shadows_family.txt): shadowslevelone (ref), shadowsleveltwo, shadowslevelthree,
  shadowslevelthreeold (keyed by dir name). Same layout scheme as ghost: builds.rs (generated), maps.rs
  (GO_CUSTOM_DATA_SHADOWS records {id, sector mask -> L+0x180, kill mask -> L+0x40}, TMP_MONSTER_MAP),
  mod.rs (globals 0x10000.., DATA_SIZE 0x180, helpers G/L/LM/GDM/GDS/k/g/build_key), monster_base.rs
  (this family's Monster_Base copy), shadows_base.rs (Shadows_base.c, 41 fns), shadows_base_particle.rs,
  level.rs (ShadowsEpisodeOne.c + 4 level files, shadows_levelConstructor/Destructor dispatch).
- Shadow AI: states 0xe START, 0xf WAITING_HERO, 0x10 SNEAKING, 0x11 WALK_ON_HERO, 0x12 ATTACK, 0x13
  WHAT_IS_NEXT, 0x14 BEINGSUCKED (hooked: struggles back towards its post, lashes out after 3-5 s), 0x17
  FLEE_TOME (book open: back to the post at sneak speed), 0x18 STAY_AWAY_FROM_TOME (at the post: scares
  every 3-5 s); events in L+0x184 (bits listed in shadows_base.rs header). Book msgs 0xd6: 100 link broken,
  0x65 hooked, 0x67 pulled in, 0x68 captured (glow off) then Monster_Base_MessageFunction. Glow = blend
  colour (0.079, 0.093, 0.28, alpha 0/1). Hero hiding = GDS game buffer 0 +0x14 & 0x40 -> teleport to post.
- DONE + compiling; tests every_shadows_build_runs, shadow_flees_open_book_and_is_captured.
- NEXT: Zombie (Ep4/6), DustDevil (Ep3), Poltergeist, FIRE_GHOST families; HUD tome meter; hero_core
  CheckInputEvents/CheckActionEvents + heroFUNC_MessageHandler.

## Step 5d: zombie family (Ep4/Ep6) -- recomp/src/modules/zombie/ (READ after compression)
- 11 builds keyed by dir name (Ep4: zombieleveltwoa, 5e7d92cd [= Ep4 zombielevelfourb], zombielevelfourc,
  zombielevelfourd; Ep6: zombieleveltwo, zombielevelfoura_b/_c/_d, 35a2d522 [= Ep6 zombielevelfourb],
  zombielevelfourb_c/_d). Files: builds.rs/maps.rs (generated by tools/rustport/gen_data_tables.py, which
  falls back to the ELF .symtab when _globals.h lacks an array), mod.rs (one global layout for all builds),
  monster_base.rs, zombie_base.rs (zombie_base.c), zombie_base_particle.rs, level.rs (episode + level files,
  zombie_levelConstructor/Destructor dispatch; the two `zombielevelfourb` builds carry `_<sha8>` suffixes).
- goCustomDataZombie records {id, -> L+0x1e0, behaviour -> L+0x194 (1 guard-wait, 2 wander, 3 guard
  post/boss, 4 fixed), kill mask -> L+0x40/L+400}.
- Book: IsTomeOpen = gb0+0x14 bit 0 && hero within 20 m -> SCARED_NOTHOOKED (0x1b/0x1c); 0xd6/0x65 hooked
  -> L+0x1cc=1, SCARED (0x19/0x1a, defensive gas attack 0x1d after 3 s); 0x67 pulled in (state 5);
  0x68 captured (Monster_Base: kill mask -> gb2+4, 0x26 to HUD 0x2a100006). Ep4 level-4 builds intercept
  0x68 (hide, ST_CAPTURED 0xb -> 0x14e to manager 0x2a00000d) and add knock-back 0xda (state 0x32) /
  respawn 0x1fd (state 0x21).
- DONE + tests every_zombie_build_runs, zombie_scared_by_open_book_and_is_captured (TestHost.away puts
  the hero 10 m off; at the origin the zombie attacks (0x10) and waits for an anim end the stand-in
  host never sends).
- NEXT: DustDevil (Ep3), Poltergeist, FIRE_GHOST families; HUD tome meter; hero_core input.

## Step 5e: dustdevil family (Ep3) -- recomp/src/modules/dustdevil/ (READ after compression)
- 4 builds keyed by dir name: dustdevilleveltwo (ref), dustdevillevelfour, dustdevillevelfive (the boss's
  six split-offs), dustdevillevelfiveboss. Files: builds.rs/maps.rs (generated), mod.rs (one global layout:
  tables, API ptrs, boss bLoading/bHalfHealth/moCameraParams_198, DATA_SIZE 0x1c4), monster_base.rs,
  dustdevil_base.rs (DustDevil_Base.c), dustdevil_glow_init.rs (DustDevilGlowInit.c: sprite rings, glow,
  debris/dust emitters), dustdevil_particle.rs (generated particle settings), level.rs (episode + 4 level
  files, dustdevil_levelConstructor/Destructor dispatch).
- goCustomDataDustDevil records {id, waypoint path -> L+0x168, kill mask -> L+0x40, sector mask -> L+0xefc}.
- AI: zig-zags along its path (0x10/0x11), chases the hero within 10 m (0x13/0x14), attack 0x15/0x16.
  Book: 0xd6/0x65 hooked -> SCARED_FLEE 0x17/0x18 (every 5 s SCARED_ATTACK 0x19 facing the book for 3 s),
  0x67 pulled in (glow destroyed, state 5), 0x68 captured (hidden; Monster_Base kill mask + HUD 0x26).
  Group 1 builds (four/five/boss) also play hook / suck sounds on the level's sound object.
- Boss (type 10): stage timer L+0x180 releases pairs of split-offs (9e/9f, a0/a1, a2/a3) as the previous
  pair is captured (gb2+4 bits 0x1000..0x20000); then roams/catchable; captured -> camera 0x54, HUD 0x1e,
  next 0x4f LoadLevel("LOADER", {2, "\MOVIES\EP03\EP03A"}) (DAT_00010fe0 decoded from the .o + reloc).
- Monster_Base 0xd6/0x67 dereferences param_3 (blend time): tests pass a pointer.
- DONE + tests every_dustdevil_build_runs, dustdevil_flees_hook_and_is_captured.
- NEXT: Poltergeist, FIRE_GHOST families; HUD tome meter; hero_core input.

## Step 5f: poltergeist family -- recomp/src/modules/poltergeist/ -- DONE, tested (17/17 smoke tests pass)
- Sources: Globals/Characters/Poltergeist_base/poltergeist__{0bb93729(ref, EP2L01C),04b00cff(EP3L01),
  1c9d41ec,312143a4,7d96cf48,b8041da2,fa6a1f63}; files Monster_Base.c, Poltergeist.c (level file),
  PoltergeistII_base.c (~1000 lines), PoltergeistII_Base_Utils.c, ep2_/ep3_poltergeist.c (episode ctor/dtor
  just call poltergeist_baseConstructor/Destructor; ep3 in 04b00cff + one more build, ep2 in the other 5).
- Family list tools/rustport/poltergeist_family.txt; builds.rs + maps.rs (goCustomDataPoltergeist 0x4c
  records {id,+4 ->L+0x1a0,+8 kill mask ->L+0x40,+0xc ->L+0x1a4,+0x10 path name 0x3c ->L+0x1a8},
  TmpMonsterMap) GENERATED. monster_base.rs adapted (ids via k(), 0xcd list 2 or 4 ids, no
  CheckIfInSector, b8041da2 single pass).
- Globals (ref): TmpMonsterMap 0x100e8, pFBEffects 0x101d8, pMath, pPathFinding, pSound, pSectorClip,
  pAnimMngr, pFunction, pMiscFunc, pDebug, pPhysic, pExtra, pGameObj, pAnim 0x10208, pLocalDataMonster_Base
  0x1020c, pGlobalDataMonster_Base, pLocalDataPoltergeistII_Base 0x10214, pGlobalDataPoltergeist_Base.
- Base stores the current state as a CODE POINTER at L+0x160: use an `addr` module + call_state
  dispatcher like hero/skeleton.rs (addr:: consts, call_code match).
- Shape variants (hdiff.py): ST_Initial 04b00cff extra `L+0x1f0=0` after `=1`; Standing_Execute 312143a4
  extra (GDP+0x10 !=0 -> clear, send self 0xb3); JumpOnThrowable_In/ChooseWalkType 1c9d41ec anim -1/0xd/4;
  BaseFUNC_MessageFunction 312143a4 no hook/drop sounds; baseConstructor: record count 2 (ref) /1 (04b00cff)
  /3 (fa6a1f63), path names; level-only fns: ST_BlackHoleLoop+CB_CUTSCENE (0bb93729),
  BlackLoop_In/Execute (b8041da2), BlackLoopOfDoom_In/Execute (fa6a1f63).
- Files: mod.rs (one global layout for all builds: goCustomDataPoltergeist 0x10000 (as in every build),
  TmpMonsterMap 0x10130, pointers from 0x10260), monster_base.rs, poltergeist_base_utils.rs,
  poltergeist_base.rs (sub-state code pointer L+0x160: `addr` consts = each function's own address in its
  build, `call_state` dispatcher panics on an unknown pointer), level.rs (entry dispatch on sha8:
  0bb93729 EP2L01C gliders (type 8, BlackHoleLoop, CB_CUTSCENE 0x16, msgs 0x113/0xc9/0x119/0x11a),
  1c9d41ec EP2L09 (0x143/0x144 sleep, 0xc1 restart returns 0, 0x1d2 SetPath), 312143a4 EP2L01A (0xb3 laugh),
  b8041da2 EP2L01B (0xde/0x143 BlackLoop), fa6a1f63 EP2L02B (type 7; 0xdf/0xde/0x136 BlackLoopOfDoom),
  04b00cff/7d96cf48 Ep3 (ep3 entry; 7d96cf48 L+400 = 1 jump allowed)).
- Ep2 = Movie Studio (not Graveyard). ep2/ep3 destructors pass an uninitialised a0; port passes 0 (unused).
- Tests: every_poltergeist_build_runs; poltergeist_scared_by_hook_and_is_captured (fa6a1f63: no path;
  uses e.GameLogicUpdate so the pre-logic notification 0x14 -> callback 0x15 runs the range checks. With
  a path (other builds) and a hero off it, the pre-logic sends a scared poltergeist back to Standing --
  faithful, the stand-in path lib reports "not on path").
- (superseded: see Step 5g)

## Step 5g: FIRE_GHOST family (Ep6) -- recomp/src/modules/fireghost/ -- DONE, tested (19 smoke tests pass)
- Sources: Globals/Characters/FIRE_GHOST/fireghost__{6f63c7c0 (ref, EP6L04A), 818e89e1 (EP6L04B),
  3a7b55ab (EP6L05)}: Monster_Base.c, FireGhost_Base.c, FireGhost_Base_particle.c (generated by
  gen_particle_settings.py), FireGhost.c (level file -> level.rs).
- Files: mod.rs (Cfg per build: literals only; one global layout), monster_base.rs, fireghost_base.rs
  (sub-state code pointer L+0x160 -> `addr` consts (ref addresses, stored in every build) + call_state),
  fireghost_base_particle.rs, level.rs (fireGhostFUNC_MessageHandler: msg 0x98 sector table per build
  -> Create/DestroyFireParticle, EP6L05 always create; fireghostConstructor: ranges / group mask (L04A),
  ranges (L04B), type 0x12 (L05)), builds.rs/maps.rs generated (gen_data_tables.py: falls back to the
  ELF .symtab when an array is missing from _globals.h).
- Fire ghost = burning shell around an ordinary ghost (record +0x10 -> L+0x1a4). Water (0xd8 / 0x1a9 while
  burning) -> Extinguished (smoke, anim 1 end swaps in the ghost: SetNextState(ghost, 0xe)); the ghost
  module is what the book captures; 0x68 to the shell = the ghost was captured. Relight timer L+0x1e8 is
  re-armed while the hero holds the book open (game buffer 0 +0x14 bit 0).
- Monster_Base's message function is never installed (the fire ghost's replaces it, no chaining).
- GetNearestWP loops `uStack_30 < (uint)L+0x1f8` (= WaypointGetNb-1): 0 waypoints = ~4G iterations, as on
  the PS2. TestHost (tests/smoke.rs) now reports 4 waypoints (i,0,0) per path.
- Tests: every_fireghost_build_runs; fireghost_extinguished_by_water_and_relights (6f63c7c0; anim end
  injected with e.Notify(1, id, anim, anim, false), which is what anim_mngr's PlayAnim registers).
- NEXT: Ep6 boss (look for its family under Globals/Characters or Ep6_ShermanTech), HUD tome meter
  (Globals/Panel panel_baseFUNC_*), hero_core input.

## Step 5h: ZOMBIE_BOSS (Ep4 boss, EP4L04) -- src/modules/zombieboss/ + zombieboss_binding.rs -- DONE, tested (20/20)
- Sources: Globals/Characters/ZOMBIE_BOSS/zombieboss/{Monster_Base.c, ZombieBoss.c, ZombieBossParticle.c
  (generated by gen_particle_settings.py), ZombieBossParticle_Func.c} and zombieboss_binding/ZombieBoss_binding.c.
  One build each (build_index("zombieboss") / ("zombieboss_binding")).
- Files: mod.rs (globals, helpers L/LM/GDM/GDZ/or32/and32, TmpMonsterMap init), monster_base.rs (copied from
  fireghost's, literals for this module), zombieboss.rs (sub-state code pointer L+0x178 -> `addr` + call_state),
  zombieboss_particle.rs, zombieboss_particle_func.rs; zombieboss_binding.rs (latent 0x2a8000bd).
- Unlike the fire ghost, ZombieBossMessageFonction DOES chain to Monster_Base_MessageFunction (except 0xc9).
- Fight: the L+0x17c event bits are set by ZombieBossFUNC_GameEventSignaler for the conditions in L+0x180.
  Struggle phases L+0x198: the boss asks the book (0x2a00000a) for its pull strength (0x125 -> L+0x1fc);
  phase 0 breaks free at <= 200, phase 1 at <= 100 (event 0x20000 -> phase+1, DEFENSIVE_ATTACK). The stomp key
  frame drops wisps (0xf7 to 0x2a0000c3), launches the shock wave 0x2a8000be, shakes the camera and sends 0x92
  (book knocked shut) to the leader hero if the book is open. 0xd6/0x67: hide + binding latent; 0xd6/0x68:
  L+500 = 1, HUD 0x2b/9 + 0x1e; 0x4f then loads "\MOVIES\EP04\EP04D" through LOADER.
- PS2 quirks kept: SCARED_FOLLOW_SET does not clear L+0x17c, so a 0x20 bit (hero in attack range) left by
  FOLLOW_HERO makes the boss gas the hero even with the book open; L+0x40 (kill mask) is never set, so the
  capture is not recorded in game buffer 2 +4; Dirt2Stomp gets the Dirt1Stomp settings; ZombieBossCB_CUT_SCENE
  uses the pointer at L+0x270, never set by the module; the 0xde message's buffer words 1..3 are uninitialised
  on the PS2 (zeroed in the port).
- Test: zombieboss_struggles_breaks_free_and_is_captured (hero 10 m away via TestHost.away).
- NEXT: Ep6 boss, HUD tome meter (Globals/Panel panel_baseFUNC_*), hero_core input.

## Step 5i: MUMMY family (Ep2) -- src/modules/mummy/ + mummy_binding.rs -- DONE, tested (22/22)
- 5 builds (tools/rustport/mummy_family.txt, ref mummyleveltwoa): mummyleveltwoa/twob/twoc, mummylevelthreea,
  mummylevelnine. Generated builds.rs (k()/g()) + maps.rs (GO_CUSTOM_DATA_MUMMY records of 0x50 bytes
  {id, sarcophagus, behaviour, kill mask, has path, path name}, TMP_MONSTER_MAP), written by mummy_build_init.
- Files: mod.rs (one global layout: goCustomDataMummy 0x10000 [480 b], TmpMonsterMap 0x101e0 [300 b], pointers
  0x10310.., DATA_SIZE 0x358), monster_base.rs, mummy_base.rs (52 fns, sub-state via set_state/addr), particle.rs,
  particle_func.rs, level.rs (MummyEpisodeTwo.c + the 5 MummyLevel<N>.c; mummy_levelConstructor/Destructor
  dispatch on build_key). mummy_binding.rs = Globals/Characters/Mummy_Base/mummy_binding (bandage latent).
- Book: hooked 0xd6/0x65 -> SCARED (sound except L02B); 0xd6/0x67 -> invisible, binding latent k(ID_2a80010c)
  stored at L+400, gets 0x67 + the book data, particles destroyed, state 5; 0xd6/0x68 sleep/sarcophagus anim.
  Monster_Base's own beam latent is k(ID_2a8000bb) (LM+0x3c). HUD = k(ID_2a100006).
- PS2 quirks kept: Destroy_Particles destroys from L+0x200 (types twice, last 8 emitters leak); L02C 0x1d2 path
  name written at L+0x2d4 (overlaps the L+0x294 buffer); L09 0xc1 writes the start pos through Mummy_Base's ptr.
- Tests: every_mummy_build_runs, mummy_hooked_pulled_in_and_captured.
- NEXT: Ep3 mummyride (Ep3_FrontierLand/Characters/MUMMY/LEVEL_03/mummyride), then remaining Monster_Base
  families (Patrolling_guard_base, mercenary_base, security_guard_base, Scientist), GHOST_BOSS L05.

## Step 5j: mummyride (Ep3 L03) -- src/modules/mummyride.rs -- DONE, tested (23/23)
- Source: Ep3_FrontierLand/Characters/MUMMY/LEVEL_03/mummyride/{mummyRide.c, Particle_FUNC.c}, 10 fns, one build.
- A scripted ride prop, NOT a book monster (no Monster_Base). 0xae parent to 0x2a000009 bone 0xe + AnimMngr anim 1;
  0xa6 anim 2; 0xaa latent 0x2a80000d + anim 3; key frames (notify type 0xb, key -1 -> ext 3) launch 0x2a80000e /
  send 0x110. 0x160/0x161/0x188 rotations about y.
- PS2 quirks kept: pSound overwritten by "SndMngr"; Destroy_Particles destroys L+0/L+4 as emitters (L+8 leaks).
- Test: mummyride_scripted_anims_and_latents. Note: SetNextState lands on the frame after.
- NEXT: remaining Monster_Base families (Patrolling_guard_base, mercenary_base, security_guard_base, Scientist),
  GHOST_BOSS L05 (Ep6), HUD tome meter (Globals/Panel panel_baseFUNC_*).

## Step 5k: FIRE_GHOST_BOSS (Ep6) -- src/modules/fireghostboss.rs -- DONE, tested (24/24)
- Source: Ep6_ShermanTech/Characters/FIRE_GHOST_BOSS/fireghostboss/{FireGhostBoss.c, FireGhostBoss_particle.c}.
- No Monster_Base; sub-state code pointer at L+0 (`addr` + call_state/go). Byte offsets verified with disasm.py
  (Ghidra mixes word-index and byte-offset views of pLocalData/pGlobalData). Global data: +0 gb0, +4 gb2,
  +8/+0xc FIRE_ELBOW/FIRE_HEAD types.
- Flow: INITIAL -> BlackHole (idle) until cut-scene end (ext 6, start=0): anim 2 + 1.5 s timer -> SearchingHero
  (30 m) -> AttackPreparation (turn 0.1257 rad/frame) -> Attack (fireball latent 0x2a80001b; four after the first
  water hit 0xd8, aimed at 0x2a00010c.. by brazier 0x2a0000ce..d1) -> key frame 3 releases (0x110).
- PS2 quirks kept: local data fetched before API tables; pFunction fetched twice; first fireball latent only in the
  first instance's local data.
- Test: fireghostboss_attacks_and_is_hit_by_water.

## Step 5l: SCIENTIST family (Ep6) -- src/modules/scientist/ -- DONE, tested (25/25)
- 2 builds: scientist__c66ebe98 (EP6L02), scientist__955aa35f (EP6L09). Generated builds.rs (k()/g(), 18 shape
  groups, 31 literals) + maps.rs (goCustomDataScientist: 4 records of 0x58 bytes {id, waypoint set, .., has path,
  path name[0x3c]}; TmpMonsterMap: type 0xe (+0xc in L02)), written by scientist_build_init.
- Files: mod.rs, monster_base.rs (shared book code), scientist_base.rs (46 fns; sub-state ptr L+0x160, addr +
  call_state/go/set_state; events L+0x178, conditions L+0x164 -> scientist_baseFUNC_GameEventSignaler),
  scientist.rs (ScientistEpisodeSix.c + Scientist.c: L02 force fields / key card / caged scientist puzzle,
  object ids literal since only c66ebe98 has that code).
- Book: shared Monster_Base (0xd6/0x65 beam latent k(ID_2a8000d0), 0x67 pulled -> state 5, 0x68 captured -> 0xb).
- PS2 quirks kept: CUT_SCENE saves/restores clipping through L+0x244, which is never set (NULL); Scared does not
  clear event 0x800; L09 Sleep never wakes; TrackPos target y = 0.5; the kill mask LM+0x40 is never written for
  scientists, so capture ORs 0 into game buffer 2 +4; srand seeded from TimerGetTime($f0), as in zombie/mummy.
- Test: every_scientist_build_runs (all 8 scientists: idle, asleep 0x1d0, woken 0x1cf, L09 water, book capture).
- NEXT: remaining Monster_Base families (Patrolling_guard_base, mercenary_base, security_guard_base), GHOST_BOSS
  L05 (Ep6), HUD tome meter (Globals/Panel panel_baseFUNC_*).

## Step 5m: HUD panel (Globals/Panel/panel__*) -- SCAFFOLDING ONLY (not in modules/mod.rs yet)
- 37 builds (tools/rustport/panel_family.txt, ref panel__014a381f; mk_family on panel_baseFUNC_UpdateTomeMeter).
  94 fns (panel.c 6, panel_base.c 75, panel_base_options.c 13); 6 builds add panel_bossmeter.c. Mostly pause /
  options / episode-files menus; book-relevant: UpdateTomeMeter (rotates win 0xf00062a by delta of GB1+4 angle,
  0..180), ToDMeterChangeControl, ToDFaderControl ("TOD_Normal_Meter"/"drama_masks" tex fade on 0xf00062c),
  UpdateCoolMeter (GB1+0 fear -> 0xf000624 rotation) + TexFaderControl ("Scooby_Shaggy_Normal/Nervous/Scared").
- Generated: src/modules/panel/builds.rs (gen_builds --partial, 1132 slots: LOCAL-DATA OFFSETS VARY PER BUILD,
  e.g. pLocalData_Panel+0x5bc vs +0x5c4), skeleton.rs + panel.rs/panel_base.rs/panel_base_options.rs stubs.
- Data image: 954 R_MIPS_32 relocs (all against .text) in the globals area: addend < first_code-0x10000 ->
  data pointer (window names etc.; absolute = data base + addend), else code addr (menu callbacks / switch jump
  tables; module-relative = 0x10000 + addend). skeleton INIT is UNRELOCATED and ref-build only: need a per-build
  image + reloc list (tables at k() literal addresses like 0x160a8 = window-description records of 0x60 bytes:
  +0 name ptr, +4 kind (0 plain/1 text/2 2D tex/3 3D obj), +8 parent (1000/2000/-1/index), +0xc flags, +0x10 dims,
  +0x18 pos, +0x20, +0x24 hidden, +0x28 font, +0x2c text, +0x33 texture name, +0x48 tiling, +0x50 color,
  +0x54 obj, +0x58, +0x5c anim). Deferred: hero core first (higher book priority).

- ALL 37 panel builds now run in-game (test panel_object_runs_in_game asserts 37/37):
  - base MessageHandler group 3 (6 boss builds) = reference + tail call MessageHandlerBossMeter(p1,p2);
    ep object per sha8. Boss meter (Show/Move/Update/MessageHandlerBossMeter) in panel_base.rs.
  - panelFUNC_Message_Boss (panel.rs): boss-level panel.c messages (0x52 fader/boss meter, cd8e4e92 extras).
  - "timer levels" 05789c5e 695761b7 cb9dbf66 dbdc8239 fe754b69 (+cd8e4e92 FIRSTMOVE): ref + 0x11b/0x11c/
    0x183 (flag word at local_size-4, cleared in FIRSTMOVE); 695761b7 doesn't toggle the notif windows.
  - panel_frontier.rs: gold-nugget (3ce3c13c, a8417f38) and gas (99c250a4 af7a3894 bcb77619 ceb7d472)
    meter levels: own ctor/dtor/FIRSTMOVE/panel_LocalST_IN_GAME (state 0x22; gas also state 2)/Message/
    RegisterTimer (notify 0x23). Quirks kept: gas msg 2 behaves like 0x33; gas 0x13 falls through to the
    base handler (gold returns); gas dtor leaks its timer and windows.
- Still todo!: panel menus (pause menu, episode files, options: panel_base_options.rs, PauseOn, menu CBs).

## Step 5n: hero core (book input path) -- src/modules/hero/ (READ after compression)
- DONE + compiling: hero_core.rs (all 17: ENTRY_POINT, CheckInput/Control/ActionEvents [chain per build from
  tables::CHECK_ACTION_EVENTS], PRE/POSTLOGIC, ValidSwitchLeader, DoTestFor* [climb/crouch share
  `notification_zone`], ResetAction), hero_misc.rs (all but heroFUNC_NextMove; `ray()` helper builds
  RayIntersectIn). hero_power_up.rs = stubs with addr_pu (other-build code addrs + 0x100000, dispatched by
  mod.rs call_code_extra).
- Tools: tools/rustport/hgroups.py <fn>.. (ref body + normalised diff of each shape group), hchain.py <fn>
  (flatten nested event-bit if-chains per group; --bodies, --loose).
- Remaining todo!() (count per file): actions 8, callbacks 8, constructor 8, crouch 10, cutscene 4, edges 11,
  follow 22, inputs 6, ladder 7, misc 1 (NextMove), movements 9, msgfunc 1 (MessageHandler 9172 b), particles 3,
  power_up 6, scared 18, sounds 2, stands 20, switch 4, scooby.rs 5. Order: constructor/scooby/callbacks/inputs
  -> msgfunc -> stands/movements/follow -> scared -> the rest; then a hero smoke test (book open/close).
- DONE: hero_callbacks.rs (all 8). heroCB_ANIM action rules are generated per build into anim_cb.rs by
  tools/rustport/gen_hero_anim_cb.py (fails loudly on unparsed statements; ids like 0xfffff860 are genuine).
  heroCB_DMSTATECHANGE edge guards absent in shape groups 1,2,5.
- DONE: hero_inputs.rs (all 7). DetectEdge: group 1 empty; groups 2/3 add the partner-talk floor zone
  (hero id / IsAnythingBetweenUs distance 0.3 vs 0.35 / text id per group). Next: msgfunc (MessageHandler).
- DONE hero_msgfunc.rs: heroFUNC_MessageHandler, all 57 builds via msg_tab.rs (gen_hero_msg.py: msgs set, hero_a/b,
  m47_to, c1_to, room0, snd86, panic_anim, door_n, door_redirect, item_range, hero_15e, m66_room, door_special).
  0x68 m68_damage flag is cosmetic (param_2 vs stack copy, same value). cargo check clean.
- DONE hero_scared.rs (all 18).
- DONE hero_stands.rs (all 20). c7b040f3 negative-literal slots via kneg(); SelectSleep compare tree -> sequential distinct cases; Run2Stand groups 3/4 hardcoded (1 build each).
- DONE hero_movements.rs (all 9). CreateMovementsPhysic: one 0xb0 frame at the original stack offsets.
- DONE hero_stands.rs (20), hero_movements.rs (9), hero_follow.rs (22). kneg() (c7b040f3 negative anim ids) is
  pub(crate) in hero_stands. heroST_Unhook_Loop keeps the skeleton's i32 param. Unhook reconnect sound by group
  (0: 0xa0000003, 3/4: 0xa0000001, 1/2/5: none; object = slot heroST_Unhook_Loop_0_1).
  Remaining todo!: actions 8, crouch 10, cutscene 4, edges 11, ladder 7, misc 1, particles 3, power_up 6,
  sounds 2, switch 4, mod.rs 1.  Book-first order: power_up, actions, particles, sounds, then the rest.
- DONE (hero, all non-cutscene): actions 8, particles 3, sounds 2 (play_sound.rs via gen_hero_playsound.py),
  power_up, NextMove (nextmove_tab.rs via gen_hero_nextmove.py; moCameraParams static = G(0x104f8)),
  switch 4, crouch 10, ladder 7, edges 11. dm() in hero_constructor is pub(crate).
  heroST_LadderOut_Loop writes Init fns into +0x158 (original quirk, kept).
  Remaining: hero_cutscene.rs 4 (skipped per user), scooby.rs level code of 17 builds (UNPORTED_LOCAL).
  Test: tests/smoke.rs every_hero_build_runs (40 builds, skips UNPORTED_LOCAL).

## Step 5m (cont.): HUD panel in-game path DONE
- Ported: ctor/dtor (base + panel.c via local.rs from gen_panel_local.py + gen_panel_disp.js for GetDisplayInfo),
  Init, CreateInput_Base/_Options, CreateHudWins, FIRSTMOVE, IN_GAME(_SET), SLIDE_IN/OUT, MessageHandler (group 0),
  PauseOff/ShowInGame/MoveMetersOut (group 0), meters/faders, InOut_Obj/ActNotification, FlashButton_*,
  EpFilesObjAddSub, Keys stack (InitKeysStack/SetItemWinPos/ResetHandleTable), PlayPauseIN/OUTAnims,
  InitIngredientWin, NOTIFY_TEXFADER, TIMER_SLIDEINS_DISPLAY.
- Ghidra stale-f0 fixes (documented in fn docs): MessageHandler 0x30/0x41, FlashButton_Update use the call results.
- Remaining todo!(): pause/episode-files/options MENUS (PauseOn, PAUSE states, MENU_* callbacks, EpFiles*,
  LTRIGGER1, TRIANGLE, Options*) and boss meter (group-1 builds). Other shape groups panic "unported shape".
- Tool: tools/rustport/fill.py <target.rs> <bodies.rs> replaces todo stubs from `//@ fn` sections (out/panel_*.rs).
- Test: smoke panel_object_runs_in_game (all builds with ported shapes).

- Panel: boss meter (Show/Move/Update/MessageHandlerBossMeter, panel_base.rs) and panel_frontier.rs (gold/gas meter levels) DONE; build + 31 tests green. Next: scooby.rs UNPORTED_LOCAL (17 level builds, non-book), panel menus.

## Step 5o: hero level code (scooby.c/shaggy.c of the level builds)
- DONE hero_level.rs: 15 builds (doors, rotating doors, go-to-point, sawing, pipe, carrousel/track bump, aqueduct, locked door). Fake code addrs addr_lv (0x2xxxxx) dispatched via mod.rs call_code_extra. scooby.rs routes by hero_level::LEVEL_BUILDS. smoke SKIP lists now only the race builds.
- 26992d64 shaggyCB_ANIM_LF_LOCAL uses the static camera params at G(0x104f8) like heroFUNC_NextMove; DAT_00013b58 = {2, "\MOVIES\EP02\EP02G"}.
- TODO: race builds c33ca2c4 / 0117f115 (hero_race.c 19 fns + their scooby.c/shaggy.c).
- DONE race builds c33ca2c4/0117f115: src/modules/hero/hero_race.rs (hero_race.c + race scooby.c/shaggy.c; fake addrs 0x32xxxx). Quirks kept: Create_Input reads +0x188 of the LEVEL buffer; MSG_ON compares loop ptr with the Init fn (never equal). smoke SKIP lists empty: all hero builds run.

- Step 5p: panel menus (pause/main/rerun/yes-no/level select, EpFiles, options) ported in 4 batches via tools/rustport/menu_brief.md + fill.py; hero_cutscene.rs 4 fns ported. No todo!() left in panel/hero. Quirks noted inline.
- Fix: panel_baseFUNC_Init timer slots used a 16-byte stride for Ghidra vector fields; correct is 12 (0x3f8,0x3f4,0x49c,0x4a0,0x498,0x3fc,0x4a4; +4 in shifted builds). It made LB+0x4a8 (pause-busy flag) a timer handle so the pause menu never opened. Test panel_menus_all_builds now asserts pause opens on all 37 builds.

## Step 5 DONE / Step 6 DONE (final state)
- Step 5 (hero book parts + monster capture/flee/suck) complete: tomedoom, wisps, hero (all 40 builds incl. level/race/cutscene),
  ghost, shadows, zombie, dustdevil, poltergeist, fireghost, zombieboss, mummy, mummyride, fireghostboss, scientist, HUD panel
  (meters, boss meter, level meters, all menus). No todo!() left.
- Step 6: TestEngine (tests/smoke.rs TestHost + engine_with_libs*) + 33 tests (lib unit tests incl. panel lit_strings_present,
  31 smoke, bevy_app). `cargo build` / `cargo test` green.
- Open risks (not covered by tests): EpFiles clue-text table at a fixed image address 0x1002c+sel*0x60; volume setters get
  raw float bits through a u32 API param; group-1 (shifted) builds assume the +4 layout where no slot exists.


## WP-K5a: level camera modules l02_camera / l09_camera

- Coverage before: unlinked camera codes on the disc were l02_camera (fadcfbe3 EP6L02 Ep6_Camera, 2d52c5bf) and
  l09_camera (734d1231, dda18870); l05_camera was taken by another WP. After linking, coverage lists no unported camera code.
- Generated with tools/rustport/c2rs.py, then `python tools/rustport/fix_camera_c2rs.py <C module dir> <b_<sha8>.rs>`
  (the hand fixes of l03_camera/xb applied per build with that build's global addresses): ReplaceOrientation body,
  FloorEvent* dispatch through the `Events` code pointers (floor_ev over the build's ManageEvent*), pEventBuffer value,
  pfVar1[0x51] = -NAN bits, TextWindowSetText varargs, V1 handle-in-float-array, V2 in_f0.
- c2rs: `undefined4` scalar globals only touched by lwc1/swc1 are now typed f32 (fDistanceCameraFollow = 3.6 was 3).
- Float-bit pass (`fix_camera_c2rs.py --bits`): Ghidra undefined4/undefined8 floats came out as value conversions:
  `r32 as f32` -> rf, `f as i64 as u32` -> to_bits (incl. `((uint)x|(uint)y)&0x7fffffff` zero tests),
  `_DAT_00010304` (cvLastFollow.y, undefined8) w64(int)/r64-as-f32 -> wf/rf (clobbered cvLastFollow.z: camera sat on the hero),
  `(float)undefined4` -> from_bits, cv*cv int muls -> float, `fTargetCameraFollowWork = 0x3f99999a` emitted as
  `wf(1067030938.0)` -> w32 bits (target 1e9 m above the hero: camera looked up, flipping 180 deg every frame).
- Verified (game --features game): EP6L02 leader found (hero 0x2a000042), camera 3.3 m behind / 2.6 m above the hero
  looking down, follows when walking/turning (PADSCRIPT up/right); EP6L09 same. EP6L02 zombie 0x2a0001a8 with GAME_BOOKTGT:
  open/hook/mash ok (life -> -1, power 180 -> 112.5), book then stays in FINALSUCKING (zombie state 5) like the EP4L02A zombie.
- Follow-up: l03_camera/xb/b_26ab69f9.rs (old int typing) has the same value-conversion patterns (245 `r32 as f32`); the
  follow path only works there because those globals mostly hold 0 / unclamped pitch. Not re-run through the --bits pass.

## WP-R4: float-global value-conversion audit (all c2rs modules)

- Tool: `python tools/rustport/audit_float_bits.py` -> work/r4/audit.json. For every generated `recomp/src/modules/**/b_*.rs`
  it rebuilds the c2rs `Module` of the source build (header path; `.../` resolved by basename), takes `Module.fdat`
  (HI16/LO16 reloc targets the MIPS code only touches with lwc1/swc1) and flags `e.r32(g(e,A)[+k])` followed by
  `as f32/i32/..` and `let a0 = g(e,A); let a1 = <int/converted value>; e.w32/w64` on those addresses.
  Covers direct global accesses only (not float fields reached through a pointer register). 7 particle modules without
  `_globals.h` are skipped (no globals).
- Real sites found (1190 rs files): l03_camera/xb (the 245 known), candle/xb 0x10018 (halo/lifespan timer frequency:
  `FloatRandom(0.15,0.9) as i64 as u32` stored 0, read back `as f32` -> TimerSetFrequency(0.0)), hs_blockhero 0x10cc4
  (1 read). Everything else is clean; remaining flags in l02/l09/l03 are false positives (w64/w32 of `r64 as u64/u32`
  and CONCAT `to_bits` bit copies).
- Fix: regeneration not used (l03 xb carries hand fixes); l03 xb got the K5a `fix_camera_c2rs.py --bits` pass (388 subs,
  same family as l02/l09) + 11 `((r32(A)).wrapping_sub(r32(B)) [ .wrapping_div(r32(C)) ]) as f32` on lwc1-only globals
  -> `rf(A) - rf(B) [/ rf(C)]`. candle/hs_blockhero hand-edited to rf/wf. Re-audit: 0 real sites.
- Verified: cargo test --release all green except logic_l19 (inv_control_gen bad memory in EP2L09/EP6L05: identical without
  R4, someone else's WIP; engine_r3.rs did not compile (other WIP) and was excluded). 66-level 9000-frame sweep vs
  /tmp/r1_soak.txt: EP1/L03B, EP2/L01C, EP4/L01 (the only packs loading these builds) identical `ok`; other diffs are
  inv_control_gen crashes / stuck counts in packs that do not load R4 modules. Screenshots (game --headless --autoplay
  --frames 900) of the 3 levels: camera behind/above the heroes, normal framing.

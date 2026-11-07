//! P32 R70: the cut-scene host's NotifyAll left `ParticleShared::logic` (a raw pointer to the Engine's
//! LogicRuntime, dereferenced mutably by RuntimeHost::group/s1_obj) set after the call, so it could outlive
//! the Engine. It now restores the previous value (None outside a logic window).
use scooby_recomp::engine::runtime::*;
use scooby_recomp::engine::script::*;

#[test]
fn p32_r70_cutscene_notifyall_restores_logic_ptr() {
    let (mut e, rt) = EngineRuntime::new_with_engine();
    assert!(!rt.particles.lock().unwrap().logic);
    RuntimeCutSceneHost::new(&mut e, &rt).NotifyAll(0x16, 0, 1);
    assert!(!rt.particles.lock().unwrap().logic, "logic pointer left set after cut-scene NotifyAll");
    // inside a window the outer pointer is kept
    rt.particles.lock().unwrap().logic = true;
    RuntimeCutSceneHost::new(&mut e, &rt).NotifyAll(0x16, 0, 0);
    assert!(rt.particles.lock().unwrap().logic);
}

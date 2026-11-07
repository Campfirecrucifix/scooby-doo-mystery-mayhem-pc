//! P32 R75: the host no longer keeps a raw `*const LogicRuntime` (taken from `&e.rt`, written through
//! while `&mut Engine` was live = UB; dangling once the Engine moved). Each Engine API wrapper lends
//! `&mut self.rt` for the call (`LogicBind`); `with_bound` is the only way in, scoped and non-reentrant.
use scooby_recomp::engine::runtime::*;
use scooby_recomp::engine::APIgameobj;
use scooby_recomp::logic::{with_bound, LogicBind};

#[test]
fn p32_r75_logic_lent_only_inside_bind() {
    let (mut e, rt) = EngineRuntime::new_with_engine();
    assert!(with_bound(|_| ()).is_none(), "nothing lent outside a call");
    {
        let _b = LogicBind::new(&mut e.rt);
        assert_eq!(with_bound(|l| with_bound(|_| ()).is_none()), Some(true), "no second &mut while lent");
        assert!(with_bound(|_| ()).is_some(), "restored after use");
    }
    assert!(with_bound(|_| ()).is_none(), "binding released with the guard");
    // an API call through the Engine wrapper leaves nothing bound (no stale pointer survives the call)
    rt.particles.lock().unwrap().logic = true;
    let _ = APIgameobj::GetGroupID(&mut e, 0x2a00_1234);
    rt.particles.lock().unwrap().logic = false;
    assert!(with_bound(|_| ()).is_none());
    // the Engine may move afterwards (load_carried): nothing points into it
    let e2 = e;
    drop(e2);
}

use scooby_recomp::engine::physic::mgr::{CollisionSystem, EFPhysicMgr, EFUpdatePhysicStruct};
use scooby_recomp::engine::physic::model::{EFCollisionModel, ObjPrim, PrimExt};
use scooby_recomp::engine::physic::module::{EFDynamicPhysic, PhysicWorld};
use scooby_recomp::engine::physic::objcoll_check::{EFCheckCollisionIn, EFCheckCollisionOut, EnvPrim};
use scooby_recomp::engine::physic::objcoll_prim::EFObjCollisionPrim;
use scooby_recomp::engine::physic::physic::flags;
use scooby_recomp::engine::physic::rwm::RwMatrix;
use scooby_recomp::engine::physic::system::{EFCollisionSystem, PrimTests};

struct W;
impl PhysicWorld for W {}

/// Deterministic sphere-vs-plane (y=0) / sphere-vs-sphere stand-ins for the prim tests that
/// other workers are finishing.
struct Mock;
impl PrimTests for Mock {
    fn CheckEnvCollision(&self, p: &ObjPrim, i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, e: &EnvPrim) -> u32 {
        let r = p.base.radius;
        let y = p.base.ltm[3][1] - e.d;
        o.normal = e.normal;
        if y < r {
            o.depth = r - y;
            return 2;
        }
        let vy = i.motion[1];
        if vy < 0.0 {
            let t = (r - y) / vy;
            if t <= i.tmax {
                o.t = t;
                return 1;
            }
        }
        0
    }
    fn CheckObjPrimCollision(&self, p: &ObjPrim, _i: &EFCheckCollisionIn, o: &mut EFCheckCollisionOut, q: &ObjPrim) -> u32 {
        let a = p.base.ltm[3];
        let b = q.base.ltm[3];
        let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        if l < p.base.radius + q.base.radius {
            o.normal = [d[0] / l, d[1] / l, d[2] / l];
            o.depth = p.base.radius + q.base.radius - l;
            return 2;
        }
        0
    }
}

fn sphere(ltm: [[f32; 4]; 4], r: f32, pflags: u32) -> ObjPrim {
    let mut b = EFObjCollisionPrim::new(ltm);
    b.radius = r;
    b.set_word(0xc, pflags);
    b.set_word(0x10, 1);
    b.set_word(0x14, 1);
    b.set_word(0x20, 1);
    ObjPrim { base: b, ext: PrimExt::Sphere }
}

fn add(mgr: &mut EFPhysicMgr, sys: &mut EFCollisionSystem, owner: u32, pos: [f32; 3], r: f32, pflags: u32) -> EFDynamicPhysic {
    let mut f = RwMatrix::identity();
    f.pos = pos;
    let id = mgr.CreatePhysic(owner, f, 0);
    mgr.PhysicMut(id).unwrap().flags |= flags::COLLIDE;
    let mut m = EFCollisionModel::new(1);
    m.prims = vec![Some(sphere([[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [pos[0], pos[1], pos[2], 1.0]], r, pflags))];
    let _ = &sys;
    mgr.PhysicMut(id).unwrap().model = m;
    EFDynamicPhysic { physic: id, sphere: pos, radius: r + 1.0, ..Default::default() }
}

fn floor() -> EnvPrim {
    EnvPrim { id: 1, mat: [0, 1, 0, 0, 0, 0, 0, 0], w4: 0, nverts: 0, normal: [0.0, 1.0, 0.0], d: 0.0, verts: vec![], edges: vec![] }
}

#[test]
fn sphere_falls_onto_floor_and_stops() {
    let mut mgr = EFPhysicMgr::new();
    let mut sys = EFCollisionSystem::new();
    sys.tests = Box::new(Mock);
    let d = add(&mut mgr, &mut sys, 1, [0.0, 1.5, 0.0], 1.0, 1);
    let id = d.physic;
    mgr.PhysicMut(id).unwrap().velocity = [0.0, -10.0, 0.0];
    let mut ups = EFUpdatePhysicStruct { dt: 0.1, passes: [vec![d], vec![]], emitter_count: 0 };
    sys.SyncModels(&mut mgr, &W);
    sys.Init(&mut mgr, &mut ups, 0, 0);
    sys.stages[0].env = vec![floor()];
    sys.Update(&mut mgr, &mut ups, &W);
    let p = mgr.Physic(id).unwrap();
    assert!(p.velocity[1] > -1e-3, "vertical velocity cancelled: {:?}", p.velocity);
    assert!(p.frame.pos[1] >= 0.99, "stopped on the floor: {:?}", p.frame.pos);
    sys.Terminate(&mut ups);
}

#[test]
fn penetrating_sphere_is_pushed_out() {
    let mut mgr = EFPhysicMgr::new();
    let mut sys = EFCollisionSystem::new();
    sys.tests = Box::new(Mock);
    let d = add(&mut mgr, &mut sys, 1, [0.0, 0.5, 0.0], 1.0, 1);
    let id = d.physic;
    let mut ups = EFUpdatePhysicStruct { dt: 0.1, passes: [vec![d], vec![]], emitter_count: 0 };
    sys.SyncModels(&mut mgr, &W);
    sys.Init(&mut mgr, &mut ups, 0, 0);
    sys.stages[0].env = vec![floor()];
    sys.CheckCollisions(&mut mgr, &mut ups, &W);
    assert!(mgr.Physic(id).unwrap().frame.pos[1] > 0.5, "{:?}", mgr.Physic(id).unwrap().frame.pos);
}

#[test]
fn notify_flags_produce_messages() {
    let mut mgr = EFPhysicMgr::new();
    let mut sys = EFCollisionSystem::new();
    sys.tests = Box::new(Mock);
    // hero prim: flag 4 (notify self) ; monster prim: no-push (2) + flag 8 (notify other)
    let a = add(&mut mgr, &mut sys, 10, [0.0, 5.0, 0.0], 1.0, 1 | 4);
    let b = add(&mut mgr, &mut sys, 20, [1.0, 5.0, 0.0], 1.0, 2 | 8);
    let mut ups = EFUpdatePhysicStruct { dt: 0.1, passes: [vec![a, b], vec![]], emitter_count: 0 };
    sys.Solve(&mut mgr, &mut ups, 0, 0, &W);
    let n = &sys.notifies;
    assert!(!n.is_empty(), "no notify");
    assert!(n.iter().any(|&(to, id, m)| to == 10 && id == 0x17 && m.hit == 1 && m.other_uid == 20));
    assert!(n.iter().any(|&(to, id, m)| to == 10 && id == 3 && m.hit == 0 && m.other_uid == 20));
}

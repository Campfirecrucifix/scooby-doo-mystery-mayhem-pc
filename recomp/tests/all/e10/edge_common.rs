// shared helpers for the edge_* tests (included by each harness)
use scooby_recomp::engine::physic::model::*;
use scooby_recomp::engine::physic::module::*;
use scooby_recomp::engine::physic::objcoll_check::EnvPrim;
use scooby_recomp::engine::physic::objcoll_prim::{EFObjCollisionPrim, IDENTITY, PRIM_SPHERE, PRIM_POLYGON};
use scooby_recomp::engine::physic::rwm::{V3, add, scale};
use scooby_recomp::engine::physic::physic::*;
use scooby_recomp::engine::physic::rwm::*;

pub fn close(a: V3, b: V3) -> bool { (0..3).all(|i| (a[i] - b[i]).abs() < 1e-3) }

/// owner physic at `pos` (identity rotation) with one sphere prim (tag 7, mask1 1) offset `off`.
pub fn owner(pos: V3, off: V3, r: f32) -> EFPhysic {
    let mut p = EFPhysic::new(5, RwMatrix::from_pos(pos));
    p.model = EFCollisionModel::new(1);
    let mut l = IDENTITY;
    l[3] = [pos[0], pos[1], pos[2], 1.0];
    let s = p.model.AddPrimitive(0, 7, PRIM_SPHERE, l).unwrap();
    s.base.radius = r;
    s.base.offset = off;
    s.base.set_word(0x20, 1);
    s.base.set_word(0x14, 0x200);
    s.PrecalcPrimData();
    p
}

pub fn center(p: &EFPhysic) -> V3 { p.model.prims[0].as_ref().unwrap().base.center }

/// env polygon (material mask1 = 1, all edges flagged)
pub fn poly(verts: Vec<V3>, normal: V3, d: f32, edges: Vec<[f32; 4]>) -> EnvPrim {
    let mut mat = [0u32; 8];
    mat[1] = 1;
    EnvPrim { id: 0, mat, w4: 0xffff, nverts: verts.len() as u16, normal, d, verts, edges }
}

/// horizontal unit square at y=0 (ledge top)
pub fn ledge() -> EnvPrim {
    poly(vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 0.0, 1.0]], [0.0, 1.0, 0.0], 0.0, vec![])
}

pub fn with<R>(p: &mut EFPhysic, frame: u32, w: &dyn PhysicWorld, f: impl FnOnce(&mut DMCtx) -> R) -> (R, Vec<PhysicNotify>) {
    let mut n = Vec::new();
    let r = {
        let mut c = DMCtx { phys: p, frame, paused_frames: 0, prev_paused: 0, world: w, notify: &mut n };
        f(&mut c)
    };
    (r, n)
}

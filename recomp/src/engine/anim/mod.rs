//! WP-D: animation (EFAnimRes/EFHAnimRes data + EFAnimationMgr runtime). See ENGINE_PORT.md.
#![allow(non_snake_case)]

pub mod res;
pub mod nodes;
pub mod mgr;
pub mod quat;
pub mod texture_node;
pub mod api;
pub mod morph;
pub mod coll_blend;
pub mod coll_prim;

use crate::engine::pack::{DTDynamicInstanciator, DTObj, DTStreamable, RawInstanciator};

/// Instanciator adding the WP-D resource classes (0x13, 0x2c, 0x49, 0x4f) on top of
/// [`RawInstanciator`] (the coordinator can fold this into level::CreateInstance).
#[derive(Default)]
pub struct AnimInstanciator(pub RawInstanciator);

impl DTDynamicInstanciator for AnimInstanciator {
    fn Instanciate(&mut self, class_id: u32) -> Option<Box<dyn DTStreamable>> {
        use res::*;
        let o: Box<dyn crate::engine::dt::property::DTObject> = match class_id {
            EFAnimRes::CLASS_ID => Box::new(EFAnimRes::default()),
            EFHAnimRes::CLASS_ID => Box::new(EFHAnimRes::default()),
            EFTextureAnimRes::CLASS_ID => Box::new(EFTextureAnimRes::default()),
            EFMorphAnimRes::CLASS_ID => Box::new(EFMorphAnimRes::default()),
            _ => return self.0.Instanciate(class_id),
        };
        Some(Box::new(DTObj(o)))
    }
}

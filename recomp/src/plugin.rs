//! Bevy integration: the [`Engine`] is a `Resource`, the original main loop's per-frame order is a
//! chain of system sets, and every game object is mirrored by an entity carrying [`GameObjectId`].
//!
//! Frame order (EFEngineController main loop / EFWorld::Update), reproduced by [`EngineSet`]:
//! 1. [`EngineSet::Input`]: platform input (raises input notifications)
//! 2. [`EngineSet::Logic`]: filtered frame time, then EFGameLogicMgr::Update (every object's logic)
//! 3. [`EngineSet::Animation`]: platform animation (raises animation notifications)
//! 4. [`EngineSet::PostAnimation`]: FlushNotification (deferred notifications)
//! 5. [`EngineSet::Physics`]: platform physics and particles (raise collision notifications)
//! 6. [`EngineSet::Finish`]: kill list, PrepareRender's logic part (next frame's object list +
//!    FlushNotification), then entity sync
//!
//! Threading (implicit in the original, preserved): the game runs every logic module on one thread
//! and the modules share memory freely, so all engine systems take `ResMut<Engine>` and therefore
//! never run in parallel with each other. No system does blocking I/O: module code is compiled in,
//! and the platform hooks ([`crate::engine::Host`]) must not block either.
use crate::engine::{Engine, Host};
use crate::logic::{ObjH, WorldEvent};
use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_time::Time;
use std::collections::HashMap;
use std::sync::Mutex;

/// Phases of one engine frame, chained in this order in `Update`.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EngineSet {
    Input,
    Logic,
    Animation,
    PostAnimation,
    Physics,
    Finish,
}

/// The game object an entity stands for (EFGameObject id).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GameObjectId(pub u32);

/// The runtime handle of that object (index into `Engine::rt.objects`).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GameObjectHandle(pub ObjH);

/// Object handle -> its entity.
#[derive(Resource, Default, Debug)]
pub struct GameObjectEntities(pub HashMap<ObjH, Entity>);

/// Written by the renderer each frame: the ids of the objects whose clip flags lack 0x400
/// (visible or forced), oldest object first. Only those objects run their logic next frame.
/// `None` = no camera, in which case the list is not rebuilt (as in EFWorld::PrepareRender).
#[derive(Resource, Default, Debug)]
pub struct LogicClipList(pub Option<Vec<u32>>);

/// Adds the engine. The [`Host`] (rendering, sound, physics...) is given to the plugin and moved
/// into the [`Engine`] resource when the plugin is built. Modules and objects are loaded by the
/// game layer afterwards (`Engine::load_module` / `add_object`, from a startup system).
pub struct ScoobyEnginePlugin {
    host: Mutex<Option<Box<dyn Host>>>,
}

impl ScoobyEnginePlugin {
    pub fn new(host: Box<dyn Host>) -> Self {
        ScoobyEnginePlugin { host: Mutex::new(Some(host)) }
    }
}

impl Plugin for ScoobyEnginePlugin {
    fn build(&self, app: &mut App) {
        let host = self.host.lock().unwrap().take().expect("ScoobyEnginePlugin built twice");
        app.insert_resource(Engine::new(host))
            .init_resource::<GameObjectEntities>()
            .init_resource::<LogicClipList>()
            .configure_sets(
                Update,
                (
                    EngineSet::Input,
                    EngineSet::Logic,
                    EngineSet::Animation,
                    EngineSet::PostAnimation,
                    EngineSet::Physics,
                    EngineSet::Finish,
                )
                    .chain(),
            )
            .add_systems(Update, engine_logic.in_set(EngineSet::Logic))
            .add_systems(Update, engine_flush_notifications.in_set(EngineSet::PostAnimation))
            .add_systems(Update, (engine_finish_frame, sync_game_objects).chain().in_set(EngineSet::Finish));
    }
}

/// EFEngineController::ComputeDeltaTime + EFGameLogicMgr::Update. Needs a `Time` resource
/// (bevy_time's TimePlugin); without one, logic steps with the filter's minimum.
pub fn engine_logic(mut e: ResMut<Engine>, time: Option<Res<Time>>) {
    let raw = time.map_or(0.0, |t| t.delta_secs());
    let dt = e.clock.ComputeDeltaTime(raw);
    e.GameLogicUpdate(dt);
}

/// The FlushNotification in EFWorld::Update between animation and physics
pub fn engine_flush_notifications(mut e: ResMut<Engine>) {
    e.FlushNotification();
}

/// The end of EFWorld::Update (kill list) and the logic part of EFWorld::PrepareRender
pub fn engine_finish_frame(mut e: ResMut<Engine>, mut clip: ResMut<LogicClipList>) {
    e.ProcessKillList();
    let list = clip.0.take();
    e.PrepareRender_Logic(list.as_deref());
}

/// Spawns/despawns one entity per game object (added or deleted this frame)
pub fn sync_game_objects(mut commands: Commands, mut e: ResMut<Engine>, mut map: ResMut<GameObjectEntities>) {
    for ev in std::mem::take(&mut e.rt.events) {
        match ev {
            WorldEvent::Added { id, handle } => {
                let ent = commands.spawn((GameObjectId(id), GameObjectHandle(handle))).id();
                if let Some(old) = map.0.insert(handle, ent) {
                    commands.entity(old).despawn();
                }
            }
            WorldEvent::Deleted { handle, .. } => {
                if let Some(ent) = map.0.remove(&handle) {
                    commands.entity(ent).despawn();
                }
            }
        }
    }
}

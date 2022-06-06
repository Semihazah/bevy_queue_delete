use std::sync::Arc;

use bevy::{
    ecs::{reflect::ReflectComponent},
    prelude::{App, Commands, Component, Entity, Plugin, Res, Query},
    reflect::{FromReflect, Reflect},
    utils::HashMap,
};
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use parking_lot::{Mutex, RwLock};

use crate::QueueDelete;

pub struct RefEntityPlugin;

impl Plugin for RefEntityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RefEntityServer>()
            .add_system(free_unused_entities)
            .add_system(mark_unused_entities);
    }
}
// *****************************************************************************************
// Resources
// *****************************************************************************************
#[derive(Default)]
pub struct RefEntityServer {
    channel: Arc<RefChangeChannel>,
    ref_counts: Arc<RwLock<HashMap<Entity, usize>>>,
    mark_unused_assets: Arc<Mutex<Vec<Entity>>>,
}

impl RefEntityServer {
    pub fn get_handle<I: Into<Entity>>(
        &self,
        id: I,
    ) -> RefEntityHandle {
        let sender = self.channel.sender.clone();
        RefEntityHandle::strong(id.into(), sender)
    }
}

// *****************************************************************************************
// Systems
// *****************************************************************************************
fn free_unused_entities(mut commands: Commands, server: Res<RefEntityServer>, valid_query: Query<Entity>) {
    let mut potential_frees = server.mark_unused_assets.lock();
    if !potential_frees.is_empty() {
        let ref_counts = server.ref_counts.read();
        for potential_free in potential_frees.drain(..) {
            if let Some(&0) = ref_counts.get(&potential_free) {
                if valid_query.get(potential_free).is_ok() {
                    commands.entity(potential_free).insert(QueueDelete);
                }
            }
        }
    }
}

fn mark_unused_entities(server: Res<RefEntityServer>) {
    let receiver = &server.channel.receiver;
    let mut ref_counts = server.ref_counts.write();
    let mut potential_frees = None;
    loop {
        let ref_change = match receiver.try_recv() {
            Ok(ref_change) => ref_change,
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => panic!("RefChange channel disconnected."),
        };
        match ref_change {
            RefChange::Increment(handle_id) => *ref_counts.entry(handle_id).or_insert(0) += 1,
            RefChange::Decrement(handle_id) => {
                let entry = ref_counts.entry(handle_id.clone()).or_insert(0);
                *entry -= 1;
                if *entry <= 0 {
                    potential_frees
                        .get_or_insert_with(|| server.mark_unused_assets.lock())
                        .push(handle_id.clone());
                    ref_counts.remove(&handle_id);
                }
            }
        }
    }
}

// *****************************************************************************************
// Structs
// *****************************************************************************************
enum RefCompHandleType {
    Weak,
    Strong(Sender<RefChange>),
}

impl core::fmt::Debug for RefCompHandleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefCompHandleType::Weak => f.write_str("Weak"),
            RefCompHandleType::Strong(_) => f.write_str("Strong"),
        }
    }
}

impl Default for RefCompHandleType {
    fn default() -> Self {
        Self::Weak
    }
}

impl RefEntityHandle {
    fn strong(entity: Entity, ref_change_sender: Sender<RefChange>) -> Self {
        ref_change_sender
            .send(RefChange::Increment(entity.clone()))
            .unwrap();
        Self {
            entity,
            handle_type: RefCompHandleType::Strong(ref_change_sender),
        }
    }

    #[inline]
    pub fn weak(entity: Entity) -> Self {
        Self {
            entity,
            handle_type: RefCompHandleType::Weak,
        }
    }

    /// Get a copy of this handle as a Weak handle
    pub fn as_weak(&self) -> RefEntityHandle {
        RefEntityHandle {
            entity: self.entity.clone(),
            handle_type: RefCompHandleType::Weak,
        }
    }

    pub fn is_weak(&self) -> bool {
        matches!(self.handle_type, RefCompHandleType::Weak)
    }

    pub fn is_strong(&self) -> bool {
        matches!(self.handle_type, RefCompHandleType::Strong(_))
    }

    /// Makes this handle Strong if it wasn't already.
    ///
    /// This method requires the corresponding [Assets](crate::Assets) collection
    pub fn make_strong(&mut self, server: &RefEntityServer) {
        if self.is_strong() {
            return;
        }
        let sender = server.channel.sender.clone();
        sender.send(RefChange::Increment(self.entity.clone())).unwrap();
        self.handle_type = RefCompHandleType::Strong(sender);
    }

    #[inline]
    pub fn clone_weak(&self) -> Self {
        RefEntityHandle::weak(self.entity.clone())
    }
}

impl Drop for RefEntityHandle {
    fn drop(&mut self) {
        match self.handle_type {
            RefCompHandleType::Strong(ref sender) => {
                // ignore send errors because this means the channel is shut down / the game has
                // stopped
                let _ = sender.send(RefChange::Decrement(self.entity.clone()));
            }
            RefCompHandleType::Weak => {}
        }
    }
}

#[derive(Component, Reflect, FromReflect, Debug)]
#[reflect(Component)]
pub struct RefEntityHandle {
    pub entity: Entity,

    #[reflect(ignore)]
    handle_type: RefCompHandleType,
}

impl Default for RefEntityHandle {
    fn default() -> Self {
        RefEntityHandle::weak(Entity::from_raw(u32::MAX))
    }
}

impl Clone for RefEntityHandle {
    fn clone(&self) -> Self {
        match self.handle_type {
            RefCompHandleType::Strong(ref sender) => {
                RefEntityHandle::strong(self.entity.clone(), sender.clone())
            }
            RefCompHandleType::Weak => RefEntityHandle::weak(self.entity.clone()),
        }
    }
}

enum RefChange {
    Increment(Entity),
    Decrement(Entity),
}

#[derive(Clone)]
struct RefChangeChannel {
    sender: Sender<RefChange>,
    receiver: Receiver<RefChange>,
}

impl Default for RefChangeChannel {
    fn default() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        RefChangeChannel { sender, receiver }
    }
}
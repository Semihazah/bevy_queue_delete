use bevy::{prelude::{Entity, World}, ecs::{system::{Command, EntityCommands}, world::EntityMut}, hierarchy::Children};

use crate::QueueDelete;


/// Inserts QueueDelete into the given entity and all its children recursively
#[derive(Debug)]
struct QueueDespawnRecursive(Entity);

/// Inserts QueueDelete into the given entity's children recursively
#[derive(Debug)]
struct QueueDespawnChildrenRecursive(Entity);

/// Function for inserting QueueDelete into an entity and all its children
pub fn queue_despawn_with_children_recursive(world: &mut World, entity: Entity) {
    queue_despawn_with_children_recursive_inner(world, entity);
}

fn queue_despawn_with_children_recursive_inner(world: &mut World, entity: Entity) {
    if let Some(children) = world.get::<Children>(entity).cloned() {
        for e in children.into_iter() {
            queue_despawn_with_children_recursive(world, *e);
        }
    }

    world.entity_mut(entity).insert(QueueDelete);
}

fn queue_despawn_children(world: &mut World, entity: Entity) {
    if let Some(children) = world.get::<Children>(entity).cloned() {
        for e in children.into_iter() {
            queue_despawn_with_children_recursive_inner(world, *e);
        }
    }

    world.entity_mut(entity).insert(QueueDelete);
}

impl Command for QueueDespawnRecursive {
    fn write(self, world: &mut bevy::prelude::World) {
        queue_despawn_with_children_recursive(world, self.0);
    }
}

impl Command for QueueDespawnChildrenRecursive {
    fn write(self, world: &mut World) {
        queue_despawn_children(world, self.0);
    }
}

/// Trait that holds functions for inserting QueueDelete recursively down the transform hierarchy
pub trait QueueDespawnRecursiveExt {
    fn queue_despawn_recursive(&mut self);
    fn queue_despawn_descendants(&mut self);
}

impl<'w, 's, 'a> QueueDespawnRecursiveExt for EntityCommands<'w, 's, 'a> {
    fn queue_despawn_recursive(&mut self) {
        let entity = self.id();
        self.commands().add(QueueDespawnRecursive(entity));
    }

    fn queue_despawn_descendants(&mut self) {
        let entity = self.id();
        self.commands().add(QueueDespawnChildrenRecursive(entity));
    }
}

impl<'w> QueueDespawnRecursiveExt for EntityMut<'w> {
    fn queue_despawn_recursive(&mut self) {
        let entity = self.id();
        unsafe {
            queue_despawn_with_children_recursive(self.world_mut(), entity);
            self.update_location();
        }
    }

    fn queue_despawn_descendants(&mut self) {
        let entity = self.id();
        unsafe {
            queue_despawn_children(self.world_mut(), entity);
            self.update_location();
        }
    }
}
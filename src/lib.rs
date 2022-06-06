use std::time::Duration;

use bevy::{
    core::{Time, Timer},
    ecs::{reflect::ReflectComponent},
    prelude::{
        Added, Commands, Component, CoreStage, Entity,
        ParallelSystemDescriptorCoercion, Plugin, Query, Res, SystemLabel, With,
    },
    reflect::{FromReflect, Reflect},
};

mod recursive;
pub use recursive::{QueueDespawnRecursiveExt, queue_despawn_with_children_recursive};

#[cfg(feature = "ref_delete")]
pub mod ref_delete;

pub struct BevyQueueDeletePlugin;

impl Plugin for BevyQueueDeletePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.register_type::<QueueDelete>()
            .add_system_to_stage(CoreStage::Last, queue_delete_system.label(QueueDelete))
            .register_type::<TimerDelete>()
            .add_system(timer_delete_system)
            .add_system(timer_start_fn)
            .register_type::<FrameCountDelete>()
            .add_system(frame_count_delete_system)
        ;

        #[cfg(feature = "ref_delete")]
        app.add_plugin(ref_delete::RefEntityPlugin);
    }
}

/// Automatically despawns entities in CoreStage::Last
/// Use QueueDelete as a Label so that cleanup can happen right before
/// Use queue_despawn_recursive and queue_despawn_descendents to insert into children
#[derive(
    Reflect, FromReflect, Component, Clone, Default, Hash, Debug, PartialEq, Eq, SystemLabel,
)]
#[reflect(Component)]
#[component(storage = "SparseSet")]
pub struct QueueDelete;

fn queue_delete_system(mut commands: Commands, query: Query<Entity, With<QueueDelete>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

/// Automatically insert QueueDelete after a set amount of time
#[derive(Reflect, Component, Clone, Default, Debug)]
#[reflect(Component)]
pub struct TimerDelete {
    pub duration: Duration,
    pub timer: Timer,
}

impl TimerDelete {
    pub fn new(duration: Duration) -> Self {
        TimerDelete {
            duration,
            timer: Timer::default(),
        }
    }
}

fn timer_delete_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut TimerDelete)>,
) {
    for (entity, mut timer) in query.iter_mut() {
        timer.timer.tick(time.delta());
        if timer.timer.just_finished() {
            commands.entity(entity).insert(QueueDelete);
        }
    }
}

fn timer_start_fn(mut query: Query<&mut TimerDelete, Added<TimerDelete>>) {
    for mut timer in query.iter_mut() {
        let duration = timer.duration;
        timer.timer.set_duration(duration);
    }
}

/// Automatically inserts QueueDelete after number of frames
#[derive(Reflect, FromReflect, Component, Clone, Default, Debug)]
#[reflect(Component)]
pub struct FrameCountDelete(pub u64);

pub fn frame_count_delete_system(
    mut commands: Commands,
    mut query: Query<(Entity, &mut FrameCountDelete)>,
) {
    for (entity, mut frames) in query.iter_mut() {
        frames.0 -= 1;
        if frames.0 <= 0 {
            commands.entity(entity).insert(QueueDelete);
        }
    }
}
use std::time::Duration;

use bevy::{
    core::{Time, Timer},
    ecs::reflect::ReflectComponent,
    prelude::{
        Added, Commands, Component, CoreStage, DespawnRecursiveExt, Entity,
        ParallelSystemDescriptorCoercion, Plugin, Query, Res, SystemLabel, With,
    },
    reflect::{FromReflect, Reflect},
};

pub struct BevyQueueDeletePlugin;

impl Plugin for BevyQueueDeletePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.register_type::<QueueDelete>()
            .add_system_to_stage(CoreStage::Last, queue_delete_system.label(QueueDelete))
            .register_type::<TimerDelete>()
            .add_system(timer_delete_system)
            .add_system(timer_start_fn)
            .register_type::<FrameCountDelete>()
            .add_system(frame_count_delete_system);
    }
}

// Use this to delete effects so that they can properly remove themselves from stat entities
// Despawns at CoreStage::Last.
#[derive(
    Reflect, FromReflect, Component, Clone, Default, Hash, Debug, PartialEq, Eq, SystemLabel,
)]
#[reflect(Component)]
pub struct QueueDelete;

pub fn queue_delete_system(mut commands: Commands, query: Query<Entity, With<QueueDelete>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

#[derive(Reflect, FromReflect, Component, Clone, Default, Debug)]
#[reflect(Component)]
pub struct TimerDelete {
    pub duration: f32,

    #[reflect(ignore)]
    pub timer: Timer,
}

impl TimerDelete {
    pub fn new(duration: f32) -> Self {
        TimerDelete {
            duration,
            timer: Timer::default(),
        }
    }
}

pub fn timer_delete_system(
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

pub fn timer_start_fn(mut query: Query<&mut TimerDelete, Added<TimerDelete>>) {
    for mut timer in query.iter_mut() {
        let duration = timer.duration;
        timer.timer.set_duration(Duration::from_secs_f32(duration));
    }
}

#[derive(Reflect, FromReflect, Component, Clone, Default, Debug)]
#[reflect(Component)]
pub struct FrameCountDelete(pub usize);

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

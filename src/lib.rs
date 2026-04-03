mod components;
mod config;
mod curve;
mod debug;
mod messages;
mod resources;
mod solver;
mod systems;

pub use components::{
    CinematicCameraBinding, CinematicCameraBrain, CinematicCameraRig, CinematicCameraState,
    CinematicDrivenCamera, CinematicPlayback, CinematicPlaybackStatus, CinematicSequence,
    CinematicShot, CinematicTargetGroup, CinematicVirtualCamera, LensTrack, LookAtTarget,
    OrientationTrack, PathTangentOrientation, PositionTrack, RailTrack, WeightedTarget,
};
pub use config::{
    CinematicBlend, CinematicEasing, MarkerTime, PlaybackLoopMode, ProceduralShake, ShotMarker,
    UpVectorMode,
};
pub use curve::{
    CinematicRail, CinematicRailCache, RailProgressUnit, RailSample, RailSplineKind, RailTraversal,
};
pub use messages::{
    CinematicBlendCompleted, CinematicBlendKind, CinematicPlaybackCommand, SequenceFinished,
    ShotFinished, ShotMarkerReached, ShotStarted,
};
pub use resources::{CinematicCameraDebugSettings, CinematicCameraDiagnostics};

use bevy::{
    app::PostStartup,
    ecs::{intern::Interned, schedule::ScheduleLabel},
    prelude::*,
};

#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum CinematicCameraSystems {
    InputOrCommands,
    AdvanceTimeline,
    SolveRig,
    ApplyCamera,
    Debug,
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct NeverDeactivateSchedule;

pub struct CinematicCameraPlugin {
    pub activate_schedule: Interned<dyn ScheduleLabel>,
    pub deactivate_schedule: Interned<dyn ScheduleLabel>,
    pub update_schedule: Interned<dyn ScheduleLabel>,
}

impl CinematicCameraPlugin {
    pub fn new(
        activate_schedule: impl ScheduleLabel,
        deactivate_schedule: impl ScheduleLabel,
        update_schedule: impl ScheduleLabel,
    ) -> Self {
        Self {
            activate_schedule: activate_schedule.intern(),
            deactivate_schedule: deactivate_schedule.intern(),
            update_schedule: update_schedule.intern(),
        }
    }

    pub fn always_on(update_schedule: impl ScheduleLabel) -> Self {
        Self::new(PostStartup, NeverDeactivateSchedule, update_schedule)
    }
}

impl Default for CinematicCameraPlugin {
    fn default() -> Self {
        Self::always_on(Update)
    }
}

impl Plugin for CinematicCameraPlugin {
    fn build(&self, app: &mut App) {
        if self.deactivate_schedule == NeverDeactivateSchedule.intern() {
            app.init_schedule(NeverDeactivateSchedule);
        }

        app.init_resource::<resources::CinematicCameraRuntimeState>()
            .init_resource::<resources::TargetHistory>()
            .init_resource::<CinematicCameraDebugSettings>()
            .init_resource::<CinematicCameraDiagnostics>()
            .add_message::<CinematicPlaybackCommand>()
            .add_message::<ShotStarted>()
            .add_message::<ShotFinished>()
            .add_message::<ShotMarkerReached>()
            .add_message::<SequenceFinished>()
            .add_message::<CinematicBlendCompleted>()
            .register_type::<CinematicBlend>()
            .register_type::<CinematicCameraBinding>()
            .register_type::<CinematicCameraBrain>()
            .register_type::<CinematicCameraDebugSettings>()
            .register_type::<CinematicCameraDiagnostics>()
            .register_type::<CinematicCameraRig>()
            .register_type::<CinematicCameraState>()
            .register_type::<CinematicVirtualCamera>()
            .register_type::<CinematicDrivenCamera>()
            .register_type::<CinematicEasing>()
            .register_type::<CinematicPlayback>()
            .register_type::<CinematicPlaybackStatus>()
            .register_type::<CinematicRail>()
            .register_type::<CinematicSequence>()
            .register_type::<CinematicShot>()
            .register_type::<CinematicTargetGroup>()
            .register_type::<LensTrack>()
            .register_type::<LookAtTarget>()
            .register_type::<MarkerTime>()
            .register_type::<OrientationTrack>()
            .register_type::<PathTangentOrientation>()
            .register_type::<PlaybackLoopMode>()
            .register_type::<PositionTrack>()
            .register_type::<ProceduralShake>()
            .register_type::<RailProgressUnit>()
            .register_type::<RailSplineKind>()
            .register_type::<RailTrack>()
            .register_type::<RailTraversal>()
            .register_type::<ShotMarker>()
            .register_type::<UpVectorMode>()
            .register_type::<WeightedTarget>()
            .add_systems(self.activate_schedule, systems::activate_runtime)
            .add_systems(self.deactivate_schedule, systems::deactivate_runtime)
            .configure_sets(
                self.update_schedule,
                (
                    CinematicCameraSystems::InputOrCommands,
                    CinematicCameraSystems::AdvanceTimeline,
                    CinematicCameraSystems::SolveRig,
                    CinematicCameraSystems::ApplyCamera,
                    CinematicCameraSystems::Debug,
                )
                    .chain(),
            )
            .add_systems(
                self.update_schedule,
                (
                    (
                        systems::sync_virtual_camera_authoring,
                        systems::ensure_runtime_components,
                        systems::rebuild_sequence_caches,
                        systems::refresh_target_history,
                        systems::apply_playback_commands,
                        systems::autoplay_rigs,
                    )
                        .chain()
                        .in_set(CinematicCameraSystems::InputOrCommands),
                    systems::advance_timeline.in_set(CinematicCameraSystems::AdvanceTimeline),
                    systems::solve_rigs.in_set(CinematicCameraSystems::SolveRig),
                    (systems::apply_camera_bindings, systems::publish_diagnostics)
                        .chain()
                        .in_set(CinematicCameraSystems::ApplyCamera),
                )
                    .run_if(systems::runtime_is_active),
            );

        if app.is_plugin_added::<bevy::gizmos::GizmoPlugin>() {
            app.add_systems(
                self.update_schedule,
                debug::draw_debug_gizmos
                    .in_set(CinematicCameraSystems::Debug)
                    .run_if(systems::runtime_is_active),
            );
        }
    }
}

#[cfg(test)]
#[path = "plugin_tests.rs"]
mod plugin_tests;

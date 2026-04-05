use std::collections::HashMap;

use bevy::prelude::*;

#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct CinematicCameraDebugSettings {
    pub enabled: bool,
    pub draw_paths: bool,
    pub draw_control_points: bool,
    pub draw_targets: bool,
    pub draw_camera_forward: bool,
    pub draw_active_samples: bool,
}

impl Default for CinematicCameraDebugSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            draw_paths: true,
            draw_control_points: true,
            draw_targets: true,
            draw_camera_forward: true,
            draw_active_samples: true,
        }
    }
}

#[derive(Resource, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Resource)]
pub struct CinematicCameraDiagnostics {
    pub active_rigs: usize,
    pub previewable_rigs: usize,
    pub applied_cameras: usize,
    pub target_history_entries: usize,
}

#[derive(Resource, Default)]
pub(crate) struct CinematicCameraRuntimeState {
    pub active: bool,
    pub debug_root: Option<Entity>,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TargetMotionState {
    pub position: Vec3,
    pub velocity: Vec3,
    pub initialized: bool,
}

#[derive(Resource)]
pub(crate) struct TargetHistory {
    pub entries: HashMap<Entity, TargetMotionState>,
    /// Velocity smoothing decay rate. Higher = faster convergence, lower = smoother.
    /// Applied as exponential smoothing: `v = lerp(v_old, v_raw, 1 - exp(-rate * dt))`.
    pub velocity_smoothing_rate: f32,
}

impl Default for TargetHistory {
    fn default() -> Self {
        Self {
            entries: HashMap::default(),
            velocity_smoothing_rate: 12.0,
        }
    }
}

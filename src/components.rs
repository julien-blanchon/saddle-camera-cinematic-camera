use bevy::{camera::PerspectiveProjection, prelude::*, reflect::Reflect};

use crate::{
    CinematicBlend, CinematicEasing, CinematicRail, PlaybackLoopMode, ProceduralShake,
    RailTraversal, ShotMarker, UpVectorMode,
};

#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct CinematicCameraRig {
    pub auto_play: bool,
    pub enabled: bool,
}

impl Default for CinematicCameraRig {
    fn default() -> Self {
        Self {
            auto_play: false,
            enabled: true,
        }
    }
}

#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct CinematicVirtualCamera {
    pub brain: Entity,
    pub priority: i32,
    pub live: bool,
    pub solo: bool,
    pub auto_play: bool,
    pub capture_gameplay_state: bool,
    pub apply_transform: bool,
    pub apply_projection: bool,
}

impl Default for CinematicVirtualCamera {
    fn default() -> Self {
        Self {
            brain: Entity::PLACEHOLDER,
            priority: 0,
            live: true,
            solo: false,
            auto_play: false,
            capture_gameplay_state: true,
            apply_transform: true,
            apply_projection: true,
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Reflect, Default, PartialEq, Eq)]
#[reflect(Component)]
pub struct CinematicCameraBrain;

#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct CinematicCameraBinding {
    pub camera: Entity,
    pub priority: i32,
    pub capture_gameplay_state: bool,
    pub apply_transform: bool,
    pub apply_projection: bool,
}

impl Default for CinematicCameraBinding {
    fn default() -> Self {
        Self {
            camera: Entity::PLACEHOLDER,
            priority: 0,
            capture_gameplay_state: true,
            apply_transform: true,
            apply_projection: true,
        }
    }
}

#[derive(Component, Clone, Debug, Reflect, Default)]
#[reflect(Component)]
pub struct CinematicSequence {
    pub shots: Vec<CinematicShot>,
    pub loop_mode: PlaybackLoopMode,
    pub restore_camera_on_finish: bool,
    pub entry_blend: CinematicBlend,
    pub exit_blend: CinematicBlend,
}

#[derive(Clone, Debug, Reflect)]
pub struct CinematicShot {
    pub name: String,
    pub duration_secs: f32,
    pub progress_easing: CinematicEasing,
    pub position: PositionTrack,
    pub orientation: OrientationTrack,
    pub lens: LensTrack,
    pub blend_in: CinematicBlend,
    pub markers: Vec<ShotMarker>,
    pub shake: ProceduralShake,
}

impl Default for CinematicShot {
    fn default() -> Self {
        Self {
            name: "Shot".into(),
            duration_secs: 1.0,
            progress_easing: CinematicEasing::Linear,
            position: PositionTrack::Fixed(Vec3::ZERO),
            orientation: OrientationTrack::Fixed(Quat::IDENTITY),
            lens: LensTrack::default(),
            blend_in: CinematicBlend::default(),
            markers: Vec::new(),
            shake: ProceduralShake::default(),
        }
    }
}

impl CinematicShot {
    pub fn fixed(
        name: impl Into<String>,
        duration_secs: f32,
        translation: Vec3,
        rotation: Quat,
    ) -> Self {
        Self {
            name: name.into(),
            duration_secs,
            position: PositionTrack::Fixed(translation),
            orientation: OrientationTrack::Fixed(rotation),
            ..default()
        }
    }

    pub fn rail(name: impl Into<String>, duration_secs: f32, rail: CinematicRail) -> Self {
        Self {
            name: name.into(),
            duration_secs,
            position: PositionTrack::Rail(RailTrack {
                rail,
                traversal: RailTraversal::default(),
            }),
            orientation: OrientationTrack::PathTangent(PathTangentOrientation::default()),
            ..default()
        }
    }
}

#[derive(Clone, Debug, Reflect)]
pub enum PositionTrack {
    Fixed(Vec3),
    Rail(RailTrack),
}

#[derive(Clone, Debug, Reflect)]
pub struct RailTrack {
    pub rail: CinematicRail,
    pub traversal: RailTraversal,
}

#[derive(Clone, Debug, Reflect)]
pub enum OrientationTrack {
    Fixed(Quat),
    PathTangent(PathTangentOrientation),
    LookAt(LookAtTarget),
}

#[derive(Clone, Debug, Reflect)]
pub struct PathTangentOrientation {
    pub up: UpVectorMode,
    pub roll_radians: f32,
}

impl Default for PathTangentOrientation {
    fn default() -> Self {
        Self {
            up: UpVectorMode::WorldY,
            roll_radians: 0.0,
        }
    }
}

#[derive(Clone, Debug, Reflect)]
pub enum LookAtTarget {
    Point {
        point: Vec3,
        up: UpVectorMode,
    },
    Entity {
        entity: Entity,
        offset: Vec3,
        up: UpVectorMode,
        look_ahead_secs: f32,
    },
    GroupEntity(Entity),
}

#[derive(Component, Clone, Debug, Reflect, Default)]
#[reflect(Component)]
pub struct CinematicTargetGroup {
    pub members: Vec<WeightedTarget>,
    pub fallback_point: Vec3,
    pub up: UpVectorMode,
    pub look_ahead_secs: f32,
}

#[derive(Clone, Copy, Debug, Reflect)]
pub struct WeightedTarget {
    pub entity: Entity,
    pub weight: f32,
    pub offset: Vec3,
}

impl WeightedTarget {
    pub fn new(entity: Entity, weight: f32) -> Self {
        Self {
            entity,
            weight,
            offset: Vec3::ZERO,
        }
    }
}

#[derive(Clone, Debug, Reflect)]
pub struct LensTrack {
    pub start_fov_y_radians: f32,
    pub end_fov_y_radians: f32,
    pub easing: CinematicEasing,
}

impl Default for LensTrack {
    fn default() -> Self {
        Self {
            start_fov_y_radians: PerspectiveProjection::default().fov,
            end_fov_y_radians: PerspectiveProjection::default().fov,
            easing: CinematicEasing::Linear,
        }
    }
}

impl LensTrack {
    pub fn fixed(fov_y_radians: f32) -> Self {
        Self {
            start_fov_y_radians: fov_y_radians,
            end_fov_y_radians: fov_y_radians,
            easing: CinematicEasing::Linear,
        }
    }

    pub fn sample(&self, progress: f32) -> f32 {
        let eased = self.easing.sample(progress);
        self.start_fov_y_radians
            .lerp(self.end_fov_y_radians, eased)
            .max(0.001)
    }
}

#[derive(Component, Clone, Copy, Debug, Reflect, PartialEq, Eq, Default)]
#[reflect(Component)]
pub enum CinematicPlaybackStatus {
    #[default]
    Stopped,
    Playing,
    Paused,
    Exiting,
}

#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct CinematicPlayback {
    pub status: CinematicPlaybackStatus,
    pub elapsed_secs: f32,
    pub speed: f32,
}

impl Default for CinematicPlayback {
    fn default() -> Self {
        Self {
            status: CinematicPlaybackStatus::Stopped,
            elapsed_secs: 0.0,
            speed: 1.0,
        }
    }
}

impl CinematicPlayback {
    pub fn play(&mut self) {
        self.status = CinematicPlaybackStatus::Playing;
    }

    pub fn pause(&mut self) {
        self.status = CinematicPlaybackStatus::Paused;
    }

    pub fn resume(&mut self) {
        self.status = CinematicPlaybackStatus::Playing;
    }

    pub fn restart(&mut self) {
        self.elapsed_secs = 0.0;
        self.status = CinematicPlaybackStatus::Playing;
    }

    pub fn stop(&mut self) {
        self.status = CinematicPlaybackStatus::Stopped;
        self.elapsed_secs = 0.0;
    }

    pub fn is_active(self) -> bool {
        self.status != CinematicPlaybackStatus::Stopped
    }
}

/// Configures optional output damping on the solved camera pose.
/// This smooths the final camera position/rotation to eliminate micro-jitter
/// from noisy target tracking, velocity estimation, or blend transitions.
///
/// Inspired by Unity Cinemachine's damping system.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct CinematicOutputDamping {
    /// Positional damping rate. Higher = faster convergence, lower = smoother.
    /// Set to 0.0 to disable position damping.
    pub position_rate: f32,
    /// Rotational damping rate. Higher = faster convergence, lower = smoother.
    /// Set to 0.0 to disable rotation damping.
    pub rotation_rate: f32,
}

impl Default for CinematicOutputDamping {
    fn default() -> Self {
        Self {
            position_rate: 20.0,
            rotation_rate: 15.0,
        }
    }
}

impl CinematicOutputDamping {
    /// Light damping — minimal smoothing for subtle jitter removal.
    pub fn light() -> Self {
        Self {
            position_rate: 30.0,
            rotation_rate: 25.0,
        }
    }

    /// Heavy damping — strong smoothing for very noisy tracking scenarios.
    pub fn heavy() -> Self {
        Self {
            position_rate: 8.0,
            rotation_rate: 6.0,
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct CinematicDrivenCamera {
    pub owner: Entity,
    pub priority: i32,
}

impl Default for CinematicDrivenCamera {
    fn default() -> Self {
        Self {
            owner: Entity::PLACEHOLDER,
            priority: 0,
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct CinematicCameraState {
    pub active: bool,
    pub translation: Vec3,
    pub rotation: Quat,
    pub look_target: Vec3,
    pub fov_y_radians: f32,
    pub sequence_time_secs: f32,
    pub current_shot: Option<usize>,
    pub blend_from_shot: Option<usize>,
    pub blend_to_shot: Option<usize>,
    pub blend_alpha: f32,
}

impl Default for CinematicCameraState {
    fn default() -> Self {
        let default_fov = PerspectiveProjection::default().fov;
        Self {
            active: false,
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            look_target: Vec3::NEG_Z,
            fov_y_radians: default_fov,
            sequence_time_secs: 0.0,
            current_shot: None,
            blend_from_shot: None,
            blend_to_shot: None,
            blend_alpha: 0.0,
        }
    }
}

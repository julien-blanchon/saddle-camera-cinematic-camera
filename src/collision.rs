use bevy::{
    picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings},
    prelude::*,
    reflect::Reflect,
};

use crate::{CinematicCameraState, CinematicPlayback, CinematicPlaybackStatus};

/// Collision avoidance policy — how the camera reacts when an obstacle is detected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum CollisionPolicy {
    /// Push the camera closer to the look target along the ray.
    #[default]
    PushCloser,
    /// Do nothing (disable collision avoidance).
    None,
}

/// Configures camera collision avoidance on a cinematic rig.
///
/// When present, a raycast is performed from the look target toward the camera position
/// each frame. If an obstacle is detected, the camera is pushed closer to the target
/// to prevent clipping through geometry.
///
/// Uses Bevy's built-in `MeshRayCast` for raycasting against renderable meshes. For
/// custom physics engine raycasting, implement your own post-solve system following
/// the same pattern.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct CinematicCollisionAvoidance {
    /// The collision policy to apply when an obstacle is detected.
    pub policy: CollisionPolicy,
    /// Padding distance to keep from the collision surface (world units).
    /// Prevents the near-clip plane from clipping into the surface.
    pub padding: f32,
    /// Smoothing rate when retracting toward the target (higher = faster snap).
    /// Set to a large value (e.g. 50.0) for near-instant retraction.
    pub retract_rate: f32,
    /// Smoothing rate when recovering back to the desired distance (higher = faster recovery).
    /// This should typically be slower than retract_rate for a natural feel.
    pub recover_rate: f32,
    /// Minimum allowed distance between camera and look target (world units).
    pub min_distance: f32,
}

impl Default for CinematicCollisionAvoidance {
    fn default() -> Self {
        Self {
            policy: CollisionPolicy::PushCloser,
            padding: 0.3,
            retract_rate: 40.0,
            recover_rate: 8.0,
            min_distance: 0.5,
        }
    }
}

impl CinematicCollisionAvoidance {
    /// Tight collision avoidance with fast retraction and small padding.
    pub fn tight() -> Self {
        Self {
            padding: 0.15,
            retract_rate: 60.0,
            recover_rate: 12.0,
            min_distance: 0.3,
            ..default()
        }
    }

    /// Loose collision avoidance with more padding and slower recovery.
    pub fn loose() -> Self {
        Self {
            padding: 0.5,
            retract_rate: 30.0,
            recover_rate: 5.0,
            min_distance: 1.0,
            ..default()
        }
    }
}

/// Internal per-rig collision state for smooth retraction/recovery.
#[derive(Component, Default)]
pub(crate) struct CollisionState {
    /// Current effective distance ratio (0..1 where 1 = full desired distance).
    pub distance_ratio: f32,
    pub initialized: bool,
}

pub(crate) fn ensure_collision_state(
    mut commands: Commands,
    query: Query<Entity, (With<CinematicCollisionAvoidance>, Without<CollisionState>)>,
) {
    for entity in &query {
        commands.entity(entity).insert(CollisionState {
            distance_ratio: 1.0,
            initialized: true,
        });
    }
}

pub(crate) fn apply_collision_avoidance(
    time: Res<Time>,
    mut mesh_ray_cast: MeshRayCast,
    mut rigs: Query<(
        &CinematicCollisionAvoidance,
        &CinematicPlayback,
        &mut CinematicCameraState,
        &mut CollisionState,
    )>,
) {
    let delta = time.delta_secs();
    if delta <= f32::EPSILON {
        return;
    }

    for (config, playback, mut state, mut collision) in &mut rigs {
        if playback.status == CinematicPlaybackStatus::Stopped {
            continue;
        }
        if config.policy == CollisionPolicy::None {
            continue;
        }
        if !collision.initialized {
            collision.distance_ratio = 1.0;
            collision.initialized = true;
        }

        let origin = state.look_target;
        let camera_pos = state.translation;
        let to_camera = camera_pos - origin;
        let desired_distance = to_camera.length();

        if desired_distance < config.min_distance {
            continue;
        }

        let direction = to_camera / desired_distance;
        let dir = match Dir3::new(direction) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let ray = Ray3d::new(origin, dir);

        let settings = MeshRayCastSettings::default();
        let hits = mesh_ray_cast.cast_ray(ray, &settings);

        let mut target_ratio = 1.0_f32;

        for &(_entity, ref hit) in hits {
            let hit_distance = hit.distance;
            if hit_distance < f32::EPSILON {
                continue;
            }
            if hit_distance < desired_distance + config.padding {
                let safe_distance = (hit_distance - config.padding).max(config.min_distance);
                let ratio = safe_distance / desired_distance;
                target_ratio = target_ratio.min(ratio);
            }
        }

        // Smooth the distance ratio: fast retraction, slow recovery.
        let rate = if target_ratio < collision.distance_ratio {
            config.retract_rate
        } else {
            config.recover_rate
        };
        let factor: f32 = (1.0_f32 - (-rate * delta).exp()).clamp(0.0, 1.0);
        collision.distance_ratio = collision
            .distance_ratio
            .lerp(target_ratio, factor)
            .clamp(0.0, 1.0);

        // Apply the adjusted position.
        state.translation = origin + direction * (desired_distance * collision.distance_ratio);
    }
}

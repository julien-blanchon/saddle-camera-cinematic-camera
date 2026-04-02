use bevy::{math::StableInterpolate, prelude::*};

use crate::{CinematicCameraState, ProceduralShake, UpVectorMode};

#[derive(Clone, Debug)]
pub(crate) struct SolvedCameraPose {
    pub translation: Vec3,
    pub rotation: Quat,
    pub look_target: Vec3,
    pub fov_y_radians: f32,
}

impl Default for SolvedCameraPose {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            look_target: Vec3::NEG_Z,
            fov_y_radians: std::f32::consts::FRAC_PI_4,
        }
    }
}

impl SolvedCameraPose {
    pub fn from_state(state: &CinematicCameraState) -> Self {
        Self {
            translation: state.translation,
            rotation: state.rotation,
            look_target: state.look_target,
            fov_y_radians: state.fov_y_radians,
        }
    }
}

pub(crate) fn blend_pose(
    from: &SolvedCameraPose,
    to: &SolvedCameraPose,
    alpha: f32,
) -> SolvedCameraPose {
    let alpha = alpha.clamp(0.0, 1.0);
    SolvedCameraPose {
        translation: from.translation.interpolate_stable(&to.translation, alpha),
        rotation: from.rotation.interpolate_stable(&to.rotation, alpha),
        look_target: from.look_target.interpolate_stable(&to.look_target, alpha),
        fov_y_radians: from
            .fov_y_radians
            .interpolate_stable(&to.fov_y_radians, alpha),
    }
}

pub(crate) fn resolve_up_vector(forward: Vec3, up_mode: UpVectorMode) -> Vec3 {
    let desired = up_mode.vector().normalize_or_zero();
    if desired.length_squared() <= f32::EPSILON {
        return fallback_up(forward);
    }

    if forward.length_squared() <= f32::EPSILON {
        return desired;
    }

    if forward.normalize().dot(desired).abs() > 0.995 {
        fallback_up(forward)
    } else {
        desired
    }
}

pub(crate) fn solve_look_rotation(
    fallback_rotation: Quat,
    origin: Vec3,
    target: Vec3,
    up_mode: UpVectorMode,
) -> Quat {
    let forward = target - origin;
    if forward.length_squared() <= f32::EPSILON {
        return fallback_rotation;
    }

    let up = resolve_up_vector(forward, up_mode);
    Transform::IDENTITY
        .looking_to(forward.normalize(), up)
        .rotation
}

pub(crate) fn solve_path_tangent_rotation(
    fallback_rotation: Quat,
    tangent: Vec3,
    up_mode: UpVectorMode,
    roll_radians: f32,
) -> Quat {
    if tangent.length_squared() <= f32::EPSILON {
        return fallback_rotation;
    }

    let up = resolve_up_vector(tangent, up_mode);
    let base = Transform::IDENTITY
        .looking_to(tangent.normalize(), up)
        .rotation;
    base * Quat::from_axis_angle(Vec3::NEG_Z, roll_radians)
}

pub(crate) fn weighted_target_center<I>(weighted_points: I, fallback: Vec3) -> Vec3
where
    I: IntoIterator<Item = (Vec3, f32)>,
{
    let mut accum = Vec3::ZERO;
    let mut weight_sum = 0.0;

    for (point, weight) in weighted_points {
        if weight <= 0.0 {
            continue;
        }
        accum += point * weight;
        weight_sum += weight;
    }

    if weight_sum <= f32::EPSILON {
        fallback
    } else {
        accum / weight_sum
    }
}

pub(crate) fn apply_procedural_shake(
    pose: &SolvedCameraPose,
    shake: ProceduralShake,
    time_secs: f32,
) -> SolvedCameraPose {
    if !shake.is_enabled() {
        return pose.clone();
    }

    let translation = Vec3::new(
        wave(time_secs, shake.frequency_hz.x, shake.seed + 1.73),
        wave(time_secs, shake.frequency_hz.y, shake.seed + 6.11),
        wave(time_secs, shake.frequency_hz.z, shake.seed + 9.42),
    ) * shake.translation_amplitude;

    let rotation = Vec3::new(
        wave(time_secs, shake.frequency_hz.x * 0.73, shake.seed + 3.21),
        wave(time_secs, shake.frequency_hz.y * 0.91, shake.seed + 5.67),
        wave(time_secs, shake.frequency_hz.z * 1.07, shake.seed + 8.95),
    ) * shake.rotation_amplitude_radians;

    let local_rotation = Quat::from_euler(EulerRot::XYZ, rotation.x, rotation.y, rotation.z);

    SolvedCameraPose {
        translation: pose.translation + translation,
        rotation: pose.rotation * local_rotation,
        look_target: pose.look_target,
        fov_y_radians: pose.fov_y_radians,
    }
}

fn fallback_up(forward: Vec3) -> Vec3 {
    let forward = forward.normalize_or_zero();
    if forward.length_squared() <= f32::EPSILON {
        return Vec3::Y;
    }

    if forward.y.abs() < 0.95 {
        Vec3::Y
    } else if forward.x.abs() < 0.95 {
        Vec3::X
    } else {
        Vec3::Z
    }
}

fn wave(time_secs: f32, frequency_hz: f32, phase: f32) -> f32 {
    let angular = std::f32::consts::TAU * frequency_hz.max(0.0) * time_secs;
    (angular + phase).sin() * 0.6 + (angular * 0.37 + phase * 0.19).sin() * 0.4
}

#[cfg(test)]
#[path = "solver_tests.rs"]
mod tests;

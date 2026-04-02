use bevy::{
    color::palettes::css::{AQUA, CRIMSON, GOLD, HOT_PINK, ORANGE, TURQUOISE, WHITE},
    gizmos::gizmos::Gizmos,
    prelude::*,
};

use crate::{
    CinematicCameraDebugSettings, CinematicCameraState, CinematicSequence, PlaybackLoopMode,
    PositionTrack, systems::CinematicSequenceCache,
};

pub(crate) fn draw_debug_gizmos(
    settings: Res<CinematicCameraDebugSettings>,
    mut gizmos: Gizmos,
    rigs: Query<(
        &CinematicSequence,
        &CinematicSequenceCache,
        &CinematicCameraState,
    )>,
) {
    if !settings.enabled {
        return;
    }

    for (sequence, cache, state) in &rigs {
        if settings.draw_paths {
            for (index, shot) in sequence.shots.iter().enumerate() {
                if let (PositionTrack::Rail(track), Some(rail_cache)) =
                    (&shot.position, cache.rail_caches[index].as_ref())
                {
                    let line_color = if state.current_shot == Some(index) {
                        AQUA
                    } else {
                        TURQUOISE
                    };
                    let total = rail_cache.total_length();
                    let steps = (track.rail.segment_count() * 24).max(16);
                    let mut previous = rail_cache
                        .sample_distance(0.0, PlaybackLoopMode::Once)
                        .position;
                    for step in 1..=steps {
                        let distance = total * (step as f32 / steps as f32);
                        let current = rail_cache
                            .sample_distance(distance, PlaybackLoopMode::Once)
                            .position;
                        gizmos.line(previous, current, line_color);
                        previous = current;
                    }

                    if settings.draw_control_points {
                        for point in &track.rail.points {
                            gizmos.cross(*point, 0.2, GOLD);
                        }
                    }
                }
            }
        }

        if settings.draw_active_samples && state.active {
            gizmos.cross(state.translation, 0.35, HOT_PINK);
        }

        if settings.draw_targets {
            gizmos.line(state.translation, state.look_target, ORANGE);
            gizmos.cross(state.look_target, 0.25, CRIMSON);
        }

        if settings.draw_camera_forward {
            let forward = state.rotation * Vec3::NEG_Z;
            gizmos.arrow(state.translation, state.translation + forward * 1.5, WHITE);
        }
    }
}

use bevy::{
    camera::{PerspectiveProjection, Projection},
    prelude::*,
};

use crate::{
    CinematicBlendCompleted, CinematicBlendKind, CinematicCameraBinding,
    CinematicCameraDiagnostics, CinematicCameraRig, CinematicCameraState, CinematicDrivenCamera,
    CinematicPlayback, CinematicPlaybackCommand, CinematicPlaybackStatus, CinematicSequence,
    CinematicShot, CinematicTargetGroup, CinematicVirtualCamera, LookAtTarget,
    OrientationTrack, PathTangentOrientation, PlaybackLoopMode, PositionTrack, SequenceFinished,
    ShotFinished, ShotMarkerReached, ShotStarted, UpVectorMode,
    curve::CinematicRailCache,
    resources::{CinematicCameraRuntimeState, TargetHistory, TargetMotionState},
    solver::{self, SolvedCameraPose},
};

#[derive(Component, Default)]
pub(crate) struct CinematicSequenceCache {
    shot_starts: Vec<f32>,
    total_duration: f32,
    pub(crate) rail_caches: Vec<Option<CinematicRailCache>>,
    timeline_events: Vec<CachedTimelineEvent>,
}

#[derive(Clone, Debug)]
struct CachedTimelineEvent {
    time_secs: f32,
    shot_index: usize,
    kind: CachedTimelineEventKind,
}

#[derive(Clone, Debug)]
enum CachedTimelineEventKind {
    ShotStarted,
    Marker(String),
    ShotFinished,
}

impl CachedTimelineEventKind {
    const fn sort_order(&self) -> u8 {
        match self {
            Self::ShotStarted => 0,
            Self::Marker(_) => 1,
            Self::ShotFinished => 2,
        }
    }
}

#[derive(Component, Default)]
pub(crate) struct CinematicRuntime {
    entry_elapsed_secs: f32,
    exit_elapsed_secs: Option<f32>,
    exit_pending_stop: bool,
    direction: f32,
    autoplay_consumed: bool,
    entry_blend_reported: bool,
    suppress_events_once: bool,
    last_blend: Option<(usize, usize)>,
    captured_snapshot: Option<CameraSnapshot>,
    exit_pose: Option<SolvedCameraPose>,
}

#[derive(Clone, Debug)]
struct CameraSnapshot {
    pose: SolvedCameraPose,
}

type CameraBindingWinner = (Entity, i32, CinematicCameraState, bool, bool, bool);

impl CameraSnapshot {
    fn from_camera(transform: &Transform, projection: Option<&Projection>) -> Self {
        let fov = projection
            .and_then(|projection| match projection {
                Projection::Perspective(perspective) => Some(perspective.fov),
                _ => None,
            })
            .unwrap_or(PerspectiveProjection::default().fov);

        Self {
            pose: SolvedCameraPose {
                translation: transform.translation,
                rotation: transform.rotation,
                look_target: transform.translation + transform.forward().as_vec3() * 10.0,
                fov_y_radians: fov,
            },
        }
    }
}

pub(crate) fn runtime_is_active(runtime: Res<CinematicCameraRuntimeState>) -> bool {
    runtime.active
}

pub(crate) fn activate_runtime(
    mut commands: Commands,
    mut runtime_state: ResMut<CinematicCameraRuntimeState>,
) {
    runtime_state.active = true;

    if runtime_state.debug_root.is_none() {
        let entity = commands.spawn((Name::new("Cinematic Camera Debug"),)).id();
        runtime_state.debug_root = Some(entity);
    }
}

pub(crate) fn deactivate_runtime(
    mut commands: Commands,
    mut runtime_state: ResMut<CinematicCameraRuntimeState>,
    driven_cameras: Query<Entity, With<CinematicDrivenCamera>>,
) {
    runtime_state.active = false;
    for entity in &driven_cameras {
        commands.entity(entity).remove::<CinematicDrivenCamera>();
    }
}

pub(crate) fn sync_virtual_camera_authoring(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &CinematicVirtualCamera,
            Option<&mut CinematicCameraRig>,
            Option<&mut CinematicCameraBinding>,
        ),
        With<CinematicSequence>,
    >,
) {
    for (entity, virtual_camera, rig, binding) in &mut query {
        let resolved_priority = if virtual_camera.solo {
            i32::MAX.saturating_sub(virtual_camera.priority.abs())
        } else {
            virtual_camera.priority
        };

        if let Some(mut rig) = rig {
            rig.auto_play = virtual_camera.auto_play;
            rig.enabled = virtual_camera.live;
        } else {
            commands.entity(entity).insert(CinematicCameraRig {
                auto_play: virtual_camera.auto_play,
                enabled: virtual_camera.live,
            });
        }

        if let Some(mut binding) = binding {
            binding.camera = virtual_camera.brain;
            binding.priority = resolved_priority;
            binding.capture_gameplay_state = virtual_camera.capture_gameplay_state;
            binding.apply_transform = virtual_camera.apply_transform;
            binding.apply_projection = virtual_camera.apply_projection;
        } else {
            commands.entity(entity).insert(CinematicCameraBinding {
                camera: virtual_camera.brain,
                priority: resolved_priority,
                capture_gameplay_state: virtual_camera.capture_gameplay_state,
                apply_transform: virtual_camera.apply_transform,
                apply_projection: virtual_camera.apply_projection,
            });
        }
    }
}

pub(crate) fn ensure_runtime_components(
    mut commands: Commands,
    query: Query<
        (
            Entity,
            Option<&CinematicRuntime>,
            Option<&CinematicSequenceCache>,
            Option<&CinematicCameraState>,
        ),
        With<CinematicCameraRig>,
    >,
) {
    for (entity, runtime, cache, state) in &query {
        if runtime.is_none() {
            commands.entity(entity).insert(CinematicRuntime {
                direction: 1.0,
                ..default()
            });
        }
        if cache.is_none() {
            commands
                .entity(entity)
                .insert(CinematicSequenceCache::default());
        }
        if state.is_none() {
            commands
                .entity(entity)
                .insert(CinematicCameraState::default());
        }
    }
}

pub(crate) fn rebuild_sequence_caches(
    mut query: Query<(&CinematicSequence, &mut CinematicSequenceCache), Changed<CinematicSequence>>,
) {
    for (sequence, mut cache) in &mut query {
        let mut shot_starts = Vec::with_capacity(sequence.shots.len());
        let mut rail_caches = Vec::with_capacity(sequence.shots.len());
        let mut timeline_events = Vec::with_capacity(sequence.shots.len() * 3);
        let mut cursor = 0.0;

        for (index, shot) in sequence.shots.iter().enumerate() {
            if index == 0 {
                shot_starts.push(0.0);
                cursor = shot.duration_secs.max(0.0001);
            } else {
                let previous = &sequence.shots[index - 1];
                let overlap = shot
                    .blend_in
                    .duration_secs
                    .min(previous.duration_secs.max(0.0))
                    .min(shot.duration_secs.max(0.0));
                cursor -= overlap;
                shot_starts.push(cursor);
                cursor += shot.duration_secs.max(0.0001);
            }

            let shot_start = shot_starts[index];
            let shot_duration = shot.duration_secs.max(0.0);
            rail_caches.push(match &shot.position {
                PositionTrack::Fixed(_) => None,
                PositionTrack::Rail(track) => Some(CinematicRailCache::rebuild(&track.rail)),
            });

            let mut shot_markers = shot
                .markers
                .iter()
                .map(|marker| {
                    (
                        shot.duration_secs
                            .max(0.0)
                            .min(marker.at.seconds(shot.duration_secs)),
                        marker.name.clone(),
                    )
                })
                .collect::<Vec<_>>();
            shot_markers.sort_by(|left, right| left.0.total_cmp(&right.0));
            timeline_events.push(CachedTimelineEvent {
                time_secs: shot_start,
                shot_index: index,
                kind: CachedTimelineEventKind::ShotStarted,
            });
            for (time_secs, marker_name) in shot_markers {
                timeline_events.push(CachedTimelineEvent {
                    time_secs: shot_start + time_secs,
                    shot_index: index,
                    kind: CachedTimelineEventKind::Marker(marker_name),
                });
            }
            timeline_events.push(CachedTimelineEvent {
                time_secs: shot_start + shot_duration,
                shot_index: index,
                kind: CachedTimelineEventKind::ShotFinished,
            });
        }

        timeline_events.sort_by(|left, right| {
            left.time_secs
                .total_cmp(&right.time_secs)
                .then(left.kind.sort_order().cmp(&right.kind.sort_order()))
                .then(left.shot_index.cmp(&right.shot_index))
        });

        cache.shot_starts = shot_starts;
        cache.total_duration = sequence
            .shots
            .last()
            .and_then(|last| {
                cache
                    .shot_starts
                    .last()
                    .copied()
                    .map(|start| start + last.duration_secs)
            })
            .unwrap_or(0.0);
        cache.rail_caches = rail_caches;
        cache.timeline_events = timeline_events;
    }
}

pub(crate) fn refresh_target_history(
    time: Res<Time>,
    mut history: ResMut<TargetHistory>,
    query: Query<(Entity, &GlobalTransform), Changed<GlobalTransform>>,
) {
    let delta = time.delta_secs();
    if delta <= f32::EPSILON {
        return;
    }

    for (entity, transform) in &query {
        let position = transform.translation();
        let entry = history.entries.entry(entity).or_default();
        let velocity = if entry.position == Vec3::ZERO && entry.velocity == Vec3::ZERO {
            Vec3::ZERO
        } else {
            (position - entry.position) / delta
        };
        *entry = TargetMotionState { position, velocity };
    }
}

pub(crate) fn apply_playback_commands(
    mut commands_reader: MessageReader<CinematicPlaybackCommand>,
    mut rigs: Query<(
        Entity,
        &CinematicSequenceCache,
        &CinematicCameraBinding,
        &mut CinematicPlayback,
        &mut CinematicRuntime,
        &CinematicCameraState,
    )>,
    cameras: Query<(&Transform, Option<&Projection>)>,
) {
    for command in commands_reader.read() {
        match command {
            CinematicPlaybackCommand::Play(rig) => {
                if let Ok((_entity, _cache, binding, mut playback, mut runtime, _state)) =
                    rigs.get_mut(*rig)
                {
                    let should_capture_snapshot =
                        playback.status == CinematicPlaybackStatus::Stopped;
                    begin_playback(
                        binding,
                        &cameras,
                        &mut playback,
                        &mut runtime,
                        false,
                        should_capture_snapshot,
                    );
                }
            }
            CinematicPlaybackCommand::Pause(rig) => {
                if let Ok((_entity, _cache, _binding, mut playback, _runtime, _state)) =
                    rigs.get_mut(*rig)
                {
                    playback.pause();
                }
            }
            CinematicPlaybackCommand::Resume(rig) => {
                if let Ok((_entity, _cache, binding, mut playback, mut runtime, _state)) =
                    rigs.get_mut(*rig)
                {
                    begin_playback(binding, &cameras, &mut playback, &mut runtime, true, false);
                }
            }
            CinematicPlaybackCommand::Restart(rig) => {
                if let Ok((_entity, _cache, binding, mut playback, mut runtime, _state)) =
                    rigs.get_mut(*rig)
                {
                    let should_capture_snapshot =
                        playback.status == CinematicPlaybackStatus::Stopped;
                    playback.elapsed_secs = 0.0;
                    runtime.direction = 1.0;
                    runtime.last_blend = None;
                    runtime.suppress_events_once = false;
                    begin_playback(
                        binding,
                        &cameras,
                        &mut playback,
                        &mut runtime,
                        false,
                        should_capture_snapshot,
                    );
                }
            }
            CinematicPlaybackCommand::Stop {
                rig,
                restore_camera,
            } => {
                if let Ok((_entity, _cache, _binding, mut playback, mut runtime, state)) =
                    rigs.get_mut(*rig)
                {
                    if *restore_camera && runtime.captured_snapshot.is_some() {
                        playback.status = CinematicPlaybackStatus::Exiting;
                        runtime.exit_elapsed_secs = Some(0.0);
                        runtime.exit_pending_stop = false;
                        runtime.exit_pose = Some(SolvedCameraPose::from_state(state));
                    } else {
                        playback.stop();
                    }
                }
            }
            CinematicPlaybackCommand::SeekSeconds { rig, seconds } => {
                if let Ok((_entity, cache, _binding, mut playback, mut runtime, _state)) =
                    rigs.get_mut(*rig)
                {
                    playback.elapsed_secs = seconds.clamp(0.0, cache.total_duration);
                    runtime.suppress_events_once = true;
                }
            }
            CinematicPlaybackCommand::SeekNormalized { rig, normalized } => {
                if let Ok((_entity, cache, _binding, mut playback, mut runtime, _state)) =
                    rigs.get_mut(*rig)
                {
                    playback.elapsed_secs = normalized.clamp(0.0, 1.0) * cache.total_duration;
                    runtime.suppress_events_once = true;
                }
            }
        }
    }
}

pub(crate) fn autoplay_rigs(
    cameras: Query<(&Transform, Option<&Projection>)>,
    mut rigs: Query<
        (
            &CinematicCameraRig,
            &CinematicCameraBinding,
            &mut CinematicPlayback,
            &mut CinematicRuntime,
        ),
        With<CinematicSequence>,
    >,
) {
    for (rig, binding, mut playback, mut runtime) in &mut rigs {
        if !rig.auto_play || runtime.autoplay_consumed {
            continue;
        }

        runtime.autoplay_consumed = true;
        begin_playback(binding, &cameras, &mut playback, &mut runtime, false, true);
    }
}

pub(crate) fn advance_timeline(
    time: Res<Time>,
    mut rigs: Query<(
        Entity,
        &CinematicSequence,
        &CinematicSequenceCache,
        &mut CinematicPlayback,
        &mut CinematicRuntime,
        &CinematicCameraState,
    )>,
    mut shot_started: MessageWriter<ShotStarted>,
    mut shot_finished: MessageWriter<ShotFinished>,
    mut markers: MessageWriter<ShotMarkerReached>,
    mut sequence_finished: MessageWriter<SequenceFinished>,
    mut blend_completed: MessageWriter<CinematicBlendCompleted>,
) {
    let delta = time.delta_secs();
    for (entity, sequence, cache, mut playback, mut runtime, state) in &mut rigs {
        if sequence.shots.is_empty() {
            playback.status = CinematicPlaybackStatus::Stopped;
            continue;
        }

        if runtime.exit_pending_stop {
            runtime.exit_pending_stop = false;
            runtime.exit_elapsed_secs = None;
            runtime.exit_pose = None;
            playback.stop();
            blend_completed.write(CinematicBlendCompleted {
                rig: entity,
                kind: CinematicBlendKind::CinematicToGameplay {
                    from_shot: state.current_shot,
                },
            });
            continue;
        }

        if let Some(exit_elapsed) = runtime.exit_elapsed_secs.as_mut() {
            *exit_elapsed += delta.max(0.0);
            let duration = sequence.exit_blend.duration_secs.max(f32::EPSILON);
            if *exit_elapsed >= duration {
                *exit_elapsed = duration;
                runtime.exit_pending_stop = true;
            }
            continue;
        }

        if playback.status == CinematicPlaybackStatus::Paused
            || playback.status == CinematicPlaybackStatus::Stopped
        {
            continue;
        }

        let previous = playback.elapsed_secs;
        let (next_time, next_direction, wrapped, finished_once) = advance_visible_time(
            previous,
            runtime.direction,
            playback.speed.max(0.0) * delta,
            cache.total_duration,
            sequence.loop_mode,
        );

        runtime.direction = next_direction;
        playback.elapsed_secs = next_time;
        runtime.entry_elapsed_secs =
            (runtime.entry_elapsed_secs + delta).min(sequence.entry_blend.duration_secs.max(0.0));

        if !runtime.suppress_events_once {
            if runtime.direction >= 0.0 {
                if wrapped {
                    emit_events_between(
                        entity,
                        previous,
                        cache.total_duration,
                        sequence,
                        cache,
                        false,
                        &mut shot_started,
                        &mut shot_finished,
                        &mut markers,
                    );
                    emit_events_between(
                        entity,
                        0.0,
                        next_time,
                        sequence,
                        cache,
                        true,
                        &mut shot_started,
                        &mut shot_finished,
                        &mut markers,
                    );
                } else {
                    emit_events_between(
                        entity,
                        previous,
                        next_time,
                        sequence,
                        cache,
                        previous <= f32::EPSILON,
                        &mut shot_started,
                        &mut shot_finished,
                        &mut markers,
                    );
                }
            }
        } else {
            runtime.suppress_events_once = false;
        }

        if finished_once {
            sequence_finished.write(SequenceFinished { rig: entity });
            if sequence.restore_camera_on_finish && runtime.captured_snapshot.is_some() {
                playback.status = CinematicPlaybackStatus::Exiting;
                runtime.exit_elapsed_secs = Some(0.0);
                runtime.exit_pending_stop = false;
                runtime.exit_pose = Some(SolvedCameraPose::from_state(state));
            } else {
                playback.stop();
            }
        }
    }
}

pub(crate) fn solve_rigs(
    transforms: Query<&GlobalTransform>,
    target_groups: Query<&CinematicTargetGroup>,
    history: Res<TargetHistory>,
    mut rigs: Query<(
        Entity,
        &CinematicCameraRig,
        &CinematicSequence,
        &CinematicSequenceCache,
        &CinematicPlayback,
        &mut CinematicRuntime,
        &mut CinematicCameraState,
    )>,
    mut blend_completed: MessageWriter<CinematicBlendCompleted>,
) {
    for (entity, rig, sequence, cache, playback, mut runtime, mut state) in &mut rigs {
        if sequence.shots.is_empty() {
            *state = CinematicCameraState::default();
            continue;
        }

        let (mut pose, blend_from, blend_to, blend_alpha) = solve_sequence_pose(
            &transforms,
            &target_groups,
            &history,
            sequence,
            cache,
            playback.elapsed_secs,
            *state,
        );

        if playback.status == CinematicPlaybackStatus::Exiting {
            if let (Some(snapshot), Some(exit_pose), Some(exit_elapsed)) = (
                runtime.captured_snapshot.as_ref(),
                runtime.exit_pose.as_ref(),
                runtime.exit_elapsed_secs,
            ) {
                pose = solver::blend_pose(
                    exit_pose,
                    &snapshot.pose,
                    sequence.exit_blend.alpha(exit_elapsed),
                );
            }
        } else if playback.status != CinematicPlaybackStatus::Stopped
            && sequence.entry_blend.is_active()
            && runtime.entry_elapsed_secs < sequence.entry_blend.duration_secs
        {
            if let Some(snapshot) = runtime.captured_snapshot.as_ref() {
                pose = solver::blend_pose(
                    &snapshot.pose,
                    &pose,
                    sequence.entry_blend.alpha(runtime.entry_elapsed_secs),
                );
            }
        }

        let current_shot = blend_to
            .or(blend_from)
            .or(dominant_shot(cache, playback.elapsed_secs));

        if playback.status != CinematicPlaybackStatus::Stopped
            && sequence.entry_blend.is_active()
            && !runtime.entry_blend_reported
            && runtime.entry_elapsed_secs >= sequence.entry_blend.duration_secs
        {
            runtime.entry_blend_reported = true;
            blend_completed.write(CinematicBlendCompleted {
                rig: entity,
                kind: CinematicBlendKind::GameplayToCinematic {
                    to_shot: current_shot,
                },
            });
        }

        if let (Some(from), Some(to)) = (blend_from, blend_to) {
            if let Some((last_from, last_to)) = runtime.last_blend {
                if (last_from, last_to) != (from, to) {
                    blend_completed.write(CinematicBlendCompleted {
                        rig: entity,
                        kind: CinematicBlendKind::ShotToShot {
                            from_shot: last_from,
                            to_shot: last_to,
                        },
                    });
                }
            }
            runtime.last_blend = Some((from, to));
        } else if let Some((last_from, last_to)) = runtime.last_blend.take() {
            blend_completed.write(CinematicBlendCompleted {
                rig: entity,
                kind: CinematicBlendKind::ShotToShot {
                    from_shot: last_from,
                    to_shot: last_to,
                },
            });
        }

        state.active = rig.enabled && (playback.is_active() || playback.elapsed_secs > 0.0);
        state.translation = pose.translation;
        state.rotation = pose.rotation;
        state.look_target = pose.look_target;
        state.fov_y_radians = pose.fov_y_radians;
        state.sequence_time_secs = playback.elapsed_secs;
        state.current_shot = current_shot;
        state.blend_from_shot = blend_from;
        state.blend_to_shot = blend_to;
        state.blend_alpha = blend_alpha;
    }
}

pub(crate) fn apply_camera_bindings(
    mut commands: Commands,
    rigs: Query<(
        Entity,
        &CinematicCameraBinding,
        &CinematicPlayback,
        &CinematicCameraState,
    )>,
    mut cameras: Query<(Entity, &mut Transform, Option<&mut Projection>)>,
    driven: Query<(Entity, &CinematicDrivenCamera)>,
    mut winners: Local<std::collections::HashMap<Entity, CameraBindingWinner>>,
) {
    winners.clear();

    for (rig_entity, binding, playback, state) in &rigs {
        if !state.active && playback.status != CinematicPlaybackStatus::Exiting {
            continue;
        }

        let should_apply = playback.status != CinematicPlaybackStatus::Stopped;
        let entry = winners.entry(binding.camera).or_insert((
            rig_entity,
            binding.priority,
            *state,
            should_apply,
            binding.apply_transform,
            binding.apply_projection,
        ));
        if binding.priority > entry.1 || (binding.priority == entry.1 && rig_entity > entry.0) {
            *entry = (
                rig_entity,
                binding.priority,
                *state,
                should_apply,
                binding.apply_transform,
                binding.apply_projection,
            );
        }
    }

    for (camera_entity, owner) in &driven {
        let should_remove = winners
            .get(&camera_entity)
            .is_none_or(|(winner, ..)| *winner != owner.owner);
        if should_remove {
            commands
                .entity(camera_entity)
                .remove::<CinematicDrivenCamera>();
        }
    }

    for (
        camera_entity,
        (owner, priority, state, should_apply, apply_transform, apply_projection),
    ) in winners.drain()
    {
        if let Ok((_entity, mut transform, projection)) = cameras.get_mut(camera_entity) {
            if should_apply {
                if apply_transform {
                    *transform = Transform::from_translation(state.translation)
                        .with_rotation(state.rotation);
                }
                if apply_projection {
                    if let Some(mut projection) = projection {
                        if let Projection::Perspective(perspective) = projection.as_mut() {
                            perspective.fov = state.fov_y_radians.max(0.001);
                        }
                    }
                }
            }

            commands
                .entity(camera_entity)
                .insert(CinematicDrivenCamera { owner, priority });
        }
    }
}

pub(crate) fn publish_diagnostics(
    history: Res<TargetHistory>,
    rigs: Query<&CinematicPlayback, With<CinematicSequence>>,
    driven: Query<&CinematicDrivenCamera>,
    mut diagnostics: ResMut<CinematicCameraDiagnostics>,
) {
    diagnostics.previewable_rigs = rigs.iter().count();
    diagnostics.active_rigs = rigs.iter().filter(|playback| playback.is_active()).count();
    diagnostics.applied_cameras = driven.iter().count();
    diagnostics.target_history_entries = history.entries.len();
}

fn begin_playback(
    binding: &CinematicCameraBinding,
    cameras: &Query<(&Transform, Option<&Projection>)>,
    playback: &mut CinematicPlayback,
    runtime: &mut CinematicRuntime,
    preserve_elapsed: bool,
    capture_snapshot: bool,
) {
    if capture_snapshot {
        runtime.captured_snapshot = if binding.capture_gameplay_state {
            cameras
                .get(binding.camera)
                .ok()
                .map(|(transform, projection)| CameraSnapshot::from_camera(transform, projection))
        } else {
            None
        };
    }

    if !preserve_elapsed {
        runtime.entry_elapsed_secs = 0.0;
        runtime.entry_blend_reported = false;
    }
    runtime.exit_elapsed_secs = None;
    runtime.exit_pending_stop = false;
    runtime.exit_pose = None;
    runtime.direction = 1.0;
    playback.play();
}

fn advance_visible_time(
    current: f32,
    direction: f32,
    delta: f32,
    total: f32,
    loop_mode: PlaybackLoopMode,
) -> (f32, f32, bool, bool) {
    if total <= f32::EPSILON {
        return (0.0, direction, false, false);
    }

    match loop_mode {
        PlaybackLoopMode::Once => {
            let next = (current + delta).clamp(0.0, total);
            (next, 1.0, false, current < total && next >= total)
        }
        PlaybackLoopMode::Loop => {
            let mut next = current + delta;
            let wrapped = next > total;
            while next > total {
                next -= total;
            }
            (next, 1.0, wrapped, false)
        }
        PlaybackLoopMode::PingPong => {
            let mut next = current + delta * direction;
            let mut direction = direction;
            let mut wrapped = false;
            while next > total || next < 0.0 {
                wrapped = true;
                if next > total {
                    next = total - (next - total);
                    direction = -1.0;
                } else if next < 0.0 {
                    next = -next;
                    direction = 1.0;
                }
            }
            (next, direction, wrapped, false)
        }
    }
}

fn dominant_shot(cache: &CinematicSequenceCache, time_secs: f32) -> Option<usize> {
    let mut result = None;
    for (index, start) in cache.shot_starts.iter().enumerate() {
        if time_secs >= *start {
            result = Some(index);
        }
    }
    result
}

fn solve_sequence_pose(
    transforms: &Query<&GlobalTransform>,
    target_groups: &Query<&CinematicTargetGroup>,
    history: &TargetHistory,
    sequence: &CinematicSequence,
    cache: &CinematicSequenceCache,
    time_secs: f32,
    fallback_state: CinematicCameraState,
) -> (SolvedCameraPose, Option<usize>, Option<usize>, f32) {
    if let Some((from, to, alpha)) = active_blend(sequence, cache, time_secs) {
        let from_local = time_secs - cache.shot_starts[from];
        let to_local = time_secs - cache.shot_starts[to];
        let from_pose = solve_shot_pose(
            &sequence.shots[from],
            cache.rail_caches[from].as_ref(),
            from_local,
            transforms,
            target_groups,
            history,
            fallback_state.rotation,
            fallback_state.look_target,
        );
        let to_pose = solve_shot_pose(
            &sequence.shots[to],
            cache.rail_caches[to].as_ref(),
            to_local,
            transforms,
            target_groups,
            history,
            from_pose.rotation,
            from_pose.look_target,
        );
        return (
            solver::blend_pose(&from_pose, &to_pose, alpha),
            Some(from),
            Some(to),
            alpha,
        );
    }

    let shot_index = dominant_shot(cache, time_secs).unwrap_or(0);
    let local_time = time_secs - cache.shot_starts[shot_index];
    (
        solve_shot_pose(
            &sequence.shots[shot_index],
            cache.rail_caches[shot_index].as_ref(),
            local_time,
            transforms,
            target_groups,
            history,
            fallback_state.rotation,
            fallback_state.look_target,
        ),
        Some(shot_index),
        None,
        1.0,
    )
}

fn active_blend(
    sequence: &CinematicSequence,
    cache: &CinematicSequenceCache,
    time_secs: f32,
) -> Option<(usize, usize, f32)> {
    for index in 1..sequence.shots.len() {
        let blend = sequence.shots[index]
            .blend_in
            .duration_secs
            .min(sequence.shots[index - 1].duration_secs.max(0.0))
            .min(sequence.shots[index].duration_secs.max(0.0));
        if blend <= f32::EPSILON {
            continue;
        }

        let start = cache.shot_starts[index];
        let end = start + blend;
        if (start..end).contains(&time_secs) {
            let alpha = sequence.shots[index].blend_in.alpha(time_secs - start);
            return Some((index - 1, index, alpha));
        }
    }

    None
}

fn solve_shot_pose(
    shot: &CinematicShot,
    rail_cache: Option<&CinematicRailCache>,
    local_time_secs: f32,
    transforms: &Query<&GlobalTransform>,
    target_groups: &Query<&CinematicTargetGroup>,
    history: &TargetHistory,
    fallback_rotation: Quat,
    fallback_look_target: Vec3,
) -> SolvedCameraPose {
    let duration = shot.duration_secs.max(0.0001);
    let progress = shot.progress_easing.sample(local_time_secs / duration);
    let (position, tangent) = match (&shot.position, rail_cache) {
        (PositionTrack::Fixed(point), _) => (*point, fallback_rotation * Vec3::NEG_Z),
        (PositionTrack::Rail(track), Some(cache)) => {
            let sample = track.traversal.sample(cache, progress);
            (sample.position, sample.tangent)
        }
        (PositionTrack::Rail(_), None) => (Vec3::ZERO, Vec3::NEG_Z),
    };

    let (rotation, look_target) = match &shot.orientation {
        OrientationTrack::Fixed(rotation) => {
            (*rotation, position + rotation.mul_vec3(Vec3::NEG_Z) * 10.0)
        }
        OrientationTrack::PathTangent(PathTangentOrientation { up, roll_radians }) => {
            let rotation =
                solver::solve_path_tangent_rotation(fallback_rotation, tangent, *up, *roll_radians);
            let forward = if tangent.length_squared() > f32::EPSILON {
                tangent.normalize()
            } else {
                rotation * Vec3::NEG_Z
            };
            (rotation, position + forward * 10.0)
        }
        OrientationTrack::LookAt(target) => {
            let look_target = resolve_look_target(
                target,
                transforms,
                target_groups,
                history,
                fallback_look_target,
            );
            let up = resolve_look_up(target, target_groups).unwrap_or(UpVectorMode::WorldY);
            (
                solver::solve_look_rotation(fallback_rotation, position, look_target, up),
                look_target,
            )
        }
    };

    let pose = SolvedCameraPose {
        translation: position,
        rotation,
        look_target,
        fov_y_radians: shot.lens.sample(progress),
    };

    solver::apply_procedural_shake(&pose, shot.shake, local_time_secs)
}

fn resolve_look_target(
    target: &LookAtTarget,
    transforms: &Query<&GlobalTransform>,
    target_groups: &Query<&CinematicTargetGroup>,
    history: &TargetHistory,
    fallback: Vec3,
) -> Vec3 {
    match target {
        LookAtTarget::Point { point, .. } => *point,
        LookAtTarget::Entity {
            entity,
            offset,
            look_ahead_secs,
            ..
        } => resolve_entity_target(*entity, *offset, *look_ahead_secs, transforms, history)
            .unwrap_or(fallback),
        LookAtTarget::GroupEntity(group_entity) => target_groups
            .get(*group_entity)
            .ok()
            .map(|group| resolve_target_group(group, transforms, history))
            .unwrap_or(fallback),
    }
}

fn resolve_look_up(
    target: &LookAtTarget,
    target_groups: &Query<&CinematicTargetGroup>,
) -> Option<crate::UpVectorMode> {
    match target {
        LookAtTarget::Point { up, .. } => Some(*up),
        LookAtTarget::Entity { up, .. } => Some(*up),
        LookAtTarget::GroupEntity(group_entity) => {
            target_groups.get(*group_entity).ok().map(|group| group.up)
        }
    }
}

fn resolve_target_group(
    group: &CinematicTargetGroup,
    transforms: &Query<&GlobalTransform>,
    history: &TargetHistory,
) -> Vec3 {
    solver::weighted_target_center(
        group.members.iter().filter_map(|weighted| {
            resolve_entity_target(
                weighted.entity,
                weighted.offset,
                group.look_ahead_secs,
                transforms,
                history,
            )
            .map(|point| (point, weighted.weight))
        }),
        group.fallback_point,
    )
}

fn resolve_entity_target(
    entity: Entity,
    offset: Vec3,
    look_ahead_secs: f32,
    transforms: &Query<&GlobalTransform>,
    history: &TargetHistory,
) -> Option<Vec3> {
    let position = transforms.get(entity).ok()?.translation() + offset;
    let velocity = history
        .entries
        .get(&entity)
        .map(|entry| entry.velocity)
        .unwrap_or(Vec3::ZERO);
    Some(position + velocity * look_ahead_secs.max(0.0))
}

fn emit_events_between(
    rig: Entity,
    from: f32,
    to: f32,
    sequence: &CinematicSequence,
    cache: &CinematicSequenceCache,
    include_from: bool,
    shot_started: &mut MessageWriter<ShotStarted>,
    shot_finished: &mut MessageWriter<ShotFinished>,
    markers: &mut MessageWriter<ShotMarkerReached>,
) {
    if to < from || (to == from && !include_from) {
        return;
    }

    for event in &cache.timeline_events {
        let before_range = if include_from {
            event.time_secs < from
        } else {
            event.time_secs <= from
        };
        if before_range {
            continue;
        }
        if event.time_secs > to {
            break;
        }

        let shot_name = sequence.shots[event.shot_index].name.clone();
        match &event.kind {
            CachedTimelineEventKind::ShotStarted => {
                shot_started.write(ShotStarted {
                    rig,
                    shot_index: event.shot_index,
                    shot_name,
                });
            }
            CachedTimelineEventKind::Marker(marker_name) => {
                markers.write(ShotMarkerReached {
                    rig,
                    shot_index: event.shot_index,
                    shot_name,
                    marker_name: marker_name.clone(),
                });
            }
            CachedTimelineEventKind::ShotFinished => {
                shot_finished.write(ShotFinished {
                    rig,
                    shot_index: event.shot_index,
                    shot_name,
                });
            }
        }
    }
}

#[cfg(test)]
#[path = "systems_tests.rs"]
mod tests;

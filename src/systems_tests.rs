use bevy::{
    camera::{PerspectiveProjection, Projection},
    ecs::message::{MessageCursor, Messages},
    prelude::*,
    time::TimeUpdateStrategy,
};

use super::CinematicSequenceCache;
use crate::{
    CinematicBlend, CinematicBlendCompleted, CinematicBlendKind, CinematicCameraBinding,
    CinematicCameraPlugin, CinematicCameraRig, CinematicDrivenCamera, CinematicPlayback,
    CinematicPlaybackCommand, CinematicPlaybackStatus, CinematicSequence, CinematicShot,
    CinematicVirtualCamera, MarkerTime, ShotMarker, ShotMarkerReached, ShotStarted,
};

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, CinematicCameraPlugin::always_on(Update)))
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(100),
        ));
    app
}

#[test]
fn activate_schedule_makes_runtime_live() {
    let mut app = test_app();
    app.update();
    assert!(
        app.world()
            .resource::<crate::resources::CinematicCameraRuntimeState>()
            .active
    );
}

#[test]
fn autoplay_advances_sequence_time() {
    let mut app = test_app();
    let rig = app
        .world_mut()
        .spawn((
            CinematicCameraRig {
                auto_play: true,
                enabled: true,
            },
            CinematicCameraBinding::default(),
            CinematicPlayback::default(),
            CinematicSequence {
                shots: vec![CinematicShot::fixed("A", 1.0, Vec3::ZERO, Quat::IDENTITY)],
                ..default()
            },
        ))
        .id();
    app.update();
    app.update();

    let playback = app.world().entity(rig).get::<CinematicPlayback>().unwrap();
    assert!(playback.elapsed_secs > 0.0);
}

#[test]
fn virtual_camera_authoring_creates_runtime_rig_and_binding() {
    let mut app = test_app();
    let camera = app
        .world_mut()
        .spawn((
            Transform::default(),
            Projection::Perspective(PerspectiveProjection::default()),
        ))
        .id();
    let rig = app
        .world_mut()
        .spawn((
            CinematicVirtualCamera {
                brain: camera,
                priority: 7,
                live: true,
                auto_play: false,
                ..default()
            },
            CinematicPlayback::default(),
            CinematicSequence {
                shots: vec![CinematicShot::fixed("A", 1.0, Vec3::ZERO, Quat::IDENTITY)],
                ..default()
            },
        ))
        .id();

    app.update();

    let rig_component = app.world().entity(rig).get::<CinematicCameraRig>().unwrap();
    let binding = app
        .world()
        .entity(rig)
        .get::<CinematicCameraBinding>()
        .unwrap();
    assert!(rig_component.enabled);
    assert_eq!(binding.camera, camera);
    assert_eq!(binding.priority, 7);
}

#[test]
fn seek_normalized_uses_cached_total_duration_instead_of_shot_sum() {
    let mut app = test_app();
    let rig = app
        .world_mut()
        .spawn((
            CinematicCameraRig::default(),
            CinematicCameraBinding::default(),
            CinematicPlayback::default(),
            CinematicSequence {
                shots: vec![
                    CinematicShot::fixed("Intro", 2.0, Vec3::ZERO, Quat::IDENTITY),
                    CinematicShot {
                        blend_in: CinematicBlend {
                            duration_secs: 0.5,
                            ..default()
                        },
                        ..CinematicShot::fixed(
                            "Close Up",
                            2.0,
                            Vec3::new(1.0, 0.0, 0.0),
                            Quat::IDENTITY,
                        )
                    },
                ],
                ..default()
            },
        ))
        .id();

    app.update();
    app.update();

    let cache = app
        .world()
        .entity(rig)
        .get::<CinematicSequenceCache>()
        .unwrap();
    assert!((cache.total_duration - 3.5).abs() < 0.0001);

    app.world_mut()
        .write_message(CinematicPlaybackCommand::SeekNormalized {
            rig,
            normalized: 1.0,
        });
    app.update();

    let playback = app.world().entity(rig).get::<CinematicPlayback>().unwrap();
    assert!((playback.elapsed_secs - 3.5).abs() < 0.0001);
}

#[test]
fn stop_with_restore_returns_camera_to_snapshot_and_releases_owner() {
    let mut app = test_app();

    let original_transform =
        Transform::from_xyz(-5.0, 2.5, 12.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y);
    let camera = app
        .world_mut()
        .spawn((
            original_transform,
            Projection::Perspective(PerspectiveProjection::default()),
        ))
        .id();
    let rig = app
        .world_mut()
        .spawn((
            CinematicCameraRig {
                auto_play: true,
                enabled: true,
            },
            CinematicCameraBinding {
                camera,
                ..default()
            },
            CinematicPlayback::default(),
            CinematicSequence {
                shots: vec![CinematicShot::fixed(
                    "Reveal",
                    0.5,
                    Vec3::new(8.0, 3.0, 4.0),
                    Quat::from_rotation_y(0.75),
                )],
                entry_blend: CinematicBlend::instant(),
                exit_blend: CinematicBlend {
                    duration_secs: 0.25,
                    ..default()
                },
                ..default()
            },
        ))
        .id();

    app.update();
    app.update();

    let driven_camera = app.world().entity(camera).get::<CinematicDrivenCamera>();
    assert!(driven_camera.is_some());

    let cinematic_transform = app.world().entity(camera).get::<Transform>().unwrap();
    assert!(
        cinematic_transform
            .translation
            .distance(original_transform.translation)
            > 1.0
    );

    app.world_mut()
        .write_message(CinematicPlaybackCommand::Stop {
            rig,
            restore_camera: true,
        });
    app.update();

    for _ in 0..5 {
        app.update();

        let playback = app.world().entity(rig).get::<CinematicPlayback>().unwrap();
        if playback.status == CinematicPlaybackStatus::Stopped {
            break;
        }
    }

    let playback = app.world().entity(rig).get::<CinematicPlayback>().unwrap();
    assert_eq!(playback.status, CinematicPlaybackStatus::Stopped);

    let restored_transform = app.world().entity(camera).get::<Transform>().unwrap();
    assert!(
        restored_transform
            .translation
            .distance(original_transform.translation)
            < 0.0001
    );
    assert!(
        restored_transform
            .rotation
            .angle_between(original_transform.rotation)
            < 0.001
    );
    assert!(
        app.world()
            .entity(camera)
            .get::<CinematicDrivenCamera>()
            .is_none()
    );

    let mut cursor = MessageCursor::<CinematicBlendCompleted>::default();
    let blends: Vec<_> = cursor
        .read(app.world().resource::<Messages<CinematicBlendCompleted>>())
        .cloned()
        .collect();
    assert!(blends.iter().any(|message| {
        message.rig == rig
            && matches!(
                message.kind,
                CinematicBlendKind::CinematicToGameplay { from_shot: Some(0) }
            )
    }));
}

#[test]
fn autoplay_emits_first_shot_and_time_zero_marker() {
    let mut app = test_app();
    let rig = app
        .world_mut()
        .spawn((
            CinematicCameraRig {
                auto_play: true,
                enabled: true,
            },
            CinematicCameraBinding::default(),
            CinematicPlayback::default(),
            CinematicSequence {
                shots: vec![CinematicShot {
                    name: "Intro".into(),
                    duration_secs: 1.0,
                    markers: vec![ShotMarker {
                        name: "frame_zero".into(),
                        at: MarkerTime::Seconds(0.0),
                    }],
                    ..CinematicShot::fixed("Intro", 1.0, Vec3::ZERO, Quat::IDENTITY)
                }],
                ..default()
            },
        ))
        .id();

    app.update();
    app.update();

    let mut started_cursor = MessageCursor::<ShotStarted>::default();
    let started: Vec<_> = started_cursor
        .read(app.world().resource::<Messages<ShotStarted>>())
        .cloned()
        .collect();
    assert!(started.iter().any(|message| {
        message.rig == rig && message.shot_index == 0 && message.shot_name == "Intro"
    }));

    let mut marker_cursor = MessageCursor::<ShotMarkerReached>::default();
    let markers: Vec<_> = marker_cursor
        .read(app.world().resource::<Messages<ShotMarkerReached>>())
        .cloned()
        .collect();
    assert!(markers.iter().any(|message| {
        message.rig == rig
            && message.shot_index == 0
            && message.shot_name == "Intro"
            && message.marker_name == "frame_zero"
    }));
}

#[test]
fn restart_from_stopped_recaptures_current_gameplay_snapshot() {
    let mut app = test_app();

    let original_transform =
        Transform::from_xyz(-5.0, 2.5, 12.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y);
    let updated_transform =
        Transform::from_xyz(7.0, 3.5, -10.0).looking_at(Vec3::new(1.0, 1.5, 0.0), Vec3::Y);

    let camera = app
        .world_mut()
        .spawn((
            original_transform,
            Projection::Perspective(PerspectiveProjection::default()),
        ))
        .id();
    let rig = app
        .world_mut()
        .spawn((
            CinematicCameraRig::default(),
            CinematicCameraBinding {
                camera,
                ..default()
            },
            CinematicPlayback::default(),
            CinematicSequence {
                shots: vec![CinematicShot::fixed(
                    "Reveal",
                    0.3,
                    Vec3::new(8.0, 3.0, 4.0),
                    Quat::from_rotation_y(0.75),
                )],
                restore_camera_on_finish: true,
                entry_blend: CinematicBlend::instant(),
                exit_blend: CinematicBlend::instant(),
                ..default()
            },
        ))
        .id();

    app.update();
    app.update();

    app.world_mut()
        .write_message(CinematicPlaybackCommand::Play(rig));
    app.update();
    app.world_mut()
        .write_message(CinematicPlaybackCommand::Stop {
            rig,
            restore_camera: false,
        });
    app.update();

    {
        let mut camera_entity = app.world_mut().entity_mut(camera);
        let mut camera_transform = camera_entity
            .get_mut::<Transform>()
            .expect("camera transform should exist");
        *camera_transform = updated_transform;
    }

    app.world_mut()
        .write_message(CinematicPlaybackCommand::Restart(rig));
    for _ in 0..8 {
        app.update();

        let playback = app.world().entity(rig).get::<CinematicPlayback>().unwrap();
        if playback.status == CinematicPlaybackStatus::Stopped {
            break;
        }
    }

    let playback = app.world().entity(rig).get::<CinematicPlayback>().unwrap();
    assert_eq!(playback.status, CinematicPlaybackStatus::Stopped);

    let restored_transform = app.world().entity(camera).get::<Transform>().unwrap();
    assert!(
        restored_transform
            .translation
            .distance(updated_transform.translation)
            < 0.0001
    );
    assert!(
        restored_transform
            .rotation
            .angle_between(updated_transform.rotation)
            < 0.001
    );
    assert!(
        restored_transform
            .translation
            .distance(original_transform.translation)
            > 1.0
    );
}

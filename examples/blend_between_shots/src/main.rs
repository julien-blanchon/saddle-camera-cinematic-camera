use saddle_camera_cinematic_camera_example_common as common;

use bevy::prelude::*;
use common::{DemoRig, OrbitTarget};
use saddle_camera_cinematic_camera::*;

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, CinematicCameraPlugin::always_on(Update)));
    common::install_common(&mut app, "Blend Between Shots");
    app.add_systems(Startup, setup);
    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    common::spawn_demo_scene(&mut commands, &mut meshes, &mut materials);
    common::spawn_overlay(&mut commands);

    let subject = common::spawn_orbit_target(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Reveal Subject",
        Color::srgb(0.80, 0.26, 0.26),
        OrbitTarget {
            center: Vec3::ZERO,
            radius: 3.0,
            height: 1.4,
            speed: 0.45,
            phase: 0.0,
        },
    );

    let camera = common::spawn_camera(
        &mut commands,
        Transform::from_xyz(-4.0, 2.4, 12.0).looking_at(Vec3::new(0.0, 1.5, 0.0), Vec3::Y),
    );

    let opening = CinematicShot {
        name: "Establish".into(),
        duration_secs: 2.8,
        position: PositionTrack::Fixed(Vec3::new(-10.0, 5.0, 14.0)),
        orientation: OrientationTrack::LookAt(LookAtTarget::Entity {
            entity: subject,
            offset: Vec3::new(0.0, 0.8, 0.0),
            up: UpVectorMode::WorldY,
            look_ahead_secs: 0.15,
        }),
        lens: LensTrack::fixed(0.92),
        ..default()
    };

    let mut push_in = CinematicShot::rail(
        "Push In",
        3.5,
        CinematicRail {
            points: vec![
                Vec3::new(-2.5, 2.4, 11.0),
                Vec3::new(-1.0, 2.2, 8.5),
                Vec3::new(2.5, 2.1, 6.0),
                Vec3::new(5.5, 2.0, 4.0),
            ],
            kind: RailSplineKind::Linear,
            ..default()
        },
    );
    push_in.orientation = OrientationTrack::LookAt(LookAtTarget::Entity {
        entity: subject,
        offset: Vec3::new(0.0, 0.6, 0.0),
        up: UpVectorMode::WorldY,
        look_ahead_secs: 0.2,
    });
    push_in.lens = LensTrack {
        start_fov_y_radians: 0.85,
        end_fov_y_radians: 0.62,
        easing: CinematicEasing::CubicInOut,
    };
    push_in.blend_in = CinematicBlend {
        duration_secs: 1.0,
        easing: CinematicEasing::SineInOut,
    };
    push_in
        .markers
        .push(ShotMarker::normalized("push_peak", 0.72));

    let close_up = CinematicShot {
        name: "Close Up".into(),
        duration_secs: 2.2,
        position: PositionTrack::Fixed(Vec3::new(6.5, 2.2, 3.2)),
        orientation: OrientationTrack::LookAt(LookAtTarget::Entity {
            entity: subject,
            offset: Vec3::new(0.0, 0.5, 0.0),
            up: UpVectorMode::WorldY,
            look_ahead_secs: 0.1,
        }),
        lens: LensTrack::fixed(0.54),
        blend_in: CinematicBlend {
            duration_secs: 0.7,
            easing: CinematicEasing::SmoothStep,
        },
        ..default()
    };
    let rig = CinematicCameraRig {
        auto_play: true,
        enabled: true,
    };
    let sequence = CinematicSequence {
        shots: vec![opening, push_in, close_up],
        restore_camera_on_finish: true,
        entry_blend: CinematicBlend {
            duration_secs: 1.0,
            easing: CinematicEasing::CubicInOut,
        },
        exit_blend: CinematicBlend {
            duration_secs: 1.0,
            easing: CinematicEasing::SineInOut,
        },
        ..default()
    };

    commands.spawn((
        Name::new("Blend Showcase Rig"),
        DemoRig,
        rig.clone(),
        CinematicCameraBinding {
            camera,
            ..default()
        },
        CinematicPlayback::default(),
        sequence.clone(),
    ));
    common::queue_example_pane(
        &mut commands,
        common::ExampleCinematicPane::from_setup(
            rig.enabled,
            rig.auto_play,
            1.0,
            sequence.entry_blend.duration_secs,
            sequence.exit_blend.duration_secs,
            &CinematicCameraDebugSettings {
                enabled: true,
                ..default()
            },
        ),
    );
}

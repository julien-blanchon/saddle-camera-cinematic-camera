use saddle_camera_cinematic_camera_example_common as common;

use bevy::prelude::*;
use saddle_camera_cinematic_camera::*;
use common::{DemoRig, OrbitTarget};

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, CinematicCameraPlugin::always_on(Update)));
    common::install_common(&mut app, "Moving Target With Look-Ahead");
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
        "Moving Target",
        Color::srgb(0.18, 0.74, 0.68),
        OrbitTarget {
            center: Vec3::new(0.0, 0.0, 0.0),
            radius: 8.0,
            height: 1.6,
            speed: 0.75,
            phase: 1.3,
        },
    );

    let camera = common::spawn_camera(
        &mut commands,
        Transform::from_xyz(-12.0, 4.0, 16.0).looking_at(Vec3::ZERO, Vec3::Y),
    );

    let mut tracking = CinematicShot::rail(
        "Inspection Rail",
        10.0,
        CinematicRail {
            points: vec![
                Vec3::new(-14.0, 5.0, 12.0),
                Vec3::new(-4.0, 4.0, 14.0),
                Vec3::new(7.0, 5.2, 10.0),
                Vec3::new(14.0, 4.4, -4.0),
                Vec3::new(4.0, 4.5, -12.0),
                Vec3::new(-10.0, 5.5, -8.0),
            ],
            closed: true,
            ..default()
        },
    );
    tracking.orientation = OrientationTrack::LookAt(LookAtTarget::Entity {
        entity: subject,
        offset: Vec3::new(0.0, 0.5, 0.0),
        up: UpVectorMode::WorldY,
        look_ahead_secs: 0.35,
    });
    tracking.lens = LensTrack {
        start_fov_y_radians: 0.8,
        end_fov_y_radians: 0.65,
        easing: CinematicEasing::SineInOut,
    };

    commands.spawn((
        Name::new("Moving Target Rig"),
        DemoRig,
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
            shots: vec![tracking],
            loop_mode: PlaybackLoopMode::Loop,
            entry_blend: CinematicBlend {
                duration_secs: 0.8,
                easing: CinematicEasing::SmoothStep,
            },
            ..default()
        },
    ));
}

use saddle_camera_cinematic_camera_example_common as common;

use bevy::prelude::*;
use saddle_camera_cinematic_camera::*;
use common::DemoRig;

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, CinematicCameraPlugin::always_on(Update)));
    common::install_common(&mut app, "Handheld Rail Pass");
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

    let camera = common::spawn_camera(
        &mut commands,
        Transform::from_xyz(-10.0, 4.0, 14.0).looking_at(Vec3::ZERO, Vec3::Y),
    );

    let mut handheld = CinematicShot::rail(
        "Handheld Sweep",
        9.0,
        CinematicRail {
            points: vec![
                Vec3::new(-12.0, 4.4, 10.0),
                Vec3::new(-5.0, 4.1, 7.0),
                Vec3::new(2.0, 4.6, 5.0),
                Vec3::new(10.0, 4.3, 2.0),
                Vec3::new(13.0, 4.8, -6.0),
            ],
            kind: RailSplineKind::Linear,
            ..default()
        },
    );
    handheld.orientation = OrientationTrack::LookAt(LookAtTarget::Point {
        point: Vec3::new(0.0, 1.8, 0.0),
        up: UpVectorMode::WorldY,
    });
    handheld.shake = ProceduralShake {
        translation_amplitude: Vec3::new(0.08, 0.05, 0.04),
        rotation_amplitude_radians: Vec3::new(0.012, 0.015, 0.02),
        frequency_hz: Vec3::new(1.8, 2.3, 2.8),
        seed: 4.0,
    };
    handheld.lens = LensTrack {
        start_fov_y_radians: 0.82,
        end_fov_y_radians: 0.74,
        easing: CinematicEasing::Linear,
    };

    commands.spawn((
        Name::new("Handheld Rig"),
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
            shots: vec![handheld],
            loop_mode: PlaybackLoopMode::Loop,
            entry_blend: CinematicBlend {
                duration_secs: 0.9,
                easing: CinematicEasing::SineInOut,
            },
            ..default()
        },
    ));
}

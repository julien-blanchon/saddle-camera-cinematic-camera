use saddle_camera_cinematic_camera_example_common as common;

use bevy::prelude::*;
use common::DemoRig;
use saddle_camera_cinematic_camera::*;

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, CinematicCameraPlugin::always_on(Update)));
    common::install_common(&mut app, "Basic Flythrough");
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
        Transform::from_xyz(-8.0, 4.0, 18.0).looking_at(Vec3::ZERO, Vec3::Y),
    );

    let mut flythrough = CinematicShot::rail(
        "Scenic Loop",
        14.0,
        CinematicRail {
            closed: true,
            points: vec![
                Vec3::new(-14.0, 6.0, 16.0),
                Vec3::new(-2.0, 4.5, 20.0),
                Vec3::new(16.0, 7.0, 8.0),
                Vec3::new(10.0, 5.0, -14.0),
                Vec3::new(-10.0, 6.5, -18.0),
                Vec3::new(-18.0, 7.5, 2.0),
            ],
            ..default()
        },
    );
    flythrough.orientation = OrientationTrack::LookAt(LookAtTarget::Point {
        point: Vec3::new(0.0, 1.5, 0.0),
        up: UpVectorMode::WorldY,
    });
    flythrough.lens = LensTrack {
        start_fov_y_radians: 0.75,
        end_fov_y_radians: 0.95,
        easing: CinematicEasing::SineInOut,
    };

    commands.spawn((
        Name::new("Basic Flythrough Rig"),
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
            shots: vec![flythrough],
            loop_mode: PlaybackLoopMode::Loop,
            entry_blend: CinematicBlend {
                duration_secs: 1.5,
                easing: CinematicEasing::SineInOut,
            },
            ..default()
        },
    ));
}

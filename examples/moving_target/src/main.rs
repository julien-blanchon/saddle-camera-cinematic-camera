use saddle_camera_cinematic_camera_example_common as common;

use bevy::prelude::*;
use common::{DemoRig, OrbitTarget};
use saddle_camera_cinematic_camera::*;

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
            radius: 5.0,
            height: 1.6,
            speed: 0.55,
            phase: 1.3,
        },
    );

    let camera = common::spawn_camera(
        &mut commands,
        Transform::from_xyz(-8.0, 4.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    );

    let mut tracking = CinematicShot::rail(
        "Inspection Rail",
        10.0,
        CinematicRail {
            points: vec![
                Vec3::new(-10.0, 4.0, 8.0),
                Vec3::new(-3.0, 3.5, 10.0),
                Vec3::new(5.0, 4.2, 7.0),
                Vec3::new(9.0, 3.8, -2.0),
                Vec3::new(3.0, 4.0, -8.0),
                Vec3::new(-7.0, 4.5, -5.0),
            ],
            closed: true,
            ..default()
        },
    );
    tracking.orientation = OrientationTrack::LookAt(LookAtTarget::Entity {
        entity: subject,
        offset: Vec3::new(0.0, 0.5, 0.0),
        up: UpVectorMode::WorldY,
        look_ahead_secs: 0.15,
    });
    tracking.lens = LensTrack {
        start_fov_y_radians: 0.8,
        end_fov_y_radians: 0.65,
        easing: CinematicEasing::SineInOut,
    };
    let rig = CinematicCameraRig {
        auto_play: true,
        enabled: true,
    };
    let sequence = CinematicSequence {
        shots: vec![tracking],
        loop_mode: PlaybackLoopMode::Loop,
        entry_blend: CinematicBlend {
            duration_secs: 0.8,
            easing: CinematicEasing::SmoothStep,
        },
        ..default()
    };

    commands.spawn((
        Name::new("Moving Target Rig"),
        DemoRig,
        rig.clone(),
        CinematicCameraBinding {
            camera,
            ..default()
        },
        CinematicPlayback::default(),
        sequence.clone(),
        // Output damping smooths look-ahead tracking of the fast-moving target.
        CinematicOutputDamping::default(),
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

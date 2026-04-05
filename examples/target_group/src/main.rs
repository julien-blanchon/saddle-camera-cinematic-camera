use saddle_camera_cinematic_camera_example_common as common;

use bevy::prelude::*;
use common::{DemoRig, OrbitTarget};
use saddle_camera_cinematic_camera::*;

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, CinematicCameraPlugin::always_on(Update)));
    common::install_common(&mut app, "Weighted Target Group");
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

    let lead = common::spawn_orbit_target(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Lead Subject",
        Color::srgb(0.92, 0.48, 0.26),
        OrbitTarget {
            center: Vec3::new(-2.0, 0.0, 0.0),
            radius: 4.0,
            height: 1.5,
            speed: 0.6,
            phase: 0.0,
        },
    );
    let wing = common::spawn_orbit_target(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Wing Subject",
        Color::srgb(0.22, 0.62, 0.86),
        OrbitTarget {
            center: Vec3::new(2.0, 0.0, 0.0),
            radius: 5.5,
            height: 1.4,
            speed: -0.48,
            phase: 1.7,
        },
    );
    let group = common::spawn_target_group(
        &mut commands,
        "Dialogue Framing Group",
        vec![
            WeightedTarget {
                entity: lead,
                weight: 1.2,
                offset: Vec3::new(0.0, 0.35, 0.0),
            },
            WeightedTarget {
                entity: wing,
                weight: 0.9,
                offset: Vec3::new(0.0, 0.35, 0.0),
            },
        ],
        Vec3::new(0.0, 1.2, 0.0),
    );

    let camera = common::spawn_camera(
        &mut commands,
        Transform::from_xyz(-12.0, 5.0, 16.0).looking_at(Vec3::ZERO, Vec3::Y),
    );

    let mut group_shot = CinematicShot::rail(
        "Group Frame",
        12.0,
        CinematicRail {
            points: vec![
                Vec3::new(-14.0, 5.0, 12.0),
                Vec3::new(-4.0, 5.0, 14.0),
                Vec3::new(8.0, 5.0, 12.0),
                Vec3::new(12.0, 5.4, 2.0),
                Vec3::new(2.0, 5.0, -12.0),
                Vec3::new(-12.0, 5.5, -8.0),
            ],
            closed: true,
            ..default()
        },
    );
    group_shot.orientation = OrientationTrack::LookAt(LookAtTarget::GroupEntity(group));
    group_shot.lens = LensTrack {
        start_fov_y_radians: 0.84,
        end_fov_y_radians: 0.70,
        easing: CinematicEasing::SmoothStep,
    };
    let rig = CinematicCameraRig {
        auto_play: true,
        enabled: true,
    };
    let sequence = CinematicSequence {
        shots: vec![group_shot],
        loop_mode: PlaybackLoopMode::Loop,
        entry_blend: CinematicBlend {
            duration_secs: 1.0,
            easing: CinematicEasing::CubicInOut,
        },
        ..default()
    };

    commands.spawn((
        Name::new("Target Group Rig"),
        DemoRig,
        rig.clone(),
        CinematicCameraBinding {
            camera,
            ..default()
        },
        CinematicPlayback::default(),
        sequence.clone(),
        // Heavy damping to smooth the rapidly-moving weighted centroid of two targets.
        CinematicOutputDamping::heavy(),
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

use saddle_camera_cinematic_camera_example_common as common;

use bevy::prelude::*;
use common::DemoRig;
use saddle_camera_cinematic_camera::*;

const PREVIEW_RIGS: usize = 100;

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, CinematicCameraPlugin::always_on(Update)));
    common::install_common(&mut app, "Stress Preview");
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
        Transform::from_xyz(-10.0, 6.0, 18.0).looking_at(Vec3::ZERO, Vec3::Y),
    );

    let active_shot = CinematicShot::rail(
        "Active Rail",
        11.0,
        CinematicRail {
            closed: true,
            points: vec![
                Vec3::new(-12.0, 5.0, 12.0),
                Vec3::new(-2.0, 5.5, 14.0),
                Vec3::new(10.0, 6.0, 10.0),
                Vec3::new(12.0, 5.2, -6.0),
                Vec3::new(-4.0, 5.8, -12.0),
            ],
            ..default()
        },
    );

    commands.spawn((
        Name::new("Stress Active Rig"),
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
            shots: vec![active_shot],
            loop_mode: PlaybackLoopMode::Loop,
            entry_blend: CinematicBlend {
                duration_secs: 0.7,
                easing: CinematicEasing::SineInOut,
            },
            ..default()
        },
    ));

    for index in 0..PREVIEW_RIGS {
        let radius = 6.0 + (index % 10) as f32 * 0.9;
        let height = 1.8 + (index / 10) as f32 * 0.05;
        let angle = index as f32 * 0.3;
        let preview_shot = CinematicShot::fixed(
            format!("Preview {}", index + 1),
            1.0,
            Vec3::new(angle.cos() * radius, height, angle.sin() * radius),
            Transform::from_xyz(angle.cos() * radius, height, angle.sin() * radius)
                .looking_at(Vec3::ZERO, Vec3::Y)
                .rotation,
        );

        commands.spawn((
            Name::new(format!("Preview Rig {}", index + 1)),
            CinematicCameraRig::default(),
            CinematicCameraBinding::default(),
            CinematicPlayback::default(),
            CinematicSequence {
                shots: vec![preview_shot],
                ..default()
            },
        ));
    }
}

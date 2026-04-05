use saddle_camera_cinematic_camera_example_common as common;

use bevy::prelude::*;
use common::OrbitTarget;
use saddle_camera_cinematic_camera::*;

#[derive(Resource)]
struct BrainCycle {
    timer: Timer,
    active_index: usize,
    virtual_cameras: [Entity; 2],
}

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, CinematicCameraPlugin::always_on(Update)));
    common::install_common(&mut app, "Virtual Camera Brain");
    app.add_systems(Startup, setup);
    app.add_systems(Update, cycle_virtual_cameras);
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
        "Hero Subject",
        Color::srgb(0.90, 0.34, 0.24),
        OrbitTarget {
            center: Vec3::new(0.0, 0.0, 0.0),
            radius: 4.8,
            height: 1.6,
            speed: 0.52,
            phase: 0.0,
        },
    );

    let brain = commands
        .spawn((
            Name::new("Brain Camera"),
            CinematicCameraBrain,
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection {
                fov: 0.84,
                ..default()
            }),
            Transform::from_xyz(-6.0, 3.0, 14.0).looking_at(Vec3::new(0.0, 1.2, 0.0), Vec3::Y),
        ))
        .id();

    let overview_sequence = CinematicSequence {
        shots: vec![CinematicShot {
            name: "Overview".into(),
            duration_secs: 6.0,
            position: PositionTrack::Rail(RailTrack {
                rail: CinematicRail {
                    closed: true,
                    points: vec![
                        Vec3::new(-14.0, 6.0, 14.0),
                        Vec3::new(-4.0, 5.4, 18.0),
                        Vec3::new(12.0, 6.2, 10.0),
                        Vec3::new(16.0, 5.8, -8.0),
                        Vec3::new(-6.0, 6.4, -14.0),
                    ],
                    ..default()
                },
                traversal: RailTraversal::default(),
            }),
            orientation: OrientationTrack::LookAt(LookAtTarget::Entity {
                entity: subject,
                offset: Vec3::new(0.0, 0.6, 0.0),
                up: UpVectorMode::WorldY,
                look_ahead_secs: 0.2,
            }),
            lens: LensTrack::fixed(0.90),
            ..default()
        }],
        loop_mode: PlaybackLoopMode::Loop,
        entry_blend: CinematicBlend {
            duration_secs: 1.0,
            easing: CinematicEasing::SineInOut,
        },
        ..default()
    };

    let close_sequence = CinematicSequence {
        shots: vec![CinematicShot {
            name: "Close Chase".into(),
            duration_secs: 4.0,
            position: PositionTrack::Rail(RailTrack {
                rail: CinematicRail {
                    points: vec![
                        Vec3::new(-2.0, 2.4, 8.5),
                        Vec3::new(1.0, 2.1, 6.5),
                        Vec3::new(4.8, 2.0, 4.4),
                        Vec3::new(7.0, 2.2, 3.2),
                    ],
                    kind: RailSplineKind::Linear,
                    ..default()
                },
                traversal: RailTraversal::default(),
            }),
            orientation: OrientationTrack::LookAt(LookAtTarget::Entity {
                entity: subject,
                offset: Vec3::new(0.0, 0.5, 0.0),
                up: UpVectorMode::WorldY,
                look_ahead_secs: 0.25,
            }),
            lens: LensTrack {
                start_fov_y_radians: 0.76,
                end_fov_y_radians: 0.58,
                easing: CinematicEasing::CubicInOut,
            },
            blend_in: CinematicBlend {
                duration_secs: 0.6,
                easing: CinematicEasing::SmoothStep,
            },
            ..default()
        }],
        loop_mode: PlaybackLoopMode::Loop,
        entry_blend: CinematicBlend {
            duration_secs: 0.8,
            easing: CinematicEasing::CubicInOut,
        },
        ..default()
    };

    let overview = commands
        .spawn((
            Name::new("Overview Virtual Camera"),
            common::DemoRig,
            CinematicVirtualCamera {
                brain,
                priority: 20,
                live: true,
                auto_play: true,
                ..default()
            },
            CinematicPlayback::default(),
            overview_sequence.clone(),
            // Output damping smooths entity tracking during priority handoffs.
            CinematicOutputDamping::light(),
        ))
        .id();

    let close = commands
        .spawn((
            Name::new("Close Virtual Camera"),
            CinematicVirtualCamera {
                brain,
                priority: 5,
                live: true,
                auto_play: true,
                ..default()
            },
            CinematicPlayback::default(),
            close_sequence.clone(),
            CinematicOutputDamping::light(),
        ))
        .id();

    commands.insert_resource(BrainCycle {
        timer: Timer::from_seconds(4.5, TimerMode::Repeating),
        active_index: 0,
        virtual_cameras: [overview, close],
    });
    common::queue_example_pane(
        &mut commands,
        common::ExampleCinematicPane::from_setup(
            true,
            true,
            1.0,
            overview_sequence.entry_blend.duration_secs,
            overview_sequence.exit_blend.duration_secs,
            &CinematicCameraDebugSettings {
                enabled: true,
                ..default()
            },
        ),
    );
}

fn cycle_virtual_cameras(
    time: Res<Time>,
    mut cycle: ResMut<BrainCycle>,
    mut virtual_cameras: Query<&mut CinematicVirtualCamera>,
) {
    if !cycle.timer.tick(time.delta()).just_finished() {
        return;
    }

    cycle.active_index = (cycle.active_index + 1) % cycle.virtual_cameras.len();

    for (index, entity) in cycle.virtual_cameras.into_iter().enumerate() {
        let Ok(mut camera) = virtual_cameras.get_mut(entity) else {
            continue;
        };
        camera.priority = if index == cycle.active_index { 30 } else { 5 };
        camera.solo = index == cycle.active_index;
    }
}

#[cfg(feature = "e2e")]
mod e2e;

use saddle_camera_cinematic_camera_example_common as common;

use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use saddle_camera_cinematic_camera::*;

#[derive(Component)]
struct LabOverlay;

#[derive(Component)]
struct OrbitTarget {
    center: Vec3,
    radius: f32,
    height: f32,
    speed: f32,
    phase: f32,
}

#[derive(Resource)]
struct LabHandles {
    rig: Entity,
    camera: Entity,
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugins(CinematicCameraPlugin::always_on(Update));
    common::install_common(&mut app, "Cinematic Lab");
    #[cfg(feature = "e2e")]
    app.add_plugins(e2e::CinematicCameraLabE2EPlugin);
    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
            animate_targets,
            restart_sequence.run_if(input_just_pressed(KeyCode::Space)),
            update_overlay,
        ),
    );

    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Name::new("Lab Sun"),
        DirectionalLight {
            illuminance: 24_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(14.0, 20.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Name::new("Lab Ground"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(80.0, 80.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.08, 0.09, 0.11),
            perceptual_roughness: 1.0,
            ..default()
        })),
    ));

    for (index, (translation, color)) in [
        (Vec3::new(-9.0, 1.2, -7.0), Color::srgb(0.24, 0.46, 0.78)),
        (Vec3::new(-1.0, 2.0, 1.5), Color::srgb(0.76, 0.36, 0.24)),
        (Vec3::new(8.0, 1.5, -3.5), Color::srgb(0.22, 0.62, 0.54)),
        (Vec3::new(11.0, 1.0, 8.0), Color::srgb(0.66, 0.58, 0.18)),
    ]
    .into_iter()
    .enumerate()
    {
        commands.spawn((
            Name::new(format!("Lab Tower {}", index + 1)),
            Mesh3d(meshes.add(Cuboid::new(2.0, 2.4 + index as f32, 2.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.82,
                ..default()
            })),
            Transform::from_translation(translation),
        ));
    }

    let subject = commands
        .spawn((
            Name::new("Lab Subject"),
            OrbitTarget {
                center: Vec3::ZERO,
                radius: 3.2,
                height: 1.5,
                speed: 0.42,
                phase: 0.0,
            },
            Mesh3d(meshes.add(Sphere::new(0.8).mesh().uv(24, 16))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.84, 0.30, 0.28),
                metallic: 0.05,
                perceptual_roughness: 0.28,
                ..default()
            })),
            Transform::from_xyz(3.2, 1.5, 0.0),
        ))
        .id();

    let camera = commands
        .spawn((
            Name::new("Lab Camera"),
            CinematicCameraBrain,
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection {
                fov: 0.9,
                ..default()
            }),
            Transform::from_xyz(-5.0, 2.8, 12.0).looking_at(Vec3::new(0.0, 1.4, 0.0), Vec3::Y),
        ))
        .id();

    let opening = CinematicShot {
        name: "Establish".into(),
        duration_secs: 1.8,
        position: PositionTrack::Fixed(Vec3::new(-10.0, 5.2, 13.5)),
        orientation: OrientationTrack::LookAt(LookAtTarget::Entity {
            entity: subject,
            offset: Vec3::new(0.0, 0.6, 0.0),
            up: UpVectorMode::WorldY,
            look_ahead_secs: 0.2,
        }),
        lens: LensTrack::fixed(0.92),
        ..default()
    };

    let mut push = CinematicShot::rail(
        "Push",
        2.4,
        CinematicRail {
            points: vec![
                Vec3::new(-2.0, 2.4, 10.0),
                Vec3::new(0.0, 2.2, 7.6),
                Vec3::new(3.5, 2.1, 5.4),
                Vec3::new(6.0, 2.0, 3.4),
            ],
            kind: RailSplineKind::Linear,
            ..default()
        },
    );
    push.orientation = OrientationTrack::LookAt(LookAtTarget::Entity {
        entity: subject,
        offset: Vec3::new(0.0, 0.5, 0.0),
        up: UpVectorMode::WorldY,
        look_ahead_secs: 0.25,
    });
    push.blend_in = CinematicBlend {
        duration_secs: 0.7,
        easing: CinematicEasing::SineInOut,
    };
    push.lens = LensTrack {
        start_fov_y_radians: 0.84,
        end_fov_y_radians: 0.62,
        easing: CinematicEasing::CubicInOut,
    };
    push.markers.push(ShotMarker::normalized("push_peak", 0.66));

    let close_up = CinematicShot {
        name: "Close Up".into(),
        duration_secs: 1.8,
        position: PositionTrack::Fixed(Vec3::new(6.6, 2.2, 3.2)),
        orientation: OrientationTrack::LookAt(LookAtTarget::Entity {
            entity: subject,
            offset: Vec3::new(0.0, 0.5, 0.0),
            up: UpVectorMode::WorldY,
            look_ahead_secs: 0.15,
        }),
        lens: LensTrack::fixed(0.54),
        blend_in: CinematicBlend {
            duration_secs: 0.5,
            easing: CinematicEasing::SmoothStep,
        },
        ..default()
    };
    let rig_settings = CinematicCameraRig {
        auto_play: true,
        enabled: true,
    };
    let sequence = CinematicSequence {
        shots: vec![opening, push, close_up],
        restore_camera_on_finish: true,
        entry_blend: CinematicBlend {
            duration_secs: 0.8,
            easing: CinematicEasing::CubicInOut,
        },
        exit_blend: CinematicBlend {
            duration_secs: 0.9,
            easing: CinematicEasing::SineInOut,
        },
        ..default()
    };

    let rig = commands
        .spawn((
            Name::new("Cinematic Lab Rig"),
            common::DemoRig,
            CinematicVirtualCamera {
                brain: camera,
                priority: 20,
                live: rig_settings.enabled,
                auto_play: rig_settings.auto_play,
                ..default()
            },
            CinematicPlayback::default(),
            sequence.clone(),
        ))
        .id();

    commands.insert_resource(LabHandles { rig, camera });
    common::queue_example_pane(
        &mut commands,
        common::ExampleCinematicPane::from_setup(
            rig_settings.enabled,
            rig_settings.auto_play,
            1.0,
            sequence.entry_blend.duration_secs,
            sequence.exit_blend.duration_secs,
            &CinematicCameraDebugSettings {
                enabled: true,
                ..default()
            },
        ),
    );

    commands.spawn((
        Name::new("Cinematic Camera Lab Overlay"),
        LabOverlay,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(20.0),
            top: Val::Px(20.0),
            width: Val::Px(420.0),
            padding: UiRect::all(Val::Px(14.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.04, 0.05, 0.08, 0.78)),
        Text::new(""),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

fn animate_targets(time: Res<Time>, mut query: Query<(&OrbitTarget, &mut Transform)>) {
    for (orbit, mut transform) in &mut query {
        let angle = orbit.phase + time.elapsed_secs() * orbit.speed;
        transform.translation = orbit.center
            + Vec3::new(
                angle.cos() * orbit.radius,
                orbit.height,
                angle.sin() * orbit.radius,
            );
    }
}

fn restart_sequence(
    handles: Res<LabHandles>,
    mut commands: MessageWriter<CinematicPlaybackCommand>,
) {
    commands.write(CinematicPlaybackCommand::Restart(handles.rig));
}

fn update_overlay(
    diagnostics: Res<CinematicCameraDiagnostics>,
    handles: Res<LabHandles>,
    camera_query: Query<Option<&CinematicDrivenCamera>, With<Camera>>,
    rig_query: Query<(&CinematicPlayback, &CinematicCameraState)>,
    mut overlay: Query<&mut Text, With<LabOverlay>>,
) {
    let Ok((playback, state)) = rig_query.get(handles.rig) else {
        return;
    };
    let Ok(owner) = camera_query.get(handles.camera) else {
        return;
    };
    let Ok(mut text) = overlay.single_mut() else {
        return;
    };

    *text = Text::new(format!(
        "Cinematic Camera Lab\nstatus: {:?}\nsequence time: {:.2}s\ncurrent shot: {:?}\nblend: {:?} -> {:?} ({:.2})\ncamera owner: {}\nactive rigs: {}\napplied cameras: {}",
        playback.status,
        state.sequence_time_secs,
        state.current_shot,
        state.blend_from_shot,
        state.blend_to_shot,
        state.blend_alpha,
        owner.map_or("gameplay".to_string(), |owner| format!("{:?}", owner.owner)),
        diagnostics.active_rigs,
        diagnostics.applied_cameras,
    ));
}

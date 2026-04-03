use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use saddle_camera_cinematic_camera::*;
use saddle_camera_cinematic_camera_example_common as common;
use saddle_camera_orbit_camera::{
    OrbitCamera, OrbitCameraInputTarget, OrbitCameraPlugin, OrbitCameraSettings,
};
use saddle_pane::prelude::*;

#[derive(Component)]
struct HeroSpinner {
    speed: f32,
}

#[derive(Component)]
struct HandoffOverlay;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveCameraMode {
    Cinematic,
    Orbit,
}

#[derive(Resource)]
struct HandoffState {
    rig: Entity,
    cinematic_camera: Entity,
    orbit_camera: Entity,
    active_mode: ActiveCameraMode,
}

#[derive(Resource, Debug, Clone, Copy, Default, Pane)]
#[pane(title = "Orbit Viewer", position = "bottom-right")]
struct IntegrationPane {
    #[pane(toggle)]
    orbit_enabled: bool,
    #[pane(toggle)]
    auto_rotate: bool,
    #[pane(slider, min = 2.0, max = 18.0, step = 0.1)]
    orbit_distance: f32,
    #[pane(slider, min = 0.0, max = 1.5, step = 0.01)]
    auto_rotate_speed: f32,
    #[pane(slider, min = 0.0, max = 2.0, step = 0.01)]
    hero_spin_speed: f32,
    #[pane(monitor)]
    active_mode_index: f32,
    #[pane(monitor)]
    orbit_yaw: f32,
    #[pane(monitor)]
    orbit_pitch: f32,
}

impl IntegrationPane {
    fn from_setup(
        orbit: &OrbitCamera,
        settings: &OrbitCameraSettings,
        hero_spin_speed: f32,
    ) -> Self {
        Self {
            orbit_enabled: false,
            auto_rotate: settings.auto_rotate.enabled,
            orbit_distance: orbit.target_distance,
            auto_rotate_speed: settings.auto_rotate.speed,
            hero_spin_speed,
            active_mode_index: 0.0,
            orbit_yaw: orbit.yaw,
            orbit_pitch: orbit.pitch,
        }
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins,
        CinematicCameraPlugin::always_on(Update),
        OrbitCameraPlugin::default(),
    ));
    common::install_common(&mut app, "Cinematic -> Orbit Handoff");
    app.register_pane::<IntegrationPane>();
    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
            spin_hero,
            sync_integration_pane,
            handle_sequence_finished,
            restart_intro.run_if(input_just_pressed(KeyCode::KeyR)),
            reflect_integration_pane,
            update_handoff_overlay,
        ),
    );
    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    common::spawn_demo_scene(&mut commands, &mut meshes, &mut materials);
    common::spawn_overlay(&mut commands);

    commands.spawn((
        Name::new("Hangar Fill"),
        PointLight {
            intensity: 28_000.0,
            range: 28.0,
            shadows_enabled: true,
            color: Color::srgb(0.88, 0.92, 1.0),
            ..default()
        },
        Transform::from_xyz(0.0, 9.0, 0.0),
    ));

    commands.spawn((
        Name::new("Hangar Rim"),
        Mesh3d(meshes.add(Torus::new(6.4, 0.18))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.46, 0.80),
            emissive: LinearRgba::from(Color::srgb(0.08, 0.18, 0.30)),
            metallic: 0.22,
            perceptual_roughness: 0.28,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.16, 0.0),
    ));

    let hero = commands
        .spawn((
            Name::new("Hero Walker"),
            HeroSpinner { speed: 0.42 },
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
        ))
        .id();

    commands.entity(hero).with_children(|parent| {
        parent.spawn((
            Name::new("Walker Torso"),
            Mesh3d(meshes.add(Cuboid::new(1.9, 2.5, 1.4))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.82, 0.28, 0.22),
                metallic: 0.28,
                perceptual_roughness: 0.34,
                ..default()
            })),
            Transform::from_xyz(0.0, 2.2, 0.0),
        ));
        parent.spawn((
            Name::new("Walker Reactor"),
            Mesh3d(meshes.add(Sphere::new(0.56).mesh().ico(5).expect("icosphere"))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.18, 0.84, 0.98),
                emissive: LinearRgba::from(Color::srgb(0.24, 0.82, 0.96)),
                metallic: 0.08,
                perceptual_roughness: 0.18,
                ..default()
            })),
            Transform::from_xyz(0.0, 2.25, -0.9),
        ));
        parent.spawn((
            Name::new("Walker Cockpit"),
            Mesh3d(meshes.add(Capsule3d::new(0.55, 0.9).mesh().rings(10).latitudes(14))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.14, 0.18, 0.22),
                metallic: 0.12,
                perceptual_roughness: 0.16,
                ..default()
            })),
            Transform {
                translation: Vec3::new(0.0, 3.55, 0.15),
                rotation: Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                ..default()
            },
        ));

        for (index, x) in [-0.7, 0.7].into_iter().enumerate() {
            parent.spawn((
                Name::new(format!("Walker Shoulder {}", index + 1)),
                Mesh3d(meshes.add(Cylinder::new(0.16, 1.8))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.78, 0.80, 0.84),
                    metallic: 0.24,
                    perceptual_roughness: 0.46,
                    ..default()
                })),
                Transform {
                    translation: Vec3::new(x, 2.7, 0.0),
                    rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                    ..default()
                },
            ));
        }

        for (index, x) in [-0.45, 0.45].into_iter().enumerate() {
            parent.spawn((
                Name::new(format!("Walker Leg {}", index + 1)),
                Mesh3d(meshes.add(Capsule3d::new(0.20, 1.8).mesh().rings(8).latitudes(10))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.66, 0.68, 0.74),
                    metallic: 0.14,
                    perceptual_roughness: 0.52,
                    ..default()
                })),
                Transform {
                    translation: Vec3::new(x, 0.95, 0.12),
                    rotation: Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                    ..default()
                },
            ));
        }
    });

    let cinematic_camera = commands
        .spawn((
            Name::new("Reveal Brain Camera"),
            CinematicCameraBrain,
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection {
                fov: 0.84,
                ..default()
            }),
            Transform::from_xyz(-8.0, 3.4, 13.0).looking_at(Vec3::new(0.0, 2.2, 0.0), Vec3::Y),
        ))
        .id();

    let orbit = OrbitCamera::looking_at(Vec3::new(0.0, 2.2, 0.0), Vec3::new(-5.0, 3.6, 8.8));
    let mut orbit_settings = OrbitCameraSettings::default();
    orbit_settings.auto_rotate.enabled = true;
    orbit_settings.auto_rotate.wait_seconds = 0.0;
    orbit_settings.auto_rotate.speed = 0.18;
    let orbit_camera = commands
        .spawn((
            Name::new("Orbit Viewer Camera"),
            Camera {
                is_active: false,
                ..default()
            },
            Camera3d::default(),
            Transform::from_xyz(-5.0, 3.6, 8.8).looking_at(Vec3::new(0.0, 2.2, 0.0), Vec3::Y),
            Projection::Perspective(PerspectiveProjection {
                fov: 0.74,
                ..default()
            }),
            OrbitCameraInputTarget,
            orbit.clone(),
            OrbitCameraSettings {
                enabled: false,
                ..orbit_settings.clone()
            },
        ))
        .id();

    let sequence = CinematicSequence {
        shots: vec![
            CinematicShot {
                name: "Arrival".into(),
                duration_secs: 2.2,
                position: PositionTrack::Rail(RailTrack {
                    rail: CinematicRail {
                        points: vec![
                            Vec3::new(-12.0, 5.6, 12.0),
                            Vec3::new(-6.0, 4.6, 10.0),
                            Vec3::new(1.0, 4.0, 9.5),
                        ],
                        kind: RailSplineKind::Linear,
                        ..default()
                    },
                    traversal: RailTraversal::default(),
                }),
                orientation: OrientationTrack::LookAt(LookAtTarget::Entity {
                    entity: hero,
                    offset: Vec3::new(0.0, 2.1, 0.0),
                    up: UpVectorMode::WorldY,
                    look_ahead_secs: 0.12,
                }),
                lens: LensTrack {
                    start_fov_y_radians: 0.92,
                    end_fov_y_radians: 0.72,
                    easing: CinematicEasing::CubicInOut,
                },
                ..default()
            },
            CinematicShot {
                name: "Hero Orbit".into(),
                duration_secs: 2.8,
                position: PositionTrack::Rail(RailTrack {
                    rail: CinematicRail {
                        closed: false,
                        points: vec![
                            Vec3::new(4.6, 2.9, 6.4),
                            Vec3::new(6.8, 2.8, 1.6),
                            Vec3::new(4.8, 2.7, -4.2),
                            Vec3::new(0.6, 2.9, -6.6),
                        ],
                        ..default()
                    },
                    traversal: RailTraversal::default(),
                }),
                orientation: OrientationTrack::LookAt(LookAtTarget::Entity {
                    entity: hero,
                    offset: Vec3::new(0.0, 2.1, 0.0),
                    up: UpVectorMode::WorldY,
                    look_ahead_secs: 0.18,
                }),
                lens: LensTrack {
                    start_fov_y_radians: 0.72,
                    end_fov_y_radians: 0.58,
                    easing: CinematicEasing::SmoothStep,
                },
                blend_in: CinematicBlend {
                    duration_secs: 0.7,
                    easing: CinematicEasing::SineInOut,
                },
                ..default()
            },
        ],
        entry_blend: CinematicBlend {
            duration_secs: 0.8,
            easing: CinematicEasing::CubicInOut,
        },
        exit_blend: CinematicBlend {
            duration_secs: 0.4,
            easing: CinematicEasing::SineInOut,
        },
        ..default()
    };

    let rig = commands
        .spawn((
            Name::new("Reveal Virtual Camera"),
            common::DemoRig,
            CinematicVirtualCamera {
                brain: cinematic_camera,
                priority: 20,
                live: true,
                auto_play: true,
                ..default()
            },
            CinematicPlayback::default(),
            sequence.clone(),
        ))
        .id();

    commands.insert_resource(HandoffState {
        rig,
        cinematic_camera,
        orbit_camera,
        active_mode: ActiveCameraMode::Cinematic,
    });
    common::queue_example_pane(
        &mut commands,
        common::ExampleCinematicPane::from_setup(
            true,
            true,
            1.0,
            sequence.entry_blend.duration_secs,
            sequence.exit_blend.duration_secs,
            &CinematicCameraDebugSettings {
                enabled: true,
                ..default()
            },
        ),
    );
    commands.insert_resource(IntegrationPane::from_setup(&orbit, &orbit_settings, 0.42));

    commands.spawn((
        Name::new("Handoff Overlay"),
        HandoffOverlay,
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(20.0),
            bottom: Val::Px(20.0),
            width: Val::Px(430.0),
            padding: UiRect::all(Val::Px(14.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.03, 0.05, 0.08, 0.84)),
        Text::new(""),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

fn spin_hero(time: Res<Time>, mut heroes: Query<(&HeroSpinner, &mut Transform)>) {
    for (spinner, mut transform) in &mut heroes {
        transform.rotate_y(spinner.speed * time.delta_secs());
    }
}

fn sync_integration_pane(
    pane: Res<IntegrationPane>,
    state: Res<HandoffState>,
    mut heroes: Query<&mut HeroSpinner>,
    mut orbit_cameras: Query<(&mut OrbitCamera, &mut OrbitCameraSettings, &mut Camera)>,
) {
    if !pane.is_changed() {
        return;
    }

    for mut spinner in &mut heroes {
        spinner.speed = pane.hero_spin_speed;
    }

    let Ok((mut orbit, mut settings, mut camera)) = orbit_cameras.get_mut(state.orbit_camera)
    else {
        return;
    };

    orbit.target_distance = pane.orbit_distance.max(2.0);
    settings.auto_rotate.enabled = pane.auto_rotate;
    settings.auto_rotate.speed = pane.auto_rotate_speed.max(0.0);

    if state.active_mode == ActiveCameraMode::Orbit {
        camera.is_active = pane.orbit_enabled;
        settings.enabled = pane.orbit_enabled;
    } else {
        camera.is_active = false;
        settings.enabled = false;
    }
}

fn handle_sequence_finished(
    mut finished: MessageReader<SequenceFinished>,
    mut state: ResMut<HandoffState>,
    mut cameras: Query<&mut Camera>,
    mut orbit_cameras: Query<(&mut OrbitCamera, &mut OrbitCameraSettings)>,
    mut pane: ResMut<IntegrationPane>,
) {
    let sequence_finished = finished.read().any(|event| event.rig == state.rig);
    if !sequence_finished || state.active_mode == ActiveCameraMode::Orbit {
        return;
    }

    if let Ok(mut camera) = cameras.get_mut(state.cinematic_camera) {
        camera.is_active = false;
    }
    if let Ok(mut camera) = cameras.get_mut(state.orbit_camera) {
        camera.is_active = pane.orbit_enabled;
    }
    if let Ok((mut orbit, mut settings)) = orbit_cameras.get_mut(state.orbit_camera) {
        orbit.snap_to_target();
        settings.enabled = pane.orbit_enabled;
    }

    state.active_mode = ActiveCameraMode::Orbit;
    pane.active_mode_index = 1.0;
}

fn restart_intro(
    mut state: ResMut<HandoffState>,
    mut playback: MessageWriter<CinematicPlaybackCommand>,
    mut cameras: Query<&mut Camera>,
    mut orbit_settings: Query<&mut OrbitCameraSettings>,
    mut pane: ResMut<IntegrationPane>,
) {
    if let Ok(mut camera) = cameras.get_mut(state.cinematic_camera) {
        camera.is_active = true;
    }
    if let Ok(mut camera) = cameras.get_mut(state.orbit_camera) {
        camera.is_active = false;
    }
    if let Ok(mut settings) = orbit_settings.get_mut(state.orbit_camera) {
        settings.enabled = false;
    }

    state.active_mode = ActiveCameraMode::Cinematic;
    pane.active_mode_index = 0.0;
    playback.write(CinematicPlaybackCommand::Restart(state.rig));
}

fn reflect_integration_pane(
    state: Res<HandoffState>,
    orbit_cameras: Query<&OrbitCamera>,
    mut pane: ResMut<IntegrationPane>,
) {
    let Ok(orbit) = orbit_cameras.get(state.orbit_camera) else {
        return;
    };

    pane.active_mode_index = match state.active_mode {
        ActiveCameraMode::Cinematic => 0.0,
        ActiveCameraMode::Orbit => 1.0,
    };
    pane.orbit_yaw = orbit.yaw;
    pane.orbit_pitch = orbit.pitch;
}

fn update_handoff_overlay(
    state: Res<HandoffState>,
    orbit_cameras: Query<&OrbitCamera>,
    mut overlays: Query<&mut Text, With<HandoffOverlay>>,
) {
    let Ok(mut text) = overlays.single_mut() else {
        return;
    };
    let Ok(orbit) = orbit_cameras.get(state.orbit_camera) else {
        return;
    };

    let mode = match state.active_mode {
        ActiveCameraMode::Cinematic => "Cinematic intro",
        ActiveCameraMode::Orbit => "Orbit viewer",
    };

    *text = Text::new(format!(
        "Cinematic -> Orbit Handoff\nmode: {mode}\nR restarts the intro after the handoff.\norbit focus {:.2?}\ndistance {:.2} yaw {:.2} pitch {:.2}\nUse the top-right pane for cinematic blends and the bottom-right pane for orbit tuning.",
        orbit.focus, orbit.distance, orbit.yaw, orbit.pitch,
    ));
}

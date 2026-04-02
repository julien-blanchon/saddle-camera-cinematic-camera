use bevy::prelude::*;
use saddle_camera_cinematic_camera::{
    CinematicCameraDebugSettings, CinematicCameraDiagnostics, CinematicCameraState,
    CinematicPlayback, CinematicTargetGroup, UpVectorMode, WeightedTarget,
};

#[derive(Component)]
pub struct DemoRig;

#[derive(Component)]
pub struct DemoOverlay;

#[derive(Component, Clone, Copy)]
pub struct OrbitTarget {
    pub center: Vec3,
    pub radius: f32,
    pub height: f32,
    pub speed: f32,
    pub phase: f32,
}

#[derive(Component)]
pub struct BobbingProp {
    pub base_height: f32,
    pub amplitude: f32,
    pub speed: f32,
    pub phase: f32,
}

#[derive(Resource)]
pub struct OverlayTitle(pub &'static str);

pub fn install_common(app: &mut App, title: &'static str) {
    app.insert_resource(OverlayTitle(title));
    app.insert_resource(CinematicCameraDebugSettings {
        enabled: true,
        ..default()
    });
    app.add_systems(Update, (animate_orbits, animate_bobbing, update_overlay));
}

pub fn spawn_demo_scene(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.spawn((
        Name::new("Scene Sun"),
        DirectionalLight {
            illuminance: 22_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(10.0, 18.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Name::new("Scene Ground"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(70.0, 70.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.08, 0.10, 0.12),
            perceptual_roughness: 1.0,
            ..default()
        })),
    ));

    let accents = [
        (Vec3::new(-8.0, 1.0, -8.0), Color::srgb(0.24, 0.45, 0.78)),
        (Vec3::new(0.0, 1.5, 2.0), Color::srgb(0.72, 0.35, 0.24)),
        (Vec3::new(6.0, 2.0, -4.0), Color::srgb(0.24, 0.62, 0.50)),
        (Vec3::new(10.0, 0.8, 8.0), Color::srgb(0.65, 0.58, 0.20)),
    ];

    for (index, (translation, color)) in accents.into_iter().enumerate() {
        commands.spawn((
            Name::new(format!("Scene Tower {}", index + 1)),
            BobbingProp {
                base_height: translation.y,
                amplitude: 0.2 + index as f32 * 0.06,
                speed: 0.5 + index as f32 * 0.15,
                phase: index as f32,
            },
            Mesh3d(meshes.add(Cuboid::new(
                2.2 + index as f32 * 0.4,
                2.0 + index as f32 * 0.8,
                2.2 + index as f32 * 0.2,
            ))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.85,
                ..default()
            })),
            Transform::from_translation(translation),
        ));
    }
}

pub fn spawn_overlay(commands: &mut Commands) {
    commands.spawn((
        Name::new("Cinematic Camera Overlay"),
        DemoOverlay,
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

pub fn spawn_camera(commands: &mut Commands, transform: Transform) -> Entity {
    commands
        .spawn((
            Name::new("Demo Camera"),
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection {
                fov: std::f32::consts::FRAC_PI_4,
                ..default()
            }),
            transform,
        ))
        .id()
}

pub fn spawn_orbit_target(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    name: &str,
    color: Color,
    orbit: OrbitTarget,
) -> Entity {
    commands
        .spawn((
            Name::new(name.to_string()),
            orbit,
            Mesh3d(meshes.add(Sphere::new(0.7).mesh().uv(24, 16))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                metallic: 0.05,
                perceptual_roughness: 0.35,
                ..default()
            })),
            Transform::from_xyz(orbit.center.x + orbit.radius, orbit.height, orbit.center.z),
        ))
        .id()
}

pub fn spawn_target_group(
    commands: &mut Commands,
    name: &str,
    members: Vec<WeightedTarget>,
    fallback_point: Vec3,
) -> Entity {
    commands
        .spawn((
            Name::new(name.to_string()),
            CinematicTargetGroup {
                members,
                fallback_point,
                up: UpVectorMode::WorldY,
                look_ahead_secs: 0.25,
            },
        ))
        .id()
}

fn animate_orbits(time: Res<Time>, mut query: Query<(&OrbitTarget, &mut Transform)>) {
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

fn animate_bobbing(time: Res<Time>, mut query: Query<(&BobbingProp, &mut Transform)>) {
    for (prop, mut transform) in &mut query {
        transform.translation.y = prop.base_height
            + (time.elapsed_secs() * prop.speed + prop.phase).sin() * prop.amplitude;
    }
}

fn update_overlay(
    title: Res<OverlayTitle>,
    diagnostics: Res<CinematicCameraDiagnostics>,
    rig_query: Query<(&Name, &CinematicPlayback, &CinematicCameraState), With<DemoRig>>,
    mut overlay: Query<&mut Text, With<DemoOverlay>>,
) {
    let Ok(mut text) = overlay.single_mut() else {
        return;
    };

    let body = rig_query
        .iter()
        .next()
        .map(|(name, playback, state)| {
            format!(
                "{}\nrig: {}\nstatus: {:?}\nsequence time: {:.2}s\ncurrent shot: {:?}\nblend: {:?} -> {:?} ({:.2})\nactive rigs: {}\napplied cameras: {}\ntarget cache: {}",
                title.0,
                name.as_str(),
                playback.status,
                state.sequence_time_secs,
                state.current_shot,
                state.blend_from_shot,
                state.blend_to_shot,
                state.blend_alpha,
                diagnostics.active_rigs,
                diagnostics.applied_cameras,
                diagnostics.target_history_entries,
            )
        })
        .unwrap_or_else(|| title.0.to_string());

    *text = Text::new(body);
}

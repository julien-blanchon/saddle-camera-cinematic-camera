use bevy::prelude::*;
use bevy_flair::prelude::InlineStyle;
use saddle_camera_cinematic_camera::{
    CinematicCameraDebugSettings, CinematicCameraDiagnostics, CinematicCameraRig,
    CinematicCameraState, CinematicCameraSystems, CinematicPlayback, CinematicSequence,
    CinematicTargetGroup, CinematicVirtualCamera, UpVectorMode, WeightedTarget,
};
use saddle_pane::prelude::*;

const PANE_DARK_THEME_VARS: &[(&str, &str)] = &[
    ("--pane-elevation-1", "#28292e"),
    ("--pane-elevation-2", "#222327"),
    ("--pane-elevation-3", "rgba(187, 188, 196, 0.10)"),
    ("--pane-border", "#3c3d44"),
    ("--pane-border-focus", "#7090b0"),
    ("--pane-border-subtle", "#333438"),
    ("--pane-text-primary", "#bbbcc4"),
    ("--pane-text-secondary", "#78797f"),
    ("--pane-text-muted", "#5c5d64"),
    ("--pane-text-on-accent", "#ffffff"),
    ("--pane-text-brighter", "#d0d1d8"),
    ("--pane-text-monitor", "#9a9ba2"),
    ("--pane-text-log", "#8a8b92"),
    ("--pane-accent", "#4a6fa5"),
    ("--pane-accent-hover", "#5a8fd5"),
    ("--pane-accent-active", "#3a5f95"),
    ("--pane-accent-subtle", "rgba(74, 111, 165, 0.15)"),
    ("--pane-accent-fill", "rgba(74, 111, 165, 0.60)"),
    ("--pane-accent-fill-hover", "rgba(90, 143, 213, 0.70)"),
    ("--pane-accent-fill-active", "rgba(90, 143, 213, 0.80)"),
    ("--pane-accent-checked", "rgba(74, 111, 165, 0.25)"),
    ("--pane-accent-checked-hover", "rgba(74, 111, 165, 0.35)"),
    ("--pane-accent-indicator", "rgba(74, 111, 165, 0.80)"),
    ("--pane-accent-knob", "#7aacdf"),
    ("--pane-widget-bg", "rgba(187, 188, 196, 0.10)"),
    ("--pane-widget-hover", "rgba(187, 188, 196, 0.15)"),
    ("--pane-widget-focus", "rgba(187, 188, 196, 0.20)"),
    ("--pane-widget-active", "rgba(187, 188, 196, 0.25)"),
    ("--pane-widget-bg-muted", "rgba(187, 188, 196, 0.06)"),
    ("--pane-tab-hover-bg", "rgba(187, 188, 196, 0.06)"),
    ("--pane-hover-bg", "rgba(255, 255, 255, 0.03)"),
    ("--pane-active-bg", "rgba(255, 255, 255, 0.05)"),
    ("--pane-popup-bg", "#1e1f24"),
    ("--pane-bg-dark", "rgba(0, 0, 0, 0.25)"),
];

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

#[derive(Resource, Debug, Clone, Copy, PartialEq, Pane)]
#[pane(title = "Cinematic Camera", position = "top-right")]
pub struct ExampleCinematicPane {
    #[pane(toggle)]
    pub rig_enabled: bool,
    #[pane(toggle)]
    pub auto_play: bool,
    #[pane(toggle)]
    pub debug_gizmos: bool,
    #[pane(toggle)]
    pub draw_paths: bool,
    #[pane(toggle)]
    pub draw_targets: bool,
    #[pane(slider, min = 0.1, max = 3.0, step = 0.05)]
    pub playback_speed: f32,
    #[pane(slider, min = 0.0, max = 3.0, step = 0.05)]
    pub entry_blend_secs: f32,
    #[pane(slider, min = 0.0, max = 3.0, step = 0.05)]
    pub exit_blend_secs: f32,
    #[pane(monitor)]
    pub active_rigs: f32,
    #[pane(monitor)]
    pub applied_cameras: f32,
    #[pane(monitor)]
    pub sequence_time_secs: f32,
}

impl Default for ExampleCinematicPane {
    fn default() -> Self {
        Self {
            rig_enabled: true,
            auto_play: true,
            debug_gizmos: true,
            draw_paths: true,
            draw_targets: true,
            playback_speed: 1.0,
            entry_blend_secs: 0.8,
            exit_blend_secs: 0.8,
            active_rigs: 0.0,
            applied_cameras: 0.0,
            sequence_time_secs: 0.0,
        }
    }
}

impl ExampleCinematicPane {
    pub fn from_setup(
        rig_enabled: bool,
        auto_play: bool,
        playback_speed: f32,
        entry_blend_secs: f32,
        exit_blend_secs: f32,
        debug: &CinematicCameraDebugSettings,
    ) -> Self {
        Self {
            rig_enabled,
            auto_play,
            debug_gizmos: debug.enabled,
            draw_paths: debug.draw_paths,
            draw_targets: debug.draw_targets,
            playback_speed,
            entry_blend_secs,
            exit_blend_secs,
            active_rigs: 0.0,
            applied_cameras: 0.0,
            sequence_time_secs: 0.0,
        }
    }
}

#[derive(Resource, Clone, Copy)]
struct ExampleCinematicPaneBootstrap(ExampleCinematicPane);

pub fn queue_example_pane(commands: &mut Commands, pane: ExampleCinematicPane) {
    commands.insert_resource(ExampleCinematicPaneBootstrap(pane));
}

pub fn install_common(app: &mut App, title: &'static str) {
    app.insert_resource(OverlayTitle(title));
    app.insert_resource(CinematicCameraDebugSettings {
        enabled: true,
        ..default()
    });
    app.add_plugins((
        bevy_flair::FlairPlugin,
        bevy_input_focus::InputDispatchPlugin,
        bevy_ui_widgets::UiWidgetsPlugins,
        bevy_input_focus::tab_navigation::TabNavigationPlugin,
        PanePlugin,
    ))
    .register_pane::<ExampleCinematicPane>()
    .add_systems(
        PreUpdate,
        (
            prime_pane_theme_vars,
            apply_bootstrapped_pane,
            sync_example_pane,
        )
            .chain(),
    )
    .add_systems(
        Update,
        (
            animate_orbits,
            animate_bobbing,
            reflect_runtime_into_pane.after(CinematicCameraSystems::ApplyCamera),
            update_overlay,
        ),
    );
}

fn prime_pane_theme_vars(mut panes: Query<&mut InlineStyle, Added<PaneRoot>>) {
    for mut style in &mut panes {
        for &(key, value) in PANE_DARK_THEME_VARS {
            style.set(key, value.to_owned());
        }
    }
}

fn apply_bootstrapped_pane(
    bootstrap: Option<Res<ExampleCinematicPaneBootstrap>>,
    mut pane: ResMut<ExampleCinematicPane>,
) {
    let Some(bootstrap) = bootstrap else {
        return;
    };

    if *pane == ExampleCinematicPane::default() {
        *pane = bootstrap.0;
    }
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
                look_ahead_secs: 0.12,
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
                "{}\n\nrig: {}\nstatus: {:?}\nsequence time: {:.2}s\ncurrent shot: {:?}\nblend: {:?} -> {:?} ({:.2})\nactive rigs: {}\napplied cameras: {}\ntarget cache: {}\n\n[Pane: top-right] Adjust playback speed, blends, debug viz",
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

fn sync_example_pane(
    mut pane: ResMut<ExampleCinematicPane>,
    bootstrap: Option<Res<ExampleCinematicPaneBootstrap>>,
    mut debug: ResMut<CinematicCameraDebugSettings>,
    mut rigs: Query<
        (
            &mut CinematicCameraRig,
            &mut CinematicPlayback,
            &mut CinematicSequence,
        ),
        Or<(With<DemoRig>, With<CinematicVirtualCamera>)>,
    >,
) {
    let has_bootstrap = bootstrap.is_some();
    if let Some(bootstrap) = bootstrap {
        if *pane == ExampleCinematicPane::default() && bootstrap.0 != *pane {
            *pane = bootstrap.0;
        }
    }

    for (mut rig, mut playback, mut sequence) in &mut rigs {
        let scene_pane = ExampleCinematicPane::from_setup(
            rig.enabled,
            rig.auto_play,
            playback.speed,
            sequence.entry_blend.duration_secs,
            sequence.exit_blend.duration_secs,
            &debug,
        );
        if !has_bootstrap && *pane == ExampleCinematicPane::default() && scene_pane != *pane {
            *pane = scene_pane;
            return;
        }

        debug.enabled = pane.debug_gizmos;
        debug.draw_paths = pane.draw_paths;
        debug.draw_targets = pane.draw_targets;
        rig.enabled = pane.rig_enabled;
        rig.auto_play = pane.auto_play;
        playback.speed = pane.playback_speed.max(0.05);
        sequence.entry_blend.duration_secs = pane.entry_blend_secs.max(0.0);
        sequence.exit_blend.duration_secs = pane.exit_blend_secs.max(0.0);
    }
}

fn reflect_runtime_into_pane(
    diagnostics: Res<CinematicCameraDiagnostics>,
    states: Query<&CinematicCameraState, With<DemoRig>>,
    mut pane: ResMut<ExampleCinematicPane>,
) {
    pane.active_rigs = diagnostics.active_rigs as f32;
    pane.applied_cameras = diagnostics.applied_cameras as f32;
    pane.sequence_time_secs = states
        .iter()
        .next()
        .map(|state| state.sequence_time_secs)
        .unwrap_or(0.0);
}

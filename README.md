# Saddle Camera Cinematic Camera

Reusable cinematic camera toolkit for Bevy: authored rails, weighted look targets, shot sequencing, per-shot blends, and clean gameplay-camera handoff.

The runtime now also exposes an explicit `CinematicVirtualCamera` authoring component that syncs into the underlying rig/binding system, giving downstream games a clearer “virtual camera + brain” vocabulary on top of the existing solver.

The crate stays project-agnostic. It does not depend on `game_core`, `Screen`, `GameSet`, or any game-specific vocabulary. Consumers wire it into their own schedules and bind it to any Bevy camera entity they already own.

For always-on examples, tools, or sandboxes, `CinematicCameraPlugin::always_on(Update)` is the simplest entrypoint. For real games, prefer `CinematicCameraPlugin::new(...)` so activation and teardown stay aligned with your own state flow.

## Quick Start

```toml
[dependencies]
bevy = "0.18"
saddle-camera-cinematic-camera = { git = "https://github.com/julien-blanchon/saddle-camera-cinematic-camera" }
```

```rust
use bevy::prelude::*;
use saddle_camera_cinematic_camera::*;

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DemoState {
    #[default]
    Gameplay,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<DemoState>()
        .add_plugins(CinematicCameraPlugin::new(
            OnEnter(DemoState::Gameplay),
            OnExit(DemoState::Gameplay),
            Update,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    let camera = commands
        .spawn((
            Name::new("Gameplay Camera"),
            Camera3d::default(),
            Transform::from_xyz(-4.0, 2.5, 12.0).looking_at(Vec3::ZERO, Vec3::Y),
        ))
        .id();

    let mut push_in = CinematicShot::rail(
        "Push In",
        3.0,
        CinematicRail {
            points: vec![
                Vec3::new(-4.0, 2.5, 10.0),
                Vec3::new(-1.0, 2.2, 7.5),
                Vec3::new(3.0, 2.0, 4.0),
                Vec3::new(6.0, 2.0, 3.0),
            ],
            kind: RailSplineKind::Linear,
            ..default()
        },
    );
    push_in.orientation = OrientationTrack::LookAt(LookAtTarget::Point {
        point: Vec3::new(0.0, 1.0, 0.0),
        up: UpVectorMode::WorldY,
    });

    commands.spawn((
        Name::new("Boss Reveal Rig"),
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
            shots: vec![push_in],
            restore_camera_on_finish: true,
            entry_blend: CinematicBlend {
                duration_secs: 0.8,
                easing: CinematicEasing::CubicInOut,
            },
            exit_blend: CinematicBlend {
                duration_secs: 0.8,
                easing: CinematicEasing::SineInOut,
            },
            ..default()
        },
    ));
}
```

## Public API

| Type | Purpose |
| --- | --- |
| `CinematicCameraPlugin` | Registers the cinematic runtime with injectable activate, deactivate, and update schedules |
| `CinematicCameraSystems` | Public ordering hooks: `InputOrCommands`, `AdvanceTimeline`, `SolveRig`, `ApplyCamera`, `Debug` |
| `CinematicVirtualCamera` | Authoring-facing virtual-camera surface that syncs into rig/binding data |
| `CinematicCameraBrain` | Optional marker for a gameplay camera that receives virtual-camera output |
| `CinematicCameraRig` | Per-rig runtime toggle and optional autoplay flag |
| `CinematicCameraBinding` | Binds a rig to a concrete Bevy camera entity and controls transform / projection writeback |
| `CinematicSequence` | Sequence of authored `CinematicShot`s, plus entry/exit blends and sequence loop policy |
| `CinematicShot` | Per-shot duration, rail or fixed position, orientation mode, blend-in, markers, shake, and lens track |
| `CinematicRail` / `CinematicRailCache` | Curve authoring and cache-backed arc-length sampling helpers |
| `CinematicTargetGroup` | Weighted multi-target framing by centroid |
| `CinematicPlayback` | Inspectable runtime playback state (`Stopped`, `Playing`, `Paused`, `Exiting`) |
| `CinematicCameraState` | Solved camera output: transform, look target, FOV, current shot, and blend summary |
| `CinematicDrivenCamera` | Ownership marker written to the bound camera while the crate drives it |
| Messages | `CinematicPlaybackCommand`, `ShotStarted`, `ShotFinished`, `ShotMarkerReached`, `SequenceFinished`, `CinematicBlendCompleted` |
| Resources | `CinematicCameraDebugSettings`, `CinematicCameraDiagnostics` |

## Current Feature Scope

Supported in v0.1:

- cache-backed rail sampling with normalized or world-distance traversal
- open and closed rails with `Once`, `Loop`, and `PingPong` traversal
- fixed, tangent-facing, point-look, entity-look, and weighted target-group orientation
- per-shot lens interpolation and additive deterministic handheld shake
- overlapped shot blends plus gameplay-camera blend-in / blend-out
- message-driven playback commands (`Play`, `Pause`, `Resume`, `Restart`, `Stop`, `Seek`)
- runtime solved-state components and opt-in gizmo debugging via `CinematicCameraDebugSettings`

Intentionally minimal in v0.1:

- editor tooling and asset import pipelines
- collision / occlusion avoidance
- automatic dolly distance solve for group framing
- orthographic lens blending
- reverse-play lifecycle messages during ping-pong playback

## Pipeline

The runtime is staged and orderable:

1. `InputOrCommands`
2. `AdvanceTimeline`
3. `SolveRig`
4. `ApplyCamera`
5. `Debug`

Sequence data is cached on `Changed<CinematicSequence>`. The solver operates on the cache and writes a `CinematicCameraState` component before any camera mutation happens. `ApplyCamera` is the only stage that writes `Transform` and `Projection` on the bound camera entity.

## Examples

| Example | Purpose | Run |
| --- | --- | --- |
| `basic` | Minimal looped flythrough on a closed rail | `cargo run -p saddle-camera-cinematic-camera-example-basic` |
| `blend_between_shots` | Gameplay-camera handoff, shot-to-shot blends, and clean return | `cargo run -p saddle-camera-cinematic-camera-example-blend-between-shots` |
| `cinematic_orbit_handoff` | Cross-crate demo: cinematic intro that hands off into an orbit-camera model viewer | `cargo run -p saddle-camera-cinematic-camera-example-cinematic-orbit-handoff` |
| `moving_target` | Rail-driven camera that tracks a moving entity with look-ahead | `cargo run -p saddle-camera-cinematic-camera-example-moving-target` |
| `target_group` | Weighted target-group framing over two moving subjects | `cargo run -p saddle-camera-cinematic-camera-example-target-group` |
| `handheld_rail` | Rail motion with deterministic additive handheld shake | `cargo run -p saddle-camera-cinematic-camera-example-handheld-rail` |
| `stress_preview` | One active rig plus 100 passive preview rigs for perf smoke | `cargo run -p saddle-camera-cinematic-camera-example-stress-preview` |
| `virtual_camera_brain` | Two authored virtual cameras hand off through a shared brain camera | `cargo run -p saddle-camera-cinematic-camera-example-virtual-camera-brain` |

All showcase examples now include a `saddle-pane` panel for live tuning of playback speed, rig enablement, blend durations, and debug draw toggles.

## Workspace Lab

The standalone examples verify the crate in isolation. The workspace also includes a crate-local lab app at
`shared/camera/saddle-camera-cinematic-camera/examples/lab`:

```bash
cargo run -p saddle-camera-cinematic-camera-lab
```

## More Docs

- [Architecture](docs/architecture.md)
- [Configuration](docs/configuration.md)

To preview rails and look targets in examples or tools, opt into debug gizmos explicitly:

```rust
app.insert_resource(CinematicCameraDebugSettings {
    enabled: true,
    ..default()
});
```

## Known Limitations

- FOV blending is only applied to perspective cameras. Orthographic bindings keep their existing projection.
- `CinematicBlendCompleted` is authored for forward-running shot transitions and gameplay handoff; reverse ping-pong transitions do not emit mirror-image lifecycle events yet.
- Target groups solve a weighted centroid only. They do not yet auto-adjust distance or FOV to keep bounds tight.

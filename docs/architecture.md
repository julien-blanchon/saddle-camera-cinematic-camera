# Architecture

`saddle-camera-cinematic-camera` is split into two halves:

1. Pure math helpers for rail sampling, easing, orientation, and pose blending.
2. ECS orchestration that owns playback state, world lookups, and the final writeback to Bevy camera entities.

That split keeps the crate reusable and testable: path math and look solving can be unit-tested without a Bevy `App`, while the world-facing parts stay explicit and easy to inspect over BRP.

## Data Flow

```text
Messages / component edits
        |
        v
InputOrCommands
  - sync `CinematicVirtualCamera` authoring into rig/binding data
  - ensure runtime components exist
  - rebuild sequence caches on Changed<CinematicSequence>
  - precompute timeline lifecycle events on Changed<CinematicSequence>
  - refresh target motion history
  - apply playback commands / autoplay
        |
        v
AdvanceTimeline
  - advance visible sequence time
  - handle loop / ping-pong / finish
  - emit shot lifecycle and marker messages
        |
        v
SolveRig
  - resolve current shot or overlap blend window
  - sample rails through the arc-length cache
  - solve look direction / target groups / shake
  - blend against gameplay snapshot for entry / exit
  - apply per-rig output damping (CinematicOutputDamping)
  - publish `CinematicCameraState`
        |
        v
CollisionAvoidance
  - ensure internal collision state on rigs with CinematicCollisionAvoidance
  - raycast from look target to solved camera position
  - asymmetric smooth retract/recover of camera distance on collision
  - modify CinematicCameraState position in-place
        |
        v
ApplyCamera
  - choose a winning rig per bound camera by priority
  - write `Transform`
  - optionally write perspective FOV
  - mark the camera with `CinematicDrivenCamera`
        |
        v
Debug
  - draw optional gizmos from the published solved state
```

## Sequence Model

Each `CinematicSequence` is an ordered list of `CinematicShot`s. Each shot has a `duration_secs` and may declare a `blend_in`.

The runtime precomputes:

- `shot_starts`: visible start time for every shot
- `total_duration`: final visible duration after subtracting overlap windows
- one `CinematicRailCache` per rail-backed shot
- sorted marker times per shot

Blend windows are overlap windows, not frozen hold transitions. If shot `B` has a `blend_in`, the runtime starts `B` early by that overlap duration and solves both `A` and `B` during the shared interval.

## Why Arc-Length Sampling

The public API exposes:

- normalized traversal (`0.0 ..= 1.0`)
- world-distance traversal (meters along the rail)

Both are routed through `CinematicRailCache`, which stores sampled cumulative distances. That means normalized travel is normalized arc length, not raw spline domain. Constant-speed travel is therefore meaningful even on non-uniform Catmull-Rom segments.

## Orientation Strategy

The crate currently supports three orientation sources:

- fixed quaternion
- path tangent
- look target

Look targets can be:

- a world-space point
- an entity with optional offset and look-ahead
- a weighted target-group entity

The solver always normalizes or replaces unstable up vectors before using `Transform::looking_to`. If the forward vector degenerates, the previous rotation is preserved instead of snapping to an arbitrary new frame.

## Gameplay Handoff

When playback begins, the binding may capture the current gameplay-owned camera transform and perspective FOV. That snapshot becomes the entry-blend source and the optional exit-blend target.

The runtime therefore never needs to “own the whole camera stack”. It only needs:

- one camera entity reference
- the current gameplay state at playback start
- its own solved pose

`CinematicDrivenCamera` on the target camera makes ownership explicit for UI, debug tools, BRP queries, and higher-level orchestration.

`CinematicVirtualCamera` is a thin authoring layer on top of that runtime model. It syncs a more Cinemachine-like “virtual camera targets a brain camera” vocabulary into the existing `CinematicCameraRig` + `CinematicCameraBinding` components, so gameplay code can toggle `live` or `solo` without bypassing the tested playback path.

## Debug Surface

The crate intentionally exposes:

- `CinematicPlayback`
- `CinematicCameraState`
- `CinematicDrivenCamera`
- `CinematicCameraDiagnostics`
- `CinematicCameraDebugSettings`

Those types are small, reflectable, and BRP-friendly. The debug system reads only those surfaces and the rail caches; it does not need any privileged internal world access.
Debug drawing is opt-in through `CinematicCameraDebugSettings.enabled`, so runtime consumers are not forced into gizmo output unless they request it.

## Output Damping

`CinematicOutputDamping` applies framerate-independent exponential smoothing to the solved pose before it is published as `CinematicCameraState`. The smoothing uses `factor = 1.0 - exp(-rate * dt)` which produces consistent visual results regardless of frame rate.

Two separate rates control position and rotation independently. Higher rates track the raw solved pose more tightly; lower rates produce heavier, more cinematic smoothing. Presets (`light`, `default`, `heavy`) cover the common cases.

Damping runs inside `SolveRig`, after the raw pose is computed and before the state component is written.

## Collision Avoidance

The optional `CinematicCollisionAvoidance` component enables mesh-based collision avoidance on a per-rig basis. The system runs in the `CollisionAvoidance` set, between `SolveRig` and `ApplyCamera`.

When enabled, the system casts a ray from the solved look target to the solved camera position using Bevy's `MeshRayCast`. If the ray hits geometry before reaching the camera, the camera is pushed closer along the target-to-camera axis. Retraction is fast (configurable `retract_rate`) while recovery is slower (`recover_rate`), matching the Unreal Engine Spring Arm pattern of "dodge fast, recover slowly" for a stable feel.

The system requires `MeshPickingPlugin` to be present in the app. If the plugin is not added, collision avoidance is silently skipped.

## Target Velocity Smoothing

Entity look-ahead (`look_ahead_secs`) relies on velocity estimation stored in `TargetHistory`. Raw frame-to-frame velocity is noisy, especially at high frame rates. The runtime applies exponential smoothing to the velocity estimate using the same `factor = 1.0 - exp(-rate * dt)` pattern, producing stable look-ahead offsets even when the target changes speed or direction abruptly.

## Current Tradeoffs

- The crate optimizes for authored sequences and reliable handoff, not editor authoring.
- Orthographic cameras are readable as bindings, but only perspective FOV is blended in v0.1.
- Target groups solve only a centroid. Dynamic auto-zoom and bound-fitting can be added later without changing the current shot / binding model.
- Collision avoidance requires `MeshPickingPlugin` and only raycasts against mesh geometry, not primitive colliders.

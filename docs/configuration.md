# Configuration

This is the tuning reference for `saddle-camera-cinematic-camera`.

## `CinematicCameraRig`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `auto_play` | `bool` | `false` | Starts playback automatically the first time the runtime sees the rig while active. |
| `enabled` | `bool` | `true` | Master per-rig toggle. Disabled rigs keep their data but no longer drive cameras. |

## `CinematicVirtualCamera`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `brain` | `Entity` | `Entity::PLACEHOLDER` | Camera entity that should receive this virtual camera's solved output. |
| `priority` | `i32` | `0` | Higher priority wins when several virtual cameras target the same brain. |
| `live` | `bool` | `true` | Mirrors into `CinematicCameraRig.enabled`. Toggle this from gameplay state logic. |
| `solo` | `bool` | `false` | Temporarily boosts priority so one virtual camera wins over its siblings. |
| `auto_play` | `bool` | `false` | Mirrors into `CinematicCameraRig.auto_play`. |
| `capture_gameplay_state` | `bool` | `true` | Mirrors into `CinematicCameraBinding.capture_gameplay_state`. |
| `apply_transform` | `bool` | `true` | Mirrors into `CinematicCameraBinding.apply_transform`. |
| `apply_projection` | `bool` | `true` | Mirrors into `CinematicCameraBinding.apply_projection`. |

Attach `CinematicVirtualCamera` plus `CinematicSequence` to the same entity when you want an explicit virtual-camera authoring surface. The runtime will populate or refresh `CinematicCameraRig` and `CinematicCameraBinding` for you.

## `CinematicCameraBinding`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `camera` | `Entity` | `Entity::PLACEHOLDER` | Bevy camera entity to drive. Must point at a real camera before playback starts. |
| `priority` | `i32` | `0` | Higher priority wins if multiple rigs target the same camera. |
| `capture_gameplay_state` | `bool` | `true` | Captures the camera's current transform and perspective FOV each time playback starts from `Stopped`. |
| `apply_transform` | `bool` | `true` | Controls whether `ApplyCamera` writes `Transform`. |
| `apply_projection` | `bool` | `true` | Controls whether `ApplyCamera` writes perspective FOV. Non-perspective projections are left untouched. |

## `CinematicSequence`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `shots` | `Vec<CinematicShot>` | `[]` | Ordered shot list. Empty sequences never drive cameras. |
| `loop_mode` | `PlaybackLoopMode` | `Once` | Sequence-level playback policy. |
| `restore_camera_on_finish` | `bool` | `false` | If true and a snapshot exists, finishing the sequence enters an exit blend back to gameplay. |
| `entry_blend` | `CinematicBlend` | instant | Blend from captured gameplay state into the sequence. |
| `exit_blend` | `CinematicBlend` | instant | Blend from the current cinematic pose back to the captured gameplay state. |

## `CinematicShot`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `name` | `String` | `"Shot"` | Used in lifecycle messages and debug output. |
| `duration_secs` | `f32` | `1.0` | Visible shot duration. Keep positive. |
| `progress_easing` | `CinematicEasing` | `Linear` | Shapes local shot time before position and lens tracks are sampled. |
| `position` | `PositionTrack` | `Fixed(Vec3::ZERO)` | Either a fixed world position or a rail traversal. |
| `orientation` | `OrientationTrack` | `Fixed(Quat::IDENTITY)` | Fixed quaternion, tangent-facing, or target-aware solve. |
| `lens` | `LensTrack` | fixed default perspective FOV | Start/end FOV and easing. |
| `blend_in` | `CinematicBlend` | instant | Overlap duration from the previous shot into this one. |
| `markers` | `Vec<ShotMarker>` | `[]` | Timed named markers emitted while advancing forward through the shot. |
| `shake` | `ProceduralShake` | disabled | Additive deterministic handheld noise sampled from local shot time. |

## Rail Authoring

### `CinematicRail`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `kind` | `RailSplineKind` | `CatmullRom` | `Linear` for hard authored corners, `CatmullRom` for smooth curves. |
| `points` | `Vec<Vec3>` | two-point fallback | At least two points are useful. Catmull-Rom falls back to linear sampling when there are too few points. |
| `closed` | `bool` | `false` | Closes the path back to its first point. |
| `samples_per_segment` | `usize` | `24` | Arc-length cache density. Raise this for large sweeping curves if motion appears uneven. |

### `RailTrack`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `rail` | `CinematicRail` | default rail | Authored path. |
| `traversal` | `RailTraversal` | `0.0 -> 1.0`, normalized, `Once` | Defines the sampled interval and rail-local loop policy. |

### `RailTraversal`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `start` | `f32` | `0.0` | Start value in normalized or distance units. |
| `end` | `f32` | `1.0` | End value in normalized or distance units. |
| `unit` | `RailProgressUnit` | `Normalized` | `Normalized` means arc-length-normalized distance, not raw spline domain. |
| `loop_mode` | `PlaybackLoopMode` | `Once` | Rail-local wrapping policy inside the shot. |

## Orientation

### `PathTangentOrientation`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `up` | `UpVectorMode` | `WorldY` | Preferred up vector for tangent solve. Replaced defensively if it becomes parallel to the tangent. |
| `roll_radians` | `f32` | `0.0` | Additional roll around the camera's local forward axis. |

### `LookAtTarget::Entity`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `entity` | `Entity` | required | Entity to look at. |
| `offset` | `Vec3` | `Vec3::ZERO` | Local world-space offset from the entity translation. |
| `up` | `UpVectorMode` | `WorldY` | Preferred up vector for look solve. |
| `look_ahead_secs` | `f32` | `0.0` | Multiplies cached target velocity and shifts the look point ahead in time. |

### `CinematicTargetGroup`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `members` | `Vec<WeightedTarget>` | `[]` | Weighted target list. Non-positive weights are ignored. |
| `fallback_point` | `Vec3` | `Vec3::ZERO` | Used if no target entity can be resolved. |
| `up` | `UpVectorMode` | `WorldY` | Group-wide up-vector hint. |
| `look_ahead_secs` | `f32` | `0.0` | Shared look-ahead time for every member in the group. |

## Lens

### `LensTrack`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `start_fov_y_radians` | `f32` | Bevy perspective default | Start vertical FOV in radians. |
| `end_fov_y_radians` | `f32` | Bevy perspective default | End vertical FOV in radians. |
| `easing` | `CinematicEasing` | `Linear` | Shapes FOV interpolation across the shot. |

## Output Damping

### `CinematicOutputDamping`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `position_rate` | `f32` | `20.0` | Exponential smoothing rate for position. Higher values track the raw solve more tightly. |
| `rotation_rate` | `f32` | `15.0` | Exponential smoothing rate for rotation. Higher values track the raw solve more tightly. |

Presets:

| Preset | `position_rate` | `rotation_rate` | Use case |
| --- | --- | --- | --- |
| `default()` | `20.0` | `15.0` | General purpose smoothing |
| `light()` | `30.0` | `25.0` | Subtle smoothing for fast-moving rigs |
| `heavy()` | `8.0` | `6.0` | Heavy cinematic smoothing for slow, deliberate shots |

Both rates use framerate-independent exponential smoothing: `factor = 1.0 - exp(-rate * dt)`.

## Collision Avoidance

### `CinematicCollisionAvoidance`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `policy` | `CollisionPolicy` | `PushCloser` | How the system responds to collisions. |
| `padding` | `f32` | `0.3` | Minimum distance to keep from the collision surface (meters). |
| `retract_rate` | `f32` | `20.0` | Exponential smoothing rate when pulling the camera closer (fast). |
| `recover_rate` | `f32` | `4.0` | Exponential smoothing rate when moving the camera back out (slow). |
| `min_distance` | `f32` | `0.5` | Minimum distance from the look target to the camera (meters). |

### `CollisionPolicy`

| Variant | Behavior |
| --- | --- |
| `PushCloser` | Move the camera closer to the look target to avoid the obstruction. |
| `None` | Record collision data but do not modify the camera position. |

Presets:

| Preset | `padding` | `retract_rate` | `recover_rate` | `min_distance` |
| --- | --- | --- | --- | --- |
| `default()` | `0.3` | `20.0` | `4.0` | `0.5` |
| `tight()` | `0.15` | `30.0` | `6.0` | `0.3` |
| `loose()` | `0.6` | `12.0` | `3.0` | `0.8` |

Collision avoidance requires `MeshPickingPlugin` to be added to the app. Without it, the collision system is not registered.

## Debug

### `CinematicCameraDebugSettings`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `enabled` | `bool` | `false` | Master gizmo toggle. Shared runtime consumers opt in explicitly; examples and the lab turn this on themselves. |
| `draw_paths` | `bool` | `true` | Draws rail lines. |
| `draw_control_points` | `bool` | `true` | Draws rail control-point crosses. |
| `draw_targets` | `bool` | `true` | Draws the solved look vector and target marker. |
| `draw_camera_forward` | `bool` | `true` | Draws the solved camera forward arrow. |
| `draw_active_samples` | `bool` | `true` | Draws the current solved camera sample point. |

## Shake

### `ProceduralShake`

| Field | Type | Default | Notes |
| --- | --- | --- | --- |
| `translation_amplitude` | `Vec3` | `Vec3::ZERO` | Local additive translation in world units. |
| `rotation_amplitude_radians` | `Vec3` | `Vec3::ZERO` | Local additive XYZ Euler offsets in radians. |
| `frequency_hz` | `Vec3` | `Vec3::ONE` | Per-axis frequency of the deterministic sine stack. |
| `seed` | `f32` | `0.0` | Phase offset used to decorrelate shots. |

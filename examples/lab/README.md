# Cinematic Camera Lab

Crate-local standalone lab app for validating the shared `cinematic_camera` crate in a real Bevy application.

## Purpose

- verify that the shared crate can drive a real workspace camera and hand control back cleanly
- keep a short authored reveal sequence available for manual inspection and screenshot-based checks
- expose playback state, active shot, blend weights, and camera ownership through an overlay

## Status

Working

## Run

```bash
cargo run -p saddle-camera-cinematic-camera-lab
```

## E2E

```bash
cargo run -p saddle-camera-cinematic-camera-lab --features e2e -- smoke_launch
cargo run -p saddle-camera-cinematic-camera-lab --features e2e -- cinematic_camera_virtual_camera
```

## Notes

- Press `Space` to restart the reveal sequence.
- The lab now exercises the explicit `CinematicVirtualCamera` + `CinematicCameraBrain` authoring path rather than only the lower-level binding components.

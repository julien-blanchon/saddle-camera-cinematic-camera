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
cargo run -p cinematic_camera_lab
```

## Notes

- Press `Space` to restart the reveal sequence.
- The lab keeps the scene generic: one moving subject, one gameplay camera, one cinematic rig, and a short blend-in / blend-out sequence.

use bevy::{ecs::message::Messages, prelude::*};
use saddle_bevy_e2e::{
    E2EPlugin, E2ESet,
    action::Action,
    actions::{assertions, inspect},
    init_scenario,
    scenario::Scenario,
};
use saddle_camera_cinematic_camera::{
    CinematicCameraBrain, CinematicCameraState, CinematicCameraSystems, CinematicDrivenCamera,
    CinematicPlayback, CinematicPlaybackCommand, CinematicPlaybackStatus, CinematicVirtualCamera,
};

use crate::LabHandles;

pub struct CinematicCameraLabE2EPlugin;

impl Plugin for CinematicCameraLabE2EPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(E2EPlugin);
        app.configure_sets(
            Update,
            E2ESet.before(CinematicCameraSystems::InputOrCommands),
        );

        let args: Vec<String> = std::env::args().collect();
        let (scenario_name, handoff) = parse_e2e_args(&args);

        if let Some(name) = scenario_name {
            if let Some(mut scenario) = scenario_by_name(&name) {
                if handoff {
                    scenario.actions.push(Action::Handoff);
                }
                init_scenario(app, scenario);
            } else {
                error!(
                    "[cinematic_camera_lab:e2e] Unknown scenario '{name}'. Available: {:?}",
                    list_scenarios()
                );
            }
        }
    }
}

fn parse_e2e_args(args: &[String]) -> (Option<String>, bool) {
    let mut scenario_name = None;
    let mut handoff = false;

    for arg in args.iter().skip(1) {
        if arg == "--handoff" {
            handoff = true;
        } else if !arg.starts_with('-') && scenario_name.is_none() {
            scenario_name = Some(arg.clone());
        }
    }

    if !handoff {
        handoff = std::env::var("E2E_HANDOFF").is_ok_and(|value| value == "1" || value == "true");
    }

    (scenario_name, handoff)
}

fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "smoke_launch" => Some(build_smoke_launch()),
        "cinematic_camera_virtual_camera" => Some(build_virtual_camera()),
        "cinematic_camera_playback_commands" => Some(build_playback_commands()),
        "cinematic_camera_shot_transitions" => Some(build_shot_transitions()),
        "cinematic_camera_seek" => Some(build_seek()),
        "cinematic_camera_moving_target" => Some(build_moving_target()),
        "cinematic_camera_target_group" => Some(build_target_group()),
        "cinematic_camera_handheld_rail" => Some(build_handheld_rail()),
        _ => None,
    }
}

fn list_scenarios() -> Vec<&'static str> {
    vec![
        "smoke_launch",
        "cinematic_camera_virtual_camera",
        "cinematic_camera_playback_commands",
        "cinematic_camera_shot_transitions",
        "cinematic_camera_seek",
        "cinematic_camera_moving_target",
        "cinematic_camera_target_group",
        "cinematic_camera_handheld_rail",
    ]
}

fn handles(world: &World) -> LabHandles {
    let handles = world.resource::<LabHandles>();
    LabHandles {
        rig: handles.rig,
        camera: handles.camera,
    }
}

fn build_smoke_launch() -> Scenario {
    Scenario::builder("smoke_launch")
        .description("Boot the cinematic lab, verify the virtual-camera authoring path is present, and capture a baseline screenshot.")
        .then(Action::WaitFrames(90))
        .then(assertions::entity_exists::<CinematicVirtualCamera>(
            "virtual camera authoring entity exists",
        ))
        .then(assertions::entity_exists::<CinematicCameraBrain>(
            "brain camera entity exists",
        ))
        .then(assertions::component_satisfies::<CinematicPlayback>(
            "playback auto-started",
            |playback| playback.status == CinematicPlaybackStatus::Playing,
        ))
        .then(assertions::log_summary("smoke_launch summary"))
        .then(Action::Screenshot("smoke_launch".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn build_virtual_camera() -> Scenario {
    Scenario::builder("cinematic_camera_virtual_camera")
        .description("Let the authored virtual camera advance through the sequence, assert the shared brain camera is being driven, and capture diagnostic output.")
        .then(Action::WaitFrames(120))
        .then(assertions::custom(
            "brain camera is driven by the rig",
            Box::new(|world: &World| {
                let handles = handles(world);
                world
                    .get::<CinematicDrivenCamera>(handles.camera)
                    .is_some_and(|driven| driven.owner == handles.rig)
            }),
        ))
        .then(assertions::custom(
            "camera state advanced into the authored sequence",
            Box::new(|world: &World| {
                let handles = handles(world);
                world
                    .get::<CinematicCameraState>(handles.rig)
                    .is_some_and(|state| {
                        state.active
                            && state.sequence_time_secs > 1.0
                            && state.current_shot.is_some()
                            && state.translation.is_finite()
                    })
            }),
        ))
        .then(inspect::dump_component_json::<CinematicCameraState>(
            "cinematic_camera_virtual_camera_state",
        ))
        .then(Action::Screenshot("cinematic_camera_virtual_camera".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn build_playback_commands() -> Scenario {
    Scenario::builder("cinematic_camera_playback_commands")
        .description(
            "Issue Pause and Resume playback commands via the message bus. Assert the playback \
             status reflects each command and that timeline time freezes while paused then \
             advances again after resume.",
        )
        // Let auto-play start and advance past the first blend-in.
        .then(Action::WaitFrames(90))
        .then(assertions::component_satisfies::<CinematicPlayback>(
            "playback is playing before pause",
            |p| p.status == CinematicPlaybackStatus::Playing,
        ))
        // Capture the elapsed time so we can compare after pause.
        .then(Action::Custom(Box::new(|world: &mut World| {
            let h = handles(world);
            let elapsed = world
                .get::<CinematicPlayback>(h.rig)
                .map(|p| p.elapsed_secs)
                .unwrap_or(0.0);
            world.insert_resource(PauseCheckpoint { elapsed_at_pause: elapsed });
        })))
        .then(Action::Screenshot("cinematic_camera_playback_commands_playing".into()))
        .then(Action::WaitFrames(1))
        // Send Pause command.
        .then(Action::Custom(Box::new(|world: &mut World| {
            let rig = handles(world).rig;
            world
                .resource_mut::<Messages<CinematicPlaybackCommand>>()
                .write(CinematicPlaybackCommand::Pause(rig));
        })))
        .then(Action::WaitFrames(4))
        .then(assertions::component_satisfies::<CinematicPlayback>(
            "status becomes Paused",
            |p| p.status == CinematicPlaybackStatus::Paused,
        ))
        // Wait and confirm time is frozen.
        .then(Action::WaitFrames(30))
        .then(assertions::custom(
            "timeline time does not advance while paused",
            Box::new(|world: &World| {
                let h = handles(world);
                let checkpoint = world.resource::<PauseCheckpoint>();
                world
                    .get::<CinematicPlayback>(h.rig)
                    .is_some_and(|p| (p.elapsed_secs - checkpoint.elapsed_at_pause).abs() < 0.1)
            }),
        ))
        .then(Action::Screenshot("cinematic_camera_playback_commands_paused".into()))
        .then(Action::WaitFrames(1))
        // Send Resume command.
        .then(Action::Custom(Box::new(|world: &mut World| {
            let rig = handles(world).rig;
            world
                .resource_mut::<Messages<CinematicPlaybackCommand>>()
                .write(CinematicPlaybackCommand::Resume(rig));
        })))
        .then(Action::WaitFrames(8))
        .then(assertions::component_satisfies::<CinematicPlayback>(
            "status becomes Playing again",
            |p| p.status == CinematicPlaybackStatus::Playing,
        ))
        .then(assertions::custom(
            "timeline time advanced past pause checkpoint",
            Box::new(|world: &World| {
                let h = handles(world);
                let checkpoint = world.resource::<PauseCheckpoint>();
                world
                    .get::<CinematicPlayback>(h.rig)
                    .is_some_and(|p| p.elapsed_secs > checkpoint.elapsed_at_pause + 0.05)
            }),
        ))
        .then(assertions::log_summary("cinematic_camera_playback_commands summary"))
        .then(inspect::dump_component_json::<CinematicPlayback>(
            "cinematic_camera_playback_commands_playback",
        ))
        .then(Action::Screenshot("cinematic_camera_playback_commands_resumed".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn build_shot_transitions() -> Scenario {
    Scenario::builder("cinematic_camera_shot_transitions")
        .description(
            "Restart the sequence from the beginning and wait long enough for the first \
             shot-to-shot transition. Assert that the current_shot index advances (i.e. the \
             system correctly cut from shot 0 to shot 1) and that the camera translation \
             remains finite throughout.",
        )
        .then(Action::WaitFrames(30))
        // Restart so the timeline is deterministic from frame 0.
        .then(Action::Custom(Box::new(|world: &mut World| {
            let rig = handles(world).rig;
            world
                .resource_mut::<Messages<CinematicPlaybackCommand>>()
                .write(CinematicPlaybackCommand::Restart(rig));
        })))
        .then(Action::WaitFrames(4))
        .then(assertions::component_satisfies::<CinematicPlayback>(
            "playback restarted at time zero",
            |p| p.elapsed_secs < 0.3 && p.status == CinematicPlaybackStatus::Playing,
        ))
        .then(Action::Screenshot("cinematic_camera_shot_transitions_shot0".into()))
        .then(Action::WaitFrames(1))
        // The "Establish" shot is 1.8 s. At 60 fps that is ~108 frames.
        // Wait 130 frames so we are comfortably past the first transition.
        .then(Action::WaitFrames(130))
        .then(assertions::custom(
            "current shot advanced to index >= 1 (shot transition occurred)",
            Box::new(|world: &World| {
                let h = handles(world);
                world
                    .get::<CinematicCameraState>(h.rig)
                    .is_some_and(|state| {
                        state.current_shot.is_some_and(|idx| idx >= 1)
                    })
            }),
        ))
        .then(assertions::custom(
            "camera translation remains finite after transition",
            Box::new(|world: &World| {
                let h = handles(world);
                world
                    .get::<CinematicCameraState>(h.rig)
                    .is_some_and(|state| state.translation.is_finite())
            }),
        ))
        .then(assertions::log_summary("cinematic_camera_shot_transitions summary"))
        .then(inspect::dump_component_json::<CinematicCameraState>(
            "cinematic_camera_shot_transitions_state",
        ))
        .then(Action::Screenshot("cinematic_camera_shot_transitions_shot1".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn build_seek() -> Scenario {
    Scenario::builder("cinematic_camera_seek")
        .description(
            "Restart the sequence, then use SeekSeconds to jump to the middle of the authored \
             sequence. Assert that the timeline time lands near the requested seconds and that \
             the camera position changes significantly from the start position.",
        )
        .then(Action::WaitFrames(30))
        // Restart to a known baseline position.
        .then(Action::Custom(Box::new(|world: &mut World| {
            let rig = handles(world).rig;
            world
                .resource_mut::<Messages<CinematicPlaybackCommand>>()
                .write(CinematicPlaybackCommand::Restart(rig));
        })))
        .then(Action::WaitFrames(4))
        // Capture baseline position right after restart.
        .then(Action::Custom(Box::new(|world: &mut World| {
            let h = handles(world);
            let translation = world
                .get::<CinematicCameraState>(h.rig)
                .map(|s| s.translation)
                .unwrap_or(Vec3::ZERO);
            world.insert_resource(SeekCheckpoint { translation_before_seek: translation });
        })))
        .then(Action::Screenshot("cinematic_camera_seek_before".into()))
        .then(Action::WaitFrames(1))
        // Seek to 3.5 s — well into the "Push" rail shot (starts ~1.8 s).
        .then(Action::Custom(Box::new(|world: &mut World| {
            let rig = handles(world).rig;
            world
                .resource_mut::<Messages<CinematicPlaybackCommand>>()
                .write(CinematicPlaybackCommand::SeekSeconds { rig, seconds: 3.5 });
        })))
        .then(Action::WaitFrames(4))
        .then(assertions::custom(
            "elapsed time lands near the requested seek target",
            Box::new(|world: &World| {
                let h = handles(world);
                world
                    .get::<CinematicPlayback>(h.rig)
                    .is_some_and(|p| (p.elapsed_secs - 3.5).abs() < 0.25)
            }),
        ))
        .then(assertions::custom(
            "seek moved the camera to a different position",
            Box::new(|world: &World| {
                let h = handles(world);
                let checkpoint = world.resource::<SeekCheckpoint>();
                world
                    .get::<CinematicCameraState>(h.rig)
                    .is_some_and(|state| {
                        state.translation.is_finite()
                            && state.translation.distance(checkpoint.translation_before_seek) > 0.5
                    })
            }),
        ))
        .then(assertions::log_summary("cinematic_camera_seek summary"))
        .then(inspect::dump_component_json::<CinematicCameraState>(
            "cinematic_camera_seek_state",
        ))
        .then(Action::Screenshot("cinematic_camera_seek_after".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn build_moving_target() -> Scenario {
    Scenario::builder("cinematic_camera_moving_target")
        .description(
            "Let the sequence auto-play through the LookAt shots while the subject orbits, \
             verify that the camera translation changes over time (camera is tracking the moving \
             subject) and that the camera state stays finite and active.",
        )
        .then(Action::WaitFrames(30))
        // Restart so the sequence is deterministic from frame 0.
        .then(Action::Custom(Box::new(|world: &mut World| {
            let rig = handles(world).rig;
            world
                .resource_mut::<Messages<CinematicPlaybackCommand>>()
                .write(CinematicPlaybackCommand::Restart(rig));
        })))
        .then(Action::WaitFrames(4))
        // Record the initial camera position.
        .then(Action::Custom(Box::new(|world: &mut World| {
            let h = handles(world);
            let translation = world
                .get::<CinematicCameraState>(h.rig)
                .map(|s| s.translation)
                .unwrap_or(Vec3::ZERO);
            world.insert_resource(MovingTargetCheckpoint {
                translation_at_start: translation,
            });
        })))
        .then(Action::Screenshot("moving_target_start".into()))
        .then(Action::WaitFrames(1))
        // Wait 90 frames — the subject has moved appreciably along its orbit.
        .then(Action::WaitFrames(90))
        .then(assertions::custom(
            "camera state remains active and finite while tracking",
            Box::new(|world: &World| {
                let h = handles(world);
                world
                    .get::<CinematicCameraState>(h.rig)
                    .is_some_and(|state| state.active && state.translation.is_finite())
            }),
        ))
        .then(assertions::custom(
            "camera translation changed while tracking the moving subject",
            Box::new(|world: &World| {
                let h = handles(world);
                let checkpoint = world.resource::<MovingTargetCheckpoint>();
                world
                    .get::<CinematicCameraState>(h.rig)
                    .is_some_and(|state| {
                        state.translation.is_finite()
                            && state
                                .translation
                                .distance(checkpoint.translation_at_start)
                                > 0.1
                    })
            }),
        ))
        .then(assertions::log_summary("cinematic_camera_moving_target summary"))
        .then(inspect::dump_component_json::<CinematicCameraState>(
            "moving_target_state",
        ))
        .then(Action::Screenshot("moving_target_after".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn build_target_group() -> Scenario {
    Scenario::builder("cinematic_camera_target_group")
        .description(
            "Verify the brain camera is driven throughout the entire authored sequence by \
             sampling the rig state at two time points separated by at least one shot \
             boundary and confirming the translation changed.",
        )
        .then(Action::WaitFrames(30))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let rig = handles(world).rig;
            world
                .resource_mut::<Messages<CinematicPlaybackCommand>>()
                .write(CinematicPlaybackCommand::Restart(rig));
        })))
        .then(Action::WaitFrames(4))
        .then(assertions::custom(
            "brain is driven by the rig at sequence start",
            Box::new(|world: &World| {
                let h = handles(world);
                world
                    .get::<CinematicDrivenCamera>(h.camera)
                    .is_some_and(|driven| driven.owner == h.rig)
            }),
        ))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let h = handles(world);
            let translation = world
                .get::<CinematicCameraState>(h.rig)
                .map(|s| s.translation)
                .unwrap_or(Vec3::ZERO);
            world.insert_resource(TargetGroupCheckpoint { translation_shot0: translation });
        })))
        .then(Action::Screenshot("target_group_shot0".into()))
        .then(Action::WaitFrames(1))
        // Wait past the first shot (Establish = 1.8 s → 108 frames at 60fps).
        .then(Action::WaitFrames(130))
        .then(assertions::custom(
            "camera translation changed after crossing shot boundary",
            Box::new(|world: &World| {
                let h = handles(world);
                let checkpoint = world.resource::<TargetGroupCheckpoint>();
                world
                    .get::<CinematicCameraState>(h.rig)
                    .is_some_and(|state| {
                        state.translation.is_finite()
                            && state.translation.distance(checkpoint.translation_shot0) > 0.2
                    })
            }),
        ))
        .then(assertions::custom(
            "brain camera is still driven after transition",
            Box::new(|world: &World| {
                let h = handles(world);
                world
                    .get::<CinematicDrivenCamera>(h.camera)
                    .is_some_and(|driven| driven.owner == h.rig)
            }),
        ))
        .then(assertions::log_summary("cinematic_camera_target_group summary"))
        .then(inspect::dump_component_json::<CinematicCameraState>(
            "target_group_state",
        ))
        .then(Action::Screenshot("target_group_shot1".into()))
        .then(Action::WaitFrames(1))
        .build()
}

fn build_handheld_rail() -> Scenario {
    Scenario::builder("cinematic_camera_handheld_rail")
        .description(
            "Seek into the rail-based Push shot (starts at ~1.8 s) and verify the camera \
             position advances along the rail while the rig state shows the correct shot index \
             and the camera translation is finite throughout.",
        )
        .then(Action::WaitFrames(30))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let rig = handles(world).rig;
            world
                .resource_mut::<Messages<CinematicPlaybackCommand>>()
                .write(CinematicPlaybackCommand::Restart(rig));
        })))
        .then(Action::WaitFrames(4))
        // Seek to the start of the Push rail shot.
        .then(Action::Custom(Box::new(|world: &mut World| {
            let rig = handles(world).rig;
            world
                .resource_mut::<Messages<CinematicPlaybackCommand>>()
                .write(CinematicPlaybackCommand::SeekSeconds { rig, seconds: 1.9 });
        })))
        .then(Action::WaitFrames(4))
        .then(assertions::custom(
            "elapsed time landed in the rail shot window",
            Box::new(|world: &World| {
                let h = handles(world);
                world
                    .get::<CinematicPlayback>(h.rig)
                    .is_some_and(|p| p.elapsed_secs >= 1.7 && p.elapsed_secs < 4.5)
            }),
        ))
        .then(assertions::custom(
            "current shot index is 1 (the Push rail shot)",
            Box::new(|world: &World| {
                let h = handles(world);
                world
                    .get::<CinematicCameraState>(h.rig)
                    .is_some_and(|state| state.current_shot.is_some_and(|idx| idx == 1))
            }),
        ))
        .then(Action::Custom(Box::new(|world: &mut World| {
            let h = handles(world);
            let translation = world
                .get::<CinematicCameraState>(h.rig)
                .map(|s| s.translation)
                .unwrap_or(Vec3::ZERO);
            world.insert_resource(RailCheckpoint { translation_rail_start: translation });
        })))
        .then(Action::Screenshot("handheld_rail_start".into()))
        .then(Action::WaitFrames(1))
        // Let the rail advance for 1.5 s worth of frames.
        .then(Action::WaitFrames(90))
        .then(assertions::custom(
            "camera moved along the rail track",
            Box::new(|world: &World| {
                let h = handles(world);
                let checkpoint = world.resource::<RailCheckpoint>();
                world
                    .get::<CinematicCameraState>(h.rig)
                    .is_some_and(|state| {
                        state.translation.is_finite()
                            && state
                                .translation
                                .distance(checkpoint.translation_rail_start)
                                > 0.3
                    })
            }),
        ))
        .then(assertions::log_summary("cinematic_camera_handheld_rail summary"))
        .then(inspect::dump_component_json::<CinematicCameraState>(
            "handheld_rail_state",
        ))
        .then(Action::Screenshot("handheld_rail_end".into()))
        .then(Action::WaitFrames(1))
        .build()
}

// ── checkpoint resources used across scenarios ────────────────────────────────

#[derive(Resource, Clone, Copy)]
struct PauseCheckpoint {
    elapsed_at_pause: f32,
}

#[derive(Resource, Clone, Copy)]
struct SeekCheckpoint {
    translation_before_seek: Vec3,
}

#[derive(Resource, Clone, Copy)]
struct MovingTargetCheckpoint {
    translation_at_start: Vec3,
}

#[derive(Resource, Clone, Copy)]
struct TargetGroupCheckpoint {
    translation_shot0: Vec3,
}

#[derive(Resource, Clone, Copy)]
struct RailCheckpoint {
    translation_rail_start: Vec3,
}

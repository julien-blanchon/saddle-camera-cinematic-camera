use bevy::prelude::*;
use saddle_bevy_e2e::{
    E2EPlugin, E2ESet,
    action::Action,
    actions::{assertions, inspect},
    init_scenario,
    scenario::Scenario,
};
use saddle_camera_cinematic_camera::{
    CinematicCameraBrain, CinematicCameraState, CinematicCameraSystems, CinematicDrivenCamera,
    CinematicPlayback, CinematicPlaybackStatus, CinematicVirtualCamera,
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
        _ => None,
    }
}

fn list_scenarios() -> Vec<&'static str> {
    vec!["smoke_launch", "cinematic_camera_virtual_camera"]
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

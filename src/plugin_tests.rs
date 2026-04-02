use bevy::{ecs::schedule::ScheduleLabel, prelude::*};

use crate::CinematicCameraPlugin;

#[test]
fn always_on_constructor_uses_requested_update_schedule() {
    let plugin = CinematicCameraPlugin::always_on(Update);
    assert_eq!(plugin.update_schedule, Update.intern());
}

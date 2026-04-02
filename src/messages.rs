use bevy::prelude::*;

#[derive(Clone, Debug, Message)]
pub enum CinematicPlaybackCommand {
    Play(Entity),
    Pause(Entity),
    Resume(Entity),
    Restart(Entity),
    Stop { rig: Entity, restore_camera: bool },
    SeekSeconds { rig: Entity, seconds: f32 },
    SeekNormalized { rig: Entity, normalized: f32 },
}

#[derive(Clone, Debug, Message)]
pub struct ShotStarted {
    pub rig: Entity,
    pub shot_index: usize,
    pub shot_name: String,
}

#[derive(Clone, Debug, Message)]
pub struct ShotFinished {
    pub rig: Entity,
    pub shot_index: usize,
    pub shot_name: String,
}

#[derive(Clone, Debug, Message)]
pub struct ShotMarkerReached {
    pub rig: Entity,
    pub shot_index: usize,
    pub shot_name: String,
    pub marker_name: String,
}

#[derive(Clone, Debug, Message)]
pub struct SequenceFinished {
    pub rig: Entity,
}

#[derive(Clone, Debug)]
pub enum CinematicBlendKind {
    GameplayToCinematic { to_shot: Option<usize> },
    ShotToShot { from_shot: usize, to_shot: usize },
    CinematicToGameplay { from_shot: Option<usize> },
}

#[derive(Clone, Debug, Message)]
pub struct CinematicBlendCompleted {
    pub rig: Entity,
    pub kind: CinematicBlendKind,
}

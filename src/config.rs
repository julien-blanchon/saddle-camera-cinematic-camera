use bevy::{
    math::curve::{
        Curve,
        easing::{EaseFunction, JumpAt},
    },
    prelude::*,
    reflect::Reflect,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum CinematicEasing {
    #[default]
    Linear,
    SmoothStep,
    SmootherStep,
    QuadraticInOut,
    CubicInOut,
    SineInOut,
    BackInOut,
    ElasticOut,
    BounceOut,
    Steps4,
}

impl CinematicEasing {
    pub fn sample(self, t: f32) -> f32 {
        let curve = match self {
            Self::Linear => EaseFunction::Linear,
            Self::SmoothStep => EaseFunction::SmoothStep,
            Self::SmootherStep => EaseFunction::SmootherStep,
            Self::QuadraticInOut => EaseFunction::QuadraticInOut,
            Self::CubicInOut => EaseFunction::CubicInOut,
            Self::SineInOut => EaseFunction::SineInOut,
            Self::BackInOut => EaseFunction::BackInOut,
            Self::ElasticOut => EaseFunction::ElasticOut,
            Self::BounceOut => EaseFunction::BounceOut,
            Self::Steps4 => EaseFunction::Steps(4, JumpAt::None),
        };
        curve.sample_clamped(t.clamp(0.0, 1.0))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum PlaybackLoopMode {
    #[default]
    Once,
    Loop,
    PingPong,
}

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct CinematicBlend {
    pub duration_secs: f32,
    pub easing: CinematicEasing,
}

impl Default for CinematicBlend {
    fn default() -> Self {
        Self {
            duration_secs: 0.0,
            easing: CinematicEasing::SineInOut,
        }
    }
}

impl CinematicBlend {
    pub const fn instant() -> Self {
        Self {
            duration_secs: 0.0,
            easing: CinematicEasing::Linear,
        }
    }

    pub fn alpha(self, elapsed_secs: f32) -> f32 {
        if self.duration_secs <= f32::EPSILON {
            return 1.0;
        }
        self.easing.sample(elapsed_secs / self.duration_secs)
    }

    pub fn is_active(self) -> bool {
        self.duration_secs > f32::EPSILON
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect)]
pub enum MarkerTime {
    Seconds(f32),
    #[default]
    EndOfShot,
    Normalized(f32),
}

impl MarkerTime {
    pub fn seconds(self, duration_secs: f32) -> f32 {
        match self {
            Self::Seconds(value) => value.clamp(0.0, duration_secs.max(0.0)),
            Self::EndOfShot => duration_secs.max(0.0),
            Self::Normalized(value) => value.clamp(0.0, 1.0) * duration_secs.max(0.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct ShotMarker {
    pub name: String,
    pub at: MarkerTime,
}

impl ShotMarker {
    pub fn normalized(name: impl Into<String>, normalized: f32) -> Self {
        Self {
            name: name.into(),
            at: MarkerTime::Normalized(normalized),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect)]
pub enum UpVectorMode {
    #[default]
    WorldY,
    Vector(Vec3),
}

impl UpVectorMode {
    pub fn vector(self) -> Vec3 {
        match self {
            Self::WorldY => Vec3::Y,
            Self::Vector(vector) => vector,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct ProceduralShake {
    pub translation_amplitude: Vec3,
    pub rotation_amplitude_radians: Vec3,
    pub frequency_hz: Vec3,
    pub seed: f32,
}

impl Default for ProceduralShake {
    fn default() -> Self {
        Self {
            translation_amplitude: Vec3::ZERO,
            rotation_amplitude_radians: Vec3::ZERO,
            frequency_hz: Vec3::splat(1.0),
            seed: 0.0,
        }
    }
}

impl ProceduralShake {
    pub fn is_enabled(self) -> bool {
        self.translation_amplitude.length_squared() > 0.0
            || self.rotation_amplitude_radians.length_squared() > 0.0
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;

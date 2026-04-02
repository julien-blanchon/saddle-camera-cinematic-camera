use bevy::{
    math::{cubic_splines::CubicCardinalSpline, curve::Curve},
    prelude::*,
    reflect::Reflect,
};

use crate::PlaybackLoopMode;

pub const DEFAULT_SAMPLES_PER_SEGMENT: usize = 24;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum RailSplineKind {
    Linear,
    #[default]
    CatmullRom,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum RailProgressUnit {
    #[default]
    Normalized,
    Distance,
}

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct RailTraversal {
    pub start: f32,
    pub end: f32,
    pub unit: RailProgressUnit,
    pub loop_mode: PlaybackLoopMode,
}

impl Default for RailTraversal {
    fn default() -> Self {
        Self {
            start: 0.0,
            end: 1.0,
            unit: RailProgressUnit::Normalized,
            loop_mode: PlaybackLoopMode::Once,
        }
    }
}

impl RailTraversal {
    pub fn sample(self, cache: &CinematicRailCache, progress: f32) -> RailSample {
        let value = self.start.lerp(self.end, progress.clamp(0.0, 1.0));
        match self.unit {
            RailProgressUnit::Normalized => cache.sample_normalized(value, self.loop_mode),
            RailProgressUnit::Distance => cache.sample_distance(value, self.loop_mode),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct CinematicRail {
    pub kind: RailSplineKind,
    pub points: Vec<Vec3>,
    pub closed: bool,
    pub samples_per_segment: usize,
}

impl Default for CinematicRail {
    fn default() -> Self {
        Self {
            kind: RailSplineKind::CatmullRom,
            points: vec![Vec3::ZERO, Vec3::new(0.0, 0.0, -5.0)],
            closed: false,
            samples_per_segment: DEFAULT_SAMPLES_PER_SEGMENT,
        }
    }
}

impl CinematicRail {
    pub fn from_points(points: impl Into<Vec<Vec3>>) -> Self {
        Self {
            points: points.into(),
            ..default()
        }
    }

    pub(crate) fn segment_count(&self) -> usize {
        match self.points.len() {
            0 | 1 => 1,
            len if self.closed => len,
            len => len - 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RailSample {
    pub position: Vec3,
    pub tangent: Vec3,
    pub distance: f32,
    pub normalized: f32,
}

#[derive(Clone, Debug, Default)]
pub struct CinematicRailCache {
    samples: Vec<CachedRailSample>,
    total_length: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct CachedRailSample {
    position: Vec3,
    tangent: Vec3,
    distance: f32,
}

impl CinematicRailCache {
    pub fn rebuild(rail: &CinematicRail) -> Self {
        let samples = if rail.points.is_empty() {
            vec![CachedRailSample::default()]
        } else if rail.points.len() == 1 {
            vec![CachedRailSample {
                position: rail.points[0],
                tangent: Vec3::Z,
                distance: 0.0,
            }]
        } else if rail.kind == RailSplineKind::CatmullRom && rail.points.len() >= 4 {
            build_spline_samples(rail)
        } else {
            build_linear_samples(rail)
        };

        let total_length = samples.last().map_or(0.0, |sample| sample.distance);
        Self {
            samples,
            total_length,
        }
    }

    pub fn total_length(&self) -> f32 {
        self.total_length
    }

    pub fn sample_normalized(&self, normalized: f32, loop_mode: PlaybackLoopMode) -> RailSample {
        if self.total_length <= f32::EPSILON {
            return self.first_sample();
        }
        let wrapped = wrap_scalar(normalized, 1.0, loop_mode);
        self.sample_distance(wrapped * self.total_length, PlaybackLoopMode::Once)
    }

    pub fn sample_distance(&self, distance: f32, loop_mode: PlaybackLoopMode) -> RailSample {
        if self.samples.len() <= 1 || self.total_length <= f32::EPSILON {
            return self.first_sample();
        }

        let target = wrap_scalar(distance, self.total_length, loop_mode);

        match self
            .samples
            .binary_search_by(|sample| sample.distance.total_cmp(&target))
        {
            Ok(index) => self.sample_at_index(index),
            Err(index) => {
                let upper = index.min(self.samples.len() - 1);
                let lower = upper.saturating_sub(1);
                interpolate_samples(
                    self.samples[lower],
                    self.samples[upper],
                    target,
                    self.total_length,
                )
            }
        }
    }

    fn sample_at_index(&self, index: usize) -> RailSample {
        let sample = self.samples[index];
        RailSample {
            position: sample.position,
            tangent: sample.tangent,
            distance: sample.distance,
            normalized: if self.total_length <= f32::EPSILON {
                0.0
            } else {
                sample.distance / self.total_length
            },
        }
    }

    fn first_sample(&self) -> RailSample {
        self.sample_at_index(0)
    }
}

fn build_spline_samples(rail: &CinematicRail) -> Vec<CachedRailSample> {
    let spline = CubicCardinalSpline::new_catmull_rom(rail.points.clone());
    let curve = if rail.closed {
        spline
            .to_curve_cyclic()
            .expect("cyclic catmull-rom curve should build")
    } else {
        spline.to_curve().expect("catmull-rom curve should build")
    };

    let resolution = (rail.segment_count() * rail.samples_per_segment.max(2)).max(2);
    let domain_end = curve.domain().end();
    let mut samples = Vec::with_capacity(resolution + 1);
    let mut last_position = curve.sample_clamped(0.0);
    let mut distance = 0.0;

    for index in 0..=resolution {
        let t = domain_end * (index as f32 / resolution as f32);
        let position = curve.sample_clamped(t);
        if index > 0 {
            distance += position.distance(last_position);
        }

        let tangent = curve.velocity(t).normalize_or_zero();
        samples.push(CachedRailSample {
            position,
            tangent,
            distance,
        });
        last_position = position;
    }

    normalize_zero_tangents(samples)
}

fn build_linear_samples(rail: &CinematicRail) -> Vec<CachedRailSample> {
    let subdivision = rail.samples_per_segment.max(1);
    let mut samples: Vec<CachedRailSample> = Vec::new();
    let mut distance = 0.0;
    let segment_count = rail.segment_count();

    for segment_index in 0..segment_count {
        let start = rail.points[segment_index];
        let end = if rail.closed {
            rail.points[(segment_index + 1) % rail.points.len()]
        } else {
            rail.points[segment_index + 1]
        };
        let tangent = (end - start).normalize_or_zero();
        let local_steps = if segment_index == 0 {
            0..=subdivision
        } else {
            1..=subdivision
        };

        for step in local_steps {
            let alpha = step as f32 / subdivision as f32;
            let position = start.lerp(end, alpha);
            if let Some(previous) = samples.last() {
                distance += position.distance(previous.position);
            }
            samples.push(CachedRailSample {
                position,
                tangent,
                distance,
            });
        }
    }

    samples
}

fn normalize_zero_tangents(mut samples: Vec<CachedRailSample>) -> Vec<CachedRailSample> {
    let len = samples.len();
    for index in 0..len {
        if samples[index].tangent.length_squared() > 0.0 {
            continue;
        }

        let prev = index.checked_sub(1).and_then(|prev| samples.get(prev));
        let next = samples.get(index + 1);
        let fallback = match (prev, next) {
            (Some(prev), Some(next)) => (next.position - prev.position).normalize_or_zero(),
            (Some(prev), None) => (samples[index].position - prev.position).normalize_or_zero(),
            (None, Some(next)) => (next.position - samples[index].position).normalize_or_zero(),
            (None, None) => Vec3::Z,
        };

        samples[index].tangent = if fallback.length_squared() > 0.0 {
            fallback
        } else {
            Vec3::Z
        };
    }

    samples
}

fn interpolate_samples(
    start: CachedRailSample,
    end: CachedRailSample,
    target_distance: f32,
    total_length: f32,
) -> RailSample {
    let span = (end.distance - start.distance).max(f32::EPSILON);
    let alpha = ((target_distance - start.distance) / span).clamp(0.0, 1.0);
    let tangent = start.tangent.lerp(end.tangent, alpha).normalize_or_zero();

    RailSample {
        position: start.position.lerp(end.position, alpha),
        tangent: if tangent.length_squared() > 0.0 {
            tangent
        } else {
            start.tangent
        },
        distance: target_distance,
        normalized: if total_length <= f32::EPSILON {
            0.0
        } else {
            target_distance / total_length
        },
    }
}

fn wrap_scalar(value: f32, max_value: f32, loop_mode: PlaybackLoopMode) -> f32 {
    if max_value <= f32::EPSILON {
        return 0.0;
    }

    match loop_mode {
        PlaybackLoopMode::Once => value.clamp(0.0, max_value),
        PlaybackLoopMode::Loop => value.rem_euclid(max_value),
        PlaybackLoopMode::PingPong => {
            let period = max_value * 2.0;
            let wrapped = value.rem_euclid(period);
            if wrapped > max_value {
                period - wrapped
            } else {
                wrapped
            }
        }
    }
}

#[cfg(test)]
#[path = "curve_tests.rs"]
mod tests;

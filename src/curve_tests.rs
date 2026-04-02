use super::*;

#[test]
fn normalized_sampling_reaches_end_of_open_path() {
    let rail = CinematicRail {
        kind: RailSplineKind::Linear,
        points: vec![Vec3::ZERO, Vec3::new(0.0, 0.0, 10.0)],
        closed: false,
        samples_per_segment: 8,
    };
    let cache = CinematicRailCache::rebuild(&rail);
    let end = cache.sample_normalized(1.0, PlaybackLoopMode::Once);
    assert_eq!(end.position, Vec3::new(0.0, 0.0, 10.0));
}

#[test]
fn distance_sampling_wraps_for_loop_and_ping_pong() {
    let rail = CinematicRail {
        kind: RailSplineKind::Linear,
        points: vec![Vec3::ZERO, Vec3::new(0.0, 0.0, 10.0)],
        closed: false,
        samples_per_segment: 8,
    };
    let cache = CinematicRailCache::rebuild(&rail);

    let looped = cache.sample_distance(12.0, PlaybackLoopMode::Loop);
    assert!((looped.position.z - 2.0).abs() < 0.0001);
    assert!((looped.normalized - 0.2).abs() < 0.0001);

    let ping_pong = cache.sample_distance(12.0, PlaybackLoopMode::PingPong);
    assert!((ping_pong.position.z - 8.0).abs() < 0.0001);
    assert!((ping_pong.normalized - 0.8).abs() < 0.0001);
}

#[test]
fn distance_traversal_samples_within_authored_window() {
    let rail = CinematicRail {
        kind: RailSplineKind::Linear,
        points: vec![Vec3::ZERO, Vec3::new(0.0, 0.0, 10.0)],
        closed: false,
        samples_per_segment: 8,
    };
    let cache = CinematicRailCache::rebuild(&rail);
    let traversal = RailTraversal {
        start: 2.0,
        end: 8.0,
        unit: RailProgressUnit::Distance,
        loop_mode: PlaybackLoopMode::Once,
    };

    let sample = traversal.sample(&cache, 0.5);
    assert!((sample.position.z - 5.0).abs() < 0.0001);
    assert!((sample.distance - 5.0).abs() < 0.0001);
}

use super::*;

#[test]
fn look_solver_uses_fallback_for_degenerate_target() {
    let fallback = Quat::from_rotation_y(0.5);
    let solved = solve_look_rotation(fallback, Vec3::ZERO, Vec3::ZERO, UpVectorMode::WorldY);
    assert_eq!(solved, fallback);
}

#[test]
fn blend_pose_interpolates_translation_rotation_and_fov() {
    let from = SolvedCameraPose {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        look_target: Vec3::NEG_Z,
        fov_y_radians: 0.5,
    };
    let to = SolvedCameraPose {
        translation: Vec3::new(10.0, 4.0, -2.0),
        rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        look_target: Vec3::new(2.0, 1.0, -4.0),
        fov_y_radians: 1.0,
    };

    let blended = blend_pose(&from, &to, 0.5);

    assert!(blended.translation.distance(Vec3::new(5.0, 2.0, -1.0)) < 0.0001);
    assert!(
        blended
            .rotation
            .angle_between(Quat::from_rotation_y(std::f32::consts::FRAC_PI_4))
            < 0.0001
    );
    assert!(blended.look_target.distance(Vec3::new(1.0, 0.5, -2.5)) < 0.0001);
    assert!((blended.fov_y_radians - 0.75).abs() < 0.0001);
}

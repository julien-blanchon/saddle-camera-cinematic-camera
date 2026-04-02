use super::*;

#[test]
fn blend_alpha_snaps_when_duration_is_zero() {
    let blend = CinematicBlend::instant();
    assert_eq!(blend.alpha(0.0), 1.0);
}

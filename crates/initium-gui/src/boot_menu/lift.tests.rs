use super::*;

#[test]
fn rises_and_reverses_from_the_current_position() {
    let now = Instant::now();
    let mut lift = Lift::default();
    lift.set_open(true, now);
    let interrupted = now + DURATION / 2;
    let offset = lift.offset(interrupted);
    assert!(offset < 0 && offset > TRAVEL as i32);

    lift.set_open(false, interrupted);
    assert_eq!(lift.offset(interrupted), offset);
    assert_eq!(lift.offset(interrupted + DURATION), 0);
}

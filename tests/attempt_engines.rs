//! Pins the shared attempt engines against a full-generality brute force.
//!
//! Three solvers restrict the moves they consider, for reasons that are
//! true but not obvious:
//!
//! * long-jump run-up freezes only "the `k` smallest dice" for each `k`,
//! * long-jump jump freezes only "the `j` largest dice" for each `j`,
//! * discus/javelin collapse freezable subsets to `(count, value-sum)`.
//!
//! Each is a claim that the discarded moves are never strictly better. A
//! Python brute force enumerating *every* subset of the rolled dice
//! reproduced the values below to nine decimal places, which is what
//! makes the restrictions safe. If someone widens or narrows a move set,
//! these numbers move and the test says so.

use dice_decathlon::disciplines::best_of_n::Attempt;
use dice_decathlon::disciplines::freeze::FreezeAttempt;
use dice_decathlon::disciplines::longjump::LongJumpAttempt;

/// Expected value of a single attempt played to maximise its own score.
fn own_score_ev(att: &dyn Attempt) -> f64 {
    att.solve(&|x| f64::from(x)).mean()
}

#[test]
fn longjump_attempt_matches_full_subset_enumeration() {
    let ev = own_score_ev(&LongJumpAttempt);
    assert!(
        (ev - 16.942_648_747).abs() < 1e-8,
        "long jump attempt EV {ev} != brute force 16.942648747"
    );
}

#[test]
fn discus_attempt_matches_full_subset_enumeration() {
    let g = |x: i32| f64::from(x);
    let ev = FreezeAttempt::new(&[2, 4, 6], 5, &g).solve().mean();
    assert!(
        (ev - 14.885_438_946).abs() < 1e-8,
        "discus attempt EV {ev} != brute force 14.885438946"
    );
}

#[test]
fn javelin_attempt_matches_full_subset_enumeration() {
    let g = |x: i32| f64::from(x);
    let ev = FreezeAttempt::new(&[1, 3, 5], 6, &g).solve().mean();
    assert!(
        (ev - 15.218_094_840).abs() < 1e-8,
        "javelin attempt EV {ev} != brute force 15.218094840"
    );
}

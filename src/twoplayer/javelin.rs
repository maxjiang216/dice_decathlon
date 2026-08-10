//! The Javelin: six dice, only **odd** faces may be frozen, best of
//! three attempts.
//!
//! Six dice but a ceiling of 30, the same as the discus with five —
//! because a javelin die is worth at most 5.

use super::best_of_three;
use super::compress::Stats;
use super::freeze::Attempt;
use super::Axis;

/// The javelin's attempt engine.
pub fn attempt() -> Attempt {
    Attempt::new(6, [1, 3, 5])
}

/// Win probability of the player who moves first, per difference.
pub fn solve_first_mover(
    axis: Axis,
    after: &(dyn Fn(i32) -> f64 + Sync),
) -> Vec<f64> {
    best_of_three::solve_first_mover(&attempt(), axis, after)
}

/// Policy storage statistics.
pub fn measure(axis: Axis, after: &(dyn Fn(i32) -> f64 + Sync)) -> Stats {
    best_of_three::measure(&attempt(), axis, after)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::twoplayer::freeze::MAX_SCORE;

    /// Under a payoff that only counts points, best-of-three javelin must
    /// reproduce the solo expected value in `tests/disciplines.rs`.
    #[test]
    fn reduces_to_the_solo_solver_under_a_linear_payoff() {
        assert!(
            (best_of_three::solo_expected_value(&attempt()) - 22.251_507_501)
                .abs()
                < 1e-9
        );
    }

    /// The javelin cannot score 27 or 29: an odd total needs an odd
    /// number of dice, and five is the most that fit under 30.
    #[test]
    fn odd_totals_above_twenty_five_are_unreachable() {
        let a = attempt();
        assert!(!a.reachable_scores[27]);
        assert!(!a.reachable_scores[29]);
        assert!(a.reachable_scores[28]);
        assert_eq!(
            a.reachable_scores[..=MAX_SCORE as usize]
                .iter()
                .filter(|r| **r)
                .count(),
            29
        );
    }
}

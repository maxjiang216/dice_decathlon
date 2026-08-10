//! The Discus: five dice, only **even** faces may be frozen, best of
//! three attempts.
//!
//! Every attainable score is even, so the banked best takes 16 values
//! rather than 31 — the tightest banked-best axis of any throwing event.

use super::best_of_three;
use super::compress::Stats;
use super::freeze::Attempt;
use super::Axis;

/// The discus's attempt engine.
pub fn attempt() -> Attempt {
    Attempt::new(5, [2, 4, 6])
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

    /// Under a payoff that only counts points, best-of-three discus must
    /// reproduce the solo expected value in `tests/disciplines.rs`.
    #[test]
    fn reduces_to_the_solo_solver_under_a_linear_payoff() {
        assert!(
            (best_of_three::solo_expected_value(&attempt()) - 22.317_089_285)
                .abs()
                < 1e-9
        );
    }

    /// Only even faces freeze, so only even totals exist.
    #[test]
    fn every_score_is_even() {
        let a = attempt();
        for s in 0..=MAX_SCORE as usize {
            assert_eq!(a.reachable_scores[s], s % 2 == 0, "score {s}");
        }
    }
}

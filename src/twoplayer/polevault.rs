//! The Pole Vault: bars from 10 to 48, three jumps each, choose how many
//! dice to throw and any one fails the jump.

use super::compress::Stats;
use super::ladder::Ladder;
use super::{ladder, Axis};
use crate::disciplines::polevault::clear_probabilities;

/// The pole vault's ladder.
pub fn ladder() -> Ladder {
    Ladder::new(&clear_probabilities())
}

/// Win probability of the player who moves first, per difference.
pub fn solve_first_mover(
    axis: Axis,
    after: &(dyn Fn(i32) -> f64 + Sync),
) -> Vec<f64> {
    ladder::solve_first_mover(&ladder(), axis, after)
}

/// Policy storage statistics.
pub fn measure(axis: Axis, after: &(dyn Fn(i32) -> f64 + Sync)) -> Stats {
    ladder::measure(&ladder(), axis, after)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::twoplayer::ladder::solo_expected_value;

    /// Matches the solo expected value in `tests/disciplines.rs`, and the
    /// EV player really does decline some bars — see the counterexample
    /// recorded in `worklog/RULES-CHECKLIST.md`.
    #[test]
    fn matches_the_solo_solver_and_does_skip() {
        let (ev, skips) = solo_expected_value(&ladder());
        assert!((ev - 17.277_634_238).abs() < 1e-9, "pole vault EV = {ev}");
        assert!(skips > 0, "expected the EV ladder to decline some bar");
    }
}

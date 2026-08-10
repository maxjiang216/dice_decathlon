//! The High Jump: bars from 10 to 30, three jumps each, all five dice
//! every time and no die count to choose.
//!
//! Structurally identical to the pole vault but far safer at the bottom
//! of the ladder: the opening bar of 10 clears with probability
//! 0.999996, against the pole vault's 0.864.
//!
//! **The high jump has no decisions.** Its only choice is attempt or
//! skip, and attempting always wins, so the policy is the constant
//! "attempt" — under expected value *and* under win probability, which
//! measures zero deviating states. Two things compound to make that so:
//! skipping is never the EV play here, and the event sits six disciplines
//! from the end, so with that much swing still to come the value function
//! is near-linear across the ±30 the event can move and win probability
//! agrees with expected value regardless.
//!
//! So its tiny policy is not a compression result; there is simply
//! nothing to store. Deviation grows as the game shortens — 0.00% here,
//! 1.03% for the pole vault with two events left, 6.32% for the 1500m.

use super::compress::Stats;
use super::ladder::Ladder;
use super::{ladder, Axis};
use crate::disciplines::highjump::clear_probabilities;

/// The high jump's ladder.
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

    /// Matches the solo expected value in `tests/disciplines.rs`.
    ///
    /// `worklog/RULES-CHECKLIST.md` flags that the pole vault's skip
    /// counterexample was never tested here, where low bars are nearly
    /// free. Whichever way it falls, record it rather than assume it.
    #[test]
    fn matches_the_solo_solver_and_records_whether_it_skips() {
        let (ev, skips) = solo_expected_value(&ladder());
        assert!((ev - 19.263_445_441).abs() < 1e-9, "high jump EV = {ev}");
        assert_eq!(
            skips, 0,
            "the high-jump EV ladder was believed never to skip; it declined \
             {skips} bars, so RULES-CHECKLIST.md needs updating"
        );
    }
}

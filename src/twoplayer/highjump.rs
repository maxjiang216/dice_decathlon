//! The High Jump: bars from 10 to 30, three jumps each, all five dice
//! every time and no die count to choose.
//!
//! Structurally identical to the pole vault but far safer at the bottom
//! of the ladder: the opening bar of 10 clears with probability
//! 0.999996, against the pole vault's 0.864.

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

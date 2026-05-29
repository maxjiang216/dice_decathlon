//! 100 Metres: 2 sets of 4 dice, 5 shared rerolls, sixes subtract.

use super::reroll_sets;
use crate::dice::score_six_penalty;
use crate::policy::Solved;

pub fn solve() -> Solved {
    Solved {
        key: "100m",
        name: "100 Metres",
        dist: reroll_sets::solve(2, 4, 5, score_six_penalty),
    }
}

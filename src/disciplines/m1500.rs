//! 1500 Metres: 8 sets of 1 die, 5 shared rerolls, sixes subtract.

use super::reroll_sets;
use crate::dice::score_six_penalty;
use crate::policy::Solved;

pub fn solve() -> Solved {
    Solved {
        key: "1500m",
        name: "1500 Metres",
        dist: reroll_sets::solve(8, 1, 5, score_six_penalty),
    }
}

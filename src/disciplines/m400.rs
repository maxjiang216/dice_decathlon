//! 400 Metres: 4 sets of 2 dice, 5 shared rerolls, sixes subtract.

use super::reroll_sets;
use crate::dice::score_six_penalty;
use crate::policy::Solved;

pub fn solve() -> Solved {
    Solved {
        key: "400m",
        name: "400 Metres",
        dist: reroll_sets::solve(4, 2, 5, score_six_penalty),
    }
}

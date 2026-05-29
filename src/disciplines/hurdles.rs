//! 110 Metre Hurdles: 5 dice, up to 5 all-dice rerolls, plain sum.
//!
//! Modelled as a single set of 5 dice (so freezing is the only way to
//! stop and every reroll rethrows all five), scored by face total with
//! no six penalty.

use super::reroll_sets;
use crate::dice::sum;
use crate::policy::Solved;

pub fn solve() -> Solved {
    Solved {
        key: "110mh",
        name: "110 Metre Hurdles",
        dist: reroll_sets::solve(1, 5, 5, sum),
    }
}

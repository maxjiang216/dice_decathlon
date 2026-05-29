//! Javelin: 6 dice, freeze odd faces only, best of 3 attempts.

use super::best_of_n::{best_of_n, Attempt};
use super::freeze::FreezeAttempt;
use crate::dp::Dist;
use crate::policy::Solved;

struct Javelin;

impl Attempt for Javelin {
    fn score_hi(&self) -> i32 {
        30 // six dice all showing 5
    }

    fn solve(&self, g: &dyn Fn(i32) -> f64) -> Dist {
        FreezeAttempt::new(&[1, 3, 5], 6, g).solve()
    }
}

pub fn solve() -> Solved {
    Solved {
        key: "javelin",
        name: "Javelin",
        dist: best_of_n(3, &Javelin),
    }
}

//! Discus: 5 dice, freeze even faces only, best of 3 attempts.

use super::best_of_n::{best_of_n, Attempt};
use super::freeze::FreezeAttempt;
use crate::dp::Dist;
use crate::policy::Solved;

struct Discus;

impl Attempt for Discus {
    fn score_hi(&self) -> i32 {
        30 // five dice all showing 6
    }

    fn solve(&self, g: &dyn Fn(i32) -> f64) -> Dist {
        FreezeAttempt::new(&[2, 4, 6], 5, g).solve()
    }
}

pub fn solve() -> Solved {
    Solved {
        key: "discus",
        name: "Discus",
        dist: best_of_n(3, &Discus),
    }
}

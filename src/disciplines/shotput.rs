//! Shot Put: up to 8 dice thrown one at a time, best of 3 attempts.
//!
//! Throw dice one by one, stopping whenever you like; rolling a 1 fouls
//! the attempt (score 0). Score is the sum of all thrown dice.

use super::best_of_n::{best_of_n, better_for, Attempt};
use crate::dp::Dist;
use crate::policy::Solved;
use std::collections::HashMap;

const MAX_DICE: u8 = 8;
const MAX_SCORE: i32 = MAX_DICE as i32 * 6; // 48 (no ones)

struct ShotPut;

struct AttemptSolver<'a> {
    g: &'a dyn Fn(i32) -> f64,
    memo: HashMap<(u8, i32), Dist>,
}

impl AttemptSolver<'_> {
    /// Distribution of the final score having already thrown `thrown`
    /// dice (no 1 yet) for a running sum of `cur`.
    fn play(&mut self, thrown: u8, cur: i32) -> Dist {
        if thrown == MAX_DICE {
            return Dist::point(cur);
        }
        let key = (thrown, cur);
        if let Some(d) = self.memo.get(&key) {
            return d.clone();
        }

        // Throw the next die: 1 fouls, faces 2..=6 advance.
        let mut throw = Dist::default();
        throw.add(0, 1.0 / 6.0); // rolled a 1 -> foul
        for face in 2..=6 {
            let child = self.play(thrown + 1, cur + face);
            throw.mix_in(&child, 1.0 / 6.0);
        }

        // At the very start a die must be thrown; otherwise stopping
        // (banking `cur`) competes with throwing again.
        let result = if thrown == 0 {
            throw
        } else {
            better_for(Dist::point(cur), throw, &self.g)
        };

        self.memo.insert(key, result.clone());
        result
    }
}

impl Attempt for ShotPut {
    fn score_hi(&self) -> i32 {
        MAX_SCORE
    }

    fn solve(&self, g: &dyn Fn(i32) -> f64) -> Dist {
        let mut solver = AttemptSolver {
            g,
            memo: HashMap::new(),
        };
        solver.play(0, 0)
    }
}

pub fn solve() -> Solved {
    Solved {
        key: "shotput",
        name: "Shot Put",
        dist: best_of_n(3, &ShotPut),
    }
}

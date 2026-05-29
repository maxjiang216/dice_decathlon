//! Generic solver for the "freeze a set, optionally reroll" running
//! events: 100m, 400m, 1500m and the 110m Hurdles.
//!
//! The eight (or five) dice are split into `sets` groups of `per_set`
//! dice. Each group is thrown once for free and may then be rerolled,
//! drawing from a shared pool of `rerolls`. Freezing a group locks its
//! score; once every group is frozen the scores are summed.
//!
//! All four events share this shape and differ only in the geometry
//! (`sets` x `per_set`) and the scoring rule, so they are expressed as
//! configurations of a single dynamic program that carries the exact
//! score distribution of optimal play.

use crate::dice::{count_outcomes, total_outcomes, Counts};
use crate::dp::{better, Dist};
use std::collections::HashMap;

/// Scoring rule applied to one frozen set of dice.
pub type ScoreFn = fn(&Counts) -> i32;

struct Solver {
    sets: u8,
    score_fn: ScoreFn,
    outcomes: Vec<(Counts, u64)>,
    total: f64,
    memo: HashMap<(u8, u8, Counts, i32), Dist>,
}

impl Solver {
    fn new(sets: u8, per_set: u32, score_fn: ScoreFn) -> Self {
        Self {
            sets,
            score_fn,
            outcomes: count_outcomes(per_set),
            total: total_outcomes(per_set) as f64,
            memo: HashMap::new(),
        }
    }

    /// Distribution of the final total given the current set's roll.
    fn solve(
        &mut self,
        set_idx: u8,
        rerolls: u8,
        current: Counts,
        acc: i32,
    ) -> Dist {
        let key = (set_idx, rerolls, current, acc);
        if let Some(d) = self.memo.get(&key) {
            return d.clone();
        }

        // Freeze: lock this set's score, advance to the next set's
        // free throw (or finish if this was the last set).
        let frozen_acc = acc + (self.score_fn)(&current);
        let freeze = if set_idx + 1 == self.sets {
            Dist::point(frozen_acc)
        } else {
            self.average_over_throw(set_idx + 1, rerolls, frozen_acc)
        };

        // Reroll: rethrow this set, spending one from the shared pool.
        let result = if rerolls > 0 {
            let reroll = self.average_over_throw(set_idx, rerolls - 1, acc);
            better(freeze, reroll)
        } else {
            freeze
        };

        self.memo.insert(key, result.clone());
        result
    }

    /// Weighted average of `solve` across one throw of `set_idx`.
    fn average_over_throw(
        &mut self,
        set_idx: u8,
        rerolls: u8,
        acc: i32,
    ) -> Dist {
        let outcomes = self.outcomes.clone();
        let mut dist = Dist::default();
        for (oc, w) in outcomes {
            let child = self.solve(set_idx, rerolls, oc, acc);
            dist.mix_in(&child, w as f64 / self.total);
        }
        dist
    }
}

/// Solve a reroll-set event and return its final-score distribution.
pub fn solve(sets: u8, per_set: u32, rerolls: u8, score_fn: ScoreFn) -> Dist {
    let mut solver = Solver::new(sets, per_set, score_fn);
    solver.average_over_throw(0, rerolls, 0)
}

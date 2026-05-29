//! Shared dynamic program for the "rising bar" events (High Jump and
//! Pole Vault).
//!
//! The bar starts at 10 and rises in steps of 2. At each height you
//! either skip it or attempt it; an attempt clears the height with some
//! probability `p_clear(h)` (already folded over the three jumps), in
//! which case the bar becomes your new best. Three failed jumps end the
//! event with your previous best. Score is the highest height cleared.

use crate::dp::{better, Dist};
use std::collections::HashMap;

const START: i32 = 10;
const STEP: i32 = 2;

struct Solver<'a> {
    p_clear: &'a dyn Fn(i32) -> f64,
    max_height: i32,
    memo: HashMap<(i32, i32), Dist>,
}

impl Solver<'_> {
    /// Distribution of the final score when facing height `h` with
    /// `best` already cleared.
    fn solve(&mut self, h: i32, best: i32) -> Dist {
        if h > self.max_height {
            return Dist::point(best);
        }
        let key = (h, best);
        if let Some(d) = self.memo.get(&key) {
            return d.clone();
        }

        let skip = self.solve(h + STEP, best);

        let p = (self.p_clear)(h);
        let cleared = self.solve(h + STEP, h);
        let mut attempt = Dist::default();
        attempt.mix_in(&cleared, p); // cleared -> bar rises to h
        attempt.add(best, 1.0 - p); // three misses -> stop at `best`

        let result = better(skip, attempt);
        self.memo.insert(key, result.clone());
        result
    }
}

/// Solve a rising-bar event given the per-height clear probability and
/// the greatest clearable height.
pub fn solve(p_clear: &dyn Fn(i32) -> f64, max_height: i32) -> Dist {
    let mut solver = Solver {
        p_clear,
        max_height,
        memo: HashMap::new(),
    };
    solver.solve(START, 0)
}

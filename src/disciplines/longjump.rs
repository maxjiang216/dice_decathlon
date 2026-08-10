//! Long Jump: best of 3 attempts, each a run-up followed by a jump.
//!
//! Run-up: throw all unfrozen dice, freeze at least one each throw; if
//! the frozen total ever exceeds 8 the attempt fouls (step over). Stop
//! with total <= 8 to jump. Only the *number* of dice carried into the
//! jump matters (they are picked up and rethrown).
//!
//! Jump: throw the carried dice, freeze at least one each throw until
//! all are frozen. Score is the sum of the jump dice.

use super::best_of_n::{best_of_n, better_for, Attempt};
use crate::dice::{count_outcomes, total_outcomes, Counts};
use crate::dp::Dist;
use crate::policy::Solved;
use std::collections::HashMap;

const N_DICE: u32 = 5;
const RUNUP_LIMIT: i32 = 8;

/// One long-jump attempt (run-up then jump), exposed so tests can
/// solve a single attempt in isolation.
pub struct LongJumpAttempt;

struct AttemptSolver<'a> {
    g: &'a dyn Fn(i32) -> f64,
    jump_memo: HashMap<(u32, i32), Dist>,
    runup_memo: HashMap<(i32, u32), Dist>,
    outcomes: HashMap<u32, Vec<(Counts, u64)>>,
}

impl AttemptSolver<'_> {
    fn outcomes_for(&mut self, n: u32) -> Vec<(Counts, u64)> {
        self.outcomes
            .entry(n)
            .or_insert_with(|| count_outcomes(n))
            .clone()
    }

    /// Jump phase: `r` dice still to freeze onto an accumulated sum
    /// `acc`. Returns the score distribution of optimal play.
    fn jump(&mut self, r: u32, acc: i32) -> Dist {
        if r == 0 {
            return Dist::point(acc);
        }
        let key = (r, acc);
        if let Some(d) = self.jump_memo.get(&key) {
            return d.clone();
        }
        let total = total_outcomes(r) as f64;
        let outcomes = self.outcomes_for(r);

        let mut dist = Dist::default();
        for (counts, w) in outcomes {
            // Freeze the top `j` dice (highest sum for that count), then
            // recurse; pick the count that maximises E[g].
            let mut best: Option<Dist> = None;
            for j in 1..=r {
                let gained = sum_largest(&counts, j);
                let child = self.jump(r - j, acc + gained);
                best = Some(match best {
                    None => child,
                    Some(prev) => better_for(prev, child, &self.g),
                });
            }
            dist.mix_in(&best.unwrap(), w as f64 / total);
        }

        self.jump_memo.insert(key, dist.clone());
        dist
    }

    /// Run-up decision point: `nf` dice frozen with total `fs`. Choose
    /// to jump now or throw the remaining dice and freeze more.
    fn runup(&mut self, fs: i32, nf: u32) -> Dist {
        let key = (fs, nf);
        if let Some(d) = self.runup_memo.get(&key) {
            return d.clone();
        }

        // Option A: jump now with the dice frozen so far.
        let mut best: Option<Dist> = if nf >= 1 {
            Some(self.jump(nf, 0))
        } else {
            None
        };

        // Option B: throw the unfrozen dice and freeze at least one.
        if nf < N_DICE {
            let throw = self.throw_in_runup(fs, nf);
            best = Some(match best {
                None => throw,
                Some(prev) => better_for(prev, throw, &self.g),
            });
        }

        let result = best.unwrap();
        self.runup_memo.insert(key, result.clone());
        result
    }

    fn throw_in_runup(&mut self, fs: i32, nf: u32) -> Dist {
        let u = N_DICE - nf;
        let total = total_outcomes(u) as f64;
        let outcomes = self.outcomes_for(u);

        let mut dist = Dist::default();
        for (counts, w) in outcomes {
            // For each possible freeze count keep the lowest dice (to
            // stay under the limit); pick the count maximising E[g].
            let mut best: Option<Dist> = None;
            for k in 1..=u {
                let added = sum_smallest(&counts, k);
                let fs2 = fs + added;
                let outcome = if fs2 > RUNUP_LIMIT {
                    Dist::point(0) // stepped over -> foul
                } else if nf + k == N_DICE {
                    self.jump(N_DICE, 0)
                } else {
                    self.runup(fs2, nf + k)
                };
                best = Some(match best {
                    None => outcome,
                    Some(prev) => better_for(prev, outcome, &self.g),
                });
            }
            dist.mix_in(&best.unwrap(), w as f64 / total);
        }
        dist
    }
}

impl Attempt for LongJumpAttempt {
    fn score_hi(&self) -> i32 {
        (N_DICE * 6) as i32 // 30
    }

    fn solve(&self, g: &dyn Fn(i32) -> f64) -> Dist {
        let mut solver = AttemptSolver {
            g,
            jump_memo: HashMap::new(),
            runup_memo: HashMap::new(),
            outcomes: HashMap::new(),
        };
        solver.runup(0, 0)
    }
}

/// Sum of the `k` smallest dice described by `counts`.
fn sum_smallest(counts: &Counts, k: u32) -> i32 {
    let mut remaining = k;
    let mut total = 0;
    for face in 1..=6 {
        let take = remaining.min(counts[face]);
        total += take as i32 * face as i32;
        remaining -= take;
        if remaining == 0 {
            break;
        }
    }
    total
}

/// Sum of the `j` largest dice described by `counts`.
fn sum_largest(counts: &Counts, j: u32) -> i32 {
    let mut remaining = j;
    let mut total = 0;
    for face in (1..=6).rev() {
        let take = remaining.min(counts[face]);
        total += take as i32 * face as i32;
        remaining -= take;
        if remaining == 0 {
            break;
        }
    }
    total
}

pub fn solve() -> Solved {
    Solved {
        key: "longjump",
        name: "Long Jump",
        dist: best_of_n(3, &LongJumpAttempt),
    }
}

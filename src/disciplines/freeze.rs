//! Shared "freeze dice of a chosen parity" attempt used by Discus
//! (even faces) and Javelin (odd faces).
//!
//! Throw all unfrozen dice, then freeze at least one die of the allowed
//! parity; if none of the allowed parity appears the attempt fouls
//! (score 0). You may stop after any freeze; the attempt also ends when
//! every die is frozen. Score is the sum of frozen dice.

use super::best_of_n::better_for;
use crate::dice::{count_outcomes, total_outcomes, Counts};
use crate::dp::Dist;
use std::collections::HashMap;

/// Attempt solver parameterised by the freezable faces.
pub struct FreezeAttempt<'a> {
    pub faces: &'a [u8],
    pub n_dice: u32,
    pub g: &'a dyn Fn(i32) -> f64,
    memo: HashMap<(u32, i32), Dist>,
    outcomes: HashMap<u32, Vec<(Counts, u64)>>,
}

impl<'a> FreezeAttempt<'a> {
    pub fn new(
        faces: &'a [u8],
        n_dice: u32,
        g: &'a dyn Fn(i32) -> f64,
    ) -> Self {
        FreezeAttempt {
            faces,
            n_dice,
            g,
            memo: HashMap::new(),
            outcomes: HashMap::new(),
        }
    }

    /// Distribution of the final score, then the start position.
    pub fn solve(&mut self) -> Dist {
        self.roll(self.n_dice, 0)
    }

    fn outcomes_for(&mut self, u: u32) -> Vec<(Counts, u64)> {
        self.outcomes
            .entry(u)
            .or_insert_with(|| count_outcomes(u))
            .clone()
    }

    /// Throw `u` unfrozen dice given a frozen sum of `fs`, play on
    /// optimally, and return the resulting score distribution.
    fn roll(&mut self, u: u32, fs: i32) -> Dist {
        let key = (u, fs);
        if let Some(d) = self.memo.get(&key) {
            return d.clone();
        }
        let total = total_outcomes(u) as f64;
        let outcomes = self.outcomes_for(u);

        let mut dist = Dist::default();
        for (counts, w) in outcomes {
            let best = self.best_response(u, fs, &counts);
            dist.mix_in(&best, w as f64 / total);
        }

        self.memo.insert(key, dist.clone());
        dist
    }

    /// Best score distribution after seeing `counts` on the thrown
    /// dice: choose which freezable dice to keep, then stop or continue.
    fn best_response(&mut self, u: u32, fs: i32, counts: &Counts) -> Dist {
        let choices = self.freeze_choices(counts);
        if choices.is_empty() {
            return Dist::point(0); // no freezable die -> foul
        }
        let mut best: Option<Dist> = None;
        for (k, add) in choices {
            let fs2 = fs + add;
            let u2 = u - k;
            let choice = if u2 == 0 {
                Dist::point(fs2) // all dice frozen -> attempt ends
            } else {
                better_for(Dist::point(fs2), self.roll(u2, fs2), &self.g)
            };
            best = Some(match best {
                None => choice,
                Some(prev) => better_for(prev, choice, &self.g),
            });
        }
        best.unwrap()
    }

    /// Distinct `(count, value-sum)` freezes of at least one die whose
    /// face is in `faces`.
    fn freeze_choices(&self, counts: &Counts) -> Vec<(u32, i32)> {
        let mut acc = vec![(0u32, 0i32)];
        for &f in self.faces {
            let avail = counts[f as usize];
            let mut next = Vec::new();
            for &(k, add) in &acc {
                for take in 0..=avail {
                    next.push((k + take, add + take as i32 * f as i32));
                }
            }
            acc = next;
        }
        acc.retain(|&(k, _)| k >= 1);
        acc.sort_unstable();
        acc.dedup();
        acc
    }
}

//! Two-player solver for the "freeze a set, optionally reroll" events:
//! 100m, 400m, 1500m and the 110m Hurdles.
//!
//! A rethrow picks up *every* die in the set, so the only choice is
//! freeze-all versus reroll-all — which means two rolls with the same
//! score are the same position, and the roll collapses to a scalar. That
//! is why [`RerollSets::set_scores`] is a score distribution rather than
//! a dice enumeration.
//!
//! The state is `(set index, rerolls left, this set's score, n)`, where
//! `n` is the running point difference with the sets frozen so far
//! already folded in. Scores add, so nothing else need be carried.

use super::Axis;

/// Where in an event a decision is being taken.
#[derive(Clone, Copy)]
pub struct Decision {
    /// Index of the set being decided.
    pub set: usize,
    /// Rerolls still in the shared pool.
    pub r: usize,
    /// Score showing on the set just thrown.
    pub score: i32,
    /// Running difference with earlier sets already folded in.
    pub n: i32,
}
use rayon::prelude::{IntoParallelIterator, ParallelIterator};

/// Geometry and scoring of one reroll-set event.
pub struct RerollSets {
    /// Number of sets thrown in sequence.
    pub sets: u8,
    /// Rerolls shared across all sets.
    pub rerolls: u8,
    /// `(score, weight)` for one throw of a set; weights sum to `total`.
    pub set_scores: Vec<(i32, u64)>,
    /// Sum of the weights in `set_scores`.
    pub total: u64,
}

impl RerollSets {
    /// Enumerate the distinct scores of one throw of `per_set` dice,
    /// with the number of ordered rolls producing each.
    ///
    /// `six_penalty` selects the running events' rule where a six
    /// subtracts six instead of adding it.
    pub fn set_score_distribution(
        per_set: u32,
        six_penalty: bool,
    ) -> (Vec<(i32, u64)>, u64) {
        let mut counts = std::collections::BTreeMap::<i32, u64>::new();
        let mut stack = vec![(0u32, 0i32)];
        while let Some((used, score)) = stack.pop() {
            if used == per_set {
                *counts.entry(score).or_default() += 1;
                continue;
            }
            for face in 1..=6i32 {
                let v = if six_penalty && face == 6 { -6 } else { face };
                stack.push((used + 1, score + v));
            }
        }
        let total = counts.values().sum();
        (counts.into_iter().collect(), total)
    }

    /// Win probability at the start of the event, for every difference
    /// on `axis`.
    ///
    /// `terminal` maps the difference after the event's last set is
    /// frozen to a win probability — for the player who moves second in
    /// the final event that is simply "am I ahead", and otherwise it is
    /// the value function of everything still to come.
    ///
    /// Parallel over the difference axis: within one
    /// `(set, rerolls)` layer every difference is an independent
    /// problem, and the layers are ordered so each depends only on ones
    /// already complete.
    /// As [`RerollSets::solve`], but keeps every layer so a policy can
    /// be read back off it.
    pub fn solve_layers(
        &self,
        axis: Axis,
        terminal: &(dyn Fn(i32) -> f64 + Sync),
    ) -> Vec<Vec<Vec<f64>>> {
        let sets = self.sets as usize;
        let nr = self.rerolls as usize + 1;
        let width = axis.len();
        let denom = self.total as f64;

        // layer[set][r] is the value before throwing `set`, holding `r`
        // rerolls, indexed by difference.
        let mut layer: Vec<Vec<Vec<f64>>> = vec![vec![Vec::new(); nr]; sets];

        for set in (0..sets).rev() {
            for r in 0..nr {
                let next = if set + 1 < sets {
                    Some(&layer[set + 1][r])
                } else {
                    None
                };
                // Rerolling costs one from the shared pool and rethrows
                // this same set, so it reads the layer computed on the
                // previous iteration of `r`.
                let cheaper = if r > 0 {
                    Some(&layer[set][r - 1])
                } else {
                    None
                };

                let values: Vec<f64> = (0..width)
                    .into_par_iter()
                    .map(|i| {
                        let n = axis.at(i);
                        let reroll = cheaper.map(|c| c[i]);
                        let mut acc = 0.0;
                        for &(score, weight) in &self.set_scores {
                            let after = n + score;
                            let freeze = next.map_or_else(
                                || terminal(after),
                                |t| t[axis.idx(after)],
                            );
                            let best =
                                reroll.map_or(freeze, |rr| freeze.max(rr));
                            acc += weight as f64 * best;
                        }
                        acc / denom
                    })
                    .collect();
                layer[set][r] = values;
            }
        }

        layer
    }

    /// Win probability at the start of the event, for every difference
    /// on `axis`.
    pub fn solve(
        &self,
        axis: Axis,
        terminal: &(dyn Fn(i32) -> f64 + Sync),
    ) -> Vec<f64> {
        let layers = self.solve_layers(axis, terminal);
        layers[0][self.rerolls as usize].clone()
    }

    /// Value of freezing, and of rerolling, at one decision point.
    ///
    /// Rerolling is unavailable with an empty pool, reported as
    /// negative infinity so it never wins a comparison.
    pub fn branch_values(
        &self,
        layers: &[Vec<Vec<f64>>],
        axis: Axis,
        terminal: &dyn Fn(i32) -> f64,
        at: Decision,
    ) -> (f64, f64) {
        let Decision { set, r, score, n } = at;
        let after = n + score;
        let freeze = if set + 1 < self.sets as usize {
            layers[set + 1][r][axis.idx(after)]
        } else {
            terminal(after)
        };
        let reroll = if r > 0 {
            layers[set][r - 1][axis.idx(n)]
        } else {
            f64::NEG_INFINITY
        };
        (freeze, reroll)
    }
}

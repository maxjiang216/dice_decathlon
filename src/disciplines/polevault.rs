//! Pole Vault: three jumps per height, choosing the dice count freely.
//!
//! On each jump you pick how many dice to throw (1-8); any 1 fails the
//! jump, otherwise you clear if the total reaches the bar. Each jump
//! independently uses the dice count that maximises the single-jump
//! clear probability.

use super::heights;
use crate::policy::Solved;

const MAX_DICE: u32 = 8;
const JUMPS_PER_HEIGHT: i32 = 3;
const MAX_HEIGHT: i32 = (MAX_DICE * 6) as i32; // 48

/// Counts of ordered length-`k` sequences over faces 2..=6 by total.
fn no_one_sum_counts(k: u32) -> Vec<u64> {
    let mut counts = vec![1u64]; // k = 0: empty sequence, sum 0
    for _ in 0..k {
        let mut next = vec![0u64; counts.len() + 6];
        for (s, &c) in counts.iter().enumerate() {
            if c == 0 {
                continue;
            }
            for face in 2..=6usize {
                next[s + face] += c;
            }
        }
        counts = next;
    }
    counts
}

/// Single-jump clear probability with `k` dice at height `h`.
fn clear_prob(k: u32, h: i32) -> f64 {
    let counts = no_one_sum_counts(k);
    let favourable: u64 = counts
        .iter()
        .enumerate()
        .filter(|(s, _)| *s as i32 >= h)
        .map(|(_, &c)| c)
        .sum();
    favourable as f64 / 6u64.pow(k) as f64
}

/// Best single-jump clear probability over all dice counts.
fn best_clear_prob(h: i32) -> f64 {
    (1..=MAX_DICE).map(|k| clear_prob(k, h)).fold(0.0, f64::max)
}

/// Probability of clearing each height, given three jumps at it and the
/// best die count for that bar.
///
/// Returned as `(height, probability)` from 10 upwards in steps of two.
/// The whole dice model of the event lives in this table: clearing pays
/// exactly the bar height, so each jump is a plain Bernoulli trial and
/// nothing carries between jumps.
pub fn clear_probabilities() -> Vec<(i32, f64)> {
    (10..=MAX_HEIGHT)
        .step_by(2)
        .map(|h| {
            let p = best_clear_prob(h);
            (h, 1.0 - (1.0 - p).powi(JUMPS_PER_HEIGHT))
        })
        .collect()
}

pub fn solve() -> Solved {
    let clear = |h: i32| {
        let p = best_clear_prob(h);
        1.0 - (1.0 - p).powi(JUMPS_PER_HEIGHT)
    };
    Solved {
        key: "polevault",
        name: "Pole Vault",
        dist: heights::solve(&clear, MAX_HEIGHT),
    }
}

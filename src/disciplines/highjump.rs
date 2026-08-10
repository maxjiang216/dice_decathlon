//! High Jump: 5 dice, three jumps per height, clear if the total of all
//! five dice reaches the bar.

use super::heights;
use crate::dice::{count_outcomes, sum, total_outcomes};
use crate::policy::Solved;

const N_DICE: u32 = 5;
const JUMPS_PER_HEIGHT: i32 = 3;

/// Probability that five dice total at least `h`.
fn clear_prob_single(h: i32) -> f64 {
    let total = total_outcomes(N_DICE) as f64;
    let favourable: u64 = count_outcomes(N_DICE)
        .iter()
        .filter(|(c, _)| sum(c) >= h)
        .map(|(_, w)| *w)
        .sum();
    favourable as f64 / total
}

/// Probability of clearing each height, given three jumps at it.
///
/// Returned as `(height, probability)` from 10 upwards in steps of two.
/// Unlike the pole vault there is no die count to choose: every jump
/// throws all five dice.
pub fn clear_probabilities() -> Vec<(i32, f64)> {
    (10..=(N_DICE * 6) as i32)
        .step_by(2)
        .map(|h| {
            let p = clear_prob_single(h);
            (h, 1.0 - (1.0 - p).powi(JUMPS_PER_HEIGHT))
        })
        .collect()
}

pub fn solve() -> Solved {
    let clear = |h: i32| {
        let p = clear_prob_single(h);
        1.0 - (1.0 - p).powi(JUMPS_PER_HEIGHT)
    };
    Solved {
        key: "highjump",
        name: "High Jump",
        dist: heights::solve(&clear, (N_DICE * 6) as i32),
    }
}

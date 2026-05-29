//! Dice-roll enumeration shared by every discipline solver.
//!
//! Rolling `n` six-sided dice has `6^n` equally likely ordered
//! outcomes. We enumerate the *canonical* outcomes (sorted face
//! tuples, or equivalently face-count vectors) together with their
//! multinomial multiplicity so an expected value can be computed as a
//! correctly weighted average instead of iterating every ordered roll.

/// Number of ordered outcomes when rolling `n` dice: `6^n`.
pub const fn total_outcomes(n: u32) -> u64 {
    6u64.pow(n)
}

/// A rolled hand described by how many dice show each face.
///
/// `counts[v]` is the number of dice showing face `v` for `v` in
/// `1..=6`; index `0` is unused so faces can be indexed directly.
pub type Counts = [u32; 7];

/// Enumerate all distinct face-count outcomes of rolling `n` dice,
/// each paired with its multinomial multiplicity (number of ordered
/// rolls that collapse to it). The multiplicities sum to `6^n`.
pub fn count_outcomes(n: u32) -> Vec<(Counts, u64)> {
    let mut out = Vec::new();
    let mut counts: Counts = [0; 7];
    enumerate(n, 1, &mut counts, &mut out);
    out
}

fn enumerate(
    remaining: u32,
    face: usize,
    counts: &mut Counts,
    out: &mut Vec<(Counts, u64)>,
) {
    if face == 6 {
        counts[6] = remaining;
        out.push((*counts, multinomial(counts)));
        counts[6] = 0;
        return;
    }
    for k in 0..=remaining {
        counts[face] = k;
        enumerate(remaining - k, face + 1, counts, out);
    }
    counts[face] = 0;
}

/// Multinomial coefficient `n! / (c1! c2! ... c6!)` where
/// `n = sum(counts)`: how many ordered rolls match these face counts.
pub fn multinomial(counts: &Counts) -> u64 {
    let n: u32 = counts[1..=6].iter().sum();
    let mut result: u64 = factorial(n);
    for c in &counts[1..=6] {
        result /= factorial(*c);
    }
    result
}

fn factorial(n: u32) -> u64 {
    (1..=n as u64).product::<u64>().max(1)
}

/// Sum of the face values described by `counts`.
pub fn sum(counts: &Counts) -> i32 {
    (1..=6).map(|v| v as i32 * counts[v] as i32).sum()
}

/// Score under the "subtract sixes" rule used by the running events:
/// faces 1-5 count as their value, a 6 counts as `-6`.
pub fn score_six_penalty(counts: &Counts) -> i32 {
    let mut s = 0;
    for v in 1..=5 {
        s += v as i32 * counts[v] as i32;
    }
    s - 6 * counts[6] as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiplicities_sum_to_total() {
        for n in 1..=6 {
            let total: u64 = count_outcomes(n).iter().map(|(_, w)| *w).sum();
            assert_eq!(total, total_outcomes(n));
        }
    }

    #[test]
    fn single_die_has_six_outcomes() {
        let outs = count_outcomes(1);
        assert_eq!(outs.len(), 6);
        assert!(outs.iter().all(|(_, w)| *w == 1));
    }

    #[test]
    fn six_penalty_scoring() {
        let mut c: Counts = [0; 7];
        c[5] = 3; // 5,5,5 -> 15
        c[6] = 2; // two sixes -> -12
        assert_eq!(score_six_penalty(&c), 3);
        assert_eq!(sum(&c), 27);
    }
}

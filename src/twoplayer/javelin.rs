//! The Javelin, solved for two players: six dice, odd faces freeze,
//! best of three attempts.
//!
//! Unlike the running events the attempts **interleave**:
//!
//! > "If a discipline consists of several attempts, all first attempts
//! > are played first, then all second attempts, and so on."
//!
//! So play runs `L₁ T₁ L₂ T₂ L₃ T₃`, and mid-event neither player's
//! result is settled. Because the event scores by `max` rather than by
//! sum, neither banked best can be folded into the running difference
//! until the event ends — which is why both must sit in the state, and
//! why this event costs four orders of magnitude more than the 1500m.

use super::freeze::{Attempt, MAX_SCORE};
use super::{clamp_prob, Axis};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};

/// Attempts each player gets.
const ATTEMPTS: usize = 3;
/// Distinct banked-best values, `0..=30`.
const BESTS: usize = (MAX_SCORE + 1) as usize;

/// Win probability of the player who moves **first**, for every
/// difference on `axis`.
///
/// `after` gives the value of a difference once the javelin is over —
/// for the real decathlon that is the 1500m, already turn-order
/// adjusted.
///
/// `axis` should be the non-negative half only ([`Axis::first_mover`]):
/// the leading player starts, so a first mover is never behind.
///
/// Parallel across the difference axis: `d` never changes during the
/// event (both players' progress lives in the banked bests), so each
/// difference is a wholly independent problem.
pub fn solve_first_mover(
    axis: Axis,
    after: &(dyn Fn(i32) -> f64 + Sync),
) -> Vec<f64> {
    let attempt = Attempt::new(6, [1, 3, 5]);
    axis.iter()
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|d| {
            // value[a * BESTS + b] with `a`, `b` the two banked bests.
            // Seeded with the event over, so the bests settle into `d`.
            let mut value = vec![0.0f64; BESTS * BESTS];
            for a in 0..BESTS {
                for b in 0..BESTS {
                    value[a * BESTS + b] = after(d + a as i32 - b as i32);
                }
            }

            // Walk the six turns backwards. Even phases are the first
            // mover's attempts, odd ones the second mover's.
            for phase in (0..2 * ATTEMPTS).rev() {
                let first_to_move = phase % 2 == 0;
                let mut next = vec![0.0f64; BESTS * BESTS];
                let mut payoff = vec![0.0f64; BESTS];
                for a in 0..BESTS {
                    for b in 0..BESTS {
                        // Scoring `s` replaces the mover's best with
                        // max(best, s) — a foul scores 0 and so leaves
                        // it alone.
                        for (s, slot) in payoff.iter_mut().enumerate() {
                            *slot = if first_to_move {
                                value[a.max(s) * BESTS + b]
                            } else {
                                value[a * BESTS + b.max(s)]
                            };
                        }
                        next[a * BESTS + b] = attempt.expected(&payoff);
                    }
                }
                value = next;
            }
            clamp_prob(value[0])
        })
        .collect()
}

/// Measure how far the javelin's two-player policy compresses.
///
/// Sweeps the difference axis in order so run lengths can be counted
/// against only the previous position, keeping memory at two policy
/// slices rather than the whole 958M-entry table.
pub fn measure(
    axis: Axis,
    after: &(dyn Fn(i32) -> f64 + Sync),
) -> super::compress::Stats {
    let attempt = Attempt::new(6, [1, 3, 5]);
    let nodes = attempt.nodes;
    let phases = 2 * ATTEMPTS;
    let control = phases * BESTS * BESTS * nodes;

    // EV baseline: the solo best-of-three policy, which depends only on
    // attempts left and the player's own banked best -- never on `d`.
    let mut ev = vec![0u8; ATTEMPTS * BESTS * nodes];
    let mut solo = vec![0.0f64; BESTS];
    for (a, slot) in solo.iter_mut().enumerate() {
        *slot = f64::from(a as i32);
    }
    for k in 0..ATTEMPTS {
        let mut next = vec![0.0f64; BESTS];
        let mut payoff = vec![0.0f64; BESTS];
        for a in 0..BESTS {
            for (s, slot) in payoff.iter_mut().enumerate() {
                *slot = solo[a.max(s)];
            }
            let base = (k * BESTS + a) * nodes;
            next[a] = attempt
                .expected_with_policy(&payoff, &mut ev[base..base + nodes]);
        }
        solo = next;
    }

    let mut runs = super::compress::RunCounter::new(control);
    let mut chosen = vec![0u8; control];
    let mut deviates = vec![false; control];
    let mut baseline = vec![0u8; nodes];

    for d in axis.iter() {
        let mut value = vec![0.0f64; BESTS * BESTS];
        for a in 0..BESTS {
            for b in 0..BESTS {
                value[a * BESTS + b] = after(d + a as i32 - b as i32);
            }
        }
        for phase in (0..phases).rev() {
            let first_to_move = phase % 2 == 0;
            // Phase 0 and 1 are each player's first attempt, so two
            // remain after it; phases 4 and 5 are the last.
            let left = ATTEMPTS - 1 - phase / 2;
            let mut next = vec![0.0f64; BESTS * BESTS];
            let mut payoff = vec![0.0f64; BESTS];
            for a in 0..BESTS {
                for b in 0..BESTS {
                    for (s, slot) in payoff.iter_mut().enumerate() {
                        *slot = if first_to_move {
                            value[a.max(s) * BESTS + b]
                        } else {
                            value[a * BESTS + b.max(s)]
                        };
                    }
                    let own = if first_to_move { a } else { b };
                    let src = (left * BESTS + own) * nodes;
                    baseline.copy_from_slice(&ev[src..src + nodes]);
                    let dst = ((phase * BESTS + a) * BESTS + b) * nodes;
                    next[a * BESTS + b] = attempt.expected_vs_baseline(
                        &payoff,
                        &baseline,
                        &mut chosen[dst..dst + nodes],
                        &mut deviates[dst..dst + nodes],
                    );
                }
            }
            value = next;
        }
        runs.push(&chosen, &deviates);
    }

    super::compress::Stats {
        control: control as u64,
        axis: axis.len() as u64,
        deviations: runs.deviations,
        dev_control: runs.dev_control(),
        action_bits: 5, // up to 24 distinct freezes, plus a stop flag
        idx_bytes: if axis.len() <= 256 { 1 } else { 2 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Against a payoff that only rewards the event score, best-of-three
    /// javelin must reproduce the solo expected value pinned in
    /// `tests/disciplines.rs`.
    ///
    /// Feeding `after(d) = d` makes the "win probability" literally the
    /// point difference, so the solver maximises expected score — the
    /// EV-optimal objective — and the first mover's own best is all that
    /// contributes at `d = 0` with the opponent scoring nothing.
    #[test]
    fn reduces_to_the_solo_solver_under_a_linear_payoff() {
        let attempt = Attempt::new(6, [1, 3, 5]);
        let mut value = vec![0.0f64; BESTS * BESTS];
        for a in 0..BESTS {
            value[a * BESTS] = f64::from(a as i32);
        }
        // Three attempts for one player only, best kept.
        for _ in 0..ATTEMPTS {
            let mut next = vec![0.0f64; BESTS * BESTS];
            let mut payoff = vec![0.0f64; BESTS];
            for a in 0..BESTS {
                for (s, slot) in payoff.iter_mut().enumerate() {
                    *slot = value[a.max(s) * BESTS];
                }
                next[a * BESTS] = attempt.expected(&payoff);
            }
            value = next;
        }
        let ev = value[0];
        assert!((ev - 22.251_507_501).abs() < 1e-9, "javelin EV = {ev}");
    }

    /// A lead nothing can overturn stays won.
    #[test]
    fn a_settled_difference_stays_settled() {
        let axis = Axis::symmetric(40);
        let v = solve_first_mover(axis, &crate::twoplayer::final_payoff);
        assert!((v[axis.idx(40)] - 1.0).abs() < 1e-12);
        assert!(v[axis.idx(-40)].abs() < 1e-12);
    }
}

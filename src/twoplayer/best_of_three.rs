//! Best-of-three attempts, interleaved between two players: the Discus
//! and the Javelin.
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
//! why these events cost four orders of magnitude more than the 1500m.

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
    attempt: &Attempt,
    axis: Axis,
    after: &(dyn Fn(i32) -> f64 + Sync),
) -> Vec<f64> {
    axis.iter()
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|d| {
            // value[a * BESTS + b] with `a`, `b` the two banked bests.
            // Seeded with the event over, so the bests settle into `d`.
            let live: Vec<usize> = (0..BESTS)
                .filter(|&s| attempt.reachable_scores[s])
                .collect();
            let mut value = vec![0.0f64; BESTS * BESTS];
            for &a in &live {
                for &b in &live {
                    value[a * BESTS + b] = after(d + a as i32 - b as i32);
                }
            }

            // Walk the six turns backwards. Even phases are the first
            // mover's attempts, odd ones the second mover's.
            for phase in (0..2 * ATTEMPTS).rev() {
                let first_to_move = phase % 2 == 0;
                let mut next = vec![0.0f64; BESTS * BESTS];
                let mut payoff = vec![0.0f64; BESTS];
                for &a in &live {
                    for &b in &live {
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
                        let floor = if first_to_move { a } else { b };
                        next[a * BESTS + b] = attempt.expected(&payoff, floor);
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
/// slices rather than the whole table.
pub fn measure(
    attempt: &Attempt,
    axis: Axis,
    after: &(dyn Fn(i32) -> f64 + Sync),
) -> super::compress::Stats {
    let nodes = attempt.nodes;
    let phases = 2 * ATTEMPTS;
    // Indexed densely so `a` and `b` stay plain scores, but only the
    // reachable bests are ever written -- that smaller figure is what a
    // stored policy would actually occupy.
    let slots = phases * BESTS * BESTS * nodes;
    // A node dead against the mover's banked best needs no stored action,
    // and deadness depends only on that best -- not on the difference --
    // so it can be counted once.
    let per_floor: Vec<usize> =
        (0..BESTS).map(|floor| attempt.live_nodes(floor)).collect();
    // Only `(phase, a, b)` triples play actually reaches. Before the
    // first mover has thrown, both bests are still zero; before the
    // second has, only the first mover's can be anything else.
    let mut control = 0usize;
    for phase in 0..phases {
        let a_thrown = phase.div_ceil(2) > 0;
        let b_thrown = phase / 2 > 0;
        for a in (0..BESTS).filter(|&s| attempt.reachable_scores[s]) {
            if !a_thrown && a != 0 {
                continue;
            }
            for b in (0..BESTS).filter(|&s| attempt.reachable_scores[s]) {
                if !b_thrown && b != 0 {
                    continue;
                }
                let mover_best = if phase % 2 == 0 { a } else { b };
                control += per_floor[mover_best];
            }
        }
    }

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
            next[a] = attempt.expected_with_policy(
                &payoff,
                a,
                &mut ev[base..base + nodes],
            );
        }
        solo = next;
    }

    let mut runs = super::compress::RunCounter::new(slots);
    let mut chosen = vec![0u8; slots];
    let mut deviates = vec![false; slots];
    let mut baseline = vec![0u8; nodes];

    let live: Vec<usize> = (0..BESTS)
        .filter(|&s| attempt.reachable_scores[s])
        .collect();
    for d in axis.iter() {
        let mut value = vec![0.0f64; BESTS * BESTS];
        for &a in &live {
            for &b in &live {
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
            for &a in &live {
                for &b in &live {
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
                        own,
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
        // Enough to name any distinct freeze plus a stop flag: the
        // javelin has 24 of them, the discus 18.
        action_bits: 5,
        idx_bytes: if axis.len() <= 256 { 1 } else { 2 },
    }
}

/// Expected score of the event under solo, EV-maximising play.
///
/// One player, three attempts, best kept — the objective the original
/// `disciplines::` solvers answer. Used to check this engine against the
/// values pinned in `tests/disciplines.rs`.
pub fn solo_expected_value(attempt: &Attempt) -> f64 {
    let mut value = vec![0.0f64; BESTS];
    for (a, slot) in value.iter_mut().enumerate() {
        *slot = f64::from(a as i32);
    }
    for _ in 0..ATTEMPTS {
        let mut next = vec![0.0f64; BESTS];
        let mut payoff = vec![0.0f64; BESTS];
        for a in 0..BESTS {
            for (s, slot) in payoff.iter_mut().enumerate() {
                *slot = value[a.max(s)];
            }
            next[a] = attempt.expected(&payoff, a);
        }
        value = next;
    }
    value[0]
}

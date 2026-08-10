//! The Pole Vault, solved for two players.
//!
//! A rising bar, and no dice state whatsoever. Clearing pays exactly the
//! bar height, so each jump is binary, so you use the die count that
//! maximises the clear probability, so the three jumps at a height
//! collapse to `1 - (1-p)³`. Nothing carries between jumps. The entire
//! dice model is a twenty-float table.
//!
//! What is left is a ladder with one decision per height — attempt or
//! skip — and one absorbing accident:
//!
//! > "If you suffer three invalid jumps at one height you have to stop."
//!
//! Heights interleave like attempts do: each player takes all three
//! jumps at a bar before the next player starts on it.
//!
//! Skipping is free — you keep your best, stay alive, lose nothing —
//! while attempting risks elimination. Under the solo expected-value
//! objective that trade never pays and the skip branch is dead code. It
//! is *not* dead here: a bar you do not need is worth nothing when only
//! the final placing counts, so a trailing player skips low bars to
//! protect a shot at the one that actually wins.

use super::{clamp_prob, Axis};
use crate::disciplines::polevault::clear_probabilities;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};

/// Best cleared, encoded as `0` for "nothing yet" or `1 + height index`.
type Best = usize;

/// Everything about a position except the height and whose turn it is.
#[derive(Clone, Copy)]
struct Standing {
    /// Bests of the first and second mover.
    best: [Best; 2],
    /// Whether each is still in the event.
    alive: [bool; 2],
}

/// Ladder geometry: the heights, and the chance of clearing each.
struct Ladder {
    heights: Vec<i32>,
    clear: Vec<f64>,
}

impl Ladder {
    fn new() -> Self {
        let table = clear_probabilities();
        Self {
            heights: table.iter().map(|&(h, _)| h).collect(),
            clear: table.iter().map(|&(_, p)| p).collect(),
        }
    }

    /// Points a `Best` is worth: nothing, or the height it names.
    fn score(&self, b: Best) -> i32 {
        if b == 0 {
            0
        } else {
            self.heights[b - 1]
        }
    }

    /// Index of a standing in the flat value table.
    fn idx(&self, s: Standing) -> usize {
        let n = self.heights.len() + 1;
        ((s.best[0] * n + s.best[1]) * 2 + usize::from(s.alive[0])) * 2
            + usize::from(s.alive[1])
    }

    fn table_len(&self) -> usize {
        let n = self.heights.len() + 1;
        n * n * 4
    }

    /// Every standing, in index order.
    fn standings(&self) -> impl Iterator<Item = Standing> + '_ {
        let n = self.heights.len() + 1;
        (0..n).flat_map(move |a| {
            (0..n).flat_map(move |b| {
                [false, true].into_iter().flat_map(move |aa| {
                    [false, true].into_iter().map(move |ab| Standing {
                        best: [a, b],
                        alive: [aa, ab],
                    })
                })
            })
        })
    }
}

/// Values of skipping and of attempting height `i`, for `mover`.
fn branches(
    l: &Ladder,
    next: &[f64],
    i: usize,
    s: Standing,
    mover: usize,
) -> (f64, f64) {
    let skip = next[l.idx(s)];
    let mut cleared = s;
    cleared.best[mover] = i + 1;
    let mut out = s;
    out.alive[mover] = false;
    let p = l.clear[i];
    let attempt = p.mul_add(next[l.idx(cleared)], (1.0 - p) * next[l.idx(out)]);
    (skip, attempt)
}

/// Win probability of the player who moves **first**, for every
/// difference on `axis`, which should be the non-negative half.
pub fn solve_first_mover(
    axis: Axis,
    after: &(dyn Fn(i32) -> f64 + Sync),
) -> Vec<f64> {
    let l = Ladder::new();
    axis.iter()
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|d| {
            // Past the top bar the event is over and both bests settle.
            let mut next = vec![0.0f64; l.table_len()];
            for s in l.standings() {
                next[l.idx(s)] =
                    after(d + l.score(s.best[0]) - l.score(s.best[1]));
            }
            for i in (0..l.heights.len()).rev() {
                // The second mover acts last at this height, so resolve
                // them first when working backwards.
                for mover in [1usize, 0] {
                    let mut cur = vec![0.0f64; l.table_len()];
                    for s in l.standings() {
                        cur[l.idx(s)] = if s.alive[mover] {
                            let (skip, attempt) =
                                branches(&l, &next, i, s, mover);
                            skip.max(attempt)
                        } else {
                            next[l.idx(s)]
                        };
                    }
                    next = cur;
                }
            }
            clamp_prob(
                next[l.idx(Standing {
                    best: [0, 0],
                    alive: [true, true],
                })],
            )
        })
        .collect()
}

/// Measure how far the pole vault's two-player policy compresses.
///
/// The baseline is the solo expected-value policy, which never skips —
/// so every stored deviation here is a height the win-probability player
/// declines to attempt.
pub fn measure(
    axis: Axis,
    after: &(dyn Fn(i32) -> f64 + Sync),
) -> super::compress::Stats {
    let l = Ladder::new();
    let n = l.heights.len();

    // Solo EV ladder: attempting weakly dominates, so this is the "always
    // attempt" policy, but compute it rather than assume it.
    let mut ev_skip = vec![false; n * (n + 1)];
    let mut ev_next = vec![0.0f64; n + 1];
    for b in 0..=n {
        ev_next[b] = f64::from(l.score(b));
    }
    for i in (0..n).rev() {
        let mut cur = vec![0.0f64; n + 1];
        for b in 0..=n {
            let p = l.clear[i];
            let attempt =
                p.mul_add(ev_next[i + 1], (1.0 - p) * f64::from(l.score(b)));
            ev_skip[i * (n + 1) + b] = ev_next[b] > attempt;
            cur[b] = ev_next[b].max(attempt);
        }
        ev_next = cur;
    }

    // One control state per (height, standing, mover) that play reaches
    // with that mover still in: a player already out never chooses again,
    // and neither can have cleared a bar they are still facing.
    let reached = |i: usize, s: &Standing| s.best[0] <= i && s.best[1] <= i;
    let mut offset = Vec::with_capacity(n);
    let mut control = 0usize;
    for i in 0..n {
        offset.push(control);
        for st in l.standings() {
            if reached(i, &st) {
                control += usize::from(st.alive[0]) + usize::from(st.alive[1]);
            }
        }
    }

    let mut runs = super::compress::RunCounter::new(control);
    let mut chosen = vec![0u8; control];
    let mut deviates = vec![false; control];

    for d in axis.iter() {
        let mut next = vec![0.0f64; l.table_len()];
        for s in l.standings() {
            next[l.idx(s)] = after(d + l.score(s.best[0]) - l.score(s.best[1]));
        }
        for i in (0..n).rev() {
            // The second mover acts last at this height, so resolve them
            // first when working backwards. Policy slots are numbered in
            // a fixed order so the same control state lands in the same
            // slot for every difference.
            for mover in [1usize, 0] {
                let mut cur = vec![0.0f64; l.table_len()];
                let mut written = 0;
                for s in l.standings() {
                    if !s.alive[mover] {
                        cur[l.idx(s)] = next[l.idx(s)];
                        continue;
                    }
                    let (skip, attempt) = branches(&l, &next, i, s, mover);
                    cur[l.idx(s)] = skip.max(attempt);
                    if !reached(i, &s) {
                        continue;
                    }
                    let ev_prefers_skip = ev_skip[i * (n + 1) + s.best[mover]];
                    let used = if ev_prefers_skip { skip } else { attempt };
                    let dev =
                        super::compress::is_deviation(skip.max(attempt), used);
                    let at = offset[i] + written;
                    deviates[at] = dev;
                    chosen[at] = u8::from(if dev {
                        skip > attempt
                    } else {
                        ev_prefers_skip
                    });
                    written += 1;
                }
                next = cur;
            }
        }
        runs.push(&chosen, &deviates);
    }

    super::compress::Stats {
        control: control as u64,
        axis: axis.len() as u64,
        deviations: runs.deviations,
        dev_control: runs.dev_control(),
        action_bits: 1,
        idx_bytes: 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Under a payoff that simply counts points, the ladder must
    /// reproduce the solo expected value pinned in `tests/disciplines.rs`.
    #[test]
    fn reduces_to_the_solo_solver_under_a_linear_payoff() {
        let l = Ladder::new();
        let n = l.heights.len();
        let mut next = vec![0.0f64; n + 1];
        for b in 0..=n {
            next[b] = f64::from(l.score(b));
        }
        for i in (0..n).rev() {
            let mut cur = vec![0.0f64; n + 1];
            for b in 0..=n {
                let p = l.clear[i];
                let attempt =
                    p.mul_add(next[i + 1], (1.0 - p) * f64::from(l.score(b)));
                cur[b] = next[b].max(attempt);
            }
            next = cur;
        }
        assert!(
            (next[0] - 17.277_634_238).abs() < 1e-9,
            "pole vault EV = {}",
            next[0]
        );
    }

    /// Skipping a height is sometimes optimal **even for the solo
    /// expected-value player**, contrary to what `RULES-CHECKLIST.md`
    /// claimed until this test was written.
    ///
    /// With 32 already banked, attempting 34 clears less than a quarter
    /// of the time, and the other three quarters end the event and
    /// forfeit every shot at 36 and above. The marginal two points do not
    /// cover that, so the bar is worth declining.
    ///
    /// This matters because the two-player measurement reports
    /// deviations against the EV policy: if that policy were "always
    /// attempt", every skip would count as a deviation.
    #[test]
    fn the_solo_player_does_sometimes_skip() {
        let l = Ladder::new();
        let n = l.heights.len();
        let mut next = vec![0.0f64; n + 1];
        for b in 0..=n {
            next[b] = f64::from(l.score(b));
        }
        let mut skips = 0;
        for i in (0..n).rev() {
            let mut cur = vec![0.0f64; n + 1];
            for b in 0..=n {
                let p = l.clear[i];
                let attempt =
                    p.mul_add(next[i + 1], (1.0 - p) * f64::from(l.score(b)));
                // `b > i` would mean having cleared a bar we are still
                // facing, which play never reaches.
                if b <= i && next[b] > attempt {
                    skips += 1;
                }
                cur[b] = next[b].max(attempt);
            }
            next = cur;
        }
        assert!(skips > 0, "expected the EV ladder to decline some bar");
    }

    /// A lead nothing can overturn stays won.
    #[test]
    fn a_settled_difference_stays_settled() {
        let axis = Axis::first_mover(48);
        let v = solve_first_mover(axis, &crate::twoplayer::final_payoff);
        assert!((v[axis.idx(48)] - 1.0).abs() < 1e-12);
    }
}

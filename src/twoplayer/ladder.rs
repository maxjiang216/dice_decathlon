//! The rising-bar events, solved for two players: High Jump and Pole
//! Vault.
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
pub struct Ladder {
    heights: Vec<i32>,
    clear: Vec<f64>,
}

impl Ladder {
    /// Build from a `(height, clear probability)` table.
    pub fn new(table: &[(i32, f64)]) -> Self {
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
    l: &Ladder,
    axis: Axis,
    after: &(dyn Fn(i32) -> f64 + Sync),
) -> Vec<f64> {
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
                                branches(l, &next, i, s, mover);
                            // The table is the first mover's win
                            // probability, so the second mover picks the
                            // branch that lowers it.
                            if mover == 0 {
                                skip.max(attempt)
                            } else {
                                skip.min(attempt)
                            }
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
    l: &Ladder,
    axis: Axis,
    after: &(dyn Fn(i32) -> f64 + Sync),
) -> super::compress::Stats {
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
                    let (skip, attempt) = branches(l, &next, i, s, mover);
                    // As above: the second mover plays against the value.
                    cur[l.idx(s)] = if mover == 0 {
                        skip.max(attempt)
                    } else {
                        skip.min(attempt)
                    };
                    if !reached(i, &s) {
                        continue;
                    }
                    let ev_prefers_skip = ev_skip[i * (n + 1) + s.best[mover]];
                    // Compare from the mover's own side.
                    let (sk, at_) = if mover == 0 {
                        (skip, attempt)
                    } else {
                        (1.0 - skip, 1.0 - attempt)
                    };
                    let used = if ev_prefers_skip { sk } else { at_ };
                    let dev = super::compress::is_deviation(sk.max(at_), used);
                    let at = offset[i] + written;
                    deviates[at] = dev;
                    chosen[at] =
                        u8::from(if dev { sk > at_ } else { ev_prefers_skip });
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

/// Expected score of the ladder under solo, EV-maximising play, and how
/// many heights that player declines.
///
/// Skipping is *not* dead code here: with a good bar already banked, a
/// low-probability attempt risks elimination and forfeits every higher
/// bar, so declining can win. See `worklog/RULES-CHECKLIST.md`.
pub fn solo_expected_value(l: &Ladder) -> (f64, usize) {
    let n = l.heights.len();
    let mut next: Vec<f64> = (0..=n).map(|b| f64::from(l.score(b))).collect();
    let mut skips = 0;
    for i in (0..n).rev() {
        let mut cur = vec![0.0f64; n + 1];
        for b in 0..=n {
            let p = l.clear[i];
            let attempt =
                p.mul_add(next[i + 1], (1.0 - p) * f64::from(l.score(b)));
            // `b > i` would mean having cleared a bar still being faced,
            // which play never reaches.
            if b <= i && next[b] > attempt {
                skips += 1;
            }
            cur[b] = next[b].max(attempt);
        }
        next = cur;
    }
    (next[0], skips)
}

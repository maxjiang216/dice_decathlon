//! Best-of-N driver for the multi-attempt throwing events (Shot Put,
//! Long Jump, Discus, Javelin).
//!
//! Each event is played as `N` independent attempts and only the best
//! result counts. The optimal play *within* an attempt depends on the
//! best score banked so far: with a strong score already in hand you
//! gamble for more, otherwise you play it safe. We capture this with a
//! value function `v(k, b)` = expected final score with `k` attempts
//! remaining and current best `b`.
//!
//! An attempt is solved against an arbitrary increasing payoff `g`
//! (the continuation value), returning the exact distribution of the
//! attempt's score under the policy that maximises `E[g(score)]`. A
//! foul scores 0.

use crate::dp::Dist;

/// A single attempt of a throwing event.
pub trait Attempt {
    /// Largest score the attempt can produce (the lower bound is 0).
    fn score_hi(&self) -> i32;

    /// Distribution of the attempt's score under the policy maximising
    /// `E[g(score)]`. A foul contributes mass at score 0.
    fn solve(&self, g: &dyn Fn(i32) -> f64) -> Dist;
}

/// Final-score distribution of the best of `n` attempts under optimal
/// play.
pub fn best_of_n(n: u8, att: &dyn Attempt) -> Dist {
    let hi = att.score_hi();
    let span = (hi + 1) as usize;
    let idx = |b: i32| b as usize;

    // v[k][b] and the attempt distribution that achieves it.
    let mut v: Vec<Vec<f64>> = vec![vec![0.0; span]; (n + 1) as usize];
    let mut dists: Vec<Vec<Dist>> =
        vec![vec![Dist::default(); span]; (n + 1) as usize];
    for b in 0..=hi {
        v[0][idx(b)] = b as f64;
    }

    for k in 1..=n as usize {
        let v_prev = v[k - 1].clone();
        for b in 0..=hi {
            // Continuation value of finishing this attempt at score x:
            // the banked best becomes max(b, x).
            let g = |x: i32| v_prev[idx(x.max(b))];
            let d = att.solve(&g);
            v[k][idx(b)] = d.expectation(g);
            dists[k][idx(b)] = d;
        }
    }

    // Forward pass: track the distribution of the running best as each
    // attempt is played with the policy chosen above.
    let mut best = Dist::point(0);
    for i in 1..=n {
        let k = (n - i + 1) as usize;
        let mut next = Dist::default();
        for (&b, &pb) in &best.mass {
            let d = &dists[k][idx(b)];
            for (&s, &ps) in &d.mass {
                next.add(b.max(s), pb * ps);
            }
        }
        best = next;
    }
    best
}

/// Pick whichever distribution yields the larger `E[g]`, preferring `a`
/// (the stop / earlier action) on a tie.
pub fn better_for<F: Fn(i32) -> f64>(a: Dist, b: Dist, g: &F) -> Dist {
    const EPS: f64 = 1e-12;
    if b.expectation(g) > a.expectation(g) + EPS {
        b
    } else {
        a
    }
}

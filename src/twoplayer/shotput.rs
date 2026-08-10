//! The Shot Put: eight dice thrown **one at a time**, stop whenever you
//! like, and a single 1 invalidates the attempt.
//!
//! > "Throw one die after the other. At any point you can stop. Your
//! > attempt must end after all eight dice. If you throw a one you suffer
//! > an invalid attempt."
//!
//! The dice never need enumerating: each throw adds one face, so the
//! state is just `(dice thrown, running sum)`. Since a 1 ends the
//! attempt, any surviving sum is built from faces 2-6, which is why the
//! sum is confined to `2t..=6t` and why the event cannot score 1.
//!
//! A foul costs only the current attempt — the banked best survives — so
//! while the running sum is at or below that best, throwing *weakly
//! dominates* stopping: the worst case returns you to exactly what
//! stopping would have given. Those positions carry no decision.

use super::attempt::AttemptEngine;
use super::compress::Stats;
use super::{best_of_three, Axis, EPS};

/// Dice available.
const DICE: usize = 8;
/// Eight dice at six, ones having ended any attempt that saw one.
pub const MAX_SCORE: usize = DICE * 6;

/// The shot put's attempt: throw or stop, one die at a time.
pub struct ShotPut {
    /// Flat index of the first node for `t` dice thrown.
    node_base: Vec<usize>,
    nodes: usize,
    reachable: Vec<bool>,
}

impl Default for ShotPut {
    fn default() -> Self {
        Self::new()
    }
}

impl ShotPut {
    /// Build the node index.
    pub fn new() -> Self {
        // A decision exists only for `1..DICE` thrown: the first throw is
        // compulsory, and after the eighth there is nothing left to do.
        let mut node_base = vec![0usize; DICE + 1];
        let mut nodes = 0;
        for t in 1..DICE {
            node_base[t] = nodes;
            nodes += 4 * t + 1; // sums 2t..=6t
        }
        let mut reachable = vec![false; MAX_SCORE + 1];
        reachable[0] = true; // a foul, or never getting going
        for s in 2..=MAX_SCORE {
            reachable[s] = true;
        }
        Self {
            node_base,
            nodes,
            reachable,
        }
    }

    /// Whether nothing left in hand can beat the banked best.
    const fn is_dead(t: usize, cur: usize, floor: usize) -> bool {
        cur + 6 * (DICE - t) <= floor
    }

    /// Solve the attempt, optionally recording a policy and comparing it
    /// against a baseline.
    fn run(
        &self,
        payoff: &[f64],
        floor: usize,
        mut policy: Option<&mut [u8]>,
        cmp: Option<(&[u8], &mut [bool])>,
    ) -> f64 {
        let (baseline, mut deviates) = match cmp {
            Some((b, d)) => (Some(b), Some(d)),
            None => (None, None),
        };
        // value[t][cur]; filled downwards because throwing only adds dice.
        let width = MAX_SCORE + 1;
        let mut value = vec![0.0f64; (DICE + 1) * width];
        for cur in 2 * DICE..=6 * DICE {
            value[DICE * width + cur] = payoff[cur];
        }
        for t in (0..DICE).rev() {
            let (lo, hi) = if t == 0 { (0, 0) } else { (2 * t, 6 * t) };
            for cur in lo..=hi {
                if Self::is_dead(t, cur, floor) {
                    value[t * width + cur] = payoff[0];
                    if t > 0 {
                        let n = self.node_base[t] + cur - 2 * t;
                        if let Some(p) = policy.as_deref_mut() {
                            p[n] = u8::MAX;
                        }
                        if let Some(d) = deviates.as_deref_mut() {
                            d[n] = false;
                        }
                    }
                    continue;
                }
                // A 1 ends it; faces 2-6 carry on with one die fewer.
                let mut throw = payoff[0];
                for face in 2..=6usize {
                    throw += value[(t + 1) * width + cur + face];
                }
                throw /= 6.0;
                value[t * width + cur] = if t == 0 {
                    throw // the first die is compulsory
                } else {
                    let stop = payoff[cur];
                    let n = self.node_base[t] + cur - 2 * t;
                    let best_stop = stop >= throw;
                    let action = match (baseline, deviates.as_deref_mut()) {
                        (Some(base), Some(dev)) => {
                            let base_stop = base[n] == 1;
                            let used = if base_stop { stop } else { throw };
                            let d = stop.max(throw) - used > EPS;
                            dev[n] = d;
                            if d {
                                best_stop
                            } else {
                                base_stop
                            }
                        }
                        _ => best_stop,
                    };
                    if let Some(p) = policy.as_deref_mut() {
                        p[n] = u8::from(action);
                    }
                    stop.max(throw)
                };
            }
        }
        value[0]
    }
}

impl AttemptEngine for ShotPut {
    fn max_score(&self) -> usize {
        MAX_SCORE
    }
    fn nodes(&self) -> usize {
        self.nodes
    }
    fn reachable_scores(&self) -> &[bool] {
        &self.reachable
    }
    fn live_nodes(&self, floor: usize) -> usize {
        let mut n = 0;
        for t in 1..DICE {
            for cur in 2 * t..=6 * t {
                // Throwing weakly dominates while the sum has not passed
                // the banked best, so those carry no real decision either.
                if !Self::is_dead(t, cur, floor) && cur > floor {
                    n += 1;
                }
            }
        }
        n
    }
    fn expected(&self, payoff: &[f64], floor: usize) -> f64 {
        self.run(payoff, floor, None, None)
    }
    fn expected_with_policy(
        &self,
        payoff: &[f64],
        floor: usize,
        policy: &mut [u8],
    ) -> f64 {
        self.run(payoff, floor, Some(policy), None)
    }
    fn expected_vs_baseline(
        &self,
        payoff: &[f64],
        floor: usize,
        baseline: &[u8],
        chosen: &mut [u8],
        deviates: &mut [bool],
    ) -> f64 {
        self.run(payoff, floor, Some(chosen), Some((baseline, deviates)))
    }
}

/// Win probability of the player who moves first, per difference.
pub fn solve_first_mover(
    axis: Axis,
    after: &(dyn Fn(i32) -> f64 + Sync),
) -> Vec<f64> {
    best_of_three::solve_first_mover(&ShotPut::new(), axis, after)
}

/// Policy storage statistics.
pub fn measure(axis: Axis, after: &(dyn Fn(i32) -> f64 + Sync)) -> Stats {
    best_of_three::measure(&ShotPut::new(), axis, after)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Matches the solo expected value in `tests/disciplines.rs`.
    #[test]
    fn reduces_to_the_solo_solver_under_a_linear_payoff() {
        let ev = best_of_three::solo_expected_value(&ShotPut::new());
        assert!((ev - 18.634_491_985).abs() < 1e-9, "shot put EV = {ev}");
    }

    /// A 1 ends the attempt, so no surviving sum can be 1.
    #[test]
    fn a_score_of_one_is_impossible() {
        let sp = ShotPut::new();
        assert!(sp.reachable[0]);
        assert!(!sp.reachable[1]);
        assert!(sp.reachable[2]);
    }
}

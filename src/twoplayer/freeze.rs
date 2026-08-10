//! One attempt of the "freeze dice of a chosen parity" events: Discus
//! (even faces) and Javelin (odd faces).
//!
//! Throw every unfrozen die, freeze at least one of the allowed parity,
//! stop whenever you like. If no die of that parity shows, the attempt
//! is invalid and scores zero. The attempt also ends once every die is
//! frozen.
//!
//! Two reductions make the roll enumeration small, both proven exact in
//! `tests/attempt_engines.rs`:
//!
//! * Dice of the *wrong* parity are interchangeable — nothing you can do
//!   distinguishes a 2 from a 4 in the javelin — so they collapse into a
//!   single class. Rolls become multisets over four symbols, not six.
//! * Two freezes with the same `(count, value-sum)` leave identical
//!   states, so only the distinct pairs need exploring.

/// Highest score either event can reach: five discus dice at 6, or six
/// javelin dice at 5.
pub const MAX_SCORE: i32 = 30;

/// One distinct throw of `u` dice, pre-reduced to what the decision
/// actually depends on.
struct Throw {
    /// Ordered rolls collapsing to this class.
    weight: u64,
    /// Distinct `(dice frozen, value added)` freezes. Empty means no die
    /// of the allowed parity showed, so the attempt is invalid.
    choices: Vec<(u32, i32)>,
}

/// Pre-enumerated throws for an event, indexed by dice in hand.
pub struct Attempt {
    /// `throws[u]` covers a throw of `u` dice; index 0 is unused.
    throws: Vec<Vec<Throw>>,
    /// `6^u`, the weight total for a throw of `u` dice.
    totals: Vec<u64>,
    /// `reachable[u]` lists the frozen sums possible with `u` dice still
    /// in hand. Only these are worth visiting: with `f = dice - u` dice
    /// frozen, each worth one of three same-parity faces, the sum is
    /// confined to `2f + 1` values rather than the full 0..=30. Skipping
    /// the rest is a 9x saving on the inner loop.
    reachable: Vec<Vec<usize>>,
    /// Flat index of the first decision node for `u` dice in hand.
    /// A node is one `(dice in hand, frozen sum, distinct throw)`, which
    /// is where a policy records which freeze was chosen.
    node_base: Vec<usize>,
    /// Total decision nodes; 701 for the javelin, 367 for the discus.
    pub nodes: usize,
    /// Largest freezable face: 5 for the javelin, 6 for the discus. Six
    /// javelin dice therefore cap at 30, exactly like five discus dice.
    max_face: i32,
    /// Which scores an attempt can actually produce. The javelin cannot
    /// score 27 or 29 — odd totals need an odd number of dice, and five
    /// is the most that fit — and the discus can only score evens. A
    /// banked best is always one of these, so the rest need never be
    /// visited.
    pub reachable_scores: Vec<bool>,
    dice: u32,
}

impl Attempt {
    /// Build the tables for `dice` dice whose freezable faces are
    /// `faces` (which must all share a parity).
    pub fn new(dice: u32, faces: [i32; 3]) -> Self {
        let mut throws = vec![Vec::new()];
        let mut totals = vec![1u64];
        for u in 1..=dice {
            let mut list = Vec::new();
            // (a, b, c) freezable dice showing faces[0..3], the rest of
            // the wrong parity and mutually indistinguishable.
            for a in 0..=u {
                for b in 0..=u - a {
                    for c in 0..=u - a - b {
                        let wrong = u - a - b - c;
                        // 3^wrong because each wrong-parity die could be
                        // any of the three faces of that parity.
                        let weight =
                            multinomial(&[a, b, c, wrong]) * 3u64.pow(wrong);
                        let mut choices = Vec::new();
                        for k0 in 0..=a {
                            for k1 in 0..=b {
                                for k2 in 0..=c {
                                    if k0 + k1 + k2 == 0 {
                                        continue;
                                    }
                                    choices.push((
                                        k0 + k1 + k2,
                                        k0 as i32 * faces[0]
                                            + k1 as i32 * faces[1]
                                            + k2 as i32 * faces[2],
                                    ));
                                }
                            }
                        }
                        choices.sort_unstable();
                        choices.dedup();
                        list.push(Throw { weight, choices });
                    }
                }
            }
            totals.push(6u64.pow(u));
            throws.push(list);
        }
        let mut reachable = vec![vec![0usize]];
        for u in 1..=dice {
            let f = (dice - u) as i32;
            let lo = f * faces[0];
            let hi = f * faces[2];
            reachable.push((lo..=hi).step_by(2).map(|v| v as usize).collect());
        }
        let mut node_base = vec![0usize];
        let mut nodes = 0;
        for u in 1..=dice as usize {
            node_base.push(nodes);
            nodes += reachable[u].len() * throws[u].len();
        }
        let width = (MAX_SCORE + 1) as usize;
        let mut reachable_scores = vec![false; width];
        reachable_scores[0] = true; // a foul scores nothing
        for k in 1..=dice as i32 {
            for lo in 0..=k {
                for mid in 0..=k - lo {
                    let hi = k - lo - mid;
                    let s = lo * faces[0] + mid * faces[1] + hi * faces[2];
                    if s <= MAX_SCORE {
                        reachable_scores[s as usize] = true;
                    }
                }
            }
        }
        Self {
            throws,
            totals,
            reachable,
            node_base,
            nodes,
            max_face: faces[2],
            reachable_scores,
            dice,
        }
    }

    /// Decision nodes worth storing when the mover has banked `floor`.
    ///
    /// A node is dropped when it is dead (nothing can beat `floor`), when
    /// the throw shows no freezable die at all (an invalid attempt, with
    /// nothing to decide), or when its single freeze uses up the last die
    /// in hand so there is not even a stop-or-continue choice left.
    pub fn live_nodes(&self, floor: usize) -> usize {
        let mut n = 0;
        for u in 1..=self.dice as usize {
            for &fs in &self.reachable[u] {
                if self.is_dead(u, fs, floor) {
                    continue;
                }
                for throw in &self.throws[u] {
                    let forced = throw.choices.is_empty()
                        || (throw.choices.len() == 1
                            && throw.choices[0].0 as usize == u);
                    if !forced {
                        n += 1;
                    }
                }
            }
        }
        n
    }

    /// Whether an attempt holding `fs` with `u` dice left can still beat
    /// `floor`, the mover's banked best.
    ///
    /// If it cannot, every outcome leaves the best untouched, so the
    /// whole subtree collapses to a single value and never needs
    /// exploring. About one node in seven is dead this way.
    const fn is_dead(&self, u: usize, fs: usize, floor: usize) -> bool {
        fs + u * self.max_face as usize <= floor
    }

    /// As [`Attempt::expected`], but also records which choice was taken
    /// at every decision node into `policy`, which must have length
    /// [`Attempt::nodes`].
    ///
    /// The action is the index into the node's distinct
    /// `(count, sum)` freezes, with the high bit set when the attempt is
    /// stopped rather than continued. `u8::MAX` marks an invalid attempt,
    /// where there is nothing to choose.
    pub fn expected_with_policy(
        &self,
        payoff: &[f64],
        floor: usize,
        policy: &mut [u8],
    ) -> f64 {
        let width = (MAX_SCORE + 1) as usize;
        let mut best = vec![0.0f64; (self.dice as usize + 1) * width];
        for u in 1..=self.dice as usize {
            for (fi, &fs) in self.reachable[u].iter().enumerate() {
                let row = self.node_base[u] + fi * self.throws[u].len();
                if self.is_dead(u, fs, floor) {
                    // Nothing here can beat the banked best, so the whole
                    // subtree is one value and no action is ever needed.
                    best[u * width + fs] = payoff[0];
                    for t in 0..self.throws[u].len() {
                        policy[row + t] = u8::MAX;
                    }
                    continue;
                }
                let mut acc = 0.0;
                for (ti, throw) in self.throws[u].iter().enumerate() {
                    let v = if throw.choices.is_empty() {
                        policy[row + ti] = u8::MAX;
                        payoff[0]
                    } else {
                        let mut top = f64::NEG_INFINITY;
                        let mut arg = 0u8;
                        for (ci, &(k, add)) in throw.choices.iter().enumerate()
                        {
                            let fs2 = fs + add as usize;
                            let u2 = u - k as usize;
                            let stop = payoff[fs2];
                            let (v, stopped) = if u2 == 0 {
                                (stop, true)
                            } else {
                                let go = best[u2 * width + fs2];
                                if stop >= go {
                                    (stop, true)
                                } else {
                                    (go, false)
                                }
                            };
                            if v > top {
                                top = v;
                                arg =
                                    (ci as u8) | if stopped { 0x80 } else { 0 };
                            }
                        }
                        policy[row + ti] = arg;
                        top
                    };
                    acc += throw.weight as f64 * v;
                }
                best[u * width + fs] = acc / self.totals[u] as f64;
            }
        }
        best[self.dice as usize * width]
    }

    /// Best achievable `E[payoff(score)]` for one attempt.
    ///
    /// `payoff` is indexed by score, `0..=MAX_SCORE`; a foul reads
    /// `payoff[0]`. A slice rather than a closure because this is the
    /// innermost loop of the whole solver and the dynamic dispatch
    /// showed up.
    pub fn expected(&self, payoff: &[f64], floor: usize) -> f64 {
        let width = (MAX_SCORE + 1) as usize;
        // best[u][fs] = value with `u` dice in hand and `fs` frozen,
        // before throwing. Filled for increasing `u` because freezing
        // only ever reduces the dice in hand.
        let mut best = vec![0.0f64; (self.dice as usize + 1) * width];
        for u in 1..=self.dice as usize {
            for &fs in &self.reachable[u] {
                if self.is_dead(u, fs, floor) {
                    best[u * width + fs] = payoff[0];
                    continue;
                }
                let mut acc = 0.0;
                for throw in &self.throws[u] {
                    let v = if throw.choices.is_empty() {
                        payoff[0] // no freezable die: invalid attempt
                    } else {
                        let mut top = f64::NEG_INFINITY;
                        for &(k, add) in &throw.choices {
                            let fs2 = fs + add as usize;
                            let u2 = u - k as usize;
                            // Stopping banks `fs2`; carrying on rethrows
                            // what is left. Freezing everything ends the
                            // attempt with no choice.
                            let v = if u2 == 0 {
                                payoff[fs2]
                            } else {
                                payoff[fs2].max(best[u2 * width + fs2])
                            };
                            top = top.max(v);
                        }
                        top
                    };
                    acc += throw.weight as f64 * v;
                }
                best[u * width + fs] = acc / self.totals[u] as f64;
            }
        }
        best[self.dice as usize * width]
    }
}

impl Attempt {
    /// As [`Attempt::expected`], but also compares every node against a
    /// baseline policy.
    ///
    /// `deviates[n]` is set when following `baseline[n]` would cost more
    /// than [`EPS`]; otherwise `chosen[n]` copies the baseline, so ties
    /// and decided positions cost nothing to store.
    pub fn expected_vs_baseline(
        &self,
        payoff: &[f64],
        floor: usize,
        baseline: &[u8],
        chosen: &mut [u8],
        deviates: &mut [bool],
    ) -> f64 {
        let width = (MAX_SCORE + 1) as usize;
        let mut best = vec![0.0f64; (self.dice as usize + 1) * width];
        for u in 1..=self.dice as usize {
            for (fi, &fs) in self.reachable[u].iter().enumerate() {
                let row = self.node_base[u] + fi * self.throws[u].len();
                if self.is_dead(u, fs, floor) {
                    // Nothing here can beat the banked best, so the whole
                    // subtree is one value and no action is ever needed.
                    best[u * width + fs] = payoff[0];
                    for t in 0..self.throws[u].len() {
                        chosen[row + t] = u8::MAX;
                        deviates[row + t] = false;
                    }
                    continue;
                }
                let mut acc = 0.0;
                for (ti, throw) in self.throws[u].iter().enumerate() {
                    let node = row + ti;
                    let v = if throw.choices.is_empty() {
                        chosen[node] = u8::MAX;
                        deviates[node] = false;
                        payoff[0]
                    } else {
                        let mut top = f64::NEG_INFINITY;
                        let mut arg = 0u8;
                        let mut base_value = f64::NEG_INFINITY;
                        let base = baseline[node];
                        for (ci, &(k, add)) in throw.choices.iter().enumerate()
                        {
                            let fs2 = fs + add as usize;
                            let u2 = u - k as usize;
                            let stop = payoff[fs2];
                            let (v, stopped) = if u2 == 0 {
                                (stop, true)
                            } else {
                                let go = best[u2 * width + fs2];
                                if stop >= go {
                                    (stop, true)
                                } else {
                                    (go, false)
                                }
                            };
                            let code =
                                (ci as u8) | if stopped { 0x80 } else { 0 };
                            if v > top {
                                top = v;
                                arg = code;
                            }
                            // The baseline names a freeze; whether it then
                            // stops is our choice, so match on the index.
                            if base != u8::MAX && (base & 0x7f) == ci as u8 {
                                base_value = v;
                            }
                        }
                        if base_value == f64::NEG_INFINITY {
                            base_value = top;
                        }
                        let dev = top - base_value > super::EPS;
                        deviates[node] = dev;
                        chosen[node] = if dev { arg } else { base };
                        top
                    };
                    acc += throw.weight as f64 * v;
                }
                best[u * width + fs] = acc / self.totals[u] as f64;
            }
        }
        best[self.dice as usize * width]
    }
}

/// `n! / (k0! k1! ...)` for small counts.
fn multinomial(counts: &[u32]) -> u64 {
    let n: u32 = counts.iter().sum();
    let mut r = factorial(n);
    for &c in counts {
        r /= factorial(c);
    }
    r
}

fn factorial(n: u32) -> u64 {
    (1..=u64::from(n)).product::<u64>().max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Weights must reconstruct every ordered roll.
    #[test]
    fn class_weights_sum_to_six_to_the_u() {
        for dice in [5u32, 6] {
            let a = Attempt::new(
                dice,
                if dice == 5 { [2, 4, 6] } else { [1, 3, 5] },
            );
            for u in 1..=dice as usize {
                let s: u64 = a.throws[u].iter().map(|t| t.weight).sum();
                assert_eq!(s, 6u64.pow(u as u32), "u={u}");
            }
        }
    }

    /// With payoff = score, this is the solo expected value, which
    /// `tests/attempt_engines.rs` pins independently.
    #[test]
    fn single_attempt_matches_the_solo_engine() {
        let identity: Vec<f64> = (0..=MAX_SCORE).map(f64::from).collect();
        let jav = Attempt::new(6, [1, 3, 5]).expected(&identity, 0);
        assert!(
            (jav - 15.218_094_840).abs() < 1e-9,
            "javelin single attempt = {jav}"
        );
        let dis = Attempt::new(5, [2, 4, 6]).expected(&identity, 0);
        assert!(
            (dis - 14.885_438_946).abs() < 1e-9,
            "discus single attempt = {dis}"
        );
    }
}

//! The Long Jump: a run-up that can bust, then a jump with whatever the
//! run-up carried.
//!
//! > "Run-up: ... freeze at least one die ... If the total of all frozen
//! > dice exceeds 8, you suffer an invalid attempt by stepping over. If
//! > you decide to stop throwing with a total of 8 or less ... you then
//! > jump. Jump: pick up your frozen dice and throw them all. Freeze at
//! > least one die and rethrow the remainder ... until you freeze all."
//!
//! Freezing is compulsory after every throw, which is the whole source of
//! risk: you commit to a rethrow *before* seeing it, and must then take
//! something. The first throw can never bust — the smallest of five dice
//! is at most 6 — so danger starts on the second.
//!
//! Two reductions keep the enumeration small, and without them this event
//! costs six times more:
//!
//! * **Run-up rolls collapse to their feasible prefix.** Only the `k`
//!   smallest are ever frozen, so a roll is seen through its sorted
//!   prefix sums; and once one exceeds the budget every larger `k` busts
//!   identically, so the whole tail above it is one undifferentiated
//!   lump. With a budget of 8, `{1,2,3,6,6}` and `{1,2,3,4,5}` are the
//!   same position. 252 five-dice multisets become 53.
//! * **Jump sums are confined.** With `r` dice still in hand at most
//!   `5 - r` are frozen, so the banked sum cannot exceed `6(5-r)`.

use super::attempt::AttemptEngine;
use super::compress::Stats;
use super::{best_of_three, Axis, EPS};

/// Dice in the event.
const DICE: usize = 5;
/// Frozen run-up total above which the attempt steps over.
const LIMIT: i32 = 8;
/// Five jump dice at six.
pub const MAX_SCORE: usize = DICE * 6;

/// A distinct throw during the jump: what the sorted faces allow.
struct JumpThrow {
    weight: u64,
    /// `top[j]` is the sum of the `j+1` largest dice.
    top: Vec<i32>,
}

/// A distinct throw during the run-up, reduced to its feasible prefix.
struct RunThrow {
    weight: u64,
    /// Running sums of the smallest dice that still fit the budget.
    /// Empty means every freeze busts, so the attempt is invalid.
    prefix: Vec<i32>,
}

/// The long jump's attempt engine.
pub struct LongJump {
    /// Reachable `(frozen count, frozen sum)` run-up positions.
    runup: Vec<(usize, i32)>,
    /// `runup_at[nf * (LIMIT + 1) + fs]` indexes into `runup`, or
    /// `usize::MAX` where that position is unreachable. Freezing dice is
    /// the inner loop of the whole solver, so this replaces a linear
    /// search over `runup` for the successor position.
    runup_at: Vec<usize>,
    /// Throws for each run-up position, in the same order.
    run_throws: Vec<Vec<RunThrow>>,
    /// Throws of `r` dice during the jump; index 0 unused.
    jump_throws: Vec<Vec<JumpThrow>>,
    /// Highest banked sum reachable with `r` dice left.
    jump_cap: Vec<usize>,
    /// Node offsets: run-up decisions, run-up throws, jump throws.
    run_choice_base: usize,
    run_throw_base: Vec<usize>,
    jump_base: Vec<Vec<usize>>,
    nodes: usize,
    reachable: Vec<bool>,
}

/// Store the chosen action at node `n`, defaulting to the baseline
/// wherever following it costs nothing.
fn record(
    policy: &mut Option<&mut [u8]>,
    deviates: &mut Option<&mut [bool]>,
    baseline: Option<&[u8]>,
    n: usize,
    best: u8,
    values: (f64, f64),
) {
    let (top, base_value) = values;
    let action = match (baseline, deviates.as_deref_mut()) {
        (Some(base), Some(dev)) => {
            let d = top - base_value > EPS;
            dev[n] = d;
            if d {
                best
            } else {
                base[n]
            }
        }
        _ => best,
    };
    if let Some(p) = policy.as_deref_mut() {
        p[n] = action;
    }
}

/// Every multiset of `n` dice as sorted faces, with its multiplicity.
fn multisets(n: usize) -> Vec<(Vec<i32>, u64)> {
    fn go(n: usize, lo: i32, cur: &mut Vec<i32>, out: &mut Vec<Vec<i32>>) {
        if cur.len() == n {
            out.push(cur.clone());
            return;
        }
        for f in lo..=6 {
            cur.push(f);
            go(n, f, cur, out);
            cur.pop();
        }
    }
    let mut out = Vec::new();
    go(n, 1, &mut Vec::new(), &mut out);
    out.into_iter()
        .map(|faces| {
            let mut w = (1..=faces.len() as u64).product::<u64>();
            let mut i = 0;
            while i < faces.len() {
                let mut j = i;
                while j < faces.len() && faces[j] == faces[i] {
                    j += 1;
                }
                w /= (1..=(j - i) as u64).product::<u64>();
                i = j;
            }
            (faces, w)
        })
        .collect()
}

impl Default for LongJump {
    fn default() -> Self {
        Self::new()
    }
}

impl LongJump {
    /// Build every table the solve needs.
    pub fn new() -> Self {
        // Reachable run-up positions: `nf` frozen dice sum to at least
        // `nf` and at most 8, and zero dice means zero sum.
        let mut runup = Vec::new();
        for nf in 0..DICE {
            if nf == 0 {
                runup.push((0, 0));
            } else {
                for fs in nf as i32..=LIMIT.min(6 * nf as i32) {
                    runup.push((nf, fs));
                }
            }
        }

        let mut runup_at = vec![usize::MAX; DICE * (LIMIT as usize + 1)];
        for (i, &(nf, fs)) in runup.iter().enumerate() {
            runup_at[nf * (LIMIT as usize + 1) + fs as usize] = i;
        }

        let mut run_throws = Vec::new();
        for &(nf, fs) in &runup {
            let budget = LIMIT - fs;
            let u = DICE - nf;
            let mut grouped: std::collections::BTreeMap<Vec<i32>, u64> =
                std::collections::BTreeMap::new();
            for (faces, w) in multisets(u) {
                let mut sum = 0;
                let mut prefix = Vec::new();
                for f in faces {
                    if sum + f > budget {
                        break;
                    }
                    sum += f;
                    prefix.push(sum);
                }
                *grouped.entry(prefix).or_default() += w;
            }
            run_throws.push(
                grouped
                    .into_iter()
                    .map(|(prefix, weight)| RunThrow { weight, prefix })
                    .collect::<Vec<_>>(),
            );
        }

        let mut jump_throws = vec![Vec::new()];
        for r in 1..=DICE {
            jump_throws.push(
                multisets(r)
                    .into_iter()
                    .map(|(faces, weight)| {
                        let mut top = Vec::with_capacity(r);
                        let mut acc = 0;
                        for f in faces.iter().rev() {
                            acc += f;
                            top.push(acc);
                        }
                        JumpThrow { weight, top }
                    })
                    .collect(),
            );
        }
        let jump_cap = (0..=DICE).map(|r| 6 * (DICE - r)).collect::<Vec<_>>();

        // Lay out policy nodes: run-up stop-or-throw, then run-up freeze
        // choices, then jump freeze choices.
        let run_choice_base = 0;
        let mut nodes = runup.len();
        let mut run_throw_base = Vec::new();
        for throws in &run_throws {
            run_throw_base.push(nodes);
            nodes += throws.len();
        }
        let mut jump_base = vec![Vec::new()];
        for r in 1..=DICE {
            let mut per_acc = Vec::new();
            for _ in 0..=jump_cap[r] {
                per_acc.push(nodes);
                nodes += jump_throws[r].len();
            }
            jump_base.push(per_acc);
        }

        let mut reachable = vec![false; MAX_SCORE + 1];
        reachable[0] = true; // stepping over
        for s in 1..=MAX_SCORE {
            reachable[s] = true;
        }
        Self {
            runup,
            runup_at,
            run_throws,
            jump_throws,
            jump_cap,
            run_choice_base,
            run_throw_base,
            jump_base,
            nodes,
            reachable,
        }
    }

    /// Solve one attempt, optionally recording and comparing a policy.
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

        let jump = self.solve_jump(
            payoff,
            floor,
            &mut policy,
            &mut deviates,
            baseline,
        );
        self.solve_runup(payoff, &jump, &mut policy, &mut deviates, baseline)
    }

    /// The jump phase: `r` dice in hand having banked `acc`. Solved for
    /// increasing `r`, because freezing only ever reduces the hand.
    fn solve_jump(
        &self,
        payoff: &[f64],
        floor: usize,
        policy: &mut Option<&mut [u8]>,
        deviates: &mut Option<&mut [bool]>,
        baseline: Option<&[u8]>,
    ) -> Vec<Vec<f64>> {
        let mut jump = vec![vec![0.0f64; MAX_SCORE + 1]; DICE + 1];
        jump[0].copy_from_slice(&payoff[..=MAX_SCORE]);
        for r in 1..=DICE {
            for acc in 0..=self.jump_cap[r] {
                // Nothing left in hand can beat the banked best, so the
                // whole subtree is one value and needs no action.
                if acc + 6 * r <= floor {
                    jump[r][acc] = payoff[0];
                    for ti in 0..self.jump_throws[r].len() {
                        let n = self.jump_base[r][acc] + ti;
                        if let Some(p) = policy.as_deref_mut() {
                            p[n] = u8::MAX;
                        }
                        if let Some(d) = deviates.as_deref_mut() {
                            d[n] = false;
                        }
                    }
                    continue;
                }
                let mut total = 0.0;
                let mut weight = 0u64;
                for (ti, throw) in self.jump_throws[r].iter().enumerate() {
                    let mut top = f64::NEG_INFINITY;
                    let mut arg = 0u8;
                    let mut base_value = f64::NEG_INFINITY;
                    let n = self.jump_base[r][acc] + ti;
                    let base = baseline.map_or(0, |b| b[n]);
                    for (j, &gained) in throw.top.iter().enumerate() {
                        let v = jump[r - j - 1][acc + gained as usize];
                        if v > top {
                            top = v;
                            arg = j as u8;
                        }
                        if j as u8 == base {
                            base_value = v;
                        }
                    }
                    if base_value == f64::NEG_INFINITY {
                        base_value = top;
                    }
                    record(
                        policy,
                        deviates,
                        baseline,
                        n,
                        arg,
                        (top, base_value),
                    );
                    total += throw.weight as f64 * top;
                    weight += throw.weight;
                }
                jump[r][acc] = total / weight as f64;
            }
        }

        jump
    }

    /// The run-up, solved for decreasing dice frozen: freezing more only
    /// ever moves forwards.
    fn solve_runup(
        &self,
        payoff: &[f64],
        jump: &[Vec<f64>],
        policy: &mut Option<&mut [u8]>,
        deviates: &mut Option<&mut [bool]>,
        baseline: Option<&[u8]>,
    ) -> f64 {
        let mut runup = vec![0.0f64; self.runup.len()];
        for (si, &(nf, fs)) in self.runup.iter().enumerate().rev() {
            let budget = LIMIT - fs;
            let u = DICE - nf;
            // Throw again, then take at least one of what shows.
            let mut total = 0.0;
            let mut weight = 0u64;
            for (ti, throw) in self.run_throws[si].iter().enumerate() {
                let n = self.run_throw_base[si] + ti;
                if throw.prefix.is_empty() {
                    // Nothing fits the budget: forced to step over.
                    if let Some(p) = policy.as_deref_mut() {
                        p[n] = u8::MAX;
                    }
                    if let Some(d) = deviates.as_deref_mut() {
                        d[n] = false;
                    }
                    total += throw.weight as f64 * payoff[0];
                    weight += throw.weight;
                    continue;
                }
                let mut top = f64::NEG_INFINITY;
                let mut arg = 0u8;
                let mut base_value = f64::NEG_INFINITY;
                let base = baseline.map_or(0, |b| b[n]);
                for (k, &added) in throw.prefix.iter().enumerate() {
                    let k = k + 1;
                    let v = if nf + k == DICE {
                        jump[DICE][0]
                    } else {
                        let slot = self.runup_at
                            [(nf + k) * (LIMIT as usize + 1) + (fs + added) as usize];
                        if slot == usize::MAX {
                            payoff[0] // stepped over
                        } else {
                            runup[slot]
                        }
                    };
                    if v > top {
                        top = v;
                        arg = (k - 1) as u8;
                    }
                    if (k - 1) as u8 == base {
                        base_value = v;
                    }
                }
                if base_value == f64::NEG_INFINITY {
                    base_value = top;
                }
                record(policy, deviates, baseline, n, arg, (top, base_value));
                total += throw.weight as f64 * top;
                weight += throw.weight;
            }
            let throw_value = if u == 0 {
                f64::NEG_INFINITY
            } else {
                total / weight as f64
            };
            // Or stop here and jump with what is frozen.
            let jump_now = if nf >= 1 {
                jump[nf][0]
            } else {
                f64::NEG_INFINITY
            };
            let _ = budget;
            let n = self.run_choice_base + si;
            let best_stop = jump_now >= throw_value;
            let base_stop = baseline.map_or(best_stop, |b| b[n] == 1);
            let used = if base_stop { jump_now } else { throw_value };
            record(
                policy,
                deviates,
                baseline,
                n,
                u8::from(best_stop),
                (jump_now.max(throw_value), used),
            );
            runup[si] = jump_now.max(throw_value);
        }
        runup[0]
    }
}

impl AttemptEngine for LongJump {
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
        // A jump position that cannot reach past the banked best carries
        // no useful decision; the run-up keeps too much optionality to
        // prune much.
        let mut n = self.runup.len();
        for throws in &self.run_throws {
            n += throws.iter().filter(|t| t.prefix.len() > 1).count();
        }
        for r in 1..=DICE {
            for acc in 0..=self.jump_cap[r] {
                if acc + 6 * r > floor && r > 1 {
                    n += self.jump_throws[r].len();
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
    best_of_three::solve_first_mover(&LongJump::new(), axis, after)
}

/// Policy storage statistics.
pub fn measure(axis: Axis, after: &(dyn Fn(i32) -> f64 + Sync)) -> Stats {
    best_of_three::measure(&LongJump::new(), axis, after)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A single attempt, scored by its own points, must match the value
    /// `tests/attempt_engines.rs` pins from a full-subset brute force.
    #[test]
    fn one_attempt_matches_the_brute_force() {
        let lj = LongJump::new();
        let payoff: Vec<f64> = (0..=MAX_SCORE).map(|s| s as f64).collect();
        let v = lj.expected(&payoff, 0);
        assert!((v - 16.942_648_747).abs() < 1e-9, "one attempt = {v}");
    }

    /// Best of three must match `tests/disciplines.rs`.
    #[test]
    fn reduces_to_the_solo_solver_under_a_linear_payoff() {
        let ev = best_of_three::solo_expected_value(&LongJump::new());
        assert!((ev - 22.394_956_800).abs() < 1e-9, "long jump EV = {ev}");
    }

    /// The first throw cannot bust: the smallest of five dice is at most
    /// 6, well under the limit of 8.
    #[test]
    fn the_first_throw_never_steps_over() {
        let lj = LongJump::new();
        assert!(lj.run_throws[0].iter().all(|t| !t.prefix.is_empty()));
    }
}

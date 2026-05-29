//! Probability distributions over integer scores and the shared
//! optimal-action rule used by every solver.
//!
//! Each discipline ultimately produces the exact distribution of its
//! final score under optimal play. Expected value, standard deviation
//! and the CDF are all derived from that distribution, so the solvers
//! never have to track moments separately.

use std::collections::BTreeMap;

/// Exact probability mass function over integer scores.
#[derive(Clone, Debug, Default)]
pub struct Dist {
    /// Map from score to probability. Probabilities sum to 1.
    pub mass: BTreeMap<i32, f64>,
}

impl Dist {
    /// A degenerate distribution placing all mass on `value`.
    #[must_use]
    pub fn point(value: i32) -> Self {
        let mut mass = BTreeMap::new();
        mass.insert(value, 1.0);
        Self { mass }
    }

    /// Add `prob` mass at `score`.
    pub fn add(&mut self, score: i32, prob: f64) {
        if prob != 0.0 {
            *self.mass.entry(score).or_insert(0.0) += prob;
        }
    }

    /// Merge `other` into `self`, scaling its mass by `weight`.
    pub fn mix_in(&mut self, other: &Self, weight: f64) {
        for (&score, &p) in &other.mass {
            self.add(score, p * weight);
        }
    }

    /// Expected value of the distribution.
    pub fn mean(&self) -> f64 {
        self.mass.iter().map(|(&s, &p)| s as f64 * p).sum()
    }

    /// Variance of the distribution.
    pub fn variance(&self) -> f64 {
        let m = self.mean();
        let ex2: f64 = self
            .mass
            .iter()
            .map(|(&s, &p)| (s as f64).powi(2) * p)
            .sum();
        (ex2 - m * m).max(0.0)
    }

    /// Standard deviation of the distribution.
    pub fn std_dev(&self) -> f64 {
        self.variance().sqrt()
    }

    /// Expected value of `g` applied to the score.
    pub fn expectation<F: Fn(i32) -> f64>(&self, g: F) -> f64 {
        self.mass.iter().map(|(&s, &p)| p * g(s)).sum()
    }

    /// Distribution of `max(self, floor)`: every outcome below `floor`
    /// is lifted to `floor`. Used to fold a prior best score into an
    /// attempt's result for best-of-N events.
    #[must_use]
    pub fn clamped_below(&self, floor: i32) -> Self {
        let mut out = Self::default();
        for (&s, &p) in &self.mass {
            out.add(s.max(floor), p);
        }
        out
    }
}

/// Pick the better of two candidate distributions under the standard
/// tiebreak used throughout: maximise expected value, then minimise
/// standard deviation, otherwise keep `a` (the "earlier"/stop action).
pub fn better(a: Dist, b: Dist) -> Dist {
    const EPS: f64 = 1e-12;
    let (ma, mb) = (a.mean(), b.mean());
    if mb > ma + EPS {
        return b;
    }
    if (mb - ma).abs() <= EPS && b.std_dev() < a.std_dev() - EPS {
        return b;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_and_variance_of_fair_die() {
        let mut d = Dist::default();
        for f in 1..=6 {
            d.add(f, 1.0 / 6.0);
        }
        assert!((d.mean() - 3.5).abs() < 1e-12);
        assert!((d.variance() - 35.0 / 12.0).abs() < 1e-12);
    }

    #[test]
    fn better_prefers_higher_mean() {
        let chosen = better(Dist::point(3), Dist::point(5));
        assert!((chosen.mean() - 5.0).abs() < 1e-12);
    }

    #[test]
    fn clamp_lifts_low_outcomes() {
        let d = Dist::point(2).clamped_below(5);
        assert!((d.mean() - 5.0).abs() < 1e-12);
    }
}

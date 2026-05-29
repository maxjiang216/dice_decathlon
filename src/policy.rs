//! The result of solving one discipline: the exact final-score
//! distribution under optimal play plus a serialisable summary.

use crate::dp::Dist;
use serde::Serialize;

/// Everything a solved discipline exposes to the rest of the program.
pub struct Solved {
    /// Short identifier, e.g. `"100m"`.
    pub key: &'static str,
    /// Human-readable name, e.g. `"100 Metres"`.
    pub name: &'static str,
    /// Exact distribution of the final (scored) result.
    pub dist: Dist,
}

/// Compact, serialisable summary written to `summary.json`.
#[derive(Serialize)]
pub struct Summary {
    pub key: String,
    pub name: String,
    pub expected_value: f64,
    pub std_dev: f64,
    pub min_score: i32,
    pub max_score: i32,
    /// Most likely outcome and its probability.
    pub mode_score: i32,
    pub mode_probability: f64,
}

impl Solved {
    /// # Panics
    ///
    /// Panics if the distribution is empty; every solver produces at
    /// least one outcome, so this does not happen in practice.
    pub fn summary(&self) -> Summary {
        let (&mode_score, &mode_probability) = self
            .dist
            .mass
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .expect("distribution is non-empty");
        Summary {
            key: self.key.to_string(),
            name: self.name.to_string(),
            expected_value: self.dist.mean(),
            std_dev: self.dist.std_dev(),
            min_score: *self.dist.mass.keys().next().unwrap(),
            max_score: *self.dist.mass.keys().next_back().unwrap(),
            mode_score,
            mode_probability,
        }
    }
}

//! How large is a two-player policy, and how far does it compress?
//!
//! The optimal action almost always agrees with the EV-maximising one,
//! because where the value function is locally linear in the difference
//! `d` the two objectives pick the same move. So a policy is stored as
//! the (difference-independent) EV policy plus the places that beat it.
//!
//! Ties resolve to the EV action deliberately. It minimises what has to
//! be stored, keeps a bot sensible once a match is decided, and plays
//! better against an opponent who errs — a position saturated at
//! `V = 0.9999` is still recoverable, and its probability assumed the
//! opponent would not slip.

use super::EPS;

/// Run-length statistics for one event's policy, gathered along the
/// difference axis.
#[derive(Default, Clone, Copy)]
pub struct Stats {
    /// Control states: everything indexing a decision except `d`.
    pub control: u64,
    /// Length of the difference axis.
    pub axis: u64,
    /// `(control, d)` pairs where the EV action is strictly worse.
    pub deviations: u64,
    /// Control states deviating for at least one `d`.
    pub dev_control: u64,
    /// Bits needed to name one action.
    pub action_bits: u64,
    /// Bytes to hold one axis position: 1 if it fits `i8`, else 2.
    pub idx_bytes: u64,
}

impl Stats {
    /// Every `(control, d)` pair.
    pub const fn states(&self) -> u64 {
        self.control * self.axis
    }
    /// One byte per state.
    pub const fn raw(&self) -> u64 {
        self.states()
    }
    /// Bit-packed to the action arity.
    pub const fn packed(&self) -> u64 {
        self.states().div_ceil(8)
    }
    /// EV baseline, one action per control state.
    pub const fn baseline(&self) -> u64 {
        (self.control * self.action_bits).div_ceil(8)
    }
    /// Baseline plus one interval slot for every control state.
    pub const fn dense(&self) -> u64 {
        self.baseline() + self.control * (2 * self.idx_bytes + self.payload())
    }
    /// Baseline plus an addressed interval per deviating control state.
    pub const fn sparse(&self) -> u64 {
        self.baseline()
            + self.dev_control * (4 + 2 * self.idx_bytes + self.payload())
    }
    /// A binary choice implies its own deviation, so it needs no action
    /// byte; anything wider does.
    const fn payload(&self) -> u64 {
        if self.action_bits <= 1 {
            0
        } else {
            1
        }
    }
}

/// Accumulates run lengths while sweeping the difference axis in order.
pub struct RunCounter {
    previous: Vec<u8>,
    /// Whether each control state has ever deviated.
    deviated: Vec<bool>,
    started: bool,
    /// Deviating `(control, d)` pairs seen so far.
    pub deviations: u64,
}

impl RunCounter {
    /// Track `control` states.
    pub fn new(control: usize) -> Self {
        Self {
            previous: vec![0; control],
            deviated: vec![false; control],
            started: false,
            deviations: 0,
        }
    }

    /// Feed one difference's worth of chosen actions.
    ///
    /// `chosen[i]` is the optimal action and `deviates[i]` says the EV
    /// action was strictly worse there; where it was not, the EV action
    /// is stored instead, which is what keeps the runs long.
    pub fn push(&mut self, chosen: &[u8], deviates: &[bool]) {
        for i in 0..chosen.len() {
            if deviates[i] {
                self.deviations += 1;
                self.deviated[i] = true;
            }
        }
        self.previous.copy_from_slice(chosen);
        self.started = true;
    }

    /// Control states that deviated somewhere.
    pub fn dev_control(&self) -> u64 {
        self.deviated.iter().filter(|d| **d).count() as u64
    }
}

/// Whether choosing `ev_value` instead of the best available costs
/// anything worth recording.
pub fn is_deviation(best: f64, ev_value: f64) -> bool {
    best - ev_value > EPS
}

/// One row of the storage comparison.
pub fn report(name: &str, s: Stats) -> String {
    format!(
        "{name:9} {states:>13} {raw:>12} {packed:>11} {dense:>10} {sparse:>10}  \
         {dev:>6.2}%  {ratio:>5.1}x",
        states = s.states(),
        raw = s.raw(),
        packed = s.packed(),
        dense = s.dense(),
        sparse = s.sparse(),
        dev = 100.0 * s.deviations as f64 / s.states() as f64,
        ratio = s.packed() as f64 / s.dense().min(s.sparse()) as f64,
    )
}

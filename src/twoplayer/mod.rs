//! Two-player optimal play: maximise the probability of finishing the
//! decathlon ahead, rather than the expected score of a single event.
//!
//! The whole game couples through one number. Let `d` be the running
//! point difference from *completed* events; then each event is solved
//! against the value function of the events after it, and the only thing
//! crossing an event boundary is a vector of win probabilities indexed
//! by `d`. See `worklog/2026-08-09-two-player-optimal-play/`.
//!
//! Everything here is `f64`. A win probability is compared against its
//! alternatives at a tolerance of [`EPS`], which is below `f32`
//! resolution near 1.0, and values are chained through ten events.

pub mod advisor;
pub mod attempt;
pub mod best_of_three;
pub mod chain;
pub mod compress;
pub mod discus;
pub mod freeze;
pub mod highjump;
pub mod javelin;
pub mod ladder;
pub mod longjump;
pub mod m1500;
pub mod polevault;
pub mod reroll_sets;
pub mod running;
pub mod shotput;

/// Values within this of each other count as tied, and values this close
/// to 0 or 1 are clamped to exactly 0 or 1.
///
/// Clamping stops rounding error compounding across ten chained events
/// and makes the decided-outcome regions exact rather than approximate.
/// The measured policy is insensitive to the exact figure: 1e-12 and
/// 1e-9 give identical output on the 1500m.
pub const EPS: f64 = 1e-9;

/// Snap a win probability to exactly 0 or 1 when it is within [`EPS`].
pub fn clamp_prob(v: f64) -> f64 {
    if v < EPS {
        0.0
    } else if v > 1.0 - EPS {
        1.0
    } else {
        v
    }
}

/// An inclusive range of point differences, used to index value vectors.
///
/// Lookups outside the range are clamped to its ends. That is exact
/// rather than approximate provided the range extends past the point
/// where the value function saturates at 0 and 1, which is what
/// [`Axis::for_event`] guarantees.
#[derive(Clone, Copy, Debug)]
pub struct Axis {
    /// Most negative difference represented.
    pub lo: i32,
    /// Most positive difference represented.
    pub hi: i32,
}

impl Axis {
    /// Build an axis spanning `-half ..= half`.
    pub const fn symmetric(half: i32) -> Self {
        Self {
            lo: -half,
            hi: half,
        }
    }

    /// The non-negative half, `0 ..= half`.
    ///
    /// This is all a *first mover* ever needs. The rulebook has the
    /// leading player start, so whoever moves first is by definition not
    /// behind, and a first-mover value function is never queried at a
    /// negative difference. Positions where our player moves second are
    /// reached by relabelling — see [`apply_turn_order`] — not by
    /// mirroring the value function, which would be wrong: moving second
    /// carries an information advantage, so `F` is genuinely asymmetric.
    pub const fn first_mover(half: i32) -> Self {
        Self { lo: 0, hi: half }
    }

    /// Axis wide enough for an event, given how far `d` can already have
    /// drifted (`acc`) and how much swing the remaining events still
    /// hold (`rem`).
    ///
    /// A difference larger than `rem` cannot be overturned, so the value
    /// there is already 0 or 1 and need not be stored — this is the
    /// "dead zone" that keeps the late events cheap. `widen` is the
    /// largest contribution the event itself can fold into `d` mid-play.
    pub fn for_event(acc: i32, rem: i32, widen: i32) -> Self {
        Self::symmetric((acc + widen).min(rem))
    }

    /// Number of representable differences.
    pub const fn len(&self) -> usize {
        (self.hi - self.lo + 1) as usize
    }

    /// Whether the axis holds no values. Never true as constructed, but
    /// clippy asks for it alongside `len`.
    pub const fn is_empty(&self) -> bool {
        self.hi < self.lo
    }

    /// Index of difference `d`, clamped into range.
    pub fn idx(&self, d: i32) -> usize {
        (d.clamp(self.lo, self.hi) - self.lo) as usize
    }

    /// The difference stored at `i`.
    pub const fn at(&self, i: usize) -> i32 {
        self.lo + i as i32
    }

    /// Iterate every difference in the axis.
    pub fn iter(&self) -> impl Iterator<Item = i32> {
        self.lo..=self.hi
    }
}

/// Terminal payoff of the whole decathlon: did the player finish ahead?
///
/// A tie is scored as half a win. The rulebook settles turn order ties
/// with a die roll but says nothing about a tied final total, so this is
/// the natural reading and it is what makes the player-swap symmetry
/// `V(d) = 1 - V(-d)` hold exactly.
pub const fn final_payoff(d: i32) -> f64 {
    match d.signum() {
        1 => 1.0,
        0 => 0.5,
        _ => 0.0,
    }
}

/// Turn the *first mover's* win probability into the win probability of
/// a nominated player, applying the rulebook's turn-order rule.
///
/// > "From the second discipline onwards, the leading player always
/// > starts... Ties are resolved by the throw of a die."
///
/// With two players "who leads" is the sign of `d`, so no extra state is
/// needed. At a tie the die roll makes it an even mix of both orderings,
/// which is exactly a half.
pub fn apply_turn_order(first_mover: &[f64], axis: Axis) -> Vec<f64> {
    axis.iter()
        .map(|d| match d.signum() {
            1 => first_mover[axis.idx(d)],
            -1 => 1.0 - first_mover[axis.idx(-d)],
            _ => 0.5,
        })
        .map(clamp_prob)
        .collect()
}

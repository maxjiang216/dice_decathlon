//! The 1500 Metres, solved for two players.
//!
//! This is the only event solvable on its own: it is last, so the value
//! of everything after it is just "am I ahead". Every earlier event
//! needs the events behind it solved first.
//!
//! Within the event the players do not interleave — the rulebook only
//! interleaves *attempts* and *heights*, and the 1500m is one attempt —
//! so the leader plays all eight dice, then the trailer plays theirs
//! knowing exactly what they must beat.

use super::{clamp_prob, final_payoff, reroll_sets::RerollSets, Axis};

/// Eight dice thrown one at a time, five shared rerolls, a six
/// subtracts six.
fn event() -> RerollSets {
    let (set_scores, total) = RerollSets::set_score_distribution(1, true);
    RerollSets {
        sets: 8,
        rerolls: 5,
        set_scores,
        total,
    }
}

/// Win probability of the player *about to move second*, as a function
/// of the difference they face once the first player has finished.
///
/// Indexed by [`Axis::idx`] on the returned axis.
fn second_mover(axis: Axis) -> Vec<f64> {
    // `n` is the difference from the second mover's point of view: their
    // frozen score has already been added, so finishing above zero wins.
    let v = event().solve(axis, &final_payoff);
    v.into_iter().map(clamp_prob).collect()
}

/// Win probability of the player who moves *first* in the 1500m, for
/// every difference on `axis`.
///
/// This is `V₉`, the value entering the final event, and the input the
/// javelin solver needs.
///
/// Moving first is a disadvantage here: the second player knows exactly
/// what they must beat, which turns their event into a pure target
/// problem. Since the rulebook has the *leading* player start, that
/// hands the information advantage to whoever is behind.
pub fn solve(axis: Axis) -> Vec<f64> {
    // The second mover faces the difference the first mover leaves
    // behind, which can exceed `axis` by a whole event's swing.
    let wide = Axis::symmetric(axis.hi + 48);
    let second = second_mover(wide);

    // The first mover's payoff is whatever the second mover fails to
    // win. Their own score has already been folded into `n`, and the
    // sign flips because `second` is written from the opponent's side.
    let terminal = move |n: i32| 1.0 - second[wide.idx(-n)];
    let v = event().solve(axis, &terminal);
    v.into_iter().map(clamp_prob).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The audit in `worklog/2026-08-09-two-player-optimal-play/` derived
    /// the axis combinatorially; this checks the solver agrees.
    #[test]
    fn axis_matches_the_audit() {
        // Entering the 1500m, 417 points of swing have accumulated but
        // only 88 remain, so the dead zone pins the axis at 2*88+1.
        let axis = Axis::for_event(417, 88, 83);
        assert_eq!(axis.len(), 177);
    }

    /// Being level is worth less than half when you have to move first.
    ///
    /// The figure is pinned rather than bounded because an independent
    /// Python implementation, sharing no code with this one, produced
    /// the same value — see the worklog entry on the 1500m probe.
    #[test]
    fn moving_first_is_a_disadvantage() {
        let axis = Axis::symmetric(88);
        let v = solve(axis);
        let level = v[axis.idx(0)];
        assert!(
            (level - 0.448_944).abs() < 5e-6,
            "V9(0) = {level}, expected 0.448944"
        );
    }

    /// The player moving second faces a pure target problem, and its
    /// value behaves like one: monotone in the margin, and pinned at the
    /// ends once the target is out of reach in either direction.
    ///
    /// Note there is no complementarity to test between the two movers.
    /// They are different decision problems — the second mover has seen
    /// the first mover's score — so `P(first wins | d)` and
    /// `P(second wins | -d)` are not two views of one quantity.
    #[test]
    fn the_second_mover_solves_a_target_problem() {
        let axis = Axis::symmetric(136);
        let second = second_mover(axis);
        for w in second.windows(2) {
            assert!(
                w[1] >= w[0] - 1e-12,
                "not monotone: {} then {}",
                w[0],
                w[1]
            );
        }
        // Eight dice score at most 40 and at least -48, so a margin
        // outside that can no longer be closed.
        assert!(second[axis.idx(-60)].abs() < f64::EPSILON);
        assert!((second[axis.idx(60)] - 1.0).abs() < f64::EPSILON);
    }

    /// A bigger lead is never worth less.
    #[test]
    fn value_is_monotone_in_the_difference() {
        let axis = Axis::symmetric(88);
        let v = solve(axis);
        for w in v.windows(2) {
            assert!(
                w[1] >= w[0] - 1e-12,
                "not monotone: {} then {}",
                w[0],
                w[1]
            );
        }
    }

    /// Leads past the remaining swing are decided, either way.
    #[test]
    fn the_dead_zone_saturates() {
        let axis = Axis::symmetric(88);
        let v = solve(axis);
        assert!((v[axis.idx(88)] - 1.0).abs() < f64::EPSILON);
        assert!(v[axis.idx(-88)].abs() < f64::EPSILON);
    }
}

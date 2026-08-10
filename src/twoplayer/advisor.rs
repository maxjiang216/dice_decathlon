//! What should I do here?
//!
//! One query answers two features: the computer opponents pick their
//! moves with it, and the "best move" hint shows the human the same
//! thing. Both need optimal play *for a given score difference*, which is
//! what separates this from the expected-value solvers.
//!
//! With more than two players the difference is taken against a single
//! chosen rival — see [`Advisor::rival`] — because the exact three- and
//! four-player game is out of reach (long jump alone would run to about
//! 1e13 states).

use super::chain::Chain;
use super::reroll_sets::{Decision, RerollSets};
use super::Axis;

/// How a computer player picks moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// Play the two-player optimum against whichever rival matters.
    Optimal,
    /// Maximise the event's own expected score, ignoring the scoreboard.
    ///
    /// This is what the `disciplines::` solvers answer, and it is a
    /// genuinely strong player — optimal play departs from it in only a
    /// few percent of positions in most events.
    EvMaximiser,
}

/// A solved competition, ready to answer move queries.
pub struct Advisor {
    chain: Chain,
}

impl Advisor {
    /// Solve the whole competition. Takes about half a minute.
    pub fn new() -> Self {
        Self {
            chain: Chain::solve(),
        }
    }

    /// The rival a player measures themselves against.
    ///
    /// Against one opponent there is no choice. With three or four
    /// players the two-player solution is applied to the current leader —
    /// or, when the player *is* the leader, to whoever is second, since
    /// that is the score they must stay ahead of. Ties break towards the
    /// earlier seat, which only matters for which of two equal rivals is
    /// named, not for the difference itself.
    pub fn rival(totals: &[i32], me: usize) -> Option<usize> {
        let leader = (0..totals.len())
            .filter(|&i| i != me)
            .max_by_key(|&i| totals[i])?;
        Some(leader)
    }

    /// Score difference a player is defending, against their rival.
    pub fn difference(totals: &[i32], me: usize) -> i32 {
        Self::rival(totals, me).map_or(0, |r| totals[me] - totals[r])
    }

    /// Whether to freeze the set on the table, in a reroll-set event.
    ///
    /// `frozen` is the player's score from sets already locked this
    /// event, `showing` the score of the set just thrown.
    ///
    /// Returns `true` to freeze. With no rethrows left there is nothing
    /// to decide and this always freezes.
    pub fn freeze_reroll_set(
        &self,
        event: usize,
        geometry: &RerollSets,
        style: Style,
        state: (usize, usize, i32),
        frozen_and_difference: (i32, i32),
    ) -> bool {
        let (set, rerolls_left, showing) = state;
        let (frozen, difference) = frozen_and_difference;
        if rerolls_left == 0 || set + 1 > geometry.sets as usize {
            return true;
        }
        // The expected-value player ignores the scoreboard entirely, so
        // its terminal payoff is just "more points is better".
        let axis = self.chain.axis(event);
        let (terminal, wide): (Box<dyn Fn(i32) -> f64 + Sync>, Axis) =
            match style {
                Style::EvMaximiser => (Box::new(f64::from), axis),
                Style::Optimal => {
                    let after = self.chain.clone_after(event);
                    (Box::new(after), axis)
                }
            };
        let layers = geometry.solve_layers(wide, terminal.as_ref());
        let at = Decision {
            set,
            r: rerolls_left,
            score: showing,
            n: difference + frozen,
        };
        let (freeze, reroll) =
            geometry.branch_values(&layers, wide, terminal.as_ref(), at);
        freeze >= reroll
    }
}

impl Default for Advisor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With two players the rival is the other player, whatever the
    /// scores.
    #[test]
    fn heads_up_has_one_rival() {
        assert_eq!(Advisor::rival(&[10, 20], 0), Some(1));
        assert_eq!(Advisor::rival(&[30, 20], 0), Some(1));
        assert_eq!(Advisor::difference(&[30, 20], 0), 10);
        assert_eq!(Advisor::difference(&[10, 20], 0), -10);
    }

    /// With more players the rival is the leader — unless that is me, in
    /// which case it is whoever is second and therefore the score I have
    /// to stay ahead of.
    #[test]
    fn the_rival_is_the_leader_or_the_runner_up() {
        let totals = [50, 90, 70];
        // Trailing: measure against the leader.
        assert_eq!(Advisor::rival(&totals, 0), Some(1));
        assert_eq!(Advisor::difference(&totals, 0), -40);
        // Leading: measure against second place.
        assert_eq!(Advisor::rival(&totals, 1), Some(2));
        assert_eq!(Advisor::difference(&totals, 1), 20);
        // Also trailing, from the middle seat.
        assert_eq!(Advisor::rival(&totals, 2), Some(1));
        assert_eq!(Advisor::difference(&totals, 2), -20);
    }

    /// A lone player has nobody to measure against.
    #[test]
    fn a_solo_player_has_no_rival() {
        assert_eq!(Advisor::rival(&[10], 0), None);
        assert_eq!(Advisor::difference(&[10], 0), 0);
    }
}

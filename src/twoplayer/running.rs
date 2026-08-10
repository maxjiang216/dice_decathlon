//! The four events whose only choice is freeze-all or rethrow-all:
//! 100 Metres, 400 Metres, 110 Metre Hurdles and the 1500 Metres.
//!
//! All four are one attempt, so the players do not interleave — the
//! rulebook interleaves attempts and heights, and these have neither.
//! The first mover plays their whole event, then the second plays theirs
//! knowing exactly what to beat.
//!
//! Because a rethrow picks up *every* die in the set, two rolls with the
//! same score are the same position, so the roll collapses to a scalar.
//! That is the whole reason these events cost thousands of states rather
//! than millions.

use super::reroll_sets::RerollSets;

/// 100 Metres: two sets of four, five shared rethrows, a six subtracts.
pub fn m100() -> RerollSets {
    RerollSets::new(2, 4, 5, true)
}

/// 400 Metres: four sets of two.
pub fn m400() -> RerollSets {
    RerollSets::new(4, 2, 5, true)
}

/// 110 Metre Hurdles: one set of five, scored as a plain sum.
///
/// The only running event with no six penalty, and so the only event in
/// the game that cannot score zero — its floor is 5.
pub fn hurdles() -> RerollSets {
    RerollSets::new(1, 5, 5, false)
}

/// 1500 Metres: eight sets of one.
pub fn m1500() -> RerollSets {
    RerollSets::new(8, 1, 5, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The distinct set scores are what the roll collapses to, and the
    /// collapse is where these events get cheap.
    #[test]
    fn rolls_collapse_to_their_scores() {
        // 4 dice have 126 face-count vectors but only 37 distinct scores;
        // 5 hurdles dice have 252 vectors and only 26 sums.
        assert_eq!(m100().set_scores.len(), 37);
        assert_eq!(m400().set_scores.len(), 15);
        assert_eq!(hurdles().set_scores.len(), 26);
        assert_eq!(m1500().set_scores.len(), 6);
    }

    /// Score ranges, each checked against the rulebook's dice.
    #[test]
    fn score_ranges_match_the_rules() {
        for (ev, lo, hi) in [
            (m100(), -48, 40),
            (m400(), -48, 40),
            (hurdles(), 5, 30),
            (m1500(), -48, 40),
        ] {
            let sets = i32::from(ev.sets);
            assert_eq!(ev.set_scores[0].0 * sets, lo);
            assert_eq!(ev.set_scores[ev.set_scores.len() - 1].0 * sets, hi);
        }
    }
}

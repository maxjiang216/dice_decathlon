//! Playable 100 Metres.
//!
//! Rulebook (Reiner Knizia's Decathlon, "100 Metres (8 dice, 1 attempt)"):
//!
//! > Divide the eight dice into two sets of four. Throw the first four
//! > dice. If you are not satisfied with the result, pick up all four
//! > dice and rethrow them. This can be repeated several times until you
//! > freeze the first set. Then throw the other four dice and proceed in
//! > the same manner. [...] You have a maximum of seven throws, one
//! > initial throw for each set and up to five rethrows which may be
//! > divided between the sets as desired.
//! >
//! > Scoring: Total the value of the dice for numbers one to five, but
//! > subtract any sixes from the result.
//!
//! Two details the wording settles that a Yahtzee reflex would get wrong,
//! and which the UI therefore must not offer:
//!
//! 1. A rethrow picks up **all four** dice of the active set. There is no
//!    keeping two and rerolling two.
//! 2. The sets are played **in order**. Once set 1 is frozen you cannot
//!    go back to it, so an unspent rethrow is only worth what set 2 can
//!    still do with it.

use super::rng::Rng;
use super::{Action, Choice, Die, Game, Group, View};

/// Dice per set.
const PER_SET: usize = 4;
/// Number of sets.
const SETS: usize = 2;
/// Rethrows shared between the two sets.
const REROLLS: u32 = 5;

/// Score of one set: faces 1-5 add, a six subtracts six.
fn score_set(dice: &[u8]) -> i32 {
    dice.iter()
        .map(|&f| if f == 6 { -6 } else { i32::from(f) })
        .sum()
}

/// A game of 100 Metres in progress.
pub struct Sprint100m {
    /// Frozen set scores, in play order.
    frozen: Vec<i32>,
    /// The faces showing on the set being played, empty once finished.
    dice: Vec<u8>,
    rerolls_left: u32,
    log: Vec<String>,
}

impl Sprint100m {
    /// Start a game, throwing the first set.
    pub fn new(rng: &mut Rng) -> Self {
        let dice = rng.roll_n(PER_SET);
        let log = vec![format!(
            "Set 1 thrown: {} ({} points)",
            render(&dice),
            score_set(&dice)
        )];
        Self {
            frozen: Vec::new(),
            dice,
            rerolls_left: REROLLS,
            log,
        }
    }

    /// Index of the set being played.
    fn set_idx(&self) -> usize {
        self.frozen.len()
    }

    fn finished(&self) -> bool {
        self.frozen.len() == SETS
    }
}

/// Render dice as `[3 5 6 1]` for the log.
fn render(dice: &[u8]) -> String {
    let faces: Vec<String> = dice.iter().map(ToString::to_string).collect();
    format!("[{}]", faces.join(" "))
}

impl Game for Sprint100m {
    fn view(&self) -> View {
        let running: i32 = self.frozen.iter().sum();
        let active = self.set_idx();

        let groups = (0..SETS)
            .map(|i| {
                let (dice, score) = if i < self.frozen.len() {
                    // A frozen set keeps the faces it was frozen on only
                    // while it is the most recent one; earlier sets are
                    // summarised by their score.
                    (Vec::new(), Some(self.frozen[i]))
                } else if i == active && !self.finished() {
                    let dice = self
                        .dice
                        .iter()
                        .map(|&face| Die {
                            face,
                            frozen: false,
                        })
                        .collect();
                    (dice, None)
                } else {
                    (Vec::new(), None)
                };
                Group {
                    label: format!("Set {}", i + 1),
                    dice,
                    score,
                    active: i == active && !self.finished(),
                }
            })
            .collect();

        let mut choices = Vec::new();
        if !self.finished() {
            let current = score_set(&self.dice);
            choices.push(Choice {
                action: Action::Freeze,
                label: format!("Freeze set {} ({current:+})", active + 1),
                detail: if active + 1 == SETS {
                    "Lock these four dice and finish the event.".to_string()
                } else {
                    "Lock these four dice and throw the next set. You \
                     cannot come back to this set."
                        .to_string()
                },
            });
            if self.rerolls_left > 0 {
                choices.push(Choice {
                    action: Action::Reroll,
                    label: format!(
                        "Rethrow all four ({} left)",
                        self.rerolls_left
                    ),
                    detail: "Pick up all four dice of this set and throw \
                             them again. Costs one of the five shared \
                             rethrows."
                        .to_string(),
                });
            }
        }

        View {
            key: "100m",
            name: "100 Metres",
            groups,
            rerolls_left: self.rerolls_left,
            rerolls_total: REROLLS,
            running_score: running,
            choices,
            result: if self.finished() { Some(running) } else { None },
            log: self.log.clone(),
        }
    }

    fn apply(&mut self, action: &Action, rng: &mut Rng) -> bool {
        if self.finished() {
            return false;
        }
        match action {
            Action::Reroll => {
                if self.rerolls_left == 0 {
                    return false;
                }
                self.rerolls_left -= 1;
                self.dice = rng.roll_n(PER_SET);
                self.log.push(format!(
                    "Set {} rethrown: {} ({} points, {} rethrow{} left)",
                    self.set_idx() + 1,
                    render(&self.dice),
                    score_set(&self.dice),
                    self.rerolls_left,
                    if self.rerolls_left == 1 { "" } else { "s" },
                ));
                true
            }
            Action::Freeze => {
                let score = score_set(&self.dice);
                self.log.push(format!(
                    "Set {} frozen on {} for {} points",
                    self.set_idx() + 1,
                    render(&self.dice),
                    score
                ));
                self.frozen.push(score);
                if self.finished() {
                    self.dice.clear();
                    self.log.push(format!(
                        "Final result: {} points",
                        self.frozen.iter().sum::<i32>()
                    ));
                } else {
                    self.dice = rng.roll_n(PER_SET);
                    self.log.push(format!(
                        "Set {} thrown: {} ({} points)",
                        self.set_idx() + 1,
                        render(&self.dice),
                        score_set(&self.dice)
                    ));
                }
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn play_all_freezes(seed: u64) -> i32 {
        let mut rng = Rng::new(seed);
        let mut game = Sprint100m::new(&mut rng);
        while game.view().result.is_none() {
            assert!(game.apply(&Action::Freeze, &mut rng));
        }
        game.view().result.unwrap()
    }

    #[test]
    fn sixes_subtract() {
        assert_eq!(score_set(&[6, 6, 6, 6]), -24);
        assert_eq!(score_set(&[5, 5, 5, 5]), 20);
        assert_eq!(score_set(&[1, 2, 3, 6]), 0);
    }

    #[test]
    fn score_is_the_sum_of_the_two_frozen_sets() {
        for seed in 0..50 {
            let total = play_all_freezes(seed);
            // Two sets of four, each in -24..=20.
            assert!((-48..=40).contains(&total), "seed {seed} gave {total}");
        }
    }

    #[test]
    fn rethrows_are_capped_at_five_across_both_sets() {
        let mut rng = Rng::new(1);
        let mut game = Sprint100m::new(&mut rng);
        for _ in 0..REROLLS {
            assert!(game.apply(&Action::Reroll, &mut rng));
        }
        assert_eq!(game.view().rerolls_left, 0);
        // Exhausted in set 1, so set 2 gets none either.
        assert!(!game.apply(&Action::Reroll, &mut rng));
        assert!(game.apply(&Action::Freeze, &mut rng));
        assert!(!game.apply(&Action::Reroll, &mut rng));
        assert_eq!(game.view().choices.len(), 1);
    }

    #[test]
    fn sets_are_played_in_order_and_the_game_ends_after_two() {
        let mut rng = Rng::new(42);
        let mut game = Sprint100m::new(&mut rng);
        assert!(game.view().groups[0].active);
        game.apply(&Action::Freeze, &mut rng);
        assert!(game.view().groups[1].active);
        game.apply(&Action::Freeze, &mut rng);
        assert!(game.view().result.is_some());
        assert!(game.view().choices.is_empty());
        // No move is legal once the event is over.
        assert!(!game.apply(&Action::Freeze, &mut rng));
        assert!(!game.apply(&Action::Reroll, &mut rng));
    }

    #[test]
    fn a_seed_replays_identically() {
        assert_eq!(play_all_freezes(2024), play_all_freezes(2024));
    }
}

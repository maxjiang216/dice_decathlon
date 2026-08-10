//! The four playable events whose only choice is freeze-all or
//! rethrow-all: 100 Metres, 400 Metres, 110 Metre Hurdles, 1500 Metres.
//!
//! They differ only in how the eight (or five) dice are grouped and
//! whether a six subtracts. The rulebook wording that matters, and which
//! the UI must therefore not contradict:
//!
//! > If you are not satisfied with the result, pick up **all** [the dice
//! > of the set] and rethrow them. This can be repeated several times
//! > until you freeze the first set. Then throw the other [...] and
//! > proceed in the same manner.
//!
//! Two details a Yahtzee reflex gets wrong: a rethrow picks up *every*
//! die in the set, and the sets are played **in order**, so an unspent
//! rethrow is only worth what the sets still to come can do with it.

use super::rng::Rng;
use super::{Action, Choice, Die, Game, Group, View};

/// Geometry and scoring of one running event.
#[derive(Clone, Copy)]
pub struct Rules {
    pub key: &'static str,
    pub name: &'static str,
    /// Dice thrown per set.
    pub per_set: usize,
    /// Sets played in order.
    pub sets: usize,
    /// Rethrows shared across every set.
    pub rerolls: u32,
    /// Whether a six subtracts six instead of adding it.
    ///
    /// True for every running event except the hurdles, which is the one
    /// scored as a plain sum — and so the one event that cannot score
    /// zero, its floor being 5.
    pub six_penalty: bool,
    /// What the UI calls one group.
    pub group_word: &'static str,
}

/// The four events, in rulebook order.
pub const RUNNING: [Rules; 4] = [
    Rules {
        key: "100m",
        name: "100 Metres",
        per_set: 4,
        sets: 2,
        rerolls: 5,
        six_penalty: true,
        group_word: "Set",
    },
    Rules {
        key: "400m",
        name: "400 Metres",
        per_set: 2,
        sets: 4,
        rerolls: 5,
        six_penalty: true,
        group_word: "Set",
    },
    Rules {
        key: "110mh",
        name: "110 Metre Hurdles",
        per_set: 5,
        sets: 1,
        rerolls: 5,
        six_penalty: false,
        group_word: "Throw",
    },
    Rules {
        key: "1500m",
        name: "1500 Metres",
        per_set: 1,
        sets: 8,
        rerolls: 5,
        six_penalty: true,
        group_word: "Die",
    },
];

/// Look up a running event by key.
pub fn rules(key: &str) -> Option<Rules> {
    RUNNING.iter().copied().find(|r| r.key == key)
}

/// A running event in progress.
pub struct Running {
    rules: Rules,
    /// Scores of the sets already locked, in play order.
    frozen: Vec<i32>,
    /// Faces showing on the set being played.
    dice: Vec<u8>,
    rerolls_left: u32,
    log: Vec<String>,
}

impl Running {
    /// Start the event, throwing the first set.
    pub fn new(rules: Rules, rng: &mut Rng) -> Self {
        let dice = rng.roll_n(rules.per_set);
        let log = vec![format!(
            "{} 1 thrown: {} ({} points)",
            rules.group_word,
            render(&dice),
            score_set(&dice, rules.six_penalty)
        )];
        Self {
            rules,
            frozen: Vec::new(),
            dice,
            rerolls_left: rules.rerolls,
            log,
        }
    }

    /// Index of the set being played.
    fn set_idx(&self) -> usize {
        self.frozen.len()
    }

    fn finished(&self) -> bool {
        self.frozen.len() == self.rules.sets
    }

    /// Score of the set currently on the table.
    pub fn showing(&self) -> i32 {
        score_set(&self.dice, self.rules.six_penalty)
    }

    /// Points already locked this event.
    pub fn locked(&self) -> i32 {
        self.frozen.iter().sum()
    }

    /// Rethrows still in the shared pool.
    pub const fn rerolls_left(&self) -> u32 {
        self.rerolls_left
    }
}

/// Score one set: faces 1-5 add; a six subtracts six unless the event
/// scores a plain sum.
fn score_set(dice: &[u8], six_penalty: bool) -> i32 {
    dice.iter()
        .map(|&f| {
            if six_penalty && f == 6 {
                -6
            } else {
                i32::from(f)
            }
        })
        .sum()
}

/// Render dice as `[3 5 6 1]` for the log.
fn render(dice: &[u8]) -> String {
    let faces: Vec<String> = dice.iter().map(ToString::to_string).collect();
    format!("[{}]", faces.join(" "))
}

impl Game for Running {
    fn view(&self) -> View {
        let running = self.locked();
        let active = self.set_idx();
        let r = self.rules;

        let groups = (0..r.sets)
            .map(|i| {
                let (dice, score) = if i < self.frozen.len() {
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
                    label: format!("{} {}", r.group_word, i + 1),
                    dice,
                    score,
                    active: i == active && !self.finished(),
                }
            })
            .collect();

        let mut choices = Vec::new();
        if !self.finished() {
            let current = self.showing();
            let noun = if r.per_set == 1 {
                "die".to_string()
            } else {
                format!("all {}", r.per_set)
            };
            choices.push(Choice {
                action: Action::Freeze,
                label: format!(
                    "Freeze {} {} ({current:+})",
                    r.group_word.to_lowercase(),
                    active + 1
                ),
                detail: if active + 1 == r.sets {
                    "Lock these dice and finish the event.".to_string()
                } else {
                    "Lock these dice and throw the next group. You cannot \
                     come back to this one."
                        .to_string()
                },
            });
            if self.rerolls_left > 0 {
                choices.push(Choice {
                    action: Action::Reroll,
                    label: format!(
                        "Rethrow {noun} ({} left)",
                        self.rerolls_left
                    ),
                    detail: format!(
                        "Pick up every die of this group and throw again. \
                         Costs one of the {} shared rethrows.",
                        r.rerolls
                    ),
                });
            }
        }

        View {
            key: r.key,
            name: r.name,
            groups,
            rerolls_left: self.rerolls_left,
            rerolls_total: r.rerolls,
            running_score: running,
            choices,
            result: if self.finished() { Some(running) } else { None },
            log: self.log.clone(),
            bar: None,
            jumps_used: None,
            jumps_total: None,
            best: None,
            attempt: None,
            attempts_total: None,
            sheet: None,
            event_index: None,
            total: None,
        }
    }

    fn apply(&mut self, action: &Action, rng: &mut Rng) -> bool {
        if self.finished() {
            return false;
        }
        let r = self.rules;
        match action {
            Action::Reroll => {
                if self.rerolls_left == 0 {
                    return false;
                }
                self.rerolls_left -= 1;
                self.dice = rng.roll_n(r.per_set);
                self.log.push(format!(
                    "{} {} rethrown: {} ({} points, {} rethrow{} left)",
                    r.group_word,
                    self.set_idx() + 1,
                    render(&self.dice),
                    self.showing(),
                    self.rerolls_left,
                    if self.rerolls_left == 1 { "" } else { "s" },
                ));
                true
            }
            Action::Attempt { .. }
            | Action::Skip
            | Action::Keep { .. }
            | Action::Stop => false,
            Action::Freeze => {
                let score = self.showing();
                self.log.push(format!(
                    "{} {} frozen: {} ({score:+})",
                    r.group_word,
                    self.set_idx() + 1,
                    render(&self.dice),
                ));
                self.frozen.push(score);
                if self.finished() {
                    self.dice.clear();
                    self.log.push(format!("Final score: {}", self.locked()));
                } else {
                    self.dice = rng.roll_n(r.per_set);
                    self.log.push(format!(
                        "{} {} thrown: {} ({} points)",
                        r.group_word,
                        self.set_idx() + 1,
                        render(&self.dice),
                        self.showing(),
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

    /// Only the hurdles score a six as six.
    #[test]
    fn the_hurdles_alone_have_no_six_penalty() {
        assert_eq!(score_set(&[6, 6, 6, 6, 6], false), 30);
        assert_eq!(score_set(&[6, 6, 6, 6, 6], true), -30);
        assert!(!rules("110mh").expect("hurdles exist").six_penalty);
        for key in ["100m", "400m", "1500m"] {
            assert!(rules(key).expect("event exists").six_penalty, "{key}");
        }
    }

    /// Dice counts match the rulebook: eight for every event but the
    /// hurdles, which uses five.
    #[test]
    fn dice_counts_match_the_rulebook() {
        for r in RUNNING {
            let total = r.per_set * r.sets;
            let want = if r.key == "110mh" { 5 } else { 8 };
            assert_eq!(total, want, "{}", r.key);
        }
    }

    /// A rethrow costs one from the shared pool, and the pool is spent
    /// across every set rather than per set. Ported from the 100m-only
    /// engine this replaced.
    #[test]
    fn rethrows_are_capped_across_all_sets() {
        let mut rng = Rng::new(3);
        let r = rules("100m").expect("100m exists");
        let mut g = Running::new(r, &mut rng);
        for _ in 0..5 {
            assert!(g.apply(&Action::Reroll, &mut rng));
        }
        // Pool exhausted: no rethrow offered, in this set or the next.
        assert_eq!(g.view().rerolls_left, 0);
        assert!(!g.apply(&Action::Reroll, &mut rng));
        assert!(g.apply(&Action::Freeze, &mut rng));
        assert!(!g.apply(&Action::Reroll, &mut rng));
        assert!(g.view().choices.iter().all(|c| c.action != Action::Reroll));
    }

    /// Sets are played in order, and the score is their sum.
    #[test]
    fn sets_are_played_in_order_and_summed() {
        let mut rng = Rng::new(11);
        let r = rules("400m").expect("400m exists");
        let mut g = Running::new(r, &mut rng);
        let mut want = 0;
        for i in 0..r.sets {
            assert!(g.view().groups[i].active, "set {i} should be active");
            want += g.showing();
            assert!(g.apply(&Action::Freeze, &mut rng));
        }
        assert_eq!(g.view().result, Some(want));
    }

    /// A seed replays identically, which is what makes a game shareable.
    #[test]
    fn a_seed_replays_identically() {
        let play = || {
            let mut rng = Rng::new(99);
            let r = rules("1500m").expect("1500m exists");
            let mut g = Running::new(r, &mut rng);
            for _ in 0..r.sets {
                g.apply(&Action::Freeze, &mut rng);
            }
            g.view().result
        };
        assert_eq!(play(), play());
    }

    /// Playing an event through freezes every set and ends with a score.
    #[test]
    fn freezing_every_set_finishes_the_event() {
        let mut rng = Rng::new(7);
        for r in RUNNING {
            let mut g = Running::new(r, &mut rng);
            for _ in 0..r.sets {
                assert!(g.apply(&Action::Freeze, &mut rng), "{}", r.key);
            }
            let v = g.view();
            assert!(v.result.is_some(), "{}", r.key);
            assert!(v.choices.is_empty(), "{}", r.key);
        }
    }
}

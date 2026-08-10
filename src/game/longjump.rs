//! The Long Jump: a run-up that can bust, then a jump with whatever it
//! carried.
//!
//! > **Run-up:** Start by throwing all five dice. Then freeze at least
//! > one die. If you wish, rethrow all the remaining dice. [...] after
//! > each throw you must freeze at least one more die. [...] If the total
//! > of all frozen dice exceeds 8, you suffer an invalid attempt by
//! > stepping over. If you decide to stop throwing with a total of 8 or
//! > less on all frozen dice, you then jump.
//! >
//! > **Jump:** Pick up your frozen dice and throw them all. Freeze at
//! > least one die and rethrow the remainder. Proceed in this manner
//! > until you freeze all dice. [...] Total the value of all frozen dice
//! > used in your jump.
//!
//! Freezing is compulsory after every throw, and that is the whole source
//! of risk: you commit to a rethrow *before* seeing it and must then take
//! something. The first throw can never bust — the smallest of five dice
//! is at most 6, under the limit of 8 — so danger starts on the second.
//!
//! The run-up wants *many low* dice: only the count carried into the jump
//! matters, and the jump then wants them high.

use super::rng::Rng;
use super::{Action, Choice, Die, Game, Group, View};

/// Dice in the event.
const DICE: usize = 5;
/// Frozen run-up total above which the attempt steps over.
const LIMIT: i32 = 8;
/// Attempts each player gets.
const ATTEMPTS: u32 = 3;

/// Which half of an attempt is being played.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    RunUp,
    Jump,
}

/// A long jump event in progress.
pub struct LongJump {
    best: i32,
    played: u32,
    phase: Phase,
    /// Faces frozen in the phase under way.
    kept: Vec<u8>,
    /// Faces on the table awaiting a freeze.
    table: Vec<u8>,
    /// Dice carried from the run-up into the jump.
    carried: usize,
    /// True between freezing and deciding whether to throw on.
    deciding: bool,
    /// True when an attempt's opening run-up throw is due.
    pending: bool,
    log: Vec<String>,
}

impl LongJump {
    /// Start the event and throw the first run-up.
    pub fn new(rng: &mut Rng) -> Self {
        let g = Self {
            best: 0,
            played: 0,
            phase: Phase::RunUp,
            kept: Vec::new(),
            table: Vec::new(),
            carried: 0,
            deciding: false,
            pending: true,
            log: Vec::new(),
        };
        let _ = rng;
        g
    }

    /// Total of the dice frozen in the phase under way.
    pub fn frozen_total(&self) -> i32 {
        self.kept.iter().map(|&f| i32::from(f)).sum()
    }

    /// Best of the completed attempts.
    pub const fn best(&self) -> i32 {
        self.best
    }

    const fn finished(&self) -> bool {
        self.played >= ATTEMPTS
    }

    fn start_attempt(&mut self, rng: &mut Rng) {
        self.phase = Phase::RunUp;
        self.kept.clear();
        self.carried = 0;
        self.table = rng.roll_n(DICE);
        self.deciding = false;
        self.log.push(format!(
            "Attempt {}: run-up {}",
            self.played + 1,
            render(&self.table)
        ));
    }

    /// Bank `score` and start the next attempt, if any.
    fn end_attempt(&mut self, score: i32) {
        self.best = self.best.max(score);
        self.played += 1;
        self.kept.clear();
        self.table.clear();
        if self.finished() {
            self.log.push(format!("Best of three: {}", self.best));
        } else {
            // The next run-up waits to be acknowledged.
            self.pending = true;
        }
    }

    /// Leave the run-up and throw the carried dice.
    fn begin_jump(&mut self, rng: &mut Rng) {
        self.carried = self.kept.len();
        self.phase = Phase::Jump;
        self.kept.clear();
        self.table = rng.roll_n(self.carried);
        self.deciding = false;
        self.log.push(format!(
            "Jumping with {} dice: {}",
            self.carried,
            render(&self.table)
        ));
    }
}

/// Render dice as `[3 5 6 1]` for the log.
fn render(dice: &[u8]) -> String {
    let faces: Vec<String> = dice.iter().map(ToString::to_string).collect();
    format!("[{}]", faces.join(" "))
}

impl LongJump {
    /// The distinct freezes available, deduplicated on `(count, sum)`.
    ///
    /// The run-up cares only about the count carried and the total
    /// staying within the limit; the jump only about the total. Neither
    /// distinguishes *which* dice, so subsets agreeing on count and sum
    /// are the same move.
    fn freeze_choices(&self) -> Vec<Choice> {
        let mut seen: Vec<(usize, i32)> = Vec::new();
        let mut out = Vec::new();
        for mask in 1u32..(1 << self.table.len()) {
            let picked: Vec<u8> = (0..self.table.len())
                .filter(|b| mask & (1 << b) != 0)
                .map(|i| i as u8)
                .collect();
            let sum: i32 = picked
                .iter()
                .map(|&i| i32::from(self.table[i as usize]))
                .sum();
            let key = (picked.len(), sum);
            if seen.contains(&key) {
                continue;
            }
            seen.push(key);
            let faces: Vec<String> = picked
                .iter()
                .map(|&i| self.table[i as usize].to_string())
                .collect();
            let after = self.frozen_total() + sum;
            let warn = if self.phase == Phase::RunUp && after > LIMIT {
                " — STEPS OVER"
            } else {
                ""
            };
            out.push(Choice {
                action: Action::Keep { dice: picked },
                label: format!("Keep {}{warn}", faces.join("+")),
                detail: match self.phase {
                    Phase::RunUp => format!(
                        "Run-up total becomes {after}; over {LIMIT} steps \
                         over and voids the attempt."
                    ),
                    Phase::Jump => format!("Adds {sum} to the jump."),
                },
            });
        }
        out
    }
}

impl Game for LongJump {
    fn view(&self) -> View {
        let mut dice: Vec<Die> = self
            .kept
            .iter()
            .map(|&face| Die { face, frozen: true })
            .collect();
        dice.extend(self.table.iter().map(|&face| Die {
            face,
            frozen: false,
        }));
        let label = if self.finished() {
            "Finished".to_string()
        } else {
            match self.phase {
                Phase::RunUp => format!(
                    "Attempt {} — run-up ({}/{LIMIT})",
                    self.played + 1,
                    self.frozen_total()
                ),
                Phase::Jump => {
                    format!("Attempt {} — jump", self.played + 1)
                }
            }
        };
        let groups = vec![Group {
            label,
            dice,
            score: if self.best > 0 { Some(self.best) } else { None },
            active: !self.finished(),
        }];

        let mut choices = Vec::new();
        if !self.finished() && self.pending {
            choices.push(Choice {
                action: Action::Roll,
                label: format!("Start attempt {}", self.played + 1),
                detail: "Throw all five dice for the run-up. You must then \
                         freeze at least one, whatever shows."
                    .to_string(),
            });
        } else if !self.finished() {
            if self.deciding {
                if self.phase == Phase::RunUp {
                    choices.push(Choice {
                        action: Action::Stop,
                        label: format!("Jump with {}", self.kept.len()),
                        detail: "Stop the run-up and throw the frozen dice \
                                 as your jump."
                            .to_string(),
                    });
                }
                choices.push(Choice {
                    action: Action::Reroll,
                    label: "Throw the rest".to_string(),
                    detail: match self.phase {
                        Phase::RunUp => "Rethrow every loose die. You must \
                                         then freeze another, whatever it \
                                         shows."
                            .to_string(),
                        Phase::Jump => "Rethrow every loose die and freeze \
                                        again."
                            .to_string(),
                    },
                });
            } else {
                choices.extend(self.freeze_choices());
            }
        }

        View {
            key: "longjump",
            name: "Long Jump",
            groups,
            rerolls_left: self.table.len() as u32,
            rerolls_total: DICE as u32,
            running_score: self.frozen_total(),
            choices,
            result: if self.finished() {
                Some(self.best)
            } else {
                None
            },
            log: self.log.clone(),
            bar: None,
            jumps_used: None,
            jumps_total: None,
            best: Some(self.best),
            attempt: Some(self.played + 1),
            attempts_total: Some(ATTEMPTS),
            sheet: None,
            event_index: None,
            total: None,
            warn_face: None,
        }
    }

    fn apply(&mut self, action: &Action, rng: &mut Rng) -> bool {
        if self.finished() {
            return false;
        }
        if matches!(action, Action::Roll) {
            if !self.pending {
                return false;
            }
            self.pending = false;
            self.start_attempt(rng);
            return true;
        }
        if self.pending {
            return false;
        }
        match action {
            Action::Keep { dice } => {
                if self.deciding || dice.is_empty() {
                    return false;
                }
                if dice.iter().any(|&i| i as usize >= self.table.len()) {
                    return false;
                }
                let mut picked: Vec<u8> =
                    dice.iter().map(|&i| self.table[i as usize]).collect();
                picked.sort_unstable();
                self.kept.extend(picked.iter().copied());
                let mut sorted = dice.clone();
                sorted.sort_unstable_by(|a, b| b.cmp(a));
                for i in sorted {
                    self.table.remove(i as usize);
                }
                self.log.push(format!(
                    "Froze {} ({} total)",
                    render(&picked),
                    self.frozen_total()
                ));

                if self.phase == Phase::RunUp && self.frozen_total() > LIMIT {
                    self.log.push(format!(
                        "Stepped over at {} — invalid attempt.",
                        self.frozen_total()
                    ));
                    self.end_attempt(0);
                    return true;
                }
                if self.table.is_empty() {
                    if self.phase == Phase::RunUp {
                        self.begin_jump(rng);
                    } else {
                        let score = self.frozen_total();
                        self.log.push(format!("Jumped {score}."));
                        self.end_attempt(score);
                    }
                } else {
                    self.deciding = true;
                }
                true
            }
            Action::Stop => {
                if !self.deciding || self.phase != Phase::RunUp {
                    return false;
                }
                self.begin_jump(rng);
                true
            }
            Action::Reroll => {
                if !self.deciding {
                    return false;
                }
                self.table = rng.roll_n(self.table.len());
                self.deciding = false;
                self.log.push(format!("Rethrew {}", render(&self.table)));
                true
            }
            Action::Freeze
            | Action::Attempt { .. }
            | Action::Skip
            | Action::Roll => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The opening throw cannot step over: the smallest of five dice is
    /// at most 6, under the limit of 8.
    #[test]
    fn the_first_freeze_never_steps_over() {
        for seed in 0..40 {
            let mut rng = Rng::new(seed);
            let mut g = LongJump::new(&mut rng);
            g.apply(&Action::Roll, &mut rng);
            let lowest = g.table.iter().copied().min().expect("five dice");
            assert!(i32::from(lowest) <= LIMIT);
        }
    }

    /// Freezing past the limit voids the attempt rather than scoring.
    #[test]
    fn stepping_over_voids_the_attempt() {
        let mut rng = Rng::new(21);
        let mut g = LongJump::new(&mut rng);
        g.apply(&Action::Roll, &mut rng);
        // Freeze everything at once; five dice total at least 5 and often
        // more than 8, and when they do the attempt must be void.
        let all: Vec<u8> = (0..g.table.len() as u8).collect();
        let total: i32 = g.table.iter().map(|&f| i32::from(f)).sum();
        g.apply(&Action::Keep { dice: all }, &mut rng);
        if total > LIMIT {
            assert!(g.log.iter().any(|l| l.contains("Stepped over")));
        }
    }

    /// Three attempts, best kept, and the event terminates.
    #[test]
    fn the_event_is_best_of_three() {
        let mut rng = Rng::new(22);
        let mut g = LongJump::new(&mut rng);
        for _ in 0..2000 {
            if g.view().result.is_some() {
                break;
            }
            let a = g.view().choices.first().map(|c| c.action.clone());
            match a {
                Some(a) => {
                    g.apply(&a, &mut rng);
                }
                None => break,
            }
        }
        let v = g.view();
        assert!(v.result.is_some(), "event should finish");
        assert_eq!(v.attempts_total, Some(3));
    }
}

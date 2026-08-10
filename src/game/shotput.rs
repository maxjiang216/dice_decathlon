//! The Shot Put: eight dice thrown one at a time.
//!
//! > Throw one die after the other. At any point you can stop. Your
//! > attempt must end after all eight dice. If you throw a one you suffer
//! > an invalid attempt.
//!
//! Best of three, and a foul costs only the attempt it happens in.
//!
//! The first die is compulsory — the rulebook does not say whether you
//! may decline to start, and an empty attempt would score 0 exactly as a
//! foul does, so nothing turns on the reading. It is recorded in
//! `worklog/RULES-CHECKLIST.md` as an inference all the same.

use super::rng::Rng;
use super::{Action, Choice, Die, Game, Group, View};

/// Dice available to one attempt.
const DICE: usize = 8;
/// Attempts each player gets.
const ATTEMPTS: u32 = 3;

/// A shot put event in progress.
pub struct ShotPut {
    best: i32,
    played: u32,
    /// Faces thrown so far this attempt, none of them a one.
    thrown: Vec<u8>,
    log: Vec<String>,
}

impl ShotPut {
    /// Start the event and throw the first attempt's compulsory die.
    pub fn new(rng: &mut Rng) -> Self {
        let mut g = Self {
            best: 0,
            played: 0,
            thrown: Vec::new(),
            log: Vec::new(),
        };
        g.throw(rng);
        g
    }

    /// Points on the table this attempt.
    pub fn running(&self) -> i32 {
        self.thrown.iter().map(|&f| i32::from(f)).sum()
    }

    /// Best of the completed attempts.
    pub const fn best(&self) -> i32 {
        self.best
    }

    const fn finished(&self) -> bool {
        self.played >= ATTEMPTS
    }

    /// Throw the next die, ending the attempt on a one or on the eighth.
    fn throw(&mut self, rng: &mut Rng) {
        let face = rng.roll_n(1)[0];
        if face == 1 {
            self.log.push(format!(
                "Attempt {}: threw a 1 — invalid, scores 0.",
                self.played + 1
            ));
            self.end_attempt(rng, 0);
            return;
        }
        self.thrown.push(face);
        self.log.push(format!(
            "Attempt {}: threw {face} ({} on {} dice)",
            self.played + 1,
            self.running(),
            self.thrown.len()
        ));
        if self.thrown.len() == DICE {
            let score = self.running();
            self.log.push(format!("All eight thrown — {score}."));
            self.end_attempt(rng, score);
        }
    }

    /// Bank `score` and start the next attempt, if any.
    fn end_attempt(&mut self, rng: &mut Rng, score: i32) {
        self.best = self.best.max(score);
        self.played += 1;
        self.thrown.clear();
        if self.finished() {
            self.log.push(format!("Best of three: {}", self.best));
        } else {
            self.throw(rng);
        }
    }
}

impl Game for ShotPut {
    fn view(&self) -> View {
        let dice: Vec<Die> = self
            .thrown
            .iter()
            .map(|&face| Die { face, frozen: true })
            .collect();
        let groups = vec![Group {
            label: if self.finished() {
                "Finished".to_string()
            } else {
                format!("Attempt {}", self.played + 1)
            },
            dice,
            score: if self.best > 0 { Some(self.best) } else { None },
            active: !self.finished(),
        }];

        let mut choices = Vec::new();
        if !self.finished() {
            choices.push(Choice {
                action: Action::Stop,
                label: format!("Stop on {}", self.running()),
                detail: "Bank what is on the table and end the attempt."
                    .to_string(),
            });
            choices.push(Choice {
                action: Action::Reroll,
                label: format!(
                    "Throw another ({} left)",
                    DICE - self.thrown.len()
                ),
                detail: "Throw one more die. A 1 voids the whole attempt; \
                         anything else adds to the total."
                    .to_string(),
            });
        }

        View {
            key: "shotput",
            name: "Shot Put",
            groups,
            rerolls_left: (DICE - self.thrown.len()) as u32,
            rerolls_total: DICE as u32,
            running_score: self.running(),
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
        }
    }

    fn apply(&mut self, action: &Action, rng: &mut Rng) -> bool {
        if self.finished() {
            return false;
        }
        match action {
            Action::Reroll => {
                self.throw(rng);
                true
            }
            Action::Stop => {
                let score = self.running();
                self.log.push(format!("Stopped on {score}."));
                self.end_attempt(rng, score);
                true
            }
            Action::Freeze
            | Action::Attempt { .. }
            | Action::Skip
            | Action::Keep { .. } => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one voids the attempt, so no surviving total can include one.
    #[test]
    fn a_one_voids_the_attempt() {
        let mut rng = Rng::new(12);
        let mut g = ShotPut::new(&mut rng);
        for _ in 0..500 {
            if g.view().result.is_some() {
                break;
            }
            assert!(!g.thrown.contains(&1));
            g.apply(&Action::Reroll, &mut rng);
        }
    }

    /// Three attempts, best kept.
    #[test]
    fn the_event_is_best_of_three() {
        let mut rng = Rng::new(13);
        let mut g = ShotPut::new(&mut rng);
        for _ in 0..500 {
            if g.view().result.is_some() {
                break;
            }
            g.apply(&Action::Stop, &mut rng);
        }
        let v = g.view();
        assert!(v.result.is_some());
        assert_eq!(v.attempts_total, Some(3));
    }
}

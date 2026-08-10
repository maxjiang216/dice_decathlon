//! The two "freeze dice of one parity" events: Discus and Javelin.
//!
//! > Start by throwing all [the] dice. Then freeze at least one die. If
//! > you wish, rethrow all the remaining dice. [...] after each throw you
//! > must freeze at least one more die. Only dice with **even** [discus]
//! > / **odd** [javelin] values may be frozen. You can [stop] throwing
//! > and finish your attempt at any time. An attempt ends automatically
//! > when all [the] dice are frozen. If, after one of your throws, you
//! > cannot freeze another die [...] you suffer an invalid attempt.
//!
//! Best of three attempts, and a foul costs only the attempt it happens
//! in — the best already banked survives.
//!
//! Only the *number and total* of the dice frozen matters, never which
//! ones: two freezes with the same count and sum leave identical
//! positions. That is proved by full-subset brute force in
//! `tests/attempt_engines.rs`, and it is why the choices offered here are
//! deduplicated on `(count, sum)` — a shorter list that loses nothing.

use super::rng::Rng;
use super::{Action, Choice, Die, Game, Group, View};

/// Geometry of one freeze event.
#[derive(Clone, Copy)]
pub struct Rules {
    pub key: &'static str,
    pub name: &'static str,
    pub dice: usize,
    /// The faces that may be frozen. All share a parity.
    pub faces: [u8; 3],
    /// What to call them in the UI.
    pub parity: &'static str,
}

/// Attempts each player gets.
const ATTEMPTS: u32 = 3;

/// The two events, in rulebook order.
pub const FREEZE: [Rules; 2] = [
    Rules {
        key: "discus",
        name: "Discus",
        dice: 5,
        faces: [2, 4, 6],
        parity: "even",
    },
    Rules {
        key: "javelin",
        name: "Javelin",
        dice: 6,
        faces: [1, 3, 5],
        parity: "odd",
    },
];

/// Look up a freeze event by key.
pub fn rules(key: &str) -> Option<Rules> {
    FREEZE.iter().copied().find(|r| r.key == key)
}

/// A freeze event in progress.
pub struct Freeze {
    rules: Rules,
    /// Best of the attempts finished so far.
    best: i32,
    /// Attempts already played.
    played: u32,
    /// Faces frozen in the attempt under way.
    kept: Vec<u8>,
    /// Faces on the table, awaiting a freeze.
    table: Vec<u8>,
    /// True once the throw showed nothing freezable.
    fouled: bool,
    /// True between freezing and deciding whether to throw on.
    deciding: bool,
    log: Vec<String>,
}

impl Freeze {
    /// Start the event and throw the first attempt's opening dice.
    pub fn new(rules: Rules, rng: &mut Rng) -> Self {
        let mut g = Self {
            rules,
            best: 0,
            played: 0,
            kept: Vec::new(),
            table: Vec::new(),
            fouled: false,
            deciding: false,
            log: Vec::new(),
        };
        g.throw(rng);
        g
    }

    /// Points frozen in the attempt under way.
    pub fn banked(&self) -> i32 {
        self.kept.iter().map(|&f| i32::from(f)).sum()
    }

    /// Best of the completed attempts.
    pub const fn best(&self) -> i32 {
        self.best
    }

    const fn finished(&self) -> bool {
        self.played >= ATTEMPTS
    }

    fn freezable(&self) -> Vec<usize> {
        self.table
            .iter()
            .enumerate()
            .filter(|(_, f)| self.rules.faces.contains(f))
            .map(|(i, _)| i)
            .collect()
    }

    /// The distinct freezes available from the dice on the table.
    ///
    /// Deduplicated on `(count, sum)`: two freezes agreeing on both leave
    /// identical positions, so listing every subset would offer the
    /// player the same move several times over.
    fn freeze_choices(&self) -> Vec<Choice> {
        let idx = self.freezable();
        let mut seen: Vec<(usize, i32)> = Vec::new();
        let mut out = Vec::new();
        for mask in 1u32..(1 << idx.len()) {
            let picked: Vec<u8> = idx
                .iter()
                .enumerate()
                .filter(|(b, _)| mask & (1 << b) != 0)
                .map(|(_, &i)| i as u8)
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
            out.push(Choice {
                action: Action::Keep { dice: picked },
                label: format!("Keep {} (+{sum})", faces.join("+")),
                detail: format!("Freeze {} die/dice for {sum} points.", key.0),
            });
        }
        out.sort_by_key(|c| -sum_of(&c.action, &self.table));
        out
    }

    /// Throw every die not yet frozen.
    fn throw(&mut self, rng: &mut Rng) {
        let n = self.rules.dice - self.kept.len();
        self.table = rng.roll_n(n);
        self.deciding = false;
        self.log.push(format!(
            "Attempt {}: threw {}",
            self.played + 1,
            render(&self.table)
        ));
        if self.freezable().is_empty() {
            self.fouled = true;
            self.log.push(format!(
                "No {} die — invalid attempt, scores 0.",
                self.rules.parity
            ));
            self.end_attempt(rng, 0);
        }
    }

    /// Bank `score` for the attempt and start the next, if any.
    fn end_attempt(&mut self, rng: &mut Rng, score: i32) {
        self.best = self.best.max(score);
        self.played += 1;
        self.kept.clear();
        self.table.clear();
        self.fouled = false;
        self.deciding = false;
        if self.finished() {
            self.log.push(format!("Best of three: {}", self.best));
        } else {
            self.throw(rng);
        }
    }
}

/// Total of the dice a `Keep` names, for ordering the choice list.
fn sum_of(action: &Action, table: &[u8]) -> i32 {
    match action {
        Action::Keep { dice } => {
            dice.iter().map(|&i| i32::from(table[i as usize])).sum()
        }
        _ => 0,
    }
}

/// Render dice as `[3 5 6 1]` for the log.
fn render(dice: &[u8]) -> String {
    let faces: Vec<String> = dice.iter().map(ToString::to_string).collect();
    format!("[{}]", faces.join(" "))
}

impl Game for Freeze {
    fn view(&self) -> View {
        let r = self.rules;
        let mut dice: Vec<Die> = self
            .kept
            .iter()
            .map(|&face| Die { face, frozen: true })
            .collect();
        dice.extend(self.table.iter().map(|&face| Die {
            face,
            frozen: false,
        }));

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
            if self.deciding {
                choices.push(Choice {
                    action: Action::Stop,
                    label: format!("Stop on {}", self.banked()),
                    detail: "Finish the attempt and bank what is frozen."
                        .to_string(),
                });
                choices.push(Choice {
                    action: Action::Reroll,
                    label: "Throw the rest".to_string(),
                    detail: format!(
                        "Rethrow every unfrozen die. You must then freeze \
                         another {} one, and if none shows the attempt is \
                         void.",
                        r.parity
                    ),
                });
            } else {
                choices.extend(self.freeze_choices());
            }
        }

        View {
            key: r.key,
            name: r.name,
            groups,
            rerolls_left: (self.rules.dice - self.kept.len()) as u32,
            rerolls_total: r.dice as u32,
            running_score: self.banked(),
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
        if self.finished() || self.fouled {
            return false;
        }
        match action {
            Action::Keep { dice } => {
                if self.deciding || dice.is_empty() {
                    return false;
                }
                let legal = self.freezable();
                if !dice.iter().all(|i| legal.contains(&(*i as usize))) {
                    return false;
                }
                let mut picked: Vec<u8> =
                    dice.iter().map(|&i| self.table[i as usize]).collect();
                picked.sort_unstable();
                let gained: i32 = picked.iter().map(|&f| i32::from(f)).sum();
                self.kept.extend(picked.iter().copied());
                // Removing from the back keeps the earlier indices valid.
                let mut sorted = dice.clone();
                sorted.sort_unstable_by(|a, b| b.cmp(a));
                for i in sorted {
                    self.table.remove(i as usize);
                }
                self.log.push(format!(
                    "Froze {} (+{gained}, {} total)",
                    render(&picked),
                    self.banked()
                ));
                if self.kept.len() == self.rules.dice {
                    let score = self.banked();
                    self.log.push(format!("All dice frozen — {score}."));
                    self.end_attempt(rng, score);
                } else {
                    self.deciding = true;
                }
                true
            }
            Action::Stop => {
                if !self.deciding {
                    return false;
                }
                let score = self.banked();
                self.log.push(format!("Stopped on {score}."));
                self.end_attempt(rng, score);
                true
            }
            Action::Reroll => {
                if !self.deciding {
                    return false;
                }
                self.throw(rng);
                true
            }
            Action::Freeze | Action::Attempt { .. } | Action::Skip => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only dice of the event's parity may be frozen.
    #[test]
    fn only_the_right_parity_can_be_frozen() {
        let mut rng = Rng::new(5);
        for r in FREEZE {
            let g = Freeze::new(r, &mut rng);
            for i in g.freezable() {
                assert!(r.faces.contains(&g.table[i]), "{}", r.key);
            }
        }
    }

    /// Every offered freeze is legal, and choices are deduplicated on
    /// (count, sum) rather than listing every subset.
    #[test]
    fn offered_freezes_are_legal_and_deduplicated() {
        let mut rng = Rng::new(6);
        let g =
            Freeze::new(rules("javelin").expect("javelin exists"), &mut rng);
        let mut pairs = Vec::new();
        for c in g.view().choices {
            if let Action::Keep { dice } = &c.action {
                let sum: i32 =
                    dice.iter().map(|&i| i32::from(g.table[i as usize])).sum();
                let key = (dice.len(), sum);
                assert!(!pairs.contains(&key), "duplicate {key:?}");
                pairs.push(key);
            }
        }
    }

    /// Three attempts, then the best of them is the result.
    #[test]
    fn the_event_is_best_of_three() {
        let mut rng = Rng::new(8);
        let mut g =
            Freeze::new(rules("discus").expect("discus exists"), &mut rng);
        for _ in 0..200 {
            if g.view().result.is_some() {
                break;
            }
            let choice = g.view().choices.first().map(|c| c.action.clone());
            match choice {
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

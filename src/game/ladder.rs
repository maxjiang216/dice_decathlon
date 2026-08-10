//! The two rising-bar events: High Jump and Pole Vault.
//!
//! > Jumping starts at the height of 10 and is increased by increments of
//! > 2. At each height you can decide, on your turn, if you try to jump
//! > the height or if you skip it. If you [go] for that height, you have
//! > three jumps in which to master it. [...] If you [suffer] three
//! > invalid [jumps] at one height you have to stop.
//!
//! **Skipping is free and sometimes right.** You keep your best, stay in
//! the event, and lose nothing; attempting risks elimination, which
//! forfeits every higher bar. Even an expected-value player declines some
//! pole vault bars — at 34 holding 32, attempting is worth 32.5197 and
//! skipping 32.5326. See `worklog/RULES-CHECKLIST.md`.
//!
//! We read "you have three jumps" as an allotment you commit to once per
//! bar: having chosen to attempt, two misses oblige you to take the
//! third. The rulebook does not explicitly forbid walking away part-way,
//! and that reading would change optimal play, so it is recorded as an
//! inference rather than assumed.

use super::rng::Rng;
use super::{Action, Choice, Die, Game, Group, View};

/// How many dice a jump throws.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Dice {
    /// Always this many — the high jump throws all five, every time.
    Fixed(u8),
    /// The jumper picks, up to this many. Any one fails the jump, so
    /// more dice reach higher but are likelier to spoil.
    UpTo(u8),
}

/// Geometry and scoring of one jumping event.
#[derive(Clone, Copy)]
pub struct Rules {
    pub key: &'static str,
    pub name: &'static str,
    /// Highest bar worth offering: where the dice can still reach.
    pub max_height: i32,
    pub dice: Dice,
    /// Whether a single 1 spoils the jump regardless of the total.
    pub ones_fail: bool,
}

/// First bar, and the step between bars.
const START: i32 = 10;
const STEP: i32 = 2;
/// Jumps allowed at each bar.
const JUMPS: u32 = 3;

/// The two events, in rulebook order.
pub const LADDERS: [Rules; 2] = [
    Rules {
        key: "highjump",
        name: "High Jump",
        max_height: 30,
        dice: Dice::Fixed(5),
        ones_fail: false,
    },
    Rules {
        key: "polevault",
        name: "Pole Vault",
        max_height: 48,
        dice: Dice::UpTo(8),
        ones_fail: true,
    },
];

/// Look up a jumping event by key.
pub fn rules(key: &str) -> Option<Rules> {
    LADDERS.iter().copied().find(|r| r.key == key)
}

/// A jumping event in progress.
pub struct Ladder {
    rules: Rules,
    /// Bar being faced.
    bar: i32,
    /// Highest bar cleared, or zero.
    best: i32,
    /// Jumps already spent at this bar.
    jumps_used: u32,
    /// Set once three jumps have been missed at one bar.
    out: bool,
    /// Faces of the last jump, for display.
    last: Vec<u8>,
    log: Vec<String>,
}

impl Ladder {
    /// Start the event at the opening bar.
    pub fn new(rules: Rules, rng: &mut Rng) -> Self {
        let _ = rng;
        Self {
            rules,
            bar: START,
            best: 0,
            jumps_used: 0,
            out: false,
            last: Vec::new(),
            log: vec![format!("Bar set at {START}.")],
        }
    }

    const fn finished(&self) -> bool {
        self.out || self.bar > self.rules.max_height
    }

    /// Best height cleared so far.
    pub const fn best(&self) -> i32 {
        self.best
    }

    /// Bar currently being faced.
    pub const fn bar(&self) -> i32 {
        self.bar
    }

    /// Move to the next bar, resetting the jump allotment.
    fn advance(&mut self) {
        self.bar += STEP;
        self.jumps_used = 0;
        if self.bar <= self.rules.max_height {
            self.log.push(format!("Bar raised to {}.", self.bar));
        }
    }
}

/// Render dice as `[3 5 6 1]` for the log.
fn render(dice: &[u8]) -> String {
    let faces: Vec<String> = dice.iter().map(ToString::to_string).collect();
    format!("[{}]", faces.join(" "))
}

impl Game for Ladder {
    fn view(&self) -> View {
        let r = self.rules;
        let dice: Vec<Die> = self
            .last
            .iter()
            .map(|&face| Die {
                face,
                frozen: false,
            })
            .collect();
        let groups = vec![Group {
            label: if self.finished() {
                "Finished".to_string()
            } else {
                format!("Bar {}", self.bar)
            },
            dice,
            score: if self.best > 0 { Some(self.best) } else { None },
            active: !self.finished(),
        }];

        let mut choices = Vec::new();
        if !self.finished() {
            let counts: Vec<u8> = match r.dice {
                Dice::Fixed(n) => vec![n],
                Dice::UpTo(n) => (1..=n).collect(),
            };
            for n in counts {
                let dice = match r.dice {
                    Dice::Fixed(_) => None,
                    Dice::UpTo(_) => Some(n),
                };
                let label = match r.dice {
                    Dice::Fixed(k) => format!("Jump ({k} dice)"),
                    Dice::UpTo(_) => format!("Jump with {n}"),
                };
                choices.push(Choice {
                    action: Action::Attempt { dice },
                    label,
                    detail: format!(
                        "Throw {n} dice; clear {} by totalling at least it{}. \
                         Miss three times at this bar and your event ends.",
                        self.bar,
                        if r.ones_fail {
                            " and showing no ones"
                        } else {
                            ""
                        }
                    ),
                });
            }
            // Only offered before committing to the bar: two misses
            // oblige you to take the third jump.
            if self.jumps_used == 0 {
                choices.push(Choice {
                    action: Action::Skip,
                    label: format!("Skip {}", self.bar),
                    detail: "Decline this bar and face the next one. Costs \
                             nothing: you keep your best and stay in."
                        .to_string(),
                });
            }
        }

        View {
            key: r.key,
            name: r.name,
            groups,
            rerolls_left: JUMPS - self.jumps_used,
            rerolls_total: JUMPS,
            running_score: self.best,
            choices,
            result: if self.finished() {
                Some(self.best)
            } else {
                None
            },
            log: self.log.clone(),
            bar: Some(self.bar),
            jumps_used: Some(self.jumps_used),
            jumps_total: Some(JUMPS),
            best: Some(self.best),
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
            Action::Skip => {
                if self.jumps_used > 0 {
                    return false; // already committed to this bar
                }
                self.log.push(format!("Skipped {}.", self.bar));
                self.last.clear();
                self.advance();
                true
            }
            Action::Attempt { dice } => {
                let n = match (r.dice, *dice) {
                    (Dice::Fixed(k), _) => k,
                    (Dice::UpTo(k), Some(d)) if d >= 1 && d <= k => d,
                    (Dice::UpTo(_), _) => return false,
                };
                let roll = rng.roll_n(n as usize);
                let total: i32 = roll.iter().map(|&f| i32::from(f)).sum();
                let spoiled = r.ones_fail && roll.contains(&1);
                let cleared = !spoiled && total >= self.bar;
                self.jumps_used += 1;
                self.last.clone_from(&roll);
                self.log.push(format!(
                    "Jump {} at {}: {} = {total}{} — {}",
                    self.jumps_used,
                    self.bar,
                    render(&roll),
                    if spoiled { " (a one!)" } else { "" },
                    if cleared { "cleared" } else { "missed" },
                ));
                if cleared {
                    self.best = self.bar;
                    self.advance();
                } else if self.jumps_used >= JUMPS {
                    self.out = true;
                    self.log.push(format!(
                        "Three misses at {} — event over on {}.",
                        self.bar, self.best
                    ));
                }
                true
            }
            Action::Freeze
            | Action::Reroll
            | Action::Keep { .. }
            | Action::Stop => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Skipping keeps your best and your place in the event.
    #[test]
    fn skipping_costs_nothing() {
        let mut rng = Rng::new(1);
        let r = rules("polevault").expect("pole vault exists");
        let mut g = Ladder::new(r, &mut rng);
        assert!(g.apply(&Action::Skip, &mut rng));
        assert_eq!(g.bar(), START + STEP);
        assert_eq!(g.best(), 0);
        assert!(g.view().result.is_none());
    }

    /// Having taken a jump you are committed: no skipping out mid-bar.
    #[test]
    fn a_started_bar_cannot_be_skipped() {
        let mut rng = Rng::new(2);
        let r = rules("highjump").expect("high jump exists");
        let mut g = Ladder::new(r, &mut rng);
        // Jump until one misses, then the skip must be gone.
        for _ in 0..40 {
            if g.view().jumps_used == Some(0) {
                assert!(g.apply(&Action::Attempt { dice: None }, &mut rng));
            } else {
                assert!(!g.apply(&Action::Skip, &mut rng));
                assert!(g
                    .view()
                    .choices
                    .iter()
                    .all(|c| c.action != Action::Skip));
                return;
            }
        }
    }

    /// The pole vault must be given a legal die count; the high jump
    /// takes none, since it always throws all five.
    #[test]
    fn die_counts_are_checked() {
        let mut rng = Rng::new(3);
        let mut pv = Ladder::new(rules("polevault").unwrap(), &mut rng);
        assert!(!pv.apply(&Action::Attempt { dice: Some(0) }, &mut rng));
        assert!(!pv.apply(&Action::Attempt { dice: Some(9) }, &mut rng));
        assert!(pv.apply(&Action::Attempt { dice: Some(3) }, &mut rng));

        let mut hj = Ladder::new(rules("highjump").unwrap(), &mut rng);
        assert!(hj.apply(&Action::Attempt { dice: None }, &mut rng));
    }

    /// Skipping every bar ends the event on nothing.
    #[test]
    fn skipping_everything_scores_zero() {
        let mut rng = Rng::new(4);
        let r = rules("highjump").expect("high jump exists");
        let mut g = Ladder::new(r, &mut rng);
        while g.view().result.is_none() {
            assert!(g.apply(&Action::Skip, &mut rng));
        }
        assert_eq!(g.view().result, Some(0));
    }
}

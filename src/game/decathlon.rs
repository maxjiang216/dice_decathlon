//! The whole competition: ten events in rulebook order, one scoresheet.
//!
//! > A decathlon consists of ten independent disciplines: 100 Metres,
//! > Long Jump, Shot Put, High Jump, 400 Metres, 110 Metre Hurdles,
//! > Discus, Pole-Vault, Javelin, 1500 Metres. The players try to score
//! > as many points as possible in each discipline. The player with the
//! > highest total of points after the ten disciplines wins.
//!
//! > When a player finishes a discipline, his best result is added to his
//! > previous running total to form his new running total.
//!
//! The events really are independent — nothing carries between them but
//! the score — so this wraps whichever event is under way and banks its
//! result when it ends. That the running total is the *only* thing
//! crossing an event boundary is also what makes the two-player solve
//! tractable; see `worklog/2026-08-09-two-player-optimal-play/`.

use super::rng::Rng;
use super::{catalogue, start, Action, Game, View};

/// A decathlon in progress.
pub struct Decathlon {
    /// Index of the event being played, in rulebook order.
    at: usize,
    /// Score of each event, `None` until it is finished.
    sheet: Vec<Option<i32>>,
    /// The event under way, or `None` once all ten are done.
    event: Option<Box<dyn Game>>,
    log: Vec<String>,
}

impl Decathlon {
    /// Start the competition on the 100 Metres.
    pub fn new(rng: &mut Rng) -> Self {
        let events = catalogue();
        let first = start(events[0].0, rng);
        Self {
            at: 0,
            sheet: vec![None; events.len()],
            event: first,
            log: vec![format!("Event 1 of {}: {}", events.len(), events[0].1)],
        }
    }

    /// Points banked from the events already finished.
    pub fn total(&self) -> i32 {
        self.sheet.iter().flatten().sum()
    }

    /// Whether every event has been played.
    pub fn finished(&self) -> bool {
        self.at >= self.sheet.len()
    }

    /// Bank `score` for the current event and move to the next.
    fn advance(&mut self, score: i32, rng: &mut Rng) {
        let events = catalogue();
        self.sheet[self.at] = Some(score);
        self.log.push(format!(
            "{} scored {score} — running total {}",
            events[self.at].1,
            self.total()
        ));
        self.at += 1;
        if self.finished() {
            self.event = None;
            self.log
                .push(format!("Decathlon complete: {} points.", self.total()));
        } else {
            self.event = start(events[self.at].0, rng);
            self.log.push(format!(
                "Event {} of {}: {}",
                self.at + 1,
                events.len(),
                events[self.at].1
            ));
        }
    }
}

impl Game for Decathlon {
    fn view(&self) -> View {
        let events = catalogue();
        let mut view = self.event.as_ref().map_or_else(
            || View {
                key: "decathlon",
                name: "Decathlon",
                groups: Vec::new(),
                rerolls_left: 0,
                rerolls_total: 0,
                running_score: self.total(),
                choices: Vec::new(),
                result: Some(self.total()),
                log: Vec::new(),
                bar: None,
                jumps_used: None,
                jumps_total: None,
                best: None,
                attempt: None,
                attempts_total: None,
                sheet: None,
                event_index: None,
                total: None,
                warn_face: None,
            },
            |g| g.view(),
        );
        // The competition's own log leads; the event's follows, so a
        // reader sees the scoresheet story before the dice story.
        let mut log = self.log.clone();
        log.extend(view.log);
        view.log = log;
        view.sheet = Some(self.sheet.clone());
        view.event_index = Some(self.at.min(events.len() - 1));
        view.total = Some(self.total());
        // Only the whole competition being over is a final result; an
        // event ending just moves the scoresheet on.
        view.result = if self.finished() {
            Some(self.total())
        } else {
            None
        };
        view
    }

    fn apply(&mut self, action: &Action, rng: &mut Rng) -> bool {
        let Some(event) = self.event.as_mut() else {
            return false;
        };
        if !event.apply(action, rng) {
            return false;
        }
        if let Some(score) = event.view().result {
            self.advance(score, rng);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Playing straight through visits all ten events and totals them.
    #[test]
    fn ten_events_are_played_and_summed() {
        let mut rng = Rng::new(1234);
        let mut d = Decathlon::new(&mut rng);
        for _ in 0..20000 {
            if d.finished() {
                break;
            }
            let a = d.view().choices.first().map(|c| c.action.clone());
            match a {
                Some(a) => {
                    assert!(d.apply(&a, &mut rng), "move should be legal");
                }
                None => panic!("stuck with no legal move"),
            }
        }
        assert!(d.finished(), "competition should finish");
        let v = d.view();
        let sheet = v.sheet.expect("scoresheet");
        assert_eq!(sheet.len(), 10);
        assert!(sheet.iter().all(Option::is_some), "every event scored");
        let summed: i32 = sheet.iter().flatten().sum();
        assert_eq!(v.result, Some(summed));
    }

    /// An event ending is not the competition ending.
    #[test]
    fn finishing_an_event_moves_on_rather_than_stopping() {
        let mut rng = Rng::new(99);
        let mut d = Decathlon::new(&mut rng);
        while d.view().sheet.as_ref().is_some_and(|s| s[0].is_none()) {
            let a = d.view().choices[0].action.clone();
            d.apply(&a, &mut rng);
        }
        let v = d.view();
        assert!(
            v.result.is_none(),
            "competition is not over after one event"
        );
        assert_eq!(v.event_index, Some(1), "should be on the second event");
        assert!(!v.choices.is_empty(), "the next event should be playable");
    }
}

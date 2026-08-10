//! A competition between two to four players.
//!
//! Turn order comes straight from the rulebook:
//!
//! > Players decide the order of play in the first discipline by the roll
//! > of a die. From the second discipline onwards, **the leading player
//! > always starts**, followed by the player with the second highest
//! > running total and so on.
//!
//! Note the direction: the leader moves *first*, which hands the
//! information advantage to whoever is behind — they play knowing exactly
//! what they must beat. The two-player solve measures that at about five
//! points of win probability in the 1500m.
//!
//! **Not yet faithful:** the rulebook also interleaves attempts and
//! heights across players ("all first attempts are played first, then all
//! second attempts"). Here each player finishes their event before the
//! next begins. That is exact for the four single-attempt events and an
//! approximation for the other six, where it hands the later player *more*
//! information than the rules allow. Recorded in
//! `worklog/RULES-CHECKLIST.md`.

use super::rng::Rng;
use super::{catalogue, start, Action, Game, View};
use serde::Serialize;

/// How a seat is played.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Control {
    /// A person, moving through the page.
    Human,
    /// A computer choosing its own moves.
    Computer {
        /// Whether it plays the two-player optimum or maximises points.
        optimal: bool,
    },
}

/// One competitor.
#[derive(Clone, Debug, Serialize)]
pub struct Seat {
    pub name: String,
    pub control: Control,
    /// Score of each event, `None` until played.
    pub sheet: Vec<Option<i32>>,
}

impl Seat {
    /// Points banked so far.
    pub fn total(&self) -> i32 {
        self.sheet.iter().flatten().sum()
    }
}

/// A competition in progress.
pub struct Meet {
    seats: Vec<Seat>,
    /// Events to contest, as catalogue indices.
    events: Vec<usize>,
    /// Which of `events` is under way.
    at: usize,
    /// Seats still to play the current event, in turn order, front first.
    queue: Vec<usize>,
    /// The event the seat at the front of the queue is playing.
    game: Option<Box<dyn Game>>,
    log: Vec<String>,
}

impl Meet {
    /// Start a competition over `events` (catalogue indices).
    ///
    /// The first event's order is settled by the dice, as the rulebook
    /// says; afterwards the leader starts.
    pub fn new(seats: Vec<Seat>, events: Vec<usize>, rng: &mut Rng) -> Self {
        let mut m = Self {
            seats,
            events,
            at: 0,
            queue: Vec::new(),
            game: None,
            log: Vec::new(),
        };
        m.begin_event(rng);
        m
    }

    /// The seat to move, if the competition is still running.
    pub fn active(&self) -> Option<usize> {
        self.queue.first().copied()
    }

    /// Every seat, for the scoreboard.
    pub fn seats(&self) -> &[Seat] {
        &self.seats
    }

    /// Whether every event has been contested.
    pub fn finished(&self) -> bool {
        self.at >= self.events.len()
    }

    /// Catalogue index of the event under way.
    pub fn event(&self) -> Option<usize> {
        self.events.get(self.at).copied()
    }

    /// Order of play for the event about to start.
    ///
    /// Highest running total first. Ties keep seat order, which stands in
    /// for the rulebook's die roll — it settles the same question and
    /// keeps a replay reproducible.
    fn order(&self, first_event: bool) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.seats.len()).collect();
        if !first_event {
            order.sort_by_key(|&i| -self.seats[i].total());
        }
        order
    }

    fn begin_event(&mut self, rng: &mut Rng) {
        if self.finished() {
            self.game = None;
            self.queue.clear();
            return;
        }
        let idx = self.events[self.at];
        let (key, name) = catalogue()[idx];
        self.queue = self.order(self.at == 0);
        let who = self.queue[0];
        self.log.push(format!(
            "{name}: {} to start.",
            self.seats[who].name.clone()
        ));
        self.game = start(key, rng);
        self.settle(rng);
    }

    /// Bank any turn that is already over before anyone can move.
    ///
    /// The shot put's first die of each attempt is compulsory and a 1
    /// voids the attempt, so all three can foul while the game is being
    /// set up; the freeze events can do the same. Such a turn scores zero
    /// and needs passing over, and the next seat's turn may end the same
    /// way, so this loops.
    fn settle(&mut self, rng: &mut Rng) {
        while let Some(score) = self.game.as_ref().and_then(|g| g.view().result)
        {
            self.finish_turn(score, rng);
            if self.finished() {
                return;
            }
        }
    }

    /// Record `score` for the seat that just played and move on.
    fn finish_turn(&mut self, score: i32, rng: &mut Rng) {
        let who = self.queue.remove(0);
        let idx = self.events[self.at];
        self.seats[who].sheet[idx] = Some(score);
        self.log.push(format!(
            "{} scored {score} ({} total).",
            self.seats[who].name,
            self.seats[who].total()
        ));
        if self.queue.is_empty() {
            self.at += 1;
            self.begin_event(rng);
        } else {
            let key = catalogue()[idx].0;
            self.game = start(key, rng);
        }
    }

    /// The position in front of the seat to move.
    pub fn view(&self) -> Option<View> {
        self.game.as_ref().map(|g| {
            let mut v = g.view();
            let mut log = self.log.clone();
            log.extend(v.log);
            v.log = log;
            v
        })
    }

    /// Play `action` for the seat to move.
    pub fn apply(&mut self, action: &Action, rng: &mut Rng) -> bool {
        let Some(game) = self.game.as_mut() else {
            return false;
        };
        if !game.apply(action, rng) {
            return false;
        }
        if let Some(score) = game.view().result {
            self.finish_turn(score, rng);
            self.settle(rng);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seats(n: usize) -> Vec<Seat> {
        (0..n)
            .map(|i| Seat {
                name: format!("P{}", i + 1),
                control: Control::Human,
                sheet: vec![None; catalogue().len()],
            })
            .collect()
    }

    /// A turn already decided before anyone moves is banked and passed
    /// over. The shot put can foul all three attempts while starting up,
    /// since the first die of each is compulsory and a 1 voids it.
    #[test]
    fn a_turn_that_is_over_before_it_starts_is_banked() {
        for seed in 0..300u64 {
            let mut rng = Rng::new(seed);
            let shotput = catalogue()
                .iter()
                .position(|(k, _)| *k == "shotput")
                .expect("shot put exists");
            let m = Meet::new(seats(2), vec![shotput], &mut rng);
            if let Some(v) = m.view() {
                assert!(v.result.is_none(), "seed {seed} starts finished");
                assert!(!v.choices.is_empty(), "seed {seed} starts stuck");
            }
        }
    }

    /// Every seat plays every event, and the scores land in the right
    /// rows.
    #[test]
    fn everyone_plays_every_event() {
        let mut rng = Rng::new(5);
        let events: Vec<usize> = (0..3).collect();
        let mut m = Meet::new(seats(3), events.clone(), &mut rng);
        for _ in 0..50000 {
            if m.finished() {
                break;
            }
            let Some(v) = m.view() else { break };
            let Some(c) = v.choices.first() else { break };
            let a = c.action.clone();
            assert!(m.apply(&a, &mut rng));
        }
        assert!(m.finished());
        for s in m.seats() {
            for e in &events {
                assert!(s.sheet[*e].is_some(), "{} missed event {e}", s.name);
            }
        }
    }

    /// From the second event on, the leader starts — which hands the
    /// player behind the information advantage.
    #[test]
    fn the_leader_starts_from_the_second_event() {
        let mut rng = Rng::new(6);
        let mut m = Meet::new(seats(3), vec![0, 1], &mut rng);
        // Play out the first event entirely.
        while m.event() == Some(0) && !m.finished() {
            let a = m.view().expect("a position").choices[0].action.clone();
            m.apply(&a, &mut rng);
        }
        let leader = (0..3)
            .max_by_key(|&i| m.seats()[i].total())
            .expect("a leader");
        assert_eq!(
            m.active(),
            Some(leader),
            "totals were {:?}",
            m.seats().iter().map(Seat::total).collect::<Vec<_>>()
        );
    }
}

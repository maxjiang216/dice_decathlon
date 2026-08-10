//! The ten events solved back to front, and the value of any score
//! difference at any point in the decathlon.
//!
//! Each event needs the value function of everything after it, so the
//! solve runs in reverse rulebook order and only the 1500m stands alone.
//! What crosses each boundary is a single vector of win probabilities
//! indexed by the point difference — about ten kilobytes for the whole
//! competition, which is what an opponent needs to carry.
//!
//! Each vector is stored from the **first mover's** side. The rulebook
//! has the leading player start, so a first mover is never behind and the
//! stored half covers `d >= 0`; below zero the two players swap roles.
//! Moving second is an information advantage — you have seen the other
//! score — so this is a relabelling, not a mirror.

use super::{
    discus, highjump, javelin, longjump, polevault, running, shotput, Axis,
};

/// One event's place in the competition, and how wide its axis must be.
///
/// `acc` is how far the difference can already have drifted, `rem` how
/// much swing the remaining events still hold, and `widen` the largest
/// contribution the event itself folds in mid-play. A difference larger
/// than `rem` cannot be overturned, which is what keeps the late events
/// cheap. Derived in `worklog/2026-08-09-two-player-optimal-play/`.
struct Geometry {
    key: &'static str,
    acc: i32,
    rem: i32,
    widen: i32,
    /// Whether the difference is a *state variable* during the event.
    ///
    /// The running events fold each frozen set straight into it, so play
    /// starting level still visits negative differences and the whole
    /// axis is needed. Everywhere else the difference is a fixed
    /// parameter and the non-negative half is the whole table.
    drifts: bool,
}

const EVENTS: [Geometry; 10] = [
    Geometry {
        key: "100m",
        acc: 0,
        rem: 505,
        widen: 68,
        drifts: true,
    },
    Geometry {
        key: "longjump",
        acc: 88,
        rem: 417,
        widen: 30,
        drifts: false,
    },
    Geometry {
        key: "shotput",
        acc: 118,
        rem: 387,
        widen: 48,
        drifts: false,
    },
    Geometry {
        key: "highjump",
        acc: 166,
        rem: 339,
        widen: 30,
        drifts: false,
    },
    Geometry {
        key: "400m",
        acc: 196,
        rem: 309,
        widen: 78,
        drifts: true,
    },
    Geometry {
        key: "110mh",
        acc: 284,
        rem: 221,
        widen: 30,
        drifts: true,
    },
    Geometry {
        key: "discus",
        acc: 309,
        rem: 196,
        widen: 30,
        drifts: false,
    },
    Geometry {
        key: "polevault",
        acc: 339,
        rem: 166,
        widen: 48,
        drifts: false,
    },
    Geometry {
        key: "javelin",
        acc: 387,
        rem: 118,
        widen: 30,
        drifts: false,
    },
    Geometry {
        key: "1500m",
        acc: 417,
        rem: 88,
        widen: 83,
        drifts: true,
    },
];

/// The whole competition solved: what every score difference is worth.
pub struct Chain {
    axes: Vec<Axis>,
    /// `first_mover[e][i]` is the win probability of the player about to
    /// start event `e`, at the difference `axes[e]` stores at `i`.
    first_mover: Vec<Vec<f64>>,
}

impl Chain {
    /// Rulebook order, index 0 being the 100 Metres.
    pub fn keys() -> Vec<&'static str> {
        EVENTS.iter().map(|g| g.key).collect()
    }

    /// Index of an event by key.
    pub fn index_of(key: &str) -> Option<usize> {
        EVENTS.iter().position(|g| g.key == key)
    }

    /// Axis the event's value vector is indexed by.
    pub fn axis(&self, event: usize) -> Axis {
        self.axes[event]
    }

    /// Win probability of a nominated player entering `event` with the
    /// difference `d` in their favour.
    ///
    /// Applies the rulebook's turn order: the leader starts, so a
    /// positive difference reads the table directly and a negative one
    /// reads the opponent's side. At a tie the die roll for turn order
    /// makes it an even mix, which is exactly a half.
    pub fn value(&self, event: usize, d: i32) -> f64 {
        if event >= EVENTS.len() {
            return super::final_payoff(d);
        }
        let axis = self.axes[event];
        let table = &self.first_mover[event];
        match d.signum() {
            1 => table[axis.idx(d)],
            -1 => 1.0 - table[axis.idx(-d)],
            _ => 0.5,
        }
    }

    /// A closure giving the value of a difference *after* `event` ends.
    ///
    /// This is what an event is solved against, and what an opponent
    /// consults once its own event is over.
    pub fn clone_after(&self, event: usize) -> impl Fn(i32) -> f64 + Sync + '_ {
        move |d: i32| self.value(event + 1, d)
    }

    /// Solve every event, back to front.
    ///
    /// Takes about half a minute across all cores; the ten vectors it
    /// produces are a few kilobytes and are the only thing that needs
    /// carrying afterwards.
    pub fn solve() -> Self {
        let axes: Vec<Axis> = EVENTS
            .iter()
            .map(|g| {
                let full = Axis::for_event(g.acc, g.rem, g.widen);
                if g.drifts {
                    full
                } else {
                    Axis::first_mover(full.hi)
                }
            })
            .collect();

        let mut first_mover: Vec<Vec<f64>> = vec![Vec::new(); EVENTS.len()];
        for e in (0..EVENTS.len()).rev() {
            let solved = Self {
                axes: axes.clone(),
                first_mover: first_mover.clone(),
            };
            let after = move |d: i32| solved.value(e + 1, d);
            let span = axes[e];
            first_mover[e] = match EVENTS[e].key {
                "100m" => running::m100().solve_first_mover(span, &after),
                "longjump" => longjump::solve_first_mover(span, &after),
                "shotput" => shotput::solve_first_mover(span, &after),
                "highjump" => highjump::solve_first_mover(span, &after),
                "400m" => running::m400().solve_first_mover(span, &after),
                "110mh" => running::hurdles().solve_first_mover(span, &after),
                "discus" => discus::solve_first_mover(span, &after),
                "polevault" => polevault::solve_first_mover(span, &after),
                "javelin" => javelin::solve_first_mover(span, &after),
                _ => running::m1500().solve_first_mover(span, &after),
            };
        }
        Self { axes, first_mover }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Past the last event the value is just "am I ahead".
    #[test]
    fn beyond_the_last_event_the_score_decides() {
        let c = Chain {
            axes: vec![Axis::symmetric(1); EVENTS.len()],
            first_mover: vec![Vec::new(); EVENTS.len()],
        };
        assert!((c.value(10, 3) - 1.0).abs() < f64::EPSILON);
        assert!((c.value(10, 0) - 0.5).abs() < f64::EPSILON);
        assert!(c.value(10, -3).abs() < f64::EPSILON);
    }

    /// Keys are the rulebook's order, which is the order of play.
    #[test]
    fn events_are_in_rulebook_order() {
        assert_eq!(Chain::keys()[0], "100m");
        assert_eq!(Chain::keys()[9], "1500m");
        assert_eq!(Chain::index_of("discus"), Some(6));
    }
}

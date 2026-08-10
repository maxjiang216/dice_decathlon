//! Interactive rules engines for playing a discipline by hand.
//!
//! The solvers in [`crate::disciplines`] answer "what is optimal"; this
//! module answers "what is legal", and is what the web UI drives. The two
//! are deliberately separate: a solver collapses a discipline to a score
//! distribution and never has to name a legal move, while a player needs
//! every intermediate position spelled out.
//!
//! Keeping the playable rules in Rust rather than in the page's
//! JavaScript means there is one implementation of the rulebook to audit,
//! not two that can drift apart. The page is a renderer.

pub mod decathlon;
pub mod freeze;
pub mod ladder;
pub mod longjump;
pub mod meet;
pub mod rng;
pub mod running;
pub mod shotput;

use serde::{Deserialize, Serialize};

/// One die on the table.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Die {
    /// The face showing, `1..=6`.
    pub face: u8,
    /// Whether this die is locked and can no longer be rethrown.
    pub frozen: bool,
}

/// A move the player can make. One variant per rulebook verb; events use
/// only the subset that applies to them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    /// Pick up the active dice and throw them again, spending a rethrow.
    Reroll,
    /// Lock the active dice as they stand.
    Freeze,
    /// Take a jump at the current bar. `dice` is how many to throw,
    /// which only the pole vault lets you choose.
    Attempt { dice: Option<u8> },
    /// Lock the named dice (indices into the dice on the table) and
    /// keep the attempt alive.
    Keep { dice: Vec<u8> },
    /// End the attempt here and bank what is frozen.
    Stop,
    /// Decline the current bar and move to the next one.
    ///
    /// Free — you keep your best and stay in — which is why it is
    /// sometimes right even though it scores nothing.
    Skip,
}

/// A labelled action, ready to be drawn as a button.
#[derive(Clone, Debug, Serialize)]
pub struct Choice {
    /// The action to send back to [`Game::apply`].
    pub action: Action,
    /// Button text.
    pub label: String,
    /// One-line explanation of what the move does, for a tooltip.
    pub detail: String,
}

/// A group of dice the UI draws together (a "set" in the rulebook).
#[derive(Clone, Debug, Serialize)]
pub struct Group {
    /// Heading, e.g. `"Set 1"`.
    pub label: String,
    /// The dice in this group, or empty if it has not been thrown yet.
    pub dice: Vec<Die>,
    /// Score of this group once frozen.
    pub score: Option<i32>,
    /// Whether this is the group the player is acting on.
    pub active: bool,
}

/// Everything the UI needs to draw the current position.
#[derive(Clone, Debug, Serialize)]
pub struct View {
    /// Discipline key, e.g. `"100m"`.
    pub key: &'static str,
    /// Human-readable discipline name.
    pub name: &'static str,
    /// Dice groups, in play order.
    pub groups: Vec<Group>,
    /// Rethrows still available from the shared pool.
    pub rerolls_left: u32,
    /// Total rethrows the discipline grants.
    pub rerolls_total: u32,
    /// Score so far from everything already frozen.
    pub running_score: i32,
    /// Legal moves; empty exactly when the event is over.
    pub choices: Vec<Choice>,
    /// Final score, set once the event is over.
    pub result: Option<i32>,
    /// Human-readable history of the attempt, oldest first.
    pub log: Vec<String>,
    /// Bar being faced, in the jumping events.
    pub bar: Option<i32>,
    /// Jumps already used at this bar.
    pub jumps_used: Option<u32>,
    /// Jumps allowed at each bar.
    pub jumps_total: Option<u32>,
    /// Best height cleared so far, in the jumping events.
    pub best: Option<i32>,
    /// Attempt being played, in the best-of-three events.
    pub attempt: Option<u32>,
    /// Attempts each player gets.
    pub attempts_total: Option<u32>,
    /// Score of each event so far, when playing the full competition.
    pub sheet: Option<Vec<Option<i32>>>,
    /// Index of the event under way, in rulebook order.
    pub event_index: Option<usize>,
    /// Total across the events already finished.
    pub total: Option<i32>,
    /// The face that hurts in this event, if any, so the UI can mark it.
    ///
    /// A six is only bad where it subtracts — the 100m, 400m and 1500m.
    /// In the discus a six is the *best* die you can freeze, and in the
    /// shot put and pole vault it is a **one** that spoils things.
    pub warn_face: Option<u8>,
}

/// A discipline that can be played move by move.
pub trait Game {
    /// The current position.
    fn view(&self) -> View;

    /// Play `action`. Returns `false` if it is not currently legal, in
    /// which case the position is unchanged.
    fn apply(&mut self, action: &Action, rng: &mut rng::Rng) -> bool;
}

/// Start a game of the discipline named by `key`, or `None` if no
/// interactive engine exists for it yet.
#[must_use]
pub fn start(key: &str, rng: &mut rng::Rng) -> Option<Box<dyn Game>> {
    if let Some(r) = running::rules(key) {
        return Some(Box::new(running::Running::new(r, rng)));
    }
    if let Some(r) = ladder::rules(key) {
        return Some(Box::new(ladder::Ladder::new(r, rng)));
    }
    if let Some(r) = freeze::rules(key) {
        return Some(Box::new(freeze::Freeze::new(r, rng)));
    }
    match key {
        "shotput" => Some(Box::new(shotput::ShotPut::new(rng))),
        "longjump" => Some(Box::new(longjump::LongJump::new(rng))),
        // The whole competition, wrapping each event in turn. Not in
        // `catalogue`, which lists the individual disciplines.
        "decathlon" => Some(Box::new(decathlon::Decathlon::new(rng))),
        _ => None,
    }
}

/// Playable disciplines as `(key, name)`, in rulebook order.
///
/// Kept in step with [`start`] by a test: a menu entry that cannot be
/// started is worse than no entry at all.
///
/// The order is the order of play, which is what a menu should offer.
#[must_use]
pub fn catalogue() -> Vec<(&'static str, &'static str)> {
    let mut out: Vec<(&'static str, &'static str)> = Vec::new();
    for r in running::RUNNING {
        out.push((r.key, r.name));
    }
    for r in ladder::LADDERS {
        out.push((r.key, r.name));
    }
    for r in freeze::FREEZE {
        out.push((r.key, r.name));
    }
    out.push(("shotput", "Shot Put"));
    out.push(("longjump", "Long Jump"));
    // Rulebook order, so the list reads like the competition.
    let order = [
        "100m",
        "longjump",
        "shotput",
        "highjump",
        "400m",
        "110mh",
        "discus",
        "polevault",
        "javelin",
        "1500m",
    ];
    out.sort_by_key(|(k, _)| order.iter().position(|o| o == k).unwrap_or(99));
    out
}

/// Keys of the disciplines that can currently be played interactively.
#[must_use]
pub fn playable() -> Vec<&'static str> {
    catalogue().into_iter().map(|(key, _)| key).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every advertised discipline must actually start, and offer a move.
    ///
    /// The catalogue and `start` are separate lists, and they drifted
    /// once already — the menu offered ten events while six of them threw
    /// "no playable engine".
    #[test]
    fn everything_advertised_can_be_started() {
        let mut rng = rng::Rng::new(4242);
        let cat = catalogue();
        assert_eq!(cat.len(), 10, "all ten events should be playable");
        for (key, name) in cat {
            let game = start(key, &mut rng)
                .unwrap_or_else(|| panic!("no engine for {key}"));
            let view = game.view();
            assert_eq!(view.key, key);
            assert_eq!(view.name, name, "catalogue name disagrees for {key}");
            assert!(!view.choices.is_empty(), "{key} starts with no moves");
        }
    }

    /// A six only costs you where it subtracts. Marking every six would
    /// be wrong in six of the ten events — in the discus it is the best
    /// die you can freeze.
    #[test]
    fn only_the_right_face_is_flagged() {
        let mut rng = rng::Rng::new(77);
        let want = |key: &str| match key {
            "100m" | "400m" | "1500m" => Some(6),
            "shotput" | "polevault" => Some(1),
            _ => None,
        };
        for (key, _) in catalogue() {
            let game = start(key, &mut rng).expect("engine exists");
            assert_eq!(game.view().warn_face, want(key), "{key}");
        }
    }

    /// `playable` is just the catalogue's keys, so it cannot drift.
    #[test]
    fn playable_matches_the_catalogue() {
        let keys: Vec<&str> = catalogue().into_iter().map(|(k, _)| k).collect();
        assert_eq!(playable(), keys);
    }
}

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

pub mod freeze;
pub mod ladder;
pub mod rng;
pub mod running;

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
    running::rules(key)
        .map(|r| Box::new(running::Running::new(r, rng)) as Box<dyn Game>)
}

/// Keys of the disciplines that can currently be played interactively.
#[must_use]
pub fn playable() -> Vec<&'static str> {
    running::RUNNING
        .iter()
        .map(|r| r.key)
        .chain(ladder::LADDERS.iter().map(|r| r.key))
        .chain(freeze::FREEZE.iter().map(|r| r.key))
        .collect()
}
